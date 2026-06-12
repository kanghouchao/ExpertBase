"use client";

import { useState } from "react";
import Link from "next/link";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Ring } from "@/components/eb/ring";
import { Tag } from "@/components/eb/tag";
import { Button, buttonVariants } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { WIKI, WIKI_CATS, type WikiEntry } from "@/lib/data/mock";
import { qualityLabel, wikiCategoryLabel } from "@/lib/i18n/data";
import { cn } from "@/lib/utils";
import { SegTabs } from "../_components/seg-tabs";

function EntryCard({ entry, onOpen }: { entry: WikiEntry; onOpen: (entry: WikiEntry) => void }) {
  const { t } = useI18n();

  return (
    <button onClick={() => onOpen(entry)} className="block h-full text-left">
      <Panel hover className="relative flex h-full flex-col">
        {entry.orphan && (
          <Icon name="flag" size={15} className="absolute top-3.5 right-3.5 text-brand" />
        )}
        <div className="mb-2 flex items-baseline gap-2 pr-5">
          <h3 className="font-serif text-[21px] leading-tight font-semibold text-ink">
            {entry.title}
          </h3>
          <span className="font-serif text-[13px] text-ink-faint italic">{entry.en}</span>
        </div>
        <p className="line-clamp-3 flex-1 text-[13px] leading-relaxed text-ink-soft">
          {entry.excerpt}
        </p>
        <div className="mt-3.5 flex items-center gap-2 font-mono text-[11.5px] text-ink-faint">
          <Tag tone="muted">{wikiCategoryLabel(entry.cat, t)}</Tag>
          <span className="flex items-center gap-1">
            <Icon name="link" size={12} />
            {entry.links}
          </span>
          <span className="flex items-center gap-1">
            <Icon name="arrowR" size={12} />
            {entry.backlinks}
          </span>
          <span
            className="ml-auto size-1.75 rounded-full"
            style={{
              background:
                entry.quality >= 85
                  ? "var(--ai)"
                  : entry.quality >= 70
                    ? "var(--gold)"
                    : "var(--brand)",
            }}
          />
        </div>
      </Panel>
    </button>
  );
}

export function WikiView() {
  const { t } = useI18n();
  const [category, setCategory] = useState<(typeof WIKI_CATS)[number]>("全部");
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<WikiEntry | null>(null);
  const list = WIKI.filter(
    (entry) =>
      (category === "全部" || entry.cat === category) &&
      (!query || entry.title.includes(query) || entry.excerpt.includes(query))
  );

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("wiki.eyebrow")}
        title={t("wiki.title")}
        sub={t("wiki.sub")}
        right={
          <>
            <Link
              href="/graph"
              className={cn(
                buttonVariants({ variant: "outline" }),
                "border-line-strong bg-surface"
              )}
            >
              <Icon name="graph" size={17} />
              {t("wiki.graph")}
            </Link>
            <Link
              href="/workshop"
              className={cn(
                buttonVariants({ variant: "outline" }),
                "border-line-strong bg-surface"
              )}
            >
              <Icon name="shield" size={17} />
              {t("wiki.optimize")}
            </Link>
          </>
        }
      />
      <div className="mb-4.5 flex flex-wrap items-center gap-3.5">
        <SegTabs
          tabs={WIKI_CATS}
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
      <div className="grid grid-cols-[repeat(auto-fill,minmax(258px,1fr))] gap-3.5">
        {list.map((entry) => (
          <EntryCard key={entry.id} entry={entry} onOpen={setOpen} />
        ))}
      </div>

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
              <Button variant="ghost" size="sm">
                <Icon name="edit" size={15} />
                {t("wiki.edit")}
              </Button>
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
              <div className="mb-1.5 flex items-baseline gap-3">
                <h1 className="font-serif text-[38px] font-semibold tracking-normal text-ink">
                  {open.title}
                </h1>
                <span className="font-serif text-xl text-ink-muted italic">{open.en}</span>
              </div>
              <div className="mb-5.5 flex gap-3.5 font-mono text-[12.5px] text-ink-faint">
                <span>{t("wiki.updated", { date: open.updated })}</span>
                <span>·</span>
                <span>{t("wiki.words", { count: open.words })}</span>
                <span>·</span>
                <span>{t("wiki.backlinks", { links: open.links, backlinks: open.backlinks })}</span>
              </div>
              <Panel className="mb-6 flex items-center gap-4">
                <Ring value={open.quality} size={46} sw={5} />
                <div className="flex-1">
                  <div className="text-[13.5px] font-semibold text-ink">
                    {t("wiki.quality", { level: qualityLabel(open.quality, t) })}
                  </div>
                  <div className="mt-0.5 text-[12.5px] text-ink-muted">
                    {t(open.orphan ? "wiki.integrity.orphan" : "wiki.integrity.ok")}
                  </div>
                </div>
                <Button>
                  <Icon name="spark" size={15} />
                  {t("wiki.aiOptimize")}
                </Button>
              </Panel>
              <article className="text-[15.5px] leading-[1.85] text-ink-soft">
                <p>{open.excerpt}</p>
                <p className="mt-4">{t("wiki.body.extra")}</p>
                <blockquote className="mt-4 rounded-r-[10px] border-l-3 border-brand bg-surface px-4.5 py-3.5 italic">
                  {t("wiki.quote")}
                </blockquote>
              </article>
              <div className="mt-7 grid grid-cols-2 gap-5">
                <div>
                  <div className="mb-2.5 font-mono text-xs font-bold tracking-widest text-ink-muted uppercase">
                    {t("wiki.related")}
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {open.related.length ? (
                      open.related.map((item) => (
                        <Tag key={item} tone="ai">
                          [[{item}]]
                        </Tag>
                      ))
                    ) : (
                      <span className="text-[13px] text-ink-faint italic">
                        {t("wiki.emptyRelated")}
                      </span>
                    )}
                  </div>
                </div>
                <div>
                  <div className="mb-2.5 font-mono text-xs font-bold tracking-widest text-ink-muted uppercase">
                    {t("wiki.backlinks.title")}
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {WIKI.filter((entry) => entry.related.includes(open.title)).map((entry) => (
                      <Tag key={entry.id} tone="line">
                        {entry.title}
                      </Tag>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          </aside>
        </div>
      )}
    </div>
  );
}
