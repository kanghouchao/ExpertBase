// グラフビューのインタラクション状態機械。force-layout と同じ継ぎ目の流儀で、
// ドラッグ vs クリックの判定・pan/zoom の数学・派生インデックス・焦点淡化の
// 意味論を純関数として持つ。座標系は viewBox（0..W, 0..H）。action は client
// 座標 + rect スナップショットを運び、pan/zoom の打ち消し（toLayout）は
// reducer 内部で行う＝ビューはイベント転送だけ。

import type { EntryRef } from "@/shared/api";

import { forceLayout, type Point } from "./force-layout";

// レイアウト空間（viewBox）。SVG は幅 100% で描き、この座標系を pan/zoom で変換する。
export const W = 1100;
export const H = 680;

// カテゴリへ順番に割り当てる配色（カテゴリ自体はユーザーデータ由来）。
const CAT_PALETTE = ["var(--ai)", "var(--brand)", "#9b5a6b", "var(--gold)", "#5e7e8b", "#7a5ae0"];

const ZOOM_MIN = 0.5;
const ZOOM_MAX = 2.6;
// この画面距離（px）を超えたらクリックではなくドラッグとみなす。
const DRAG_THRESHOLD = 3;

export type Rect = { left: number; top: number; width: number; height: number };

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

export type GraphInteractionState = {
  nodes: EntryRef[];
  edges: [string, string][];
  pos: Record<string, Point>;
  seed: number;
  view: { zoom: number; panX: number; panY: number };
  selected: string | null;
  hover: string | null;
  catFilter: string | null;
  showWeak: boolean;
  mode: PanMode | NodeMode | null;
  relaying: boolean;
};

export type GraphAction =
  | { type: "graphLoaded"; nodes: EntryRef[]; edges: [string, string][] }
  | { type: "bgDown"; x: number; y: number }
  | { type: "nodeDown"; id: string; x: number; y: number; rect: Rect }
  | { type: "pointerMove"; x: number; y: number; rect: Rect }
  | { type: "pointerUp" }
  | { type: "hoverEnter"; id: string }
  | { type: "hoverLeave" }
  | { type: "wheelZoom"; deltaY: number }
  | { type: "zoomStep"; delta: number }
  | { type: "resetView" }
  | { type: "relayout" }
  | { type: "relayoutSettled" }
  | { type: "select"; id: string }
  | { type: "clearSelection" }
  | { type: "toggleWeak" }
  | { type: "toggleCatFilter"; cat: string };

export const initialState: GraphInteractionState = {
  nodes: [],
  edges: [],
  pos: {},
  seed: 7,
  view: { zoom: 1, panX: 0, panY: 0 },
  selected: null,
  hover: null,
  catFilter: null,
  showWeak: false,
  mode: null,
  relaying: false,
};

// 画面座標 → レイアウト座標（pan/zoom を打ち消す）。
function toLayout(
  x: number,
  y: number,
  rect: Rect,
  view: GraphInteractionState["view"]
): Point {
  const vx = (x - rect.left) * (W / rect.width);
  const vy = (y - rect.top) * (H / rect.height);
  return { x: (vx - view.panX) / view.zoom, y: (vy - view.panY) / view.zoom };
}

// ノード集合とエッジから viewBox に収まる座標を計算する（forceLayout の癒着点）。
function layoutOf(
  nodes: EntryRef[],
  edges: [string, string][],
  seed: number
): Record<string, Point> {
  return forceLayout(
    nodes.map((n) => n.path),
    edges,
    W,
    H,
    seed
  );
}

// ビュー中心を保ったままズーム（境界 0.5–2.6）。
function zoomTo(view: GraphInteractionState["view"], zoom: number) {
  const z = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoom));
  return {
    zoom: z,
    panX: view.panX + (W / 2) * (view.zoom - z),
    panY: view.panY + (H / 2) * (view.zoom - z),
  };
}

export function reduce(s: GraphInteractionState, a: GraphAction): GraphInteractionState {
  switch (a.type) {
    case "graphLoaded":
      return { ...s, nodes: a.nodes, edges: a.edges, pos: layoutOf(a.nodes, a.edges, s.seed) };
    case "bgDown":
      return {
        ...s,
        selected: null,
        mode: { type: "pan", sx: a.x, sy: a.y, px: s.view.panX, py: s.view.panY },
      };
    case "nodeDown": {
      const l = toLayout(a.x, a.y, a.rect, s.view);
      const p = s.pos[a.id];
      return {
        ...s,
        mode: { type: "node", id: a.id, ox: p.x - l.x, oy: p.y - l.y, moved: false, sx: a.x, sy: a.y },
      };
    }
    case "pointerMove": {
      const m = s.mode;
      if (!m) return s;
      if (m.type === "pan") {
        return { ...s, view: { ...s.view, panX: m.px + (a.x - m.sx), panY: m.py + (a.y - m.sy) } };
      }
      const moved = m.moved || Math.hypot(a.x - m.sx, a.y - m.sy) > DRAG_THRESHOLD;
      const l = toLayout(a.x, a.y, a.rect, s.view);
      return {
        ...s,
        mode: { ...m, moved },
        pos: { ...s.pos, [m.id]: { x: l.x + m.ox, y: l.y + m.oy } },
      };
    }
    case "pointerUp": {
      const m = s.mode;
      // ドラッグせずに離した＝クリック扱いで選択トグル。
      const selected =
        m && m.type === "node" && !m.moved ? (s.selected === m.id ? null : m.id) : s.selected;
      return { ...s, mode: null, selected };
    }
    case "hoverEnter":
      return s.mode ? s : { ...s, hover: a.id };
    case "hoverLeave":
      return { ...s, hover: null };
    case "wheelZoom":
      return { ...s, view: zoomTo(s.view, s.view.zoom + s.view.zoom * -a.deltaY * 0.0014) };
    case "zoomStep":
      return { ...s, view: zoomTo(s.view, s.view.zoom + a.delta) };
    case "resetView":
      return { ...s, view: initialState.view };
    case "relayout": {
      const seed = s.seed + 3;
      return {
        ...s,
        seed,
        pos: layoutOf(s.nodes, s.edges, seed),
        selected: null,
        view: initialState.view,
        relaying: true,
      };
    }
    case "relayoutSettled":
      return { ...s, relaying: false };
    case "select":
      return { ...s, selected: a.id };
    case "clearSelection":
      return { ...s, selected: null };
    case "toggleWeak":
      return { ...s, showWeak: !s.showWeak, selected: null, catFilter: null };
    case "toggleCatFilter":
      return { ...s, catFilter: s.catFilter === a.cat ? null : a.cat, showWeak: false };
  }
}

// ── 派生インデックス（度数・隣接・id 逆引き・カテゴリ配色） ──

export type GraphIndices = {
  deg: Record<string, number>;
  nbrOf: Record<string, Set<string>>;
  nodeById: Record<string, EntryRef>;
  catColors: Record<string, string>;
};

export function deriveIndices(nodes: EntryRef[], edges: [string, string][]): GraphIndices {
  const deg: Record<string, number> = {};
  const nbrOf: Record<string, Set<string>> = {};
  const nodeById: Record<string, EntryRef> = {};
  nodes.forEach((n) => {
    deg[n.path] = 0;
    nbrOf[n.path] = new Set();
    nodeById[n.path] = n;
  });
  edges.forEach(([a, b]) => {
    deg[a] = (deg[a] ?? 0) + 1;
    deg[b] = (deg[b] ?? 0) + 1;
    nbrOf[a]?.add(b);
    nbrOf[b]?.add(a);
  });
  const cats = [...new Set(nodes.map((n) => n.cat || "uncategorized"))];
  const catColors = Object.fromEntries(
    cats.map((cat, i) => [cat, CAT_PALETTE[i % CAT_PALETTE.length]])
  );
  return { deg, nbrOf, nodeById, catColors };
}

/** ノードのカテゴリ名（空は uncategorized へ畳む）。 */
export function catOf(idx: GraphIndices, id: string): string {
  return idx.nodeById[id]?.cat || "uncategorized";
}

export function colorOf(idx: GraphIndices, id: string): string {
  return idx.catColors[catOf(idx, id)];
}

// 後端の orphans と同じく、入出力リンクが一つもないノードだけを孤立とする。
export function isOrphan(idx: GraphIndices, id: string): boolean {
  return (idx.deg[id] ?? 0) === 0;
}

export function neighboursOf(idx: GraphIndices, id: string): EntryRef[] {
  return [...(idx.nbrOf[id] ?? [])]
    .map((nbr) => idx.nodeById[nbr])
    .filter((n): n is EntryRef => Boolean(n));
}

// ── 表示の意味論（ビューは真偽値を属性へ結ぶだけ） ──

/** 焦点＝選択が優先、無ければホバー。 */
export function activeNode(s: GraphInteractionState): string | null {
  return s.selected ?? s.hover;
}

/** ドラッグ中のノード id（つまんだ瞬間から）。 */
export function dragTarget(s: GraphInteractionState): string | null {
  return s.mode?.type === "node" ? s.mode.id : null;
}

// 焦点（選択/ホバー）・カテゴリ絞り込み・孤立強調に応じて淡くするか。
export function dimNode(s: GraphInteractionState, idx: GraphIndices, id: string): boolean {
  if (s.showWeak) return !isOrphan(idx, id);
  if (s.catFilter) return catOf(idx, id) !== s.catFilter;
  const active = activeNode(s);
  if (active) return id !== active && !idx.nbrOf[active]?.has(id);
  return false;
}

/** 焦点の端点を持つ辺＝強調表示。 */
export function edgeActive(s: GraphInteractionState, a: string, b: string): boolean {
  const active = activeNode(s);
  return active != null && (a === active || b === active);
}

export function dimEdge(
  s: GraphInteractionState,
  idx: GraphIndices,
  a: string,
  b: string
): boolean {
  return (
    (activeNode(s) != null && !edgeActive(s, a, b)) ||
    (s.catFilter != null && catOf(idx, a) !== s.catFilter && catOf(idx, b) !== s.catFilter) ||
    (s.showWeak && !isOrphan(idx, a) && !isOrphan(idx, b))
  );
}

// ラベルは焦点・ドラッグ中・焦点の隣接・高次数（≥4）・孤立強調中の孤立だけ。
export function labelVisible(s: GraphInteractionState, idx: GraphIndices, id: string): boolean {
  const active = activeNode(s);
  return (
    id === active ||
    id === dragTarget(s) ||
    Boolean(active && idx.nbrOf[active]?.has(id)) ||
    (idx.deg[id] ?? 0) >= 4 ||
    (s.showWeak && isOrphan(idx, id))
  );
}

export function nodeRadius(degree: number): number {
  return 7 + Math.min(degree, 11) * 2;
}

export function labelFontSize(degree: number): number {
  return Math.max(11, 10 + degree * 0.3);
}
