"use client";

import { useMemo, useState } from "react";
import Link from "next/link";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Tag } from "@/components/eb/tag";
import { Button, buttonVariants } from "@/components/ui/button";
import { GRAPH_CATS, GRAPH_DATA } from "@/lib/data/mock";
import { cn } from "@/lib/utils";

export function GraphView() {
  const [selected, setSelected] = useState<string | null>("yancha");
  const selectedNode = GRAPH_DATA.nodes.find((node) => node.id === selected);
  const neighbours = useMemo(() => {
    if (!selected) return [];
    return GRAPH_DATA.edges
      .filter(([a, b]) => a === selected || b === selected)
      .map(([a, b]) => GRAPH_DATA.nodes.find((node) => node.id === (a === selected ? b : a)))
      .filter(Boolean);
  }, [selected]);

  return (
    <div className="view-enter">
      <PageHead
        eyebrow="关系图谱 · Graph"
        title="知识的关联网络"
        sub={`${GRAPH_DATA.nodes.length} 个概念、${GRAPH_DATA.edges.length} 条双向链接，由关系图谱自动编织。`}
        right={
          <>
            <Button variant="outline" className="border-line-strong bg-surface">
              <Icon name="layers" size={17} />
              重新布局
            </Button>
            <Link
              href="/wiki"
              className={cn(
                buttonVariants({ variant: "outline" }),
                "border-line-strong bg-surface"
              )}
            >
              <Icon name="book" size={17} />
              列表视图
            </Link>
          </>
        }
      />
      <div className="grid grid-cols-[1fr_322px] gap-4.5">
        <Panel pad={0} className="relative overflow-hidden">
          <div className="absolute top-3.5 left-3.5 z-10 flex flex-wrap gap-1.75">
            {Object.entries(GRAPH_CATS).map(([cat, color]) => (
              <span
                key={cat}
                className="flex items-center gap-1.5 rounded-full border border-line bg-surface px-2.5 py-1 font-mono text-[11.5px] font-semibold text-ink-soft"
              >
                <span className="size-2 rounded-full" style={{ background: color }} />
                {cat}
              </span>
            ))}
          </div>
          <svg
            viewBox="0 0 1100 680"
            className="block w-full bg-[radial-gradient(circle_at_50%_40%,var(--surface),var(--surface-2))]"
          >
            {GRAPH_DATA.edges.map(([a, b]) => {
              const from = GRAPH_DATA.nodes.find((node) => node.id === a)!;
              const to = GRAPH_DATA.nodes.find((node) => node.id === b)!;
              const active = selected && (a === selected || b === selected);
              return (
                <line
                  key={`${a}-${b}`}
                  x1={from.x * 11}
                  y1={from.y * 6.8}
                  x2={to.x * 11}
                  y2={to.y * 6.8}
                  stroke={active ? "var(--brand)" : "var(--line-strong)"}
                  strokeWidth={active ? 2 : 1}
                  opacity={active ? 0.85 : 0.45}
                />
              );
            })}
            {GRAPH_DATA.nodes.map((node) => {
              const active = node.id === selected;
              const color = GRAPH_CATS[node.cat];
              return (
                <g key={node.id} onClick={() => setSelected(node.id)} className="cursor-pointer">
                  {active && (
                    <circle
                      cx={node.x * 11}
                      cy={node.y * 6.8}
                      r="48"
                      fill="var(--brand)"
                      opacity=".12"
                    />
                  )}
                  <circle
                    cx={node.x * 11}
                    cy={node.y * 6.8}
                    r={active ? 22 : 16}
                    fill={color}
                    stroke="var(--surface)"
                    strokeWidth="3"
                  />
                  <text
                    x={node.x * 11}
                    y={node.y * 6.8 + 34}
                    textAnchor="middle"
                    className="fill-ink font-serif text-[14px] font-semibold"
                  >
                    {node.label}
                  </text>
                </g>
              );
            })}
          </svg>
        </Panel>
        <Panel>
          {selectedNode && (
            <>
              <Tag tone="line">
                <span
                  className="size-1.75 rounded-full"
                  style={{ background: GRAPH_CATS[selectedNode.cat] }}
                />
                {selectedNode.cat}
              </Tag>
              <h2 className="mt-3 font-serif text-[28px] font-semibold text-ink">
                {selectedNode.label}
              </h2>
              <div className="mt-1.5 font-mono text-[12.5px] text-ink-muted">
                {neighbours.length} 个连接 · 直接连接
              </div>
              <div className="mt-5 grid gap-1.5">
                {neighbours.map(
                  (node) =>
                    node && (
                      <button
                        key={node.id}
                        onClick={() => setSelected(node.id)}
                        className="flex items-center gap-2.5 rounded-lg border border-line bg-surface-2 px-3 py-2 text-left"
                      >
                        <span
                          className="size-2 rounded-full"
                          style={{ background: GRAPH_CATS[node.cat] }}
                        />
                        <span className="text-[13.5px] font-semibold text-ink">{node.label}</span>
                        <span className="ml-auto font-mono text-[11px] text-ink-faint">
                          {node.cat}
                        </span>
                      </button>
                    )
                )}
              </div>
              <Button className="mt-4 w-full">
                <Icon name="spark" size={15} />
                AI 寻找更多关联
              </Button>
            </>
          )}
        </Panel>
      </div>
    </div>
  );
}
