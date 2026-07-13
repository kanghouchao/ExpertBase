import { describe, expect, test } from "bun:test";

import { detectSlashTrigger } from "./use-skill-slash";

describe("detectSlashTrigger", () => {
  test("行頭の / を検出し、直後のトークンをクエリとして返す", () => {
    expect(detectSlashTrigger("/tea", 4)).toEqual({ lineStart: 0, query: "tea" });
    expect(detectSlashTrigger("/", 1)).toEqual({ lineStart: 0, query: "" });
  });

  test("非行頭の / には反応しない", () => {
    expect(detectSlashTrigger("見て/tea", 6)).toBeNull();
    expect(detectSlashTrigger("a/b", 3)).toBeNull();
  });

  test("/ の後に空白を挟んで続けて打つと反応しない（コマンドとして打ち終えたとみなす）", () => {
    expect(detectSlashTrigger("/tea more", 9)).toBeNull();
    expect(detectSlashTrigger("/tea ", 5)).toBeNull();
  });

  test("複数行では現在行の行頭で判定する", () => {
    const value = "前の行\n/tea";
    expect(detectSlashTrigger(value, value.length)).toEqual({ lineStart: 4, query: "tea" });
  });

  test("現在行に / が無ければ反応しない（前の行に / があっても無関係）", () => {
    const value = "/skip\n本文を書く";
    expect(detectSlashTrigger(value, value.length)).toBeNull();
  });

  test("カーソルが / の直後（トークン長 0）でも反応する", () => {
    const value = "/tea";
    expect(detectSlashTrigger(value, 1)).toEqual({ lineStart: 0, query: "" });
  });
});
