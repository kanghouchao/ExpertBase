"use client";

import type { ReactNode } from "react";

import { Icon } from "@/shared/ui/icon";
import { Tag } from "@/shared/ui/tag";
import { useI18n } from "@/shared/providers/providers";
import { RAW_TYPE, STATUS, type RawMaterial } from "@/lib/data/types";

export function MaterialRow({ material, action }: { material: RawMaterial; action?: ReactNode }) {
  const { t } = useI18n();
  const rawType = RAW_TYPE[material.type];

  return (
    <div className="flex items-center gap-3 border-t border-line px-4 py-3 first:border-t-0">
      <span
        className="grid size-8 place-items-center rounded-lg bg-surface-2"
        style={{ color: rawType.color }}
      >
        <Icon name={rawType.icon} size={16} />
      </span>
      <div className="min-w-0 flex-1">
        <div className="truncate text-[13.5px] font-semibold text-ink">{material.title}</div>
        <div className="mt-0.5 truncate font-mono text-[11px] text-ink-faint">
          {material.source} · {material.date} · {material.size}
        </div>
      </div>
      <Tag tone={STATUS[material.status].tone}>{t(`st.${material.status}`)}</Tag>
      {action}
    </div>
  );
}
