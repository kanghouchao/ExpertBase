"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

import { cn } from "@/lib/utils";
import { Panel } from "@/components/eb/panel";
import { Tag } from "@/components/eb/tag";
import { Icon } from "@/components/eb/icon";
import { EmptyState } from "@/components/eb/empty-state";
import { useI18n } from "@/components/providers";
import { listInbox } from "@/lib/tauri/client";
import { inboxToMaterial } from "@/lib/data/adapt";
import { useKbStore } from "@/lib/kb/store";
import { RAW_TYPE, STATUS, type RawMaterial } from "@/lib/data/types";

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
