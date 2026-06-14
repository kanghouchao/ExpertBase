"use client";

import { usePathname } from "next/navigation";

import { Icon } from "@/shared/ui/icon";
import { useI18n } from "@/shared/providers/providers";
import { NAV } from "@/shared/config/nav";

export function TitleBar({ kbLabel, onSettings }: { kbLabel: string; onSettings: () => void }) {
  const { t } = useI18n();
  const pathname = usePathname();
  const routeId = NAV.find((n) => n.href === pathname)?.id ?? "dash";

  return (
    <div className="flex h-11 flex-none items-center gap-3.5 border-b border-line bg-paper-2 px-4 select-none">
      <div className="flex flex-1 items-center gap-2.25 text-[12.5px] font-semibold text-ink-muted">
        <Icon name="book" size={13} className="text-brand" />
        <span className="font-serif text-[13.5px] text-ink-soft">{kbLabel}</span>
        <span className="text-ink-faint">—</span>
        <span>{t(`nav.${routeId}`)}</span>
      </div>
      <button
        type="button"
        onClick={onSettings}
        title={t("cfg.title")}
        className="flex h-7 items-center gap-1.5 rounded-lg border border-line bg-surface px-2.5 text-[12.5px] font-semibold text-ink-soft shadow-(--shadow-sm) transition-colors hover:bg-surface-2 hover:text-ink"
      >
        <Icon name="gear" size={15} />
        <span>{t("cfg.title")}</span>
      </button>
    </div>
  );
}
