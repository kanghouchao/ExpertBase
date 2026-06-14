# Local Core MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the L1 local-core MVP: a Markdown-as-source-of-truth knowledge base with a derived SQLite/FTS5 index, AI-free capture, an AI-assisted workshop (BYO Anthropic key), and the Wiki/Graph/Search/Dashboard views wired to real data.

**Architecture:** Markdown files under `<kb-root>/{inbox,entries,attachments}` are the source of truth. A SQLite database at `.expertbase/index.sqlite` is a fully rebuildable derived index (entry metadata, link edges, FTS5, inbox state). Rust owns all I/O and AI calls behind Tauri commands returning `Result<T, String>`; the React UI talks only through the typed client in `frontend/src/lib/tauri`. AI lives only in the workshop, behind an `AiProvider` trait so it is fakeable in tests.

**Tech Stack:** Tauri 2 (Rust, edition 2021), rusqlite (bundled SQLite + FTS5), serde/serde_yaml (frontmatter), regex (`[[links]]`), chrono (timestamps), reqwest (Anthropic API), Next.js 16 static export + React 19 (CodeMirror 6 editor, react-force-graph).

**Spec:** `docs/superpowers/specs/2026-06-13-local-core-mvp-design.md`

---

## File Structure

Rust (`src-tauri/src/`):

- `kb.rs` — EXISTING registry/config + commands. Add `mod entry; mod index; mod store;`, an `active_kb_root` helper, and the data-layer commands. Keep existing code untouched.
- `kb/entry.rs` — `EntryMeta`/`Entry` model, frontmatter parse/serialize, `[[link]]` extraction, word count. Pure functions.
- `kb/index.rs` — SQLite open/schema/rebuild/upsert/delete + queries (list, backlinks, orphans, search, stats, graph). Takes `&Connection` + `&Path`.
- `kb/store.rs` — filesystem read/write of `entries/*.md` and `inbox/*.md`, orchestrating index updates. Thin command wrappers live in `kb.rs`.
- `capture.rs` + `capture/{web,doc}.rs` — normalize each source into an `inbox/*.md` material.
- `ai.rs` + `ai/claude.rs` — `AiProvider` trait, request/result types, `ClaudeProvider`, and a test `FakeProvider`.
- `workshop.rs` — assemble `StructureRequest` from a material + related entries, call `AiProvider`, confirm result into `entries/`.

Frontend (`frontend/src/`):

- `lib/tauri/client.ts` — EXTEND with typed wrappers for the new commands + shared types.
- `lib/data/store.ts`, `lib/data/types.ts` — replace the empty mock exports with hook-based loaders calling the client.
- The existing views under `app/(app)/**` — wire to real data; remove empty-mock wiring.

---

## Phase 1 — KB Data Layer (Rust)

Foundation. Everything else depends on it. Fully testable with `bun run test` (no UI, no network).

### Task 1.1: Add data-layer dependencies

**Files:** Modify `src-tauri/Cargo.toml`

- [ ] **Step 1: Add dependencies under `[dependencies]`**

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
serde_yaml = "0.9"
regex = "1"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

- [ ] **Step 2: Verify the crate graph resolves and FTS5 is available**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: builds (downloads + compiles rusqlite/bundled SQLite). If FTS5 turns out unavailable in a later test, add `features = ["bundled", "modern_sqlite"]` — but bundled libsqlite3-sys enables FTS5 by default.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore(kb): add rusqlite/serde_yaml/regex/chrono for the data layer"
```

### Task 1.2: Entry model — frontmatter parse/serialize

**Files:** Create `src-tauri/src/kb/entry.rs`; Modify `src-tauri/src/kb.rs` (add `mod entry;`)

- [ ] **Step 1: Declare the submodule in `kb.rs`**

At the top of `src-tauri/src/kb.rs`, after the existing `use` lines, add:

```rust
pub mod entry;
pub mod index;
pub mod store;
```

(Create `index.rs`/`store.rs` as empty files now so the crate compiles; they are filled in later tasks. `touch src-tauri/src/kb/index.rs src-tauri/src/kb/store.rs`.)

- [ ] **Step 2: Write the failing tests in `kb/entry.rs`**

```rust
use serde::{Deserialize, Serialize};

/// 通常条目の `type` 既定値（OKF 互換の必須フィールド）。
fn default_entry_type() -> String {
  "Entry".to_string()
}

/// 条目（entries/*.md）の frontmatter。宣言順が YAML 出力順になる。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EntryMeta {
  #[serde(rename = "type", default = "default_entry_type")]
  pub kind: String,
  pub title: String,
  #[serde(default)]
  pub description: String,
  #[serde(default)]
  pub cat: String,
  #[serde(default)]
  pub tags: Vec<String>,
  pub created: String,
  pub updated: String,
}

/// frontmatter + 本文。本文中の `[[タイトル]]` がリンク。
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
  pub meta: EntryMeta,
  pub body: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample() -> Entry {
    Entry {
      meta: EntryMeta {
        kind: "Entry".into(),
        title: "緑茶の淹れ方".into(),
        description: "基本手順".into(),
        cat: "tea".into(),
        tags: vec!["howto".into(), "tea".into()],
        created: "2026-06-14".into(),
        updated: "2026-06-14".into(),
      },
      body: "湯温は [[煎茶]] で 70 度。\n\n参考: [[水質]]。\n".into(),
    }
  }

  #[test]
  fn parse_then_serialize_round_trips() {
    let entry = sample();
    let text = serialize_entry(&entry).unwrap();
    let parsed = parse_entry(&text).unwrap();
    assert_eq!(parsed, entry);
  }

  #[test]
  fn parse_defaults_type_when_missing() {
    let raw = "---\ntitle: t\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\nbody\n";
    let entry = parse_entry(raw).unwrap();
    assert_eq!(entry.meta.kind, "Entry");
    assert_eq!(entry.body, "body\n");
  }

  #[test]
  fn parse_rejects_text_without_frontmatter() {
    assert!(parse_entry("no frontmatter here").is_err());
  }

  #[test]
  fn extract_links_dedupes_in_order() {
    assert_eq!(
      extract_links("a [[X]] b [[Y]] c [[X]]"),
      vec!["X".to_string(), "Y".to_string()]
    );
  }

  #[test]
  fn word_count_counts_whitespace_separated_tokens() {
    assert_eq!(word_count("hello  world\nthree"), 3);
  }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::entry`
Expected: FAIL — `serialize_entry`/`parse_entry`/`extract_links`/`word_count` not found.

- [ ] **Step 4: Implement the functions in `kb/entry.rs`** (above the `#[cfg(test)]` block)

```rust
use once_cell::sync::Lazy; // if not desired, build the Regex inline; see note
use regex::Regex;

const FENCE: &str = "---";

/// frontmatter 付き Markdown を Entry に解析する。
pub fn parse_entry(raw: &str) -> Result<Entry, String> {
  let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
  let raw = raw.trim_start_matches('\r');
  let rest = raw
    .strip_prefix(&format!("{FENCE}\n"))
    .or_else(|| raw.strip_prefix(&format!("{FENCE}\r\n")))
    .ok_or_else(|| "frontmatter が見つかりません".to_string())?;
  let end = rest
    .find(&format!("\n{FENCE}"))
    .ok_or_else(|| "frontmatter の終端が見つかりません".to_string())?;
  let yaml = &rest[..end];
  // 終端フェンス行（とその改行）を飛ばして本文を取り出す。
  let after = &rest[end + 1 + FENCE.len()..];
  let body = after
    .strip_prefix("\r\n")
    .or_else(|| after.strip_prefix('\n'))
    .unwrap_or(after);
  let body = body.strip_prefix("\r\n").or_else(|| body.strip_prefix('\n')).unwrap_or(body);
  let meta: EntryMeta = serde_yaml::from_str(yaml).map_err(|e| e.to_string())?;
  Ok(Entry { meta, body: body.to_string() })
}

/// Entry を frontmatter 付き Markdown 文字列へ直列化する。
pub fn serialize_entry(entry: &Entry) -> Result<String, String> {
  let yaml = serde_yaml::to_string(&entry.meta).map_err(|e| e.to_string())?;
  Ok(format!("{FENCE}\n{yaml}{FENCE}\n\n{}", entry.body))
}

static LINK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\[([^\]\[]+)\]\]").unwrap());

/// 本文中の `[[タイトル]]` を出現順・重複排除で抽出する。
pub fn extract_links(body: &str) -> Vec<String> {
  let mut seen = std::collections::HashSet::new();
  let mut out = Vec::new();
  for cap in LINK_RE.captures_iter(body) {
    let title = cap[1].trim().to_string();
    if seen.insert(title.clone()) {
      out.push(title);
    }
  }
  out
}

/// 空白区切りの語数。MVP の統計用の素朴な実装。
pub fn word_count(body: &str) -> usize {
  body.split_whitespace().count()
}
```

Note on `Lazy`: avoid adding `once_cell`. Instead compile the regex inside `extract_links` (`let re = Regex::new(...).unwrap();`) to keep dependencies minimal, OR use `std::sync::OnceLock`. Use `OnceLock`:

```rust
use std::sync::OnceLock;
fn link_re() -> &'static Regex {
  static RE: OnceLock<Regex> = OnceLock::new();
  RE.get_or_init(|| Regex::new(r"\[\[([^\]\[]+)\]\]").unwrap())
}
```
and call `link_re().captures_iter(body)`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::entry`
Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/kb.rs src-tauri/src/kb/entry.rs src-tauri/src/kb/index.rs src-tauri/src/kb/store.rs
git commit -m "feat(kb): add entry frontmatter parse/serialize and link extraction"
```

### Task 1.3: SQLite index — schema, rebuild, queries

**Files:** Modify `src-tauri/src/kb/index.rs`

- [ ] **Step 1: Write the failing tests** (define the public surface used by store/commands)

```rust
use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

use super::entry::Entry;

/// インデックスの軽量な条目参照（一覧・バックリンク・孤立で共用）。
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryRef {
  pub path: String,
  pub title: String,
  pub cat: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
  pub path: String,
  pub title: String,
  pub excerpt: String,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
  pub entries: usize,
  pub links: usize,
  pub orphans: usize,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphData {
  pub nodes: Vec<EntryRef>,
  pub edges: Vec<(String, String)>, // (src_path, dst_path) 解決済みのみ
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::entry::EntryMeta;

  fn meta(title: &str, cat: &str) -> EntryMeta {
    EntryMeta {
      kind: "Entry".into(),
      title: title.into(),
      description: String::new(),
      cat: cat.into(),
      tags: vec![],
      created: "2026-06-14".into(),
      updated: "2026-06-14".into(),
    }
  }

  fn mem() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    ensure_schema(&conn).unwrap();
    conn
  }

  #[test]
  fn upsert_and_search_round_trip() {
    let conn = mem();
    let entry = Entry { meta: meta("緑茶", "tea"), body: "湯温は 70 度".into() };
    upsert_entry(&conn, "entries/green.md", &entry).unwrap();
    let hits = search(&conn, "湯温").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "entries/green.md");
  }

  #[test]
  fn backlinks_and_orphans() {
    let conn = mem();
    let a = Entry { meta: meta("A", "x"), body: "see [[B]]".into() };
    let b = Entry { meta: meta("B", "x"), body: "no links".into() };
    let c = Entry { meta: meta("C", "x"), body: "lonely".into() };
    upsert_entry(&conn, "entries/a.md", &a).unwrap();
    upsert_entry(&conn, "entries/b.md", &b).unwrap();
    upsert_entry(&conn, "entries/c.md", &c).unwrap();

    let back = backlinks(&conn, "B").unwrap();
    assert_eq!(back.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(), vec!["entries/a.md"]);

    // A は B を指す(outbound)、B は A から指される(inbound)、C は孤立。
    let orphan_paths: Vec<_> = orphans(&conn).unwrap().into_iter().map(|r| r.path).collect();
    assert_eq!(orphan_paths, vec!["entries/c.md".to_string()]);
  }

  #[test]
  fn delete_removes_from_all_tables() {
    let conn = mem();
    let a = Entry { meta: meta("A", "x"), body: "see [[B]]".into() };
    upsert_entry(&conn, "entries/a.md", &a).unwrap();
    delete_entry(&conn, "entries/a.md").unwrap();
    assert_eq!(stats(&conn).unwrap().entries, 0);
    assert_eq!(stats(&conn).unwrap().links, 0);
    assert!(search(&conn, "B").unwrap().is_empty());
  }

  #[test]
  fn stats_counts_entries_links_orphans() {
    let conn = mem();
    upsert_entry(&conn, "entries/a.md", &Entry { meta: meta("A", "x"), body: "[[B]]".into() }).unwrap();
    upsert_entry(&conn, "entries/b.md", &Entry { meta: meta("B", "x"), body: "x".into() }).unwrap();
    let s = stats(&conn).unwrap();
    assert_eq!(s.entries, 2);
    assert_eq!(s.links, 1);
    assert_eq!(s.orphans, 0); // A->B なので両者ともリンクに関与
  }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::index`
Expected: FAIL — functions/`ensure_schema` not defined.

- [ ] **Step 3: Implement schema + mutations + queries** (above tests)

```rust
use super::entry::{extract_links, word_count};

/// スキーマを作成する（存在すれば何もしない）。
pub fn ensure_schema(conn: &Connection) -> Result<(), String> {
  conn.execute_batch(
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
     CREATE TABLE IF NOT EXISTS inbox(
       path TEXT PRIMARY KEY,
       type TEXT NOT NULL,
       source TEXT NOT NULL DEFAULT '',
       status TEXT NOT NULL DEFAULT 'pending',
       captured_at TEXT NOT NULL DEFAULT ''
     );
     CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(title, body, path UNINDEXED);",
  )
  .map_err(|e| e.to_string())
}

/// `.expertbase/index.sqlite` を開き、スキーマを保証する。
pub fn open_index(root: &Path) -> Result<Connection, String> {
  let dir = root.join(".expertbase");
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let conn = Connection::open(dir.join("index.sqlite")).map_err(|e| e.to_string())?;
  ensure_schema(&conn)?;
  Ok(conn)
}

/// 条目をインデックスへ upsert する（メタ + リンク + FTS）。
pub fn upsert_entry(conn: &Connection, rel_path: &str, entry: &Entry) -> Result<(), String> {
  delete_entry(conn, rel_path)?;
  let tags = entry.meta.tags.join(",");
  conn.execute(
    "INSERT INTO entries(path,type,title,description,cat,tags,updated,words)
       VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
    rusqlite::params![
      rel_path,
      entry.meta.kind,
      entry.meta.title,
      entry.meta.description,
      entry.meta.cat,
      tags,
      entry.meta.updated,
      word_count(&entry.body) as i64,
    ],
  )
  .map_err(|e| e.to_string())?;
  for dst in extract_links(&entry.body) {
    conn
      .execute("INSERT INTO links(src_path,dst_title) VALUES(?1,?2)", rusqlite::params![rel_path, dst])
      .map_err(|e| e.to_string())?;
  }
  conn
    .execute(
      "INSERT INTO entries_fts(title,body,path) VALUES(?1,?2,?3)",
      rusqlite::params![entry.meta.title, entry.body, rel_path],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// 条目をインデックスの全テーブルから削除する。
pub fn delete_entry(conn: &Connection, rel_path: &str) -> Result<(), String> {
  conn.execute("DELETE FROM entries WHERE path=?1", [rel_path]).map_err(|e| e.to_string())?;
  conn.execute("DELETE FROM links WHERE src_path=?1", [rel_path]).map_err(|e| e.to_string())?;
  conn.execute("DELETE FROM entries_fts WHERE path=?1", [rel_path]).map_err(|e| e.to_string())?;
  Ok(())
}

/// 指定タイトルを参照している条目（バックリンク）。
pub fn backlinks(conn: &Connection, title: &str) -> Result<Vec<EntryRef>, String> {
  let mut stmt = conn
    .prepare(
      "SELECT e.path,e.title,e.cat FROM links l
         JOIN entries e ON e.path=l.src_path
         WHERE l.dst_title=?1 ORDER BY e.path",
    )
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([title], |r| Ok(EntryRef { path: r.get(0)?, title: r.get(1)?, cat: r.get(2)? }))
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// 孤立条目: 他から参照されず(inbound 無し)かつ自身もリンクを持たない(outbound 無し)。
pub fn orphans(conn: &Connection) -> Result<Vec<EntryRef>, String> {
  let mut stmt = conn
    .prepare(
      "SELECT e.path,e.title,e.cat FROM entries e
         WHERE e.path NOT IN (SELECT src_path FROM links)
           AND e.title NOT IN (SELECT dst_title FROM links)
         ORDER BY e.path",
    )
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| Ok(EntryRef { path: r.get(0)?, title: r.get(1)?, cat: r.get(2)? }))
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// FTS5 全文検索。マッチ箇所のスニペットを返す。
pub fn search(conn: &Connection, query: &str) -> Result<Vec<SearchHit>, String> {
  if query.trim().is_empty() {
    return Ok(vec![]);
  }
  let mut stmt = conn
    .prepare(
      "SELECT path,title,snippet(entries_fts,1,'[',']','…',12) FROM entries_fts
         WHERE entries_fts MATCH ?1 ORDER BY rank LIMIT 50",
    )
    .map_err(|e| e.to_string())?;
  // FTS5 構文エラーを避けるためクエリは引用してフレーズ扱いにする。
  let phrase = format!("\"{}\"", query.replace('"', "\"\""));
  let rows = stmt
    .query_map([phrase], |r| Ok(SearchHit { path: r.get(0)?, title: r.get(1)?, excerpt: r.get(2)? }))
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// インデックス全体から条目一覧を返す。
pub fn list_entries(conn: &Connection) -> Result<Vec<EntryRef>, String> {
  let mut stmt = conn
    .prepare("SELECT path,title,cat FROM entries ORDER BY updated DESC, path")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| Ok(EntryRef { path: r.get(0)?, title: r.get(1)?, cat: r.get(2)? }))
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// ダッシュボード用の統計。
pub fn stats(conn: &Connection) -> Result<Stats, String> {
  let entries: usize = conn
    .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0))
    .map_err(|e| e.to_string())? as usize;
  let links: usize = conn
    .query_row("SELECT COUNT(*) FROM links", [], |r| r.get::<_, i64>(0))
    .map_err(|e| e.to_string())? as usize;
  let orphans = orphans(conn)?.len();
  Ok(Stats { entries, links, orphans })
}

/// グラフ描画用のノード + 解決済みエッジ（dst_title を既存条目 path へ解決）。
pub fn graph(conn: &Connection) -> Result<GraphData, String> {
  let nodes = list_entries(conn)?;
  let mut stmt = conn
    .prepare(
      "SELECT l.src_path,e.path FROM links l JOIN entries e ON e.title=l.dst_title",
    )
    .map_err(|e| e.to_string())?;
  let edges = stmt
    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
  Ok(GraphData { nodes, edges })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::index`
Expected: PASS (4 tests). If FTS5 errors with "no such module: fts5", switch rusqlite features to `["bundled"]` confirmed and re-run; bundled enables FTS5.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/kb/index.rs
git commit -m "feat(kb): add SQLite index schema, mutations, and queries"
```

### Task 1.4: Inbox index helpers

**Files:** Modify `src-tauri/src/kb/index.rs`

- [ ] **Step 1: Write failing tests**

```rust
  #[test]
  fn inbox_upsert_list_and_status() {
    let conn = mem();
    upsert_inbox(&conn, "inbox/a.md", "web", "https://x", "pending", "2026-06-14T00:00:00Z").unwrap();
    let items = list_inbox(&conn).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, "pending");
    set_inbox_status(&conn, "inbox/a.md", "processed").unwrap();
    assert_eq!(list_inbox(&conn).unwrap()[0].status, "processed");
  }
```

Add the `InboxItem` type near the other index types:

```rust
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
  pub path: String,
  #[serde(rename = "type")]
  pub kind: String,
  pub source: String,
  pub status: String,
  pub captured_at: String,
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::index::tests::inbox`
Expected: FAIL — `upsert_inbox`/`list_inbox`/`set_inbox_status` not found.

- [ ] **Step 3: Implement**

```rust
/// 受信箱素材をインデックスへ upsert する。
pub fn upsert_inbox(
  conn: &Connection,
  rel_path: &str,
  kind: &str,
  source: &str,
  status: &str,
  captured_at: &str,
) -> Result<(), String> {
  conn
    .execute(
      "INSERT INTO inbox(path,type,source,status,captured_at) VALUES(?1,?2,?3,?4,?5)
         ON CONFLICT(path) DO UPDATE SET type=?2,source=?3,status=?4,captured_at=?5",
      rusqlite::params![rel_path, kind, source, status, captured_at],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// 受信箱の状態を更新する（pending → processed など）。
pub fn set_inbox_status(conn: &Connection, rel_path: &str, status: &str) -> Result<(), String> {
  conn
    .execute("UPDATE inbox SET status=?2 WHERE path=?1", rusqlite::params![rel_path, status])
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// 受信箱の一覧（取り込み新しい順）。
pub fn list_inbox(conn: &Connection) -> Result<Vec<InboxItem>, String> {
  let mut stmt = conn
    .prepare("SELECT path,type,source,status,captured_at FROM inbox ORDER BY captured_at DESC, path")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| {
      Ok(InboxItem {
        path: r.get(0)?,
        kind: r.get(1)?,
        source: r.get(2)?,
        status: r.get(3)?,
        captured_at: r.get(4)?,
      })
    })
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::index`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/kb/index.rs
git commit -m "feat(kb): index inbox state (upsert/list/status)"
```

### Task 1.5: Store — filesystem read/write + rebuild

**Files:** Modify `src-tauri/src/kb/store.rs`

- [ ] **Step 1: Write failing tests** (`tempfile` for the KB root)

```rust
use std::fs;
use std::path::Path;

use super::entry::{Entry, EntryMeta};
use super::index;

fn meta(title: &str) -> EntryMeta {
  EntryMeta {
    kind: "Entry".into(),
    title: title.into(),
    description: String::new(),
    cat: "x".into(),
    tags: vec![],
    created: "2026-06-14".into(),
    updated: "2026-06-14".into(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn write_entry_creates_file_and_indexes_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let entry = Entry { meta: meta("緑茶"), body: "[[煎茶]] が大事".into() };
    let rel = write_entry(root, &entry).unwrap();
    assert!(root.join(&rel).is_file());

    let conn = index::open_index(root).unwrap();
    index::rebuild(&conn, root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").is_ok(), true);
  }

  #[test]
  fn rebuild_scans_entries_and_inbox_from_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("entries")).unwrap();
    fs::create_dir_all(root.join("inbox")).unwrap();
    fs::write(
      root.join("entries/a.md"),
      "---\ntype: Entry\ntitle: A\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n[[B]]\n",
    )
    .unwrap();
    fs::write(
      root.join("inbox/m.md"),
      "---\ntype: web\nsource: https://x\nstatus: pending\ncaptured_at: 2026-06-14T00:00:00Z\n---\n\ntext\n",
    )
    .unwrap();

    let conn = index::open_index(root).unwrap();
    index::rebuild(&conn, root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::list_inbox(&conn).unwrap().len(), 1);
  }

  #[test]
  fn read_entry_round_trips_written_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let entry = Entry { meta: meta("緑茶"), body: "本文".into() };
    let rel = write_entry(root, &entry).unwrap();
    let read = read_entry(root, &rel).unwrap();
    assert_eq!(read.meta.title, "緑茶");
  }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb::store`
Expected: FAIL — `write_entry`/`read_entry`/`index::rebuild` not found.

- [ ] **Step 3: Implement store + add `rebuild`/inbox material model**

In `kb/store.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::entry::{parse_entry, serialize_entry, Entry};
use super::index;

/// タイトルから安全なファイル名(slug)を作る。日本語等はそのまま、パス区切りのみ除去。
fn slug(title: &str) -> String {
  let cleaned: String = title
    .chars()
    .map(|c| if "/\\:*?\"<>|".contains(c) { '-' } else { c })
    .collect();
  let cleaned = cleaned.trim().replace(' ', "-");
  if cleaned.is_empty() { "untitled".to_string() } else { cleaned }
}

/// 条目を `entries/<slug>.md` に書き出し、相対パスを返す。重複時は連番。
pub fn write_entry(root: &Path, entry: &Entry) -> Result<String, String> {
  let dir = root.join("entries");
  fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let base = slug(&entry.meta.title);
  let mut rel = format!("entries/{base}.md");
  let mut n = 2;
  while root.join(&rel).exists() {
    rel = format!("entries/{base}-{n}.md");
    n += 1;
  }
  fs::write(root.join(&rel), serialize_entry(entry)?).map_err(|e| e.to_string())?;
  Ok(rel)
}

/// 既存条目を相対パスから読む。
pub fn read_entry(root: &Path, rel_path: &str) -> Result<Entry, String> {
  let text = fs::read_to_string(root.join(rel_path)).map_err(|e| e.to_string())?;
  parse_entry(&text)
}

/// 既存条目を上書き保存する（相対パス指定）。
pub fn save_entry(root: &Path, rel_path: &str, entry: &Entry) -> Result<(), String> {
  fs::write(root.join(rel_path), serialize_entry(entry)?).map_err(|e| e.to_string())
}
```

In `kb/index.rs`, add `rebuild` (it needs the inbox frontmatter; parse via a small helper):

```rust
/// ディスクの entries/ と inbox/ を走査してインデックスを再構築する。
pub fn rebuild(conn: &Connection, root: &Path) -> Result<(), String> {
  conn
    .execute_batch("DELETE FROM entries; DELETE FROM links; DELETE FROM entries_fts; DELETE FROM inbox;")
    .map_err(|e| e.to_string())?;
  let entries_dir = root.join("entries");
  if entries_dir.is_dir() {
    for ent in std::fs::read_dir(&entries_dir).map_err(|e| e.to_string())? {
      let path = ent.map_err(|e| e.to_string())?.path();
      if path.extension().and_then(|s| s.to_str()) != Some("md") {
        continue;
      }
      let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
      let entry = super::entry::parse_entry(&text)?;
      let rel = format!("entries/{}", path.file_name().unwrap().to_string_lossy());
      upsert_entry(conn, &rel, &entry)?;
    }
  }
  let inbox_dir = root.join("inbox");
  if inbox_dir.is_dir() {
    for ent in std::fs::read_dir(&inbox_dir).map_err(|e| e.to_string())? {
      let path = ent.map_err(|e| e.to_string())?.path();
      if path.extension().and_then(|s| s.to_str()) != Some("md") {
        continue;
      }
      let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
      let m = super::store::parse_material(&text)?;
      let rel = format!("inbox/{}", path.file_name().unwrap().to_string_lossy());
      upsert_inbox(conn, &rel, &m.meta.kind, &m.meta.source, &m.meta.status, &m.meta.captured_at)?;
    }
  }
  Ok(())
}
```

Add the inbox material model + parser in `kb/store.rs`:

```rust
fn default_status() -> String { "pending".to_string() }

/// 受信箱素材（inbox/*.md）の frontmatter。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MaterialMeta {
  #[serde(rename = "type")]
  pub kind: String, // text/web/pdf/doc/audio/video/image
  #[serde(default)]
  pub source: String,
  #[serde(default = "default_status")]
  pub status: String, // pending/processed
  #[serde(default)]
  pub attachment: String,
  #[serde(default)]
  pub captured_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
  pub meta: MaterialMeta,
  pub body: String,
}

/// 受信箱素材を解析する（条目と同じフェンス規約）。
pub fn parse_material(raw: &str) -> Result<Material, String> {
  // entry の分割ロジックを共有するため、frontmatter を取り出してから型を変える。
  let entry_like = super::entry::split_frontmatter(raw)?;
  let meta: MaterialMeta = serde_yaml::from_str(&entry_like.0).map_err(|e| e.to_string())?;
  Ok(Material { meta, body: entry_like.1 })
}
```

Refactor `kb/entry.rs` to expose the split helper used by both parsers:

```rust
/// frontmatter(YAML) と本文に分割する共通ヘルパ。
pub fn split_frontmatter(raw: &str) -> Result<(String, String), String> {
  // parse_entry の分割部分をここに移し、parse_entry は split + deserialize に組み替える。
  // 返り値: (yaml文字列, 本文文字列)
  // 実装は parse_entry 内の分割ロジックと同一。
  // ...（Task 1.2 の分割ロジックをそのまま移植）
}
```

(When implementing: move the split logic from `parse_entry` into `split_frontmatter`, and make `parse_entry` call it then `serde_yaml::from_str`. Re-run the Task 1.2 tests to confirm no regression.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml kb`
Expected: PASS (all kb tests: entry + index + store).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/kb/store.rs src-tauri/src/kb/index.rs src-tauri/src/kb/entry.rs
git commit -m "feat(kb): add store read/write and full index rebuild from disk"
```

### Task 1.6: Tauri commands for the data layer

**Files:** Modify `src-tauri/src/kb.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Add an `active_kb_root` helper + commands in `kb.rs`**

```rust
use std::path::PathBuf;

/// アクティブなナレッジベースのルートパスを返す。未選択ならエラー。
fn active_kb_root(home: &Path) -> Result<PathBuf, String> {
  let registry = load_registry(home)?;
  let active = registry.active.ok_or("アクティブなナレッジベースがありません")?;
  Ok(PathBuf::from(active))
}

/// アクティブ KB のインデックスを開く（必要なら再構築）。
fn open_active(home: &Path) -> Result<(PathBuf, rusqlite::Connection), String> {
  let root = active_kb_root(home)?;
  let conn = index::open_index(&root)?;
  Ok((root, conn))
}

#[tauri::command]
pub fn kb_rebuild_index(app: tauri::AppHandle) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = open_active(&home)?;
  index::rebuild(&conn, &root)
}

#[tauri::command]
pub fn kb_list_entries(app: tauri::AppHandle) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::list_entries(&conn)
}

#[tauri::command]
pub fn kb_search(app: tauri::AppHandle, query: String) -> Result<Vec<index::SearchHit>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::search(&conn, &query)
}

#[tauri::command]
pub fn kb_backlinks(app: tauri::AppHandle, title: String) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::backlinks(&conn, &title)
}

#[tauri::command]
pub fn kb_stats(app: tauri::AppHandle) -> Result<index::Stats, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::stats(&conn)
}

#[tauri::command]
pub fn kb_graph(app: tauri::AppHandle) -> Result<index::GraphData, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::graph(&conn)
}

#[tauri::command]
pub fn kb_orphans(app: tauri::AppHandle) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::orphans(&conn)
}

#[tauri::command]
pub fn kb_read_entry(app: tauri::AppHandle, path: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let root = active_kb_root(&home)?;
  std::fs::read_to_string(root.join(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kb_save_entry(app: tauri::AppHandle, path: String, content: String) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = open_active(&home)?;
  let entry = entry::parse_entry(&content)?;
  std::fs::write(root.join(&path), &content).map_err(|e| e.to_string())?;
  index::upsert_entry(&conn, &path, &entry)
}

#[tauri::command]
pub fn kb_list_inbox(app: tauri::AppHandle) -> Result<Vec<index::InboxItem>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::list_inbox(&conn)
}
```

- [ ] **Step 2: Register commands in `lib.rs`**

Add to the existing `tauri::generate_handler!` list (after `kb::kb_set_active`):

```rust
      kb::kb_rebuild_index,
      kb::kb_list_entries,
      kb::kb_search,
      kb::kb_backlinks,
      kb::kb_stats,
      kb::kb_graph,
      kb::kb_orphans,
      kb::kb_read_entry,
      kb::kb_save_entry,
      kb::kb_list_inbox
```

- [ ] **Step 3: Verify compilation + full test run**

Run: `bun run test`
Expected: all Rust tests PASS, including the new `kb::*` tests.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/kb.rs src-tauri/src/lib.rs
git commit -m "feat(kb): expose data-layer Tauri commands (search/graph/stats/entries/inbox)"
```

---

## Phase 2 — Capture (Rust, AI-free)

Normalize each source into an `inbox/*.md` material. No AI. `capture.rs` defines the command surface + shared `write_material`; submodules handle web/doc extraction.

### Task 2.1: Capture core — write a material to `inbox/`

**Files:** Create `src-tauri/src/capture.rs`; Modify `src-tauri/src/lib.rs` (`mod capture;` + register)

- [ ] **Step 1: Failing test** — `write_material(root, kind, source, body, attachment) -> rel_path`, writes frontmatter (`type/source/status=pending/captured_at` via chrono) + body, and indexes it.

```rust
#[test]
fn write_text_material_creates_inbox_file_and_indexes() {
  let tmp = tempfile::tempdir().unwrap();
  let root = tmp.path();
  let conn = crate::kb::index::open_index(root).unwrap();
  let rel = write_material(root, &conn, "text", "paste", "メモ本文", None).unwrap();
  assert!(root.join(&rel).is_file());
  assert_eq!(crate::kb::index::list_inbox(&conn).unwrap().len(), 1);
}
```

- [ ] **Step 2/3:** Run (fails), then implement `write_material` (build `MaterialMeta`, `captured_at = chrono::Utc::now().to_rfc3339()`, serialize via the entry fence convention, write `inbox/<timestamp>-<n>.md`, then `index::upsert_inbox`). Reuse `kb::store::Material`/`MaterialMeta`.

- [ ] **Step 4:** Add commands `capture_text(content, source)` and `capture_file(path)` (digital PDF/Word → text via Task 2.2; audio/video/image → copy to `attachments/`, empty/optional body). Register in `lib.rs`. Run `bun run test`.

- [ ] **Step 5:** Commit `feat(capture): normalize text/file sources into inbox materials`.

### Task 2.2: Web + document extraction

**Files:** Create `src-tauri/src/capture/web.rs`, `src-tauri/src/capture/doc.rs`; Modify `Cargo.toml`

- [ ] Add deps (verify exact versions with `cargo add`): `dom_smoothie` + `htmd` (web), `pdf-extract`, `docx-rs`.
- [ ] `web.rs`: `extract_readable(html, url) -> (title, markdown)` using dom_smoothie (Readability) → htmd (HTML→MD). Test with a small inline HTML string asserting the article body survives and chrome is dropped.
- [ ] `doc.rs`: `extract_pdf(bytes) -> String` (pdf-extract), `extract_docx(path) -> String` (docx-rs). Test with a tiny fixture if feasible; otherwise unit-test the dispatch by extension and gate the heavy extraction behind an integration test.
- [ ] Command `capture_web(url)` fetches via reqwest (added in Phase 3; if Phase 3 not yet done, add reqwest here) → `extract_readable` → `write_material(kind="web", source=url)`.
- [ ] Run `bun run test`; commit `feat(capture): web readability + PDF/Word text extraction`.

---

## Phase 3 — AI Provider (Rust)

`AiProvider` trait isolates the LLM. MVP ships `ClaudeProvider` (Anthropic API via reqwest, BYO key) + a `FakeProvider` for tests. **Before writing the Claude client, consult the `claude-api` skill** for current model IDs, the Messages API shape, and streaming.

### Task 3.1: Trait + request/result types + fake

**Files:** Create `src-tauri/src/ai.rs`; Modify `src-tauri/src/lib.rs` (`mod ai;`)

- [ ] Define (per spec §AI接合面):

```rust
pub struct EntrySummary { pub title: String, pub excerpt: String }
pub struct StructureRequest { pub source_text: String, pub related: Vec<EntrySummary>, pub instruction: String }
pub struct StructureResult { pub title: String, pub cat: String, pub body_markdown: String, pub suggested_links: Vec<String> }
#[derive(Debug)]
pub enum AiError { NoKey, Network(String), RateLimited, Other(String) }
pub trait AiProvider { fn structure(&self, req: StructureRequest) -> Result<StructureResult, AiError>; }
```

(MVP: synchronous, non-streaming first — spec allows "完了後に一括表示". Streaming via Tauri `Channel` is a later enhancement task.)

- [ ] `FakeProvider` returns a deterministic `StructureResult` echoing input + first related title as a suggested link.
- [ ] Test the trait via the fake (no network). Commit `feat(ai): AiProvider trait, request/result types, fake provider`.

### Task 3.2: ClaudeProvider

**Files:** Create `src-tauri/src/ai/claude.rs`; Modify `Cargo.toml` (reqwest)

- [ ] Add `reqwest = { version = "0.12", features = ["json"] }` and ensure tokio runtime (Tauri provides one; use `reqwest::blocking` inside a command via `tauri::async_runtime::spawn_blocking`, OR make commands `async` and use async reqwest). Decide at implementation; prefer async commands.
- [ ] Build the Anthropic Messages request: model id from the `claude-api` skill, system prompt instructing structured output (title/cat/body/suggested_links as JSON), user content = instruction + source + related excerpts. Parse the JSON block from the response.
- [ ] Key storage: read API key from KB config (`kb.toml`) or a Rust-side settings file under `~/.expertBase/`. UI never sees the key. Add `ai_set_key`/`ai_has_key` commands.
- [ ] Map HTTP errors → `AiError` variants. Unit-test request building + response parsing with a captured JSON sample (no live network). Commit `feat(ai): Anthropic ClaudeProvider with BYO key`.

---

## Phase 4 — Workshop (Rust)

Orchestrates: material → related entries (FTS) → `AiProvider` → confirm into `entries/`.

### Task 4.1: RAG assembly + draft

**Files:** Create `src-tauri/src/workshop.rs`; Modify `lib.rs`

- [ ] `related_entries(conn, source_text, n) -> Vec<EntrySummary>`: take salient terms from the material, FTS-search, return top-N title+excerpt. Test with a seeded index + fake material.
- [ ] `draft(provider, conn, material_body, instruction) -> StructureResult`: assemble `StructureRequest` (source + related + instruction), call provider. Test with `FakeProvider`.
- [ ] Command `workshop_draft(inboxPath, instruction)`: read material, run `draft` with the configured provider, return `StructureResult`. Commit `feat(workshop): RAG assembly and AI draft orchestration`.

### Task 4.2: Confirm into entries

**Files:** Modify `src-tauri/src/workshop.rs`

- [ ] `confirm(root, conn, result, inbox_rel) -> entry_rel`: build `Entry` (meta from result.title/cat, `created`/`updated`=today, body=result.body_markdown), `kb::store::write_entry`, `index::upsert_entry`, then `index::set_inbox_status(inbox_rel, "processed")`. Test: after confirm, entry file exists, indexed, inbox item is `processed`.
- [ ] Command `workshop_confirm(inboxPath, title, cat, body)` (UI may have hand-edited the draft). Manual path (no AI) reuses the same confirm. Commit `feat(workshop): confirm draft into entries and mark inbox processed`.

---

## Phase 5 — Frontend wiring

Replace the empty mock data with real data via the typed client. Keep IPC behind `lib/tauri`. Honor the no-mock-data convention: empty states stay, but real data flows when present.

### Task 5.1: Extend the typed client

**Files:** Modify `frontend/src/lib/tauri/client.ts`

- [ ] Add types + wrappers for every Phase 1–4 command: `EntryRef`, `SearchHit`, `Stats`, `GraphData`, `InboxItem`, `StructureResult`; functions `listEntries`, `search`, `backlinks`, `stats`, `graph`, `orphans`, `readEntry`, `saveEntry`, `listInbox`, `captureText`, `captureWeb`, `captureFile`, `workshopDraft`, `workshopConfirm`, `rebuildIndex`. Each `isTauri()`-guarded like the existing wrappers. Lint. Commit.

### Task 5.2: Dashboard, Search, Wiki, Graph, Workshop, Capture views

**Files:** Modify the existing views under `frontend/src/app/(app)/**` and `frontend/src/components/dashboard/**`; replace `frontend/src/lib/data/store.ts`.

- [ ] **Dashboard:** load `stats()` + recent entries (`listEntries`) + `orphans()`; render real counts; keep empty state when zero.
- [ ] **Search:** input → `search(query)` → result list linking to Wiki entries.
- [ ] **Wiki:** `listEntries()` browse; open → `readEntry(path)` in a CodeMirror 6 markdown editor (add `@codemirror` deps); `saveEntry` on save; show `backlinks(title)`.
- [ ] **Graph:** `graph()` → `react-force-graph` (add dep); node click → open Wiki entry.
- [ ] **Capture view:** paste text → `captureText`; URL → `captureWeb`; file picker → `captureFile`; refresh inbox list.
- [ ] **Workshop view:** inbox list (`listInbox`, pending) → select → left source / right result; "AI 生成" → `workshopDraft`; editable result; "確定" → `workshopConfirm`; manual path writes without AI.
- [ ] After each view: `bun run lint`; after all: `bun run build`. Commit per view.

### Task 5.3: Integration smoke

- [ ] `bun run dev`, walk one loop: capture text → workshop draft (fake or real key) → confirm → entry appears in Wiki/Graph/Search, Dashboard counts update. Document the result. Commit any fixes.

---

## Self-Review Notes

- **Spec coverage:** KB layer (1.1–1.6), Capture incl. web/PDF/Word/media (2.1–2.2), AI trait + Claude + BYO key (3.1–3.2), Workshop RAG + confirm + manual path (4.1–4.2), Wiki/Graph/Search/Dashboard + Capture/Workshop UI (5.1–5.3). Deferred items (Whisper/OCR/multimodal/vector/merge/full-lint/publish/bots/plugins) are intentionally absent.
- **Type consistency:** `EntryMeta.kind` (serde rename "type"), `EntryRef`/`SearchHit`/`Stats`/`GraphData`/`InboxItem` defined once in `kb/index.rs` and reused by commands + mirrored in `client.ts`. `StructureRequest`/`StructureResult`/`EntrySummary` defined once in `ai.rs`.
- **Phases 2–5 granularity:** intentionally coarser than Phase 1; each task is refined into TDD micro-steps at execution time (dependency/API verification points are called out: rusqlite FTS5, gray_matter-vs-serde_yaml decided as serde_yaml, capture crate versions, Anthropic model id via claude-api skill, async-vs-blocking reqwest).
