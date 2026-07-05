import { describe, expect, test } from "bun:test";

import { resolveActiveNav } from "./nav";

describe("resolveActiveNav", () => {
  test("マッチしないパスはダッシュボードにフォールバックする", () => {
    expect(resolveActiveNav("/", null)).toBe("dash");
    expect(resolveActiveNav("/unknown", null)).toBe("dash");
  });

  test("各セクションのパスをそのナビ項目に対応させる", () => {
    expect(resolveActiveNav("/workshop", null)).toBe("workshop");
    expect(resolveActiveNav("/wiki", null)).toBe("wiki");
    expect(resolveActiveNav("/graph", null)).toBe("graph");
  });

  test("サブパスは親セクションを選択したままにする", () => {
    expect(resolveActiveNav("/workshop/anything", null)).toBe("workshop");
  });

  test("特定の会話を開いている間は親「ワークショップ」をハイライトしない", () => {
    expect(resolveActiveNav("/workshop", 42)).toBeNull();
  });

  test("会話 ID はワークショップ以外のセクションに影響しない", () => {
    expect(resolveActiveNav("/wiki", 42)).toBe("wiki");
    expect(resolveActiveNav("/graph", 42)).toBe("graph");
  });
});
