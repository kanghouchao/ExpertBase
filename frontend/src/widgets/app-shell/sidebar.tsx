"use client";

import { usePathname } from "next/navigation";

import { Logo } from "@/shared/ui/logo";
import { NavItem } from "./nav-item";
import { KbSwitcher } from "./kb-switcher";
import { useI18n } from "@/shared/providers/providers";
import { NAV } from "@/shared/config/nav";

export function Sidebar({ onAddKb }: { onAddKb: () => void }) {
  const { t } = useI18n();
  const pathname = usePathname();
  const activeId =
    NAV.find((n) => pathname === n.href || (n.href !== "/" && pathname.startsWith(`${n.href}/`)))
      ?.id ?? "dash";

  const renderItem = (item: (typeof NAV)[number]) => (
    <NavItem
      key={item.id}
      item={item}
      active={item.id === activeId}
      label={t(`nav.${item.id}`)}
      sublabel={t(`nav.${item.id}.sub`)}
    />
  );

  return (
    <aside className="flex w-62 flex-none flex-col border-r border-line bg-paper-2 px-4 py-5">
      <div className="flex items-center gap-2.75 px-2 pt-1 pb-5.5">
        <Logo size={34} />
        <div>
          <div className="font-serif text-[19px] leading-none font-semibold tracking-[-0.01em]">
            ExpertBase
          </div>
          <div className="mt-0.75 font-mono text-[10.5px] tracking-[0.06em] text-ink-muted">
            {t("app.tagline")}
          </div>
        </div>
      </div>

      <nav className="flex flex-1 flex-col gap-0.75">
        {NAV.slice(0, 5).map(renderItem)}
        <div className="mx-2 my-3 h-px bg-line" />
        <div className="px-3.25 pb-2 font-mono text-[10.5px] font-semibold tracking-[0.12em] text-ink-faint uppercase">
          {t("nav.group.external")}
        </div>
        {NAV.slice(5).map(renderItem)}
      </nav>

      <KbSwitcher onAdd={onAddKb} />
    </aside>
  );
}
