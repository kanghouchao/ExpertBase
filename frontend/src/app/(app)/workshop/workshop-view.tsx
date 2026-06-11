import Link from "next/link";

import { Icon, type IconName } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Ring } from "@/components/eb/ring";
import { Tag } from "@/components/eb/tag";
import { Button } from "@/components/ui/button";
import { LINT, PENDING, RAW_MATERIALS, STATS, type TagTone } from "@/lib/data/mock";
import { cn } from "@/lib/utils";
import { MaterialRow } from "../_components/material-row";

function QueueCard({
  icon,
  title,
  body,
  tone = "accent",
  href,
}: {
  icon: IconName;
  title: string;
  body: string;
  tone?: TagTone;
  href: string;
}) {
  return (
    <Link href={href}>
      <Panel hover className="h-full">
        <div className="mb-3 flex items-start gap-3">
          <span
            className={cn(
              "grid size-9 place-items-center rounded-[10px]",
              tone === "ai" ? "bg-ai-wash text-ai" : "bg-brand-wash text-brand"
            )}
          >
            <Icon name={icon} size={18} />
          </span>
          <div>
            <div className="font-semibold text-ink">{title}</div>
            <p className="mt-1 text-[13px] leading-relaxed text-ink-muted">{body}</p>
          </div>
        </div>
        <div className="flex items-center justify-between border-t border-line pt-3">
          <Tag tone={tone}>AI 辅助</Tag>
          <Icon name="arrowR" size={15} className="text-ink-faint" />
        </div>
      </Panel>
    </Link>
  );
}

export function WorkshopView() {
  return (
    <div className="view-enter">
      <PageHead
        eyebrow="工作坊 · Workshop"
        title="整理与加工"
        sub="把素材加工成知识，把已有知识修补完善。你定方向，AI 在一旁辅助。"
        right={
          <Button variant="outline" className="border-line-strong bg-surface">
            <Icon name="scan" size={17} />
            重新体检
          </Button>
        }
      />

      <Panel pad={0} className="mb-5.5 overflow-hidden border-ai-soft">
        <div className="flex items-center gap-5 px-6 py-4.5">
          <Ring value={STATS.health} size={58} sw={6} />
          <div className="flex-1">
            <div className="text-[15px] font-bold text-ink">
              知识库健康度 {STATS.health >= 70 ? "良好" : "待提升"}
            </div>
            <div className="mt-1 text-[13px] text-ink-muted">
              AI 体检 12 分钟前 · 这里集中处理素材加工与知识修补
            </div>
          </div>
          <div className="flex gap-7 pr-2">
            <div>
              <div className="font-serif text-[26px] leading-none font-bold text-brand">
                {PENDING}
              </div>
              <div className="mt-1 font-mono text-[11px] text-ink-faint">待加工素材</div>
            </div>
            <div>
              <div className="font-serif text-[26px] leading-none font-bold text-ai">
                {LINT.length}
              </div>
              <div className="mt-1 font-mono text-[11px] text-ink-faint">待优化知识</div>
            </div>
          </div>
        </div>
      </Panel>

      <div className="grid grid-cols-[1fr_1fr] gap-5">
        <div>
          <h2 className="mb-3 font-mono text-[12px] font-bold tracking-[0.12em] text-ink-muted uppercase">
            待加工素材
          </h2>
          <Panel pad={0}>
            {RAW_MATERIALS.filter((item) => item.status !== "processed").map((item) => (
              <MaterialRow
                key={item.id}
                material={item}
                action={
                  <Button size="sm" variant="outline">
                    加工
                  </Button>
                }
              />
            ))}
          </Panel>
        </div>
        <div>
          <h2 className="mb-3 font-mono text-[12px] font-bold tracking-[0.12em] text-ink-muted uppercase">
            待优化知识
          </h2>
          <div className="grid gap-3">
            {LINT.map((issue) => (
              <QueueCard
                key={issue.id}
                icon={
                  issue.type === "orphan"
                    ? "flag"
                    : issue.type === "dup"
                      ? "merge"
                      : issue.type === "stale"
                        ? "clock"
                        : "edit"
                }
                title={issue.title}
                body={issue.detail}
                tone={issue.sev === "high" ? "accent" : "ai"}
                href="/wiki"
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
