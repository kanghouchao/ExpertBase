"use client";

import { useState } from "react";
import Link from "next/link";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Ring } from "@/components/eb/ring";
import { Tag } from "@/components/eb/tag";
import { Button, buttonVariants } from "@/components/ui/button";
import { WIKI, WIKI_CATS, type WikiEntry } from "@/lib/data/mock";
import { cn } from "@/lib/utils";
import { SegTabs } from "../_components/seg-tabs";

function EntryCard({ entry, onOpen }: { entry: WikiEntry; onOpen: (entry: WikiEntry) => void }) {
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
          <Tag tone="muted">{entry.cat}</Tag>
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
        eyebrow="知识库 · Wiki"
        title="你的私人知识库"
        sub="48 个条目，由双向链接编织成网。需要修补或优化时，到工作坊集中处理。"
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
              关系图谱
            </Link>
            <Link
              href="/workshop"
              className={cn(
                buttonVariants({ variant: "outline" }),
                "border-line-strong bg-surface"
              )}
            >
              <Icon name="shield" size={17} />
              去工作坊优化
            </Link>
          </>
        }
      />
      <div className="mb-4.5 flex flex-wrap items-center gap-3.5">
        <SegTabs tabs={WIKI_CATS} value={category} onChange={setCategory} />
        <div className="flex-1" />
        <label className="flex w-60 items-center gap-2 rounded-[10px] border border-line-strong bg-surface px-3 py-2">
          <Icon name="search" size={16} className="text-ink-muted" />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="搜索条目…"
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
              <Tag tone="accent">{open.cat}</Tag>
              <div className="flex-1" />
              <Button variant="ghost" size="sm">
                <Icon name="edit" size={15} />
                编辑
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
                <span>更新于 {open.updated}</span>
                <span>·</span>
                <span>{open.words} 字</span>
                <span>·</span>
                <span>
                  {open.links} / {open.backlinks} 反链
                </span>
              </div>
              <Panel className="mb-6 flex items-center gap-4">
                <Ring value={open.quality} size={46} sw={5} />
                <div className="flex-1">
                  <div className="text-[13.5px] font-semibold text-ink">
                    条目质量 {open.quality >= 85 ? "优秀" : open.quality >= 70 ? "良好" : "待完善"}
                  </div>
                  <div className="mt-0.5 text-[12.5px] text-ink-muted">
                    {open.orphan ? "孤立条目 · 暂无双向链接" : "结构完整 · 引用充分"}
                  </div>
                </div>
                <Button>
                  <Icon name="spark" size={15} />
                  AI 优化
                </Button>
              </Panel>
              <article className="text-[15.5px] leading-[1.85] text-ink-soft">
                <p>{open.excerpt}</p>
                <p className="mt-4">
                  在实际操作中，这一环节往往决定成品上限。经验丰富的匠人会依据气候、原料状态与目标风味做出细微调整。
                </p>
                <blockquote className="mt-4 rounded-r-[10px] border-l-3 border-brand bg-surface px-4.5 py-3.5 italic">
                  “机器能做大概，但顶级的还是要靠手上的感觉。” —— 摘自原始访谈录音
                </blockquote>
              </article>
              <div className="mt-7 grid grid-cols-2 gap-5">
                <div>
                  <div className="mb-2.5 font-mono text-xs font-bold tracking-[0.1em] text-ink-muted uppercase">
                    关联条目 →
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {open.related.length ? (
                      open.related.map((item) => (
                        <Tag key={item} tone="ai">
                          [[{item}]]
                        </Tag>
                      ))
                    ) : (
                      <span className="text-[13px] text-ink-faint italic">暂无 · AI 建议关联</span>
                    )}
                  </div>
                </div>
                <div>
                  <div className="mb-2.5 font-mono text-xs font-bold tracking-[0.1em] text-ink-muted uppercase">
                    ← 反向链接
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
