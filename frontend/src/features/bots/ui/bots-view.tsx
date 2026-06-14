"use client";

import { PageHead } from "@/shared/ui/page-head";
import { Panel } from "@/shared/ui/panel";
import { EmptyState } from "@/shared/ui/empty-state";
import { useI18n } from "@/shared/providers/providers";

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
