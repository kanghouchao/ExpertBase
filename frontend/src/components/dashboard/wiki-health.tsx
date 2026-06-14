"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

import { Panel } from "@/components/eb/panel";
import { Icon } from "@/components/eb/icon";
import { EmptyState } from "@/components/eb/empty-state";
import { useI18n } from "@/components/providers";
import { orphans as fetchOrphans, type EntryRef } from "@/lib/tauri/client";
import { useKbStore } from "@/lib/kb/store";

// ナレッジベースの健全性診断。MVP では孤立条目（被リンク・発リンクともに無い条目）の検出のみ。
export function WikiHealth() {
  const { t } = useI18n();
  const { available } = useKbStore();
  const [orphans, setOrphans] = useState<EntryRef[]>([]);

  useEffect(() => {
    if (!available) return;
    void fetchOrphans().then(setOrphans);
  }, [available]);

  return (
    <Panel pad={22}>
      <div className="mb-1.5 flex items-center gap-2.5">
        <Icon name="shield" size={18} className="text-ai" />
        <span className="text-[15px] font-bold">{t("dash.health")}</span>
      </div>
      {orphans.length === 0 ? (
        <EmptyState icon="shield" title={t("empty.health")} sub={t("empty.health.sub")} />
      ) : (
        <div className="mt-3 grid gap-1.5">
          {orphans.slice(0, 6).map((o) => (
            <Link
              key={o.path}
              href="/wiki"
              className="flex items-center gap-2.5 rounded-lg border border-line bg-surface-2 px-3 py-2 transition-colors hover:bg-surface"
            >
              <Icon name="flag" size={14} className="flex-none text-brand" />
              <span className="truncate text-[13.5px] font-semibold text-ink">{o.title}</span>
              <span className="ml-auto font-mono text-[11px] text-ink-faint">{o.cat}</span>
            </Link>
          ))}
        </div>
      )}
    </Panel>
  );
}
