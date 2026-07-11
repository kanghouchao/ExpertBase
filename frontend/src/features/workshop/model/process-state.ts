import type { ChatTurn, WorkshopMessage, WorkshopToolEvent } from "@/shared/api";

// エージェントのツール呼び出し 1 件（表示用）。summary はツール結果到着後に埋まる。
export type ToolEvent = WorkshopToolEvent;

export type ProcessMessage = WorkshopMessage;

export function toChatTurn(message: ProcessMessage): ChatTurn {
  return message.role === "user"
    ? { role: "user", content: message.text }
    : { role: "assistant", content: message.text };
}

export function canRemoveSource(messageCount: number, sourceCount: number): boolean {
  return messageCount === 0 && sourceCount > 1;
}

// 対話の進捗フェーズ（確定的状態機。UI の反馈はここから駆動する）。
export type ChatUiPhase =
  | "idle"
  | "connecting"
  | "thinking"
  | "loadingModel"
  | "generating";

/** フェーズ → i18n キー（spinner ラベル）。 */
export function phaseLabelKey(phase: ChatUiPhase): string {
  switch (phase) {
    case "connecting":
      return "workshop.phase.connecting";
    case "thinking":
      return "workshop.phase.thinking";
    case "loadingModel":
      return "workshop.phase.loadingModel";
    case "generating":
      return "workshop.phase.generating";
    default:
      return "workshop.st.idle";
  }
}

/**
 * 実行中ラベルの i18n キー。数字は出さない。
 * 思考モデルは本文生成中を「起草」、非思考モデルは素直に「生成中」と出す。
 */
export function runningLabelKey(phase: ChatUiPhase, thinking: boolean): string {
  if (phase === "generating") {
    return thinking ? "workshop.phase.drafting" : "workshop.phase.generating";
  }
  return phaseLabelKey(phase);
}
