"use client";

import Link from "next/link";
import { usePathname, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";

import { useKbStore } from "@/entities/knowledge-base";
import { workshopApi, type WorkshopConversationSummary } from "@/shared/api";
import { cn } from "@/shared/lib/utils";
import { useI18n } from "@/shared/providers/providers";
import { Icon } from "@/shared/ui/icon";
import {
  collapseHistory,
  HISTORY_PAGE_SIZE,
  onWorkshopHistoryChanged,
  parseConversationId,
} from "../model/history";

export function WorkshopHistoryNav() {
  const { active } = useKbStore();
  return <WorkshopHistoryNavState key={active?.path ?? ""} />;
}

function WorkshopHistoryNavState() {
  const { t } = useI18n();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const activeId =
    pathname === "/workshop" ? parseConversationId(searchParams.get("conversation")) : null;
  const [items, setItems] = useState<WorkshopConversationSummary[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const requestRef = useRef(0);

  const loadFirstPage = useCallback(async () => {
    const request = ++requestRef.current;
    try {
      const page = await workshopApi.listConversations(0);
      if (request !== requestRef.current) return;
      setItems(page.items);
      setHasMore(page.hasMore);
      setError(false);
    } catch {
      if (request !== requestRef.current) return;
      setItems([]);
      setHasMore(false);
      setError(true);
    } finally {
      if (request === requestRef.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    let current = true;
    queueMicrotask(() => {
      if (current) void loadFirstPage();
    });
    const unsubscribe = onWorkshopHistoryChanged(() => void loadFirstPage());
    return () => {
      current = false;
      requestRef.current += 1;
      unsubscribe();
    };
  }, [loadFirstPage]);

  async function loadMore() {
    if (loading || !hasMore) return;
    const request = ++requestRef.current;
    setLoading(true);
    setError(false);
    try {
      const page = await workshopApi.listConversations(items.length);
      if (request !== requestRef.current) return;
      setItems((current) => [...current, ...page.items]);
      setHasMore(page.hasMore);
    } catch {
      if (request !== requestRef.current) return;
      setError(true);
    } finally {
      if (request === requestRef.current) setLoading(false);
    }
  }

  function collapse() {
    setItems((current) => collapseHistory(current));
    setHasMore(true);
  }

  if (items.length === 0 && !error) return null;

  return (
    <div className="ml-7 border-l border-line pl-2.5">
      <div className="mb-1 px-2 font-mono text-[10px] font-bold tracking-wider text-ink-faint uppercase">
        {t("workshop.history")}
      </div>
      <div className="flex flex-col gap-0.5">
        {items.map((item) => (
          <Link
            key={item.id}
            href={`/workshop?conversation=${item.id}`}
            aria-current={activeId === item.id ? "page" : undefined}
            className={cn(
              "flex min-w-0 items-center gap-1.5 rounded-md px-2 py-1.5 text-[12px] transition-colors",
              activeId === item.id
                ? "bg-surface text-ink"
                : "text-ink-muted hover:bg-surface-2 hover:text-ink"
            )}
          >
            <Icon name="chat" size={12} />
            <span className="truncate">{item.title}</span>
          </Link>
        ))}
      </div>
      {error ? (
        <div className="px-2 py-1 text-[11px] text-brand">{t("workshop.historyError")}</div>
      ) : null}
      <div className="mt-1 flex gap-2 px-2">
        {hasMore ? (
          <button
            type="button"
            disabled={loading}
            onClick={() => void loadMore()}
            className="text-[11px] font-semibold text-ink-muted hover:text-ink disabled:opacity-40"
          >
            {t("workshop.historyMore")}
          </button>
        ) : null}
        {items.length > HISTORY_PAGE_SIZE ? (
          <button
            type="button"
            onClick={collapse}
            className="text-[11px] font-semibold text-ink-muted hover:text-ink"
          >
            {t("workshop.historyCollapse")}
          </button>
        ) : null}
      </div>
    </div>
  );
}
