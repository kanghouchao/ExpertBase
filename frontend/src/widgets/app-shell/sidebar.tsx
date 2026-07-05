"use client";

import { usePathname, useSearchParams } from "next/navigation";
import { Suspense } from "react";

import {
  parseConversationId,
  requestNewWorkshopConversation,
  WorkshopHistoryNav,
} from "@/features/workshop";
import { NAV, resolveActiveNav } from "@/shared/config/nav";
import { useI18n } from "@/shared/providers/providers";
import { Logo } from "@/shared/ui/logo";
import { NavItem } from "./nav-item";
import { KbSwitcher } from "./kb-switcher";

export function Sidebar({ onAddKb }: { onAddKb: () => void }) {
  const { t } = useI18n();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const conversationId = parseConversationId(searchParams.get("conversation"));
  const activeId = resolveActiveNav(pathname, conversationId);

  const renderItem = (item: (typeof NAV)[number]) => (
    <div key={item.id} className="flex flex-col gap-1">
      <NavItem
        item={item}
        active={item.id === activeId}
        label={t(`nav.${item.id}`)}
        sublabel={t(`nav.${item.id}.sub`)}
        onClick={item.id === "workshop" ? requestNewWorkshopConversation : undefined}
      />
      {item.id === "workshop" ? (
        <Suspense fallback={null}>
          <WorkshopHistoryNav />
        </Suspense>
      ) : null}
    </div>
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

      <nav className="flex min-h-0 flex-1 flex-col gap-0.75 overflow-y-auto">
        {NAV.map(renderItem)}
      </nav>

      <KbSwitcher onAdd={onAddKb} />
    </aside>
  );
}
