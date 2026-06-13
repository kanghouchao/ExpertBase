"use client";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { EmptyState } from "@/components/eb/empty-state";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { WIKI } from "@/lib/data/store";
import { useKbStore } from "@/lib/kb/store";

export function ShowcaseView() {
  const { t } = useI18n();
  const { active } = useKbStore();

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("showcase.eyebrow")}
        title={t("showcase.title")}
        sub={t("showcase.sub")}
      />
      <div className="grid grid-cols-[1.1fr_0.9fr] gap-5">
        {/* 公開サイトのプレビュー。文言はアクティブなナレッジベース由来 */}
        <Panel className="overflow-hidden" pad={0}>
          <div className="bg-ink px-8 py-7 text-paper">
            <div className="mb-12 flex items-center justify-between">
              <div className="font-serif text-xl font-semibold">{active?.name ?? "ExpertBase"}</div>
              <Button variant="secondary" size="sm">
                {t("showcase.login")}
              </Button>
            </div>
            <h2 className="max-w-120 font-serif text-[42px] leading-none font-medium">
              {active?.name ?? t("showcase.title")}
            </h2>
            <div className="mt-6 flex max-w-110 items-center gap-3 rounded-xl bg-paper/10 px-4 py-3 text-paper/65">
              <Icon name="search" size={17} />
              {t("showcase.search")}
            </div>
          </div>
          <div className="bg-surface p-5">
            {WIKI.length === 0 ? (
              <EmptyState icon="eye" title={t("empty.wiki")} sub={t("empty.showcase.sub")} />
            ) : (
              <div className="grid grid-cols-3 gap-3">
                {WIKI.slice(0, 3).map((entry) => (
                  <div key={entry.id} className="rounded-xl border border-line bg-surface-2 p-4">
                    <div className="font-serif text-xl font-semibold">{entry.title}</div>
                    <p className="mt-2 line-clamp-2 text-[12.5px] leading-relaxed text-ink-muted">
                      {entry.excerpt}
                    </p>
                  </div>
                ))}
              </div>
            )}
          </div>
        </Panel>

        {/* 公開機能はプラグイン基盤の実装後に提供する */}
        <Panel pad={0}>
          <EmptyState icon="globe" title={t("empty.publish")} sub={t("empty.publish.sub")} />
        </Panel>
      </div>
    </div>
  );
}
