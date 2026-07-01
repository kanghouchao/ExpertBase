# Agent モジュール分離 + 複数プロバイダ 設計

**日付**: 2026-07-01
**ブランチ**: refactor/agent-provider-seam
**関連 Issue**: #13（`ai` を汎用ブラックボックス `agent` モジュールへ、依存注入で業務と分離）

## 目標

workshop に置かれている Rig の agent ループ（`workshop/infrastructure/rig_agent.rs`）を、業務非依存の汎用 **`agent` モジュール**へ抜き出す。業務ツールは呼び出し側（workshop）が **依存注入**し、agent はブラックボックスとしてツールループと推論だけを回す。あわせて **複数プロバイダ**（Ollama + llama.app）を選べる縫い目をここで正しく作る（近い将来 llama.app 等 API を足すため。後付けだとコードが散らかる、というユーザー要求）。

## 核心的な洞察（rig-core 0.39 をソース確認）

- rig は Ollama / OpenAI / Anthropic / llamafile など **全プロバイダを同梱**。プロバイダごとに集成コードを書く話ではない。
- **ループはプロバイダ非依存**。`stream_chat().multi_turn().await` が産む `MultiTurnStreamItem` の消費は、どのプロバイダでも同じ。プロバイダ差は `Agent<M>` の **構築時だけ**。
- `openai::Client::builder().api_key(k).base_url(url).build()` を確認。**OpenAI 互換のローカルサーバ**（LM Studio / llama.cpp server / vLLM / llama.app 類）は openai client + base_url で賄える。ローカルは key 不要なので dummy key を渡す。
- 独自の Provider trait は**作らない**。rig の `CompletionModel` / `Agent<M>` がその抽象。ツール注入の口も rig の `Tool` / `ToolDyn` がそのまま担う（新トレイト不要）。

## アーキテクチャ

### レイヤの落とし所

**`agent` モジュール（旧 `ai`、ブラックボックス、業務ゼロ依存）**

| 置くもの | 内容 |
|---|---|
| `domain`: `ChatTurn` / `StreamProgress` / `AiError`（既存）+ `Provider` + `AiSettings` | プロバイダ非依存の値 |
| `infrastructure/ollama.rs` | モデル発見（`available` / `list_models`）そのまま |
| `infrastructure/runner.rs`（新, workshop から移設）| ジェネリック `drive<M>` ループ + `run(provider, …)` 分岐 |
| `infrastructure/settings_store.rs`（新）| `~/.expertBase/ai.toml` の読み書き |
| `interface` | 既存 `ai_has_key` / `ai_list_ollama_models` + 新 `ai_get_settings` / `ai_set_settings` |

**workshop モジュール（業務、ツール注入 + 自前 preamble）**

- preamble（`AGENT_SYSTEM` / `agent_system_with`）を `ai/agent.rs` から **workshop へ移す**（KB とツール名を語る＝業務の指示層）。
- `workshop/infrastructure/rig_agent.rs` は薄くなる：4 ツールを構築して `agent::run` へ委譲するだけ。ループは持たない。
- `tools.rs`（read_source / search_kb / write_entry / fetch_web）は不変。

### プロバイダの縫い目

```rust
// agent::domain
pub enum Provider { Ollama, LlamaApp }

// agent::infrastructure::runner
pub async fn run(provider, base_url: Option<&str>, model, think, system,
                 tools: Vec<Box<dyn ToolDyn>>, messages, cancel, tx) -> Result<String, AiError>;

async fn drive<M: CompletionModel + 'static>(agent: Agent<M>, prompt, history, cancel, tx)
    -> Result<String, AiError>;  // 現行の while ループを 1 本だけ。プロバイダごとに単態化。
```

`run` は provider を match して該当 client で `Agent<M>` を組み、共通の `drive` を呼ぶ。ツールは全 arm 共通で `.tools(tools)`。`num_ctx` / `think` は Ollama 固有なので Ollama arm だけで `additional_params` に載せる（llama.app arm では載せない）。

### 設定と IPC

- 永続化：`AiSettings { provider, model, llama_app_url }` を `~/.expertBase/ai.toml` へ（kb の toml 方式を踏襲、新規依存なし）。
- コマンド：`ai_get_settings` / `ai_set_settings`。
- **`workshop_chat` の IPC 署名は不変**：provider と base_url は設定から**バックエンドが読む**（provider はグローバル設定、model は会話ごとに前端が選ぶ、という住み分け）。`application::chat` は provider / base_url を明示引数で受け取り純粋に保つ。

## 確定した決定（覆し可能）

1. **モジュール改名 `ai` → `agent`**（Issue #13 の明示要求）。コマンド名 `ai_*` は IPC 契約なので据え置き（前端の無駄な変更を避ける）。
2. **独自トレイトを作らない**。rig の `Tool` / `CompletionModel` を注入・抽象の口として使う。二個目の業務モジュールがツールを注ぐ段になったら rig 遮蔽を再評価。
3. **llama.app = OpenAI 互換ローカル端点**と仮定：rig openai client + 設定の base_url + dummy key。もし独自プロトコルなら arm 本体の数行を差し替えるだけ（縫い目は不変）。`ponytail:` コメントで明示。
4. provider はグローバル設定、model は会話ごと。

## 境界（YAGNI）

- クラウドプロバイダ・API キー保管・接続表 UI は**今回入れない**（ユーザーが「ローカル無 key のみ」を選択）。
- llama.app のモデル発見はしない（ユーザーが既定モデルを直接入力）。`ai_list_ollama_models` は Ollama 専用のまま。
- 独自 agent トレイト、複数同時生成、multi-agent、RAG は入れない。

## テスト / 検証

- **単体テスト（廉価・有意）**：settings_store の欠落時デフォルト + 往復、`Provider` の既定値 + serde、preamble（移設後も既存テストが通る）、`run` の LlamaApp + base_url 未設定エラー。
- **搬移でカバーできない**：`drive` の実ストリームループは実モデルが要るため廉価な単体テストが書けない（現行 `rig_agent::run` も無テスト）。**バイト等価で移設**して挙動を保つ。
- `bun run test`（= cargo test）、前端は `bun run lint` + `bun run build`。

## 仮定 / 要検証

- llama.app が OpenAI 互換 `/v1` であること（最有力。違えば arm 差し替え）。
- `drive<M>` の where 句は rig の `StreamingChat for Agent<M,P>` 実装（`M: CompletionModel + 'static`, `M::StreamingResponse: GetTokenUsage`）に合わせてコンパイラで確定する。

## 追記（2026-07-01・2 巡目：URL 設定 + モデル検証・発見）

ユーザー要望で上記 YAGNI 境界のうち「モデル発見なし／接続 UI なし」を撤回し、以下を追加した。

- **URL は両 provider とも設定可能・既定値あり**。`AiSettings` に `ollama_url` を追加し `{ provider, model, ollama_url, llama_app_url }`。空欄は `domain::resolve_base_url(provider, raw)` が provider 既定へ解決（Ollama=`http://127.0.0.1:11434`、llama.app=`http://127.0.0.1:8080/v1`）。llama.app は llama.cpp の `llama serve`（OpenAI 互換、既定 8080）と判明。
- **モデル発見を llama.app にも**。新コマンド `ai_list_models(provider, base_url)`：Ollama は既存 `/api/tags`+`/api/show`、llama.app は OpenAI `GET {base}/models`（能力は返らないので tools=true / thinking=false 固定）。設定画面の「検証」ボタンが叩き、成功＝端点が生きている＝既定モデル入力が `datalist` で「可選可手填」になる。
- **挙動変更**：空 URL は既定へ解決するので、1 巡目で入れた「llama.app URL 未設定エラー」ガードは廃止（空＝既定という新要件と両立しない）。URL 解決は `domain::resolve_base_url` の単体テストで担保。
- **runner**：`run` の `base_url` は `Option<&str>` → 生 `&str`（空欄可）。両 arm とも `base_url` を明示指定（Ollama も `Client::builder().base_url()` で remote 可）。
- **レビュー修正（Copilot）**：`provider` に `#[serde(default)]`（旧/手編集 ai.toml の欠落耐性）、前端は設定読み込みを Ollama 可用性と分離（Ollama 未起動でも llama.app を選べる）、設定ダイアログの読み込み失敗は既定へフォールバック。
