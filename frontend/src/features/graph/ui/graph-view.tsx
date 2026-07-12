"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";

import { Icon } from "@/shared/ui/icon";
import { PageHead } from "@/shared/ui/page-head";
import { Panel } from "@/shared/ui/panel";
import { Tag } from "@/shared/ui/tag";
import { EmptyState } from "@/shared/ui/empty-state";
import { Button, buttonVariants } from "@/shared/ui/button";
import { useI18n } from "@/shared/providers/providers";
import { kbApi, type EntryRef } from "@/shared/api";
import { useKbStore } from "@/entities/knowledge-base";
import { cn } from "@/shared/lib/utils";
import { forceLayout, type Point } from "../model/force-layout";
import { isOrphanDegree } from "../model/graph-metrics";

// カテゴリへ順番に割り当てる配色（カテゴリ自体はユーザーデータ由来）。
const CAT_PALETTE = ["var(--ai)", "var(--brand)", "#9b5a6b", "var(--gold)", "#5e7e8b", "#7a5ae0"];

// レイアウト空間（viewBox）。SVG は幅 100% で描き、この座標系を pan/zoom で変換する。
const W = 1100;
const H = 680;

type PanMode = { type: "pan"; sx: number; sy: number; px: number; py: number };
type NodeMode = {
  type: "node";
  id: string;
  ox: number;
  oy: number;
  moved: boolean;
  sx: number;
  sy: number;
};

export function GraphView() {
  const { t } = useI18n();
  const { available } = useKbStore();

  const [nodes, setNodes] = useState<EntryRef[]>([]);
  const [edges, setEdges] = useState<[string, string][]>([]);
  const [pos, setPos] = useState<Record<string, Point>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const [hover, setHover] = useState<string | null>(null);
  const [catFilter, setCatFilter] = useState<string | null>(null);
  const [showWeak, setShowWeak] = useState(false);
  const [view, setView] = useState({ zoom: 1, panX: 0, panY: 0 });
  const [dragId, setDragId] = useState<string | null>(null);
  const [relaying, setRelaying] = useState(false);

  const [grabbing, setGrabbing] = useState(false);

  const svgRef = useRef<SVGSVGElement>(null);
  const viewRef = useRef(view);
  const posRef = useRef(pos);
  const seedRef = useRef(7);
  const mode = useRef<PanMode | NodeMode | null>(null);

  // ハンドラから最新値を読むためのミラー（描画中は ref を触らない＝ref ルール順守）。
  useEffect(() => {
    viewRef.current = view;
  }, [view]);
  useEffect(() => {
    posRef.current = pos;
  }, [pos]);

  useEffect(() => {
    if (!available) return;
    void (async () => {
      const g = await kbApi.graph();
      setNodes(g.nodes);
      setEdges(g.edges);
      setPos(
        forceLayout(
          g.nodes.map((n) => n.path),
          g.edges,
          W,
          H
        )
      );
    })();
  }, [available]);

  // 派生インデックス（度数・隣接・id 逆引き・カテゴリ配色）。
  const deg = useMemo(() => {
    const d: Record<string, number> = {};
    nodes.forEach((n) => (d[n.path] = 0));
    edges.forEach(([a, b]) => {
      d[a] = (d[a] ?? 0) + 1;
      d[b] = (d[b] ?? 0) + 1;
    });
    return d;
  }, [nodes, edges]);

  const nbrOf = useMemo(() => {
    const m: Record<string, Set<string>> = {};
    nodes.forEach((n) => (m[n.path] = new Set()));
    edges.forEach(([a, b]) => {
      m[a]?.add(b);
      m[b]?.add(a);
    });
    return m;
  }, [nodes, edges]);

  const nodeById = useMemo(() => {
    const m: Record<string, EntryRef> = {};
    nodes.forEach((n) => (m[n.path] = n));
    return m;
  }, [nodes]);

  const catColors = useMemo(() => {
    const cats = [...new Set(nodes.map((n) => n.cat || "uncategorized"))];
    return Object.fromEntries(cats.map((cat, i) => [cat, CAT_PALETTE[i % CAT_PALETTE.length]]));
  }, [nodes]);

  const active = selected ?? hover;
  const activeNbr = active ? nbrOf[active] : null;
  const catOf = (id: string) => nodeById[id]?.cat || "uncategorized";
  const colorOf = (id: string) => catColors[catOf(id)];

  // 焦点（選択/ホバー）・カテゴリ絞り込み・孤立強調に応じて淡くするか。
  const dimNode = (id: string) => {
    if (showWeak) return !isOrphanDegree(deg[id] ?? 0);
    if (catFilter) return catOf(id) !== catFilter;
    if (active) return id !== active && !activeNbr?.has(id);
    return false;
  };
  const rad = (id: string) => 7 + Math.min(deg[id] ?? 0, 11) * 2;

  // 画面座標 → レイアウト座標（pan/zoom を打ち消す）。
  const toLayout = (clientX: number, clientY: number): Point => {
    const svg = svgRef.current;
    if (!svg) return { x: 0, y: 0 };
    const r = svg.getBoundingClientRect();
    const vx = (clientX - r.left) * (W / r.width);
    const vy = (clientY - r.top) * (H / r.height);
    const { zoom, panX, panY } = viewRef.current;
    return { x: (vx - panX) / zoom, y: (vy - panY) / zoom };
  };

  // ビュー中心を保ったままズーム。
  const applyZoom = (next: (z: number) => number) =>
    setView((v) => {
      const z = Math.min(2.6, Math.max(0.5, next(v.zoom)));
      return { zoom: z, panX: v.panX + (W / 2) * (v.zoom - z), panY: v.panY + (H / 2) * (v.zoom - z) };
    });
  const resetView = () => setView({ zoom: 1, panX: 0, panY: 0 });

  // wheel は passive:false でネイティブに張る＝ページスクロールを止めてズームできる。
  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const d = -e.deltaY * 0.0014;
      setView((v) => {
        const z = Math.min(2.6, Math.max(0.5, v.zoom + v.zoom * d));
        return {
          zoom: z,
          panX: v.panX + (W / 2) * (v.zoom - z),
          panY: v.panY + (H / 2) * (v.zoom - z),
        };
      });
    };
    svg.addEventListener("wheel", onWheel, { passive: false });
    return () => svg.removeEventListener("wheel", onWheel);
  }, []);

  const bgDown = (e: React.MouseEvent) => {
    setSelected(null);
    setGrabbing(true);
    mode.current = { type: "pan", sx: e.clientX, sy: e.clientY, px: view.panX, py: view.panY };
  };
  const nodeDown = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    const l = toLayout(e.clientX, e.clientY);
    const p = posRef.current[id];
    mode.current = {
      type: "node",
      id,
      ox: p.x - l.x,
      oy: p.y - l.y,
      moved: false,
      sx: e.clientX,
      sy: e.clientY,
    };
    setGrabbing(true);
    setDragId(id);
  };
  const onMove = (e: React.MouseEvent) => {
    const m = mode.current;
    if (!m) return;
    if (m.type === "pan") {
      setView((v) => ({ ...v, panX: m.px + (e.clientX - m.sx), panY: m.py + (e.clientY - m.sy) }));
    } else {
      if (Math.hypot(e.clientX - m.sx, e.clientY - m.sy) > 3) m.moved = true;
      const l = toLayout(e.clientX, e.clientY);
      setPos((prev) => ({ ...prev, [m.id]: { x: l.x + m.ox, y: l.y + m.oy } }));
    }
  };
  const onUp = () => {
    const m = mode.current;
    mode.current = null;
    setDragId(null);
    setGrabbing(false);
    // ドラッグせずに離した＝クリック扱いで選択トグル。
    if (m && m.type === "node" && !m.moved) setSelected((s) => (s === m.id ? null : m.id));
  };

  const relayout = () => {
    seedRef.current += 3;
    setRelaying(true);
    setPos(
      forceLayout(
        nodes.map((n) => n.path),
        edges,
        W,
        H,
        seedRef.current
      )
    );
    setSelected(null);
    resetView();
    window.setTimeout(() => setRelaying(false), 650);
  };

  const selectedNode = selected ? nodeById[selected] : null;
  const neighbours: EntryRef[] = selected
    ? [...(nbrOf[selected] ?? [])]
        .map((id) => nodeById[id])
        .filter((n): n is EntryRef => Boolean(n))
    : [];

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("graph.eyebrow")}
        title={t("graph.title")}
        sub={t("graph.sub", { nodes: nodes.length, edges: edges.length })}
        right={
          <>
            <Button
              variant="outline"
              className="border-line-strong bg-surface"
              onClick={relayout}
            >
              <Icon name="layers" size={17} />
              {t("graph.relayout")}
            </Button>
            <Button
              variant={showWeak ? "default" : "outline"}
              className={showWeak ? undefined : "border-line-strong bg-surface"}
              onClick={() => {
                setShowWeak((v) => !v);
                setSelected(null);
                setCatFilter(null);
              }}
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
      {nodes.length === 0 ? (
        <Panel pad={0}>
          <EmptyState icon="graph" title={t("empty.graph")} sub={t("empty.graph.sub")} />
        </Panel>
      ) : (
        <div className={cn("grid gap-4.5", selected ? "grid-cols-[1fr_322px]" : "grid-cols-1")}>
          <Panel pad={0} className="relative overflow-hidden">
            {/* カテゴリ凡例 / 絞り込み */}
            <div className="absolute top-3.5 left-3.5 z-10 flex max-w-[74%] flex-wrap gap-1.75">
              {Object.entries(catColors).map(([cat, color]) => {
                const on = catFilter === cat;
                return (
                  <button
                    key={cat}
                    type="button"
                    onClick={() => {
                      setCatFilter(on ? null : cat);
                      setShowWeak(false);
                    }}
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
                  ["+", () => applyZoom((z) => z + 0.25)],
                  ["−", () => applyZoom((z) => z - 0.25)],
                  ["⟳", resetView],
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
              {nodes.length} {t("graph.nodes")} · {edges.length} {t("graph.links")} ·{" "}
              {Math.round(view.zoom * 100)}%
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
              onMouseDown={bgDown}
              onMouseMove={onMove}
              onMouseUp={onUp}
              onMouseLeave={onUp}
            >
              <defs>
                <radialGradient id="graph-glow">
                  <stop offset="0%" stopColor="var(--brand)" stopOpacity="0.22" />
                  <stop offset="100%" stopColor="var(--brand)" stopOpacity="0" />
                </radialGradient>
              </defs>
              <g transform={`translate(${view.panX},${view.panY}) scale(${view.zoom})`}>
                {/* エッジ */}
                {edges.map(([a, b], i) => {
                  const pa = pos[a];
                  const pb = pos[b];
                  if (!pa || !pb) return null;
                  const on = active != null && (a === active || b === active);
                  const dim =
                    (active != null && !on) ||
                    (catFilter != null && catOf(a) !== catFilter && catOf(b) !== catFilter) ||
                    (showWeak &&
                      !isOrphanDegree(deg[a] ?? 0) &&
                      !isOrphanDegree(deg[b] ?? 0));
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
                        transition: relaying
                          ? "all .6s cubic-bezier(.2,.7,.2,1)"
                          : "opacity .2s, stroke .2s",
                      }}
                    />
                  );
                })}
                {/* ノード（円） */}
                {nodes.map((n) => {
                  const p = pos[n.path];
                  if (!p) return null;
                  const r = rad(n.path);
                  const dim = dimNode(n.path);
                  const on = n.path === active;
                  const orphan = isOrphanDegree(deg[n.path] ?? 0);
                  const isDrag = n.path === dragId;
                  return (
                    <g
                      key={n.path}
                      style={{
                        cursor: "grab",
                        opacity: dim ? 0.18 : 1,
                        transition: relaying ? "transform .6s cubic-bezier(.2,.7,.2,1)" : "opacity .2s",
                      }}
                      onMouseEnter={() => !mode.current && setHover(n.path)}
                      onMouseLeave={() => setHover(null)}
                      onMouseDown={(e) => nodeDown(e, n.path)}
                    >
                      {on && <circle cx={p.x} cy={p.y} r={r + 30} fill="url(#graph-glow)" />}
                      {showWeak && orphan && (
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
                        fill={colorOf(n.path)}
                        stroke={isDrag ? "var(--brand)" : "var(--surface)"}
                        strokeWidth={isDrag ? 3.5 : 2.5}
                        style={{
                          filter: on || isDrag ? "brightness(1.1)" : "none",
                          transition: relaying
                            ? "cx .6s cubic-bezier(.2,.7,.2,1), cy .6s cubic-bezier(.2,.7,.2,1)"
                            : "stroke .15s",
                        }}
                      />
                    </g>
                  );
                })}
                {/* ラベル（最終パス＝どの円にも隠れない。紙色の縁取りで可読性を確保） */}
                {nodes.map((n) => {
                  const p = pos[n.path];
                  if (!p) return null;
                  const r = rad(n.path);
                  const on = n.path === active;
                  const orphan = isOrphanDegree(deg[n.path] ?? 0);
                  const isDrag = n.path === dragId;
                  const showLabel =
                    on ||
                    isDrag ||
                    Boolean(activeNbr?.has(n.path)) ||
                    (deg[n.path] ?? 0) >= 4 ||
                    (showWeak && orphan);
                  if (!showLabel) return null;
                  return (
                    <text
                      key={n.path}
                      x={p.x}
                      y={p.y + r + 14}
                      textAnchor="middle"
                      fontSize={Math.max(11, 10 + (deg[n.path] ?? 0) * 0.3)}
                      fontWeight={on ? 700 : 600}
                      className="font-serif"
                      fill="var(--ink)"
                      stroke="var(--surface)"
                      strokeWidth={4}
                      paintOrder="stroke"
                      strokeLinejoin="round"
                      style={{
                        pointerEvents: "none",
                        opacity: dimNode(n.path) ? 0.18 : 1,
                        transition: relaying ? "all .6s cubic-bezier(.2,.7,.2,1)" : "opacity .2s",
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
                    style={{ background: colorOf(selectedNode.path) }}
                  />
                  {selectedNode.cat || "uncategorized"}
                </Tag>
                <button
                  type="button"
                  onClick={() => setSelected(null)}
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
              {isOrphanDegree(deg[selectedNode.path] ?? 0) && (
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
                    onClick={() => setSelected(node.path)}
                    className="flex items-center gap-2.5 rounded-lg border border-line bg-surface-2 px-3 py-2 text-left transition-colors hover:border-line-strong"
                  >
                    <span
                      className="size-2 rounded-full"
                      style={{ background: colorOf(node.path) }}
                    />
                    <span className="text-[13.5px] font-semibold text-ink">{node.title}</span>
                    <span className="ml-auto font-mono text-[11px] text-ink-faint">
                      {node.cat || "uncategorized"}
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
