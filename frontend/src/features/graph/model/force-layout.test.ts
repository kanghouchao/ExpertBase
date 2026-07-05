import { describe, expect, test } from "bun:test";

import { forceLayout } from "./force-layout";

const W = 1100;
const H = 680;

describe("forceLayout", () => {
  test("ノードが無ければ空を返す", () => {
    expect(forceLayout([], [], W, H)).toEqual({});
  });

  test("全ノードに座標を割り当て、viewport 内に収める", () => {
    const ids = ["a", "b", "c", "d"];
    const edges: [string, string][] = [
      ["a", "b"],
      ["b", "c"],
      ["c", "d"],
    ];
    const pos = forceLayout(ids, edges, W, H);
    expect(Object.keys(pos).sort()).toEqual(ids);
    for (const id of ids) {
      expect(pos[id].x).toBeGreaterThanOrEqual(0);
      expect(pos[id].x).toBeLessThanOrEqual(W);
      expect(pos[id].y).toBeGreaterThanOrEqual(0);
      expect(pos[id].y).toBeLessThanOrEqual(H);
    }
  });

  test("同じ seed なら決定論的に同じ結果を返す", () => {
    const ids = ["a", "b", "c"];
    const edges: [string, string][] = [["a", "b"]];
    expect(forceLayout(ids, edges, W, H, 7)).toEqual(forceLayout(ids, edges, W, H, 7));
  });

  test("未知の端点を含むエッジは無視して落ちない", () => {
    const pos = forceLayout(["a", "b"], [["a", "zzz"]], W, H);
    expect(Object.keys(pos).sort()).toEqual(["a", "b"]);
  });

  test("大規模グラフは楕円上へ線形時間で配置する", () => {
    const ids = Array.from({ length: 201 }, (_, i) => String(i));
    const pos = forceLayout(ids, [], W, H);
    const rx = (W - 140) / 2;
    const ry = (H - 140) / 2;

    for (const id of ids) {
      const x = (pos[id].x - W / 2) / rx;
      const y = (pos[id].y - H / 2) / ry;
      expect(x * x + y * y).toBeCloseTo(1, 8);
    }
  });
});
