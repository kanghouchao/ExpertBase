import { invoke, isTauri, Channel } from "@tauri-apps/api/core";

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
export type SearchHit = { path: string; title: string; excerpt: string };

/** ダッシュボード統計。 */
export type Stats = { entries: number; links: number; orphans: number };

/** グラフ描画データ。edges は解決済みの (srcPath, dstPath)。 */
export type GraphData = { nodes: EntryRef[]; edges: [string, string][] };

/** 受信箱素材の状態。 */
export type InboxItem = {
  path: string;
  type: string;
  source: string;
  status: string;
  capturedAt: string;
};

/** AI の応答。kind="chat" のときは bodyMarkdown に会話返信が入り、他は空。 */
export type StructureResult = {
  kind: "entry" | "chat";
  title: string;
  cat: string;
  bodyMarkdown: string;
  suggestedLinks: string[];
};

/** 会話の 1 ターン（多輪・記憶のためフロントが履歴を組み立てて渡す）。 */
export type ChatTurn = {
  role: "user" | "assistant";
  content: string;
};

/** 草稿生成のフェーズイベント（Rust DraftEvent と一致）。 */
export type DraftPhase =
  | { phase: "retrieving" }
  | { phase: "thinking"; delta: string }
  | { phase: "loadingModel" }
  | { phase: "generating"; chars: number }
  | { phase: "structuring"; chars: number }
  | { phase: "narration"; delta: string }
  | { phase: "toolCall"; name: string; args: string }
  | { phase: "toolResult"; name: string; summary: string };

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

/** 受信箱素材の生 Markdown を読む。 */
export async function readInboxMaterial(path: string): Promise<string> {
  return invoke<string>("kb_read_inbox_material", { path });
}

/** 条目を上書き保存する（frontmatter 検証付き）。 */
export async function saveEntry(path: string, content: string): Promise<void> {
  await invoke("kb_save_entry", { path, content });
}

/** 受信箱素材を削除する（素材・添付・インデックスをまとめて消す）。 */
export async function deleteInboxMaterial(path: string): Promise<void> {
  await invoke("kb_delete_inbox_material", { path });
}

/** 受信箱一覧。 */
export async function listInbox(): Promise<InboxItem[]> {
  if (!isTauri()) return [];
  return invoke<InboxItem[]>("kb_list_inbox");
}

/** テキスト/Markdown を取り込む。確定した素材の相対パスを返す。 */
export async function captureText(content: string, source: string): Promise<string> {
  return invoke<string>("capture_text", { content, source });
}

/** ローカルファイルを取り込む。 */
export async function captureFile(path: string): Promise<string> {
  return invoke<string>("capture_file", { path });
}

/** Web ページを取り込む。 */
export async function captureWeb(url: string): Promise<string> {
  return invoke<string>("capture_web", { url });
}

/** 録音した WAV を取り込む。確定した素材の相対パスを返す。 */
export async function captureAudio(wav: Uint8Array, source: string): Promise<string> {
  return invoke<string>("capture_audio", { wav, source });
}

/** モデルダウンロードの進捗（Rust 側 DownloadProgress と一致）。 */
export type DownloadProgress = { downloaded: number; total: number | null };

/**
 * 受信箱の audio 素材を転写し、本文へ書き戻す。転写後のテキストを返す。
 * 初回はモデルをダウンロードし、`onProgress` で進捗を受け取る。
 * language は "auto" | "zh" | "ja" | "en"。
 */
export async function transcribeMaterial(
  inboxPath: string,
  language: string,
  onProgress?: (progress: DownloadProgress) => void
): Promise<string> {
  const channel = new Channel<DownloadProgress>();
  if (onProgress) channel.onmessage = onProgress;
  return invoke<string>("transcribe_material", { inboxPath, language, onProgress: channel });
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

/** 複数の受信箱素材 + 会話履歴から AI 応答を生成する。onPhase で進捗フェーズを受け取る。 */
export async function workshopDraft(
  inboxPaths: string[],
  messages: ChatTurn[],
  model: string,
  think: boolean,
  tools: boolean,
  onPhase?: (phase: DraftPhase) => void
): Promise<StructureResult> {
  const channel = new Channel<DraftPhase>();
  if (onPhase) channel.onmessage = onPhase;
  return invoke<StructureResult>("workshop_draft", {
    inboxPaths,
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

/** 承認内容を条目として確定する。確定した条目の相対パスを返す。 */
export async function workshopConfirm(input: {
  inboxPaths: string[];
  title: string;
  cat: string;
  body: string;
}): Promise<string> {
  return invoke<string>("workshop_confirm", input);
}
