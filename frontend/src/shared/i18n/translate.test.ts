import { describe, expect, test } from "bun:test";

import { createT, translateError } from "./translate";

describe("translateError", () => {
  const t = createT("zh");

  test("translates a coded AppError with params", () => {
    const result = translateError(t, { code: "err.generic", params: { detail: "boom" } });
    expect(result).toBe(t("err.generic", { detail: "boom" }));
  });

  test("falls back to String(e) for non-AppError values", () => {
    expect(translateError(t, "plain string")).toBe("plain string");
    expect(translateError(t, new Error("oops"))).toBe(String(new Error("oops")));
  });
});
