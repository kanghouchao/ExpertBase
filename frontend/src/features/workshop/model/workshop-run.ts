//! 工坊「進行中の 1 ターン」を組件状態の外（モジュール単例）に持つストア。
//! 会話を切り替えても実行を殺さず（後台で継続）、終わったら捕獲した会話 id へ存盤する。
//! 本地モデルは直列なので同時に 1 ターンだけ＝後端の単飛取消フラグのままで足りる。

import { workshopApi, type ChatPhase, type Skill, type WorkshopMessage } from "@/shared/api";

import { notifyWorkshopHistoryChanged } from "./history";
import { toChatTurn, type ChatUiPhase, type ToolEvent } from "./process-state";

// 進行中の 1 ターンの実時態。baseHistory は送信時に存盤済みの履歴（待回复の user メッセージ含む）。
// confirm は未応答の確認要求（破壊的ツールの human-in-the-loop）。応答するとカードを畳む。
export type RunSnapshot = {
  kbPath: string;
  conversationId: number;
  phase: ChatUiPhase;
  baseHistory: WorkshopMessage[];
  narration: string;
  thinking: string;
  tools: ToolEvent[];
  confirm: { id: number; summary: string } | null;
};

export type RunStoreState = {
  active: RunSnapshot | null;
  error: { kbPath: string; conversationId: number; cause: unknown } | null;
};

export type StopResult = {
  prompt: string;
  history: WorkshopMessage[];
};

type StartArgs = {
  kbPath: string;
  conversationId: number;
  sourceIds: string[];
  baseHistory: WorkshopMessage[];
  model: string;
  think: boolean;
  tools: boolean;
  /** 発見済み技能一覧（activate_skill の成功判定に本文を照合するため）。 */
  skills: Skill[];
  /** この会話で発動済みの技能名（ボタン発動・モデル自動発動を問わず一本化、重複排除済み）。 */
  activatedSkillNames: string[];
  /** activate_skill ツールの成功を観測したら呼ぶ（呼び出し側が activatedSkillNames へ記帳する）。 */
  onSkillActivated: (name: string) => void;
};

let state: RunStoreState = { active: null, error: null };
// 停止/破棄フラグ。run の途中・終了で実行を続行すべきか判定する。
let cancelled = false;
const listeners = new Set<() => void>();

// state は emit のときだけ差し替える＝useSyncExternalStore へ安定参照を返す。
function emit(next: RunStoreState): void {
  state = next;
  for (const listener of listeners) listener();
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function getSnapshot(): RunStoreState {
  return state;
}

export function isRunForConversation(
  run: RunSnapshot | null,
  kbPath: string | null | undefined,
  conversationId: number | null
): run is RunSnapshot {
  return run !== null && run.kbPath === kbPath && run.conversationId === conversationId;
}

// 現在のアクティブ実行が conversationId のものか。破棄・差し替え後は false。
function isActive(kbPath: string, conversationId: number): boolean {
  return (
    state.active !== null &&
    state.active.kbPath === kbPath &&
    state.active.conversationId === conversationId
  );
}

function patch(kbPath: string, conversationId: number, fields: Partial<RunSnapshot>): void {
  if (!isActive(kbPath, conversationId)) return;
  emit({ active: { ...(state.active as RunSnapshot), ...fields }, error: state.error });
}

// 明示発動（スラッシュコマンド/チップ）は activate_skill ツールを呼ばず system prompt へ
// 直接注入するので、そのままだと「技能が効いた」という手掛かりが会話に一切残らない
// （検索・書き込みツールと違って何のカードも出ない）。ここで実際のツール呼び出しと同じ
// ToolEvent を合成し、既存の ToolCallCard/ToolCallLog にそのまま乗せる（新しい表示は作らない、
// 展開すると注入した本文そのものが見える＝モデル自動発動時の activate_skill 結果と同じ体裁）。
function activatedSkillToolEvents(args: StartArgs): ToolEvent[] {
  return args.activatedSkillNames
    .map((name) => args.skills.find((skill) => skill.name === name))
    .filter((skill): skill is Skill => skill !== undefined)
    .map((skill) => ({
      name: "activate_skill",
      args: JSON.stringify({ name: skill.name }),
      summary: skill.body,
    }));
}

/** 1 ターンを開始する。アクティブ態を同期で立て、後は後台で流す。 */
export function startRun(args: StartArgs): void {
  cancelled = false;
  emit({
    active: {
      kbPath: args.kbPath,
      conversationId: args.conversationId,
      phase: "connecting",
      baseHistory: args.baseHistory,
      narration: "",
      thinking: "",
      tools: activatedSkillToolEvents(args),
      confirm: null,
    },
    error: null,
  });
  void run(args);
}

async function run(args: StartArgs): Promise<void> {
  const { kbPath, conversationId, sourceIds, baseHistory } = args;
  const tools: ToolEvent[] = activatedSkillToolEvents(args);
  let narration = "";
  let thinking = "";
  try {
    const reply = await workshopApi.chat(
      {
        sourceIds,
        messages: baseHistory.map(toChatTurn),
        model: args.model,
        think: args.think,
        tools: args.tools,
        activatedSkillNames: args.activatedSkillNames,
      },
      (p: ChatPhase) => {
        if (!isActive(kbPath, conversationId)) return; // 破棄済みなら書かない
        if (p.phase === "narration") {
          narration += p.delta;
          patch(kbPath, conversationId, { phase: "generating", narration });
        } else if (p.phase === "thinking") {
          thinking += p.delta;
          patch(kbPath, conversationId, { phase: "thinking", thinking });
        } else if (p.phase === "toolCall") {
          tools.push({ name: p.name, args: p.args });
          patch(kbPath, conversationId, { tools: [...tools] });
        } else if (p.phase === "toolResult") {
          // 直近の同名・未完了の呼び出しに結果サマリを埋める。
          // 確認待ち中は他ツールが動かないので、結果到着＝確認解消としてカードも畳む。
          const target = [...tools].reverse().find((tool) => tool.name === p.name && !tool.summary);
          if (target) target.summary = p.summary;
          patch(kbPath, conversationId, { tools: [...tools], confirm: null });
          // activate_skill の成功を観測したら発動済みへ記帳する。技能名は toolResult 自体には
          // 乗らないので、対応する toolCall の args から拾う。成功時の戻り値は該当技能の本文
          // そのもの（activate_skill.rs）なので、本文と一致するかで成功/失敗を判定する
          // （失敗通知の文面に依存しない＝本文がたまたま "(" で始まっても誤判定しない）。
          if (p.name === "activate_skill" && target) {
            try {
              const skillName = (JSON.parse(target.args) as { name?: string }).name;
              const skill = skillName ? args.skills.find((s) => s.name === skillName) : undefined;
              if (skill && p.summary === skill.body) args.onSkillActivated(skill.name);
            } catch {
              // args の解析に失敗しても無視する（記帳漏れは次ターンの catalog 再掲で回復可能）。
            }
          }
        } else if (p.phase === "confirmRequest") {
          patch(kbPath, conversationId, { confirm: { id: p.id, summary: p.summary } });
        } else {
          patch(kbPath, conversationId, { phase: "loadingModel" });
        }
      }
    );
    if (!isActive(kbPath, conversationId)) return; // 破棄済み（KB 切替など）＝存盤しない
    await finishSave(args, [
      ...baseHistory,
      {
        role: "ai",
        text: reply || narration,
        thinking: thinking || undefined,
        tools: tools.length ? tools : undefined,
      },
    ]);
  } catch (error) {
    if (!isActive(kbPath, conversationId)) return; // discardActive で破棄済み
    if (cancelled) {
      // 停止: 途中まで出た本文を失わずに保存（無ければ user メッセージは既に存盤済み）。
      if (narration || thinking || tools.length) {
        await finishSave(args, [
          ...baseHistory,
          {
            role: "ai",
            text: narration,
            thinking: thinking || undefined,
            tools: tools.length ? tools : undefined,
          },
        ]);
      } else {
        emit({ active: null, error: null });
        notifyWorkshopHistoryChanged();
      }
    } else {
      // 失敗: user メッセージは存盤済み。エラー文だけ視図へ渡す。
      emit({
        active: null,
        error: { kbPath, conversationId, cause: error },
      });
    }
  }
}

async function finishSave(args: StartArgs, completed: WorkshopMessage[]): Promise<void> {
  try {
    await workshopApi.saveConversation({
      kbPath: args.kbPath,
      id: args.conversationId,
      sourceIds: args.sourceIds,
      messages: completed,
    });
    if (!isActive(args.kbPath, args.conversationId)) return;
    emit({ active: null, error: null });
    notifyWorkshopHistoryChanged();
  } catch (error) {
    if (!isActive(args.kbPath, args.conversationId)) return;
    emit({
      active: null,
      error: { kbPath: args.kbPath, conversationId: args.conversationId, cause: error },
    });
  }
}

/** 表示中の対話だけを停止する。出力前なら送信直前へ戻す情報を返す。 */
export function stopActive(
  kbPath: string | null | undefined,
  conversationId: number | null
): StopResult | null {
  const active = state.active;
  if (!isRunForConversation(active, kbPath, conversationId)) return null;
  cancelled = true;

  const hasOutput = !!active.narration || !!active.thinking || active.tools.length > 0;
  if (hasOutput) {
    void workshopApi.cancel();
    return null;
  }

  const last = active.baseHistory.at(-1);
  const rollback =
    last?.role === "user"
      ? { prompt: last.text, history: active.baseHistory.slice(0, -1) }
      : null;
  emit({ active: null, error: null });
  void workshopApi.cancel();
  return rollback;
}

/** 表示中の実行の確認要求へ応答し（許可 / 拒否を後端へ回填）、カードを畳む。
 * IPC が失敗したらカードを残す（再試行の入口）＝後端が待ち続けるのに応答手段を失わない。 */
export async function answerConfirm(
  kbPath: string | null | undefined,
  conversationId: number | null,
  approved: boolean
): Promise<void> {
  const active = state.active;
  if (!isRunForConversation(active, kbPath, conversationId) || !active.confirm) return;
  const request = active.confirm;
  try {
    await workshopApi.confirm(request.id, approved);
    patch(active.kbPath, active.conversationId, { confirm: null });
  } catch (cause) {
    if (!isActive(active.kbPath, active.conversationId)) return;
    // カードは残したまま、エラーは既存のエラー経路（実行中も横に表示される）で見せる。
    emit({
      active: state.active,
      error: { kbPath: active.kbPath, conversationId: active.conversationId, cause },
    });
  }
}

/** KB 切替など: 進行中の実行を捨てる（存盤目標が動くので保存しない）。 */
export function discardActive(): void {
  if (!state.active) return;
  cancelled = true;
  emit({ active: null, error: null });
  void workshopApi.cancel();
}
