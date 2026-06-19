"use client";

import { useEffect, useState } from "react";

import { Icon } from "@/shared/ui/icon";
import { useI18n } from "@/shared/providers/providers";
import { cn } from "@/shared/lib/utils";

// 2 段階のインライン削除コントロール。ネイティブ confirm ダイアログの代わりに、
// 1 回目のクリックで武装（確認ボタンを表示）、2 回目で実行する。約 2.8 秒で自動解除。
export function DeleteButton({
  onDelete,
  title,
  className,
}: {
  onDelete: () => void;
  title?: string;
  className?: string;
}) {
  const { t } = useI18n();
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    if (!armed) return;
    const id = setTimeout(() => setArmed(false), 2800);
    return () => clearTimeout(id);
  }, [armed]);

  if (armed) {
    return (
      <span className="inline-flex flex-none items-center gap-1.5" onClick={(e) => e.stopPropagation()}>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            setArmed(false);
            onDelete();
          }}
          className="inline-flex h-7 items-center gap-1.5 rounded-lg border border-brand bg-brand px-2.5 text-[12.5px] font-semibold whitespace-nowrap text-white"
        >
          <Icon name="trash" size={14} />
          {t("c.confirmDel")}
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            setArmed(false);
          }}
          title={t("c.cancel")}
          aria-label={t("c.cancel")}
          className="grid size-7 flex-none place-items-center rounded-lg border border-line-strong bg-surface text-ink-muted"
        >
          <Icon name="x" size={15} />
        </button>
      </span>
    );
  }

  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        setArmed(true);
      }}
      title={title ?? t("c.delete")}
      aria-label={title ?? t("c.delete")}
      className={cn(
        "grid size-7 flex-none place-items-center rounded-lg border border-line bg-surface text-ink-faint transition-colors hover:border-brand-soft hover:text-brand",
        className
      )}
    >
      <Icon name="trash" size={15} />
    </button>
  );
}
