# Agent モジュール分離 + 複数プロバイダ 実装計画

> 設計は [specs/2026-07-01-agent-provider-decoupling-design.md](../specs/2026-07-01-agent-provider-decoupling-design.md)。ステップは `- [ ]` で追跡。各タスク後に `bun run test` / `bun run lint` を確認。

**Goal:** `ai` を汎用ブラックボックス `agent` へ改名・分離し、業務ツールを注入、Ollama + llama.app の二プロバイダを選べる縫い目を作る。IPC 契約（`workshop_chat` の署名）と Ollama の挙動は不変。

**Tech Stack:** Rust / Tauri 2 / rig-core 0.39、TypeScript / React / Next.js（前端設定 UI）

**Baseline:** cargo test 67 passed。

---

### Task 1: モジュール改名 `ai` → `agent`

**Files:** `git mv src-tauri/src/ai src-tauri/src/agent`、`src-tauri/src/lib.rs`、`crate::ai` を参照する全箇所（`workshop/application.rs`, `workshop/infrastructure/rig_agent.rs`）

- [ ] `git mv` でディレクトリ改名。`lib.rs` の `mod ai;` → `mod agent;`、`ai::interface::*` 登録 → `agent::interface::*`。
- [ ] `crate::ai` → `crate::agent` を全置換。コマンド名 `ai_has_key` / `ai_list_ollama_models` は据え置き。
- [ ] `cargo test`：67 passed のまま（純機械改名、挙動不変）。

### Task 2: preamble を workshop へ移設

**Files:** delete `src-tauri/src/agent/agent.rs`、new `src-tauri/src/workshop/prompt.rs`、`workshop/mod.rs`、`workshop/application.rs`、`agent/mod.rs`

- [ ] `AGENT_SYSTEM` / `agent_system_with` + その `#[cfg(test)]` を `workshop/prompt.rs` へ丸ごと移す。`agent/mod.rs` から `pub mod agent;` を外す。
- [ ] `workshop/mod.rs` に `mod prompt;`。`application::chat` の `use crate::agent::agent::agent_system_with` を `super::prompt::agent_system_with` へ。
- [ ] `cargo test`：移設したテストを含め 67 passed。

### Task 3: 汎用ランナー化 + Provider 分岐

**Files:** new `src-tauri/src/agent/infrastructure/runner.rs`、`agent/domain.rs`（+`Provider`）、`agent/mod.rs`（`pub use`）、`agent/infrastructure/mod.rs`、slim `workshop/infrastructure/rig_agent.rs`、`workshop/application.rs`

- [ ] `agent/domain.rs` に `Provider { Ollama, LlamaApp }`（Serialize/Deserialize、camelCase、Default=Ollama）+ 既定値テスト。
- [ ] 現行 `rig_agent::run` のループ本体を `runner.rs` へ移し、`drive<M>`（ジェネリック）+ `pub async fn run(provider, base_url: Option<&str>, model, think, system, tools, messages, cancel, tx)` に再構成。where 句はコンパイラで確定。
  - Ollama arm：`ollama::Client::new(Nothing)` + `additional_params({num_ctx, think})`（現行と**バイト等価**）。
  - LlamaApp arm：`openai::Client::builder().api_key("expertbase-local").base_url(base_url).build()`（key 不要のローカル、dummy）。`base_url` が None なら `AiError::Other`。`// ponytail:` で OpenAI 互換前提を明示。
- [ ] `agent/mod.rs`：`pub use infrastructure::runner::run;`、`pub use domain::Provider;`。
- [ ] `workshop/infrastructure/rig_agent.rs`：4 ツール構築 + `agent::run(...)` 委譲だけに縮小。`application::chat` は `provider` / `base_url` を明示引数で受けて素通し。
- [ ] `run` の「LlamaApp + base_url None → エラー」を単体テスト。
- [ ] `cargo test`：green（Ollama 挙動不変）。

### Task 4: AiSettings + 永続化 + コマンド

**Files:** `agent/domain.rs`（+`AiSettings`）、new `agent/infrastructure/settings_store.rs`、`agent/interface.rs`、`lib.rs`

- [ ] `AiSettings { provider: Provider, model: String, llama_app_url: String }`（Serialize/Deserialize、camelCase、Default）。
- [ ] `settings_store`：`~/.expertBase/ai.toml` の `load(home)`（欠落時 Default）/ `save(home, &AiSettings)`。kb `config_store` の toml + `create_dir_all` 方式を踏襲。
- [ ] 単体テスト：欠落時 Default、save→load 往復。
- [ ] `interface`：`ai_get_settings` / `ai_set_settings`。`lib.rs` の `generate_handler!` に登録。
- [ ] `cargo test`：green。

### Task 5: provider を workshop_chat へ配線

**Files:** `workshop/interface.rs`、`workshop/application.rs`

- [ ] `workshop_chat`：`agent::settings_store::load(home)` で provider + llama_app_url を読み、`application::chat` へ渡す。IPC 署名（前端が送る引数）は不変。model は IPC の値を使う。
- [ ] `cargo test`：green。

### Task 6: 前端 — 設定 UI と provider ハンドリング

**Files:** `frontend/src/shared/api/tauri/client.ts`、`frontend/src/widgets/app-shell/settings-dialog.tsx`、`frontend/src/features/workshop/ui/workshop-view.tsx`、`frontend/src/shared/i18n/dictionaries.ts`（該当キー）

- [ ] client：`Provider` / `AiSettings` 型、`aiGetSettings` / `aiSetSettings` バインディング。
- [ ] SettingsDialog：AI 節（provider 選択 = Ollama / llama.app、既定モデル入力、llama.app の URL 入力）。load/save。
- [ ] workshop-view：設定を読み、`provider === "llamaApp"` のときは Ollama モデル一覧を使わず `settings.model` を使う。Ollama は現行どおり。
- [ ] i18n：新ラベル（3 言語）。
- [ ] `bun run lint` + `bun run build`：green。

### Task 7: 検証・コミット・PR

- [ ] `bun run test`（cargo）+ `bun run lint` + `bun run --cwd frontend build` を全て green で確認。
- [ ] タスクごと、または論理単位でコミット。
- [ ] `gh pr create` で PR（Issue #13 を参照）。
