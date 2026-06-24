import type { ChatTurn, StructureResult } from "@/shared/api/tauri/client";

export type ProcessMessage<Source = unknown> =
  | { role: "user"; text: string; sources?: Source[] }
  | { role: "ai"; result: StructureResult };

type DraftSource = {
  id: string;
  title: string;
  preview: string;
};

export function toChatTurn<Source>(message: ProcessMessage<Source>): ChatTurn {
  return message.role === "user"
    ? { role: "user", content: message.text }
    : { role: "assistant", content: JSON.stringify(message.result) };
}

export function replaceLatestEntryResult<Source>(
  messages: ProcessMessage<Source>[],
  result: StructureResult
): ProcessMessage<Source>[] {
  const index = messages.findLastIndex(
    (message) => message.role === "ai" && message.result.kind === "entry"
  );
  if (index === -1) return messages;
  return messages.map((message, current) => (current === index ? { role: "ai", result } : message));
}

export function buildManualDraft(
  sources: DraftSource[],
  rawByPath: Record<string, string>
): StructureResult {
  return {
    kind: "entry",
    title: sources[0]?.title ?? "",
    cat: "",
    bodyMarkdown: sources
      .map((source) => stripFrontmatter(rawByPath[source.id] ?? source.preview))
      .join("\n\n---\n\n"),
    suggestedLinks: [],
  };
}

export function sameSourceIds(expected: string[], actual: string[]): boolean {
  return expected.length === actual.length && expected.every((id, index) => id === actual[index]);
}

export function canRemoveSource(messageCount: number, sourceCount: number): boolean {
  return messageCount === 0 && sourceCount > 1;
}

function stripFrontmatter(markdown: string): string {
  if (!markdown.startsWith("---")) return markdown.trim();
  const end = markdown.indexOf("\n---", 3);
  if (end === -1) return markdown.trim();
  return markdown.slice(end + 4).trim();
}

// AI 草稿生成のフェーズ（確定的状態機。UI の反馈はここから駆動する）。
export type DraftUiPhase = "idle" | "connecting" | "loadingModel" | "generating" | "done";

export function isGeneratingPhase(phase: DraftUiPhase): boolean {
  return phase === "connecting" || phase === "loadingModel" || phase === "generating";
}

/** フェーズ → i18n キー（spinner ラベル / inspector status）。 */
export function phaseLabelKey(phase: DraftUiPhase): string {
  switch (phase) {
    case "connecting":
      return "workshop.phase.connecting";
    case "loadingModel":
      return "workshop.phase.loadingModel";
    case "generating":
      return "workshop.phase.generating";
    case "done":
      return "workshop.st.done";
    default:
      return "workshop.st.idle";
  }
}
