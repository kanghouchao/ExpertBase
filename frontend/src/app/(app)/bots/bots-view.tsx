"use client";

import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { EmptyState } from "@/components/eb/empty-state";
import { useI18n } from "@/components/providers";

// ボットはまだ作成できない（チャネルプラグインの実装待ち）。
export function BotsView() {
  const { t } = useI18n();

  return (
    <div className="view-enter">
      <PageHead eyebrow={t("bots.eyebrow")} title={t("bots.title")} sub={t("bots.sub")} />
      <Panel pad={0}>
        <EmptyState icon="bot" title={t("empty.bots")} sub={t("empty.bots.sub")} />
      </Panel>
    </div>
  );
}
