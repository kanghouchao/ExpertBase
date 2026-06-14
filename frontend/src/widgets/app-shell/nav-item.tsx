import Link from "next/link";

import { cn } from "@/shared/lib/utils";
import { Icon } from "@/shared/ui/icon";
import type { NavItem as NavItemData } from "@/shared/config/nav";

export function NavItem({
  item,
  active,
  label,
  sublabel,
  badge,
}: {
  item: NavItemData;
  active: boolean;
  label: string;
  sublabel: string;
  badge?: number;
}) {
  const ai = item.tone === "ai";
  return (
    <Link
      href={item.href}
      aria-current={active ? "page" : undefined}
      className={cn(
        "group relative flex items-center gap-3 rounded-[11px] px-3.25 py-2.5 transition-colors",
        active
          ? "bg-surface text-ink shadow-(--shadow-sm)"
          : "text-ink-soft hover:bg-surface-2"
      )}
    >
      {active && (
        <span
          className="absolute top-1/2 -left-3.25 h-5 w-1 -translate-y-1/2 rounded-[9px]"
          style={{ background: ai ? "var(--ai)" : "var(--brand)" }}
        />
      )}
      <span
        className={cn("flex-none", active ? (ai ? "text-ai" : "text-brand") : "text-ink-muted")}
      >
        <Icon name={item.icon} size={19} />
      </span>
      <span className="min-w-0 flex-1">
        <span
          className={cn("block text-sm leading-tight", active ? "font-bold" : "font-semibold")}
        >
          {label}
        </span>
        <span className="block font-mono text-[10.5px] tracking-[0.04em] text-ink-faint">
          {sublabel}
        </span>
      </span>
      {badge != null && badge > 0 && (
        <span className="grid h-4.5 min-w-4.5 place-items-center rounded-full bg-brand px-1.25 text-[11px] font-bold text-white">
          {badge}
        </span>
      )}
    </Link>
  );
}
