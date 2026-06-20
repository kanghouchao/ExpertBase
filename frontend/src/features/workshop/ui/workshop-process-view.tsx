"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { EmptyState } from "@/shared/ui/empty-state";
import { Icon } from "@/shared/ui/icon";
import { Tag } from "@/shared/ui/tag";
import { Button, buttonVariants } from "@/shared/ui/button";
import { useI18n } from "@/shared/providers/providers";
import {
  aiHasKey,
  listOllamaModels,
  listInbox,
  readInboxMaterial,
  workshopConfirm,
  workshopDraft,
  type ChatTurn,
  type InboxItem,
  type OllamaModel,
  type StructureResult,
} from "@/shared/api/tauri/client";
import { inboxToMaterial, RAW_TYPE, type RawMaterial } from "@/entities/material";
import { useKbStore } from "@/entities/knowledge-base";

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

const PREVIEW_RAW = `……所以你看，杀青的温度其实没有一个死数字，要看茶青的含水量。手摸下去，第一遍要“高温杀透”，让它快速失水；第二遍才看香气和叶色。`;

// Tauri 外プレビューで「+」素材選択を見せるための候補プール（!available 時のみ）。
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

const PREVIEW_MODELS: OllamaModel[] = [{ name: "qwen3:8b" }, { name: "llama3.1:8b" }];

// プレビュー（Tauri 外）の加工パネルに見せる素材。入力素材から組み立てるだけで、
// AI 生成結果（関連リンク等）は空のまま——出力は偽装しない。
const PREVIEW_DRAFT: StructureResult = {
  kind: "entry",
  title: PREVIEW_SOURCE.title,
  cat: "",
  bodyMarkdown: PREVIEW_RAW,
  suggestedLinks: [],
};

// 会話メッセージ。ユーザー発話 or AI 応答（草稿 entry / 会話返信 chat）。
type Msg =
  | { role: "user"; text: string; sources?: RawMaterial[] }
  | { role: "ai"; result: StructureResult };

function toTurn(m: Msg): ChatTurn {
  return m.role === "user"
    ? { role: "user", content: m.text }
    : { role: "assistant", content: m.result.bodyMarkdown };
}

export function WorkshopProcessView() {
  const { t } = useI18n();
  const router = useRouter();
  const params = useSearchParams();
  const { available } = useKbStore();
  const inboxPath = params.get("path") ?? "";

  const [sources, setSources] = useState<RawMaterial[]>([]);
  const [rawByPath, setRawByPath] = useState<Record<string, string>>({});
  const [inbox, setInbox] = useState<InboxItem[]>([]);
  const [showPicker, setShowPicker] = useState(false);
  const [instruction, setInstruction] = useState("");
  const [hasOllama, setHasOllama] = useState(false);
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [selectedModel, setSelectedModel] = useState("");
  const [messages, setMessages] = useState<Msg[]>([]);
  const [draft, setDraft] = useState<StructureResult | null>(null);
  const [generating, setGenerating] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const threadEnd = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!inboxPath || !available) return;
    void (async () => {
      setError(null);
      try {
        const [inboxList, raw] = await Promise.all([listInbox(), readInboxMaterial(inboxPath)]);
        setInbox(inboxList);
        const found = inboxList.find((candidate) => candidate.path === inboxPath) ?? null;
        setSources(found ? [materialFromInbox(found)] : []);
        setRawByPath({ [inboxPath]: raw });
        setMessages([]);
        setDraft({
          kind: "entry",
          title: found ? materialFromInbox(found).title : "",
          cat: "",
          bodyMarkdown: stripFrontmatter(raw),
          suggestedLinks: [],
        });
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
  }, [messages, generating, draft]);

  const visibleSources = available ? sources : [PREVIEW_SOURCE];
  const visibleHasOllama = available ? hasOllama : true;
  const visibleModels = available ? models : PREVIEW_MODELS;
  const visibleSelectedModel = available ? selectedModel : selectedModel || PREVIEW_MODELS[0].name;
  // 追加可能な素材プール = 未処理の inbox − すでに選択済みのもの。
  const pool = inbox
    .filter((candidate) => candidate.status !== "processed")
    .map(materialFromInbox)
    .filter((candidate) => !sources.some((s) => s.id === candidate.id));
  const visiblePool = available ? pool : PREVIEW_POOL;
  const canGenerate =
    visibleSources.length > 0 && visibleHasOllama && !!visibleSelectedModel && !generating;

  async function addSource(material: RawMaterial) {
    setShowPicker(false);
    if (!available || sources.some((s) => s.id === material.id)) return;
    setSources((prev) => [...prev, material]);
    if (!rawByPath[material.id]) {
      try {
        const raw = await readInboxMaterial(material.id);
        setRawByPath((prev) => ({ ...prev, [material.id]: raw }));
      } catch {
        /* 本文取得の失敗は致命的でない（カードは material.preview にフォールバック）。 */
      }
    }
  }

  function removeSource(id: string) {
    if (!available) return;
    setSources((prev) => prev.filter((s) => s.id !== id));
  }

  // 会話履歴つきで AI を呼ぶ。entry なら草稿を更新、chat なら会話気泡として表示。
  async function runTurn(history: Msg[]) {
    setMessages(history);
    setInstruction("");
    setShowPicker(false);
    setGenerating(true);
    setError(null);
    try {
      const result = await workshopDraft(
        sources.map((s) => s.id),
        history.map(toTurn),
        visibleSelectedModel
      );
      setMessages([...history, { role: "ai", result }]);
      if (result.kind === "entry") setDraft(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      // 失敗したターンは履歴から外し、入力を戻して再試行できるようにする。
      const last = history[history.length - 1];
      setMessages(history.slice(0, -1));
      if (last?.role === "user") setInstruction(last.text);
    } finally {
      setGenerating(false);
    }
  }

  async function handleSend() {
    if (!canGenerate) return;
    const userMsg: Msg =
      messages.length === 0
        ? { role: "user", text: instruction.trim(), sources: visibleSources }
        : { role: "user", text: instruction.trim() };
    void runTurn([...messages, userMsg]);
  }

  // 末尾の AI 応答を捨てて、直前のユーザー発話まででやり直す。
  function handleRegenerate() {
    if (!canGenerate || messages.length === 0) return;
    const last = messages[messages.length - 1];
    void runTurn(last.role === "ai" ? messages.slice(0, -1) : messages);
  }

  async function handleConfirm() {
    if (sources.length === 0 || !draft?.title.trim() || !draft.bodyMarkdown.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await workshopConfirm({
        inboxPaths: sources.map((s) => s.id),
        title: draft.title,
        cat: draft.cat,
        body: draft.bodyMarkdown,
      });
      router.push("/workshop");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  const notFound = !inboxPath || (visibleSources.length === 0 && !error);
  // 会話に出た最後の entry 草稿だけを編集可能にする（それより前は読み取り専用スナップ）。
  const lastEntryIdx = messages.reduce(
    (acc, m, i) => (m.role === "ai" && m.result.kind === "entry" ? i : acc),
    -1
  );
  // Ollama が無いときは会話せず、素材から起こした草稿を直接編集して確定する手動経路。
  const showManualDraft = !visibleHasOllama && !!draft && messages.length === 0;
  const canConfirm =
    sources.length > 0 && !!draft?.title.trim() && !!draft?.bodyMarkdown.trim() && !busy;
  // 実データの draft が無いプレビューでは、パネルの体裁だけ見せる（確認は無効）。
  const visibleDraft = draft ?? (available ? null : PREVIEW_DRAFT);

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

      {/* lg 以上は全高 2 カラム（会話は内部スクロール）、それ未満は 1 カラムでページ全体スクロール。 */}
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

              {/* 会話（多輪）。AI entry は草稿カード、AI chat は会話気泡。 */}
              {messages.map((m, i) =>
                m.role === "user" ? (
                  <div key={i} className="flex justify-end">
                    <div className="flex max-w-[84%] flex-col items-end gap-2.5">
                      {m.sources && m.sources.length > 0 && (
                        <div className="flex w-full flex-col gap-2.5">
                          {m.sources.map((s) => (
                            <SourceCard key={s.id} material={s} source={rawByPath[s.id] ?? ""} />
                          ))}
                        </div>
                      )}
                      {m.text ? (
                        <div className="rounded-[14px_14px_4px_14px] bg-brand px-4 py-2.5 text-[14px] leading-relaxed text-white">
                          {m.text}
                        </div>
                      ) : (
                        <div className="font-mono text-[12.5px] text-ink-faint">
                          {t("workshop.attached")}
                        </div>
                      )}
                    </div>
                  </div>
                ) : m.result.kind === "chat" ? (
                  <ChatRow key={i} ai>
                    <div className="text-[14.5px] leading-relaxed whitespace-pre-wrap text-ink-soft">
                      {m.result.bodyMarkdown}
                    </div>
                  </ChatRow>
                ) : (
                  <ChatRow key={i} ai>
                    {i === lastEntryIdx && draft ? (
                      <DraftCard
                        draft={draft}
                        done
                        onChange={setDraft}
                        onRegenerate={handleRegenerate}
                        canRegenerate={canGenerate}
                        busy={busy}
                      />
                    ) : (
                      <DraftCard draft={m.result} done onChange={() => {}} readOnly />
                    )}
                  </ChatRow>
                )
              )}

              {/* AI: 生成中 */}
              {generating && (
                <ChatRow ai>
                  <div className="flex items-center gap-2.5 text-[13.5px] text-ink-soft">
                    <span className="size-4 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
                    {t("workshop.thinking")}
                  </div>
                </ChatRow>
              )}

              {/* 手動経路（Ollama 無し）: 素材から起こした草稿を直接編集して確定 */}
              {showManualDraft && draft && (
                <ChatRow ai>
                  <DraftCard draft={draft} done={false} onChange={setDraft} readOnly={false} />
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
              {visibleSources.length > 0 && !generating && (
                <div className="mb-2.5 flex flex-wrap gap-2">
                  {visibleSources.map((m) => (
                    <SourceChip
                      key={m.id}
                      material={m}
                      onRemove={visibleSources.length > 1 ? () => removeSource(m.id) : undefined}
                    />
                  ))}
                </div>
              )}
              <textarea
                value={instruction}
                onChange={(event) => setInstruction(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                    event.preventDefault();
                    void handleSend();
                  }
                }}
                placeholder={t("workshop.composerPh")}
                rows={2}
                className="w-full resize-none bg-transparent px-1 text-[14.5px] leading-relaxed text-ink outline-none"
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
                          {model.name}
                        </option>
                      ))
                    )}
                  </select>
                  <Icon name="chevD" size={14} className="text-ink-muted" />
                </div>
                <div className="flex-1" />
                <Button
                  className="h-9 bg-brand px-5 text-white hover:bg-brand/85"
                  disabled={!canGenerate}
                  onClick={handleSend}
                >
                  <Icon name="send" size={15} />
                  {generating ? t("workshop.running") : t("workshop.send")}
                </Button>
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
            <div className="mt-2 text-center font-mono text-[11px] text-ink-faint">
              ⌘↵ {t("workshop.send")}
            </div>
          </div>
        </div>

        {/* ── インスペクタ（実データのみ） ── */}
        {visibleDraft && (
          <Inspector
            model={visibleSelectedModel}
            generating={generating}
            draft={visibleDraft}
            canConfirm={canConfirm}
            busy={busy}
            onConfirm={handleConfirm}
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

function DraftCard({
  draft,
  done,
  onChange,
  onRegenerate,
  canRegenerate,
  busy,
  readOnly = false,
}: {
  draft: StructureResult;
  done: boolean;
  onChange: (draft: StructureResult) => void;
  onRegenerate?: () => void;
  canRegenerate?: boolean;
  busy?: boolean;
  readOnly?: boolean;
}) {
  const { t } = useI18n();
  return (
    <div className="overflow-hidden rounded-xl border border-ai-soft bg-surface shadow-(--shadow-md)">
      <div className="flex items-center gap-2 border-b border-line px-5 py-3 font-mono text-[11.5px] font-bold text-ink-muted">
        {done ? t("workshop.draftReady") : t("workshop.draftTitle")}
      </div>
      <div className="px-5 py-4">
        <input
          value={draft.title}
          onChange={(event) => onChange({ ...draft, title: event.target.value })}
          readOnly={readOnly}
          placeholder={t("workshop.titleField")}
          className="w-full bg-transparent font-serif text-[22px] font-semibold text-ink outline-none"
        />
        <input
          value={draft.cat}
          onChange={(event) => onChange({ ...draft, cat: event.target.value })}
          readOnly={readOnly}
          placeholder={t("workshop.catField")}
          className="mt-2 w-full rounded-lg border border-line-strong bg-surface-2 px-3 py-2 text-[13px] text-ink outline-none"
        />
        <textarea
          value={draft.bodyMarkdown}
          onChange={(event) => onChange({ ...draft, bodyMarkdown: event.target.value })}
          readOnly={readOnly}
          placeholder={t("workshop.prepareSub")}
          className="mt-3.5 min-h-64 w-full resize-y rounded-lg border border-line-strong bg-surface-2 p-3.5 font-mono text-[13.5px] leading-relaxed text-ink outline-none"
        />
        {draft.suggestedLinks.length > 0 && (
          <div className="mt-4 rounded-xl bg-ai-wash p-3.5">
            <div className="mb-2 flex items-center gap-1.5 text-[12px] font-bold text-ai">
              <Icon name="link" size={14} />
              {t("workshop.suggestedLinks")}
            </div>
            <div className="flex flex-wrap gap-1.5">
              {draft.suggestedLinks.map((link) => (
                <Tag key={link} tone="ai">
                  [[{link}]]
                </Tag>
              ))}
            </div>
          </div>
        )}
      </div>
      {!readOnly && (
        <div className="flex items-center gap-2.5 border-t border-line p-3.5">
          <Button variant="outline" disabled={busy || !canRegenerate} onClick={onRegenerate}>
            <Icon name="refresh" size={15} />
            {t("workshop.regen")}
          </Button>
          <span className="text-[12px] leading-snug text-ink-faint">{t("workshop.draftHint")}</span>
        </div>
      )}
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

function SourceCard({ material, source }: { material: RawMaterial; source: string }) {
  const type = RAW_TYPE[material.type];
  const visibleSource = stripFrontmatter(source) || material.preview;
  return (
    <div className="overflow-hidden rounded-xl border border-line bg-surface">
      <div className="flex items-center gap-3 border-b border-line px-4 py-3">
        <span
          className="grid size-8 flex-none place-items-center rounded-lg bg-surface-2"
          style={{ color: type.color }}
        >
          <Icon name={type.icon} size={16} />
        </span>
        <div className="min-w-0">
          <div className="truncate text-[14px] font-bold text-ink">{material.title}</div>
          <div className="mt-0.5 truncate font-mono text-[11.5px] text-ink-faint">
            {material.source} · {material.date}
          </div>
        </div>
      </div>
      <div className="max-h-40 overflow-auto px-4 py-3 text-[13px] leading-relaxed whitespace-pre-wrap text-ink-soft">
        {visibleSource}
      </div>
    </div>
  );
}

// 右側の加工パネル。実際に得られる情報だけを映す：選択モデル・出力先・
// 生成後の suggestedLinks・確定アクション。品質スコアや行差分のような
// バックエンドが返さない指標は出さない（AI 出力を偽装しない方針）。
function Inspector({
  model,
  generating,
  draft,
  canConfirm,
  busy,
  onConfirm,
}: {
  model: string;
  generating: boolean;
  draft: StructureResult;
  canConfirm: boolean;
  busy: boolean;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  const status = generating
    ? { label: t("workshop.running"), color: "var(--gold)" }
    : canConfirm
      ? { label: t("workshop.st.done"), color: "var(--ai)" }
      : { label: t("workshop.st.idle"), color: "var(--ink-muted)" };
  const slug = (draft.title || "untitled")
    .trim()
    .toLowerCase()
    .replace(/\.md$/, "")
    .replace(/\s+/g, "-");
  const linksReady = !generating && draft.suggestedLinks.length > 0;

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
              <div className="text-[11px] text-ink-muted">{t("workshop.insp.modelSub")}</div>
            </div>
          </div>
        </InspRow>

        <InspRow label={t("workshop.insp.target")}>
          <div className="flex items-baseline gap-2">
            <span className="truncate font-serif text-[17px] font-semibold text-ink">
              {draft.title || t("workshop.titleField")}
            </span>
          </div>
          <div className="mt-2 flex items-center gap-2">
            <Icon name="doc" size={13} className="flex-none text-ink-faint" />
            <span className="truncate font-mono text-[11.5px] text-ink-muted">
              {`wiki/${slug}.md`}
            </span>
            <Tag tone="ai" className="flex-none">
              {t("workshop.insp.new")}
            </Tag>
          </div>
          {draft.cat && (
            <div className="mt-2">
              <Tag tone="accent">{draft.cat}</Tag>
            </div>
          )}
        </InspRow>

        <InspRow label={t("workshop.suggestedLinks")}>
          {linksReady ? (
            <div className="flex flex-wrap gap-1.5">
              {draft.suggestedLinks.map((link) => (
                <Tag key={link} tone="ai">
                  [[{link}]]
                </Tag>
              ))}
            </div>
          ) : (
            <span className="text-[12.5px] text-ink-faint">{t("workshop.insp.linksPending")}</span>
          )}
        </InspRow>

        {!generating && !canConfirm && (
          <p className="mt-3.5 text-[12px] leading-relaxed text-ink-faint">
            {t("workshop.insp.hint")}
          </p>
        )}
      </div>

      <div className="mt-auto border-t border-line p-3.5">
        <Button className="w-full justify-center" disabled={!canConfirm} onClick={onConfirm}>
          <Icon name="check" size={15} />
          {busy ? t("workshop.running") : t("workshop.confirm")}
        </Button>
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

function stripFrontmatter(markdown: string): string {
  if (!markdown.startsWith("---")) return markdown.trim();
  const end = markdown.indexOf("\n---", 3);
  if (end === -1) return markdown.trim();
  return markdown.slice(end + 4).trim();
}
