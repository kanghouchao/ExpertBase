import { describe, expect, test } from "bun:test";

import {
  activeKbChanged,
  collapseHistory,
  createConversationRunGuard,
  parseConversationId,
} from "./history";

describe("parseConversationId", () => {
  test("accepts positive integer ids only", () => {
    expect(parseConversationId("42")).toBe(42);
    expect(parseConversationId(null)).toBeNull();
    expect(parseConversationId("0")).toBeNull();
    expect(parseConversationId("1.5")).toBeNull();
    expect(parseConversationId("abc")).toBeNull();
  });
});

test("collapseHistory keeps exactly the first backend page", () => {
  const items = Array.from({ length: 27 }, (_, index) => ({
    id: index + 1,
    title: `conversation ${index + 1}`,
    updatedAt: "2026-06-30T00:00:00.000Z",
  }));
  expect(collapseHistory(items).map((item) => item.id)).toEqual(
    Array.from({ length: 20 }, (_, index) => index + 1)
  );
});

test("activeKbChanged ignores initial load and detects later KB changes", () => {
  expect(activeKbChanged(null, "/kb/a")).toBeFalse();
  expect(activeKbChanged("/kb/a", "/kb/b")).toBeTrue();
  expect(activeKbChanged("/kb/a", null)).toBeTrue();
});

test("invalidated conversation runs never become current again", () => {
  const guard = createConversationRunGuard();
  const first = guard.start();
  expect(guard.isCurrent(first)).toBeTrue();

  guard.invalidate();
  expect(guard.isCurrent(first)).toBeFalse();

  const second = guard.start();
  expect(guard.isCurrent(first)).toBeFalse();
  expect(guard.isCurrent(second)).toBeTrue();

  guard.invalidate();
  expect(guard.isCurrent(second)).toBeFalse();
});
