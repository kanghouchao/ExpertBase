import test from "node:test";
import assert from "node:assert/strict";

import {
  buildManualDraft,
  canRemoveSource,
  replaceLatestEntryResult,
  sameSourceIds,
  toChatTurn,
} from "../frontend/src/features/workshop/model/process-state.ts";

const entry = {
  kind: "entry",
  title: "Tea",
  cat: "tea",
  bodyMarkdown: "Body",
  suggestedLinks: ["Green tea"],
};

test("assistant turn preserves the complete structured result", () => {
  assert.deepEqual(JSON.parse(toChatTurn({ role: "ai", result: entry }).content), entry);
});

test("editing replaces only the latest entry result", () => {
  const chat = { kind: "chat", title: "", cat: "", bodyMarkdown: "Hi", suggestedLinks: [] };
  const edited = { ...entry, title: "Edited" };
  const messages = [
    { role: "ai", result: entry },
    { role: "ai", result: chat },
  ];

  assert.deepEqual(replaceLatestEntryResult(messages, edited), [
    { role: "ai", result: edited },
    { role: "ai", result: chat },
  ]);
});

test("manual draft includes every selected source in order", () => {
  const sources = [
    { id: "inbox/a.md", title: "A", preview: "fallback A" },
    { id: "inbox/b.md", title: "B", preview: "fallback B" },
  ];
  const raw = {
    "inbox/a.md": "---\ntype: text\n---\n\nAlpha",
    "inbox/b.md": "---\ntype: text\n---\n\nBeta",
  };

  assert.equal(buildManualDraft(sources, raw).bodyMarkdown, "Alpha\n\n---\n\nBeta");
});

test("draft source snapshot must exactly match current sources", () => {
  assert.equal(sameSourceIds(["a", "b"], ["a", "b"]), true);
  assert.equal(sameSourceIds(["a"], ["a", "b"]), false);
});

test("source removal is allowed only before the first turn", () => {
  assert.equal(canRemoveSource(0, 2), true);
  assert.equal(canRemoveSource(1, 2), false);
  assert.equal(canRemoveSource(0, 1), false);
});
