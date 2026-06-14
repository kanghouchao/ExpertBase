"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

import { cn } from "@/shared/lib/utils";
import { Panel } from "@/shared/ui/panel";
import { Tag } from "@/shared/ui/tag";
import { Icon } from "@/shared/ui/icon";
import { EmptyState } from "@/shared/ui/empty-state";
import { useI18n } from "@/shared/providers/providers";
import { listInbox } from "@/shared/api/tauri/client";
import { inboxToMaterial } from "@/entities/material";
import { useKbStore } from "@/entities/knowledge-base";
import { RAW_TYPE, STATUS, type RawMaterial } from "@/entities/material";

export function RecentMaterials() {
  const { t } = useI18n();
  const { available } = useKbStore();
  const [items, setItems] = useState<RawMaterial[]>([]);

  useEffect(() => {
    if (!available) return;
    void listInbox().then((inbox) => setItems(inbox.map(inboxToMaterial)));
  }, [available]);

  return (
    <Panel pad={0} className="overflow-hidden">
      <div className="flex items-center justify-between border-b border-line px-5.5 py-4.5">
        <span className="text-[15px] font-bold">{t("dash.recent")}</span>
        <Link
          href="/capture"
          className="flex items-center gap-1 text-[13px] font-semibold text-brand hover:underline"
        >
          {t("dash.viewAll")} <Icon name="chevR" size={13} />
        </Link>
      </div>
      {items.length === 0 && (
        <EmptyState icon="inbox" title={t("empty.materials")} sub={t("empty.materials.sub")} />
      )}
      {items.slice(0, 4).map((r, i) => {
        const ty = RAW_TYPE[r.type];
        return (
          <Link
            key={r.id}
            href="/capture"
            className={cn(
              "flex items-center gap-3 px-5.5 py-3.25 transition-colors hover:bg-surface-2",
              i < 3 && "border-b border-line"
            )}
          >
            <span
              className="grid size-8.5 flex-none place-items-center rounded-[9px] bg-surface-2"
              style={{ color: ty.color }}
            >
              <Icon name={ty.icon} size={18} />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-sm font-semibold">{r.title}</span>
              <span className="mt-0.5 block font-mono text-xs text-ink-faint">
                {r.source} · {r.date}
              </span>
            </span>
            <Tag tone={STATUS[r.status].tone}>{t(`st.${r.status}`)}</Tag>
          </Link>
        );
      })}
    </Panel>
  );
}
