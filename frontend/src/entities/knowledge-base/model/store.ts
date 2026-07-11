"use client";

// ナレッジベース一覧のクライアント側ストア。
// AppShell が起動時に load() を呼び、各ビューは useKbStore() で参照する。
// providers.tsx と同じ useSyncExternalStore パターンを踏襲する。

import { useSyncExternalStore } from "react";

import { kbApi, type Kb } from "@/shared/api";

export type KbState = {
  // 初回の読み込みが完了したか
  loaded: boolean;
  // Tauri 環境で実行されているか（false なら作成・切替は不可）
  available: boolean;
  kbs: Kb[];
  active: Kb | null;
  defaultParent: string;
  error: string | null;
};

const INITIAL: KbState = {
  loaded: false,
  available: false,
  kbs: [],
  active: null,
  defaultParent: "~/ExpertBase",
  error: null,
};

let state: KbState = INITIAL;
const listeners = new Set<() => void>();

function emit(next: KbState): void {
  state = next;
  listeners.forEach((l) => l());
}

function subscribe(cb: () => void): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

function getSnapshot(): KbState {
  return state;
}

function getServerSnapshot(): KbState {
  return INITIAL;
}

/** ナレッジベース一覧を再取得する。 */
export async function refreshKbs(): Promise<void> {
  try {
    const list = await kbApi.listKbs();
    if (!list) {
      emit({ ...INITIAL, loaded: true });
      return;
    }
    const active = list.kbs.find((kb) => kb.path === list.active) ?? null;
    emit({
      loaded: true,
      available: true,
      kbs: list.kbs,
      active,
      defaultParent: list.defaultParent,
      error: null,
    });
  } catch (e) {
    emit({
      ...INITIAL,
      loaded: true,
      available: true,
      error: e instanceof Error ? e.message : String(e),
    });
  }
}

/** ナレッジベースを新規作成し、一覧を更新する。失敗時は例外を投げる。 */
export async function createAndActivateKb(input: {
  name: string;
  description: string;
  path: string;
}): Promise<void> {
  await kbApi.createKb(input);
  await refreshKbs();
}

/** アクティブなナレッジベースを切り替え、一覧を更新する。 */
export async function switchKb(path: string): Promise<void> {
  await kbApi.setActiveKb(path);
  await refreshKbs();
}

/** ナレッジベースを削除し、一覧を更新する。失敗時は例外を投げる。 */
export async function removeKb(path: string): Promise<void> {
  try {
    await kbApi.deleteKb(path);
  } finally {
    await refreshKbs();
  }
}

export function useKbStore(): KbState {
  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}
