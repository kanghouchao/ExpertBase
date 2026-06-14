use std::path::Path;

use rusqlite::Connection;
use tauri::Manager;

use crate::ai::{AiError, AiProvider, EntrySummary, StructureRequest, StructureResult};
use crate::kb::entry::{Entry, EntryMeta};
use crate::kb::{index, store};

/// 素材本文から検索候補語を取り出す（空白・句読点で分割、3 文字以上）。
/// trigram FTS の制約上 3 文字未満は使えない。空白の無い CJK は recall が限られる（MVP の既知の制限）。
fn candidate_terms(source: &str, max: usize) -> Vec<String> {
  let mut seen = std::collections::HashSet::new();
  let mut terms = Vec::new();
  for raw in source.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation()) {
    let t = raw.trim();
    if t.chars().count() >= 3 && seen.insert(t.to_string()) {
      terms.push(t.to_string());
      if terms.len() >= max {
        break;
      }
    }
  }
  terms
}

/// 新素材に関連する既存条目を FTS で引く（RAG の検索段）。上位 n 件の title + excerpt。
pub fn related_entries(
  conn: &Connection,
  source: &str,
  n: usize,
) -> Result<Vec<EntrySummary>, String> {
  let mut seen_paths = std::collections::HashSet::new();
  let mut out = Vec::new();
  for term in candidate_terms(source, 12) {
    for hit in index::search(conn, &term)? {
      if seen_paths.insert(hit.path.clone()) {
        out.push(EntrySummary { title: hit.title, excerpt: hit.excerpt });
        if out.len() >= n {
          return Ok(out);
        }
      }
    }
  }
  Ok(out)
}

/// RAG 編成: 関連条目を引き、AiProvider で構造化草稿を生成する。
pub fn draft<P: AiProvider>(
  provider: &P,
  conn: &Connection,
  source_text: &str,
  instruction: &str,
) -> Result<StructureResult, AiError> {
  let related = related_entries(conn, source_text, 5).map_err(AiError::Other)?;
  provider.structure(StructureRequest {
    source_text: source_text.to_string(),
    related,
    instruction: instruction.to_string(),
  })
}

/// 承認された内容を `entries/` に確定し、インデックス更新 + 受信箱を processed にする。
/// AI 草稿でも手動入力でも同じ経路を通る。
pub fn confirm(
  root: &Path,
  conn: &Connection,
  title: &str,
  cat: &str,
  body: &str,
  inbox_rel: &str,
) -> Result<String, String> {
  let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
  let entry = Entry {
    meta: EntryMeta {
      kind: "Entry".into(),
      title: title.to_string(),
      description: String::new(),
      cat: cat.to_string(),
      tags: vec![],
      created: today.clone(),
      updated: today,
    },
    body: body.to_string(),
  };
  let rel = store::write_entry(root, &entry)?;
  index::upsert_entry(conn, &rel, &entry)?;
  index::set_inbox_status(conn, inbox_rel, "processed")?;
  Ok(rel)
}

/// 受信箱素材 + 指示文から AI 構造化草稿を生成する。
#[tauri::command]
pub fn workshop_draft(
  app: tauri::AppHandle,
  inbox_path: String,
  instruction: String,
) -> Result<StructureResult, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = crate::kb::open_active(&home)?;
  let inbox_rel = crate::kb::checked_kb_markdown_path(&inbox_path, "inbox")?;
  let raw = std::fs::read_to_string(root.join(inbox_rel)).map_err(|e| e.to_string())?;
  let material = store::parse_material(&raw)?;
  let key = crate::ai::get_api_key(&home).ok_or_else(|| AiError::NoKey.to_string())?;
  let provider = crate::ai::claude::ClaudeProvider::new(key);
  draft(&provider, &conn, &material.body, &instruction).map_err(|e| e.to_string())
}

/// 承認内容を条目として確定する（UI で手編集済みの値を受け取る）。
#[tauri::command]
pub fn workshop_confirm(
  app: tauri::AppHandle,
  inbox_path: String,
  title: String,
  cat: String,
  body: String,
) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = crate::kb::open_active(&home)?;
  let inbox_rel = crate::kb::checked_kb_markdown_path(&inbox_path, "inbox")?;
  confirm(&root, &conn, &title, &cat, &body, &inbox_rel.to_string_lossy())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ai::FakeProvider;
  use crate::kb::entry::EntryMeta;

  fn seed_entry(conn: &Connection, path: &str, title: &str, body: &str) {
    let entry = Entry {
      meta: EntryMeta {
        kind: "Entry".into(),
        title: title.into(),
        description: String::new(),
        cat: "x".into(),
        tags: vec![],
        created: "2026-06-14".into(),
        updated: "2026-06-14".into(),
      },
      body: body.into(),
    };
    index::upsert_entry(conn, path, &entry).unwrap();
  }

  #[test]
  fn related_entries_finds_keyword_matches() {
    let conn = Connection::open_in_memory().unwrap();
    index::ensure_schema(&conn).unwrap();
    // 検索語は 3 文字以上が必要（trigram）。「淹れ方」(3 文字) で一致させる。
    seed_entry(&conn, "entries/green.md", "緑茶の淹れ方", "湯温は70度。淹れ方の基本。");
    let related = related_entries(&conn, "新しい 淹れ方 のメモ", 5).unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].title, "緑茶の淹れ方");
  }

  #[test]
  fn draft_with_fake_provider_returns_result() {
    let conn = Connection::open_in_memory().unwrap();
    index::ensure_schema(&conn).unwrap();
    seed_entry(&conn, "entries/green.md", "緑茶の淹れ方", "湯温は70度。淹れ方の基本。");
    let result = draft(&FakeProvider, &conn, "新しい 淹れ方 の本文", "整理して").unwrap();
    assert_eq!(result.title, "新しい 淹れ方 の本文");
    // FakeProvider は関連条目のタイトルをリンク候補にする。
    assert_eq!(result.suggested_links, vec!["緑茶の淹れ方".to_string()]);
  }

  #[test]
  fn confirm_writes_entry_indexes_it_and_marks_inbox_processed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    index::upsert_inbox(&conn, "inbox/m.md", "text", "paste", "pending", "2026-06-14T00:00:00Z")
      .unwrap();

    let rel = confirm(root, &conn, "緑茶", "tea", "湯温は [[煎茶]] で70度", "inbox/m.md").unwrap();
    assert!(root.join(&rel).is_file());
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
    let inbox = index::list_inbox(&conn).unwrap();
    assert_eq!(inbox[0].status, "processed");
  }
}
