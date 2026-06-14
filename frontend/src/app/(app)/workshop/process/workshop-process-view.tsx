"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { EmptyState } from "@/components/eb/empty-state";
import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Button, buttonVariants } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import {
  aiHasKey,
  listOllamaModels,
  listInbox,
  readInboxMaterial,
  workshopConfirm,
  workshopDraft,
  type InboxItem,
  type OllamaModel,
} from "@/lib/tauri/client";
import { inboxToMaterial } from "@/lib/data/adapt";
import { RAW_TYPE, type RawMaterial } from "@/lib/data/types";
import { useKbStore } from "@/lib/kb/store";

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

const PREVIEW_RAW = `---
type: audio
source: 与制茶师傅的访谈录音
status: pending
captured_at: 2026-06-14T02:14:50Z
---

……所以你看，杀青的温度其实没有一个死数字，要看茶青的含水量。手摸下去，第一遍要“高温杀透”，让它快速失水；第二遍才看香气和叶色。`;

const PREVIEW_MODELS: OllamaModel[] = [{ name: "qwen3:8b" }, { name: "llama3.1:8b" }];

export function WorkshopProcessView() {
  const { t } = useI18n();
  const router = useRouter();
  const params = useSearchParams();
  const { available } = useKbStore();
  const inboxPath = params.get("path") ?? "";
  const [item, setItem] = useState<RawMaterial | null>(null);
  const [source, setSource] = useState("");
  const [instruction, setInstruction] = useState("");
  const [title, setTitle] = useState("");
  const [cat, setCat] = useState("");
  const [body, setBody] = useState("");
  const [hasOllama, setHasOllama] = useState(false);
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [selectedModel, setSelectedModel] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!inboxPath || !available) return;
    void (async () => {
      setError(null);
      try {
        const [inbox, raw] = await Promise.all([listInbox(), readInboxMaterial(inboxPath)]);
        const found = inbox.find((candidate) => candidate.path === inboxPath) ?? null;
        setItem(found ? materialFromInbox(found) : null);
        setSource(raw);
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

  const visibleItem = available ? item : PREVIEW_SOURCE;
  const visibleSource = available ? source : PREVIEW_RAW;
  const visibleHasOllama = available ? hasOllama : true;
  const visibleModels = available ? models : PREVIEW_MODELS;
  const visibleSelectedModel = available ? selectedModel : (selectedModel || PREVIEW_MODELS[0].name);

  async function handleGenerate() {
    if (!inboxPath) return;
    setBusy(true);
    setError(null);
    try {
      const result = await workshopDraft(inboxPath, instruction, visibleSelectedModel);
      setTitle(result.title);
      setCat(result.cat);
      setBody(result.bodyMarkdown);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleConfirm() {
    if (!inboxPath || !title.trim() || !body.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await workshopConfirm({ inboxPath, title, cat, body });
      router.push("/workshop");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="view-enter pb-10">
      <div className="mb-2 flex items-center justify-between gap-4">
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

      {!inboxPath || (!visibleItem && !visibleSource && !error) ? (
        <div className="rounded-xl border border-line bg-surface shadow-(--shadow-md)">
          <EmptyState icon="shield" title={t("workshop.notFound")} sub={t("workshop.queueHint")} />
        </div>
      ) : (
        <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-6 max-lg:grid-cols-1">
          <section>
            <SectionLabel icon="folder" label={t("workshop.rawMaterial")} />
            <div className="rounded-xl border border-line bg-surface shadow-(--shadow-md)">
              {visibleItem && <SourceCard material={visibleItem} source={visibleSource} />}
            </div>
            <button className="mt-4 flex h-13 w-full items-center justify-center gap-2 rounded-xl border border-dashed border-line-strong text-[15px] font-bold text-ink-soft">
              <Icon name="merge" size={17} />
              {t("workshop.mergeMore")}
            </button>
          </section>

          <section>
            <SectionLabel icon="book" label={t("workshop.organizedEntry")} />
            <div className="flex min-h-[520px] flex-col overflow-hidden rounded-xl border border-line bg-surface shadow-(--shadow-md)">
              <div className="flex min-h-17 items-center justify-between border-b border-line px-5">
                <div>
                  <input
                    value={title}
                    onChange={(event) => setTitle(event.target.value)}
                    placeholder={t("workshop.draftTitle")}
                    className="w-full bg-transparent text-[16px] font-bold text-ink outline-none"
                  />
                  <div className="mt-0.5 text-[12px] text-ink-muted">{body ? t("workshop.readyReview") : t("workshop.waitingAi")}</div>
                </div>
                <span className="inline-flex items-center gap-1.5 rounded-full bg-ai-wash px-3 py-1 text-[12px] font-bold text-ai">
                  <Icon name="spark" size={12} />
                  AI {t("workshop.assist")}
                </span>
              </div>
              <div className="flex-1 px-8 py-8">
                <input
                  value={cat}
                  onChange={(event) => setCat(event.target.value)}
                  placeholder={t("workshop.catField")}
                  className="mb-3 w-full rounded-lg border border-line-strong bg-surface-2 px-3 py-2 font-mono text-[13px] text-ink outline-none"
                />
                <textarea
                  value={body}
                  onChange={(event) => setBody(event.target.value)}
                  placeholder={t("workshop.prepareSub")}
                  className="min-h-[350px] w-full resize-y rounded-lg border border-line-strong bg-surface-2 p-3.5 font-mono text-[13.5px] leading-relaxed text-ink outline-none"
                />
              </div>
              <div className="border-t border-line p-4">
                <div className="mb-3 flex items-center gap-2">
                  <label className="font-mono text-[12px] font-bold tracking-[0.06em] text-ink-muted">
                    {t("workshop.model")}
                  </label>
                  <select
                    value={visibleSelectedModel}
                    onChange={(event) => setSelectedModel(event.target.value)}
                    disabled={!visibleHasOllama || visibleModels.length === 0 || busy}
                    className="h-8 min-w-44 rounded-lg border border-line-strong bg-surface px-2.5 font-mono text-[12px] font-semibold text-ink outline-none disabled:opacity-50"
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
                </div>
                <div className="flex items-center gap-3 max-sm:flex-col max-sm:items-stretch">
                  <input
                    value={instruction}
                    onChange={(event) => setInstruction(event.target.value)}
                    placeholder={t("workshop.instruction.ph")}
                    className="h-11 min-w-0 flex-1 rounded-lg border border-line-strong bg-surface px-4 text-[14px] text-ink outline-none"
                  />
                  <Button
                    className="h-11 bg-ai px-5 text-white hover:bg-ai/85"
                    disabled={!visibleHasOllama || !visibleSelectedModel || busy}
                    onClick={handleGenerate}
                  >
                    <Icon name="spark" size={15} />
                    {t("workshop.aiOrganize")}
                  </Button>
                </div>
                {!visibleHasOllama && <div className="mt-2 text-[12px] text-ink-faint">{t("workshop.noKey")}</div>}
                {visibleHasOllama && visibleModels.length === 0 && (
                  <div className="mt-2 text-[12px] text-ink-faint">{t("workshop.noModelsHint")}</div>
                )}
                {error && <div className="mt-2 text-[12.5px] font-semibold text-brand">{error}</div>}
                <div className="mt-3 flex justify-end">
                  <Button disabled={!title.trim() || !body.trim() || busy} onClick={handleConfirm}>
                    <Icon name="check" size={15} />
                    {t("workshop.confirm")}
                  </Button>
                </div>
              </div>
            </div>
          </section>
        </div>
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

function SectionLabel({ icon, label }: { icon: "folder" | "book"; label: string }) {
  return (
    <div className="mb-3 flex items-center gap-2 font-mono text-[12px] font-bold tracking-[0.06em] text-ink-muted">
      <Icon name={icon} size={14} />
      {label}
    </div>
  );
}

function SourceCard({ material, source }: { material: RawMaterial; source: string }) {
  const type = RAW_TYPE[material.type];
  const visibleSource = stripFrontmatter(source) || material.preview;
  return (
    <div>
      <div className="flex items-center gap-4 px-5 py-4">
        <span className="grid size-10 place-items-center rounded-lg bg-surface-2" style={{ color: type.color }}>
          <Icon name={type.icon} size={18} />
        </span>
        <div className="min-w-0">
          <div className="truncate text-[16px] font-bold text-ink">{material.title}</div>
          <div className="mt-0.5 font-mono text-[12px] text-ink-faint">
            {material.source} · {material.date}
          </div>
        </div>
      </div>
      <div className="max-h-44 overflow-auto border-t border-line px-5 py-4 text-[14px] leading-relaxed whitespace-pre-wrap text-ink-soft">
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
