import { afterAll, beforeEach, describe, expect, test } from "bun:test";

import {
  fakeBackend,
  setBackend,
  type ChatPhase,
  type ChatTurn,
  type OllamaModel,
  type WorkshopConversation,
  type WorkshopMessage,
} from "@/shared/api";

// history.ts の window イベントを動かすため EventTarget を window として与える。
(globalThis as { window?: EventTarget }).window = new EventTarget();

// 後端を fake 土台 + 上書きで差し替える(実 IPC を呼ばない・猴補なし)。
type ChatImpl = (
  sourceIds: string[],
  messages: ChatTurn[],
  model: string,
  think: boolean,
  tools: boolean,
  onPhase?: (p: ChatPhase) => void
) => Promise<string>;

type SaveInput = {
  kbPath: string;
  id: number | null;
  sourceIds: string[];
  messages: WorkshopMessage[];
};

// tools 対応モデルが 1 件ある = canGenerate が立つ最小構成。
const MODELS: OllamaModel[] = [{ name: "qwen3:8b", thinking: true, tools: true }];

let chatImpl: ChatImpl = async () => "";
let saveImpl = async (input: SaveInput): Promise<WorkshopConversation> => ({
  id: input.id ?? 99,
  title: "t",
  sourceIds: input.sourceIds,
  messages: input.messages,
  createdAt: "",
  updatedAt: "",
});
let getConversationImpl: (id: number) => Promise<WorkshopConversation> = () =>
  Promise.reject(new Error("not stubbed"));

let calls: string[] = []; // "save" / "chat" の呼び出し順序記録
let saveCalls: SaveInput[] = [];
let getConversationCalls: number[] = [];
let cancelCalls = 0;
let navCalls: string[] = [];

const sessionTestBackend = {
  ...fakeBackend,
  agent: {
    ...fakeBackend.agent,
    hasKey: async () => true,
    listOllamaModels: async () => MODELS,
  },
  workshop: {
    ...fakeBackend.workshop,
    chat: (...args: Parameters<ChatImpl>) => {
      calls.push("chat");
      return chatImpl(...args);
    },
    cancel: async () => {
      cancelCalls += 1;
    },
    saveConversation: async (input: SaveInput) => {
      calls.push("save");
      saveCalls.push(input);
      return saveImpl(input);
    },
    getConversation: (id: number) => {
      getConversationCalls.push(id);
      return getConversationImpl(id);
    },
  },
};

import { discardActive, getSnapshot as getRunSnapshot } from "./workshop-run";
import { createWorkshopSession } from "./workshop-session";

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
const KB = "/home/user/ExpertBase/tea";

// コントローラ生成 + 購読開始。テスト末尾で detach() すること。
function makeSession() {
  const session = createWorkshopSession({ navigate: (url) => navCalls.push(url) });
  const detach = session.attach();
  return { session, detach };
}

// KB あり・モデル探測完了済みの状態まで進める(視図の初期表示に相当)。
async function readySession(requested: number | null = null) {
  const made = makeSession();
  made.session.syncRoute({ kbPath: KB, conversationId: requested, available: true });
  await tick();
  return made;
}

beforeEach(() => {
  // 適用はモジュール頂層でなくここ = ファイル実行順に依存しない。
  setBackend(sessionTestBackend);
  // 前のテストが残した進行中 run を掃除してから記録をリセットする。
  discardActive();
  chatImpl = async () => "";
  saveImpl = async (input) => ({
    id: input.id ?? 99,
    title: "t",
    sourceIds: input.sourceIds,
    messages: input.messages,
    createdAt: "",
    updatedAt: "",
  });
  getConversationImpl = () => Promise.reject(new Error("not stubbed"));
  calls = [];
  saveCalls = [];
  getConversationCalls = [];
  cancelCalls = 0;
  navCalls = [];
});

// 上書きを他のテストファイルへ漏らさない。
afterAll(() => setBackend(fakeBackend));

describe("send", () => {
  test("先に存盤してから chat を始め、新会話は id を捕獲して navigate する", async () => {
    const { session, detach } = await readySession();
    let resolveChat: (v: string) => void = () => {};
    chatImpl = () =>
      new Promise<string>((resolve) => {
        resolveChat = resolve;
      });

    session.setInstruction("問い");
    await session.send();

    // 順序約束: 存盤が chat より先 = 後台生成が会話 id を捕獲できる。
    expect(calls).toEqual(["save", "chat"]);
    expect(saveCalls[0].kbPath).toBe(KB);
    expect(saveCalls[0].id).toBeNull();
    expect(saveCalls[0].messages.at(-1)).toEqual({ role: "user", text: "問い" });

    // 新会話: 保存が返した id を捕獲し、URL を追従させる。入力欄は空へ。
    expect(session.getSnapshot().conversationId).toBe(99);
    expect(navCalls).toEqual(["/workshop?conversation=99"]);
    expect(session.getSnapshot().instruction).toBe("");

    discardActive();
    resolveChat("");
    await tick();
    detach();
  });

  test("存盤失敗: 入力を回填し、run を起こさない", async () => {
    const { session, detach } = await readySession();
    const boom = { code: "err.db" };
    saveImpl = async () => {
      throw boom;
    };

    session.setInstruction("問い");
    await session.send();

    expect(calls).toEqual(["save"]); // chat は呼ばれない
    expect(session.getSnapshot().instruction).toBe("問い");
    expect(session.getSnapshot().error).toBe(boom);
    expect(navCalls).toEqual([]);
    detach();
  });
});

describe("KB 切替", () => {
  test("進行中の run を破棄し、本地態を畳んで /workshop へ戻す", async () => {
    const { session, detach } = await readySession();
    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = () =>
      new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });
    session.setInstruction("問い");
    await session.send();
    navCalls = []; // send 分の navigate を消して切替分だけ見る

    session.syncRoute({
      kbPath: "/home/user/ExpertBase/coffee",
      conversationId: 99,
      available: true,
    });

    // 存盤目標が動くので run は保存せず捨てる。本地態も畳む。
    expect(getRunSnapshot().active).toBeNull();
    expect(cancelCalls).toBe(1);
    expect(session.getSnapshot().messages).toEqual([]);
    expect(session.getSnapshot().conversationId).toBeNull();
    expect(navCalls).toEqual(["/workshop"]);

    rejectChat(new Error("Cancelled"));
    await tick();
    expect(saveCalls).toHaveLength(1); // send の存盤のみ = 破棄後は保存しない
    detach();
  });

  test("同 KB 内の会話切替: 後台の run は生き続け、別会話を DB から描く", async () => {
    const prior: WorkshopMessage[] = [
      { role: "user", text: "前の問い" },
      { role: "ai", text: "前の答え" },
    ];
    const { session, detach } = await readySession();
    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = () =>
      new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });
    session.setInstruction("問い");
    await session.send(); // run: 会話 99

    getConversationImpl = async () => ({
      id: 3,
      title: "old",
      sourceIds: ["/a.pdf"],
      messages: prior,
      createdAt: "",
      updatedAt: "",
    });
    session.syncRoute({ kbPath: KB, conversationId: 3, available: true });
    await tick();

    const snap = session.getSnapshot();
    expect(getRunSnapshot().active?.conversationId).toBe(99); // 殺していない
    expect(cancelCalls).toBe(0);
    expect(snap.messages).toEqual(prior);
    expect(snap.sourceIds).toEqual(["/a.pdf"]);
    expect(snap.generating).toBeFalse(); // 見ている会話は生成中ではない
    expect(snap.someoneGenerating).toBeTrue(); // が、後台では誰かが生成中

    discardActive();
    rejectChat(new Error("Cancelled"));
    await tick();
    detach();
  });
});

describe("stop と会話読込", () => {
  test("stop 出力前: 促し語を回填し、履歴を回退して存盤する", async () => {
    const prior: WorkshopMessage[] = [
      { role: "user", text: "前の問い" },
      { role: "ai", text: "前の答え" },
    ];
    getConversationImpl = async () => ({
      id: 7,
      title: "t",
      sourceIds: [],
      messages: prior,
      createdAt: "",
      updatedAt: "",
    });
    const { session, detach } = await readySession(7);

    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = () =>
      new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });
    session.setInstruction("やり直す問い");
    await session.send(); // 会話 7 に存盤して run 開始(出力はまだ無い)

    session.stop();

    // 送信直前へ戻す: 入力欄に促し語、履歴は送信前へ。
    const snap = session.getSnapshot();
    expect(snap.instruction).toBe("やり直す問い");
    expect(snap.messages).toEqual(prior);
    expect(cancelCalls).toBe(1);

    // 回退した履歴も存盤される(user メッセージを残さない)。
    await tick();
    expect(saveCalls.at(-1)?.id).toBe(7);
    expect(saveCalls.at(-1)?.messages).toEqual(prior);

    rejectChat(new Error("Cancelled"));
    await tick();
    detach();
  });

  test("URL の会話を DB から読み込む", async () => {
    const conv: WorkshopConversation = {
      id: 7,
      title: "t",
      sourceIds: ["/a.pdf"],
      messages: [
        { role: "user", text: "問い" },
        { role: "ai", text: "答え" },
      ],
      createdAt: "",
      updatedAt: "",
    };
    getConversationImpl = async () => conv;
    const { session, detach } = await readySession(7);

    const snap = session.getSnapshot();
    expect(getConversationCalls).toEqual([7]);
    expect(snap.conversationId).toBe(7);
    expect(snap.messages).toEqual(conv.messages);
    expect(snap.sourceIds).toEqual(["/a.pdf"]);
    detach();
  });

  test("生成中の会話は DB を読まず run の実時態を描く", async () => {
    const { session, detach } = await readySession();
    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = (_s, _m, _mo, _th, _to, onPhase) => {
      onPhase?.({ phase: "narration", delta: "途中" });
      return new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });
    };
    session.setInstruction("問い");
    await session.send(); // run: 会話 99

    // navigate 後の URL 反映(視図なら searchParams 変化で syncRoute が走る)。
    session.syncRoute({ kbPath: KB, conversationId: 99, available: true });
    await tick();

    const snap = session.getSnapshot();
    expect(getConversationCalls).toEqual([]); // DB は読まない
    expect(snap.generating).toBeTrue();
    expect(snap.narrationBuf).toBe("途中");
    expect(snap.displayMessages).toEqual([{ role: "user", text: "問い" }]);

    discardActive();
    rejectChat(new Error("Cancelled"));
    await tick();
    detach();
  });

  test("読込失敗: 本地態を畳み、原因を error に出す", async () => {
    const boom = { code: "err.notFound" };
    getConversationImpl = () => Promise.reject(boom);
    const { session, detach } = await readySession(7);

    const snap = session.getSnapshot();
    expect(snap.messages).toEqual([]);
    expect(snap.conversationId).toBeNull();
    expect(snap.error).toBe(boom);
    detach();
  });

  test("在途の読込は「新規会話へ移動」で無効化される", async () => {
    // 旧視図は effect cleanup(current=false)で在途読込を捨てていた。その等価物を鎖定する。
    let resolveLoad: (c: WorkshopConversation) => void = () => {};
    getConversationImpl = () =>
      new Promise<WorkshopConversation>((resolve) => {
        resolveLoad = resolve;
      });
    const { session, detach } = await readySession(7); // 読込は在途のまま

    session.syncRoute({ kbPath: KB, conversationId: null, available: true }); // 新規会話へ
    resolveLoad({
      id: 7,
      title: "t",
      sourceIds: ["/a.pdf"],
      messages: [{ role: "user", text: "旧" }],
      createdAt: "",
      updatedAt: "",
    });
    await tick();

    // 遅れて落ちた読込が新規会話画面を汚染しない。
    const snap = session.getSnapshot();
    expect(snap.messages).toEqual([]);
    expect(snap.sourceIds).toEqual([]);
    expect(snap.conversationId).toBeNull();
    detach();
  });

  test("在途の読込は「生成中の会話へ移動」でも無効化される", async () => {
    const { session, detach } = await readySession();
    let rejectChat: (reason: unknown) => void = () => {};
    chatImpl = () =>
      new Promise<string>((_resolve, reject) => {
        rejectChat = reject;
      });
    session.setInstruction("問い");
    await session.send(); // run: 会話 99

    // 会話 3 の読込を在途にしたまま、生成中の会話 99 へ戻る。
    let resolveLoad: (c: WorkshopConversation) => void = () => {};
    getConversationImpl = () =>
      new Promise<WorkshopConversation>((resolve) => {
        resolveLoad = resolve;
      });
    session.syncRoute({ kbPath: KB, conversationId: 3, available: true });
    session.syncRoute({ kbPath: KB, conversationId: 99, available: true });
    resolveLoad({
      id: 3,
      title: "old",
      sourceIds: ["/other.pdf"],
      messages: [{ role: "user", text: "旧" }],
      createdAt: "",
      updatedAt: "",
    });
    await tick();

    // 会話 3 の素材・id で汚染されない(汚染されると stop() が誤った素材を存盤する)。
    const snap = session.getSnapshot();
    expect(snap.conversationId).toBe(99);
    expect(snap.sourceIds).toEqual([]);
    expect(snap.displayMessages).toEqual([{ role: "user", text: "問い" }]); // run 実時態

    discardActive();
    rejectChat(new Error("Cancelled"));
    await tick();
    detach();
  });

  test("読込失敗後に別会話へ移ると、error は settle を待たず即座に消える", async () => {
    const boom = { code: "err.notFound" };
    getConversationImpl = () => Promise.reject(boom);
    const { session, detach } = await readySession(7);
    expect(session.getSnapshot().error).toBe(boom);

    getConversationImpl = () => new Promise(() => {}); // 次の読込は永遠に settle しない
    session.syncRoute({ kbPath: KB, conversationId: 8, available: true });

    expect(session.getSnapshot().error).toBeNull();
    detach();
  });

  test("生成中に再掛載しても、run 収尾で DB から定稿を読み直す", async () => {
    // 最後の流式イベント後〜finishSave の間に再掛載した窓でも収尾を見逃さないこと。
    const { session: first, detach: detachFirst } = await readySession();
    let resolveChat: (v: string) => void = () => {};
    chatImpl = () =>
      new Promise<string>((resolve) => {
        resolveChat = resolve;
      });
    first.setInstruction("問い");
    await first.send(); // run: 会話 99
    detachFirst();

    // 再掛載(新実例)。以降、収尾まで流式イベントは一度も来ない。
    const { session, detach } = makeSession();
    session.syncRoute({ kbPath: KB, conversationId: 99, available: true });
    await tick();
    expect(session.getSnapshot().generating).toBeTrue();

    getConversationImpl = async () => ({
      id: 99,
      title: "t",
      sourceIds: [],
      messages: [
        { role: "user", text: "問い" },
        { role: "ai", text: "答え" },
      ],
      createdAt: "",
      updatedAt: "",
    });
    resolveChat("答え");
    await tick(); // run の finishSave
    await tick(); // 収尾検知 → DB 再読

    expect(session.getSnapshot().generating).toBeFalse();
    expect(session.getSnapshot().messages.at(-1)).toEqual({ role: "ai", text: "答え" });
    detach();
  });

  test("run 完了: 保存済みの会話を DB から読み直して確定態へ入れる", async () => {
    const { session, detach } = await readySession();
    let resolveChat: (v: string) => void = () => {};
    chatImpl = () =>
      new Promise<string>((resolve) => {
        resolveChat = resolve;
      });
    session.setInstruction("問い");
    await session.send();
    session.syncRoute({ kbPath: KB, conversationId: 99, available: true }); // URL 反映

    getConversationImpl = async () => ({
      id: 99,
      title: "t",
      sourceIds: [],
      messages: [
        { role: "user", text: "問い" },
        { role: "ai", text: "答え" },
      ],
      createdAt: "",
      updatedAt: "",
    });
    resolveChat("答え");
    await tick(); // run の finishSave
    await tick(); // 完了検知 → DB 再読

    const snap = session.getSnapshot();
    expect(snap.generating).toBeFalse();
    expect(snap.messages.at(-1)).toEqual({ role: "ai", text: "答え" });
    detach();
  });
});
