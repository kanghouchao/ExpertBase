"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { EmptyState } from "@/shared/ui/empty-state";
import { Icon } from "@/shared/ui/icon";
import { Tag } from "@/shared/ui/tag";
import { Button, buttonVariants } from "@/shared/ui/button";
import { Markdown } from "@/shared/ui/markdown";
import { useI18n } from "@/shared/providers/providers";
import {
  aiHasKey,
  listOllamaModels,
  listInbox,
  workshopCancel,
  workshopChat,
  type ChatPhase,
  type InboxItem,
  type OllamaModel,
} from "@/shared/api/tauri/client";
import { inboxToMaterial, RAW_TYPE, type RawMaterial } from "@/entities/material";
import { useKbStore } from "@/entities/knowledge-base";
import {
  canRemoveSource,
  isGeneratingPhase,
  runningLabelKey,
  toChatTurn,
  type ChatUiPhase,
  type ProcessMessage,
  type ToolEvent,
} from "../model/process-state";

// Tauri 外（静的プレビュー）で会話シェルだけを見せるための素材。AI 出力は偽装しない。
const PREVIEW_SOURCE: RawMaterial = {
  id: "inbox/tea-master.md",
  type: "audio",
  title: "与制茶师傅的访谈录音",
  source: "录音",
  date: "02:14:50",
  status: "transcribed",
  size: "",
  preview:
    "……所以你看，杀青的温度其实没有一个死数字，要看茶青的含水量。手摸下去，第一遍要“高温杀透”，让它...",
  words: 0,
  tags: [],
};

// Tauri 外プレビューで「+」素材選択を見せるための候補プール（!available 时のみ）。
const PREVIEW_POOL: RawMaterial[] = [
  {
    id: "inbox/whitepaper.md",
    type: "pdf",
    title: "2024 普洱仓储白皮书.pdf",
    source: "PDF",
    date: "42 页 · 昨天",
    status: "pending",
    size: "",
    preview:
      "本白皮书梳理了干仓与湿仓的温湿度区间、霉变风险阈值，以及不同年份饼茶的转化曲线对照...",
    words: 0,
    tags: [],
  },
  {
    id: "inbox/gaiwan.md",
    type: "video",
    title: "盖碗冲泡手法教学.mov",
    source: "视频",
    date: "11:38 · 3 天前",
    status: "pending",
    size: "",
    preview: "关键帧：注水高度、出汤角度、留根与否的对比演示...",
    words: 0,
    tags: [],
  },
];

const PREVIEW_MODELS: OllamaModel[] = [
  { name: "qwen3:8b", thinking: true, tools: true },
  { name: "llama3.1:8b", thinking: false, tools: false },
];

// 会話メッセージ。ユーザー発話 or AI 応答。
type Msg = ProcessMessage;

export function WorkshopProcessView() {
  const { t } = useI18n();
  const params = useSearchParams();
  const { available } = useKbStore();
  const inboxPath = params.get("path") ?? "";

  const [sources, setSources] = useState<RawMaterial[]>([]);
  const [inbox, setInbox] = useState<InboxItem[]>([]);
  const [showPicker, setShowPicker] = useState(false);
  const [instruction, setInstruction] = useState("");
  const [hasOllama, setHasOllama] = useState(false);
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [selectedModel, setSelectedModel] = useState("");
  const [messages, setMessages] = useState<Msg[]>([]);
  const [phase, setPhase] = useState<ChatUiPhase>("idle");
  const [thinkingBuf, setThinkingBuf] = useState("");
  // 生成中に流れる「AI が今書いている本文」。過程を可視化する。
  const [narrationBuf, setNarrationBuf] = useState("");
  // 生成中のツール呼び出しログ（検索・書き込みなど）。会話にカードで見せる。
  const [toolLog, setToolLog] = useState<ToolEvent[]>([]);
  const generating = isGeneratingPhase(phase);
  const [error, setError] = useState<string | null>(null);
  const threadEnd = useRef<HTMLDivElement>(null);
  const composerRef = useRef<HTMLTextAreaElement>(null);
  // 停止ボタン押下フラグ。中断は失敗扱いせず（赤エラーを出さず）idle へ戻すために使う。
  const cancelRef = useRef(false);

  useEffect(() => {
    if (!inboxPath || !available) return;
    void (async () => {
      setError(null);
      try {
        const inboxList = await listInbox();
        setInbox(inboxList);
        const found = inboxList.find((candidate) => candidate.path === inboxPath) ?? null;
        setSources(found ? [materialFromInbox(found)] : []);
        setMessages([]);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    })();
  }, [available, inboxPath]);

  useEffect(() => {
    if (!available) return;
    void (async () => {
      try {
        const [ollama, modelList] = await Promise.all([aiHasKey(), listOllamaModels()]);
        setHasOllama(ollama);
        setModels(modelList);
        setSelectedModel((current) =>
          current && modelList.some((model) => model.name === current)
            ? current
            : (modelList[0]?.name ?? "")
        );
      } catch {
        setHasOllama(false);
        setModels([]);
        setSelectedModel("");
      }
    })();
  }, [available]);

  // 新しいメッセージが出たら会話を最下部に追従させる。
  useEffect(() => {
    threadEnd.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages, generating]);

  // 入力欄は 1 行から始まり、内容に合わせて自動的に伸びる（送信後は空＝1 行に戻る）。
  useEffect(() => {
    const el = composerRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [instruction]);

  const visibleSources = available ? sources : [PREVIEW_SOURCE];
  const visibleHasOllama = available ? hasOllama : true;
  const visibleModels = available ? models : PREVIEW_MODELS;
  const visibleSelectedModel = available ? selectedModel : selectedModel || PREVIEW_MODELS[0].name;
  const selectedModelInfo = visibleModels.find((model) => model.name === visibleSelectedModel);
  const selectedThinking = selectedModelInfo?.thinking ?? false;
  const selectedTools = selectedModelInfo?.tools ?? false;
  // 追加可能な素材プール = 未処理の inbox − すでに選択済みのもの。
  const pool = inbox
    .filter((candidate) => candidate.status !== "processed")
    .map(materialFromInbox)
    .filter((candidate) => !sources.some((s) => s.id === candidate.id));
  const visiblePool = available ? pool : PREVIEW_POOL;
  const canGenerate =
    visibleSources.length > 0 && visibleHasOllama && !!visibleSelectedModel && !generating;

  function addSource(material: RawMaterial) {
    setShowPicker(false);
    if (!available || sources.some((s) => s.id === material.id)) return;
    setSources((current) => [...current, material]);
  }

  function removeSource(id: string) {
    if (!available || messages.length > 0) return;
    setSources((current) => current.filter((source) => source.id !== id));
  }

  // 会話履歴つきで対話エージェントを 1 ターン回す。思考・ツール・本文を流式表示し、最終返信を会話へ積む。
  async function runTurn(history: Msg[]) {
    setMessages(history);
    setInstruction("");
    setShowPicker(false);
    setPhase("connecting");
    setThinkingBuf("");
    setNarrationBuf("");
    setToolLog([]);
    setError(null);
    cancelRef.current = false;
    try {
      let thinking = "";
      let narration = "";
      const tools: ToolEvent[] = [];
      const reply = await workshopChat(
        sources.map((s) => s.id),
        history.map(toChatTurn),
        visibleSelectedModel,
        selectedThinking,
        selectedTools,
        (p: ChatPhase) => {
          if (p.phase === "narration") {
            setPhase("generating");
            narration += p.delta;
            setNarrationBuf(narration);
          } else if (p.phase === "toolCall") {
            tools.push({ name: p.name, args: p.args });
            setToolLog([...tools]);
          } else if (p.phase === "toolResult") {
            // 直近の同名・未完了の呼び出しに結果サマリを埋める。
            const target = [...tools]
              .reverse()
              .find((tool) => tool.name === p.name && !tool.summary);
            if (target) target.summary = p.summary;
            setToolLog([...tools]);
          } else if (p.phase === "thinking") {
            setPhase("thinking");
            thinking += p.delta;
            setThinkingBuf(thinking);
          } else {
            setPhase("loadingModel");
          }
        }
      );
      setMessages([
        ...history,
        {
          role: "ai",
          text: reply || narration,
          thinking: thinking || undefined,
          tools: tools.length ? tools : undefined,
        },
      ]);
      setPhase("idle");
    } catch (e) {
      // 停止ボタンによる中断はエラー表示しない（ユーザーが意図した中断）。
      if (!cancelRef.current) setError(e instanceof Error ? e.message : String(e));
      // 失敗・中断したターンは履歴から外し、入力を戻して再試行できるようにする。
      const last = history[history.length - 1];
      setMessages(history.slice(0, -1));
      if (last?.role === "user") setInstruction(last.text);
      setPhase("idle");
    }
  }

  // 進行中の生成を止める。後端のフラグを立て、stream が次チャンク前に打ち切る。
  function handleStop() {
    cancelRef.current = true;
    void workshopCancel();
  }

  async function handleSend() {
    if (!canGenerate || !instruction.trim()) return;
    void runTurn([...messages, { role: "user", text: instruction.trim() }]);
  }

  const notFound = !inboxPath || (visibleSources.length === 0 && !error);

  if (notFound) {
    return (
      <div className="view-enter mx-auto flex max-w-3xl flex-col pb-4">
        <ProcessTopBar t={t} />
        <div className="mt-4 rounded-xl border border-line bg-surface shadow-(--shadow-md)">
          <EmptyState icon="shield" title={t("workshop.notFound")} sub={t("workshop.queueHint")} />
        </div>
      </div>
    );
  }

  return (
    <div className="view-enter flex flex-col lg:h-full">
      <ProcessTopBar t={t} />

      {/* lg 以上は 2 カラム（会話は内部スクロール）、それ未満は 1 カラムでページ全体スクロール。 */}
      <div className="flex flex-col gap-5 pt-5 lg:min-h-0 lg:flex-1 lg:flex-row">
        {/* ── 会話列 ── */}
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="lg:min-h-0 lg:flex-1 lg:overflow-auto">
            <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-1 py-1">
              {/* AI 最初のあいさつ（会話開始前のみ）。素材はコンポーザーのチップで示す。 */}
              {messages.length === 0 && (
                <ChatRow ai>
                  <div className="mb-1.5 flex items-center gap-2">
                    <span className="text-[12.5px] font-bold text-ai">{t("workshop.aiName")}</span>
                    <span className="truncate font-mono text-[10.5px] text-ink-faint">
                      {visibleSelectedModel}
                    </span>
                  </div>
                  <div className="text-[14.5px] leading-relaxed text-ink-soft">
                    {t("workshop.chat.greeting")}
                  </div>
                </ChatRow>
              )}

              {/* 会話（多輪）。 */}
              {messages.map((m, i) =>
                m.role === "user" ? (
                  <div key={i} className="flex justify-end">
                    {/* 素材は右パネル/コンポーザーで示すので会話には出さない（履歴で常に文脈に入る）。 */}
                    <div className="max-w-[84%] rounded-[14px_14px_4px_14px] bg-brand px-4 py-2.5 text-[14px] leading-relaxed whitespace-pre-wrap text-white">
                      {m.text}
                    </div>
                  </div>
                ) : (
                  <ChatRow key={i} ai>
                    {m.thinking && <ThinkingPanel text={m.thinking} streaming={false} />}
                    {m.tools && m.tools.length > 0 && <ToolCallLog tools={m.tools} />}
                    {/* AI 出力は Markdown。表示時にレンダリングする（保存は Markdown のまま）。 */}
                    <Markdown className="text-[14.5px] text-ink-soft">{m.text}</Markdown>
                  </ChatRow>
                )
              )}

              {/* AI: 生成中 */}
              {generating && (
                <ChatRow ai>
                  {thinkingBuf && (
                    <ThinkingPanel text={thinkingBuf} streaming={phase === "thinking"} />
                  )}
                  {/* エージェントのツール呼び出し（検索・書き込み）をカードで見せる。 */}
                  {toolLog.length > 0 && (
                    <div className="mb-2.5 flex flex-col gap-1.5">
                      {toolLog.map((tool, idx) => (
                        <ToolCallCard key={idx} tool={tool} />
                      ))}
                    </div>
                  )}
                  {/* 「AI が今書いている本文」を流式表示＝過程が見える（数字ではなく実テキスト）。 */}
                  {narrationBuf && (
                    <div className="mb-2.5 text-[14px] leading-relaxed whitespace-pre-wrap text-ink-soft">
                      {narrationBuf}
                    </div>
                  )}
                  <div className="flex items-center gap-2.5 text-[13.5px] text-ink-soft">
                    <span className="size-4 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
                    {t(runningLabelKey(phase, selectedThinking))}
                  </div>
                </ChatRow>
              )}

              <div ref={threadEnd} />
            </div>
          </div>

          {/* ── サジェスト + コンポーザー ── */}
          <div className="mx-auto mt-3 w-full max-w-3xl">
            {messages.length === 0 && visibleHasOllama && (
              <div className="mb-2.5 flex flex-wrap items-center gap-2">
                <span className="font-mono text-[10.5px] font-bold tracking-[0.1em] text-ink-faint uppercase">
                  {t("workshop.sug.label")}
                </span>
                {[t("workshop.sug.1"), t("workshop.sug.2"), t("workshop.sug.3")].map((s) => (
                  <button
                    key={s}
                    type="button"
                    onClick={() => setInstruction(s)}
                    className="inline-flex items-center gap-1.5 rounded-full border border-line-strong bg-surface px-3 py-1.5 text-[12.5px] font-semibold text-ink-soft shadow-(--shadow-sm) transition-colors hover:bg-surface-2"
                  >
                    <Icon name="spark" size={12} className="text-ai" />
                    {s}
                  </button>
                ))}
              </div>
            )}

            <div className="rounded-[18px] border border-line-strong bg-surface p-3 shadow-(--shadow-md)">
              {/* 関連文档は会話開始前だけ示す。最初の送信で文脈に入るので以降は隠す（+ は常に追加可）。 */}
              {visibleSources.length > 0 && messages.length === 0 && (
                <div className="mb-2.5 flex flex-wrap gap-2">
                  {visibleSources.map((m) => (
                    <SourceChip
                      key={m.id}
                      material={m}
                      onRemove={
                        canRemoveSource(messages.length, visibleSources.length)
                          ? () => removeSource(m.id)
                          : undefined
                      }
                    />
                  ))}
                </div>
              )}
              <textarea
                ref={composerRef}
                value={instruction}
                onChange={(event) => setInstruction(event.target.value)}
                onKeyDown={(event) => {
                  // Enter で送信、Shift+Enter で改行（複数行入力）。
                  if (event.key === "Enter" && !event.shiftKey) {
                    event.preventDefault();
                    void handleSend();
                  }
                }}
                placeholder={t("workshop.composerPh")}
                rows={1}
                className="max-h-40 w-full resize-none overflow-y-auto bg-transparent px-1 text-[14.5px] leading-relaxed text-ink outline-none"
              />
              <div className="mt-2 flex items-center gap-2.5">
                {/* ＋ 素材を追加（未処理 inbox から選ぶ） */}
                <div className="relative flex-none">
                  <button
                    type="button"
                    onClick={() => setShowPicker((prev) => !prev)}
                    disabled={visiblePool.length === 0 || generating}
                    title={t("workshop.addMaterial")}
                    className="grid size-9 place-items-center rounded-[10px] border border-line-strong bg-surface text-ink-soft transition-colors hover:bg-surface-2 disabled:opacity-40"
                  >
                    <Icon name="plus" size={18} />
                  </button>
                  {showPicker && visiblePool.length > 0 && (
                    <div className="absolute bottom-[calc(100%+9px)] left-0 z-30 w-[300px] rounded-xl border border-line bg-surface p-1.5 shadow-(--shadow-lg)">
                      <div className="px-2 py-1.5 font-mono text-[10.5px] font-bold tracking-[0.1em] text-ink-muted uppercase">
                        {t("workshop.addMaterial")}
                      </div>
                      {visiblePool.map((m) => {
                        const poolType = RAW_TYPE[m.type];
                        return (
                          <button
                            key={m.id}
                            type="button"
                            onClick={() => addSource(m)}
                            className="flex w-full items-center gap-2.5 rounded-lg px-2 py-2 text-left transition-colors hover:bg-surface-2"
                          >
                            <span
                              className="grid size-7 flex-none place-items-center rounded-md bg-surface-2"
                              style={{ color: poolType.color }}
                            >
                              <Icon name={poolType.icon} size={14} />
                            </span>
                            <span className="min-w-0 flex-1">
                              <span className="block truncate text-[13px] font-semibold text-ink">
                                {m.title}
                              </span>
                              <span className="block truncate font-mono text-[10.5px] text-ink-faint">
                                {m.source}
                              </span>
                            </span>
                            <Icon name="plus" size={14} className="flex-none text-ink-muted" />
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>
                <div className="flex h-9 min-w-0 max-w-[240px] items-center gap-1.5 rounded-[10px] border border-line-strong bg-surface px-2.5">
                  <Icon name="bot" size={15} className="flex-none text-ai" />
                  <select
                    value={visibleSelectedModel}
                    onChange={(event) => setSelectedModel(event.target.value)}
                    disabled={!visibleHasOllama || visibleModels.length === 0 || generating}
                    className="min-w-0 flex-1 truncate appearance-none bg-transparent font-mono text-[12px] font-semibold text-ink outline-none disabled:opacity-50"
                  >
                    {visibleModels.length === 0 ? (
                      <option value="">{t("workshop.noModels")}</option>
                    ) : (
                      visibleModels.map((model) => (
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
                {generating ? (
                  <Button variant="outline" className="h-9 px-5" onClick={handleStop}>
                    <Icon name="x" size={15} />
                    {t("workshop.stop")}
                  </Button>
                ) : (
                  <Button
                    className="h-9 bg-brand px-5 text-white hover:bg-brand/85"
                    disabled={!canGenerate}
                    onClick={handleSend}
                  >
                    <Icon name="send" size={15} />
                    {t("workshop.send")}
                  </Button>
                )}
              </div>
              {!visibleHasOllama && (
                <div className="mt-2 px-1 text-[12px] text-ink-faint">{t("workshop.noKey")}</div>
              )}
              {visibleHasOllama && visibleModels.length === 0 && (
                <div className="mt-2 px-1 text-[12px] text-ink-faint">
                  {t("workshop.noModelsHint")}
                </div>
              )}
              {error && (
                <div className="mt-2 px-1 text-[12.5px] font-semibold text-brand">{error}</div>
              )}
            </div>
          </div>
        </div>

        {/* ── 右側ステータス（実行状態 + モデル + 在席素材。草稿は出さない） ── */}
        <Inspector
          model={visibleSelectedModel}
          generating={generating}
          runningLabel={t(runningLabelKey(phase, selectedThinking))}
          thinking={selectedThinking}
          tools={selectedTools}
          sources={visibleSources}
        />
      </div>
    </div>
  );
}

function materialFromInbox(item: InboxItem): RawMaterial {
  const material = inboxToMaterial(item);
  return {
    ...material,
    status: item.type === "audio" || item.type === "video" ? "transcribed" : material.status,
    preview: item.source || material.title,
  };
}

function ProcessTopBar({ t }: { t: (key: string) => string }) {
  return (
    <div className="flex flex-none items-center gap-3.5 border-b border-line pb-4">
      <Link className={buttonVariants({ variant: "outline", size: "lg" })} href="/workshop">
        <Icon name="chevL" size={15} />
        {t("nav.workshop")}
      </Link>
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

// チャット行（左にアバター、右に本文）。ai=true は AI 側。
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

// 折りたたみパネル。思考トレース（推論・mono）を表示する。
// streaming 中は自動展開＋底部追従、終わったら自動的に「ラベル · N 字」へ折りたたむ（再展開可）。
function ThinkingPanel({ text, streaming }: { text: string; streaming: boolean }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(streaming);
  const [wasStreaming, setWasStreaming] = useState(streaming);
  const bodyRef = useRef<HTMLDivElement>(null);
  // streaming の切替に追従して自動開閉（レンダー中の状態調整＝React 推奨パターン）。
  if (wasStreaming !== streaming) {
    setWasStreaming(streaming);
    setOpen(streaming);
  }
  // 流式中は本文を最下部に追従させる。
  useEffect(() => {
    if (streaming && bodyRef.current) bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [text, streaming]);
  return (
    <div className="mb-2.5 overflow-hidden rounded-lg border border-ai-soft bg-ai-wash/40">
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left"
      >
        <Icon name={open ? "chevD" : "chevR"} size={13} className="flex-none text-ai" />
        <span className="text-[12px] font-bold text-ai">{t("workshop.think.label")}</span>
        {streaming ? (
          <span className="size-3 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
        ) : (
          <span className="font-mono text-[11px] text-ink-faint">· {text.length}</span>
        )}
      </button>
      {open && (
        <div
          ref={bodyRef}
          className="max-h-48 overflow-auto border-t border-ai-soft px-3 py-2 font-mono text-[12px] leading-relaxed whitespace-pre-wrap text-ink-soft"
        >
          {text}
        </div>
      )}
    </div>
  );
}

// エージェントのツール呼び出し 1 件のカード（検索・書き込み）。args は JSON 文字列なので値だけ抜いて表示。
function ToolCallCard({ tool }: { tool: ToolEvent }) {
  let argText = tool.args;
  try {
    const parsed = JSON.parse(tool.args);
    argText = Object.values(parsed)
      .map((v) => String(v))
      .join(", ");
  } catch {
    /* JSON でなければ生文字列のまま表示 */
  }
  const icon = tool.name === "write_entry" ? "doc" : "search";
  return (
    <div className="inline-flex items-center gap-2 rounded-lg border border-line bg-surface-2 px-3 py-1.5 text-[12.5px]">
      <Icon name={icon} size={13} className="flex-none text-ai" />
      <span className="font-mono font-semibold text-ink">{tool.name}</span>
      {argText && <span className="truncate text-ink-soft">{argText}</span>}
      {tool.summary && <span className="flex-none text-ink-faint">· {tool.summary}</span>}
    </div>
  );
}

// 完成したターンに残すツール呼び出しログ。
function ToolCallLog({ tools }: { tools: ToolEvent[] }) {
  return (
    <div className="mb-2.5 flex flex-col gap-1.5">
      {tools.map((tool, idx) => (
        <ToolCallCard key={idx} tool={tool} />
      ))}
    </div>
  );
}

// 右側のステータス面板。草稿は無くなったので、実行状態・モデル・在席素材だけを映す。
function Inspector({
  model,
  generating,
  runningLabel,
  thinking,
  tools,
  sources,
}: {
  model: string;
  generating: boolean;
  runningLabel: string;
  thinking: boolean;
  tools: boolean;
  sources: RawMaterial[];
}) {
  const { t } = useI18n();
  const status = generating
    ? { label: runningLabel, color: "var(--gold)" }
    : { label: t("workshop.st.idle"), color: "var(--ink-muted)" };
  return (
    <aside className="flex w-full flex-none flex-col overflow-hidden rounded-2xl border border-line bg-surface shadow-(--shadow-sm) lg:w-80">
      <div className="flex items-center gap-2.5 border-b border-line px-4.5 py-3.5">
        <Icon name="layers" size={16} className="text-ai" />
        <div className="text-[13.5px] font-bold text-ink">{t("workshop.insp.title")}</div>
        <div className="flex-1" />
        <span
          className="inline-flex items-center gap-1.5 font-mono text-[12px] font-bold"
          style={{ color: status.color }}
        >
          <span
            className="size-2 rounded-full"
            style={{
              background: status.color,
              animation: generating ? "pulseDot 1s infinite" : "none",
            }}
          />
          {status.label}
        </span>
      </div>

      <div className="min-h-0 flex-1 overflow-auto px-4.5 pb-4">
        <InspRow label={t("workshop.insp.model")} first>
          <div className="flex items-center gap-2.5">
            <span className="grid size-7 flex-none place-items-center rounded-lg bg-ai-wash text-ai">
              <Icon name="bot" size={15} />
            </span>
            <div className="min-w-0">
              <div className="truncate text-[13.5px] font-semibold text-ink">
                {model || "Ollama"}
              </div>
              {(thinking || tools) && (
                <div className="mt-1 flex flex-wrap gap-1">
                  {thinking && <Tag tone="ai">{t("workshop.think.badge")}</Tag>}
                  {tools && <Tag tone="ai">{t("workshop.tools.badge")}</Tag>}
                </div>
              )}
            </div>
          </div>
        </InspRow>

        {sources.length > 0 && (
          <InspRow label={t("workshop.insp.sources")}>
            <div className="flex flex-col gap-2">
              {sources.map((s) => {
                const type = RAW_TYPE[s.type];
                return (
                  <div key={s.id} className="flex items-center gap-2.5">
                    <span
                      className="grid size-7 flex-none place-items-center rounded-lg bg-surface-2"
                      style={{ color: type.color }}
                    >
                      <Icon name={type.icon} size={14} />
                    </span>
                    <div className="min-w-0">
                      <div className="truncate text-[13px] font-semibold text-ink">{s.title}</div>
                      <div className="truncate font-mono text-[10.5px] text-ink-faint">
                        {s.source}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </InspRow>
        )}
      </div>
    </aside>
  );
}

function InspRow({
  label,
  children,
  first = false,
}: {
  label: string;
  children: ReactNode;
  first?: boolean;
}) {
  return (
    <div className={first ? "py-3" : "border-t border-line py-3"}>
      <div className="mb-2 font-mono text-[10.5px] font-bold tracking-[0.1em] text-ink-muted uppercase">
        {label}
      </div>
      {children}
    </div>
  );
}

// コンポーザー内の添付チップ。onRemove があれば × で外せる。
function SourceChip({ material, onRemove }: { material: RawMaterial; onRemove?: () => void }) {
  const type = RAW_TYPE[material.type];
  return (
    <span className="inline-flex max-w-[260px] items-center gap-2 rounded-[9px] border border-line bg-surface-2 py-1.5 pr-2 pl-2.5">
      <span
        className="grid size-5 flex-none place-items-center rounded-md bg-surface"
        style={{ color: type.color }}
      >
        <Icon name={type.icon} size={13} />
      </span>
      <span className="truncate text-[12.5px] font-semibold text-ink">{material.title}</span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          title={material.title}
          className="grid flex-none place-items-center text-ink-faint transition-colors hover:text-ink"
        >
          <Icon name="x" size={14} />
        </button>
      )}
    </span>
  );
}
