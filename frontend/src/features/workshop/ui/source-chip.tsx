"use client";

import { Icon } from "@/shared/ui/icon";
import { RAW_TYPE, type RawMaterial } from "@/entities/material";

// コンポーザー内の添付チップ。onRemove があれば × で外せる。
export function SourceChip({ material, onRemove }: { material: RawMaterial; onRemove?: () => void }) {
  const type = RAW_TYPE[material.type];
  return (
    <span className="inline-flex max-w-65 items-center gap-2 rounded-[9px] border border-line bg-surface-2 py-1.5 pr-2 pl-2.5">
      <span
        className="grid size-5 flex-none place-items-center rounded-md bg-surface"
        style={{ color: type.color }}
      >
        <Icon name={type.icon} size={13} />
      </span>
      <span className="truncate text-[12.5px] font-semibold text-ink">{material.title}</span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          title={material.title}
          className="grid flex-none place-items-center text-ink-faint transition-colors hover:text-ink"
        >
          <Icon name="x" size={14} />
        </button>
      )}
    </span>
  );
}
