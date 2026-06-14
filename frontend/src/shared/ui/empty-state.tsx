import type { ReactNode } from "react";

import { Icon, type IconName } from "@/shared/ui/icon";

// データが空のときに各ビューへ表示する共通プレースホルダー。
export function EmptyState({
  icon = "inbox",
  title,
  sub,
  action,
}: {
  icon?: IconName;
  title: ReactNode;
  sub?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center gap-2 px-6 py-14 text-center">
      <span className="grid size-12 place-items-center rounded-[13px] bg-surface-2 text-ink-faint">
        <Icon name={icon} size={22} />
      </span>
      <div className="mt-1 text-[14px] font-semibold text-ink-soft">{title}</div>
      {sub && <p className="max-w-90 text-[12.5px] leading-relaxed text-ink-muted">{sub}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}
