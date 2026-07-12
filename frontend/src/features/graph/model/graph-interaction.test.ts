import { describe, expect, test } from "bun:test";

import type { EntryRef } from "@/shared/api";

import { forceLayout } from "./force-layout";
import {
  H,
  W,
  activeNode,
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
  type GraphInteractionState,
} from "./graph-interaction";

const node = (path: string, cat = ""): EntryRef => ({ path, title: path, cat });

// rect を viewBox と同寸にすると client 座標 = viewBox 座標（1:1）。
const rect = { left: 0, top: 0, width: W, height: H };

const NODES = [node("a", "x"), node("b", "x"), node("c", "y"), node("lone")];
const EDGES: [string, string][] = [
  ["a", "b"],
  ["b", "c"],
];

const loaded = (): GraphInteractionState =>
  reduce(initialState, { type: "graphLoaded", nodes: NODES, edges: EDGES });

describe("reduce: 読込と再配置", () => {
  test("graphLoaded はノード・エッジを取り込み、既定 seed で forceLayout する", () => {
    const s = loaded();
    expect(s.nodes).toEqual(NODES);
    expect(s.edges).toEqual(EDGES);
    expect(s.pos).toEqual(
      forceLayout(
        NODES.map((n) => n.path),
        EDGES,
        W,
        H,
        7
      )
    );
  });

  test("relayout は seed を進めて再配置し、選択とビューを畳んでアニメーション標識を立てる", () => {
    let s = loaded();
    s = reduce(s, { type: "select", id: "a" });
    s = reduce(s, { type: "zoomStep", delta: 0.25 });
    s = reduce(s, { type: "relayout" });
    expect(s.seed).toBe(10);
    expect(s.pos).toEqual(
      forceLayout(
        NODES.map((n) => n.path),
        EDGES,
        W,
        H,
        10
      )
    );
    expect(s.selected).toBeNull();
    expect(s.view).toEqual({ zoom: 1, panX: 0, panY: 0 });
    expect(s.relaying).toBe(true);
    expect(reduce(s, { type: "relayoutSettled" }).relaying).toBe(false);
  });
});

describe("reduce: ドラッグ vs クリック状態機", () => {
  test("3px 以下で離す＝クリック扱いで選択トグル", () => {
    let s = loaded();
    const p = s.pos.a;
    s = reduce(s, { type: "nodeDown", id: "a", x: p.x, y: p.y, rect });
    expect(dragTarget(s)).toBe("a");
    s = reduce(s, { type: "pointerMove", x: p.x + 2, y: p.y, rect });
    s = reduce(s, { type: "pointerUp" });
    expect(s.selected).toBe("a");
    expect(s.mode).toBeNull();
    // 同じノードをもう一度クリック＝解除。
    s = reduce(s, { type: "nodeDown", id: "a", x: p.x, y: p.y, rect });
    s = reduce(s, { type: "pointerUp" });
    expect(s.selected).toBeNull();
  });

  test("3px 超のドラッグは選択せず座標だけ更新する", () => {
    let s = loaded();
    const p = s.pos.a;
    s = reduce(s, { type: "nodeDown", id: "a", x: p.x, y: p.y, rect });
    s = reduce(s, { type: "pointerMove", x: p.x + 10, y: p.y - 4, rect });
    expect(s.pos.a).toEqual({ x: p.x + 10, y: p.y - 4 });
    s = reduce(s, { type: "pointerUp" });
    expect(s.selected).toBeNull();
  });

  test("一度 3px を超えたら戻してもクリックにはならない", () => {
    let s = loaded();
    const p = s.pos.a;
    s = reduce(s, { type: "nodeDown", id: "a", x: p.x, y: p.y, rect });
    s = reduce(s, { type: "pointerMove", x: p.x + 10, y: p.y, rect });
    s = reduce(s, { type: "pointerMove", x: p.x, y: p.y, rect });
    s = reduce(s, { type: "pointerUp" });
    expect(s.selected).toBeNull();
  });

  test("pan/zoom 中のドラッグは画面座標をレイアウト座標へ打ち消して適用する", () => {
    let s = loaded();
    const p = s.pos.a;
    s = { ...s, view: { zoom: 2, panX: 100, panY: 50 } };
    // レイアウト点 p が映る画面位置からつまみ、画面で +10px 動かす → レイアウトでは +10/zoom。
    const cx = p.x * 2 + 100;
    const cy = p.y * 2 + 50;
    s = reduce(s, { type: "nodeDown", id: "a", x: cx, y: cy, rect });
    s = reduce(s, { type: "pointerMove", x: cx + 10, y: cy, rect });
    expect(s.pos.a.x).toBeCloseTo(p.x + 5, 8);
    expect(s.pos.a.y).toBeCloseTo(p.y, 8);
  });

  test("背景ドラッグはパンし、押下時点で選択を畳む", () => {
    let s = loaded();
    s = reduce(s, { type: "select", id: "a" });
    s = reduce(s, { type: "bgDown", x: 100, y: 100 });
    expect(s.selected).toBeNull();
    s = reduce(s, { type: "pointerMove", x: 130, y: 80, rect });
    expect(s.view.panX).toBe(30);
    expect(s.view.panY).toBe(-20);
    s = reduce(s, { type: "pointerUp" });
    expect(s.mode).toBeNull();
  });

  test("ドラッグ中の hoverEnter は無視し、終わったら効く", () => {
    let s = loaded();
    const p = s.pos.a;
    s = reduce(s, { type: "nodeDown", id: "a", x: p.x, y: p.y, rect });
    s = reduce(s, { type: "hoverEnter", id: "b" });
    expect(s.hover).toBeNull();
    s = reduce(s, { type: "pointerUp" });
    s = reduce(s, { type: "hoverEnter", id: "b" });
    expect(s.hover).toBe("b");
    s = reduce(s, { type: "hoverLeave" });
    expect(s.hover).toBeNull();
  });
});

describe("reduce: ズーム境界と中心保持", () => {
  test("zoomStep は 0.5–2.6 に収める", () => {
    let s = loaded();
    for (let i = 0; i < 20; i++) s = reduce(s, { type: "zoomStep", delta: 0.25 });
    expect(s.view.zoom).toBe(2.6);
    for (let i = 0; i < 20; i++) s = reduce(s, { type: "zoomStep", delta: -0.25 });
    expect(s.view.zoom).toBe(0.5);
  });

  test("wheelZoom も同じ境界に収める", () => {
    let s = loaded();
    s = reduce(s, { type: "wheelZoom", deltaY: -100000 });
    expect(s.view.zoom).toBe(2.6);
    s = reduce(s, { type: "wheelZoom", deltaY: 100000 });
    expect(s.view.zoom).toBe(0.5);
  });

  test("ズームはレイアウト中心 (W/2, H/2) の画面位置を固定したまま行う", () => {
    let s = loaded();
    s = { ...s, view: { zoom: 1.3, panX: 40, panY: -25 } };
    // レイアウト点 L の画面位置 = L * zoom + pan。L = レイアウト中心が不動点。
    const screenOfCentre = (v: { zoom: number; panX: number; panY: number }) => ({
      x: (W / 2) * v.zoom + v.panX,
      y: (H / 2) * v.zoom + v.panY,
    });
    const before = screenOfCentre(s.view);
    s = reduce(s, { type: "zoomStep", delta: 0.25 });
    const after = screenOfCentre(s.view);
    expect(after.x).toBeCloseTo(before.x, 8);
    expect(after.y).toBeCloseTo(before.y, 8);
  });

  test("resetView は等倍・原点へ戻す", () => {
    let s = loaded();
    s = reduce(s, { type: "zoomStep", delta: 0.5 });
    s = reduce(s, { type: "resetView" });
    expect(s.view).toEqual({ zoom: 1, panX: 0, panY: 0 });
  });
});

describe("reduce: 焦点フィルタの排他", () => {
  test("toggleWeak は選択とカテゴリ絞込を畳む", () => {
    let s = loaded();
    s = reduce(s, { type: "select", id: "a" });
    s = reduce(s, { type: "toggleCatFilter", cat: "x" });
    s = reduce(s, { type: "toggleWeak" });
    expect(s.showWeak).toBe(true);
    expect(s.selected).toBeNull();
    expect(s.catFilter).toBeNull();
    expect(reduce(s, { type: "toggleWeak" }).showWeak).toBe(false);
  });

  test("toggleCatFilter は同カテゴリで解除し、showWeak を畳む", () => {
    let s = loaded();
    s = reduce(s, { type: "toggleWeak" });
    s = reduce(s, { type: "toggleCatFilter", cat: "x" });
    expect(s.catFilter).toBe("x");
    expect(s.showWeak).toBe(false);
    s = reduce(s, { type: "toggleCatFilter", cat: "y" });
    expect(s.catFilter).toBe("y");
    expect(reduce(s, { type: "toggleCatFilter", cat: "y" }).catFilter).toBeNull();
  });

  test("clearSelection は選択だけ畳む", () => {
    let s = loaded();
    s = reduce(s, { type: "select", id: "a" });
    s = reduce(s, { type: "clearSelection" });
    expect(s.selected).toBeNull();
  });
});

describe("deriveIndices", () => {
  test("度数・隣接・逆引き・カテゴリ配色を導出する", () => {
    const idx = deriveIndices(NODES, EDGES);
    expect(idx.deg).toEqual({ a: 1, b: 2, c: 1, lone: 0 });
    expect(idx.nbrOf.b).toEqual(new Set(["a", "c"]));
    expect(idx.nbrOf.a).toEqual(new Set(["b"]));
    expect(idx.nodeById.c).toEqual(NODES[2]);
    // cat 空文字は uncategorized として配色される。
    expect(Object.keys(idx.catColors).sort()).toEqual(["uncategorized", "x", "y"].sort());
  });

  test("孤立判定は接続数 0 だけとする（graph-metrics 吸収）", () => {
    const idx = deriveIndices(NODES, EDGES);
    expect(isOrphan(idx, "lone")).toBe(true);
    expect(isOrphan(idx, "a")).toBe(false);
  });

  test("neighboursOf は隣接ノードの実体を返す", () => {
    const idx = deriveIndices(NODES, EDGES);
    expect(neighboursOf(idx, "b").map((n) => n.path).sort()).toEqual(["a", "c"]);
    expect(neighboursOf(idx, "lone")).toEqual([]);
  });
});

describe("焦点淡化の優先順位: showWeak > catFilter > active", () => {
  const idx = deriveIndices(NODES, EDGES);

  test("showWeak 時は非孤立だけ淡くする", () => {
    const s = { ...loaded(), showWeak: true, catFilter: "x", selected: "a" };
    expect(dimNode(s, idx, "lone")).toBe(false);
    expect(dimNode(s, idx, "a")).toBe(true);
  });

  test("catFilter 時は異カテゴリを淡くする", () => {
    const s = { ...loaded(), catFilter: "x" };
    expect(dimNode(s, idx, "a")).toBe(false);
    expect(dimNode(s, idx, "c")).toBe(true);
  });

  test("焦点（selected 優先、無ければ hover）時は非隣接を淡くする", () => {
    const hovered = { ...loaded(), hover: "a" };
    expect(activeNode(hovered)).toBe("a");
    expect(dimNode(hovered, idx, "a")).toBe(false);
    expect(dimNode(hovered, idx, "b")).toBe(false);
    expect(dimNode(hovered, idx, "c")).toBe(true);
    const both = { ...loaded(), selected: "c", hover: "a" };
    expect(activeNode(both)).toBe("c");
    expect(dimNode(both, idx, "a")).toBe(true);
  });

  test("無焦点なら誰も淡くしない", () => {
    const s = loaded();
    expect(NODES.every((n) => !dimNode(s, idx, n.path))).toBe(true);
  });
});

describe("エッジの強調と淡化", () => {
  const idx = deriveIndices(NODES, EDGES);

  test("焦点の端点を持つ辺は強調、他は淡化", () => {
    const s = { ...loaded(), selected: "a" };
    expect(edgeActive(s, "a", "b")).toBe(true);
    expect(dimEdge(s, idx, "a", "b")).toBe(false);
    expect(edgeActive(s, "b", "c")).toBe(false);
    expect(dimEdge(s, idx, "b", "c")).toBe(true);
  });

  test("catFilter 時は両端とも異カテゴリの辺だけ淡化", () => {
    const s = { ...loaded(), catFilter: "y" };
    expect(dimEdge(s, idx, "b", "c")).toBe(false);
    expect(dimEdge(s, idx, "a", "b")).toBe(true);
  });

  test("showWeak 時は両端とも非孤立の辺を淡化", () => {
    const s = { ...loaded(), showWeak: true };
    expect(dimEdge(s, idx, "a", "b")).toBe(true);
  });
});

describe("ラベル表示と寸法", () => {
  test("焦点・ドラッグ中・隣接・高次数・孤立強調中の孤立だけ表示する", () => {
    const nodes = [...NODES, node("hub"), node("d"), node("e"), node("f"), node("g")];
    const edges: [string, string][] = [
      ...EDGES,
      ["hub", "d"],
      ["hub", "e"],
      ["hub", "f"],
      ["hub", "g"],
    ];
    const idx = deriveIndices(nodes, edges);
    let s = reduce(initialState, { type: "graphLoaded", nodes, edges });
    expect(labelVisible(s, idx, "hub")).toBe(true); // deg 4
    expect(labelVisible(s, idx, "a")).toBe(false);
    s = { ...s, hover: "a" };
    expect(labelVisible(s, idx, "a")).toBe(true); // 焦点
    expect(labelVisible(s, idx, "b")).toBe(true); // 隣接
    expect(labelVisible(s, idx, "c")).toBe(false);
    const weak = { ...reduce(initialState, { type: "graphLoaded", nodes, edges } as const), showWeak: true };
    expect(labelVisible(weak, idx, "lone")).toBe(true); // 孤立
    const p = s.pos.c;
    const dragging = reduce(s, { type: "nodeDown", id: "c", x: p.x, y: p.y, rect });
    expect(labelVisible(dragging, idx, "c")).toBe(true); // ドラッグ中
  });

  test("半径は次数 11 で頭打ち、フォントサイズは下限 11", () => {
    expect(nodeRadius(0)).toBe(7);
    expect(nodeRadius(11)).toBe(29);
    expect(nodeRadius(20)).toBe(29);
    expect(labelFontSize(0)).toBe(11);
    expect(labelFontSize(10)).toBe(13);
  });
});
