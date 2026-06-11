"use client";

import { cn } from "@/lib/utils";

export function SegTabs<T extends string>({
  tabs,
  value,
  onChange,
}: {
  tabs: readonly T[];
  value: T;
  onChange: (value: T) => void;
}) {
  return (
    <div className="inline-flex rounded-[11px] border border-line bg-surface-2 p-0.75">
      {tabs.map((tab) => {
        const active = tab === value;
        return (
          <button
            key={tab}
            onClick={() => onChange(tab)}
            className={cn(
              "rounded-lg px-3.5 py-1.75 text-[13px] font-semibold whitespace-nowrap transition",
              active ? "bg-surface text-ink shadow-(--shadow-sm)" : "text-ink-muted hover:text-ink"
            )}
          >
            {tab}
          </button>
        );
      })}
    </div>
  );
}
