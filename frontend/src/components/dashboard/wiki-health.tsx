"use client";

import Link from "next/link";

import { cn } from "@/lib/utils";
import { buttonVariants } from "@/components/ui/button";
import { Panel } from "@/components/eb/panel";
import { Icon } from "@/components/eb/icon";
import { Ring } from "@/components/eb/ring";
import { useI18n } from "@/components/providers";
import { LINT, STATS } from "@/lib/data/mock";
import { L } from "@/lib/data/overrides";

const DOT: Record<string, string> = {
  high: "var(--brand)",
  med: "var(--gold)",
  low: "var(--ink-faint)",
};

export function WikiHealth() {
  const { t, lang } = useI18n();
  const level =
    STATS.health >= 85
      ? t("dash.health.lv.a")
      : STATS.health >= 70
        ? t("dash.health.lv.b")
        : t("dash.health.lv.c");

  return (
    <Panel pad={22}>
      <div className="mb-1.5 flex items-center gap-2.5">
        <Icon name="shield" size={18} className="text-ai" />
        <span className="text-[15px] font-bold">{t("dash.health")}</span>
      </div>
      <p className="mb-4.5 text-[12.5px] leading-relaxed text-ink-muted">
        {t("dash.health.scan")}
      </p>

      <div className="mb-4.5 flex items-center gap-4">
        <Ring value={STATS.health} size={64} sw={6} />
        <div>
          <div className="text-[13.5px] font-semibold">
            {t("dash.health.good")}
            {level}
          </div>
          <div className="mt-0.75 text-[12.5px] text-ink-muted">
            {t("dash.health.found.a")}
            <b className="text-brand">{STATS.orphans}</b>
            {t("dash.health.found.b")}
          </div>
        </div>
      </div>

      {LINT.slice(0, 3).map((l) => (
        <div key={l.id} className="flex gap-2.5 border-t border-line py-2.5">
          <span
            className="mt-1.5 size-1.5 flex-none rounded-full"
            style={{ background: DOT[l.sev] }}
          />
          <div>
            <div className="text-[13px] font-semibold">{L("lint", l, "title", lang)}</div>
            <div className="mt-0.5 text-xs leading-snug text-ink-muted">
              {L("lint", l, "detail", lang)}
            </div>
          </div>
        </div>
      ))}

      <Link
        href="/workshop"
        className={cn(
          buttonVariants({ size: "sm" }),
          "mt-4 w-full bg-ai text-white hover:bg-ai/90"
        )}
      >
        <Icon name="spark" size={15} />
        {t("dash.health.cta")}
      </Link>
    </Panel>
  );
}
