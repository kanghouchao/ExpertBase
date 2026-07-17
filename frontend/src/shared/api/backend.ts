import { fakeBackend } from "./fake";
import { isTauriRuntime, tauriBackend } from "./tauri";
import type { AgentApi, Backend, KbApi, PluginApi, WorkshopApi } from "./types";

// 後端の接縫。適配器の選択はここで一度だけ（旧 client.ts の 17 個の isTauri 守衛の集約先）。
// Node（静的書き出しの prerender）では isTauri が false → fake に倒れる。
let current: Backend = isTauriRuntime() ? tauriBackend : fakeBackend;

/** 後端を丸ごと差し替える（テスト用。fake を土台に個別メソッドを上書きして使う）。 */
export function setBackend(next: Backend): void {
  current = next;
}

// 消費側の呼び出し面。呼び出し時に current へ委譲する＝ setBackend 後の呼び出しは新しい後端へ届く。

export const kbApi: KbApi = {
  listKbs: () => current.kb.listKbs(),
  createKb: (input) => current.kb.createKb(input),
  setActiveKb: (path) => current.kb.setActiveKb(path),
  deleteKb: (path) => current.kb.deleteKb(path),
  listEntries: () => current.kb.listEntries(),
  search: (query) => current.kb.search(query),
  backlinks: (title) => current.kb.backlinks(title),
  stats: () => current.kb.stats(),
  graph: () => current.kb.graph(),
  orphans: () => current.kb.orphans(),
  readEntry: (path) => current.kb.readEntry(path),
  saveEntry: (path, content) => current.kb.saveEntry(path, content),
};

export const agentApi: AgentApi = {
  hasKey: () => current.agent.hasKey(),
  listOllamaModels: () => current.agent.listOllamaModels(),
  listModels: (provider, baseUrl) => current.agent.listModels(provider, baseUrl),
  getSettings: () => current.agent.getSettings(),
  setSettings: (settings) => current.agent.setSettings(settings),
};

export const pluginApi: PluginApi = {
  listSkills: () => current.plugin.listSkills(),
};

export const workshopApi: WorkshopApi = {
  chat: (input, onPhase) => current.workshop.chat(input, onPhase),
  cancel: () => current.workshop.cancel(),
  confirm: (id, approved) => current.workshop.confirm(id, approved),
  listConversations: (offset) => current.workshop.listConversations(offset),
  getConversation: (id) => current.workshop.getConversation(id),
  saveConversation: (input) => current.workshop.saveConversation(input),
  pickSourceFile: () => current.workshop.pickSourceFile(),
};
