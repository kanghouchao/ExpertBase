"use client";

import { Icon } from "@/components/eb/icon";
import { useI18n } from "@/components/providers";
import { NAV, type RouteId } from "@/lib/data/mock";

// Placeholder for views not yet built this phase. Keeps the sidebar, routing,
// active state, and i18n working end-to-end.
export function ComingSoon({ id }: { id: RouteId }) {
  const { t } = useI18n();
  const item = NAV.find((n) => n.id === id) ?? NAV[0];

  return (
    <div className="view-enter mx-auto flex min-h-[60vh] max-w-170 flex-col items-center justify-center text-center">
      <div className="mb-6 grid size-16 place-items-center rounded-[18px] border border-line bg-surface text-brand shadow-(--shadow-sm)">
        <Icon name={item.icon} size={30} />
      </div>
      <div className="mb-2.5 font-mono text-xs font-semibold tracking-[0.16em] text-brand uppercase">
        {t(`nav.${id}.en`)}
      </div>
      <h1 className="font-serif text-[34px] font-medium text-ink">{t(`nav.${id}`)}</h1>
      <p className="mt-3 text-[15px] text-ink-muted">{t("coming.soon")}</p>
      <p className="mt-1 text-[13px] text-ink-faint">{t("coming.soon.sub")}</p>
    </div>
  );
}
