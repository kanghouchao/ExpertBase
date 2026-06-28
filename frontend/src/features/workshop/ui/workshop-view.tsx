"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";

import { EmptyState } from "@/shared/ui/empty-state";
import { Icon } from "@/shared/ui/icon";
import { Tag } from "@/shared/ui/tag";
import { Button } from "@/shared/ui/button";
import { Markdown } from "@/shared/ui/markdown";
import { PageHead } from "@/shared/ui/page-head";
import { DeleteButton } from "@/shared/ui/delete-button";
import { cn } from "@/shared/lib/utils";
import { useI18n } from "@/shared/providers/providers";
import {
  aiHasKey,
  listOllamaModels,
  listInbox,
  deleteInboxMaterial,
  pickLocalFile,
  workshopCancel,
  workshopChat,
  type ChatPhase,
  type InboxItem,
  type OllamaModel,
} from "@/shared/api/tauri/client";
import { inboxToMaterial, RAW_TYPE, type RawMaterial, type RawType } from "@/entities/material";
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

// Tauri 外（静的プレビュー）で選択リストの形だけ見せる素材。AI 出力は偽装しない。
const PREVIEW_MATERIALS: RawMaterial[] = [
  {
    id: "inbox/tea-master.md",
    type: "audio",
    title: "与制茶师傅的访谈录音",
    source: "录音",
    date: "02:14:50 · 2 小时前",
    status: "transcribed",
    size: "",
    preview: "",
    words: 0,
    tags: [],
  },
  {
    id: "inbox/whitepaper.md",
    type: "pdf",
    title: "2024 普洱仓储白皮书.pdf",
    source: "PDF",
    date: "42 页 · 昨天",
    status: "pending",
    size: "",
    preview: "",
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
    preview: "",
    words: 0,
    tags: [],
  },
  {
    id: "inbox/mountain.md",
    type: "audio",
    title: "语音备忘：勐海茶山见闻",
    source: "录音",
    date: "08:31 · 上周",
    status: "transcribed",
    size: "",
    preview: "",
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

export function WorkshopView() {
  const { t } = useI18n();
  const { available } = useKbStore();

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
  // 「新しい対話」押下フラグ。進行中ターンの解決が選択態をクリアした状態を上書きしないようにする。
  const resetRef = useRef(false);

  // 受信箱を読み込み、選択リストの母数にする（path 引数は廃止）。
  useEffect(() => {
    if (!available) return;
    void (async () => {
      setError(null);
      try {
        setInbox(await listInbox());
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    })();
  }, [available]);

  useEffect(() => {
    if (!available) return;
    void (async () => {
      try {
        const [ollama, modelList] = await Promise.all([aiHasKey(), listOllamaModels()]);
        setHasOllama(ollama);
        setModels(modelList);
        // 既定モデルの nudge: Qwen3（2026 本地工具调用最稳）→ 任意の tools 対応 → 先頭。
        setSelectedModel((current) =>
          current && modelList.some((model) => model.name === current)
            ? current
            : (modelList.find((model) => /qwen3/i.test(model.name))?.name ??
              modelList.find((model) => model.tools)?.name ??
              modelList[0]?.name ??
              "")
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

  const visibleSources = sources;
  const visibleHasOllama = available ? hasOllama : true;
  const visibleModels = available ? models : PREVIEW_MODELS;
  const visibleSelectedModel = available ? selectedModel : selectedModel || PREVIEW_MODELS[0].name;
  const selectedModelInfo = visibleModels.find((model) => model.name === visibleSelectedModel);
  const selectedThinking = selectedModelInfo?.thinking ?? false;
  const selectedTools = selectedModelInfo?.tools ?? false;

  // 選択リストの母数 = 未処理の inbox。
  const pending = inbox
    .filter((candidate) => candidate.status !== "processed")
    .map(materialFromInbox);
  const visiblePending = available ? pending : PREVIEW_MATERIALS;

  // 「＋」追加プール = 未処理 inbox − すでに選択済み。
  const pool = pending.filter((candidate) => !sources.some((s) => s.id === candidate.id));
  const visiblePool = available
    ? pool
    : PREVIEW_MATERIALS.filter((candidate) => !sources.some((s) => s.id === candidate.id));

  // 工作坊は tools 対応モデル必須（素材を read_source で読む・条目を write_entry で書くため）。
  const canGenerate =
    visibleSources.length > 0 &&
    visibleHasOllama &&
    !!visibleSelectedModel &&
    selectedTools &&
    !generating;

  // 選択リスト/チップで素材をトグル選択（純ローカル状態。プレビューでも動く）。
  function toggleSource(material: RawMaterial) {
    setSources((current) =>
      current.some((s) => s.id === material.id)
        ? current.filter((s) => s.id !== material.id)
        : [...current, material]
    );
  }

  // 外部のローカルファイルを素材に追加する（id は絶対パス。AI が read_source で読む、KB へは落とさない）。
  async function addLocalFile() {
    setShowPicker(false);
    const path = await pickLocalFile();
    if (!path) return;
    const material = materialFromFile(path, t("workshop.addLocalFile"));
    setSources((current) =>
      current.some((s) => s.id === material.id) ? current : [...current, material]
    );
  }

  // 受信箱から素材を削除（選択中なら外す）。DeleteButton の 2 段階確認で直接削除する。
  async function handleDelete(path: string) {
    setError(null);
    try {
      await deleteInboxMaterial(path);
      setSources((current) => current.filter((s) => s.id !== path));
      setInbox(await listInbox());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  // 対話を終えて選択態へ戻す（「新しい対話」）。状態は揮発的なので破棄でよい。
  function reset() {
    // 生成中でも押せる。後端を止め、進行中ターンの解決でこのクリアが上書きされないよう resetRef を立てる。
    resetRef.current = true;
    void workshopCancel();
    setMessages([]);
    setSources([]);
    setInstruction("");
    setShowPicker(false);
    setPhase("idle");
    setError(null);
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
    resetRef.current = false;
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
      // 「新しい対話」で破棄済みなら、解決結果で選択態を上書きしない。
      if (resetRef.current) return;
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
      // 「新しい対話」で破棄済みなら、履歴復元も入力復元もしない。
      if (resetRef.current) return;
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

  return (
    <div className="view-enter flex flex-col lg:h-full">
      {messages.length === 0 ? (
        <PageHead
          eyebrow={t("workshop.eyebrow")}
          title={t("workshop.title")}
          sub={t("workshop.listSub")}
        />
      ) : (
        <ProcessTopBar t={t} onReset={reset} />
      )}

      {/* lg 以上は 2 カラム（会話は内部スクロール）、それ未満は 1 カラムでページ全体スクロール。 */}
      <div className="flex flex-col gap-5 pt-5 lg:min-h-0 lg:flex-1 lg:flex-row">
        {/* ── 会話列 ── */}
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="lg:min-h-0 lg:flex-1 lg:overflow-auto">
            <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-1 py-1">
              {/* 選択態：可多選の待加工素材リスト（会話開始前のみ）。選択は下のチップへ反映。 */}
              {messages.length === 0 && (
                <MaterialSelect
                  t={t}
                  materials={visiblePending}
                  selectedIds={visibleSources.map((s) => s.id)}
                  onToggle={toggleSource}
                  onDelete={available ? handleDelete : undefined}
                />
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
                          ? () => toggleSource(m)
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
                    disabled={generating}
                    title={t("workshop.addMaterial")}
                    className="grid size-9 place-items-center rounded-[10px] border border-line-strong bg-surface text-ink-soft transition-colors hover:bg-surface-2 disabled:opacity-40"
                  >
                    <Icon name="plus" size={18} />
                  </button>
                  {showPicker && (
                    <div className="absolute bottom-[calc(100%+9px)] left-0 z-30 w-[300px] rounded-xl border border-line bg-surface p-1.5 shadow-(--shadow-lg)">
                      <div className="px-2 py-1.5 font-mono text-[10.5px] font-bold tracking-[0.1em] text-ink-muted uppercase">
                        {t("workshop.addMaterial")}
                      </div>
                      {/* 外部ローカルファイルを追加（OS のファイル選択ダイアログ）。 */}
                      <button
                        type="button"
                        onClick={() => void addLocalFile()}
                        className="flex w-full items-center gap-2.5 rounded-lg px-2 py-2 text-left transition-colors hover:bg-surface-2"
                      >
                        <span className="grid size-7 flex-none place-items-center rounded-md bg-surface-2 text-ink-soft">
                          <Icon name="plus" size={14} />
                        </span>
                        <span className="min-w-0 flex-1 truncate text-[13px] font-semibold text-ink">
                          {t("workshop.addLocalFile")}
                        </span>
                      </button>
                      {visiblePool.map((m) => {
                        const poolType = RAW_TYPE[m.type];
                        return (
                          <button
                            key={m.id}
                            type="button"
                            onClick={() => toggleSource(m)}
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
              {/* tools 非対応モデルでは送信不可＝素材読み取り/書き込みが回らない。理由を提示する。 */}
              {visibleHasOllama && visibleModels.length > 0 && visibleSelectedModel && !selectedTools && (
                <div className="mt-2 px-1 text-[12px] text-ink-faint">
                  {t("workshop.toolsRequired")}
                </div>
              )}
              {error && (
                <div className="mt-2 px-1 text-[12.5px] font-semibold text-brand">{error}</div>
              )}
            </div>
          </div>
        </div>

        {/* ── 右側ステータス（対話態のみ。実行状態 + モデル + 在席素材。草稿は出さない） ── */}
        {messages.length > 0 && (
          <Inspector
            model={visibleSelectedModel}
            generating={generating}
            runningLabel={t(runningLabelKey(phase, selectedThinking))}
            thinking={selectedThinking}
            tools={selectedTools}
            sources={visibleSources}
          />
        )}
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

// 外部ローカルファイルを素材チップへ。id は絶対パス、拡張子で表示型を決める（pdf/doc/その他=note）。
function materialFromFile(path: string, label: string): RawMaterial {
  const name = path.split(/[\\/]/).pop() || path;
  const ext = name.includes(".") ? (name.split(".").pop() ?? "").toLowerCase() : "";
  const type: RawType = ext === "pdf" ? "pdf" : ext === "docx" || ext === "doc" ? "doc" : "note";
  return {
    id: path,
    type,
    title: name,
    source: label,
    date: "",
    status: "pending",
    size: "",
    preview: "",
    words: 0,
    tags: [],
  };
}

// 選択態の素材リスト。行クリックで選択トグル、右の DeleteButton で受信箱から削除。
function MaterialSelect({
  t,
  materials,
  selectedIds,
  onToggle,
  onDelete,
}: {
  t: (key: string) => string;
  materials: RawMaterial[];
  selectedIds: string[];
  onToggle: (material: RawMaterial) => void;
  onDelete?: (path: string) => void;
}) {
  if (materials.length === 0) {
    return (
      <div className="rounded-xl border border-line bg-surface shadow-(--shadow-sm)">
        <EmptyState
          icon="layers"
          title={t("workshop.selectEmpty")}
          sub={t("workshop.selectEmptyHint")}
        />
      </div>
    );
  }
  return (
    <div>
      <div className="mb-2.5 font-mono text-[10.5px] font-bold tracking-[0.1em] text-ink-muted uppercase">
        {t("workshop.pendingMaterials")} · {materials.length}
      </div>
      <div className="grid grid-cols-2 gap-2.5">
        {materials.map((m) => {
          const type = RAW_TYPE[m.type];
          const selected = selectedIds.includes(m.id);
          return (
            <div
              key={m.id}
              role="button"
              tabIndex={0}
              onClick={() => onToggle(m)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onToggle(m);
                }
              }}
              className={cn(
                "flex cursor-pointer items-center gap-3 rounded-xl border bg-surface px-4 py-3 shadow-(--shadow-sm) transition-colors",
                selected ? "border-brand bg-brand/5" : "border-line hover:bg-surface-2"
              )}
            >
              <span
                className={cn(
                  "grid size-5 flex-none place-items-center rounded-md border",
                  selected
                    ? "border-brand bg-brand text-white"
                    : "border-line-strong text-transparent"
                )}
              >
                <Icon name="check" size={13} />
              </span>
              <span
                className="grid size-9 flex-none place-items-center rounded-lg bg-surface-2"
                style={{ color: type.color }}
              >
                <Icon name={type.icon} size={16} />
              </span>
              <div className="min-w-0 flex-1">
                <div className="truncate text-[14px] font-semibold text-ink">{m.title}</div>
                <div className="truncate font-mono text-[11px] text-ink-faint">
                  {m.source} · {m.date}
                </div>
              </div>
              {onDelete && (
                <span onClick={(e) => e.stopPropagation()}>
                  <DeleteButton onDelete={() => onDelete(m.id)} />
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ProcessTopBar({ t, onReset }: { t: (key: string) => string; onReset: () => void }) {
  return (
    <div className="flex flex-none items-center gap-3.5 border-b border-line pb-4">
      <Button variant="outline" size="lg" onClick={onReset}>
        <Icon name="chevL" size={15} />
        {t("workshop.newChat")}
      </Button>
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

// エージェントのツール呼び出し 1 件のカード（検索・読み取り・書き込み）。args は JSON 文字列なので値だけ抜いて表示。
// 結果（read_source なら素材本文）は長くなるので折りたたみ、ヘッダは浅色。クリックで展開する。
function ToolCallCard({ tool }: { tool: ToolEvent }) {
  const [open, setOpen] = useState(false);
  let argText = tool.args;
  try {
    const parsed = JSON.parse(tool.args);
    argText = Object.values(parsed)
      .map((v) => String(v))
      .join(", ");
  } catch {
    /* JSON でなければ生文字列のまま表示 */
  }
  const icon = tool.name === "search_kb" ? "search" : "doc";
  const hasResult = Boolean(tool.summary);
  return (
    <div className="overflow-hidden rounded-lg border border-line bg-surface-2 text-[12.5px]">
      <button
        type="button"
        onClick={() => hasResult && setOpen((prev) => !prev)}
        className={cn(
          "flex w-full items-center gap-2 px-3 py-1.5 text-left text-ink-faint",
          hasResult ? "cursor-pointer" : "cursor-default"
        )}
      >
        {hasResult && <Icon name={open ? "chevD" : "chevR"} size={12} className="flex-none" />}
        <Icon name={icon} size={13} className="flex-none text-ai" />
        <span className="font-mono font-semibold text-ink-soft">{tool.name}</span>
        {argText && <span className="truncate">{argText}</span>}
      </button>
      {open && hasResult && (
        <div className="max-h-48 overflow-auto border-t border-line px-3 py-2 font-mono text-[12px] leading-relaxed whitespace-pre-wrap break-words text-ink-soft">
          {tool.summary}
        </div>
      )}
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
