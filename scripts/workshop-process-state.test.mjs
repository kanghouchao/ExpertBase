import test from "node:test";
import assert from "node:assert/strict";

import {
  buildManualDraft,
  canRemoveSource,
  isGeneratingPhase,
  lineDiff,
  phaseLabelKey,
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

test("isGeneratingPhase is true only while a turn is in flight", () => {
  assert.equal(isGeneratingPhase("idle"), false);
  assert.equal(isGeneratingPhase("connecting"), true);
  assert.equal(isGeneratingPhase("retrieving"), true);
  assert.equal(isGeneratingPhase("loadingModel"), true);
  assert.equal(isGeneratingPhase("generating"), true);
  assert.equal(isGeneratingPhase("done"), false);
});

test("thinking phase is generating and has its own label", () => {
  assert.equal(isGeneratingPhase("thinking"), true);
  assert.equal(phaseLabelKey("thinking"), "workshop.phase.thinking");
});

test("phaseLabelKey maps each phase to its own i18n key", () => {
  assert.equal(phaseLabelKey("retrieving"), "workshop.phase.retrieving");
  assert.equal(phaseLabelKey("loadingModel"), "workshop.phase.loadingModel");
  assert.equal(phaseLabelKey("connecting"), "workshop.phase.connecting");
  assert.equal(phaseLabelKey("generating"), "workshop.phase.generating");
  assert.equal(phaseLabelKey("idle"), "workshop.st.idle");
  assert.equal(phaseLabelKey("done"), "workshop.st.done");
});

test("lineDiff counts added and removed non-empty lines, source vs draft", () => {
  // 同一行は変更なし、draft だけの行は +、source だけの行は −。
  const source = "A\nB\nC";
  const draft = "A\nB\nD\nE";
  assert.deepEqual(lineDiff(source, draft), { added: 2, removed: 1 });
  // 空白行・前後空白は無視する。
  assert.deepEqual(lineDiff("  X  \n\n", "X\nY"), { added: 1, removed: 0 });
  // 同一なら 0/0。
  assert.deepEqual(lineDiff("A\nB", "A\nB"), { added: 0, removed: 0 });
});
