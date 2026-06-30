import type { WorkshopConversationSummary } from "@/shared/api/tauri/client";

export const HISTORY_PAGE_SIZE = 20;

const NEW_CONVERSATION_EVENT = "expertbase:workshop:new-conversation";
const HISTORY_CHANGED_EVENT = "expertbase:workshop:history-changed";

export function parseConversationId(value: string | null): number | null {
  if (!value || !/^\d+$/.test(value)) return null;
  const id = Number(value);
  return Number.isSafeInteger(id) && id > 0 ? id : null;
}

export function collapseHistory(
  items: WorkshopConversationSummary[]
): WorkshopConversationSummary[] {
  return items.slice(0, HISTORY_PAGE_SIZE);
}

export function activeKbChanged(previous: string | null, current: string | null): boolean {
  return previous !== null && previous !== current;
}

export function createConversationRunGuard() {
  let current = 0;
  return {
    start(): number {
      current += 1;
      return current;
    },
    invalidate(): void {
      current += 1;
    },
    isCurrent(run: number): boolean {
      return run === current;
    },
  };
}

export function requestNewWorkshopConversation(): void {
  window.dispatchEvent(new Event(NEW_CONVERSATION_EVENT));
}

export function onNewWorkshopConversation(listener: () => void): () => void {
  window.addEventListener(NEW_CONVERSATION_EVENT, listener);
  return () => window.removeEventListener(NEW_CONVERSATION_EVENT, listener);
}

export function notifyWorkshopHistoryChanged(): void {
  window.dispatchEvent(new Event(HISTORY_CHANGED_EVENT));
}

export function onWorkshopHistoryChanged(listener: () => void): () => void {
  window.addEventListener(HISTORY_CHANGED_EVENT, listener);
  return () => window.removeEventListener(HISTORY_CHANGED_EVENT, listener);
}
