import { beforeEach, describe, expect, mock, test } from "bun:test";

import type { ChatPhase } from "@/shared/api/tauri/client";
import type { RunStoreState } from "./workshop-run";

// history.ts の window イベントを動かすため EventTarget を window として与える。
(globalThis as { window?: EventTarget }).window = new EventTarget();

// 後端クライアントを差し替える（実 IPC を呼ばない）。
type ChatImpl = (
  sourceIds: string[],
  messages: { role: string; content: string }[],
  model: string,
  think: boolean,
  tools: boolean,
  onPhase?: (p: ChatPhase) => void
) => Promise<string>;

let chatImpl: ChatImpl = async () => "";
let saveCalls: {
  kbPath: string;
  id: number | null;
  sourceIds: string[];
  messages: { role: string; text: string }[];
}[] = [];
let cancelCalls = 0;
let confirmCalls: { id: number; approved: boolean }[] = [];

mock.module("@/shared/api/tauri/client", () => ({
  workshopChat: (...args: Parameters<ChatImpl>) => chatImpl(...args),
  workshopCancel: async () => {
    cancelCalls += 1;
  },
  workshopConfirm: async (id: number, approved: boolean) => {
    confirmCalls.push({ id, approved });
  },
  saveWorkshopConversation: async (input: (typeof saveCalls)[number]) => {
    saveCalls.push(input);
    return {
      id: input.id ?? 99,
      title: "t",
      sourceIds: input.sourceIds,
      messages: input.messages,
      createdAt: "",
      updatedAt: "",
    };
  },
}));

const {
  startRun,
  stopActive,
  discardActive,
  isRunForConversation,
  subscribe,
  getSnapshot,
  answerConfirm,
} = await import("./workshop-run");

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
const HISTORY_EVENT = "expertbase:workshop:history-changed";

function recordSnapshots() {
  const seen: RunStoreState[] = [];
  const unsub = subscribe(() => seen.push(getSnapshot()));
  return { seen, unsub };
}

beforeEach(() => {
  chatImpl = async () => "";
  saveCalls = [];
  cancelCalls = 0;
  confirmCalls = [];
});

test("run identity includes the opaque KB path on every platform", () => {
  const run = {
    kbPath: "C:\\Users\\user\\ExpertBase\\tea",
    conversationId: 1,
    phase: "thinking" as const,
    baseHistory: [],
    narration: "",
    thinking: "",
    tools: [],
    confirm: null,
  };

  expect(isRunForConversation(run, "C:\\Users\\user\\ExpertBase\\tea", 1)).toBeTrue();
  expect(isRunForConversation(run, "/home/user/ExpertBase/tea", 1)).toBeFalse();
  expect(isRunForConversation(run, "C:\\Users\\user\\ExpertBase\\tea", 2)).toBeFalse();
});

describe("startRun", () => {
  test("streams buffers then saves the AI reply to the captured conversation id", async () => {
    chatImpl = async (_s, _m, _mo, _th, _to, onPhase) => {
      onPhase?.({ phase: "loadingModel" });
      onPhase?.({ phase: "thinking", delta: "考え" });
      onPhase?.({ phase: "narration", delta: "答え" });
      onPhase?.({ phase: "narration", delta: "だ" });
      onPhase?.({ phase: "toolCall", name: "search_kb", args: "{}" });
      onPhase?.({ phase: "toolResult", name: "search_kb", summary: "hit" });
      return "答えだ";
    };
    const { seen, unsub } = recordSnapshots();
    let historyChanged = 0;
    const onHistory = () => (historyChanged += 1);
    window.addEventListener(HISTORY_EVENT, onHistory);

    startRun({
      kbPath: "/home/user/ExpertBase/tea",
      conversationId: 7,
      sourceIds: ["/a.pdf"],
      baseHistory: [{ role: "user", text: "問い" }],
      model: "qwen3:8b",
      think: true,
      tools: true,
    });
    await tick();

    // 流式中の途中状態がスナップショットに現れる。
    expect(seen.some((s) => s.active?.narration === "答えだ")).toBeTrue();
    expect(seen.some((s) => s.active?.thinking === "考え")).toBeTrue();
    expect(seen.some((s) => (s.active?.tools.length ?? 0) > 0)).toBeTrue();

    // 捕獲した id へ、末尾に AI 返信を足して保存する。
    expect(saveCalls).toHaveLength(1);
    expect(saveCalls[0].kbPath).toBe("/home/user/ExpertBase/tea");
    expect(saveCalls[0].id).toBe(7);
    const msgs = saveCalls[0].messages;
    const last = msgs[msgs.length - 1] as { role: string; text: string; tools?: { summary?: string }[] };
    expect(last.role).toBe("ai");
    expect(last.text).toBe("答えだ");
    expect(last.tools?.[0].summary).toBe("hit");

    // 完了後はアクティブが消え、履歴更新が通知される。
    expect(getSnapshot().active).toBeNull();
    expect(historyChanged).toBe(1);

    unsub();
    window.removeEventListener(HISTORY_EVENT, onHistory);
  });
});

describe("answerConfirm", () => {
  test("surfaces the confirm request and relays the user's answer", async () => {
    let resolveChat: (v: string) => void = () => {};
    chatImpl = (_s, _m, _mo, _th, _to, onPhase) => {
      onPhase?.({ phase: "confirmRequest", id: 42, summary: "delete entries/a.md" });
      return new Promise<string>((resolve) => {
        resolveChat = resolve;
      });
    };

    startRun({
      kbPath: "/home/user/ExpertBase/tea",
      conversationId: 11,
      sourceIds: [],
      baseHistory: [{ role: "user", text: "問" }],
      model: "qwen3:8b",
      think: false,
      tools: true,
    });
    await tick();

    // 確認要求がカード用状態として現れる。
    expect(getSnapshot().active?.confirm).toEqual({ id: 42, summary: "delete entries/a.md" });

    // 応答すると後端へ回填され、カードは畳まれる。
    answerConfirm("/home/user/ExpertBase/tea", 11, true);
    expect(confirmCalls).toEqual([{ id: 42, approved: true }]);
    expect(getSnapshot().active?.confirm).toBeNull();

    // 応答済み・要求なしの二重応答は無視される。
    answerConfirm("/home/user/ExpertBase/tea", 11, false);
    expect(confirmCalls).toHaveLength(1);

    resolveChat("done");
    await tick();
  });

  test("ignores answers for a different conversation", async () => {
    let resolveChat: (v: string) => void = () => {};
    chatImpl = (_s, _m, _mo, _th, _to, onPhase) => {
      onPhase?.({ phase: "confirmRequest", id: 7, summary: "update entries/b.md" });
      return new Promise<string>((resolve) => {
        resolveChat = resolve;
      });
    };

    startRun({
      kbPath: "/home/user/ExpertBase/tea",
      conversationId: 12,
      sourceIds: [],
      baseHistory: [{ role: "user", text: "問" }],
      model: "qwen3:8b",
      think: false,
      tools: true,
    });
    await tick();

    answerConfirm("/home/user/ExpertBase/tea", 999, false);
    expect(confirmCalls).toHaveLength(0);
    expect(getSnapshot().active?.confirm).toEqual({ id: 7, summary: "update entries/b.md" });

    resolveChat("done");
    await tick();
  });
});

describe("stopActive", () => {
  test("returns the prompt and previous history when stopped before AI output", async () => {
    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = () =>
      new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });

    startRun({
      kbPath: "/home/user/ExpertBase/tea",
      conversationId: 8,
      sourceIds: [],
      baseHistory: [
        { role: "user", text: "前の問い" },
        { role: "ai", text: "前の答え" },
        { role: "user", text: "やり直す問い" },
      ],
      model: "qwen3:8b",
      think: true,
      tools: true,
    });
    await tick();

    expect(stopActive("/home/user/ExpertBase/tea", 8)).toEqual({
      prompt: "やり直す問い",
      history: [
        { role: "user", text: "前の問い" },
        { role: "ai", text: "前の答え" },
      ],
    });
    expect(getSnapshot().active).toBeNull();
    expect(cancelCalls).toBe(1);

    rejectChat(new Error("Cancelled"));
    await tick();
    expect(saveCalls).toHaveLength(0);
  });

  test("does not stop a background conversation", async () => {
    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = () =>
      new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });

    startRun({
      kbPath: "/home/user/ExpertBase/tea",
      conversationId: 9,
      sourceIds: [],
      baseHistory: [{ role: "user", text: "問" }],
      model: "qwen3:8b",
      think: false,
      tools: true,
    });
    await tick();

    expect(stopActive("/home/user/ExpertBase/tea", 10)).toBeNull();
    expect(getSnapshot().active?.conversationId).toBe(9);
    expect(cancelCalls).toBe(0);

    discardActive();
    rejectChat(new Error("Cancelled"));
    await tick();
  });

  test("keeps the partial narration as the AI reply", async () => {
    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = (_s, _m, _mo, _th, _to, onPhase) => {
      onPhase?.({ phase: "narration", delta: "途中まで" });
      return new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });
    };

    startRun({
      kbPath: "C:\\Users\\user\\ExpertBase\\tea",
      conversationId: 3,
      sourceIds: [],
      baseHistory: [{ role: "user", text: "問" }],
      model: "qwen3:8b",
      think: false,
      tools: true,
    });
    await tick();
    expect(getSnapshot().active?.narration).toBe("途中まで");

    expect(stopActive("C:\\Users\\user\\ExpertBase\\tea", 3)).toBeNull();
    expect(cancelCalls).toBe(1);
    rejectChat(new Error("Cancelled")); // 後端が接続を drop ＝ reject
    await tick();

    expect(saveCalls).toHaveLength(1);
    expect(saveCalls[0].kbPath).toBe("C:\\Users\\user\\ExpertBase\\tea");
    const msgs = saveCalls[0].messages;
    const last = msgs[msgs.length - 1] as { role: string; text: string };
    expect(last.role).toBe("ai");
    expect(last.text).toBe("途中まで");
    expect(getSnapshot().active).toBeNull();
  });
});

describe("discardActive", () => {
  test("drops the run without saving", async () => {
    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = (_s, _m, _mo, _th, _to, onPhase) => {
      onPhase?.({ phase: "narration", delta: "x" });
      return new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });
    };

    startRun({
      kbPath: "/home/user/ExpertBase/tea",
      conversationId: 5,
      sourceIds: [],
      baseHistory: [{ role: "user", text: "問" }],
      model: "qwen3:8b",
      think: false,
      tools: true,
    });
    await tick();

    discardActive();
    expect(getSnapshot().active).toBeNull();
    rejectChat(new Error("Cancelled"));
    await tick();

    expect(saveCalls).toHaveLength(0);
  });
});
