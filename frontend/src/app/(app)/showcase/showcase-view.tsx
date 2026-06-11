import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Tag } from "@/components/eb/tag";
import { Button } from "@/components/ui/button";
import { DEPLOY_HISTORY, WIKI } from "@/lib/data/mock";

export function ShowcaseView() {
  return (
    <div className="view-enter">
      <PageHead
        eyebrow="展示层 · Showcase"
        title="对外发布的知识门户"
        sub="把知识库的一部分发布为公开网站，会员可浏览、搜索、收藏，并一键部署到云端。"
      />
      <div className="grid grid-cols-[1.1fr_0.9fr] gap-5">
        <Panel className="overflow-hidden" pad={0}>
          <div className="bg-ink px-8 py-7 text-paper">
            <div className="mb-12 flex items-center justify-between">
              <div className="font-serif text-xl font-semibold">茶语知识库</div>
              <Button variant="secondary" size="sm">
                会员登录
              </Button>
            </div>
            <h2 className="max-w-120 font-serif text-[42px] leading-none font-medium">
              一座关于茶的 <span className="text-brand-soft italic">私人知识库</span>
            </h2>
            <p className="mt-4 max-w-110 text-[15px] leading-relaxed text-paper/75">
              从制茶工艺到冲泡技巧，沉淀十余年的经验。也可以直接问我们的 AI 助手。
            </p>
            <div className="mt-6 flex max-w-110 items-center gap-3 rounded-xl bg-paper/10 px-4 py-3 text-paper/65">
              <Icon name="search" size={17} />
              问点什么，比如“岩茶怎么醒茶”…
            </div>
          </div>
          <div className="grid grid-cols-3 gap-3 bg-surface p-5">
            {WIKI.slice(0, 3).map((entry) => (
              <div key={entry.id} className="rounded-xl border border-line bg-surface-2 p-4">
                <Tag tone="muted">{entry.cat}</Tag>
                <div className="mt-3 font-serif text-xl font-semibold">{entry.title}</div>
                <p className="mt-2 line-clamp-2 text-[12.5px] leading-relaxed text-ink-muted">
                  {entry.excerpt}
                </p>
              </div>
            ))}
          </div>
        </Panel>

        <div className="grid gap-4">
          <Panel>
            <div className="mb-4 flex items-center justify-between">
              <div>
                <div className="text-[15px] font-bold text-ink">云端发布</div>
                <div className="text-[12.5px] text-ink-muted">由发布类插件驱动</div>
              </div>
              <Tag tone="ai">已上线</Tag>
            </div>
            <div className="rounded-xl border border-line bg-surface-2 p-4">
              <div className="flex items-center gap-3">
                <span className="grid size-10 place-items-center rounded-[10px] bg-ink text-paper">
                  <Icon name="globe" size={19} />
                </span>
                <div className="flex-1">
                  <div className="font-semibold">Vercel</div>
                  <div className="font-mono text-[11px] text-ink-faint">全球 CDN · 自动 HTTPS</div>
                </div>
                <Tag tone="accent">推荐</Tag>
              </div>
              <Button className="mt-4 w-full">
                <Icon name="upload" size={15} />
                重新部署
              </Button>
            </div>
          </Panel>
          <Panel>
            <div className="mb-3 text-[15px] font-bold text-ink">部署历史</div>
            <div className="grid gap-2">
              {DEPLOY_HISTORY.map((item) => (
                <div
                  key={item.ver}
                  className="flex items-center gap-3 rounded-lg border border-line bg-surface-2 px-3 py-2.5"
                >
                  <Tag
                    tone={
                      item.status === "error" ? "accent" : item.status === "live" ? "ai" : "muted"
                    }
                  >
                    {item.ver}
                  </Tag>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13px] font-semibold">{item.commit}</div>
                    <div className="font-mono text-[11px] text-ink-faint">
                      {item.when} · {item.dur}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </Panel>
        </div>
      </div>
    </div>
  );
}
