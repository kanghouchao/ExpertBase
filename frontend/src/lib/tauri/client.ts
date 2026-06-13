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
