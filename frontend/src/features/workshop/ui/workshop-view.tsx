"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

import { Icon } from "@/shared/ui/icon";
import { PageHead } from "@/shared/ui/page-head";
import { buttonVariants } from "@/shared/ui/button";
import { useI18n } from "@/shared/providers/providers";
import { deleteInboxMaterial, listInbox, type InboxItem } from "@/shared/api/tauri/client";
import { inboxToMaterial, RAW_TYPE, type RawMaterial } from "@/entities/material";
import { useKbStore } from "@/entities/knowledge-base";

const PREVIEW_MATERIALS: RawMaterial[] = [
  {
    id: "inbox/tea-master.md",
    type: "audio",
    title: "与制茶师傅的访谈录音",
    source: "录音",
    date: "02:14:50 · 2 小时前",
    status: "transcribed",
    size: "",
    preview: "……所以你看，杀青的温度其实没有一个死数字，要看茶青的含水量。手摸下去，第一遍要“高温杀透”，让它...",
    words: 0,
    tags: ["制茶", "杀青"],
  },
  {
    id: "inbox/whitepaper.md",
    type: "pdf",
    title: "2024 普洱仓储白皮书.pdf",
    source: "PDF",
    date: "42 页 · 昨天",
    status: "pending",
    size: "",
    preview: "本白皮书梳理了干仓与湿仓的温湿度区间、霉变风险阈值，以及不同年份饼茶的转化曲线对照...",
    words: 0,
    tags: ["仓储", "普洱"],
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
    tags: ["冲泡", "盖碗"],
  },
  {
    id: "inbox/mountain.md",
    type: "audio",
    title: "语音备忘：勐海茶山见闻",
    source: "录音",
    date: "08:31 · 上周",
    status: "transcribed",
    size: "",
    preview: "老班章和老曼峨的距离其实很近，但口感差异巨大，苦底化得快不快是关键...",
    words: 0,
    tags: ["茶山", "普洱"],
  },
];

export function WorkshopView() {
  const { t } = useI18n();
  const { available } = useKbStore();
  const [pending, setPending] = useState<RawMaterial[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      const inbox = await listInbox();
      setPending(
        inbox.filter((item) => item.status !== "processed").map((item) => materialFromInbox(item))
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  useEffect(() => {
    if (!available) return;
    void (async () => {
      try {
        const inbox = await listInbox();
        setPending(
          inbox
            .filter((item) => item.status !== "processed")
            .map((item) => materialFromInbox(item))
        );
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    })();
  }, [available]);

  async function handleDelete(path: string) {
    if (!confirm(t("capture.delete.confirm"))) return;
    setError(null);
    try {
      await deleteInboxMaterial(path);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  const visibleMaterials = available ? pending : PREVIEW_MATERIALS;
  const pendingCount = visibleMaterials.length;
  const transcribedCount = visibleMaterials.filter((item) => item.status === "transcribed").length;
  const health = available
    ? pendingCount === 0
      ? 100
      : Math.max(50, 90 - pendingCount * 3 + transcribedCount)
    : 78;

  return (
    <div className="view-enter pb-10">
      <div className="flex items-start justify-between gap-4">
        <PageHead eyebrow={t("workshop.eyebrow")} title={t("workshop.title")} sub={t("workshop.listSub")} />
        <button className="mt-20 inline-flex h-11 items-center gap-2 rounded-lg border border-line-strong bg-surface px-4 text-[14px] font-bold text-ink shadow-(--shadow-sm) max-lg:hidden">
          <Icon name="shield" size={17} />
          {t("workshop.recheck")}
        </button>
      </div>

      <section className="mb-6 flex min-h-28 items-center justify-between gap-5 rounded-xl border border-ai-soft bg-surface px-7 py-5 shadow-(--shadow-md)">
        <div className="flex items-center gap-6">
          <div className="grid size-16 place-items-center rounded-full border-[6px] border-gold text-[22px] font-bold text-gold">
            {health}
          </div>
          <div>
            <div className="text-[17px] font-bold text-ink">{t("workshop.healthGood")}</div>
            <div className="mt-1 text-[13.5px] text-ink-muted">{t("workshop.healthSub")}</div>
          </div>
        </div>
        <div className="flex gap-10 pr-3">
          <Metric value={pendingCount} label={t("workshop.waitingMaterials")} />
          <Metric value={4} label={t("workshop.waitingKnowledge")} gold />
        </div>
      </section>

      <div className="mb-5 inline-flex rounded-lg border border-line bg-surface-2 p-1">
        <button className="rounded-md bg-surface px-3 py-1.5 text-[14px] font-bold text-ink shadow-(--shadow-sm)">
          {t("workshop.pendingMaterials")} · {pendingCount}
        </button>
        <button className="rounded-md px-3 py-1.5 text-[14px] font-bold text-ink-muted">
          {t("workshop.pendingKnowledge")} · 4
        </button>
      </div>

      <div className="grid gap-4">
        {visibleMaterials.map((item) => (
          <WorkshopQueueCard
            key={item.id}
            material={item}
            onDelete={available ? handleDelete : undefined}
          />
        ))}
      </div>
      {error && <div className="mt-3 text-[12.5px] font-semibold text-brand">{error}</div>}
    </div>
  );
}

function materialFromInbox(item: InboxItem): RawMaterial {
  const material = inboxToMaterial(item);
  return {
    ...material,
    status: item.type === "audio" || item.type === "video" ? "transcribed" : material.status,
    preview: item.source || material.title,
    tags: [],
  };
}

function Metric({ value, label, gold = false }: { value: number; label: string; gold?: boolean }) {
  return (
    <div className="text-center">
      <div className={gold ? "text-[34px] font-bold text-gold" : "text-[34px] font-bold text-brand"}>
        {value}
      </div>
      <div className="mt-0.5 text-[12px] text-ink-muted">{label}</div>
    </div>
  );
}

function WorkshopQueueCard({
  material,
  onDelete,
}: {
  material: RawMaterial;
  onDelete?: (path: string) => void;
}) {
  const { t } = useI18n();
  const type = RAW_TYPE[material.type];
  const isTranscribed = material.status === "transcribed";
  return (
    <article className="relative min-h-39 overflow-hidden rounded-xl border border-line bg-surface px-7 py-5 shadow-(--shadow-md)">
      <div
        className="absolute top-0 bottom-0 left-0 w-1.5"
        style={{ background: material.type === "video" ? "#9b5a6b" : "var(--brand)" }}
      />
      <div className="flex gap-4">
        <span className="grid size-10 place-items-center rounded-lg bg-surface-2" style={{ color: type.color }}>
          <Icon name={type.icon} size={18} />
        </span>
        <div className="min-w-0 flex-1 pr-40">
          <div className="text-[16px] font-bold text-ink">{material.title}</div>
          <div className="mt-1 font-mono text-[12px] text-ink-faint">
            {material.source} · {material.date}
          </div>
          <p className="mt-4 truncate text-[14px] leading-relaxed text-ink-soft">{material.preview}</p>
          <div className="mt-4 flex gap-2">
            {material.tags.map((tag) => (
              <span key={tag} className="rounded-full bg-surface-2 px-2.5 py-1 text-[12px] font-bold text-ink-muted">
                # {tag}
              </span>
            ))}
          </div>
        </div>
        <div className="absolute top-6 right-6 flex items-center gap-2">
          {onDelete && (
            <button
              type="button"
              onClick={() => onDelete(material.id)}
              aria-label={t("capture.delete")}
              title={t("capture.delete")}
              className="grid size-7 place-items-center rounded-lg text-ink-faint transition-colors hover:bg-surface-2 hover:text-brand"
            >
              <Icon name="trash" size={14} />
            </button>
          )}
          <div className="rounded-full bg-ai-wash px-3 py-1 text-[12px] font-bold text-ai">
            {isTranscribed ? t("st.transcribed") : t("st.pending")}
          </div>
        </div>
        <Link
          className={buttonVariants({ size: "lg", className: "absolute right-6 bottom-5 bg-brand px-5 text-white hover:bg-brand/85" })}
          href={`/workshop/process?path=${encodeURIComponent(material.id)}`}
        >
          <Icon name="arrowR" size={15} />
          {t("workshop.processKnowledge")}
        </Link>
      </div>
    </article>
  );
}
