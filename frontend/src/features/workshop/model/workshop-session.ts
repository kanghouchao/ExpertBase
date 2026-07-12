//! 工坊視図の第二層編排(send / stop / 会話読込 / KB 切替 / モデル選択)を
//! 組件の外へ下ろした純 TS コントローラ。視図は快照を描くだけ、React 接線は
//! use-workshop-session.ts の薄い hook が担う。テストは workshop-run と同じ流儀
//! (setBackend + fake)でこのモジュールを直接実体化して行う。

import { agentApi, workshopApi, type AiProvider, type OllamaModel } from "@/shared/api";

import {
  activeKbChanged,
  notifyWorkshopHistoryChanged,
  onNewWorkshopConversation,
} from "./history";
import type { ChatUiPhase, ProcessMessage, ToolEvent } from "./process-state";
import {
  getSnapshot as getRunSnapshot,
  isRunForConversation,
  startRun,
  stopActive,
  answerConfirm,
  discardActive,
  subscribe as subscribeRun,
} from "./workshop-run";

// Tauri 外(通常ブラウザのプレビュー)で見せる仮のモデル一覧。
const PREVIEW_MODELS: OllamaModel[] = [
  { name: "qwen3:8b", thinking: true, tools: true },
  { name: "llama3.1:8b", thinking: false, tools: false },
];

/** 視図がそのまま描ける快照。派生値もここで計算済み = 視図に編排を残さない。 */
export type WorkshopSessionSnapshot = {
  // 確定態(存盤済み)
  conversationId: number | null;
  messages: ProcessMessage[];
  sourceIds: string[];
  instruction: string;
  /** 編排エラーの生の原因(翻訳は視図が translateError で行う)。 */
  error: unknown;
  // モデル態(探測結果 + 設定)
  provider: AiProvider;
  models: OllamaModel[];
  selectedModel: string;
  // 派生(available / llamaApp / プレビューを畳み込み済み)
  visibleHasOllama: boolean;
  visibleModels: OllamaModel[];
  visibleSelectedModel: string;
  selectedThinking: boolean;
  selectedTools: boolean;
  canGenerate: boolean;
  someoneGenerating: boolean;
  // run 投影(見ている会話が生成中のときだけ実時態が入る)
  generating: boolean;
  phase: ChatUiPhase;
  displayMessages: ProcessMessage[];
  thinkingBuf: string;
  narrationBuf: string;
  toolLog: ToolEvent[];
  confirmReq: { id: number; summary: string } | null;
};

export type RouteInput = {
  kbPath: string | null;
  /** URL が指す会話 id(無ければ null = 新規会話画面)。 */
  conversationId: number | null;
  available: boolean;
};

export type WorkshopSessionDeps = {
  /** router.replace の注入点(テストでは記録配列)。 */
  navigate: (url: string) => void;
};

export type WorkshopSession = ReturnType<typeof createWorkshopSession>;

/** 1 回の掛載につき 1 実例。attach() で外部購読を開始し、返り値で解除する。 */
export function createWorkshopSession(deps: WorkshopSessionDeps) {
  // ── 内部状態(快照は emit 時に丸ごと作り直す = 安定参照) ──
  let route: RouteInput = { kbPath: null, conversationId: null, available: false };
  let capturedId: number | null = null; // 存盤で捕獲した会話 id
  let messages: ProcessMessage[] = [];
  let sourceIds: string[] = [];
  let instruction = "";
  let localError: unknown = null;
  let provider: AiProvider = "ollama";
  let settingsModel = "";
  let hasOllama = false;
  let models: OllamaModel[] = [];
  let selectedModel = "";

  let prevKbPath: string | null = null;
  let prevAvailable = false;
  let lastRunKey = 0; // run の起止検知(流式 buffer の変化では会話同期を回さない)
  let loadSeq = 0; // 会話読込の競合防止(後着だけ反映)

  const listeners = new Set<() => void>();
  let snapshot = compute();

  function compute(): WorkshopSessionSnapshot {
    const run = getRunSnapshot();
    const viewedId = route.conversationId;
    const activeRun = run.active;
    const isViewingActive = isRunForConversation(activeRun, route.kbPath, viewedId);
    const someoneGenerating = activeRun !== null;

    // llama.app は能力探測がないので、設定した既定モデルを tools 対応として 1 件だけ扱う。
    const isLlamaApp = provider === "llamaApp";
    const llamaModels: OllamaModel[] = settingsModel
      ? [{ name: settingsModel, thinking: false, tools: true }]
      : [];
    const visibleHasOllama = isLlamaApp ? true : route.available ? hasOllama : true;
    const visibleModels = isLlamaApp ? llamaModels : route.available ? models : PREVIEW_MODELS;
    const visibleSelectedModel = isLlamaApp
      ? settingsModel
      : route.available
        ? selectedModel
        : selectedModel || PREVIEW_MODELS[0].name;
    const selectedInfo = visibleModels.find((model) => model.name === visibleSelectedModel);
    const selectedThinking = selectedInfo?.thinking ?? false;
    const selectedTools = selectedInfo?.tools ?? false;

    // run のエラーは見ている会話のものだけを見せる(既存の視図と同じ判定)。
    const runError =
      run.error && run.error.kbPath === route.kbPath && run.error.conversationId === viewedId
        ? run.error.cause
        : null;

    return {
      conversationId: capturedId,
      messages,
      sourceIds,
      instruction,
      error: localError ?? runError,
      provider,
      models,
      selectedModel,
      visibleHasOllama,
      visibleModels,
      visibleSelectedModel,
      selectedThinking,
      selectedTools,
      // 工坊は tools 対応モデル必須。他会話が生成中なら本地モデルは直列なので送れない。
      canGenerate:
        visibleHasOllama && !!visibleSelectedModel && selectedTools && !someoneGenerating,
      someoneGenerating,
      generating: isViewingActive,
      phase: isViewingActive ? activeRun.phase : "idle",
      displayMessages: isViewingActive ? activeRun.baseHistory : messages,
      thinkingBuf: isViewingActive ? activeRun.thinking : "",
      narrationBuf: isViewingActive ? activeRun.narration : "",
      toolLog: isViewingActive ? activeRun.tools : [],
      confirmReq: isViewingActive ? activeRun.confirm : null,
    };
  }

  function emit(): void {
    snapshot = compute();
    for (const listener of listeners) listener();
  }

  // 本地ビューだけ畳む(後台で走る生成は殺さない)。
  function resetLocal(): void {
    capturedId = null;
    messages = [];
    sourceIds = [];
    instruction = "";
    localError = null;
    loadSeq += 1; // 途中の読込結果を捨てる
  }

  // ── 会話同期: URL が指す会話を確定態へ読み込む ──
  function syncConversation(): void {
    const requested = route.conversationId;
    if (requested === null) {
      if (capturedId !== null) {
        resetLocal();
        emit();
      }
      return;
    }
    // 進行中の会話なら DB から読まず、run の実時態をそのまま描く。
    if (getRunSnapshot().active?.conversationId === requested) {
      capturedId = requested;
      emit();
      return;
    }
    localError = null;
    const seq = ++loadSeq;
    void workshopApi
      .getConversation(requested)
      .then((conversation) => {
        if (seq !== loadSeq) return; // 後着ではない読込は捨てる
        capturedId = conversation.id;
        messages = conversation.messages;
        sourceIds = conversation.sourceIds;
        emit();
      })
      .catch((cause) => {
        if (seq !== loadSeq) return;
        resetLocal();
        localError = cause;
        emit();
      });
  }

  // ── モデル探測: 設定と Ollama を別系統で読む(片方の失敗が他方を巻き込まない) ──
  async function loadModels(): Promise<void> {
    let defaultModel = "";
    try {
      const settings = await agentApi.getSettings();
      provider = settings.provider;
      settingsModel = settings.model;
      defaultModel = settings.model;
    } catch {
      // 設定読み込み失敗は既定(Ollama / 空)のまま扱う。
    }
    try {
      const [ollama, modelList] = await Promise.all([
        agentApi.hasKey(),
        agentApi.listOllamaModels(),
      ]);
      hasOllama = ollama;
      models = modelList;
      // 既定モデルの nudge: 現選択 → 設定の既定 → Qwen3 → 任意の tools 対応 → 先頭。
      const has = (name: string) => modelList.some((model) => model.name === name);
      if (!(selectedModel && has(selectedModel))) {
        selectedModel =
          defaultModel && has(defaultModel)
            ? defaultModel
            : (modelList.find((model) => /qwen3/i.test(model.name))?.name ??
              modelList.find((model) => model.tools)?.name ??
              modelList[0]?.name ??
              "");
      }
    } catch {
      hasOllama = false;
      models = [];
      selectedModel = "";
    }
    emit();
  }

  return {
    subscribe(listener: () => void): () => void {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },

    getSnapshot(): WorkshopSessionSnapshot {
      return snapshot;
    },

    /** 外部購読(run ストア・新規会話イベント)を開始する。返り値で解除。 */
    attach(): () => void {
      const unsubRun = subscribeRun(() => {
        // run の起止でだけ会話同期(完了 → DB 再読で確定態へ)。流式 buffer は投影のみ。
        const runKey = getRunSnapshot().active?.conversationId ?? 0;
        if (runKey !== lastRunKey) {
          lastRunKey = runKey;
          if (route.conversationId !== null) syncConversation();
        }
        emit();
      });
      const offNew = onNewWorkshopConversation(() => {
        resetLocal();
        deps.navigate("/workshop");
        emit();
      });
      return () => {
        unsubRun();
        offNew();
      };
    },

    /** 路由入力(KB / URL 会話 / 可用性)の変化を取り込む。冪等 = 同値なら何もしない。 */
    syncRoute(next: RouteInput): void {
      const kbSwitched = activeKbChanged(prevKbPath, next.kbPath);
      prevKbPath = next.kbPath;
      const conversationChanged =
        route.kbPath !== next.kbPath || route.conversationId !== next.conversationId;
      const becameAvailable = next.available && !prevAvailable;
      prevAvailable = next.available;
      route = next;

      if (kbSwitched) {
        discardActive(); // KB が変わると存盤目標も動く = 進行中の実行は捨てる
        resetLocal();
        deps.navigate("/workshop");
        emit();
        return;
      }
      if (becameAvailable) void loadModels();
      if (conversationChanged) syncConversation();
    },

    setInstruction(text: string): void {
      instruction = text;
      emit();
    },

    selectModel(name: string): void {
      selectedModel = name;
      emit();
    },

    /** 素材のトグル(チップの × で外す)。id は外部ファイルの絶対パス。 */
    toggleSource(path: string): void {
      sourceIds = sourceIds.includes(path)
        ? sourceIds.filter((id) => id !== path)
        : [...sourceIds, path];
      emit();
    },

    /** 外部のローカルファイルを素材に追加する(AI が read_source で読む。KB へは落とさない)。 */
    async addLocalFile(): Promise<void> {
      const path = await workshopApi.pickSourceFile();
      if (!path || sourceIds.includes(path)) return;
      sourceIds = [...sourceIds, path];
      emit();
    },

    /** 1 ターン送信。存盤が chat より先 = 後台生成が会話 id を捕獲できる。 */
    async send(): Promise<void> {
      const text = instruction.trim();
      const kbPath = route.kbPath;
      if (!snapshot.canGenerate || !text || !kbPath) return;
      const userMsg: ProcessMessage = { role: "user", text };
      const baseHistory = [...snapshot.displayMessages, userMsg];
      const ids = sourceIds;
      instruction = "";
      localError = null;
      emit();
      let id = capturedId;
      try {
        const saved = await workshopApi.saveConversation({
          kbPath,
          id,
          sourceIds: ids,
          messages: baseHistory,
        });
        id = saved.id;
      } catch (cause) {
        // 失敗時だけ入力を戻す(打ち直しさせない)。
        localError = cause;
        instruction = userMsg.text;
        emit();
        return;
      }
      capturedId = id;
      messages = baseHistory;
      emit();
      notifyWorkshopHistoryChanged();
      if (route.conversationId !== id) deps.navigate(`/workshop?conversation=${id}`);
      startRun({
        kbPath,
        conversationId: id,
        sourceIds: ids,
        baseHistory,
        model: snapshot.visibleSelectedModel,
        think: snapshot.selectedThinking,
        tools: snapshot.selectedTools,
      });
    },

    /** 進行中の生成を止める。出力前なら送信直前の入力と履歴へ戻し、回退分も存盤する。 */
    stop(): void {
      const kbPath = route.kbPath;
      const viewedId = route.conversationId;
      const rollback = stopActive(kbPath, viewedId);
      if (!rollback || !kbPath || viewedId === null) return;
      instruction = rollback.prompt;
      messages = rollback.history;
      emit();
      void workshopApi
        .saveConversation({ kbPath, id: viewedId, sourceIds, messages: rollback.history })
        .then(() => notifyWorkshopHistoryChanged())
        .catch((cause) => {
          localError = cause;
          emit();
        });
    },

    /** 破壊的操作の確認要求へ応答する(許可 / 拒否)。 */
    confirm(approved: boolean): void {
      void answerConfirm(route.kbPath, route.conversationId, approved);
    },
  };
}
