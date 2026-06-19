"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { EmptyState } from "@/shared/ui/empty-state";
import { Icon } from "@/shared/ui/icon";
import { PageHead } from "@/shared/ui/page-head";
import { Button, buttonVariants } from "@/shared/ui/button";
import { useI18n } from "@/shared/providers/providers";
import {
  aiHasKey,
  listOllamaModels,
  listInbox,
  readInboxMaterial,
  workshopConfirm,
  workshopDraft,
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
  preview: "……所以你看，杀青的温度其实没有一个死数字，要看茶青的含水量。手摸下去，第一遍要“高温杀透”，让它...",
  words: 0,
  tags: [],
};

const PREVIEW_RAW = `……所以你看，杀青的温度其实没有一个死数字，要看茶青的含水量。手摸下去，第一遍要“高温杀透”，让它快速失水；第二遍才看香气和叶色。`;

const PREVIEW_MODELS: OllamaModel[] = [{ name: "qwen3:8b" }, { name: "llama3.1:8b" }];

type Phase = "idle" | "generating" | "done";

export function WorkshopProcessView() {
  const { t } = useI18n();
  const router = useRouter();
  const params = useSearchParams();
  const { available } = useKbStore();
  const inboxPath = params.get("path") ?? "";

  const [item, setItem] = useState<RawMaterial | null>(null);
  const [source, setSource] = useState("");
  const [instruction, setInstruction] = useState("");
  const [hasOllama, setHasOllama] = useState(false);
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [selectedModel, setSelectedModel] = useState("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [draft, setDraft] = useState<StructureResult | null>(null);
  const [sent, setSent] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const threadEnd = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!inboxPath || !available) return;
    void (async () => {
      setError(null);
      try {
        const [inbox, raw] = await Promise.all([listInbox(), readInboxMaterial(inboxPath)]);
        const found = inbox.find((candidate) => candidate.path === inboxPath) ?? null;
        setItem(found ? materialFromInbox(found) : null);
        setSource(raw);
        setDraft({
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
  }, [phase, sent, draft]);

  const visibleItem = available ? item : PREVIEW_SOURCE;
  const visibleSource = available ? source : PREVIEW_RAW;
  const visibleHasOllama = available ? hasOllama : true;
  const visibleModels = available ? models : PREVIEW_MODELS;
  const visibleSelectedModel = available ? selectedModel : selectedModel || PREVIEW_MODELS[0].name;
  const canGenerate =
    !!inboxPath && visibleHasOllama && !!visibleSelectedModel && phase !== "generating";

  async function handleGenerate() {
    if (!inboxPath || !canGenerate) return;
    setSent(instruction.trim());
    setPhase("generating");
    setError(null);
    try {
      const result = await workshopDraft(inboxPath, instruction, visibleSelectedModel);
      setDraft(result);
      setPhase("done");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase("idle");
    }
  }

  async function handleConfirm() {
    if (!inboxPath || !draft?.title.trim() || !draft.bodyMarkdown.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await workshopConfirm({
        inboxPath,
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

  const notFound = !inboxPath || (!visibleItem && !visibleSource && !error);

  return (
    <div className="view-enter mx-auto flex max-w-3xl flex-col pb-4">
      <div className="mb-3 flex items-center justify-between gap-4">
        <Link className={buttonVariants({ variant: "outline", size: "lg" })} href="/workshop">
          <Icon name="chevL" size={14} />
          {t("nav.workshop")}
        </Link>
        <span className="inline-flex items-center gap-1.5 rounded-full bg-ai-wash px-4 py-2 font-mono text-[12px] font-bold tracking-[0.08em] text-ai">
          <Icon name="spark" size={13} />
          AI {t("workshop.assist")} · {visibleSelectedModel || "Ollama"}
        </span>
      </div>

      <PageHead eyebrow={t("workshop.processCrumb")} title={t("workshop.processTitle")} sub="" />

      {notFound ? (
        <div className="rounded-xl border border-line bg-surface shadow-(--shadow-md)">
          <EmptyState icon="shield" title={t("workshop.notFound")} sub={t("workshop.queueHint")} />
        </div>
      ) : (
        <>
          <div className="flex flex-col gap-6">
            {/* AI からの最初のあいさつ */}
            <ChatRow ai>
              <div className="text-[14.5px] leading-relaxed text-ink-soft">
                {t("workshop.chat.greeting")}
              </div>
              {visibleItem && (
                <div className="mt-3">
                  <SourceCard material={visibleItem} source={visibleSource} />
                </div>
              )}
            </ChatRow>

            {/* 送信後のユーザーメッセージ */}
            {sent !== null && (
              <div className="flex justify-end">
                {sent ? (
                  <div className="max-w-[80%] rounded-[14px_14px_4px_14px] bg-brand px-4 py-2.5 text-[14px] leading-relaxed text-white">
                    {sent}
                  </div>
                ) : (
                  <div className="font-mono text-[12.5px] text-ink-faint">
                    {t("workshop.attached")}
                  </div>
                )}
              </div>
            )}

            {/* AI: 生成中 */}
            {phase === "generating" && (
              <ChatRow ai>
                <div className="flex items-center gap-2.5 text-[13.5px] text-ink-soft">
                  <span className="size-4 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
                  {t("workshop.thinking")}
                </div>
              </ChatRow>
            )}

            {/* AI 草稿または手動編集 */}
            {draft && phase !== "generating" && (
              <ChatRow ai>
                <div className="mb-2.5 flex items-center gap-2 font-mono text-[11.5px] font-bold text-ink-muted">
                  {phase === "done" ? t("workshop.draftReady") : t("workshop.draftTitle")}
                </div>
                <DraftCard
                  draft={draft}
                  onChange={setDraft}
                  onRegenerate={handleGenerate}
                  onConfirm={handleConfirm}
                  canRegenerate={canGenerate}
                  busy={busy}
                />
              </ChatRow>
            )}

            <div ref={threadEnd} />
          </div>

          {/* コンポーザー（下部に固定） */}
          <div className="sticky bottom-0 mt-6 bg-paper/85 pt-3 pb-1 backdrop-blur">
            <div className="rounded-[18px] border border-line-strong bg-surface p-3 shadow-(--shadow-md)">
              {visibleItem && (
                <div className="mb-2.5">
                  <SourceChip material={visibleItem} />
                </div>
              )}
              <textarea
                value={instruction}
                onChange={(event) => setInstruction(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                    event.preventDefault();
                    void handleGenerate();
                  }
                }}
                placeholder={t("workshop.composerPh")}
                rows={2}
                className="w-full resize-none bg-transparent px-1 text-[14.5px] leading-relaxed text-ink outline-none"
              />
              <div className="mt-2 flex items-center gap-2.5">
                <label className="font-mono text-[11.5px] font-bold tracking-[0.06em] text-ink-muted">
                  {t("workshop.model")}
                </label>
                <select
                  value={visibleSelectedModel}
                  onChange={(event) => setSelectedModel(event.target.value)}
                  disabled={!visibleHasOllama || visibleModels.length === 0 || phase === "generating"}
                  className="h-9 min-w-40 rounded-lg border border-line-strong bg-surface px-2.5 font-mono text-[12px] font-semibold text-ink outline-none disabled:opacity-50"
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
                <div className="flex-1" />
                <Button
                  className="h-9 bg-ai px-5 text-white hover:bg-ai/85"
                  disabled={!canGenerate}
                  onClick={handleGenerate}
                >
                  <Icon name="send" size={15} />
                  {phase === "generating" ? t("workshop.running") : t("workshop.send")}
                </Button>
              </div>
              {!visibleHasOllama && (
                <div className="mt-2 px-1 text-[12px] text-ink-faint">{t("workshop.noKey")}</div>
              )}
              {visibleHasOllama && visibleModels.length === 0 && (
                <div className="mt-2 px-1 text-[12px] text-ink-faint">{t("workshop.noModelsHint")}</div>
              )}
              {error && (
                <div className="mt-2 px-1 text-[12.5px] font-semibold text-brand">{error}</div>
              )}
            </div>
            <div className="mt-2 text-center font-mono text-[11px] text-ink-faint">
              ⌘↵ {t("workshop.send")}
            </div>
          </div>
        </>
      )}
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
  onChange,
  onRegenerate,
  onConfirm,
  canRegenerate,
  busy,
}: {
  draft: StructureResult;
  onChange: (draft: StructureResult) => void;
  onRegenerate: () => void;
  onConfirm: () => void;
  canRegenerate: boolean;
  busy: boolean;
}) {
  const { t } = useI18n();
  return (
    <div className="overflow-hidden rounded-xl border border-ai-soft bg-surface shadow-(--shadow-md)">
      <div className="px-5 py-4">
        <input
          value={draft.title}
          onChange={(event) => onChange({ ...draft, title: event.target.value })}
          placeholder={t("workshop.titleField")}
          className="w-full bg-transparent font-serif text-[22px] font-semibold text-ink outline-none"
        />
        <input
          value={draft.cat}
          onChange={(event) => onChange({ ...draft, cat: event.target.value })}
          placeholder={t("workshop.catField")}
          className="mt-2 w-full rounded-lg border border-line-strong bg-surface-2 px-3 py-2 text-[13px] text-ink outline-none"
        />
        <textarea
          value={draft.bodyMarkdown}
          onChange={(event) => onChange({ ...draft, bodyMarkdown: event.target.value })}
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
                <span
                  key={link}
                  className="rounded-full bg-ai-wash px-2.5 py-1 font-mono text-[12px] font-bold text-ai ring-1 ring-ai-soft"
                >
                  [[{link}]]
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
      <div className="flex gap-2.5 border-t border-line p-3.5">
        <Button variant="outline" disabled={busy || !canRegenerate} onClick={onRegenerate}>
          <Icon name="refresh" size={15} />
          {t("workshop.regen")}
        </Button>
        <Button
          className="flex-1"
          disabled={busy || !draft.title.trim() || !draft.bodyMarkdown.trim()}
          onClick={onConfirm}
        >
          <Icon name="check" size={15} />
          {t("workshop.confirm")}
        </Button>
      </div>
    </div>
  );
}

// コンポーザー内の添付チップ（単一素材、削除不可）。
function SourceChip({ material }: { material: RawMaterial }) {
  const type = RAW_TYPE[material.type];
  return (
    <span className="inline-flex max-w-[260px] items-center gap-2 rounded-[9px] border border-line bg-surface-2 px-2.5 py-1.5">
      <span
        className="grid size-5 flex-none place-items-center rounded-md bg-surface"
        style={{ color: type.color }}
      >
        <Icon name={type.icon} size={13} />
      </span>
      <span className="truncate text-[12.5px] font-semibold text-ink">{material.title}</span>
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

function stripFrontmatter(markdown: string): string {
  if (!markdown.startsWith("---")) return markdown.trim();
  const end = markdown.indexOf("\n---", 3);
  if (end === -1) return markdown.trim();
  return markdown.slice(end + 4).trim();
}
