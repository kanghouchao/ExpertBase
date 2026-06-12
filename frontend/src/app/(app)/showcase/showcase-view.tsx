"use client";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Tag } from "@/components/eb/tag";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { L } from "@/lib/data/overrides";
import { DEPLOY_HISTORY, WIKI } from "@/lib/data/mock";
import { wikiCategoryLabel } from "@/lib/i18n/data";

export function ShowcaseView() {
  const { t, lang } = useI18n();

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("showcase.eyebrow")}
        title={t("showcase.title")}
        sub={t("showcase.sub")}
      />
      <div className="grid grid-cols-[1.1fr_0.9fr] gap-5">
        <Panel className="overflow-hidden" pad={0}>
          <div className="bg-ink px-8 py-7 text-paper">
            <div className="mb-12 flex items-center justify-between">
              <div className="font-serif text-xl font-semibold">{t("showcase.kb")}</div>
              <Button variant="secondary" size="sm">
                {t("showcase.login")}
              </Button>
            </div>
            <h2 className="max-w-120 font-serif text-[42px] leading-none font-medium">
              {t("showcase.hero.a")}
              <span className="text-brand-soft italic">{t("showcase.hero.b")}</span>
            </h2>
            <p className="mt-4 max-w-110 text-[15px] leading-relaxed text-paper/75">
              {t("showcase.hero.sub")}
            </p>
            <div className="mt-6 flex max-w-110 items-center gap-3 rounded-xl bg-paper/10 px-4 py-3 text-paper/65">
              <Icon name="search" size={17} />
              {t("showcase.search")}
            </div>
          </div>
          <div className="grid grid-cols-3 gap-3 bg-surface p-5">
            {WIKI.slice(0, 3).map((entry) => (
              <div key={entry.id} className="rounded-xl border border-line bg-surface-2 p-4">
                <Tag tone="muted">{wikiCategoryLabel(entry.cat, t)}</Tag>
                <div className="mt-3 font-serif text-xl font-semibold">{entry.title}</div>
                <p className="mt-2 line-clamp-2 text-[12.5px] leading-relaxed text-ink-muted">
                  {entry.excerpt}
                </p>
              </div>
            ))}
          </div>
        </Panel>

        <div className="grid gap-4">
          <Panel>
            <div className="mb-4 flex items-center justify-between">
              <div>
                <div className="text-[15px] font-bold text-ink">{t("showcase.publish")}</div>
                <div className="text-[12.5px] text-ink-muted">{t("showcase.publish.sub")}</div>
              </div>
              <Tag tone="ai">{t("showcase.live")}</Tag>
            </div>
            <div className="rounded-xl border border-line bg-surface-2 p-4">
              <div className="flex items-center gap-3">
                <span className="grid size-10 place-items-center rounded-[10px] bg-ink text-paper">
                  <Icon name="globe" size={19} />
                </span>
                <div className="flex-1">
                  <div className="font-semibold">Vercel</div>
                  <div className="font-mono text-[11px] text-ink-faint">{t("showcase.cdn")}</div>
                </div>
                <Tag tone="accent">{t("showcase.recommended")}</Tag>
              </div>
              <Button className="mt-4 w-full">
                <Icon name="upload" size={15} />
                {t("showcase.redeploy")}
              </Button>
            </div>
          </Panel>
          <Panel>
            <div className="mb-3 text-[15px] font-bold text-ink">{t("showcase.history")}</div>
            <div className="grid gap-2">
              {DEPLOY_HISTORY.map((item) => (
                <div
                  key={item.ver}
                  className="flex items-center gap-3 rounded-lg border border-line bg-surface-2 px-3 py-2.5"
                >
                  <Tag
                    tone={
                      item.status === "error" ? "accent" : item.status === "live" ? "ai" : "muted"
                    }
                  >
                    {item.ver}
                  </Tag>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13px] font-semibold">
                      {L("deploy", item, "commit", lang)}
                    </div>
                    <div className="font-mono text-[11px] text-ink-faint">
                      {L("deploy", item, "when", lang)} · {item.dur}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </Panel>
        </div>
      </div>
    </div>
  );
}
