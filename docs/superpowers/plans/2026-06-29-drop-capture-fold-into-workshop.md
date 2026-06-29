# 収集箱の廃止・工坊への畳み込み Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 収集箱（capture / inbox）を廃止し、その生きた能力（文書抽出・Web 取得）を工坊の AI ツールへ畳み込んで、フローを「素材を会話に持ち込んで即処理」へ一本化する。

**Architecture:** 工坊の Rig エージェントは外部ファイルを `read_source(id)` で読み、新たに `fetch_web(url)` で Web 本文を読む。素材は KB へ複製せず、引用文字列（外部絶対パス）だけを `entry.sources` に残す。inbox テーブル・capture コマンド・録音 UI は削除し、asr モジュールはコードをディスクに残したままコンパイル対象から外して休止する。

**Tech Stack:** Rust / Tauri 2 / Rig (`rig-core`) / rusqlite(FTS5) / `dom_smoothie`+`htmd`(Readability) / `pdf_extract`+`dotext`（doc 抽出）。フロント: Next.js 静的エクスポート / TypeScript / Tailwind v4 / FSD。

## Global Constraints

- 言語方針：コードコメント・ドキュメントは日本語。`AGENTS.md`/`CLAUDE.md` は英語。
- Rust: edition 2021、最低 1.77.2。コマンドは `Result<T, String>`、内部エラーは `map_err(|e| e.to_string())`。IPC 構造体は `#[serde(rename_all = "camelCase")]`。インデント 2 スペース、`cargo fmt` を全体に掛けない。
- DDD: interface（Tauri コマンド）は薄く保つ。新規ロジックは同ファイル `#[cfg(test)] mod tests` に単体テスト。FS テストは `tempfile`。
- フロント: 偽データを作らない（空状態 + 無効化）。Server Component 既定、ブラウザ API が要るものだけ `"use client"`。IPC は `shared/api/tauri` の型付きクライアント経由。
- TDD: 振る舞い変更は失敗するテストを先に書く/直す。フロントは単体テストランナーが無い（`bun run test` は cargo のみ）ため、検証は `bun run lint` + `bun run build`。
- 検証コマンド（リポジトリ root）：`bun run test`（= `cargo test --manifest-path src-tauri/Cargo.toml`）、`bun run lint`、`bun run build`。
- 出処方針：被引用ファイルを KB へ複製しない・`attachments/` は作らない。`entry.sources` には外部絶対パスの文字列だけを残す。
- 境界（YAGNI、持ち込まない）：録音 / 動画 / ASR / 臨時目录、Web の多階層クロール・readability トグル、`fetch_web` のドメイン許可リスト / SSRF 防御、出処リンクの健全性チェック。
- 決定事項（本計画で確定）：
  - **asr は休止**（disconnect）。inbox を kb から完全撤去し、asr の各レイヤ `mod` 宣言を外す。asr ソースはディスクに残置、`transcribe_material` コマンドは登録解除。
  - **dashboard は inbox データを撤去**。「最近録入」ウィジェット・inbox 統計タイル・採集/整理のパイプライン件数を外し、`/capture` への CTA を `/workshop` へ向ける。

---

## File Structure

### Backend（`src-tauri/src/`）

- `asr/mod.rs` — 各レイヤの `mod` 宣言を外して休止（ソースは残置）。
- `lib.rs` — `generate_handler!` から capture 系・kb inbox 系・asr `transcribe_material` を削除。
- `capture/mod.rs` — 「抽出器の置き場（doc + web）」へ縮退。`doc`/`web` の関数を再エクスポート。
- `capture/application.rs` / `capture/domain.rs` / `capture/interface.rs` — **削除**。
- `capture/infrastructure/{doc.rs,web.rs,mod.rs}` — 保持（再利用）。
- `workshop/infrastructure/tools.rs` — `ReadSource` を外部ファイル専用へ簡素化、`WriteEntry.inbox_rels`→`source_refs`、`FetchWeb` ツール追加。
- `workshop/infrastructure/rig_agent.rs` — `run` の `inbox_rels` 撤去、`FetchWeb` 登録。
- `workshop/application.rs` — `chat` の inbox フィルタ撤去、`confirm` を `source_refs`（記録のみ）へ。
- `workshop/interface.rs` — `source_ids` 検証を「絶対パスのみ」へ。
- `ai/agent.rs` — system プロンプトに `fetch_web` の説明を追加。
- `kb/infrastructure/index.rs` — `InboxItem` / `upsert_inbox` / `list_inbox` / `set_inbox_status` / `delete_inbox` / inbox スキーマ / rebuild の inbox 枝を削除。
- `kb/infrastructure/store.rs` — rebuild テストを entries のみへ。
- `kb/application.rs` — `read_inbox_material` / `delete_inbox_material`（+テスト）削除。
- `kb/interface.rs` — `kb_read_inbox_material` / `kb_delete_inbox_material` / `kb_list_inbox` 削除。
- `kb/mod.rs` — `checked_kb_markdown_path` の crate 再エクスポート削除。
- `kb/domain/material.rs` — **保持**（asr が将来再利用。`pub use` で公開のため未使用警告は出ない）。

### Frontend（`frontend/src/`）

- `features/workshop/ui/workshop-view.tsx` — inbox 依存を撤去、「+」は外部ファイル添付のみ。
- `features/capture/**` — **削除**（`index.ts` / `ui/capture-view.tsx` / `ui/material-row.tsx` / `lib/recorder.ts`）。
- `app/(app)/capture/page.tsx` — **削除**。
- `features/dashboard/ui/dashboard-view.tsx` — inbox データ撤去、CTA を `/workshop` へ。
- `features/dashboard/ui/recent-materials.tsx` — **削除**。
- `shared/config/nav.ts` — `capture` 項と型を削除。
- `widgets/app-shell/sidebar.tsx` — `NAV.slice` 境界を 5→4 へ調整。
- `shared/api/tauri/client.ts` — capture / inbox / transcribe の関数・型を削除。
- `entities/material/model/adapt.ts` — **削除**。`index.ts` から `inboxToMaterial`/`STATUS` 公開を撤去。
- `shared/i18n/dictionaries.ts` — capture / tabs / inbox 専用キーを 3 言語分削除。

---

## Task 1: asr モジュールを休止する（disconnect）

inbox を kb から撤去する前提を作る。asr の各レイヤは inbox（`index::upsert_inbox` 等）に依存しているので、コンパイル対象から外す。ソースはディスクに残置し、別 spec で再導入する。

**Files:**
- Modify: `src-tauri/src/asr/mod.rs`
- Modify: `src-tauri/src/lib.rs:47`

**Interfaces:**
- Consumes: なし。
- Produces: `crate::asr::*` はコンパイルされなくなる。`asr::interface::transcribe_material` コマンドは消える。

- [ ] **Step 1: `asr/mod.rs` の各レイヤ宣言を外す**

`src-tauri/src/asr/mod.rs` を次に置き換える：

```rust
//! 音声認識（ASR）機能 — 工坊への搬入待ちで休止中（コードはディスク上に残置）。
//! 録音 UI と inbox を廃止した時点で transcribe の呼び出し経路と inbox 契約が消えたため、
//! 各レイヤをコンパイル対象から外す。再導入は別 spec（録音 / 動画 / 臨時目录）で行う。
//! ponytail: mod 宣言だけ外す最小休止。再開時はこの 4 行のコメントを戻す。
// mod application;
// mod domain;
// mod infrastructure;
// pub mod interface;
```

- [ ] **Step 2: `lib.rs` から asr コマンド登録を外す**

`src-tauri/src/lib.rs` の `generate_handler!` 内、`workshop::interface::workshop_cancel,` の次の行を削除する：

```rust
      workshop::interface::workshop_cancel,
      asr::interface::transcribe_material   // ← この行を削除（末尾要素なので前行のカンマは残す）
```

削除後の末尾はこうなる：

```rust
      workshop::interface::workshop_chat,
      workshop::interface::workshop_cancel
    ])
```

`mod asr;`（`lib.rs:2`）はそのまま残す（空モジュールとして有効。ソースを残置していることを示す）。

- [ ] **Step 3: コンパイル + テストが通ることを確認**

Run: `bun run test`
Expected: PASS（asr 関連テストは走らなくなる。inbox 関数はまだ存在するので他は不変）。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/asr/mod.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
refactor(asr): park asr module — drop layer decls and command registration

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: capture を「抽出器の置き場」へ縮退する

capture の取り込みユースケース（inbox への書き込み）を削除し、doc/web 抽出器だけを残す。これで capture が `index::upsert_inbox` を使わなくなる。

**Files:**
- Delete: `src-tauri/src/capture/application.rs`
- Delete: `src-tauri/src/capture/domain.rs`
- Delete: `src-tauri/src/capture/interface.rs`
- Modify: `src-tauri/src/capture/mod.rs`
- Modify: `src-tauri/src/lib.rs:39-42`

**Interfaces:**
- Consumes: `capture::infrastructure::doc`, `capture::infrastructure::web`（保持）。
- Produces: `crate::capture::{extract_pdf, extract_docx, fetch_html, extract_readable}`（Task 3 / Task 4 が使う）。

- [ ] **Step 1: capture コマンド登録を `lib.rs` から外す**

`src-tauri/src/lib.rs` の `generate_handler!` から次の 4 行を削除する：

```rust
      capture::interface::capture_text,
      capture::interface::capture_file,
      capture::interface::capture_audio,
      capture::interface::capture_web,
```

- [ ] **Step 2: capture の application / domain / interface を削除**

```bash
git rm src-tauri/src/capture/application.rs src-tauri/src/capture/domain.rs src-tauri/src/capture/interface.rs
```

- [ ] **Step 3: `capture/mod.rs` を縮退版に置き換える**

`src-tauri/src/capture/mod.rs` を次に置き換える：

```rust
//! 抽出器の置き場（doc + web）。元 capture 機能は工坊へ畳み込み済み。
//! ponytail: 名前と実体がズレるが capture→extract 改名は import 波及のため見送り（後日検討）。

mod infrastructure;

// 文書テキスト抽出（PDF/Word）と Web 取得 / 本文抽出。
// workshop の read_source（外部ファイル）と fetch_web（URL）が再利用する。
pub use infrastructure::doc::{extract_docx, extract_pdf};
pub use infrastructure::web::{extract_readable, fetch_html};
```

`src-tauri/src/capture/infrastructure/mod.rs`（`pub mod doc; pub mod web;`）はそのまま。

- [ ] **Step 4: コンパイル + テストが通ることを確認**

Run: `bun run test`
Expected: PASS（capture/infrastructure の doc/web テストは残る。capture コマンドは消える）。

- [ ] **Step 5: Commit**

```bash
git add -A src-tauri/src/capture src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
refactor(capture): shrink to doc/web extractors — drop ingest use cases

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: `fetch_web` ツールを追加する

ユーザーが会話に渡した URL を AI が本文 Markdown として読めるようにする。`capture::web` の `fetch_html` + `extract_readable` を再利用（新規依存なし）。

**Files:**
- Modify: `src-tauri/src/workshop/infrastructure/tools.rs`
- Modify: `src-tauri/src/workshop/infrastructure/rig_agent.rs:25,52-56`
- Modify: `src-tauri/src/ai/agent.rs`
- Test: `src-tauri/src/workshop/infrastructure/tools.rs`（`#[cfg(test)] mod tests`）, `src-tauri/src/ai/agent.rs`（同）

**Interfaces:**
- Consumes: `crate::capture::{fetch_html, extract_readable}`（Task 2 で再エクスポート済み）。
- Produces: `pub struct FetchWeb;`（`rig_core::tool::Tool` 実装、`NAME = "fetch_web"`、引数 `{ url: String }`、出力 `String`）。

- [ ] **Step 1: 失敗するテストを書く（空 URL ガード + 抽出経路）**

`src-tauri/src/workshop/infrastructure/tools.rs` の `mod tests` 末尾（`write_entry_tool_rejects_missing_fields` の後）に追加する：

```rust
  #[tokio::test]
  async fn fetch_web_rejects_empty_url() {
    let out = FetchWeb.call(FetchArgs { url: "  ".into() }).await.unwrap();
    assert!(out.contains("needs a non-empty url"), "was: {out}");
  }

  #[tokio::test]
  async fn fetch_web_formats_extracted_body_with_title() {
    // 実ネットは叩かない。抽出器の出力整形（title を見出しに前置）だけを検証する。
    let body = super::format_web_body("緑茶の淹れ方", "湯温は70度。");
    assert!(body.starts_with("# 緑茶の淹れ方"));
    assert!(body.contains("湯温は70度。"));
    // タイトルが空なら見出しを足さない。
    assert_eq!(super::format_web_body("  ", "本文だけ"), "本文だけ");
  }
```

- [ ] **Step 2: テストが失敗（コンパイルエラー）することを確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml fetch_web`
Expected: FAIL（`FetchWeb` / `FetchArgs` / `format_web_body` 未定義）。

- [ ] **Step 3: `FetchWeb` ツールと整形ヘルパを実装**

`src-tauri/src/workshop/infrastructure/tools.rs` の冒頭 import を更新する（`use crate::capture::{extract_docx, extract_pdf};` を置換）：

```rust
use crate::capture::{extract_docx, extract_pdf, extract_readable, fetch_html};
```

`mod tests` の直前（`fn write_blocking` の後）に追加する：

```rust
/// fetch_web の引数。URL を緩く受ける。
#[derive(Deserialize)]
pub struct FetchArgs {
  #[serde(default)]
  url: String,
}

/// ユーザーが会話に渡した URL の本文を Markdown で返す読み取りツール。
/// `web::fetch_html`（HTTPS 取得）+ `web::extract_readable`（Readability→Markdown）を再利用する。
/// 単一 URL の本文抽出のみ。許可リスト / SSRF 防御は入れない（local-first・単一ユーザー前提）。
pub struct FetchWeb;

impl Tool for FetchWeb {
  const NAME: &'static str = "fetch_web";
  type Error = Infallible;
  type Args = FetchArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Fetch a web page the user gave you and return its main text as Markdown. Use it when the user shares a URL to read, summarize, or save."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "url": { "type": "string", "description": "The page URL to fetch" }
        },
        "required": ["url"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let url = args.url.trim();
    if url.is_empty() {
      return Ok("(fetch_web needs a non-empty url)".to_string());
    }
    let html = match fetch_html(url).await {
      Ok(h) => h,
      Err(e) => return Ok(format!("(fetch error: {e})")),
    };
    match extract_readable(&html, url) {
      Ok((title, markdown)) => Ok(format_web_body(&title, &markdown)),
      Err(e) => Ok(format!("(extract error: {e})")),
    }
  }
}

/// タイトルがあれば本文の先頭に `# title` を前置する（無ければ本文のみ）。
fn format_web_body(title: &str, markdown: &str) -> String {
  if title.trim().is_empty() {
    markdown.to_string()
  } else {
    format!("# {}\n\n{}", title.trim(), markdown)
  }
}
```

- [ ] **Step 4: テストが通ることを確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml fetch_web`
Expected: PASS。

- [ ] **Step 5: `rig_agent` の工具集へ登録**

> 注: 本タスクでは `ReadSource` / `WriteEntry` のシグネチャは**まだ旧形**（`ReadSource { root, sources }` / `WriteEntry { root, inbox_rels }`）。Task 4 で最終形へ変える。本 Step は `FetchWeb` を**追加するだけ**で、既存の `ReadSource` / `WriteEntry` 構築は触らない。

`src-tauri/src/workshop/infrastructure/rig_agent.rs:25` の import に `FetchWeb` を加える（既存の `ReadSource, SearchKb, WriteEntry` はそのまま）：

```rust
use super::tools::{FetchWeb, ReadSource, SearchKb, WriteEntry};
```

同ファイルの tools ベクタ（現 `:52-56`）の `WriteEntry` 行の後に 1 行だけ追加する。追加後のベクタ（`ReadSource` / `WriteEntry` は旧形のまま）：

```rust
  let tools: Vec<Box<dyn ToolDyn>> = vec![
    Box::new(ReadSource { root: root.to_path_buf(), sources: sources.to_vec() }),
    Box::new(SearchKb { root: root.to_path_buf() }),
    Box::new(WriteEntry { root: root.to_path_buf(), inbox_rels: inbox_rels.to_vec() }),
    Box::new(FetchWeb),
  ];
```

- [ ] **Step 6: system プロンプトに `fetch_web` を追記**

`src-tauri/src/ai/agent.rs` の `AGENT_SYSTEM` 定数、`search_kb(query): ...` の行の後に 1 行追加する：

```rust
- search_kb(query): Search existing entries by keyword; returns matching titles and excerpts. Use it to find related notes and avoid duplicates.
- fetch_web(url): Fetch a web page the user gave you and return its main text as Markdown. Use it when the user shares a URL to read, summarize, or save.
```

`agent_system_lists_source_ids_and_mentions_tools` テスト（`src-tauri/src/ai/agent.rs`）に `fetch_web` の言及アサートを追加する：

```rust
    assert!(s.contains("write_entry"));
    assert!(s.contains("fetch_web"));
```

- [ ] **Step 7: 全テストが通ることを確認**

Run: `bun run test`
Expected: PASS。

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/workshop/infrastructure/tools.rs src-tauri/src/workshop/infrastructure/rig_agent.rs src-tauri/src/ai/agent.rs
git commit -m "$(cat <<'EOF'
feat(workshop): add fetch_web tool — read a URL's main text as Markdown

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: read_source を外部ファイル専用へ、source を `source_refs`（記録のみ）へ

工坊の素材契約を「外部絶対パスのみ」に統一し、`confirm` から inbox 簿記（processed マーク）を撤去して引用文字列だけを `entry.sources` に残す。

**Files:**
- Modify: `src-tauri/src/workshop/infrastructure/tools.rs`（`ReadSource` / `WriteEntry` / `read_blocking` / `write_blocking` + tests）
- Modify: `src-tauri/src/workshop/infrastructure/rig_agent.rs:33-56`（`run` シグネチャ）
- Modify: `src-tauri/src/workshop/application.rs`（`chat` / `confirm` + tests）
- Modify: `src-tauri/src/workshop/interface.rs:75-90`（`source_ids` 検証）

**Interfaces:**
- Consumes: `crate::capture::{extract_pdf, extract_docx}`, `crate::workshop::application::confirm`。
- Produces:
  - `pub struct ReadSource { pub sources: Vec<String> }`
  - `pub struct WriteEntry { pub root: PathBuf, pub source_refs: Vec<String> }`
  - `fn confirm(root: &Path, conn: &Connection, title: &str, cat: &str, body: &str, source_refs: &[String]) -> Result<String, String>`
  - `async fn rig_agent::run(model: &str, think: bool, system: &str, root: &Path, sources: &[String], messages: Vec<ChatTurn>, cancel: Arc<AtomicBool>, tx: &UnboundedSender<StreamProgress>) -> Result<String, AiError>`（`inbox_rels` 引数を撤去）。

- [ ] **Step 1: 失敗するように既存テストを更新する（tools.rs）**

`src-tauri/src/workshop/infrastructure/tools.rs` の `mod tests` で：

1. `read_source_reads_inbox_material_body` テスト（`use crate::kb::material::...;` 行含む丸ごと）を削除する。
2. `read_source_rejects_unknown_id` と `read_source_reads_external_local_file` は `ReadSource { root: ..., sources: ... }` を `ReadSource { sources: ... }` に直す：

```rust
  #[tokio::test]
  async fn read_source_reads_external_local_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("外部メモ.md");
    std::fs::write(&file, "外部ファイルの内容").unwrap();
    let id = file.to_string_lossy().to_string();

    let tool = ReadSource { sources: vec![id.clone()] };
    let out = tool.call(ReadArgs { id }).await.unwrap();

    assert!(out.contains("外部ファイルの内容"));
  }

  #[tokio::test]
  async fn read_source_rejects_unknown_id() {
    let tool = ReadSource { sources: vec![] };
    let out = tool.call(ReadArgs { id: "/abs/secret.md".into() }).await.unwrap();
    assert!(out.contains("unknown source id"));
  }
```

3. `write_entry_tool_persists_and_marks_inbox` を inbox を使わない版へ置き換える：

```rust
  #[tokio::test]
  async fn write_entry_tool_persists_and_records_source_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();

    let tool = WriteEntry {
      root: root.to_path_buf(),
      source_refs: vec!["/abs/report.pdf".into()],
    };
    let out = tool
      .call(WriteArgs { title: "緑茶".into(), cat: "tea".into(), body: "湯温は [[煎茶]] で70度".into() })
      .await
      .unwrap();

    assert!(out.starts_with("Saved entry to"));
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
    // 引用文字列（外部パス）が entry.sources に文字列として記録される。
    let rel = out.trim_start_matches("Saved entry to ").trim();
    let saved = std::fs::read_to_string(root.join(rel)).unwrap();
    let entry = crate::kb::entry::parse_entry(&saved).unwrap();
    assert_eq!(entry.meta.sources, vec!["/abs/report.pdf".to_string()]);
  }
```

4. `write_entry_tool_rejects_missing_fields` の `WriteEntry { root: ..., inbox_rels: vec![] }` を `WriteEntry { root: ..., source_refs: vec![] }` に直す。

- [ ] **Step 2: テストが失敗（コンパイルエラー）することを確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p expert_base_lib workshop::infrastructure::tools`
Expected: FAIL（`ReadSource`/`WriteEntry` のフィールド不一致）。

- [ ] **Step 3: `ReadSource` / `read_blocking` を外部専用へ実装**

`src-tauri/src/workshop/infrastructure/tools.rs`：

`ReadSource` 構造体（現 `:45-48`）を置き換える：

```rust
/// 添付素材を id で読む読み取りツール（外部絶対パスのローカルファイルのみ）。
/// sources は許可された素材 id の集合＝モデルが任意のパスを読むのを防ぐ。
/// 読み取りのみ・KB へ落とさない。
pub struct ReadSource {
  pub sources: Vec<String>,
}
```

`ReadSource::call`（現 `:72-79`）を置き換える：

```rust
  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let sources = self.sources.clone();
    let out = tokio::task::spawn_blocking(move || read_blocking(&sources, &args.id))
      .await
      .unwrap_or_else(|e| format!("(read task failed: {e})"));
    Ok(out)
  }
```

`read_blocking`（現 `:160-197`）を置き換える：

```rust
/// 素材読み取り（ブロッキング）。id を許可集合で検証してから、拡張子で抽出器を選ぶ。
/// source は外部絶対パスのみ（pdf/docx は抽出、その他はテキスト読み）。
/// エラーは全てモデル向け文字列で返す（ループ継続）。読み取りのみ・KB へ落とさない。
fn read_blocking(sources: &[String], id: &str) -> String {
  let id = id.trim();
  if id.is_empty() {
    return "(read_source needs a non-empty id)".to_string();
  }
  // 許可された素材だけ読む（モデルが任意パスを読むのを防ぐ）。
  if !sources.iter().any(|s| s == id) {
    return format!("(unknown source id: {id})");
  }
  let path = Path::new(id);
  let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
  let text = match ext.as_str() {
    "pdf" => extract_pdf(path),
    "docx" => extract_docx(path),
    _ => std::fs::read_to_string(path).map_err(|e| e.to_string()),
  }
  .map_err(|e| format!("read error: {e}"));
  match text {
    Ok(body) if body.trim().is_empty() => format!("(source {id} is empty)"),
    Ok(body) => body,
    Err(e) => format!("({e})"),
  }
}
```

- [ ] **Step 4: `WriteEntry` / `write_blocking` を `source_refs` へ実装**

`WriteEntry` 構造体（現 `:118-122`）を置き換える：

```rust
/// 新しい条目を KB へ書き込むツール（application::confirm へ委譲）。
/// source_refs は添付素材の引用文字列（外部絶対パス）＝そのまま entry.sources に記録する。
pub struct WriteEntry {
  pub root: PathBuf,
  pub source_refs: Vec<String>,
}
```

`WriteEntry::call`（現 `:148-155`）を置き換える：

```rust
  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let root = self.root.clone();
    let source_refs = self.source_refs.clone();
    let out = tokio::task::spawn_blocking(move || write_blocking(&root, &source_refs, args))
      .await
      .unwrap_or_else(|e| format!("(write task failed: {e})"));
    Ok(out)
  }
```

`write_blocking`（現 `:225-239`）を置き換える：

```rust
/// 条目書き込み（ブロッキング）。title/body を検証 → confirm で確定する。
fn write_blocking(root: &Path, source_refs: &[String], args: WriteArgs) -> String {
  let title = args.title.trim();
  let body = args.body.trim();
  if title.is_empty() || body.is_empty() {
    return "(write_entry needs a non-empty title and body)".to_string();
  }
  let conn = match index::open_index(root) {
    Ok(c) => c,
    Err(e) => return format!("(index error: {e})"),
  };
  match crate::workshop::application::confirm(root, &conn, title, args.cat.trim(), body, source_refs) {
    Ok(rel) => format!("Saved entry to {rel}"),
    Err(e) => format!("(write error: {e})"),
  }
}
```

- [ ] **Step 5: tools のテストが通ることを確認**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p expert_base_lib workshop::infrastructure::tools`
Expected: PASS。

- [ ] **Step 6: `confirm` の inbox 簿記を撤去する（application.rs）**

`src-tauri/src/workshop/application.rs` の `confirm`（現 `:41-69`）を置き換える：

```rust
/// 承認された内容を `entries/` に確定し、インデックスを更新する。
/// write_entry ツール（infra）経由で呼ばれる（書き込みの実体）。複数素材でも同じ経路を通る。
/// source_refs は添付素材の引用文字列（外部絶対パス）。KB へは複製せず文字列だけ残す。
pub fn confirm(
  root: &Path,
  conn: &Connection,
  title: &str,
  cat: &str,
  body: &str,
  source_refs: &[String],
) -> Result<String, String> {
  let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
  let entry = Entry {
    meta: EntryMeta {
      kind: "Entry".into(),
      title: title.to_string(),
      description: String::new(),
      cat: cat.to_string(),
      tags: vec![],
      sources: source_refs.to_vec(),
      created: today.clone(),
      updated: today,
    },
    body: body.to_string(),
  };
  let rel = store::write_entry(root, &entry)?;
  index::upsert_entry(conn, &rel, &entry)?;
  Ok(rel)
}
```

`chat`（現 `:24-37`）を置き換える（inbox_rels フィルタを撤去）：

```rust
#[allow(clippy::too_many_arguments)]
pub async fn chat(
  model: String,
  think: bool,
  root: PathBuf,
  sources: Vec<String>,
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: UnboundedSender<StreamProgress>,
) -> Result<String, AiError> {
  let system = agent_system_with(&sources);
  rig_agent::run(&model, think, &system, &root, &sources, messages, cancel, &tx).await
}
```

- [ ] **Step 7: `confirm` のテストを `source_refs` へ書き換える（application.rs）**

`src-tauri/src/workshop/application.rs` の `mod tests` を次の 2 テストへ置き換える（inbox を一切使わない）：

```rust
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn confirm_records_source_refs_as_entry_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = crate::kb::index::open_index(root).unwrap();

    let rel = confirm(root, &conn, "緑茶", "tea", "本文", &["/abs/report.pdf".into()]).unwrap();

    let saved = std::fs::read_to_string(root.join(&rel)).unwrap();
    let entry = crate::kb::entry::parse_entry(&saved).unwrap();
    assert_eq!(entry.meta.sources, vec!["/abs/report.pdf".to_string()]);
  }

  #[test]
  fn confirm_writes_one_entry_and_indexes_links() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();

    // 複数素材を 1 条目に合成する。引用は外部パスの文字列として残す。
    let refs = vec!["/abs/a.pdf".to_string(), "/abs/b.docx".to_string()];
    let rel = confirm(root, &conn, "緑茶", "tea", "湯温は [[煎茶]] で70度", &refs).unwrap();
    assert!(root.join(&rel).is_file());
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
    let saved = std::fs::read_to_string(root.join(&rel)).unwrap();
    let entry = crate::kb::entry::parse_entry(&saved).unwrap();
    assert_eq!(entry.meta.sources, refs);
  }
}
```

- [ ] **Step 8: `rig_agent::run` から `inbox_rels` を撤去（rig_agent.rs）**

`src-tauri/src/workshop/infrastructure/rig_agent.rs` の `run` シグネチャ（現 `:33-43`）から `inbox_rels: &[String],` 引数を削除する。tools ベクタ（Task 3 Step 5 で `FetchWeb` を追加済み）を最終形へ：

```rust
pub(crate) async fn run(
  model: &str,
  think: bool,
  system: &str,
  root: &Path,
  sources: &[String],
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: &UnboundedSender<StreamProgress>,
) -> Result<String, AiError> {
```

```rust
  let tools: Vec<Box<dyn ToolDyn>> = vec![
    Box::new(ReadSource { sources: sources.to_vec() }),
    Box::new(SearchKb { root: root.to_path_buf() }),
    Box::new(WriteEntry { root: root.to_path_buf(), source_refs: sources.to_vec() }),
    Box::new(FetchWeb),
  ];
```

- [ ] **Step 9: `workshop_chat` の source 検証を「絶対パスのみ」へ（interface.rs）**

`src-tauri/src/workshop/interface.rs` の `spawn_blocking` 検証ブロック（現 `:72-90`）を置き換える：

```rust
  // 素材 id の検証はブロッキング寄り。root + 検証済み sources を別スレッドで用意。
  // 本文はここでは読まない＝AI が read_source で個別に読む。id は外部ファイルの絶対パスのみ。
  let (root, sources) =
    tauri::async_runtime::spawn_blocking(move || -> Result<(PathBuf, Vec<String>), String> {
      let (root, _conn) = crate::kb::open_active(&home)?;
      let mut sources = Vec::with_capacity(source_ids.len());
      for id in &source_ids {
        if std::path::Path::new(id).is_absolute() {
          sources.push(id.clone());
        } else {
          return Err(format!("source must be an absolute path: {id}"));
        }
      }
      Ok((root, sources))
    })
    .await
    .map_err(|e| e.to_string())??;
```

同ファイルの `application::chat(...)` 呼び出し（現 `:94-96`）は引数順が変わらない（chat のシグネチャは Step 6 でも同一）。確認のみ：

```rust
  let agent = tauri::async_runtime::spawn(application::chat(
    model, think, root, sources, messages, cancel_flag, tx,
  ));
```

- [ ] **Step 10: 全テストが通ることを確認**

Run: `bun run test`
Expected: PASS。

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/workshop
git commit -m "$(cat <<'EOF'
refactor(workshop): sources are external paths only; record source_refs, drop inbox bookkeeping

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: kb から inbox を完全撤去する

これで inbox を参照する compiled code が無くなる（asr 休止=T1、capture 縮退=T2、workshop=T4）。

**Files:**
- Modify: `src-tauri/src/kb/interface.rs`（3 コマンド削除）
- Modify: `src-tauri/src/kb/application.rs`（2 関数 + 1 テスト + import 削除）
- Modify: `src-tauri/src/kb/infrastructure/index.rs`（`InboxItem` / 4 関数 / スキーマ / rebuild 枝 / 1 テスト削除）
- Modify: `src-tauri/src/kb/infrastructure/store.rs`（rebuild テスト更新）
- Modify: `src-tauri/src/kb/mod.rs`（`checked_kb_markdown_path` 再エクスポート削除）
- Modify: `src-tauri/src/lib.rs`（3 コマンド登録削除）

**Interfaces:**
- Consumes: なし（撤去のみ）。
- Produces: `kb::index` から inbox 系シンボルが消える。`kb::open_active` / `kb::material` / `kb::index::{open_index,upsert_entry,search,...}` は不変。

- [ ] **Step 1: rebuild テストを entries のみへ更新（store.rs）**

`src-tauri/src/kb/infrastructure/store.rs` の `rebuild_scans_entries_and_inbox_from_disk` を置き換える：

```rust
  #[test]
  fn rebuild_scans_entries_from_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("entries")).unwrap();
    fs::write(
      root.join("entries/a.md"),
      "---\ntype: Entry\ntitle: A\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n[[B]]\n",
    )
    .unwrap();

    let conn = index::open_index(root).unwrap();
    index::rebuild(&conn, root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "B").unwrap().len(), 1);
  }
```

- [ ] **Step 2: kb index テストから inbox を削除（index.rs）**

`src-tauri/src/kb/infrastructure/index.rs` の `mod tests` から `inbox_upsert_list_and_status` テストを丸ごと削除する。

- [ ] **Step 3: テスト側の inbox 参照を消した状態でテストが通ることを確認（チェックポイント）**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p expert_base_lib kb::`
Expected: PASS（実装はまだ inbox を持つので緑のまま）。

> 注: inbox 撤去は純削除なので「fail first」は使えない。本タスクは「テスト側の inbox 参照を先に消す（Step 1-2）→ 実装を消す（Step 4 以降）→ 緑を維持」で進める。Step 3 はその中間チェックポイント。

- [ ] **Step 4: index.rs から inbox 実装を削除**

`src-tauri/src/kb/infrastructure/index.rs`：

1. `InboxItem` 構造体（現 `:44-54`）を削除。
2. `ensure_schema` の `CREATE TABLE IF NOT EXISTS inbox(...)` ブロック（現 `:79-85`）を `execute_batch` 文字列から削除する。残すスキーマは entries / links / インデックス / entries_fts。
3. `rebuild` の `execute_batch` から `DELETE FROM inbox;` を削除し（現 `:106`）、inbox スキャンブロック（現 `:124-137` の `let inbox_dir = ...` 〜 対応する `}`）を削除する。
4. `upsert_inbox`（現 `:298-315`）、`set_inbox_status`（現 `:317-326`）、`delete_inbox`（現 `:328-334`）、`list_inbox`（現 `:336-353`）を削除。

削除後の `ensure_schema` の `execute_batch` 引数：

```rust
      "CREATE TABLE IF NOT EXISTS entries(
         path TEXT PRIMARY KEY,
         type TEXT NOT NULL,
         title TEXT NOT NULL,
         description TEXT NOT NULL DEFAULT '',
         cat TEXT NOT NULL DEFAULT '',
         tags TEXT NOT NULL DEFAULT '',
         updated TEXT NOT NULL DEFAULT '',
         words INTEGER NOT NULL DEFAULT 0
       );
       CREATE TABLE IF NOT EXISTS links(
         src_path TEXT NOT NULL,
         dst_title TEXT NOT NULL
       );
       CREATE INDEX IF NOT EXISTS idx_links_dst ON links(dst_title);
       CREATE INDEX IF NOT EXISTS idx_links_src ON links(src_path);
       CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_title ON entries(title);
       CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts
         USING fts5(title, body, path UNINDEXED, tokenize='trigram');",
```

削除後の `rebuild`（inbox スキャン除去・`DELETE FROM inbox` 除去）：

```rust
pub fn rebuild(conn: &Connection, root: &Path) -> Result<(), String> {
  conn
    .execute_batch("DELETE FROM entries; DELETE FROM links; DELETE FROM entries_fts;")
    .map_err(|e| e.to_string())?;

  let entries_dir = root.join("entries");
  if entries_dir.is_dir() {
    for ent in std::fs::read_dir(&entries_dir).map_err(|e| e.to_string())? {
      let path = ent.map_err(|e| e.to_string())?.path();
      if path.extension().and_then(|s| s.to_str()) != Some("md") {
        continue;
      }
      let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
      let entry = crate::kb::domain::entry::parse_entry(&text)?;
      let rel = format!("entries/{}", path.file_name().unwrap().to_string_lossy());
      upsert_entry(conn, &rel, &entry)?;
    }
  }
  Ok(())
}
```

- [ ] **Step 5: kb application から inbox 関数を削除（application.rs）**

`src-tauri/src/kb/application.rs`：

1. `read_inbox_material`（現 `:130-135`）と `delete_inbox_material`（現 `:137-154`）を削除。
2. 先頭の `use crate::kb::domain::material;`（現 `:10`）を削除（他に未使用になる）。
3. `mod tests` から `delete_inbox_material_removes_file_attachment_and_index` テスト（現 `:315-335`）を削除。

- [ ] **Step 6: kb interface から inbox コマンドを削除（interface.rs）**

`src-tauri/src/kb/interface.rs` から `kb_read_inbox_material`（現 `:109-113`）、`kb_delete_inbox_material`（現 `:121-125`）、`kb_list_inbox`（現 `:127-132`）を削除する。

- [ ] **Step 7: `checked_kb_markdown_path` 再エクスポートを削除（mod.rs）**

`src-tauri/src/kb/mod.rs` の次の行（現 `:13`）を削除する：

```rust
pub(crate) use domain::registry::checked_kb_markdown_path;
```

（`registry.rs` の定義は残す。kb::application が `registry::checked_kb_markdown_path` を直接使い続ける。）

- [ ] **Step 8: lib.rs から kb inbox コマンド登録を削除**

`src-tauri/src/lib.rs` の `generate_handler!` から次の 3 行を削除する：

```rust
      kb::interface::kb_read_inbox_material,
      kb::interface::kb_delete_inbox_material,
      kb::interface::kb_list_inbox,
```

- [ ] **Step 9: 全テストが通ることを確認**

Run: `bun run test`
Expected: PASS（inbox を参照する compiled code が消え、警告も出ない）。

念のため未使用 import / dead_code 警告が無いことを確認：

Run: `cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -i warning || echo "no warnings"`
Expected: `no warnings`（または無関係な既存警告のみ）。

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/kb src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
refactor(kb): remove inbox — drop table, index helpers, commands

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: 工坊フロントから inbox 依存を撤去する

`workshop-view.tsx` の選択池（pending / pool）と inbox 利用を消し、「+」を外部ファイル添付のみにする。素材チップ・selection 状態・Inspector は残す。

**Files:**
- Modify: `frontend/src/features/workshop/ui/workshop-view.tsx`

**Interfaces:**
- Consumes: `pickLocalFile`, `workshopChat`, `workshopCancel`, `aiHasKey`, `listOllamaModels`（client.ts）, `RAW_TYPE`/`RawMaterial`/`RawType`（entities/material）。
- Produces: なし（UI のみ）。

> フロントは単体テストランナーが無い（`bun run test` は cargo のみ）。検証は `bun run lint` + `bun run build`。

- [ ] **Step 1: import から inbox 系を削除**

`frontend/src/features/workshop/ui/workshop-view.tsx` 冒頭の client import（現 `:14-25`）から `listInbox` / `deleteInboxMaterial` / `InboxItem` を削除する：

```ts
import {
  aiHasKey,
  listOllamaModels,
  pickLocalFile,
  workshopCancel,
  workshopChat,
  type ChatPhase,
  type OllamaModel,
} from "@/shared/api/tauri/client";
```

material import（現 `:26`）から `inboxToMaterial` を削除する：

```ts
import { RAW_TYPE, type RawMaterial, type RawType } from "@/entities/material";
```

- [ ] **Step 2: PREVIEW_MATERIALS と inbox 状態・派生を削除**

- `PREVIEW_MATERIALS` 定数（現 `:38-88`）を削除する。
- `const [inbox, setInbox] = useState<InboxItem[]>([]);`（現 `:103`）を削除する。
- `const [showPicker, setShowPicker] = useState(false);`（現 `:104`）を削除する。
- inbox を読む `useEffect`（現 `:126-136`、`setInbox(await listInbox())` を含むブロック）を削除する。
- `pending` / `visiblePending` / `pool` / `visiblePool` の派生（現 `:183-193`）を削除する。
- `materialFromInbox` 関数（現 `:605-612`）を削除する。

- [ ] **Step 3: 「+」を外部ファイル添付のみへ簡素化**

`addLocalFile`（現 `:213-221`）から `setShowPicker(false);` 行を削除する：

```ts
  // 外部のローカルファイルを素材に追加する（id は絶対パス。AI が read_source で読む、KB へは落とさない）。
  async function addLocalFile() {
    const path = await pickLocalFile();
    if (!path) return;
    const material = materialFromFile(path, t("workshop.addLocalFile"));
    setSources((current) =>
      current.some((s) => s.id === material.id) ? current : [...current, material]
    );
  }
```

`handleDelete`（現 `:224-233`、inbox 削除）を丸ごと削除する。

`reset()`（現 `:236-246`）と `runTurn()`（現 `:249-` の本体冒頭）から `setShowPicker(false);` の行を削除する。

- [ ] **Step 4: 会話開始前の MaterialSelect を撤去**

会話列の `{messages.length === 0 && ( <MaterialSelect ... /> )}` ブロック（現 `:349-357`）を削除する。`MaterialSelect` 関数定義（現 `:634-717`）も削除する。

- [ ] **Step 5: 「+」ボタンの popover を直接呼び出しへ置き換え**

コンポーザーの「+」ボタン部（現 `:463-521` の `<div className="relative flex-none"> ... </div>`）を次に置き換える（popover と pool を撤去、クリックで直接ファイル選択）：

```tsx
                {/* ＋ 外部ローカルファイルを素材に追加（OS のファイル選択ダイアログ） */}
                <button
                  type="button"
                  onClick={() => void addLocalFile()}
                  disabled={generating}
                  title={t("workshop.addLocalFile")}
                  className="grid size-9 flex-none place-items-center rounded-[10px] border border-line-strong bg-surface text-ink-soft transition-colors hover:bg-surface-2 disabled:opacity-40"
                >
                  <Icon name="plus" size={18} />
                </button>
```

- [ ] **Step 6: lint と build が通ることを確認**

Run: `bun run lint`
Expected: PASS（未使用 import / 変数なし）。

Run: `bun run build`
Expected: PASS。

- [ ] **Step 7: Commit**

```bash
git add frontend/src/features/workshop/ui/workshop-view.tsx
git commit -m "$(cat <<'EOF'
refactor(workshop-ui): drop inbox pool — '+' attaches external files only

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: ダッシュボードから inbox データを撤去する

`listInbox` 利用・「最近録入」ウィジェット・inbox 統計タイル・採集/整理の件数を外し、CTA を `/workshop` へ向ける。条目 / リンク統計は残す。

**Files:**
- Modify: `frontend/src/features/dashboard/ui/dashboard-view.tsx`
- Delete: `frontend/src/features/dashboard/ui/recent-materials.tsx`

**Interfaces:**
- Consumes: `stats`（client.ts）, `WikiHealth`。
- Produces: なし（UI のみ）。

- [ ] **Step 1: recent-materials を削除**

```bash
git rm frontend/src/features/dashboard/ui/recent-materials.tsx
```

- [ ] **Step 2: dashboard-view の import を整える**

`frontend/src/features/dashboard/ui/dashboard-view.tsx`：

`import { RecentMaterials } from "./recent-materials";`（現 `:12`）を削除する。
client import（現 `:14`）を `stats` のみへ：

```ts
import { stats as fetchStats } from "@/shared/api/tauri/client";
```

- [ ] **Step 3: state と取得を条目統計のみへ**

`data` の state（現 `:98`）とエフェクト（現 `:100-111`）を置き換える：

```ts
  const [data, setData] = useState({ wikiCount: 0, links: 0 });

  useEffect(() => {
    if (!available) return;
    void (async () => {
      const s = await fetchStats();
      setData({ wikiCount: s?.entries ?? 0, links: s?.links ?? 0 });
    })();
  }, [available]);
```

- [ ] **Step 4: PipelineStep の count を任意化し、採集/整理から外す**

`PipelineStep` の props 型（現 `:51-63`）で `count: string;` を `count?: string;` にし、count レンダリング（現 `:82`）を条件付きへ：

```tsx
        {count && <div className="mt-0.75 font-mono text-xs text-ink-faint">{count}</div>}
```

パイプライン本体（現 `:148-180`）で、採集（collect）・整理（work）の `count` 行を外し、KB / 関係 / 服务だけ件数を残す：

```tsx
        <div className="flex items-start">
          <PipelineStep icon="inbox" label={t("dash.p.collect")} tone="accent" />
          <PipelineStep icon="merge" label={t("dash.p.work")} tone="accent" />
          <PipelineStep
            icon="book"
            label={t("dash.p.kb")}
            count={count(data.wikiCount, "unit.entries")}
            tone="accent"
          />
          <PipelineStep
            icon="graph"
            label={t("dash.p.link")}
            count={count(data.links, "unit.links")}
            tone="ai"
          />
          <PipelineStep
            icon="bot"
            label={t("dash.p.serve")}
            count={count(0, "unit.members")}
            tone="accent"
            last
          />
        </div>
```

- [ ] **Step 5: inbox 統計タイルを外し、CTA を /workshop へ**

StatTile 群（現 `:183-212`）から先頭の inbox タイル（`label={t("dash.t.inbox")}` のもの）を削除し、wiki / links / qa の 3 枚を残す。

「Add」CTA（現 `:133-136`）の遷移先を `/workshop` へ：

```tsx
            <Link href="/workshop" className={cn(buttonVariants({ size: "lg" }))}>
              <Icon name="plus" size={17} />
              {t("c.add")}
            </Link>
```

- [ ] **Step 6: 下段グリッドを WikiHealth のみへ**

下段（現 `:214-217`）の `RecentMaterials` を外し、WikiHealth 単独にする：

```tsx
      <WikiHealth />
```

- [ ] **Step 7: lint と build が通ることを確認**

Run: `bun run lint`
Expected: PASS。

Run: `bun run build`
Expected: PASS。

- [ ] **Step 8: Commit**

```bash
git add -A frontend/src/features/dashboard
git commit -m "$(cat <<'EOF'
refactor(dashboard): drop inbox data — remove recent materials, point CTA to workshop

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: capture フィーチャ・ルート・ナビ項を削除する

これで `captureText` / `captureAudio` / `listInbox` 等の最後の利用者（capture-view）が消える。`/capture` への残リンクは Task 7 で解消済み。

**Files:**
- Delete: `frontend/src/features/capture/` 全体
- Delete: `frontend/src/app/(app)/capture/page.tsx`
- Modify: `frontend/src/shared/config/nav.ts`
- Modify: `frontend/src/widgets/app-shell/sidebar.tsx:43,48`

**Interfaces:**
- Consumes: なし（削除のみ）。
- Produces: `RouteId` から `"capture"` が消える。

- [ ] **Step 1: capture フィーチャとルートを削除**

```bash
git rm -r frontend/src/features/capture "frontend/src/app/(app)/capture"
```

- [ ] **Step 2: nav.ts から capture を削除**

`frontend/src/shared/config/nav.ts`：

`RouteId` 型から `| "capture"` を削除する：

```ts
export type RouteId =
  | "dash"
  | "workshop"
  | "wiki"
  | "graph"
  | "publish"
  | "bots"
  | "plugins";
```

`NAV` 配列から capture 項（現 `:24`）を削除する：

```ts
export const NAV: NavItem[] = [
  { id: "dash", href: "/", icon: "dash" },
  { id: "workshop", href: "/workshop", icon: "merge" },
  { id: "wiki", href: "/wiki", icon: "book" },
  { id: "graph", href: "/graph", icon: "graph", tone: "ai" },
  { id: "publish", href: "/publish", icon: "broadcast" },
  { id: "bots", href: "/bots", icon: "bot" },
  { id: "plugins", href: "/plugins", icon: "plug" },
];
```

- [ ] **Step 3: sidebar の slice 境界を調整**

`NAV` が 8→7 項になり、上段（主要）/下段（補助）の境界がずれる。`frontend/src/widgets/app-shell/sidebar.tsx` の `NAV.slice(0, 5)` を `NAV.slice(0, 4)` に、`NAV.slice(5)` を `NAV.slice(4)` に変える（上段: dash/workshop/wiki/graph、下段: publish/bots/plugins）：

```tsx
        {NAV.slice(0, 4).map(renderItem)}
```

```tsx
        {NAV.slice(4).map(renderItem)}
```

- [ ] **Step 4: lint と build が通ることを確認**

Run: `bun run lint`
Expected: PASS。

Run: `bun run build`
Expected: PASS（`/capture` ルートが消え、リンクも残っていない）。

- [ ] **Step 5: Commit**

```bash
git add -A frontend/src
git commit -m "$(cat <<'EOF'
feat(nav): remove capture feature, route, and nav entry

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: client.ts と material エンティティの死んだ面を撤去する

capture-view 削除後、未使用になった IPC 関数・型を消す。

**Files:**
- Modify: `frontend/src/shared/api/tauri/client.ts`
- Delete: `frontend/src/entities/material/model/adapt.ts`
- Modify: `frontend/src/entities/material/index.ts`
- Modify: `frontend/src/entities/material/model/types.ts`

**Interfaces:**
- Consumes: なし。
- Produces: `client.ts` から `InboxItem` / `DownloadProgress` / capture・inbox・transcribe 関数が消える。material は `RawMaterial`/`RAW_TYPE`/`RawType`/`RawStatus` のみ公開。

- [ ] **Step 1: client.ts から inbox / capture / transcribe を削除**

`frontend/src/shared/api/tauri/client.ts` から次を削除する：

- `InboxItem` 型（現 `:56-63`）
- `readInboxMaterial`（現 `:132-135`）
- `deleteInboxMaterial`（現 `:142-145`）
- `listInbox`（現 `:147-151`）
- `captureText`（現 `:153-156`）
- `captureFile`（現 `:158-161`）
- `captureWeb`（現 `:163-166`）
- `captureAudio`（現 `:168-171`）
- `DownloadProgress` 型 + `transcribeMaterial`（現 `:173-189`）

`workshopChat` の doc コメント（現 `:203-204`）を実態へ更新する：

```ts
/** 添付素材（外部ファイルの絶対パス id）+ 会話履歴で対話エージェントを 1 ターン回す。
 * 最終返信本文を返す。onPhase で進捗を受け取る。 */
```

- [ ] **Step 2: material の inbox アダプタを削除**

```bash
git rm frontend/src/entities/material/model/adapt.ts
```

`frontend/src/entities/material/index.ts` を次に置き換える（`inboxToMaterial` と未使用の `STATUS` 公開を撤去）：

```ts
// material エンティティの公開 API。
export type { RawMaterial, RawType, RawStatus } from "./model/types";
export { RAW_TYPE } from "./model/types";
```

`frontend/src/entities/material/model/types.ts` から未使用になった `STATUS` 定数（ファイル末尾の `export const STATUS: ...` ブロック）を削除する。`RawStatus` 型と `RawMaterial`（`status: RawStatus` を含む）と `RAW_TYPE` は残す。

- [ ] **Step 3: lint と build が通ることを確認**

Run: `bun run lint`
Expected: PASS。

Run: `bun run build`
Expected: PASS。

- [ ] **Step 4: Commit**

```bash
git add -A frontend/src
git commit -m "$(cat <<'EOF'
chore(client): drop dead capture/inbox/transcribe IPC and material adapter

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: 死んだ i18n キーを削除する

capture / tabs / inbox 専用キーを 3 言語（zh / en / ja）分まとめて削除する。未使用キーはビルドを壊さないが、混乱を避けるため掃除する。

**Files:**
- Modify: `frontend/src/shared/i18n/dictionaries.ts`

**Interfaces:**
- Consumes: なし。
- Produces: なし。

- [ ] **Step 1: 削除候補が本当に未使用か確認する**

Run:
```bash
cd frontend && grep -rnE '"(capture\.|tabs\.(upload|record|manual|web))|nav\.capture|empty\.materials|workshop\.(selectEmpty|selectEmptyHint|pendingMaterials)|dash\.(t\.inbox|recent|viewAll)|unit\.(materials|pending)|st\.(pending|transcribed|processed)' src | grep -v 'shared/i18n/dictionaries.ts'
```
Expected: 出力なし（= どれも dictionaries 以外から参照されていない）。もし参照が残っていたら、そのキーは削除対象から外す。

- [ ] **Step 2: 各言語ブロックから次のキー行を削除する**

zh / en / ja の各辞書ブロックで、以下のキーを持つ行を削除する（キー名は 3 言語共通）：

- `tabs.upload`, `tabs.record`, `tabs.manual`, `tabs.web`
- `capture.eyebrow`, `capture.title`, `capture.sub`, `capture.upload.title`, `capture.upload.sub`, `capture.web.placeholder`, `capture.web.crawl`, `capture.web.depth`, `capture.web.readability`, `capture.web.examples`, `capture.web.hint`, `capture.record.tip`, `capture.record.start`, `capture.record.stop`, `capture.record.lang`, `capture.record.lang.auto`, `capture.record.recording`, `capture.record.downloading`, `capture.record.transcribing`, `capture.manual.placeholder`, `capture.manual.count`, `capture.manual.save`, `capture.disabled`, `capture.recent`, `capture.delete`, `capture.delete.confirm`, `capture.summary`, `capture.toWorkshop`
- `nav.capture`, `nav.capture.sub`
- `empty.materials`, `empty.materials.sub`
- `workshop.selectEmpty`, `workshop.selectEmptyHint`, `workshop.pendingMaterials`
- `dash.t.inbox`, `dash.t.inbox.s`, `dash.recent`, `dash.viewAll`
- `unit.materials`, `unit.pending`
- `st.pending`, `st.transcribed`, `st.processed`

> 残すキー（参照が残る）: `dash.p.collect` / `dash.p.work`（パイプラインのラベル）、`unit.entries` / `unit.links` / `unit.members`、`workshop.addMaterial` / `workshop.addLocalFile`。

- [ ] **Step 3: lint と build が通ることを確認**

Run: `bun run lint && bun run build`
Expected: PASS。

- [ ] **Step 4: Commit**

```bash
git add frontend/src/shared/i18n/dictionaries.ts
git commit -m "$(cat <<'EOF'
chore(i18n): remove dead capture/inbox keys

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Final Verification

- [ ] **Step 1: 全バックエンドテスト**

Run: `bun run test`
Expected: PASS。

- [ ] **Step 2: フロント lint + build**

Run: `bun run lint && bun run build`
Expected: PASS。

- [ ] **Step 3: whisper feature ビルドが壊れていないこと（asr 休止の確認）**

Run: `cargo build --manifest-path src-tauri/Cargo.toml --features whisper 2>&1 | tail -5`
Expected: 成功（asr がコンパイル対象外なので、削除した inbox 関数を参照しない）。
> 注: `cmake` と C/C++ コンパイラが必要（macOS は `brew install cmake`、`MACOSX_DEPLOYMENT_TARGET=11.0`）。環境が無ければスキップし、その旨を報告する。

- [ ] **Step 4: 残存 inbox/capture 参照のスイープ**

Run:
```bash
grep -rniE 'inbox|capture_(text|file|web|audio)|transcribe_material|kb_list_inbox' src-tauri/src | grep -v '//' | grep -v 'src-tauri/src/asr/'
```
Expected: 出力なし（asr ソースは休止＝コメント外で参照が残っていてもコンパイルされない。それ以外に inbox/capture コマンド参照が無いこと）。

Run:
```bash
cd frontend && grep -rniE 'inbox|capture|/capture' src
```
Expected: 出力なし（または無関係な一致のみ）。

---

## 仮定 / 既知の限界（spec より）

- `fetch_web` は単一 URL の本文抽出のみ。多階層クロール・readability トグル・ドメイン許可リスト / SSRF 防御は入れない（local-first・単一ユーザー前提。多テナント化 / 自動巡回時に再評価）。
- 出処リンクの健全性チェック（原ファイル存在確認）はしない。リンクは原ファイルの移動 / 削除で烂れ得る。
- `fetch_web` で取得した URL は自動では `entry.sources` に入らない（添付素材ではないため）。必要ならモデルが本文中に URL を残す。明示の自動記録は YAGNI。
- asr は休止（disconnect）。録音 / 動画 / ASR / 臨時目录の再導入は別 spec。`src-tauri/src/asr/` のソースはディスクに残置。
- `capture` モジュールは「抽出器の置き場（doc + web）」として名前と実体がズレたまま残る。`extract` への改名は import 波及のため見送り（mod.rs にコメントで明示）。
