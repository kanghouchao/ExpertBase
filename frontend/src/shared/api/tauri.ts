import { invoke, isTauri, Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  Backend,
  ChatPhase,
  EntryRef,
  GraphData,
  Kb,
  KbList,
  OllamaModel,
  SearchHit,
  Stats,
  AiSettings,
  WorkshopConversation,
  WorkshopConversationPage,
} from "./types";

// Tauri IPC 適配器。@tauri-apps の import は全庫でこのファイルだけ。
// isTauri 守衛は個々の呼び出しに持たない＝適配器の選択（backend.ts）が唯一の分岐。

export const isTauriRuntime = isTauri;

export const tauriBackend: Backend = {
  kb: {
    listKbs: () => invoke<KbList>("kb_list"),
    createKb: (input) => invoke<Kb>("kb_create", input),
    setActiveKb: async (path) => {
      await invoke("kb_set_active", { path });
    },
    deleteKb: async (path) => {
      await invoke("kb_delete", { path });
    },
    listEntries: () => invoke<EntryRef[]>("kb_list_entries"),
    search: (query) => invoke<SearchHit[]>("kb_search", { query }),
    backlinks: (title) => invoke<EntryRef[]>("kb_backlinks", { title }),
    stats: () => invoke<Stats>("kb_stats"),
    graph: () => invoke<GraphData>("kb_graph"),
    orphans: () => invoke<EntryRef[]>("kb_orphans"),
    readEntry: (path) => invoke<string>("kb_read_entry", { path }),
    saveEntry: async (path, content) => {
      await invoke("kb_save_entry", { path, content });
    },
  },
  agent: {
    hasKey: () => invoke<boolean>("ai_has_key"),
    listOllamaModels: () => invoke<OllamaModel[]>("ai_list_ollama_models"),
    listModels: (provider, baseUrl) =>
      invoke<OllamaModel[]>("ai_list_models", { provider, baseUrl }),
    getSettings: () => invoke<AiSettings>("ai_get_settings"),
    setSettings: async (settings) => {
      await invoke("ai_set_settings", { settings });
    },
  },
  workshop: {
    chat: (sourceIds, messages, model, think, tools, onPhase) => {
      const channel = new Channel<ChatPhase>();
      if (onPhase) channel.onmessage = onPhase;
      return invoke<string>("workshop_chat", {
        sourceIds,
        messages,
        model,
        think,
        tools,
        onEvent: channel,
      });
    },
    cancel: async () => {
      await invoke("workshop_cancel");
    },
    confirm: async (id, approved) => {
      await invoke("workshop_confirm", { id, approved });
    },
    listConversations: (offset) =>
      invoke<WorkshopConversationPage>("workshop_list_conversations", { offset }),
    getConversation: (id) => invoke<WorkshopConversation>("workshop_get_conversation", { id }),
    saveConversation: (input) => invoke<WorkshopConversation>("workshop_save_conversation", input),
    pickSourceFile: async () => {
      const picked = await open({ multiple: false, directory: false });
      return typeof picked === "string" ? picked : null;
    },
  },
};
