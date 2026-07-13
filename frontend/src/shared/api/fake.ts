import { DEFAULT_AI_SETTINGS, type Backend } from "./types";

// Tauri 外（通常のブラウザ・prerender）用の空実装。
// 旧 client.ts の isTauri 守衛の回退値を正確に鏡写しする＝空状態 + 禁用動作の約定を守る。
// 守衛が無かった呼び出し（デスクトップ必須の操作）は同じく失敗させる＝黙って成功して嘘の状態を作らない。

function desktopOnly(): Promise<never> {
  return Promise.reject(new Error("Expert Base backend is unavailable outside the desktop shell"));
}

export const fakeBackend: Backend = {
  kb: {
    listKbs: async () => null,
    createKb: () => desktopOnly(),
    setActiveKb: () => desktopOnly(),
    deleteKb: () => desktopOnly(),
    listEntries: async () => [],
    search: async () => [],
    backlinks: async () => [],
    stats: async () => null,
    graph: async () => ({ nodes: [], edges: [] }),
    orphans: async () => [],
    readEntry: () => desktopOnly(),
    saveEntry: () => desktopOnly(),
  },
  agent: {
    hasKey: async () => false,
    listOllamaModels: async () => [],
    listModels: async () => [],
    getSettings: async () => DEFAULT_AI_SETTINGS,
    setSettings: async () => {},
  },
  plugin: {
    listSkills: async () => [],
  },
  workshop: {
    chat: () => desktopOnly(),
    cancel: async () => {},
    confirm: async () => {},
    listConversations: async () => ({ items: [], hasMore: false }),
    getConversation: () => desktopOnly(),
    saveConversation: () => desktopOnly(),
    pickSourceFile: async () => null,
  },
};
