import { invoke, isTauri, Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

/** 後端 IPC の統一エラー契約。code は前端辞書の完全な key（例 "err.kb.nameRequired"）。*/
export type AppError = { code: string; params?: Record<string, string> };

// ナレッジベースはパスを一意な識別子として扱う（Rust 側と同じ契約）。
export type Kb = {
  name: string;
  path: string;
};

export type KbList = {
  kbs: Kb[];
  active: string | null;
  // 新規作成ウィザードで提示する既定の親ディレクトリ（例: ~/ExpertBase）
  defaultParent: string;
};

/** 登録済みナレッジベース一覧。Tauri 外（通常のブラウザ）では null。 */
export async function listKbs(): Promise<KbList | null> {
  if (!isTauri()) return null;
  return invoke<KbList>("kb_list");
}

/** ナレッジベースを新規作成して登録し、アクティブに切り替える。 */
export async function createKb(input: {
  name: string;
  description: string;
  path: string;
}): Promise<Kb> {
  return invoke<Kb>("kb_create", input);
}

/** 登録済みナレッジベースをアクティブに切り替える。 */
export async function setActiveKb(path: string): Promise<void> {
  await invoke("kb_set_active", { path });
}

/** ナレッジベースを削除する（レジストリ登録解除＋ディスク上のフォルダ削除）。不可逆。 */
export async function deleteKb(path: string): Promise<void> {
  await invoke("kb_delete", { path });
}

// ── データ層（Rust の派生インデックスから来る型。camelCase は Rust 側と一致） ──

/** 条目の軽量参照（一覧・バックリンク・孤立・グラフノード）。 */
export type EntryRef = { path: string; title: string; cat: string };

/** 全文検索の 1 件。 */
export type SearchHit = { path: string; title: string; cat: string; excerpt: string };

/** ダッシュボード統計。 */
export type Stats = { entries: number; links: number; orphans: number };

/** グラフ描画データ。edges は解決済みの (srcPath, dstPath)。 */
export type GraphData = { nodes: EntryRef[]; edges: [string, string][] };

/** 会話の 1 ターン（多輪・記憶のためフロントが履歴を組み立てて渡す）。 */
export type ChatTurn = {
  role: "user" | "assistant";
  content: string;
};

export type WorkshopToolEvent = { name: string; args: string; summary?: string };

export type WorkshopMessage =
  | { role: "user"; text: string; thinking?: never; tools?: never }
  | { role: "ai"; text: string; thinking?: string; tools?: WorkshopToolEvent[] };

export type WorkshopConversationSummary = {
  id: number;
  title: string;
  updatedAt: string;
};

export type WorkshopConversationPage = {
  items: WorkshopConversationSummary[];
  hasMore: boolean;
};

export type WorkshopConversation = {
  id: number;
  title: string;
  sourceIds: string[];
  messages: WorkshopMessage[];
  createdAt: string;
  updatedAt: string;
};

/** 対話の進捗イベント（Rust StreamProgress と一致）。 */
export type ChatPhase =
  | { phase: "thinking"; delta: string }
  | { phase: "loadingModel" }
  | { phase: "narration"; delta: string }
  | { phase: "toolCall"; name: string; args: string }
  | { phase: "toolResult"; name: string; summary: string }
  | { phase: "confirmRequest"; id: number; summary: string };

export type OllamaModel = {
  name: string;
  thinking: boolean;
  tools: boolean;
};

/** インデックスをファイルから再構築する。 */
export async function rebuildIndex(): Promise<void> {
  if (!isTauri()) return;
  await invoke("kb_rebuild_index");
}

/** 条目一覧（更新が新しい順）。Tauri 外では空配列。 */
export async function listEntries(): Promise<EntryRef[]> {
  if (!isTauri()) return [];
  return invoke<EntryRef[]>("kb_list_entries");
}

/** 全文検索（3 文字以上）。 */
export async function searchEntries(query: string): Promise<SearchHit[]> {
  if (!isTauri()) return [];
  return invoke<SearchHit[]>("kb_search", { query });
}

/** 指定タイトルへのバックリンク。 */
export async function backlinks(title: string): Promise<EntryRef[]> {
  if (!isTauri()) return [];
  return invoke<EntryRef[]>("kb_backlinks", { title });
}

/** ダッシュボード統計。Tauri 外では null。 */
export async function stats(): Promise<Stats | null> {
  if (!isTauri()) return null;
  return invoke<Stats>("kb_stats");
}

/** グラフデータ。 */
export async function graph(): Promise<GraphData> {
  if (!isTauri()) return { nodes: [], edges: [] };
  return invoke<GraphData>("kb_graph");
}

/** 孤立条目。 */
export async function orphans(): Promise<EntryRef[]> {
  if (!isTauri()) return [];
  return invoke<EntryRef[]>("kb_orphans");
}

/** 条目の生 Markdown を読む。 */
export async function readEntry(path: string): Promise<string> {
  return invoke<string>("kb_read_entry", { path });
}

/** 条目を上書き保存する（frontmatter 検証付き）。 */
export async function saveEntry(path: string, content: string): Promise<void> {
  await invoke("kb_save_entry", { path, content });
}

/** ローカル Ollama が応答するか。 */
export async function aiHasKey(): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("ai_has_key");
}

/** ローカル Ollama に存在するモデル一覧。 */
export async function listOllamaModels(): Promise<OllamaModel[]> {
  if (!isTauri()) return [];
  return invoke<OllamaModel[]>("ai_list_ollama_models");
}

/** AI プロバイダ（Rust Provider と一致）。ローカル端点のみ。 */
export type AiProvider = "ollama" | "llamaApp";

/** AI 設定（Rust AiSettings と一致）。URL は空欄なら後端が provider 既定へ解決する。 */
export type AiSettings = {
  provider: AiProvider;
  model: string;
  ollamaUrl: string;
  llamaAppUrl: string;
  braveApiKey: string;
};

export const DEFAULT_AI_SETTINGS: AiSettings = {
  provider: "ollama",
  model: "",
  ollamaUrl: "",
  llamaAppUrl: "",
  braveApiKey: "",
};

/** provider 既定の URL（空欄時のプレースホルダ表示用。実際の解決は後端）。 */
export const DEFAULT_PROVIDER_URL: Record<AiProvider, string> = {
  ollama: "http://127.0.0.1:11434",
  llamaApp: "http://127.0.0.1:8080/v1",
};

/** 指定 provider + URL でモデル一覧を取る（設定画面の「検証」）。空 URL は後端が既定へ解決。 */
export async function listModels(provider: AiProvider, baseUrl: string): Promise<OllamaModel[]> {
  if (!isTauri()) return [];
  return invoke<OllamaModel[]>("ai_list_models", { provider, baseUrl });
}

/** 保存済み AI 設定を読む（既定は Ollama）。Tauri 外では既定値。 */
export async function getAiSettings(): Promise<AiSettings> {
  if (!isTauri()) return DEFAULT_AI_SETTINGS;
  return invoke<AiSettings>("ai_get_settings");
}

/** AI 設定を保存する。 */
export async function setAiSettings(settings: AiSettings): Promise<void> {
  if (!isTauri()) return;
  await invoke("ai_set_settings", { settings });
}

/** 添付素材（外部ファイルの絶対パス id）+ 会話履歴で対話エージェントを 1 ターン回す。
 * 最終返信本文を返す。onPhase で進捗を受け取る。 */
export async function workshopChat(
  sourceIds: string[],
  messages: ChatTurn[],
  model: string,
  think: boolean,
  tools: boolean,
  onPhase?: (phase: ChatPhase) => void
): Promise<string> {
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
}

/** 進行中の生成を中断する（停止ボタン）。共有フラグを立てるだけで即返る。 */
export async function workshopCancel(): Promise<void> {
  if (!isTauri()) return;
  await invoke("workshop_cancel");
}

/** エージェントの確認要求へ応答する（許可 / 拒否）。 */
export async function workshopConfirm(id: number, approved: boolean): Promise<void> {
  if (!isTauri()) return;
  await invoke("workshop_confirm", { id, approved });
}

/** アクティブ KB の対話履歴を取得する。 */
export async function listWorkshopConversations(
  offset: number
): Promise<WorkshopConversationPage> {
  if (!isTauri()) return { items: [], hasMore: false };
  return invoke<WorkshopConversationPage>("workshop_list_conversations", { offset });
}

/** アクティブ KB から保存済み対話を取得する。 */
export async function getWorkshopConversation(id: number): Promise<WorkshopConversation> {
  return invoke<WorkshopConversation>("workshop_get_conversation", { id });
}

/** 完了済みの対話状態をアクティブ KB へ保存する。 */
export async function saveWorkshopConversation(input: {
  kbPath: string;
  id: number | null;
  sourceIds: string[];
  messages: WorkshopMessage[];
}): Promise<WorkshopConversation> {
  return invoke<WorkshopConversation>("workshop_save_conversation", input);
}

/** ローカルファイルを 1 つ選ぶ（素材として添付する用）。選ばなければ null（Tauri 外も null）。 */
export async function pickLocalFile(): Promise<string | null> {
  if (!isTauri()) return null;
  const picked = await open({ multiple: false, directory: false });
  return typeof picked === "string" ? picked : null;
}
