import { expect, test } from "bun:test";

test("孤立ノードは接続数 0 だけとする", async () => {
  const metrics = await import("./graph-metrics").catch(() => null);

  expect(metrics?.isOrphanDegree(0)).toBe(true);
  expect(metrics?.isOrphanDegree(1)).toBe(false);
});
