"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Tag } from "@/components/eb/tag";
import { EmptyState } from "@/components/eb/empty-state";
import { buttonVariants } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { graph as fetchGraph } from "@/lib/tauri/client";
import { useKbStore } from "@/lib/kb/store";
import type { GraphData, GraphNode } from "@/lib/data/types";
import { cn } from "@/lib/utils";

// カテゴリへ順番に割り当てる配色（カテゴリ自体はユーザーデータ由来）。
const CAT_PALETTE = [
  "var(--ai)",
  "var(--brand)",
  "#9b5a6b",
  "var(--gold)",
  "#5e7e8b",
  "#7a5ae0",
];

// 派生インデックスのノード（座標なし）に円環レイアウトを与える。x,y は 0..100 空間。
function layout(nodes: { path: string; title: string; cat: string }[]): GraphNode[] {
  const n = Math.max(nodes.length, 1);
  return nodes.map((node, i) => {
    const angle = (2 * Math.PI * i) / n - Math.PI / 2;
    return {
      id: node.path,
      label: node.title,
      cat: node.cat || "uncategorized",
      x: 50 + 38 * Math.cos(angle),
      y: 50 + 40 * Math.sin(angle),
    };
  });
}

export function GraphView() {
  const { t } = useI18n();
  const { available } = useKbStore();
  const [data, setData] = useState<GraphData>({ nodes: [], edges: [] });
  const [selected, setSelected] = useState<string | null>(null);

  useEffect(() => {
    if (!available) return;
    void (async () => {
      const g = await fetchGraph();
      setData({ nodes: layout(g.nodes), edges: g.edges });
    })();
  }, [available]);

  const catColors = useMemo(() => {
    const cats = [...new Set(data.nodes.map((node) => node.cat))];
    return Object.fromEntries(cats.map((cat, i) => [cat, CAT_PALETTE[i % CAT_PALETTE.length]]));
  }, [data]);
  const selectedNode = data.nodes.find((node) => node.id === selected);
  const neighbours = useMemo(() => {
    if (!selected) return [];
    return data.edges
      .filter(([a, b]) => a === selected || b === selected)
      .map(([a, b]) => data.nodes.find((node) => node.id === (a === selected ? b : a)))
      .filter(Boolean);
  }, [selected, data]);

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("graph.eyebrow")}
        title={t("graph.title")}
        sub={t("graph.sub", { nodes: data.nodes.length, edges: data.edges.length })}
        right={
          <Link
            href="/wiki"
            className={cn(buttonVariants({ variant: "outline" }), "border-line-strong bg-surface")}
          >
            <Icon name="book" size={17} />
            {t("graph.list")}
          </Link>
        }
      />
      {data.nodes.length === 0 ? (
        <Panel pad={0}>
          <EmptyState icon="graph" title={t("empty.graph")} sub={t("empty.graph.sub")} />
        </Panel>
      ) : (
        <div className="grid grid-cols-[1fr_322px] gap-4.5">
          <Panel pad={0} className="relative overflow-hidden">
            <div className="absolute top-3.5 left-3.5 z-10 flex flex-wrap gap-1.75">
              {Object.entries(catColors).map(([cat, color]) => (
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
              {data.edges.map(([a, b]) => {
                const from = data.nodes.find((node) => node.id === a);
                const to = data.nodes.find((node) => node.id === b);
                if (!from || !to) return null;
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
              {data.nodes.map((node) => {
                const active = node.id === selected;
                const color = catColors[node.cat];
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
                    style={{ background: catColors[selectedNode.cat] }}
                  />
                  {selectedNode.cat}
                </Tag>
                <h2 className="mt-3 font-serif text-[28px] font-semibold text-ink">
                  {selectedNode.label}
                </h2>
                <div className="mt-1.5 font-mono text-[12.5px] text-ink-muted">
                  {t("graph.connections", { count: neighbours.length })}
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
                            style={{ background: catColors[node.cat] }}
                          />
                          <span className="text-[13.5px] font-semibold text-ink">
                            {node.label}
                          </span>
                          <span className="ml-auto font-mono text-[11px] text-ink-faint">
                            {node.cat}
                          </span>
                        </button>
                      )
                  )}
                </div>
              </>
            )}
          </Panel>
        </div>
      )}
    </div>
  );
}
