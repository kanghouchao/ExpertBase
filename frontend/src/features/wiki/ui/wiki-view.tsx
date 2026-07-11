"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

import { Icon } from "@/shared/ui/icon";
import { PageHead } from "@/shared/ui/page-head";
import { Panel } from "@/shared/ui/panel";
import { Tag } from "@/shared/ui/tag";
import { Button, buttonVariants } from "@/shared/ui/button";
import { Markdown } from "@/shared/ui/markdown";
import { useI18n } from "@/shared/providers/providers";
import { EmptyState } from "@/shared/ui/empty-state";
import {
  backlinks as fetchBacklinks,
  listEntries,
  orphans as fetchOrphans,
  readEntry,
  saveEntry,
  searchEntries,
  type EntryRef,
} from "@/shared/api/tauri/client";
import { wikiCategoryLabel } from "@/shared/i18n/data";
import { cn } from "@/shared/lib/utils";
import { useKbStore } from "@/entities/knowledge-base";
import { SegTabs } from "@/shared/ui/seg-tabs";

// カテゴリはユーザーデータ由来。「全部」だけはセンチネルとして常に先頭に置く。
const ALL_CAT = "全部";

function EntryCard({
  entry,
  orphan,
  onOpen,
}: {
  entry: EntryRef & { excerpt?: string };
  orphan?: boolean;
  onOpen: (entry: EntryRef) => void;
}) {
  const { t } = useI18n();

  return (
    <button onClick={() => onOpen(entry)} className="block h-full text-left">
      <Panel hover className="relative flex h-full flex-col">
        {orphan && (
          <span className="absolute top-3.5 right-3.5 text-brand" title={t("wiki.orphan")}>
            <Icon name="flag" size={15} />
          </span>
        )}
        <div className="mb-2 flex items-baseline gap-2 pr-5">
          <h3 className="font-serif text-[21px] leading-tight font-semibold text-ink">
            {entry.title}
          </h3>
        </div>
        <p className="line-clamp-3 flex-1 text-[13px] leading-relaxed text-ink-soft">
          {entry.excerpt}
        </p>
        <div className="mt-3.5 flex items-center gap-2 font-mono text-[11.5px] text-ink-faint">
          <Tag tone="muted">{wikiCategoryLabel(entry.cat, t)}</Tag>
        </div>
      </Panel>
    </button>
  );
}

export function WikiView() {
  const { t } = useI18n();
  const { available } = useKbStore();
  const [entries, setEntries] = useState<EntryRef[]>([]);
  const [orphanPaths, setOrphanPaths] = useState<Set<string>>(new Set());
  const [hits, setHits] = useState<(EntryRef & { excerpt: string })[]>([]);
  const [category, setCategory] = useState<string>(ALL_CAT);
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<EntryRef | null>(null);
  const [body, setBody] = useState("");
  const [links, setLinks] = useState<EntryRef[]>([]);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");

  async function reload() {
    const [refs, orphanRefs] = await Promise.all([listEntries(), fetchOrphans()]);
    setEntries(refs.map((ref) => ({ ...ref, cat: ref.cat || "uncategorized" })));
    setOrphanPaths(new Set(orphanRefs.map((o) => o.path)));
  }

  useEffect(() => {
    if (!available) return;
    void (async () => {
      const [refs, orphanRefs] = await Promise.all([listEntries(), fetchOrphans()]);
      setEntries(refs.map((ref) => ({ ...ref, cat: ref.cat || "uncategorized" })));
      setOrphanPaths(new Set(orphanRefs.map((o) => o.path)));
    })();
  }, [available]);

  // 全文検索（trigram のため 3 文字以上で発火）。
  useEffect(() => {
    const q = query.trim();
    void (async () => {
      if (q.length < 3) {
        setHits([]);
        return;
      }
      const found = await searchEntries(q);
      setHits(
        found.map((hit) => ({
          path: hit.path,
          title: hit.title,
          cat: "uncategorized",
          excerpt: hit.excerpt,
        }))
      );
    })();
  }, [query]);

  const searching = query.trim().length >= 3;
  const cats = [ALL_CAT, ...new Set(entries.map((entry) => entry.cat))];
  const list = searching
    ? hits
    : entries.filter((entry) => category === ALL_CAT || entry.cat === category);

  async function openEntry(entry: EntryRef) {
    setOpen(entry);
    setEditing(false);
    setBody("");
    setLinks([]);
    const [content, back] = await Promise.all([readEntry(entry.path), fetchBacklinks(entry.title)]);
    setBody(content);
    setLinks(back);
  }

  async function handleSave() {
    if (!open) return;
    await saveEntry(open.path, draft);
    setBody(draft);
    setEditing(false);
    await reload();
  }

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("wiki.eyebrow")}
        title={t("wiki.title")}
        sub={t("wiki.sub")}
        right={
          <Link
            href="/graph"
            className={cn(buttonVariants({ variant: "outline" }), "border-line-strong bg-surface")}
          >
            <Icon name="graph" size={17} />
            {t("wiki.graph")}
          </Link>
        }
      />
      <div className="mb-4.5 flex flex-wrap items-center gap-3.5">
        <SegTabs
          tabs={cats}
          value={category}
          onChange={setCategory}
          label={(item) => wikiCategoryLabel(item, t)}
        />
        <div className="flex-1" />
        <label className="flex w-60 items-center gap-2 rounded-[10px] border border-line-strong bg-surface px-3 py-2">
          <Icon name="search" size={16} className="text-ink-muted" />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("wiki.search")}
            className="w-full bg-transparent text-[13.5px] text-ink outline-none"
          />
        </label>
      </div>
      {list.length === 0 ? (
        <Panel pad={0}>
          <EmptyState icon="book" title={t("empty.wiki")} sub={t("empty.wiki.sub")} />
        </Panel>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(258px,1fr))] gap-3.5">
          {list.map((entry) => (
            <EntryCard
              key={entry.path}
              entry={entry}
              orphan={orphanPaths.has(entry.path)}
              onOpen={openEntry}
            />
          ))}
        </div>
      )}

      {open && (
        <div
          className="fixed inset-0 z-50 flex justify-end bg-black/35 backdrop-blur-[3px]"
          onClick={() => setOpen(null)}
        >
          <aside
            className="h-full w-[min(640px,92vw)] overflow-auto bg-paper shadow-(--shadow-lg)"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="sticky top-0 z-10 flex items-center gap-3 border-b border-line bg-paper px-7 py-4.5">
              <Tag tone="accent">{wikiCategoryLabel(open.cat, t)}</Tag>
              <div className="flex-1" />
              {editing ? (
                <Button variant="ghost" size="sm" onClick={handleSave}>
                  <Icon name="check" size={15} />
                </Button>
              ) : (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setDraft(body);
                    setEditing(true);
                  }}
                >
                  <Icon name="edit" size={15} />
                  {t("wiki.edit")}
                </Button>
              )}
              <Button
                variant="outline"
                size="icon"
                className="border-line-strong bg-surface"
                onClick={() => setOpen(null)}
              >
                <Icon name="x" size={16} />
              </Button>
            </div>
            <div className="px-8 py-7">
              <h1 className="mb-5.5 font-serif text-[38px] font-semibold tracking-normal text-ink">
                {open.title}
              </h1>
              {editing ? (
                <textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  className="min-h-100 w-full resize-y rounded-xl border border-line-strong bg-surface-2 p-3.5 font-mono text-[13.5px] leading-relaxed text-ink outline-none"
                />
              ) : (
                <Markdown className="text-[15.5px] leading-[1.85] text-ink-soft">
                  {stripFrontmatter(body)}
                </Markdown>
              )}
              <div className="mt-7">
                <div className="mb-2.5 font-mono text-xs font-bold tracking-widest text-ink-muted uppercase">
                  {t("wiki.backlinks.title")}
                </div>
                <div className="flex flex-wrap gap-2">
                  {links.length ? (
                    links.map((entry) => (
                      <Tag key={entry.path} tone="line">
                        {entry.title}
                      </Tag>
                    ))
                  ) : (
                    <span className="text-[13px] text-ink-faint italic">
                      {t("wiki.emptyRelated")}
                    </span>
                  )}
                </div>
              </div>
            </div>
          </aside>
        </div>
      )}
    </div>
  );
}

// 条目の生 Markdown から frontmatter（先頭の --- ブロック）を外して本文だけ表示する。
function stripFrontmatter(markdown: string): string {
  if (!markdown.startsWith("---")) return markdown.trim();
  const end = markdown.indexOf("\n---", 3);
  if (end === -1) return markdown.trim();
  return markdown.slice(end + 4).trim();
}
