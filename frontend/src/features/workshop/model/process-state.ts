import type { ChatTurn, StructureResult } from "@/shared/api/tauri/client";

export type ProcessMessage<Source = unknown> =
  | { role: "user"; text: string; sources?: Source[] }
  | { role: "ai"; result: StructureResult; thinking?: string; narration?: string };

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
  return messages.map((message, current) =>
    current === index && message.role === "ai" ? { ...message, result } : message
  );
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
export type DraftUiPhase =
  | "idle"
  | "connecting"
  | "retrieving"
  | "thinking"
  | "loadingModel"
  | "generating"
  | "structuring"
  | "done";

export function isGeneratingPhase(phase: DraftUiPhase): boolean {
  return (
    phase === "connecting" ||
    phase === "retrieving" ||
    phase === "thinking" ||
    phase === "loadingModel" ||
    phase === "generating" ||
    phase === "structuring"
  );
}

/** フェーズ → i18n キー（spinner ラベル / inspector status）。 */
export function phaseLabelKey(phase: DraftUiPhase): string {
  switch (phase) {
    case "connecting":
      return "workshop.phase.connecting";
    case "retrieving":
      return "workshop.phase.retrieving";
    case "thinking":
      return "workshop.phase.thinking";
    case "loadingModel":
      return "workshop.phase.loadingModel";
    case "generating":
      return "workshop.phase.generating";
    case "structuring":
      return "workshop.phase.structuring";
    case "done":
      return "workshop.st.done";
    default:
      return "workshop.st.idle";
  }
}

/**
 * 実行中ラベルの i18n キー。数字は出さない。
 * 二段式（思考モデル）は generating=起草 / structuring=整理 と段で区別し、
 * 非思考モデルの単段 generating は素直に「生成中」を出す。
 */
export function runningLabelKey(phase: DraftUiPhase, thinking: boolean): string {
  if (phase === "generating") {
    return thinking ? "workshop.phase.drafting" : "workshop.phase.generating";
  }
  return phaseLabelKey(phase);
}

/**
 * source と draft の行差分（確定的・実測値。品質スコアのような偽装はしない）。
 * 空白行と前後空白を無視し、draft だけにある行を added、source だけにある行を removed とする。
 */
export function lineDiff(source: string, draft: string): { added: number; removed: number } {
  const lines = (text: string) =>
    text
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
  const src = new Set(lines(source));
  const dst = new Set(lines(draft));
  let added = 0;
  for (const line of dst) if (!src.has(line)) added += 1;
  let removed = 0;
  for (const line of src) if (!dst.has(line)) removed += 1;
  return { added, removed };
}
