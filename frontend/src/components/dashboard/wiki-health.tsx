"use client";

import { Panel } from "@/components/eb/panel";
import { Icon } from "@/components/eb/icon";
import { EmptyState } from "@/components/eb/empty-state";
import { useI18n } from "@/components/providers";

// ナレッジベースの健全性診断。診断は Wiki 項目が貯まってから意味を持つため、
// データ層が未実装の現段階では空状態のみを表示する。
export function WikiHealth() {
  const { t } = useI18n();

  return (
    <Panel pad={22}>
      <div className="mb-1.5 flex items-center gap-2.5">
        <Icon name="shield" size={18} className="text-ai" />
        <span className="text-[15px] font-bold">{t("dash.health")}</span>
      </div>
      <EmptyState icon="shield" title={t("empty.health")} sub={t("empty.health.sub")} />
    </Panel>
  );
}
