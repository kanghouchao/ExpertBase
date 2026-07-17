"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";

import { Icon } from "@/shared/ui/icon";
import { Tag } from "@/shared/ui/tag";
import { Button } from "@/shared/ui/button";
import { Markdown } from "@/shared/ui/markdown";
import { PageHead } from "@/shared/ui/page-head";
import { cn } from "@/shared/lib/utils";
import { useI18n } from "@/shared/providers/providers";
import { translateError } from "@/shared/i18n/translate";
import type { RawMaterial, RawType } from "@/entities/material";
import { SkillChip } from "@/features/plugin";
import { canRemoveSource, runningLabelKey } from "../model/process-state";
import { useSkillSlash } from "../model/use-skill-slash";
import { useWorkshopSession } from "../model/use-workshop-session";
import { Inspector } from "./inspector";
import { SkillSlashMenu } from "./skill-slash-menu";
import { SourceChip } from "./source-chip";
import { ThinkingPanel } from "./thinking-panel";
import { ToolCallCard, ToolCallLog } from "./tool-call-card";

// 編排は useWorkshopSession(workshop-session コントローラ)が全部持つ。
// ここは快照を描き、操作をコントローラへ回すだけの純渲染。
export function WorkshopView() {
  const { t } = useI18n();
  const s = useWorkshopSession();
  const threadEnd = useRef<HTMLDivElement>(null);
  const composerRef = useRef<HTMLTextAreaElement>(null);

  // 入力欄 `/` 技能発動（issue #44）。カーソル位置は選択範囲が動くたび読み直す。
  const [cursor, setCursor] = useState(0);
  const slash = useSkillSlash({ value: s.instruction, cursor, skills: s.skills });
  // setInstruction 後の再描画を待ってからテキストエリアの実カーソルを動かす（受控入力の作法）。
  const pendingCursorRef = useRef<number | null>(null);
  useEffect(() => {
    const pos = pendingCursorRef.current;
    if (pos === null) return;
    pendingCursorRef.current = null;
    const el = composerRef.current;
    if (el) el.setSelectionRange(pos, pos);
    setCursor(pos);
  }, [s.instruction]);

  // 候補選択の確定: `/query` を打ち消して技能を発動する。ボタン発動と同じ記帳口
  // （s.activateSkill）に合流するので、以降の挙動（重複排除・chip 表示）は共通。
  function selectSlashSkill(skill: { name: string }): void {
    if (!(slash.state.open && "matches" in slash.state)) return;
    const { lineStart } = slash.state;
    const next = s.instruction.slice(0, lineStart) + s.instruction.slice(cursor);
    pendingCursorRef.current = lineStart;
    s.setInstruction(next);
    s.activateSkill(skill.name);
  }

  // 新しいメッセージ・流式の進みに合わせて会話を最下部に追従させる。
  useEffect(() => {
    threadEnd.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [s.displayMessages, s.generating, s.narrationBuf, s.thinkingBuf]);

  // 入力欄は 1 行から始まり、内容に合わせて自動的に伸びる(送信後は空＝1 行に戻る)。
  useEffect(() => {
    const el = composerRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [s.instruction]);

  // 生成中は Escape で停止できる。
  const { stop } = s;
  useEffect(() => {
    if (!s.generating) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") stop();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [s.generating, stop]);

  // 素材はコントローラでは絶対パスの列。表示するときだけチップ用の型へ投影する。
  const visibleSources = s.sourceIds.map((path) =>
    materialFromFile(path, t("workshop.addLocalFile"))
  );
  const composerHint = composerHintKey(s);

  return (
    <div className="view-enter flex flex-col lg:h-full">
      {s.displayMessages.length > 0 && <ProcessTopBar t={t} />}

      {/* lg 以上は 2 カラム(会話は内部スクロール)、それ未満は 1 カラムでページ全体スクロール。 */}
      {/* 未開始(空)時は会話列を縦中央へ寄せ、ウェルカム見出し + コンポーザーを中央に置く。 */}
      <div
        className={cn(
          "flex flex-col gap-5 lg:min-h-0 lg:flex-1 lg:flex-row",
          s.displayMessages.length > 0 && "pt-5"
        )}
      >
        {/* ── 会話列 ── */}
        <div
          className={cn(
            "flex min-w-0 flex-1 flex-col",
            s.displayMessages.length === 0 && "lg:justify-center"
          )}
        >
          <div
            className={
              s.displayMessages.length === 0
                ? "lg:flex-none"
                : "lg:min-h-0 lg:flex-1 lg:overflow-auto"
            }
          >
            <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-1 py-1">
              {/* 未開始時はウェルカム見出しを中央に置く。 */}
              {s.displayMessages.length === 0 && (
                <PageHead
                  eyebrow={t("workshop.eyebrow")}
                  title={t("workshop.title")}
                  sub={t("workshop.listSub")}
                />
              )}
              {/* 会話(多輪)。 */}
              {s.displayMessages.map((m, i) =>
                m.role === "user" ? (
                  <div key={i} className="flex justify-end">
                    {/* 素材は右パネル/コンポーザーで示すので会話には出さない(履歴で常に文脈に入る)。 */}
                    <div className="max-w-[84%] rounded-[14px_14px_4px_14px] bg-brand px-4 py-2.5 text-[14px] leading-relaxed whitespace-pre-wrap text-white">
                      {m.text}
                    </div>
                  </div>
                ) : (
                  <ChatRow key={i} ai>
                    {m.thinking && <ThinkingPanel text={m.thinking} streaming={false} />}
                    {m.tools && m.tools.length > 0 && <ToolCallLog tools={m.tools} />}
                    {/* AI 出力は Markdown。表示時にレンダリングする(保存は Markdown のまま)。 */}
                    <Markdown className="text-[14.5px] text-ink-soft">{m.text}</Markdown>
                  </ChatRow>
                )
              )}

              {/* AI: 生成中 */}
              {s.generating && (
                <ChatRow ai>
                  {s.thinkingBuf && (
                    <ThinkingPanel text={s.thinkingBuf} streaming={s.phase === "thinking"} />
                  )}
                  {/* エージェントのツール呼び出し(検索・書き込み)をカードで見せる。 */}
                  {s.toolLog.length > 0 && (
                    <div className="mb-2.5 flex flex-col gap-1.5">
                      {s.toolLog.map((tool, idx) => (
                        <ToolCallCard key={idx} tool={tool} />
                      ))}
                    </div>
                  )}
                  {/* 破壊的操作の確認カード。応答するまでエージェントはツール内で待つ。 */}
                  {s.confirmReq && (
                    <div className="mb-2.5 overflow-hidden rounded-lg border border-line bg-surface-2 text-[12.5px]">
                      <div className="px-3 py-2">
                        <div className="mb-1 font-semibold text-ink-soft">
                          {t("workshop.confirm.request")}
                        </div>
                        <div className="font-mono text-[12px] leading-relaxed whitespace-pre-wrap text-ink-faint">
                          {s.confirmReq.summary}
                        </div>
                      </div>
                      <div className="flex gap-2 border-t border-line px-3 py-2">
                        <Button size="sm" onClick={() => s.confirm(true)}>
                          {t("workshop.confirm.allow")}
                        </Button>
                        <Button size="sm" variant="outline" onClick={() => s.confirm(false)}>
                          {t("workshop.confirm.deny")}
                        </Button>
                      </div>
                    </div>
                  )}
                  {/* 「AI が今書いている本文」を流式表示＝過程が見える(数字ではなく実テキスト)。 */}
                  {s.narrationBuf && (
                    <div className="mb-2.5 text-[14px] leading-relaxed whitespace-pre-wrap text-ink-soft">
                      {s.narrationBuf}
                    </div>
                  )}
                  <div className="flex items-center gap-2.5 text-[13.5px] text-ink-soft">
                    <span className="size-4 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
                    {t(runningLabelKey(s.phase, s.selectedThinking))}
                  </div>
                </ChatRow>
              )}

              <div ref={threadEnd} />
            </div>
          </div>

          {/* ── サジェスト + コンポーザー ── */}
          <div className="mx-auto mt-3 w-full max-w-3xl">
            {s.displayMessages.length === 0 && s.visibleHasOllama && (
              <div className="mb-2.5 flex flex-wrap items-center gap-2">
                <span className="font-mono text-[10.5px] font-bold tracking-widest text-ink-faint uppercase">
                  {t("workshop.sug.label")}
                </span>
                {[t("workshop.sug.1"), t("workshop.sug.2"), t("workshop.sug.3")].map((sug) => (
                  <button
                    key={sug}
                    type="button"
                    onClick={() => s.setInstruction(sug)}
                    className="inline-flex items-center gap-1.5 rounded-full border border-line-strong bg-surface px-3 py-1.5 text-[12.5px] font-semibold text-ink-soft shadow-(--shadow-sm) transition-colors hover:bg-surface-2"
                  >
                    <Icon name="spark" size={12} className="text-ai" />
                    {sug}
                  </button>
                ))}
              </div>
            )}

            <div className="rounded-[18px] border border-line-strong bg-surface p-3 shadow-(--shadow-md)">
              {/* 関連文档は会話開始前だけ示す。最初の送信で文脈に入るので以降は隠す(+ は常に追加可)。 */}
              {visibleSources.length > 0 && s.displayMessages.length === 0 && (
                <div className="mb-2.5 flex flex-wrap gap-2">
                  {visibleSources.map((m) => (
                    <SourceChip
                      key={m.id}
                      material={m}
                      onRemove={
                        canRemoveSource(s.displayMessages.length, visibleSources.length)
                          ? () => s.toggleSource(m.id)
                          : undefined
                      }
                    />
                  ))}
                </div>
              )}
              {s.activatedSkillNames.length > 0 && (
                <div className="mb-2.5 flex flex-wrap gap-2">
                  {s.activatedSkillNames.map((name) => {
                    const skill = s.skills.find((sk) => sk.name === name);
                    if (!skill) return null;
                    return (
                      <SkillChip key={name} skill={skill} onRemove={() => s.deactivateSkill(name)} />
                    );
                  })}
                </div>
              )}
              <div className="relative">
                {slash.state.open && (
                  <SkillSlashMenu
                    matches={"matches" in slash.state ? slash.state.matches : null}
                    activeIndex={"activeIndex" in slash.state ? slash.state.activeIndex : 0}
                    onSelect={selectSlashSkill}
                    onHover={(index) => {
                      // moveActive は ±1 ずつしか動かせないので、必要な歩数だけ呼ぶ
                      // （関数更新なので同一イベント内で複数回呼んでも正しく積算される）。
                      const current = "activeIndex" in slash.state ? slash.state.activeIndex : 0;
                      const steps = index - current;
                      for (let i = 0; i < Math.abs(steps); i += 1) {
                        slash.moveActive(steps > 0 ? 1 : -1);
                      }
                    }}
                  />
                )}
                <textarea
                  ref={composerRef}
                  value={s.instruction}
                  onChange={(event) => {
                    s.setInstruction(event.target.value);
                    setCursor(event.target.selectionStart ?? event.target.value.length);
                  }}
                  onKeyUp={(event) => setCursor(event.currentTarget.selectionStart ?? 0)}
                  onClick={(event) => setCursor(event.currentTarget.selectionStart ?? 0)}
                  onKeyDown={(event) => {
                    if (slash.state.open && "matches" in slash.state) {
                      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                        event.preventDefault();
                        slash.moveActive(event.key === "ArrowDown" ? 1 : -1);
                        return;
                      }
                      if (event.key === "Enter") {
                        event.preventDefault();
                        if (slash.activeSkill) selectSlashSkill(slash.activeSkill);
                        return;
                      }
                      if (event.key === "Escape") {
                        event.preventDefault();
                        slash.close();
                        return;
                      }
                    } else if (slash.state.open && event.key === "Escape") {
                      // 技能 0 件の案内表示。Esc だけ拾い、Enter は通常の送信へ流す。
                      event.preventDefault();
                      slash.close();
                      return;
                    }
                    // Enter で送信、Shift+Enter で改行(複数行入力)。
                    if (event.key === "Enter" && !event.shiftKey) {
                      event.preventDefault();
                      void s.send();
                    }
                  }}
                  placeholder={t("workshop.composerPh")}
                  rows={1}
                  className="max-h-40 w-full resize-none overflow-y-auto bg-transparent px-1 text-[14.5px] leading-relaxed text-ink outline-none"
                />
              </div>
              <div className="mt-2 flex items-center gap-2.5">
                {/* ＋ 外部ローカルファイルを素材に追加(OS のファイル選択ダイアログ) */}
                <button
                  type="button"
                  onClick={() => void s.addLocalFile()}
                  disabled={s.generating}
                  title={t("workshop.addLocalFile")}
                  className="grid size-9 flex-none place-items-center rounded-[10px] border border-line-strong bg-surface text-ink-soft transition-colors hover:bg-surface-2 disabled:opacity-40"
                >
                  <Icon name="plus" size={18} />
                </button>
                <div className="flex h-9 min-w-0 max-w-60 items-center gap-1.5 rounded-[10px] border border-line-strong bg-surface px-2.5">
                  <Icon name="bot" size={15} className="flex-none text-ai" />
                  <select
                    value={s.visibleSelectedModel}
                    onChange={(event) => s.selectModel(event.target.value)}
                    disabled={!s.visibleHasOllama || s.visibleModels.length === 0 || s.generating}
                    className="min-w-0 flex-1 truncate appearance-none bg-transparent font-mono text-[12px] font-semibold text-ink outline-none disabled:opacity-50"
                  >
                    {s.visibleModels.length === 0 ? (
                      <option value="">{t("workshop.noModels")}</option>
                    ) : (
                      s.visibleModels.map((model) => (
                        <option key={model.name} value={model.name}>
                          {[
                            model.name,
                            model.thinking ? t("workshop.think.badge") : null,
                            model.tools ? t("workshop.tools.badge") : null,
                          ]
                            .filter(Boolean)
                            .join(" · ")}
                        </option>
                      ))
                    )}
                  </select>
                  <Icon name="chevD" size={14} className="text-ink-muted" />
                </div>
                <div className="flex-1" />
                {s.generating ? (
                  <Button variant="outline" className="h-9 px-5" onClick={s.stop}>
                    <Icon name="x" size={15} />
                    {t("workshop.stop")}
                  </Button>
                ) : (
                  <Button
                    className="h-9 bg-brand px-5 text-white hover:bg-brand/85"
                    disabled={!s.canGenerate}
                    onClick={() => void s.send()}
                  >
                    <Icon name="send" size={15} />
                    {t("workshop.send")}
                  </Button>
                )}
              </div>
              {/* 送信不可の理由は一行だけ(Ollama 未起動/モデル無し/他会話が生成中は
                  互いに排他なので、常に高々一件しか出ない一本のステータス行にまとめる)。 */}
              {composerHint && (
                <div className="mt-2 px-1 text-[12px] text-ink-faint">{t(composerHint)}</div>
              )}
              {s.error != null && (
                <div className="mt-2 px-1 text-[12.5px] font-semibold text-brand">
                  {translateError(t, s.error)}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ── 右側ステータス(対話態のみ。実行状態 + モデル + 在席素材。草稿は出さない) ── */}
        {s.displayMessages.length > 0 && (
          <Inspector
            model={s.visibleSelectedModel}
            generating={s.generating}
            runningLabel={t(runningLabelKey(s.phase, s.selectedThinking))}
            thinking={s.selectedThinking}
            tools={s.selectedTools}
            sources={visibleSources}
          />
        )}
      </div>
    </div>
  );
}

// 外部ローカルファイルを素材チップへ。id は絶対パス、拡張子で表示型を決める(pdf/doc/その他=note)。
function materialFromFile(path: string, label: string): RawMaterial {
  const name = path.split(/[\\/]/).pop() || path;
  const ext = name.includes(".") ? (name.split(".").pop() ?? "").toLowerCase() : "";
  const type: RawType = ext === "pdf" ? "pdf" : ext === "docx" || ext === "doc" ? "doc" : "note";
  return {
    id: path,
    type,
    title: name,
    source: label,
  };
}

// コンポーザー下の一行ステータス。3条件は互いに排他(Ollama 未起動 → モデル無し →
// 他会話が生成中、の優先順)なので、常に高々一つの i18n key を返す。
function composerHintKey(s: {
  visibleHasOllama: boolean;
  visibleModels: unknown[];
  someoneGenerating: boolean;
  generating: boolean;
}): string | null {
  if (!s.visibleHasOllama) return "workshop.noKey";
  if (s.visibleModels.length === 0) return "workshop.noModelsHint";
  if (s.someoneGenerating && !s.generating) return "workshop.generatingElsewhere";
  return null;
}

function ProcessTopBar({ t }: { t: (key: string) => string }) {
  return (
    <div className="flex flex-none items-center gap-3.5 border-b border-line pb-4">
      <div className="min-w-0 flex-1">
        <div className="font-mono text-[11px] font-semibold tracking-[0.14em] text-ink-muted uppercase">
          {t("workshop.processCrumb")}
        </div>
        <h1 className="mt-0.75 truncate font-serif text-[21px] font-medium text-ink">
          {t("workshop.processTitle")}
        </h1>
      </div>
      <Tag tone="ai" className="flex-none">
        <Icon name="spark" size={12} /> AI {t("workshop.assist")} · Ollama
      </Tag>
    </div>
  );
}

// チャット行(左にアバター、右に本文)。ai=true は AI 側。
function ChatRow({ ai, children }: { ai?: boolean; children: ReactNode }) {
  return (
    <div className="flex items-start gap-3">
      <div
        className="grid size-8.5 flex-none place-items-center rounded-[10px] text-white shadow-(--shadow-sm)"
        style={{ background: ai ? "var(--ai)" : "var(--brand)" }}
      >
        <Icon name={ai ? "spark" : "edit"} size={16} />
      </div>
      <div className="min-w-0 flex-1 pt-0.5">{children}</div>
    </div>
  );
}
