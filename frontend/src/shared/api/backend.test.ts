import { afterEach, describe, expect, test } from "bun:test";

import {
  agentApi,
  DEFAULT_AI_SETTINGS,
  fakeBackend,
  kbApi,
  setBackend,
  workshopApi,
} from "./index";

// 各テスト後に必ず既定（fake）へ戻す＝テスト間の後端汚染を防ぐ。
afterEach(() => setBackend(fakeBackend));

describe("fakeBackend", () => {
  test("mirrors the browser fallbacks of the old isTauri guards", async () => {
    expect(await kbApi.listKbs()).toBeNull();
    expect(await kbApi.listEntries()).toEqual([]);
    expect(await kbApi.search("緑茶")).toEqual([]);
    expect(await kbApi.backlinks("緑茶")).toEqual([]);
    expect(await kbApi.stats()).toBeNull();
    expect(await kbApi.graph()).toEqual({ nodes: [], edges: [] });
    expect(await kbApi.orphans()).toEqual([]);
    expect(await agentApi.hasKey()).toBeFalse();
    expect(await agentApi.listOllamaModels()).toEqual([]);
    expect(await agentApi.listModels("ollama", "")).toEqual([]);
    expect(await agentApi.getSettings()).toEqual(DEFAULT_AI_SETTINGS);
    await agentApi.setSettings(DEFAULT_AI_SETTINGS); // no-op で解決する
    await workshopApi.cancel(); // no-op で解決する
    await workshopApi.confirm(1, true); // no-op で解決する
    expect(await workshopApi.listConversations(0)).toEqual({ items: [], hasMore: false });
    expect(await workshopApi.pickSourceFile()).toBeNull();
  });

  test("rejects desktop-only operations instead of pretending they worked", async () => {
    // 旧 client.ts で isTauri 守衛が無かった呼び出し＝ブラウザでは invoke がそのまま失敗していた。
    // fake も同じく失敗させる（黙って成功すると UI が嘘の状態を映す）。
    await expect(kbApi.createKb({ name: "t", description: "", path: "/t" })).rejects.toThrow();
    await expect(kbApi.setActiveKb("/t")).rejects.toThrow();
    await expect(kbApi.deleteKb("/t")).rejects.toThrow();
    await expect(kbApi.readEntry("entries/a.md")).rejects.toThrow();
    await expect(kbApi.saveEntry("entries/a.md", "x")).rejects.toThrow();
    await expect(workshopApi.chat([], [], "m", false, false)).rejects.toThrow();
    await expect(workshopApi.getConversation(1)).rejects.toThrow();
    await expect(
      workshopApi.saveConversation({ kbPath: "/t", id: null, sourceIds: [], messages: [] })
    ).rejects.toThrow();
  });
});

describe("setBackend", () => {
  test("swaps the implementation behind the domain api objects", async () => {
    const ref = { path: "entries/a.md", title: "a", cat: "note" };
    setBackend({
      ...fakeBackend,
      kb: { ...fakeBackend.kb, listEntries: async () => [ref] },
    });

    expect(await kbApi.listEntries()).toEqual([ref]);

    setBackend(fakeBackend);
    expect(await kbApi.listEntries()).toEqual([]);
  });
});
