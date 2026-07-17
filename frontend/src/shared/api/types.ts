// 後端契約の型と、域ごとの接口（接縫の契約）。
// Rust 側と一致する形（camelCase / 値の意味）はここで一元管理する。

/** 後端 IPC の統一エラー契約。code は前端辞書の完全な key（例 "err.kb.nameRequired"）。*/
export type AppError = { code: string; params?: Record<string, string> };

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

// ── データ層(Rust の派生インデックスから来る型。camelCase は Rust 側と一致) ──

/** 条目の軽量参照（一覧・バックリンク・孤立・グラフノード）。 */
export type EntryRef = { path: string; title: string; cat: string };

/** 全文検索の 1 件。 */
export type SearchHit = { path: string; title: string; cat: string; excerpt: string };

/** ダッシュボード統計。 */
export type Stats = { entries: number; links: number; orphans: number };

/** グラフ描画データ。edges は解決済みの (srcPath, dstPath)。 */
export type GraphData = { nodes: EntryRef[]; edges: [string, string][] };

/** 会話の 1 ターン（多輪・記憶のためフロントが履歴を組み立てて渡す）。 */
export type ChatTurn = {
  role: "user" | "assistant";
  content: string;
};

export type WorkshopToolEvent = { name: string; args: string; summary?: string };

export type WorkshopMessage =
  | { role: "user"; text: string; thinking?: never; tools?: never }
  | { role: "ai"; text: string; thinking?: string; tools?: WorkshopToolEvent[] };

export type WorkshopConversationSummary = {
  id: number;
  title: string;
  updatedAt: string;
};

export type WorkshopConversationPage = {
  items: WorkshopConversationSummary[];
  hasMore: boolean;
};

export type WorkshopConversation = {
  id: number;
  title: string;
  sourceIds: string[];
  messages: WorkshopMessage[];
  createdAt: string;
  updatedAt: string;
};

/** 対話の進捗イベント（Rust StreamProgress と一致）。 */
export type ChatPhase =
  | { phase: "thinking"; delta: string }
  | { phase: "loadingModel" }
  | { phase: "narration"; delta: string }
  | { phase: "toolCall"; name: string; args: string }
  | { phase: "toolResult"; name: string; summary: string }
  | { phase: "confirmRequest"; id: number; summary: string };

export type OllamaModel = {
  name: string;
  thinking: boolean;
  tools: boolean;
};

/** 技能の由来（Rust SkillSource と一致）。 */
export type SkillSource = "kb" | "user";

/** 発見済み Agent Skill（Rust plugin::Skill と一致）。 */
export type Skill = {
  name: string;
  description: string;
  /** frontmatter 剥離済みの本文。 */
  body: string;
  /** SKILL.md への絶対パス（表示用）。 */
  location: string;
  source: SkillSource;
  /** `scripts/` サブディレクトリの有無（本バージョンは実行しない、UI 注記用）。 */
  hasScripts: boolean;
};

/** AI プロバイダ（Rust Provider と一致）。ローカル端点のみ。 */
export type AiProvider = "ollama" | "llamaApp";

/** AI 設定（Rust AiSettings と一致）。URL は空欄なら後端が provider 既定へ解決する。 */
export type AiSettings = {
  provider: AiProvider;
  model: string;
  ollamaUrl: string;
  llamaAppUrl: string;
  braveApiKey: string;
};

export const DEFAULT_AI_SETTINGS: AiSettings = {
  provider: "ollama",
  model: "",
  ollamaUrl: "",
  llamaAppUrl: "",
  braveApiKey: "",
};

/** provider 既定の URL（空欄時のプレースホルダ表示用。実際の解決は後端）。 */
export const DEFAULT_PROVIDER_URL: Record<AiProvider, string> = {
  ollama: "http://127.0.0.1:11434",
  llamaApp: "http://127.0.0.1:8080/v1",
};

// ── 域接口(接縫の契約。適配器: tauri.ts / fake.ts) ──

/** ナレッジベースの登記簿（作成・切替・削除）と条目データ（一覧・検索・読み書き）。 */
export type KbApi = {
  /** 登録済みナレッジベース一覧。後端が無ければ null。 */
  listKbs(): Promise<KbList | null>;
  /** ナレッジベースを新規作成して登録し、アクティブに切り替える。 */
  createKb(input: { name: string; description: string; path: string }): Promise<Kb>;
  /** 登録済みナレッジベースをアクティブに切り替える。 */
  setActiveKb(path: string): Promise<void>;
  /** ナレッジベースを削除する（レジストリ登録解除＋ディスク上のフォルダ削除）。不可逆。 */
  deleteKb(path: string): Promise<void>;
  /** 条目一覧（更新が新しい順）。 */
  listEntries(): Promise<EntryRef[]>;
  /** 全文検索（3 文字以上）。 */
  search(query: string): Promise<SearchHit[]>;
  /** 指定タイトルへのバックリンク。 */
  backlinks(title: string): Promise<EntryRef[]>;
  /** ダッシュボード統計。後端が無ければ null。 */
  stats(): Promise<Stats | null>;
  /** グラフデータ。 */
  graph(): Promise<GraphData>;
  /** 孤立条目。 */
  orphans(): Promise<EntryRef[]>;
  /** 条目の生 Markdown を読む。 */
  readEntry(path: string): Promise<string>;
  /** 条目を上書き保存する（frontmatter 検証付き）。 */
  saveEntry(path: string, content: string): Promise<void>;
};

/** AI プロバイダ設定とモデル探測。 */
export type AgentApi = {
  /** ローカル Ollama が応答するか。 */
  hasKey(): Promise<boolean>;
  /** ローカル Ollama に存在するモデル一覧。 */
  listOllamaModels(): Promise<OllamaModel[]>;
  /** 指定 provider + URL でモデル一覧を取る（設定画面の「検証」）。空 URL は後端が既定へ解決。 */
  listModels(provider: AiProvider, baseUrl: string): Promise<OllamaModel[]>;
  /** 保存済み AI 設定を読む（既定は Ollama）。 */
  getSettings(): Promise<AiSettings>;
  /** AI 設定を保存する。 */
  setSettings(settings: AiSettings): Promise<void>;
};

/** 発見済み Agent Skill の一覧。 */
export type PluginApi = {
  /** KB `skills/` + `~/.agents/skills/` の発見済み技能一覧（同名は KB 側が勝つ）。 */
  listSkills(): Promise<Skill[]>;
};

/** chat 1 ターン分の入力一式（結伴して旅するフィールドは個別引数ではなく一括りで渡す）。 */
export type WorkshopChatInput = {
  /** 添付素材（外部ファイルの絶対パス id）。 */
  sourceIds: string[];
  /** 会話履歴（末尾が今回の user 発話）。 */
  messages: ChatTurn[];
  model: string;
  think: boolean;
  /** 選択モデルの tools 能力。false ならツールを一切登録しない（明示発動だけが効く）。 */
  tools: boolean;
  /** この会話で発動済みの技能名（ボタン発動・モデル自動発動を問わず）。 */
  activatedSkillNames: string[];
};

/** 工坊の対話エージェントと対話履歴。 */
export type WorkshopApi = {
  /** 対話エージェントを 1 ターン回す。最終返信本文を返す。onPhase で進捗を受け取る。 */
  chat(input: WorkshopChatInput, onPhase?: (phase: ChatPhase) => void): Promise<string>;
  /** 進行中の生成を中断する（停止ボタン）。共有フラグを立てるだけで即返る。 */
  cancel(): Promise<void>;
  /** エージェントの確認要求へ応答する（許可 / 拒否）。 */
  confirm(id: number, approved: boolean): Promise<void>;
  /** アクティブ KB の対話履歴を取得する。 */
  listConversations(offset: number): Promise<WorkshopConversationPage>;
  /** アクティブ KB から保存済み対話を取得する。 */
  getConversation(id: number): Promise<WorkshopConversation>;
  /** 完了済みの対話状態をアクティブ KB へ保存する。 */
  saveConversation(input: {
    kbPath: string;
    id: number | null;
    sourceIds: string[];
    messages: WorkshopMessage[];
  }): Promise<WorkshopConversation>;
  /** 素材として添付するローカルファイルを 1 つ選ぶ。選ばなければ null。 */
  pickSourceFile(): Promise<string | null>;
};

/** 後端全体。適配器（tauri / fake）はこの形を丸ごと実装する。 */
export type Backend = {
  kb: KbApi;
  agent: AgentApi;
  plugin: PluginApi;
  workshop: WorkshopApi;
};
