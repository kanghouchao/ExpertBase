import { invoke, isTauri } from "@tauri-apps/api/core";

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

/** AI が生成した構造化草稿。 */
export type StructureResult = {
  title: string;
  cat: string;
  bodyMarkdown: string;
  suggestedLinks: string[];
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

/** Anthropic API キーを保存する（Rust 側にのみ保存、UI は保持しない）。 */
export async function aiSetKey(key: string): Promise<void> {
  await invoke("ai_set_key", { key });
}

/** API キーが設定済みか。 */
export async function aiHasKey(): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("ai_has_key");
}

/** 受信箱素材から AI 構造化草稿を生成する。 */
export async function workshopDraft(
  inboxPath: string,
  instruction: string
): Promise<StructureResult> {
  return invoke<StructureResult>("workshop_draft", { inboxPath, instruction });
}

/** 承認内容を条目として確定する。確定した条目の相対パスを返す。 */
export async function workshopConfirm(input: {
  inboxPath: string;
  title: string;
  cat: string;
  body: string;
}): Promise<string> {
  return invoke<string>("workshop_confirm", input);
}
