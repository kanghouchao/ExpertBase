"use client";

import { useEffect, useMemo, useReducer, useRef } from "react";
import Link from "next/link";

import { Icon } from "@/shared/ui/icon";
import { PageHead } from "@/shared/ui/page-head";
import { Panel } from "@/shared/ui/panel";
import { Tag } from "@/shared/ui/tag";
import { EmptyState } from "@/shared/ui/empty-state";
import { Button, buttonVariants } from "@/shared/ui/button";
import { useI18n } from "@/shared/providers/providers";
import { kbApi } from "@/shared/api";
import { useKbStore } from "@/entities/knowledge-base";
import { cn } from "@/shared/lib/utils";
import {
  H,
  W,
  activeNode,
  catOf,
  colorOf,
  deriveIndices,
  dimEdge,
  dimNode,
  dragTarget,
  edgeActive,
  initialState,
  isOrphan,
  labelFontSize,
  labelVisible,
  neighboursOf,
  nodeRadius,
  reduce,
} from "../model/graph-interaction";

// インタラクションの意味論（ドラッグ/クリック・pan/zoom・焦点淡化）は
// graph-interaction reducer が持つ。このコンポーネントは SVG の描画と
// DOM イベントの転送だけを担う。
export function GraphView() {
  const { t } = useI18n();
  const { available } = useKbStore();

  const [s, dispatch] = useReducer(reduce, initialState);
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!available) return;
    void (async () => {
      const g = await kbApi.graph();
      dispatch({ type: "graphLoaded", nodes: g.nodes, edges: g.edges });
    })();
  }, [available]);

  // relayout のアニメーション窓（650ms）。タイマーだけビューが持ち、畳む判定は reducer が持つ。
  useEffect(() => {
    if (!s.relaying) return;
    const timer = window.setTimeout(() => dispatch({ type: "relayoutSettled" }), 650);
    return () => window.clearTimeout(timer);
  }, [s.relaying]);

  // wheel は passive:false でネイティブに張る＝ページスクロールを止めてズームできる。
  // svg は空状態では描かれないので、出現に合わせて張り直す。
  const hasGraph = s.nodes.length > 0;
  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      dispatch({ type: "wheelZoom", deltaY: e.deltaY });
    };
    svg.addEventListener("wheel", onWheel, { passive: false });
    return () => svg.removeEventListener("wheel", onWheel);
  }, [hasGraph]);

  const idx = useMemo(() => deriveIndices(s.nodes, s.edges), [s.nodes, s.edges]);
  const active = activeNode(s);
  const dragId = dragTarget(s);
  const grabbing = s.mode !== null;
  const selectedNode = s.selected ? idx.nodeById[s.selected] : null;
  const neighbours = s.selected ? neighboursOf(idx, s.selected) : [];

  const rectOf = () => {
    const svg = svgRef.current;
    return svg ? svg.getBoundingClientRect() : { left: 0, top: 0, width: W, height: H };
  };
  const onMove = (e: React.MouseEvent) => {
    if (!s.mode) return;
    dispatch({ type: "pointerMove", x: e.clientX, y: e.clientY, rect: rectOf() });
  };

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("graph.eyebrow")}
        title={t("graph.title")}
        sub={t("graph.sub", { nodes: s.nodes.length, edges: s.edges.length })}
        right={
          <>
            <Button
              variant="outline"
              className="border-line-strong bg-surface"
              onClick={() => dispatch({ type: "relayout" })}
            >
              <Icon name="layers" size={17} />
              {t("graph.relayout")}
            </Button>
            <Button
              variant={s.showWeak ? "default" : "outline"}
              className={s.showWeak ? undefined : "border-line-strong bg-surface"}
              onClick={() => dispatch({ type: "toggleWeak" })}
            >
              <Icon name="flag" size={17} />
              {t("graph.weak")}
            </Button>
            <Link
              href="/wiki"
              className={cn(buttonVariants({ variant: "outline" }), "border-line-strong bg-surface")}
            >
              <Icon name="book" size={17} />
              {t("graph.list")}
            </Link>
          </>
        }
      />
      {!hasGraph ? (
        <Panel pad={0}>
          <EmptyState icon="graph" title={t("empty.graph")} sub={t("empty.graph.sub")} />
        </Panel>
      ) : (
        <div className={cn("grid gap-4.5", s.selected ? "grid-cols-[1fr_322px]" : "grid-cols-1")}>
          <Panel pad={0} className="relative overflow-hidden">
            {/* カテゴリ凡例 / 絞り込み */}
            <div className="absolute top-3.5 left-3.5 z-10 flex max-w-[74%] flex-wrap gap-1.75">
              {Object.entries(idx.catColors).map(([cat, color]) => {
                const on = s.catFilter === cat;
                return (
                  <button
                    key={cat}
                    type="button"
                    onClick={() => dispatch({ type: "toggleCatFilter", cat })}
                    className="flex items-center gap-1.5 rounded-full border border-line px-2.5 py-1 font-mono text-[11.5px] font-semibold transition-colors"
                    style={{ background: on ? color : "var(--surface)", color: on ? "#fff" : "var(--ink-soft)" }}
                  >
                    <span
                      className="size-2 rounded-full"
                      style={{ background: on ? "#fff" : color }}
                    />
                    {cat}
                  </button>
                );
              })}
            </div>

            {/* ズームコントロール */}
            <div className="absolute bottom-3.5 left-3.5 z-10 flex flex-col gap-1.5">
              {(
                [
                  ["+", () => dispatch({ type: "zoomStep", delta: 0.25 })],
                  ["−", () => dispatch({ type: "zoomStep", delta: -0.25 })],
                  ["⟳", () => dispatch({ type: "resetView" })],
                ] as const
              ).map(([label, fn]) => (
                <button
                  key={label}
                  type="button"
                  onClick={fn}
                  className="grid size-8 place-items-center rounded-[9px] border border-line bg-surface text-[16px] text-ink-soft shadow-(--shadow-sm) transition-colors hover:bg-surface-2"
                >
                  {label}
                </button>
              ))}
            </div>

            {/* 情報行 / 操作ヒント */}
            <div className="absolute right-4.5 bottom-4 z-10 font-mono text-[11.5px] text-ink-faint">
              {s.nodes.length} {t("graph.nodes")} · {s.edges.length} {t("graph.links")} ·{" "}
              {Math.round(s.view.zoom * 100)}%
            </div>
            <div className="absolute top-3.5 right-4 z-10 flex items-center gap-1.5 font-mono text-[11px] text-ink-faint">
              <Icon name="drag" size={13} />
              {t("graph.drag")}
            </div>

            <svg
              ref={svgRef}
              viewBox={`0 0 ${W} ${H}`}
              className="block w-full touch-none select-none"
              style={{
                background: "radial-gradient(circle at 50% 40%, var(--surface), var(--surface-2))",
                cursor: grabbing ? "grabbing" : "grab",
              }}
              onMouseDown={(e) => dispatch({ type: "bgDown", x: e.clientX, y: e.clientY })}
              onMouseMove={onMove}
              onMouseUp={() => dispatch({ type: "pointerUp" })}
              onMouseLeave={() => dispatch({ type: "pointerUp" })}
            >
              <defs>
                <radialGradient id="graph-glow">
                  <stop offset="0%" stopColor="var(--brand)" stopOpacity="0.22" />
                  <stop offset="100%" stopColor="var(--brand)" stopOpacity="0" />
                </radialGradient>
              </defs>
              <g transform={`translate(${s.view.panX},${s.view.panY}) scale(${s.view.zoom})`}>
                {/* エッジ */}
                {s.edges.map(([a, b], i) => {
                  const pa = s.pos[a];
                  const pb = s.pos[b];
                  if (!pa || !pb) return null;
                  const on = edgeActive(s, a, b);
                  const dim = dimEdge(s, idx, a, b);
                  return (
                    <line
                      key={i}
                      x1={pa.x}
                      y1={pa.y}
                      x2={pb.x}
                      y2={pb.y}
                      stroke={on ? "var(--brand)" : "var(--line-strong)"}
                      strokeWidth={on ? 2 : 1}
                      opacity={dim ? 0.1 : on ? 0.85 : 0.4}
                      style={{
                        transition: s.relaying
                          ? "all .6s cubic-bezier(.2,.7,.2,1)"
                          : "opacity .2s, stroke .2s",
                      }}
                    />
                  );
                })}
                {/* ノード（円） */}
                {s.nodes.map((n) => {
                  const p = s.pos[n.path];
                  if (!p) return null;
                  const r = nodeRadius(idx.deg[n.path] ?? 0);
                  const dim = dimNode(s, idx, n.path);
                  const on = n.path === active;
                  const orphan = isOrphan(idx, n.path);
                  const isDrag = n.path === dragId;
                  return (
                    <g
                      key={n.path}
                      style={{
                        cursor: "grab",
                        opacity: dim ? 0.18 : 1,
                        transition: s.relaying ? "transform .6s cubic-bezier(.2,.7,.2,1)" : "opacity .2s",
                      }}
                      onMouseEnter={() => dispatch({ type: "hoverEnter", id: n.path })}
                      onMouseLeave={() => dispatch({ type: "hoverLeave" })}
                      onMouseDown={(e) => {
                        e.stopPropagation();
                        dispatch({ type: "nodeDown", id: n.path, x: e.clientX, y: e.clientY, rect: rectOf() });
                      }}
                    >
                      {on && <circle cx={p.x} cy={p.y} r={r + 30} fill="url(#graph-glow)" />}
                      {s.showWeak && orphan && (
                        <circle
                          cx={p.x}
                          cy={p.y}
                          r={r + 6}
                          fill="none"
                          stroke="var(--brand)"
                          strokeWidth={1.4}
                          strokeDasharray="3 4"
                        />
                      )}
                      <circle
                        cx={p.x}
                        cy={p.y}
                        r={r}
                        fill={colorOf(idx, n.path)}
                        stroke={isDrag ? "var(--brand)" : "var(--surface)"}
                        strokeWidth={isDrag ? 3.5 : 2.5}
                        style={{
                          filter: on || isDrag ? "brightness(1.1)" : "none",
                          transition: s.relaying
                            ? "cx .6s cubic-bezier(.2,.7,.2,1), cy .6s cubic-bezier(.2,.7,.2,1)"
                            : "stroke .15s",
                        }}
                      />
                    </g>
                  );
                })}
                {/* ラベル（最終パス＝どの円にも隠れない。紙色の縁取りで可読性を確保） */}
                {s.nodes.map((n) => {
                  const p = s.pos[n.path];
                  if (!p) return null;
                  if (!labelVisible(s, idx, n.path)) return null;
                  const r = nodeRadius(idx.deg[n.path] ?? 0);
                  const on = n.path === active;
                  return (
                    <text
                      key={n.path}
                      x={p.x}
                      y={p.y + r + 14}
                      textAnchor="middle"
                      fontSize={labelFontSize(idx.deg[n.path] ?? 0)}
                      fontWeight={on ? 700 : 600}
                      className="font-serif"
                      fill="var(--ink)"
                      stroke="var(--surface)"
                      strokeWidth={4}
                      paintOrder="stroke"
                      strokeLinejoin="round"
                      style={{
                        pointerEvents: "none",
                        opacity: dimNode(s, idx, n.path) ? 0.18 : 1,
                        transition: s.relaying ? "all .6s cubic-bezier(.2,.7,.2,1)" : "opacity .2s",
                      }}
                    >
                      {n.title}
                    </text>
                  );
                })}
              </g>
            </svg>
          </Panel>

          {selectedNode && (
            <Panel>
              <div className="flex items-start justify-between">
                <Tag tone="line">
                  <span
                    className="size-1.75 rounded-full"
                    style={{ background: colorOf(idx, selectedNode.path) }}
                  />
                  {catOf(idx, selectedNode.path)}
                </Tag>
                <button
                  type="button"
                  onClick={() => dispatch({ type: "clearSelection" })}
                  className="text-ink-muted transition-colors hover:text-ink"
                >
                  <Icon name="x" size={16} />
                </button>
              </div>
              <h2 className="mt-3 font-serif text-[28px] font-semibold text-ink">
                {selectedNode.title}
              </h2>
              <div className="mt-1.5 font-mono text-[12.5px] text-ink-muted">
                {t("graph.connections", { count: neighbours.length })}
              </div>
              {isOrphan(idx, selectedNode.path) && (
                <div className="mt-4 flex gap-2.5 rounded-[10px] bg-brand-wash px-3.5 py-2.75">
                  <Icon name="flag" size={16} className="mt-0.5 flex-none text-brand" />
                  <span className="text-[12.5px] leading-relaxed text-ink-soft">
                    {t("graph.weakHint")}
                  </span>
                </div>
              )}
              <div className="mt-5 grid max-h-70 gap-1.5 overflow-auto">
                {neighbours.map((node) => (
                  <button
                    key={node.path}
                    type="button"
                    onClick={() => dispatch({ type: "select", id: node.path })}
                    className="flex items-center gap-2.5 rounded-lg border border-line bg-surface-2 px-3 py-2 text-left transition-colors hover:border-line-strong"
                  >
                    <span
                      className="size-2 rounded-full"
                      style={{ background: colorOf(idx, node.path) }}
                    />
                    <span className="text-[13.5px] font-semibold text-ink">{node.title}</span>
                    <span className="ml-auto font-mono text-[11px] text-ink-faint">
                      {catOf(idx, node.path)}
                    </span>
                  </button>
                ))}
              </div>
            </Panel>
          )}
        </div>
      )}
    </div>
  );
}
