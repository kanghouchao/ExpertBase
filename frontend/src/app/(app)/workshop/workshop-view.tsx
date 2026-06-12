"use client";

import Link from "next/link";

import { Icon, type IconName } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Ring } from "@/components/eb/ring";
import { Tag } from "@/components/eb/tag";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { LINT, PENDING, RAW_MATERIALS, STATS, type TagTone } from "@/lib/data/mock";
import { L } from "@/lib/data/overrides";
import { qualityLabel } from "@/lib/i18n/data";
import { cn } from "@/lib/utils";
import { MaterialRow } from "../_components/material-row";

function QueueCard({
  icon,
  title,
  body,
  tone = "accent",
  href,
}: {
  icon: IconName;
  title: string;
  body: string;
  tone?: TagTone;
  href: string;
}) {
  const { t } = useI18n();

  return (
    <Link href={href}>
      <Panel hover className="h-full">
        <div className="mb-3 flex items-start gap-3">
          <span
            className={cn(
              "grid size-9 place-items-center rounded-[10px]",
              tone === "ai" ? "bg-ai-wash text-ai" : "bg-brand-wash text-brand"
            )}
          >
            <Icon name={icon} size={18} />
          </span>
          <div>
            <div className="font-semibold text-ink">{title}</div>
            <p className="mt-1 text-[13px] leading-relaxed text-ink-muted">{body}</p>
          </div>
        </div>
        <div className="flex items-center justify-between border-t border-line pt-3">
          <Tag tone={tone}>{t("workshop.aiAssist")}</Tag>
          <Icon name="arrowR" size={15} className="text-ink-faint" />
        </div>
      </Panel>
    </Link>
  );
}

export function WorkshopView() {
  const { t, lang } = useI18n();

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("workshop.eyebrow")}
        title={t("workshop.title")}
        sub={t("workshop.sub")}
        right={
          <Button variant="outline" className="border-line-strong bg-surface">
            <Icon name="scan" size={17} />
            {t("workshop.rescan")}
          </Button>
        }
      />

      <Panel pad={0} className="mb-5.5 overflow-hidden border-ai-soft">
        <div className="flex items-center gap-5 px-6 py-4.5">
          <Ring value={STATS.health} size={58} sw={6} />
          <div className="flex-1">
            <div className="text-[15px] font-bold text-ink">
              {t("workshop.health", { level: qualityLabel(STATS.health, t) })}
            </div>
            <div className="mt-1 text-[13px] text-ink-muted">{t("workshop.scan")}</div>
          </div>
          <div className="flex gap-7 pr-2">
            <div>
              <div className="font-serif text-[26px] leading-none font-bold text-brand">
                {PENDING}
              </div>
              <div className="mt-1 font-mono text-[11px] text-ink-faint">
                {t("workshop.pendingMaterials")}
              </div>
            </div>
            <div>
              <div className="font-serif text-[26px] leading-none font-bold text-ai">
                {LINT.length}
              </div>
              <div className="mt-1 font-mono text-[11px] text-ink-faint">
                {t("workshop.pendingKnowledge")}
              </div>
            </div>
          </div>
        </div>
      </Panel>

      <div className="grid grid-cols-[1fr_1fr] gap-5">
        <div>
          <h2 className="mb-3 font-mono text-[12px] font-bold tracking-[0.12em] text-ink-muted uppercase">
            {t("workshop.pendingMaterials")}
          </h2>
          <Panel pad={0}>
            {RAW_MATERIALS.filter((item) => item.status !== "processed").map((item) => (
              <MaterialRow
                key={item.id}
                material={item}
                action={
                  <Button size="sm" variant="outline">
                    {t("workshop.process")}
                  </Button>
                }
              />
            ))}
          </Panel>
        </div>
        <div>
          <h2 className="mb-3 font-mono text-[12px] font-bold tracking-[0.12em] text-ink-muted uppercase">
            {t("workshop.pendingKnowledge")}
          </h2>
          <div className="grid gap-3">
            {LINT.map((issue) => (
              <QueueCard
                key={issue.id}
                icon={
                  issue.type === "orphan"
                    ? "flag"
                    : issue.type === "dup"
                      ? "merge"
                      : issue.type === "stale"
                        ? "clock"
                        : "edit"
                }
                title={L("lint", issue, "title", lang)}
                body={L("lint", issue, "detail", lang)}
                tone={issue.sev === "high" ? "accent" : "ai"}
                href="/wiki"
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
