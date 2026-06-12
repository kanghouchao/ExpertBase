import { invoke, isTauri } from "@tauri-apps/api/core";

export type KbStatus = {
  root: string;
  initialized: boolean;
};

/** Local knowledge base status, or null when running in a plain browser. */
export async function getKbStatus(): Promise<KbStatus | null> {
  if (!isTauri()) return null;
  return invoke<KbStatus>("kb_status");
}
