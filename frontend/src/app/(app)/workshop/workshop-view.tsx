"use client";

import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { EmptyState } from "@/components/eb/empty-state";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { RAW_MATERIALS } from "@/lib/data/store";
import { MaterialRow } from "../_components/material-row";

export function WorkshopView() {
  const { t } = useI18n();
  const pending = RAW_MATERIALS.filter((item) => item.status !== "processed");

  return (
    <div className="view-enter">
      <PageHead eyebrow={t("workshop.eyebrow")} title={t("workshop.title")} sub={t("workshop.sub")} />

      <div className="grid grid-cols-[1fr_1fr] gap-5">
        <div>
          <h2 className="mb-3 font-mono text-[12px] font-bold tracking-[0.12em] text-ink-muted uppercase">
            {t("workshop.pendingMaterials")}
          </h2>
          <Panel pad={0}>
            {pending.length === 0 && (
              <EmptyState
                icon="merge"
                title={t("empty.materials")}
                sub={t("empty.materials.sub")}
              />
            )}
            {pending.map((item) => (
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
          <Panel pad={0}>
            <EmptyState icon="shield" title={t("empty.health")} sub={t("empty.health.sub")} />
          </Panel>
        </div>
      </div>
    </div>
  );
}
