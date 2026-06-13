"use client";

import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { EmptyState } from "@/components/eb/empty-state";
import { useI18n } from "@/components/providers";

// プラグイン基盤は未実装のため、マーケットは空状態のみを表示する。
export function PluginsView() {
  const { t } = useI18n();

  return (
    <div className="view-enter">
      <PageHead eyebrow={t("plugins.eyebrow")} title={t("plugins.title")} sub={t("plugins.sub")} />
      <Panel pad={0}>
        <EmptyState icon="plug" title={t("empty.plugins")} sub={t("empty.plugins.sub")} />
      </Panel>
    </div>
  );
}
