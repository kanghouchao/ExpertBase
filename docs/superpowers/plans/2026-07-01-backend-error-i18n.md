# 後端エラー国際化 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rust 後端の全 IPC コマンドを `Result<T, String>` から `Result<T, AppError>`（`{code, params}`）へ移行し、前端の既存 i18n（`t(key, params)`）で三言語表示できるようにする。

**Architecture:** 新規 `src-tauri/src/error.rs` に `AppError` を定義。kb → agent → workshop の順に、各モジュールの domain/application/infrastructure/interface 層を通しで `Result<T, AppError>` に置き換える（Rust の型は `?` で連鎖するので、1 モジュールは 1 コミットで通しでコンパイルが通る単位にする）。`agent::AiError` は削除して `AppError` に統一する。前端は `AppError` 型 + `translateError` ヘルパーを追加し、コンポーネント内 catch はその場で翻訳、store 外 catch（`workshop-run.ts`）は生値を保持して描画側で翻訳する。

**Tech Stack:** Rust / serde（`AppError` は `#[derive(Serialize)]`）、TypeScript / 既存 `shared/i18n`

## Global Constraints

- `code` は前端辞書の完全な key（例 `"err.kb.nameRequired"`）。前端でのプレフィックス合成はしない。
- 底層ライブラリの素通しエラー（`.map_err(|e| e.to_string())`)は個別コード化せず、全て `AppError::generic(e)`（code = `"err.generic"`, params = `{detail}`）に置き換える。
- `extract/infrastructure/doc.rs` と `workshop/infrastructure/tools.rs` の `read_blocking`/`search_blocking`/`write_blocking` 内のツールループ文字列（LLM 向け）は対象外。**触らない**。
- `frontend/src/app/layout.tsx` の `catch(_){}`（テーマ初期化）、`entities/knowledge-base/model/store.ts` の `refreshKbs()` の `error` フィールド（現在どこからも参照されていない）は対象外。**触らない**。
- 各モジュールタスクの最後に必ず `cargo test --manifest-path src-tauri/Cargo.toml`（または `bun run test`）を通すこと。
- 参照: 設計は [specs/2026-07-01-backend-error-i18n-design.md](../specs/2026-07-01-backend-error-i18n-design.md)。エラーコードカタログ（19 個の `err.*` key と対応する params）はこの設計ドキュメントの表を正とする。

---

### Task 1: `AppError` 型を追加する

**Files:**
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/lib.rs`（`mod error;` を追加）

**Interfaces:**
- Produces: `pub struct AppError { pub code: String, pub params: Option<BTreeMap<String,String>> }`、`AppError::code(code: &str) -> Self`、`AppError::param(code: &str, key: &str, value: impl std::fmt::Display) -> Self`、`AppError::params(code: &str, pairs: impl IntoIterator<Item = (&'static str, String)>) -> Self`、`AppError::generic(e: impl std::fmt::Display) -> Self`（code は固定で `"err.generic"`、params は `{"detail": e.to_string()}`）。以降の全タスクはこの 4 関数と `pub` フィールド `code`/`params` を使う。

- [ ] **Step 1: 失敗するテストを書く**

`src-tauri/src/error.rs` を新規作成し、まずテストモジュールだけ書く:

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
  pub code: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub params: Option<BTreeMap<String, String>>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn code_has_no_params() {
    let e = AppError::code("err.agent.cancelled");
    assert_eq!(e.code, "err.agent.cancelled");
    assert_eq!(e.params, None);
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v, serde_json::json!({ "code": "err.agent.cancelled" }));
  }

  #[test]
  fn param_carries_single_key() {
    let e = AppError::param("err.agent.network", "detail", "timeout");
    assert_eq!(e.code, "err.agent.network");
    assert_eq!(e.params.unwrap().get("detail"), Some(&"timeout".to_string()));
  }

  #[test]
  fn params_carries_multiple_keys() {
    let e = AppError::params(
      "err.agent.modelListFailed",
      [("status", "500".to_string()), ("detail", "boom".to_string())],
    );
    let params = e.params.unwrap();
    assert_eq!(params.get("status"), Some(&"500".to_string()));
    assert_eq!(params.get("detail"), Some(&"boom".to_string()));
  }

  #[test]
  fn generic_wraps_any_displayable_error_as_detail() {
    let e = AppError::generic("boom");
    assert_eq!(e.code, "err.generic");
    assert_eq!(e.params.unwrap().get("detail"), Some(&"boom".to_string()));
  }
}
```

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml error::tests`
Expected: FAIL（`AppError::code` 等の関数が未定義でコンパイルエラー）

- [ ] **Step 3: 最小実装を書く**

`error.rs` の `struct AppError` 定義の直後に追記:

```rust
impl AppError {
  pub fn code(code: &str) -> Self {
    Self { code: code.to_string(), params: None }
  }

  pub fn param(code: &str, key: &str, value: impl std::fmt::Display) -> Self {
    let mut params = BTreeMap::new();
    params.insert(key.to_string(), value.to_string());
    Self { code: code.to_string(), params: Some(params) }
  }

  pub fn params(code: &str, pairs: impl IntoIterator<Item = (&'static str, String)>) -> Self {
    let params: BTreeMap<String, String> =
      pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    Self { code: code.to_string(), params: Some(params) }
  }

  pub fn generic(e: impl std::fmt::Display) -> Self {
    Self::param("err.generic", "detail", e)
  }
}
```

- [ ] **Step 4: テストを実行して成功を確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml error::tests`
Expected: PASS（4 tests）

- [ ] **Step 5: `lib.rs` に登録**

`src-tauri/src/lib.rs` の先頭:

```rust
mod agent;
mod error;
mod extract;
mod kb;
mod workshop;
```

- [ ] **Step 6: コミット**

```bash
git add src-tauri/src/error.rs src-tauri/src/lib.rs
git commit -m "feat(error): add shared AppError { code, params } IPC error type"
```

---

### Task 2: `kb` モジュールを `AppError` へ移行する

**Files:**
- Modify: `src-tauri/src/kb/domain/registry.rs`
- Modify: `src-tauri/src/kb/domain/entry.rs`
- Modify: `src-tauri/src/kb/application.rs`
- Modify: `src-tauri/src/kb/infrastructure/index.rs`
- Modify: `src-tauri/src/kb/infrastructure/store.rs`
- Modify: `src-tauri/src/kb/infrastructure/config_store.rs`
- Modify: `src-tauri/src/kb/interface.rs`

**Interfaces:**
- Consumes: `crate::error::AppError`（Task 1）。
- Produces: kb の全 public 関数（`create_kb`/`set_active`/`delete_kb`/`save_entry`/`read_entry`/`active_kb_root`/`open_active`/`parse_entry`/`split_frontmatter`/`checked_kb_markdown_path`/`load_registry`/`save_registry`/`write_kb_config`/`ensure_schema`/`open_index`/`rebuild`/`upsert_entry`/`list_entries`/`search`/`backlinks`/`stats`/`graph`/`orphans`/`delete_entry`/`write_entry`/`serialize_entry` 等)は全て `Result<_, AppError>` を返す。以降のタスク（workshop）はこれに依存する。

置換ルール（この機械的な変換を kb 配下の該当ファイル全てに適用する）:

1. 全ての関数シグネチャで `Result<T, String>` → `Result<T, AppError>`。
2. `.map_err(|e| e.to_string())` → `.map_err(AppError::generic)`。
3. 下記の手書きエラーは対応する `AppError::code(...)` に置き換える（該当ファイル冒頭に `use crate::error::AppError;` を追加）:

| ファイル:行 | 旧 | 新 |
|---|---|---|
| `kb/application.rs:16` | `Err("没有激活的知识库")?` (`.ok_or(...)`)| `.ok_or_else(|| AppError::code("err.kb.noActiveKb"))?` |
| `kb/application.rs:36` | `Err("知识库名称不能为空".into())` | `Err(AppError::code("err.kb.nameRequired"))` |
| `kb/application.rs:40` | `Err("存储位置不能为空".into())` | `Err(AppError::code("err.kb.pathRequired"))` |
| `kb/application.rs:47` | `Err("该位置已注册为知识库".into())` | `Err(AppError::code("err.kb.pathAlreadyRegistered"))` |
| `kb/application.rs:51` | `Err("该目录已经包含...".into())` | `Err(AppError::code("err.kb.pathAlreadyHasKb"))` |
| `kb/application.rs:75` | `Err("未找到该知识库".into())` | `Err(AppError::code("err.kb.notFound"))` |
| `kb/application.rs:89` | `.ok_or("未找到该知识库")?` | `.ok_or_else(|| AppError::code("err.kb.notFound"))?` |
| `kb/application.rs:94` | `Err("知识库元数据不是目录".into())` | `Err(AppError::code("err.kb.metaNotDirectory"))` |
| `kb/domain/entry.rs:46` | `.ok_or_else(\|\| "frontmatter が見つかりません".to_string())?` | `.ok_or_else(|| AppError::code("err.kb.entryFrontmatterMissing"))?` |
| `kb/domain/entry.rs:50` | `.ok_or_else(\|\| "frontmatter の終端が見つかりません".to_string())?` | `.ok_or_else(|| AppError::code("err.kb.entryFrontmatterUnterminated"))?` |
| `kb/domain/registry.rs:44` | `Err("知识库路径必须是相对路径".into())` | `Err(AppError::code("err.kb.pathMustBeRelative"))` |
| `kb/domain/registry.rs:55` | `Err("知识库路径不在允许的 Markdown 目录内".into())` | `Err(AppError::code("err.kb.pathOutsideAllowedDir"))` |
| `kb/infrastructure/index.rs:116` | `Err(format!("同名の条目が既に存在します: {}", path))` | `Err(AppError::param("err.kb.duplicateEntryName", "path", path))` |

**Note:** `entry.rs` の `serde_yaml::from_str(&yaml).map_err(\|e\| e.to_string())?`（frontmatter の YAML 構文エラー）は手書き文案ではないので規則 2 で `AppError::generic` に。

- [ ] **Step 1: 既存テストの文言アサーションをコード比較へ書き換える（先に失敗させる）**

`kb/infrastructure/index.rs` のテストモジュール内、`assert!(err.contains("同名の条目"))` を探して:

```rust
    let err = upsert_entry(&conn, "entries/dup.md", &second).unwrap_err();
    assert_eq!(err.code, "err.kb.duplicateEntryName");
    assert_eq!(err.params.unwrap().get("path"), Some(&"entries/green.md".to_string()));
```

（元のテストが `.contains` で見ていた変数名・セットアップはそのまま残し、最後のアサーションだけ差し替える。）

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::`
Expected: FAIL（コンパイルエラー。`AppError` 型はまだ kb 側で使われていない/ `err.code` フィールドが `String` の `.code` として存在しない）

- [ ] **Step 3: 7 ファイルへ上記の機械的置換ルールを適用する**

各ファイル冒頭に `use crate::error::AppError;` を追加し、シグネチャと `Err(...)`/`.map_err(...)` を上の表・規則どおりに書き換える。`kb/interface.rs` は `Result<T, String>` を返す 13 関数全てのシグネチャだけ変更（本体の `.map_err(|e| e.to_string())?`（home_dir 取得部分）も規則 2 で置換）。

- [ ] **Step 4: テストを実行して成功を確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::`
Expected: PASS（全 kb テスト green。他の既存テストは `.is_err()` だけを見ているため型変更の影響を受けない）

- [ ] **Step 5: コミット**

```bash
git add src-tauri/src/kb
git commit -m "refactor(kb): migrate Result<T, String> to Result<T, AppError>"
```

---

### Task 3: `agent` モジュールを `AppError` へ移行し `AiError` を削除する

**Files:**
- Modify: `src-tauri/src/agent/domain.rs`（`AiError` enum + `Display` impl + そのテストを削除）
- Modify: `src-tauri/src/agent/mod.rs`（`pub use domain::{..., AiError, ...}` から `AiError` を外す）
- Modify: `src-tauri/src/agent/infrastructure/ollama.rs`
- Modify: `src-tauri/src/agent/infrastructure/openai_compat.rs`
- Modify: `src-tauri/src/agent/infrastructure/runner.rs`
- Modify: `src-tauri/src/agent/infrastructure/settings_store.rs`
- Modify: `src-tauri/src/agent/interface.rs`

**Interfaces:**
- Consumes: `crate::error::AppError`（Task 1）。
- Produces: `agent::run(...)` は `Result<String, AppError>`。`agent::settings_store::load`/`save` は `Result<_, AppError>`。`agent::infrastructure::{ollama, openai_compat}::list_models` は `Result<Vec<OllamaModel>, AppError>`。workshop（Task 4）はこれらに依存する。

- [ ] **Step 1: `agent/domain.rs` から `AiError` を削除する（先に壊す）**

`agent/domain.rs` の `AiError` enum・`impl std::fmt::Display for AiError`・`#[test] fn ai_error_displays_messages` をまるごと削除する。

- [ ] **Step 2: 依存箇所のコンパイルが壊れることを確認**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: FAIL（`ollama.rs`/`openai_compat.rs`/`runner.rs`/`agent/mod.rs` が `AiError` を参照できず compile error）

- [ ] **Step 3: `agent/mod.rs` の再エクスポートを更新**

```rust
pub use domain::{resolve_base_url, ChatTurn, Provider, StreamProgress};
```

（`AiError` を削除。他は変更なし。）

- [ ] **Step 4: `ollama.rs` を書き換える**

`use crate::agent::AiError;` → `use crate::error::AppError;` に変更。関数シグネチャの `Result<_, AiError>` → `Result<_, AppError>`。エラー構築箇所:

```rust
pub fn list_models(base_url: &str) -> Result<Vec<OllamaModel>, AppError> {
  let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(3))
    .build()
    .map_err(|e| AppError::param("err.agent.network", "detail", e))?;
  let resp = client
    .get(format!("{base_url}/api/tags"))
    .send()
    .map_err(|e| AppError::param("err.agent.network", "detail", e))?;
  let status = resp.status();
  let text = resp.text().map_err(|e| AppError::param("err.agent.network", "detail", e))?;
  if status.as_u16() != 200 {
    return Err(AppError::params(
      "err.agent.modelListFailed",
      [("status", status.to_string()), ("detail", text)],
    ));
  }
  let mut models = parse_models_response(&text)?;
  for model in &mut models {
    if let Ok(show) = client
      .post(format!("{base_url}/api/show"))
      .json(&serde_json::json!({ "model": model.name }))
      .send()
      .and_then(|r| r.text())
    {
      model.thinking = show_supports_thinking(&show);
      model.tools = show_supports_tools(&show);
    }
  }
  Ok(models)
}
```

`fn parse_models_response(body: &str) -> Result<Vec<OllamaModel>, AppError>` の中身も `.map_err(|e| AiError::Other(e.to_string()))?` → `.map_err(AppError::generic)?`（これは JSON パース失敗＝底層ライブラリ素通しなので `agent.modelListFailed` ではなく `err.generic`）。

- [ ] **Step 5: `openai_compat.rs` を同様に書き換える**

同じパターン。`Err(AiError::Other(format!("模型列表读取失败({status}): {text}")))` → `Err(AppError::params("err.agent.modelListFailed", [("status", status.to_string()), ("detail", text)]))`。`parse_models` 内の JSON パース失敗は `AppError::generic`。

- [ ] **Step 6: `runner.rs` を書き換える**

`use crate::agent::{resolve_base_url, AiError, ChatTurn, Provider, StreamProgress};` → `AiError` を `crate::error::AppError` に差し替え（import 行を `use crate::agent::{resolve_base_url, ChatTurn, Provider, StreamProgress}; use crate::error::AppError;` に）。

```rust
pub async fn run(..., tx: &UnboundedSender<StreamProgress>) -> Result<String, AppError> {
  let Some((last, rest)) = messages.split_last() else {
    return Err(AppError::code("err.agent.emptyConversation"));
  };
  ...
  Provider::Ollama => {
    let client = ollama::Client::builder()
      .api_key(OllamaApiKey::default())
      .base_url(&base_url)
      .build()
      .map_err(|e| AppError::param("err.agent.network", "detail", e))?;
    ...
  }
  Provider::LlamaApp => {
    let client = openai::Client::builder()
      .api_key("expertbase-local")
      .base_url(&base_url)
      .build()
      .map_err(|e| AppError::param("err.agent.network", "detail", e))?;
    ...
  }
}

async fn drive<M>(...) -> Result<String, AppError>
where ... {
  ...
  if cancel.load(Ordering::Relaxed) {
    return Err(AppError::code("err.agent.cancelled"));
  }
  ...
  Err(e) => return Err(AppError::generic(e)),
  ...
}
```

（`Err(e) => return Err(AppError::generic(e))` — stream item のエラーは rig 内部の任意エラーなので手書き文案ではなく `err.generic`。）

テスト `run_errors_on_empty_messages` の `assert!(matches!(err, AiError::Other(_)))` を `assert_eq!(err.code, "err.agent.emptyConversation")` に変更。

- [ ] **Step 7: `settings_store.rs` を書き換える**

`Result<AiSettings, String>` → `Result<AiSettings, AppError>`。全 `.map_err(|e| e.to_string())` → `.map_err(AppError::generic)`。`use crate::error::AppError;` を追加。

- [ ] **Step 8: `agent/interface.rs` を書き換える**

5 コマンド全てのシグネチャを `Result<T, AppError>` に。`load_settings` ヘルパーも同様。`.map_err(|e| e.to_string())`（`home_dir()` 用）→ `.map_err(AppError::generic)`。`use crate::error::AppError;` を追加。

- [ ] **Step 9: テストを実行して成功を確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml agent::`
Expected: PASS

- [ ] **Step 10: コミット**

```bash
git add src-tauri/src/agent
git commit -m "refactor(agent): replace AiError with AppError, migrate to code+params"
```

---

### Task 4: `workshop` モジュールを `AppError` へ移行する

**Files:**
- Modify: `src-tauri/src/workshop/application.rs`
- Modify: `src-tauri/src/workshop/interface.rs`
- Modify: `src-tauri/src/workshop/infrastructure/history.rs`（`Result<T, String>` を返す関数のシグネチャのみ機械的に変更、手書き文案なし）

**Interfaces:**
- Consumes: `crate::error::AppError`（Task 1）、`crate::agent::run` が `Result<String, AppError>` を返すこと（Task 3）、kb の全関数が `Result<_, AppError>` を返すこと（Task 2）。
- Produces: `workshop::application::{save_active_conversation, save_conversation, get_conversation, list_conversations, chat, confirm}` は全て `Result<_, AppError>`。全 4 個の `workshop_*` コマンドは `Result<T, AppError>`。

- [ ] **Step 1: 既存テストの文言アサーションを先に書き換える（失敗させる）**

`workshop/application.rs` のテストモジュール内:

```rust
    let error = ensure_active_kb(&second_path, first_path.to_str().unwrap()).unwrap_err();
    assert_eq!(error.code, "err.workshop.kbSwitchedDuringSave");
```

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml workshop::application`
Expected: FAIL（コンパイルエラー、`error.code` が `String` に生えていない）

- [ ] **Step 3: `workshop/application.rs` を書き換える**

`use crate::agent::{AiError, ChatTurn, Provider, StreamProgress};` → `use crate::agent::{ChatTurn, Provider, StreamProgress}; use crate::error::AppError;`

```rust
fn ensure_active_kb(active_root: &Path, expected_kb_path: &str) -> Result<(), AppError> {
  if active_root != Path::new(expected_kb_path) {
    return Err(AppError::code("err.workshop.kbSwitchedDuringSave"));
  }
  Ok(())
}
```

以降の `save_active_conversation`/`save_conversation`/`get_conversation`/`list_conversations`/`confirm` は戻り値の `String` を `AppError` に置換するだけ（本文の `?` 連鎖は変更不要）。`pub async fn chat(...) -> Result<String, AppError>`（既に `AiError` → `AppError` は Task 3 で決着済みなのでここは戻り値の型を書き換えるだけ）。

- [ ] **Step 4: `workshop/infrastructure/history.rs` の `Result<T, String>` 関数シグネチャを機械的に `Result<T, AppError>` に置換し、内部の `.map_err(|e| e.to_string())` を `.map_err(AppError::generic)` に置換する**

（ファイル内に手書きの人間可読文案は無い。`use crate::error::AppError;` を追加。）

- [ ] **Step 5: `workshop/interface.rs` を書き換える**

4 コマンド（`workshop_save_conversation`/`workshop_get_conversation`/`workshop_list_conversations`/`workshop_chat`）の戻り値を `Result<T, AppError>` に。`use crate::error::AppError;` を追加。全ての `.map_err(|e| e.to_string())` を `.map_err(AppError::generic)` に。`workshop_chat` 内のハードコード:

```rust
        } else {
          return Err(AppError::param("err.workshop.sourceMustBeAbsolute", "id", id));
        }
```

`workshop_chat` 末尾の `agent.await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())` は `agent.await.map_err(AppError::generic)?` （spawn の JoinError は素通し）。第二の `?` は不要になる（`application::chat` が既に `Result<String, AppError>` を返すので、そのまま返せばよい）:

```rust
  agent.await.map_err(AppError::generic)?
```

- [ ] **Step 6: テストを実行して成功を確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml workshop::`
Expected: PASS

- [ ] **Step 7: 全体テストを実行**

Run: `bun run test`
Expected: PASS（全モジュール green。カウントは元の 67 前後から `AppError` の 4 テスト増）

- [ ] **Step 8: コミット**

```bash
git add src-tauri/src/workshop
git commit -m "refactor(workshop): migrate Result<T, String> to Result<T, AppError>"
```

---

### Task 5: `src-tauri/AGENTS.md` の IPC 規約を更新する

**Files:**
- Modify: `src-tauri/AGENTS.md`

**Interfaces:** なし（ドキュメントのみ）。

- [ ] **Step 1: 「IPC Command Practices」節を書き換える**

旧:
```
- Define commands with `#[tauri::command]` returning `Result<T, String>`, mapping internal errors with `map_err(|e| e.to_string())`.
```

新:
```
- Define commands with `#[tauri::command]` returning `Result<T, AppError>` (`src/error.rs`). Hand-authored user-facing errors use `AppError::code(...)` / `::param(...)` / `::params(...)` with a key matching the frontend i18n dictionary (e.g. `"err.kb.nameRequired"`). Passthrough errors from lower-level libraries (io/sqlite/reqwest/etc.) use `AppError::generic(e)`, which maps to a single generic frontend key (`"err.generic"`) carrying the raw detail as a param. Do not implement `Display` for domain error types that flow to the IPC boundary — that reintroduces hardcoded human text.
```

- [ ] **Step 2: コミット**

```bash
git add src-tauri/AGENTS.md
git commit -m "docs(src-tauri): document AppError IPC error convention"
```

---

### Task 6: 前端 — `AppError` 型と `translateError` ヘルパー

**Files:**
- Modify: `frontend/src/shared/api/tauri/client.ts`
- Modify: `frontend/src/shared/i18n/translate.ts`
- Test: `frontend/src/shared/i18n/translate.test.ts`

**Interfaces:**
- Produces: `export type AppError = { code: string; params?: Record<string, string> }`（`client.ts`）、`export function isAppError(e: unknown): e is AppError`、`export function translateError(t: Translate, e: unknown): string`（`translate.ts`）。Task 8 の全 catch サイトがこれを使う。

- [ ] **Step 1: 失敗するテストを書く**

`frontend/src/shared/i18n/translate.test.ts` を新規作成:

```ts
import { describe, expect, test } from "bun:test";

import { createT } from "./translate";
import { translateError } from "./translate";

describe("translateError", () => {
  const t = createT("zh");

  test("translates a coded AppError with params", () => {
    const result = translateError(t, { code: "err.generic", params: { detail: "boom" } });
    expect(result).toBe(t("err.generic", { detail: "boom" }));
  });

  test("falls back to String(e) for non-AppError values", () => {
    expect(translateError(t, "plain string")).toBe("plain string");
    expect(translateError(t, new Error("oops"))).toBe(String(new Error("oops")));
  });
});
```

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `cd frontend && bun test src/shared/i18n/translate.test.ts`
Expected: FAIL（`translateError` が存在しない）

- [ ] **Step 3: `client.ts` に型を追加**

`frontend/src/shared/api/tauri/client.ts` の先頭付近（他の型定義の隣）に追記:

```ts
/** 後端 IPC の統一エラー契約。code は前端辞書の完全な key。*/
export type AppError = { code: string; params?: Record<string, string> };
```

- [ ] **Step 4: `translate.ts` にヘルパーを追加**

```ts
import { DICT, type Lang } from "./dictionaries";
import type { AppError } from "@/shared/api/tauri/client";

// ...(既存の createT はそのまま)...

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && typeof (e as { code?: unknown }).code === "string";
}

export function translateError(t: Translate, e: unknown): string {
  if (isAppError(e)) return t(e.code, e.params);
  return String(e);
}
```

- [ ] **Step 5: テストを実行して成功を確認**

Run: `cd frontend && bun test src/shared/i18n/translate.test.ts`
Expected: PASS（2 tests）

- [ ] **Step 6: コミット**

```bash
git add frontend/src/shared/api/tauri/client.ts frontend/src/shared/i18n/translate.ts frontend/src/shared/i18n/translate.test.ts
git commit -m "feat(frontend): add AppError type and translateError helper"
```

---

### Task 7: 前端辞書に `err.*` キーを追加する（zh/en/ja）

**Files:**
- Modify: `frontend/src/shared/i18n/dictionaries.ts`

**Interfaces:**
- Consumes: なし。
- Produces: 19 個の `err.*` key が `zh`/`en`/`ja` の 3 つの `Dict` 全てに存在する。Task 8 が参照する。

- [ ] **Step 1: 3 言語のオブジェクトそれぞれに以下を追記する**

`zh` オブジェクトに追記:

```ts
  // errors
  "err.kb.noActiveKb": "没有激活的知识库",
  "err.kb.nameRequired": "知识库名称不能为空",
  "err.kb.pathRequired": "存储位置不能为空",
  "err.kb.pathAlreadyRegistered": "该位置已注册为知识库",
  "err.kb.pathAlreadyHasKb": "该目录已经包含 ExpertBase 知识库，请选择其他位置",
  "err.kb.notFound": "未找到该知识库",
  "err.kb.metaNotDirectory": "知识库元数据不是目录",
  "err.kb.entryFrontmatterMissing": "未找到 frontmatter",
  "err.kb.entryFrontmatterUnterminated": "frontmatter 未闭合",
  "err.kb.pathMustBeRelative": "知识库路径必须是相对路径",
  "err.kb.pathOutsideAllowedDir": "知识库路径不在允许的 Markdown 目录内",
  "err.kb.duplicateEntryName": "同名条目已存在：{path}",
  "err.agent.emptyConversation": "对话消息为空",
  "err.agent.modelListFailed": "模型列表读取失败（{status}）：{detail}",
  "err.agent.network": "网络错误：{detail}",
  "err.agent.cancelled": "已取消",
  "err.workshop.kbSwitchedDuringSave": "知识库已切换，已取消保存对话",
  "err.workshop.sourceMustBeAbsolute": "素材路径必须是绝对路径：{id}",
  "err.generic": "操作失败：{detail}",
```

`en` オブジェクトに追記:

```ts
  // errors
  "err.kb.noActiveKb": "No active knowledge base",
  "err.kb.nameRequired": "Knowledge base name is required",
  "err.kb.pathRequired": "Storage location is required",
  "err.kb.pathAlreadyRegistered": "This location is already registered",
  "err.kb.pathAlreadyHasKb": "This folder already has an ExpertBase knowledge base, choose another location",
  "err.kb.notFound": "Knowledge base not found",
  "err.kb.metaNotDirectory": "Knowledge base metadata is not a directory",
  "err.kb.entryFrontmatterMissing": "Frontmatter not found",
  "err.kb.entryFrontmatterUnterminated": "Frontmatter is not terminated",
  "err.kb.pathMustBeRelative": "Knowledge base path must be relative",
  "err.kb.pathOutsideAllowedDir": "Knowledge base path is outside the allowed Markdown directory",
  "err.kb.duplicateEntryName": "An entry with the same name already exists: {path}",
  "err.agent.emptyConversation": "Conversation is empty",
  "err.agent.modelListFailed": "Failed to list models ({status}): {detail}",
  "err.agent.network": "Network error: {detail}",
  "err.agent.cancelled": "Cancelled",
  "err.workshop.kbSwitchedDuringSave": "Knowledge base switched, save cancelled",
  "err.workshop.sourceMustBeAbsolute": "Source path must be absolute: {id}",
  "err.generic": "Operation failed: {detail}",
```

`ja` オブジェクトに追記:

```ts
  // errors
  "err.kb.noActiveKb": "アクティブなナレッジベースがありません",
  "err.kb.nameRequired": "ナレッジベース名を入力してください",
  "err.kb.pathRequired": "保存先を入力してください",
  "err.kb.pathAlreadyRegistered": "この場所は既にナレッジベースとして登録されています",
  "err.kb.pathAlreadyHasKb": "このフォルダには既に ExpertBase のナレッジベースがあります。別の場所を選んでください",
  "err.kb.notFound": "ナレッジベースが見つかりません",
  "err.kb.metaNotDirectory": "ナレッジベースのメタデータがディレクトリではありません",
  "err.kb.entryFrontmatterMissing": "frontmatter が見つかりません",
  "err.kb.entryFrontmatterUnterminated": "frontmatter の終端が見つかりません",
  "err.kb.pathMustBeRelative": "ナレッジベースのパスは相対パスにしてください",
  "err.kb.pathOutsideAllowedDir": "ナレッジベースのパスが許可された Markdown ディレクトリ外です",
  "err.kb.duplicateEntryName": "同名の条目が既に存在します：{path}",
  "err.agent.emptyConversation": "対話メッセージが空です",
  "err.agent.modelListFailed": "モデル一覧の取得に失敗しました（{status}）：{detail}",
  "err.agent.network": "ネットワークエラー：{detail}",
  "err.agent.cancelled": "キャンセルしました",
  "err.workshop.kbSwitchedDuringSave": "ナレッジベースが切り替わったため、保存を中止しました",
  "err.workshop.sourceMustBeAbsolute": "素材パスは絶対パスにしてください：{id}",
  "err.generic": "操作に失敗しました：{detail}",
```

- [ ] **Step 2: 既存の翻訳テストがあれば実行、無ければ lint で構文確認**

Run: `cd frontend && bun run lint`
Expected: PASS（新規キーは単純な文字列リテラルなので型エラーは出ない）

- [ ] **Step 3: コミット**

```bash
git add frontend/src/shared/i18n/dictionaries.ts
git commit -m "feat(i18n): add err.* translation keys for backend AppError codes"
```

---

### Task 8: 前端 — catch サイトを `translateError` に接続する

**Files:**
- Modify: `frontend/src/widgets/app-shell/kb-switcher.tsx`
- Modify: `frontend/src/features/onboarding/ui/onboarding.tsx`
- Modify: `frontend/src/features/workshop/ui/workshop-view.tsx`
- Modify: `frontend/src/features/workshop/model/workshop-run.ts`

**Interfaces:**
- Consumes: `translateError`（Task 6）、`err.*` キー（Task 7）。

- [ ] **Step 1: `kb-switcher.tsx` — コンポーネント内 catch をその場で翻訳**

`useI18n` を import して `t` を取得済みでなければ追加し、catch を書き換える:

```ts
    } catch (error) {
      setDeleteError(translateError(t, error));
    }
```

（`import { translateError } from "@/shared/i18n/translate";` と `const { t } = useI18n();` を追加。）

- [ ] **Step 2: `onboarding.tsx` — 同様に書き換える**

```ts
    } catch (e) {
      setError(translateError(t, e));
      setBusy(false);
    }
```

- [ ] **Step 3: `workshop-view.tsx` — 3 箇所のコンポーネント内 catch を書き換える**

`t` は既にコンポーネント内で使用中（i18n 表示があるため）。3 箇所とも:

```ts
setError(translateError(t, loadError));
setError(translateError(t, saveError));
```

（既存の `error instanceof Error ? error.message : String(error)` パターンを `translateError(t, error)` に置換。）

- [ ] **Step 4: `workshop-run.ts` — 生のエラー値を保持するよう変更（`t` が無いので翻訳しない）**

`RunStoreState.error` の型を変更:

```ts
export type RunStoreState = {
  active: RunSnapshot | null;
  error: { kbPath: string; conversationId: number; cause: unknown } | null;
};
```

2 箇所の catch（`message: error instanceof Error ? error.message : String(error)`）を:

```ts
      error: {
        kbPath: args.kbPath,
        conversationId: args.conversationId,
        cause: error,
      },
```

- [ ] **Step 5: `workshop-view.tsx` の描画箇所を更新**

`{error ?? runError?.message}` を:

```tsx
{error ?? (runError ? translateError(t, runError.cause) : undefined)}
```

- [ ] **Step 6: `workshop-run.test.ts` を確認する**

確認済み: このテストファイルは `error`/`runError`/`.message` フィールドを一切アサーションしていない（`rg -n "\.message|runError|error:" workshop-run.test.ts` が無ヒット）。Step 4 の型変更はこのテストに影響しないため、変更不要。

- [ ] **Step 7: テストを実行**

Run: `cd frontend && bun test`
Expected: PASS

- [ ] **Step 8: lint と build**

Run: `bun run lint && bun run --cwd frontend build`
Expected: PASS

- [ ] **Step 9: コミット**

```bash
git add frontend/src/widgets/app-shell/kb-switcher.tsx frontend/src/features/onboarding/ui/onboarding.tsx frontend/src/features/workshop/ui/workshop-view.tsx frontend/src/features/workshop/model/workshop-run.ts frontend/src/features/workshop/model/workshop-run.test.ts
git commit -m "feat(frontend): translate AppError at every user-facing catch site"
```

---

### Task 9: 最終検証

**Files:** なし（検証のみ）。

- [ ] **Step 1: バックエンド全テスト**

Run: `bun run test`
Expected: PASS

- [ ] **Step 2: フロントエンド lint**

Run: `bun run lint`
Expected: PASS

- [ ] **Step 3: フロントエンドビルド**

Run: `bun run --cwd frontend build`
Expected: PASS

- [ ] **Step 4: フロントエンドテスト（`bun test`、全体）**

Run: `cd frontend && bun test`
Expected: PASS

- [ ] **Step 5: issue #12 を閉じる準備ができたことを確認し、まとめコミットは不要（各タスクで既にコミット済み）**
