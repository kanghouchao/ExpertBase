//! 工坊「進行中の 1 ターン」を組件状態の外（モジュール単例）に持つストア。
//! 会話を切り替えても実行を殺さず（後台で継続）、終わったら捕獲した会話 id へ存盤する。
//! 本地モデルは直列なので同時に 1 ターンだけ＝後端の単飛取消フラグのままで足りる。

import {
  saveWorkshopConversation,
  workshopCancel,
  workshopChat,
  type ChatPhase,
  type WorkshopMessage,
} from "@/shared/api/tauri/client";

import { notifyWorkshopHistoryChanged } from "./history";
import { toChatTurn, type ChatUiPhase, type ToolEvent } from "./process-state";

// 進行中の 1 ターンの実時態。baseHistory は送信時に存盤済みの履歴（待回复の user メッセージ含む）。
export type RunSnapshot = {
  kbPath: string;
  conversationId: number;
  phase: ChatUiPhase;
  baseHistory: WorkshopMessage[];
  narration: string;
  thinking: string;
  tools: ToolEvent[];
};

export type RunStoreState = {
  active: RunSnapshot | null;
  error: { kbPath: string; conversationId: number; message: string } | null;
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
      tools: [],
    },
    error: null,
  });
  void run(args);
}

async function run(args: StartArgs): Promise<void> {
  const { kbPath, conversationId, sourceIds, baseHistory } = args;
  const tools: ToolEvent[] = [];
  let narration = "";
  let thinking = "";
  try {
    const reply = await workshopChat(
      sourceIds,
      baseHistory.map(toChatTurn),
      args.model,
      args.think,
      args.tools,
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
          const target = [...tools].reverse().find((tool) => tool.name === p.name && !tool.summary);
          if (target) target.summary = p.summary;
          patch(kbPath, conversationId, { tools: [...tools] });
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
        error: {
          kbPath,
          conversationId,
          message: error instanceof Error ? error.message : String(error),
        },
      });
    }
  }
}

async function finishSave(args: StartArgs, completed: WorkshopMessage[]): Promise<void> {
  try {
    await saveWorkshopConversation({
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
      error: {
        kbPath: args.kbPath,
        conversationId: args.conversationId,
        message: error instanceof Error ? error.message : String(error),
      },
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
    void workshopCancel();
    return null;
  }

  const last = active.baseHistory.at(-1);
  const rollback =
    last?.role === "user"
      ? { prompt: last.text, history: active.baseHistory.slice(0, -1) }
      : null;
  emit({ active: null, error: null });
  void workshopCancel();
  return rollback;
}

/** KB 切替など: 進行中の実行を捨てる（存盤目標が動くので保存しない）。 */
export function discardActive(): void {
  if (!state.active) return;
  cancelled = true;
  emit({ active: null, error: null });
  void workshopCancel();
}
