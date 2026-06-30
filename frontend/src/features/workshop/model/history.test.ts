import { describe, expect, test } from "bun:test";

import { collapseHistory, parseConversationId } from "./history";

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
