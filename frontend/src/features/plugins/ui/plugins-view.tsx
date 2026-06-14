"use client";

import { PageHead } from "@/shared/ui/page-head";
import { Panel } from "@/shared/ui/panel";
import { EmptyState } from "@/shared/ui/empty-state";
import { useI18n } from "@/shared/providers/providers";

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
