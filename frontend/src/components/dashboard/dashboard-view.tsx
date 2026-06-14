"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

import { cn } from "@/lib/utils";
import { Button, buttonVariants } from "@/components/ui/button";
import { Panel } from "@/components/eb/panel";
import { PageHead } from "@/components/eb/page-head";
import { Icon, type IconName } from "@/components/eb/icon";
import { useI18n } from "@/components/providers";
import { RecentMaterials } from "@/components/dashboard/recent-materials";
import { WikiHealth } from "@/components/dashboard/wiki-health";
import { listInbox, stats as fetchStats } from "@/lib/tauri/client";
import { useKbStore } from "@/lib/kb/store";

type Tone = "accent" | "ai";

function StatTile({
  icon,
  label,
  value,
  sub,
  tone,
}: {
  icon: IconName;
  label: string;
  value: string;
  sub: string;
  tone: Tone;
}) {
  return (
    <Panel pad={18} className="min-w-0 flex-1">
      <div className="mb-3.5 flex items-center gap-2.5">
        <span
          className={cn(
            "grid size-7.5 place-items-center rounded-lg",
            tone === "ai" ? "bg-ai-wash text-ai" : "bg-brand-wash text-brand"
          )}
        >
          <Icon name={icon} size={17} />
        </span>
        <span className="text-[13px] font-semibold text-ink-muted">{label}</span>
      </div>
      <div className="font-serif text-[30px] leading-none font-bold text-ink">{value}</div>
      <div className="mt-1.75 text-[12.5px] text-ink-faint">{sub}</div>
    </Panel>
  );
}

function PipelineStep({
  icon,
  label,
  count,
  tone,
  last,
}: {
  icon: IconName;
  label: string;
  count: string;
  tone: Tone | "muted";
  last?: boolean;
}) {
  const square =
    tone === "ai"
      ? "bg-ai-wash text-ai"
      : tone === "accent"
        ? "bg-brand-wash text-brand"
        : "bg-surface-2 text-ink-muted";
  return (
    <div className="flex flex-1 items-center">
      <div className="flex-1 text-center">
        <div
          className={cn(
            "mx-auto mb-2.5 grid size-12 place-items-center rounded-[13px] border border-line",
            square
          )}
        >
          <Icon name={icon} size={22} />
        </div>
        <div className="text-[13.5px] font-semibold text-ink">{label}</div>
        <div className="mt-0.75 font-mono text-xs text-ink-faint">{count}</div>
      </div>
      {!last && (
        <div className="relative -mt-7.5 h-0.5 w-13.5 flex-none bg-line-strong">
          <span className="absolute -top-0.75 -right-px text-ink-faint">
            <Icon name="chevR" size={8} />
          </span>
        </div>
      )}
    </div>
  );
}

export function DashboardView() {
  const { t, lang } = useI18n();
  const { active, available } = useKbStore();
  const [data, setData] = useState({ rawCount: 0, pending: 0, wikiCount: 0, links: 0 });

  useEffect(() => {
    if (!available) return;
    void (async () => {
      const [s, inbox] = await Promise.all([fetchStats(), listInbox()]);
      setData({
        rawCount: inbox.length,
        pending: inbox.filter((item) => item.status !== "processed").length,
        wikiCount: s?.entries ?? 0,
        links: s?.links ?? 0,
      });
    })();
  }, [available]);

  const nf = new Intl.NumberFormat(lang === "zh" ? "zh-CN" : lang);
  const count = (value: number, unitKey: string) => `${nf.format(value)} ${t(unitKey)}`;

  return (
    <div className="view-enter mx-auto max-w-270">
      <PageHead
        eyebrow={t("dash.eyebrow")}
        title={
          <>
            {t("dash.greet.a")}
            <span className="text-brand italic">{active?.name ?? t("nav.dash")}</span>
          </>
        }
        sub={t("dash.sub")}
        right={
          <>
            <Button variant="outline" size="lg" className="border-line-strong bg-surface">
              <Icon name="search" size={17} />
              {t("c.search")}
            </Button>
            <Link href="/capture" className={cn(buttonVariants({ size: "lg" }))}>
              <Icon name="plus" size={17} />
              {t("c.add")}
            </Link>
          </>
        }
      />

      <Panel pad={26} className="mb-5.5">
        <div className="mb-6 flex items-center justify-between">
          <div>
            <div className="text-[15px] font-bold">{t("dash.pipeline")}</div>
            <div className="mt-0.75 text-[12.5px] text-ink-muted">{t("dash.pipeline.sub")}</div>
          </div>
        </div>
        <div className="flex items-start">
          <PipelineStep
            icon="inbox"
            label={t("dash.p.collect")}
            count={count(data.rawCount, "unit.materials")}
            tone="accent"
          />
          <PipelineStep
            icon="merge"
            label={t("dash.p.work")}
            count={count(data.pending, "unit.pending")}
            tone="accent"
          />
          <PipelineStep
            icon="book"
            label={t("dash.p.kb")}
            count={count(data.wikiCount, "unit.entries")}
            tone="accent"
          />
          <PipelineStep
            icon="graph"
            label={t("dash.p.link")}
            count={count(data.links, "unit.links")}
            tone="ai"
          />
          <PipelineStep
            icon="bot"
            label={t("dash.p.serve")}
            count={count(0, "unit.members")}
            tone="accent"
            last
          />
        </div>
      </Panel>

      <div className="mb-5.5 flex gap-3.5">
        <StatTile
          icon="inbox"
          label={t("dash.t.inbox")}
          value={nf.format(data.rawCount)}
          sub={t("dash.t.inbox.s")}
          tone="accent"
        />
        <StatTile
          icon="book"
          label={t("dash.t.wiki")}
          value={nf.format(data.wikiCount)}
          sub={t("dash.t.wiki.s")}
          tone="accent"
        />
        <StatTile
          icon="link"
          label={t("dash.t.links")}
          value={nf.format(data.links)}
          sub={t("dash.t.links.s")}
          tone="ai"
        />
        <StatTile
          icon="chat"
          label={t("dash.t.qa")}
          value={nf.format(0)}
          sub={t("dash.t.qa.s")}
          tone="ai"
        />
      </div>

      <div className="grid grid-cols-[1.4fr_1fr] gap-4.5">
        <RecentMaterials />
        <WikiHealth />
      </div>
    </div>
  );
}
