//! workshop インフラ: KB 書き込み系ツール（write_entry / update_entry / delete_entry）。破壊的操作は確認ゲートを通す。

use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::Deserialize;
use serde_json::json;

use crate::kb::index;

use super::super::confirm::ConfirmGate;
use super::{resolve_entry, with_index, UsedSources};

/// write_entry の引数。title/body 必須だが緩く受けて中身で検証する。
#[derive(Deserialize)]
pub struct WriteArgs {
  #[serde(default)]
  title: String,
  #[serde(default)]
  cat: String,
  #[serde(default)]
  body: String,
}

/// 新しい条目を KB へ書き込むツール（application::confirm へ委譲）。
/// 同じ実行内で正常に読んだファイル / URL を entry.sources に記録する。
/// 書き込みは破壊的操作＝実行前に確認ゲートでユーザーの許可を取る。
pub struct WriteEntry {
  pub root: PathBuf,
  pub used_sources: UsedSources,
  pub gate: Arc<ConfirmGate>,
}

impl Tool for WriteEntry {
  const NAME: &'static str = "write_entry";
  type Error = Infallible;
  type Args = WriteArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Save a new entry into the knowledge base. Call only when the user asks to save or store the content. The user is asked to approve the save before it happens; if they deny it, do not retry unless asked."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "title": { "type": "string", "description": "Concise entry heading" },
          "cat": { "type": "string", "description": "Short lowercase English category, e.g. tea, finance, privacy" },
          "body": { "type": "string", "description": "Entry body in Markdown, using [[title]] links to related notes" }
        },
        "required": ["title", "body"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    // 引数検証を先に＝不正な要求で確認カードを出さない。
    let title = args.title.trim().to_string();
    if title.is_empty() || args.body.trim().is_empty() {
      return Ok("(write_entry needs a non-empty title and body)".to_string());
    }
    // 破壊的操作＝実行前にユーザー確認。拒否は説明文で返し、agent ループは継続する。
    if !self.gate.request(&format!("write_entry: save new entry \"{title}\"")).await {
      return Ok(format!("(user denied write_entry: {title})"));
    }
    let root = self.root.clone();
    let source_refs = self.used_sources.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let out = tokio::task::spawn_blocking(move || write_blocking(&root, &source_refs, args))
      .await
      .unwrap_or_else(|e| format!("(write task failed: {e})"));
    Ok(out)
  }
}

/// update_entry の引数。id（path / 正確な title）と新しい本文を緩く受ける。
#[derive(Deserialize)]
pub struct UpdateEntryArgs {
  #[serde(default)]
  id: String,
  #[serde(default)]
  body: String,
}

/// 既存条目の本文を上書きするツール（application::overwrite へ委譲）。
/// 上書きは破壊的操作＝実行前に確認ゲートでユーザーの許可を取る。
/// 確認カードには対象条目と新旧の差異概要（文字数）を載せる。
pub struct UpdateEntry {
  pub root: PathBuf,
  pub gate: Arc<ConfirmGate>,
}

impl Tool for UpdateEntry {
  const NAME: &'static str = "update_entry";
  type Error = Infallible;
  type Args = UpdateEntryArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Overwrite the body of an existing knowledge base entry, located by its path (entries/*.md) or exact title. The new body replaces the old one entirely, so read the entry first and include everything that should remain. Call only when the user asks to change a saved entry. The user is asked to approve the change before it happens; if they deny it, do not retry unless asked."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "id": { "type": "string", "description": "Entry path (entries/*.md) or exact entry title" },
          "body": { "type": "string", "description": "New entry body in Markdown (replaces the old body)" }
        },
        "required": ["id", "body"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    // 引数検証と位置決めを先に＝不正・不在の要求で確認カードを出さない。
    let id = args.id.trim().to_string();
    let body = args.body.trim().to_string();
    if id.is_empty() || body.is_empty() {
      return Ok("(update_entry needs a non-empty id and body)".to_string());
    }
    let root = self.root.clone();
    let prep = tokio::task::spawn_blocking(move || update_prepare(&root, &id))
      .await
      .unwrap_or_else(|e| Err(format!("(update task failed: {e})")));
    let (rel, title, old_chars) = match prep {
      Ok(found) => found,
      Err(notice) => return Ok(notice),
    };
    // 破壊的操作＝実行前にユーザー確認。拒否は説明文で返し、agent ループは継続する。
    let summary = format!(
      "update_entry: overwrite \"{title}\" ({rel})\nbody: {old_chars} chars -> {} chars",
      body.chars().count()
    );
    if !self.gate.request(&summary).await {
      return Ok(format!("(user denied update_entry: {title})"));
    }
    let root = self.root.clone();
    let out = tokio::task::spawn_blocking(move || update_blocking(&root, &rel, &body))
      .await
      .unwrap_or_else(|e| format!("(update task failed: {e})"));
    Ok(out)
  }
}

/// delete_entry の引数。id（path / 正確な title）を緩く受ける。
#[derive(Deserialize)]
pub struct DeleteEntryArgs {
  #[serde(default)]
  id: String,
}

/// 既存条目を削除するツール（kb::delete_entry_at へ委譲、ファイル + 索引）。
/// 削除は不可逆な破壊的操作＝実行前に確認ゲートでユーザーの許可を取る。
/// 確認カードには条目標題と被参照（backlinks）状況を載せ、断リンクを警告する。
pub struct DeleteEntry {
  pub root: PathBuf,
  pub gate: Arc<ConfirmGate>,
}

impl Tool for DeleteEntry {
  const NAME: &'static str = "delete_entry";
  type Error = Infallible;
  type Args = DeleteEntryArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Delete an existing knowledge base entry, located by its path (entries/*.md) or exact title. This is irreversible and breaks [[links]] pointing to the entry. Call only when the user asks to delete a saved entry. The user is asked to approve the deletion before it happens; if they deny it, do not retry unless asked."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "id": { "type": "string", "description": "Entry path (entries/*.md) or exact entry title" }
        },
        "required": ["id"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    // 引数検証と位置決めを先に＝不正・不在の要求で確認カードを出さない。
    let id = args.id.trim().to_string();
    if id.is_empty() {
      return Ok("(delete_entry needs a non-empty path or title)".to_string());
    }
    let root = self.root.clone();
    let prep = tokio::task::spawn_blocking(move || delete_prepare(&root, &id))
      .await
      .unwrap_or_else(|e| Err(format!("(delete task failed: {e})")));
    let (rel, title, backlinks) = match prep {
      Ok(found) => found,
      Err(notice) => return Ok(notice),
    };
    // 破壊的操作＝実行前にユーザー確認。被参照状況を添えて断リンクを警告する。
    let links_line = if backlinks.is_empty() {
      "no other entries link to it".to_string()
    } else {
      format!(
        "referenced by {} entr{} (their [[links]] will break): {}",
        backlinks.len(),
        if backlinks.len() == 1 { "y" } else { "ies" },
        backlinks.join(", ")
      )
    };
    let summary = format!("delete_entry: delete \"{title}\" ({rel})\n{links_line}");
    if !self.gate.request(&summary).await {
      return Ok(format!("(user denied delete_entry: {title})"));
    }
    let root = self.root.clone();
    let out = tokio::task::spawn_blocking(move || delete_blocking(&root, &rel))
      .await
      .unwrap_or_else(|e| format!("(delete task failed: {e})"));
    Ok(out)
  }
}

/// update_entry の位置決め（ブロッキング・読み取りのみ）。対象の相対パス・タイトル・
/// 旧本文の文字数（確認カードの差異概要用）を返す。
fn update_prepare(root: &Path, id: &str) -> Result<(String, String, usize), String> {
  let (rel, title) = with_index(root, |conn| resolve_entry(conn, id))?;
  let text = std::fs::read_to_string(root.join(&rel)).map_err(|e| format!("(read error: {e})"))?;
  let entry =
    crate::kb::entry::parse_entry(&text).map_err(|e| format!("(parse error: {e:?})"))?;
  // 差異概要の文字数は前後の空白を除いて数える（末尾改行で 1 ずれた数字を出さない）。
  Ok((rel, title, entry.body.trim().chars().count()))
}

/// 条目上書き（ブロッキング）。kb::update_entry_body（条目持久化）で本文差し替え + 索引更新。
fn update_blocking(root: &Path, rel: &str, body: &str) -> String {
  match crate::kb::update_entry_body(root, rel, body) {
    Ok(()) => format!("Updated {rel}"),
    Err(e) => format!("(update error: {e:?})"),
  }
}

/// delete_entry の位置決め（ブロッキング・読み取りのみ）。対象の相対パス・タイトル・
/// 被参照元のタイトル一覧（確認カードの断リンク警告用）を返す。
/// resolve と backlinks は同じ接続を共用する（一回の delete で二度開かない）。
fn delete_prepare(root: &Path, id: &str) -> Result<(String, String, Vec<String>), String> {
  with_index(root, |conn| {
    let (rel, title) = resolve_entry(conn, id)?;
    let backlinks = index::backlinks(conn, &title)
      .map_err(|e| format!("(index error: {e:?})"))?
      .into_iter()
      .map(|r| r.title)
      .collect();
    Ok((rel, title, backlinks))
  })
}

/// 条目削除（ブロッキング）。kb::delete_entry_at（条目持久化）でファイルと索引を原子的に消す。
fn delete_blocking(root: &Path, rel: &str) -> String {
  match crate::kb::delete_entry_at(root, rel) {
    Ok(()) => format!("Deleted {rel}"),
    Err(e) => format!("(delete error: {e:?})"),
  }
}

/// 条目書き込み（ブロッキング）。kb::create_entry（条目持久化）で確定する。
/// title/body の空検証は `WriteEntry::call` が確認カードの前に済ませている。
fn write_blocking(root: &Path, source_refs: &[String], args: WriteArgs) -> String {
  match crate::kb::create_entry(root, args.title.trim(), args.cat.trim(), args.body.trim(), source_refs) {
    Ok(rel) => format!("Saved entry to {rel}"),
    Err(e) => format!("(write error: {e:?})"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use super::super::tests::{auto_gate, manual_gate, seed_entry_file};
  use std::sync::Mutex;

  #[tokio::test]
  async fn write_entry_tool_persists_and_records_source_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();

    let used_sources = Arc::new(Mutex::new(vec!["/abs/report.pdf".into()]));
    let tool = WriteEntry { root: root.to_path_buf(), used_sources, gate: auto_gate(true) };
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

  #[tokio::test]
  async fn write_entry_tool_rejects_missing_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    index::open_index(root).unwrap();
    let tool = WriteEntry {
      root: root.to_path_buf(),
      used_sources: Arc::new(Mutex::new(Vec::new())),
      gate: auto_gate(true),
    };

    let out = tool
      .call(WriteArgs { title: "  ".into(), cat: "tea".into(), body: "x".into() })
      .await
      .unwrap();
    assert!(out.contains("needs a non-empty title and body"));
  }

  #[tokio::test]
  async fn write_entry_waits_for_user_confirmation_before_saving() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();

    let (gate, mut rx, pending) = manual_gate();
    let tool = WriteEntry {
      root: root.to_path_buf(),
      used_sources: Arc::new(Mutex::new(Vec::new())),
      gate,
    };
    let task = tokio::spawn(async move {
      tool
        .call(WriteArgs { title: "緑茶".into(), cat: "tea".into(), body: "本文".into() })
        .await
        .unwrap()
    });

    // 確認要求が流れ、応答するまで書き込まれない。
    let Some(crate::agent::StreamProgress::ConfirmRequest { id, summary }) = rx.recv().await else {
      panic!("expected ConfirmRequest event");
    };
    assert!(summary.contains("緑茶"), "was: {summary}");
    assert_eq!(index::stats(&conn).unwrap().entries, 0);

    crate::workshop::infrastructure::confirm::resolve(&pending, id, true);
    let out = task.await.unwrap();
    assert!(out.starts_with("Saved entry to"), "was: {out}");
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
  }

  #[tokio::test]
  async fn write_entry_denial_returns_notice_and_saves_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    let tool = WriteEntry {
      root: root.to_path_buf(),
      used_sources: Arc::new(Mutex::new(Vec::new())),
      gate: auto_gate(false),
    };

    let out = tool
      .call(WriteArgs { title: "緑茶".into(), cat: "tea".into(), body: "本文".into() })
      .await
      .unwrap();

    // 拒否は説明文としてモデルへ返り、KB には何も書かれない（ループは継続）。
    assert!(out.contains("user denied write_entry"), "was: {out}");
    assert_eq!(index::stats(&conn).unwrap().entries, 0);
  }

  #[tokio::test]
  async fn update_entry_tool_overwrites_body_and_index_after_approval() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    seed_entry_file(root, "entries/green.md", "緑茶", "湯温は70度");

    let tool = UpdateEntry { root: root.to_path_buf(), gate: auto_gate(true) };
    let out = tool
      .call(UpdateEntryArgs { id: "緑茶".into(), body: "湯温は80度 [[煎茶]]".into() })
      .await
      .unwrap();

    assert!(out.starts_with("Updated"), "was: {out}");
    // ファイル: 本文だけ差し替わり、メタ（title / created）は維持、updated は当日へ進む。
    let saved = std::fs::read_to_string(root.join("entries/green.md")).unwrap();
    let entry = crate::kb::entry::parse_entry(&saved).unwrap();
    assert_eq!(entry.body, "湯温は80度 [[煎茶]]");
    assert_eq!(entry.meta.title, "緑茶");
    assert_eq!(entry.meta.created, "2026-06-14");
    assert_ne!(entry.meta.updated, "2026-06-14");
    // 索引: 新本文で検索でき、旧本文は消え、リンクも張り直される。
    let conn = index::open_index(root).unwrap();
    assert_eq!(index::search(&conn, "湯温は80度").unwrap().len(), 1);
    assert!(index::search(&conn, "湯温は70度").unwrap().is_empty());
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
  }

  #[tokio::test]
  async fn update_entry_denial_keeps_entry_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    seed_entry_file(root, "entries/green.md", "緑茶", "湯温は70度");
    let before = std::fs::read_to_string(root.join("entries/green.md")).unwrap();

    let tool = UpdateEntry { root: root.to_path_buf(), gate: auto_gate(false) };
    let out = tool
      .call(UpdateEntryArgs { id: "緑茶".into(), body: "改ざん".into() })
      .await
      .unwrap();

    // 拒否は説明文としてモデルへ返り、ファイルも索引も変わらない（ループは継続）。
    assert!(out.contains("user denied update_entry"), "was: {out}");
    assert_eq!(std::fs::read_to_string(root.join("entries/green.md")).unwrap(), before);
    let conn = index::open_index(root).unwrap();
    assert_eq!(index::search(&conn, "湯温は70度").unwrap().len(), 1);
  }

  #[tokio::test]
  async fn update_entry_reports_unknown_entry_and_missing_args() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    index::open_index(root).unwrap();
    // gate は拒否固定＝以下の応答が拒否文でないことが「確認前に返っている」ことの証明。
    let tool = UpdateEntry { root: root.to_path_buf(), gate: auto_gate(false) };

    let out = tool.call(UpdateEntryArgs { id: "無い".into(), body: "x".into() }).await.unwrap();
    assert!(out.contains("no entry found"), "was: {out}");
    let empty = tool.call(UpdateEntryArgs { id: "  ".into(), body: "  ".into() }).await.unwrap();
    assert!(empty.contains("non-empty"), "was: {empty}");
  }

  #[tokio::test]
  async fn update_entry_confirm_summary_shows_change_overview() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    seed_entry_file(root, "entries/green.md", "緑茶", "湯温は70度");

    let (gate, mut rx, pending) = manual_gate();
    let tool = UpdateEntry { root: root.to_path_buf(), gate };
    let task = tokio::spawn(async move {
      tool
        .call(UpdateEntryArgs { id: "entries/green.md".into(), body: "湯温は80度です".into() })
        .await
        .unwrap()
    });

    // 確認カードには「どの条目か」と新旧の差異概要（文字数）が載り、応答まで書き込まれない。
    let Some(crate::agent::StreamProgress::ConfirmRequest { id, summary }) = rx.recv().await else {
      panic!("expected ConfirmRequest event");
    };
    assert!(summary.contains("緑茶"), "was: {summary}");
    assert!(summary.contains("entries/green.md"), "was: {summary}");
    assert!(summary.contains("6 chars -> 8 chars"), "was: {summary}");
    let unchanged = std::fs::read_to_string(root.join("entries/green.md")).unwrap();
    assert!(unchanged.contains("湯温は70度"), "must not write before approval");

    crate::workshop::infrastructure::confirm::resolve(&pending, id, true);
    let out = task.await.unwrap();
    assert!(out.starts_with("Updated"), "was: {out}");
  }

  #[tokio::test]
  async fn delete_entry_tool_deletes_file_and_index_after_approval() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    seed_entry_file(root, "entries/green.md", "緑茶", "湯温は70度");

    let tool = DeleteEntry { root: root.to_path_buf(), gate: auto_gate(true) };
    let out = tool.call(DeleteEntryArgs { id: "緑茶".into() }).await.unwrap();

    assert!(out.starts_with("Deleted"), "was: {out}");
    assert!(!root.join("entries/green.md").exists());
    let conn = index::open_index(root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 0);
    assert!(index::search(&conn, "湯温は70度").unwrap().is_empty());
  }

  #[tokio::test]
  async fn delete_entry_denial_keeps_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    seed_entry_file(root, "entries/green.md", "緑茶", "湯温は70度");

    let tool = DeleteEntry { root: root.to_path_buf(), gate: auto_gate(false) };
    let out = tool.call(DeleteEntryArgs { id: "緑茶".into() }).await.unwrap();

    // 拒否は説明文としてモデルへ返り、ファイルも索引も残る（ループは継続）。
    assert!(out.contains("user denied delete_entry"), "was: {out}");
    assert!(root.join("entries/green.md").exists());
    let conn = index::open_index(root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
  }

  #[tokio::test]
  async fn delete_entry_reports_unknown_entry_and_empty_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    index::open_index(root).unwrap();
    // gate は拒否固定＝以下の応答が拒否文でないことが「確認前に返っている」ことの証明。
    let tool = DeleteEntry { root: root.to_path_buf(), gate: auto_gate(false) };

    let out = tool.call(DeleteEntryArgs { id: "無い".into() }).await.unwrap();
    assert!(out.contains("no entry found"), "was: {out}");
    let empty = tool.call(DeleteEntryArgs { id: "  ".into() }).await.unwrap();
    assert!(empty.contains("non-empty"), "was: {empty}");
  }

  #[tokio::test]
  async fn delete_entry_confirm_summary_includes_title_and_backlinks() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // 煎茶 は 緑茶 から [[煎茶]] で参照されている＝削除すると断リンクが生じる。
    seed_entry_file(root, "entries/green.md", "緑茶", "淹れ方は [[煎茶]] を参照");
    seed_entry_file(root, "entries/sencha.md", "煎茶", "蒸し製の緑茶");

    let (gate, mut rx, pending) = manual_gate();
    let tool = DeleteEntry { root: root.to_path_buf(), gate };
    let task =
      tokio::spawn(async move { tool.call(DeleteEntryArgs { id: "煎茶".into() }).await.unwrap() });

    // 確認カードには条目標題と被参照（backlinks）状況が載り、応答まで削除されない。
    let Some(crate::agent::StreamProgress::ConfirmRequest { id, summary }) = rx.recv().await else {
      panic!("expected ConfirmRequest event");
    };
    assert!(summary.contains("煎茶"), "was: {summary}");
    assert!(summary.contains("entries/sencha.md"), "was: {summary}");
    assert!(summary.contains("緑茶"), "was: {summary}");
    assert!(root.join("entries/sencha.md").exists(), "must not delete before approval");

    crate::workshop::infrastructure::confirm::resolve(&pending, id, true);
    let out = task.await.unwrap();
    assert!(out.starts_with("Deleted"), "was: {out}");
    assert!(!root.join("entries/sencha.md").exists());
  }

  #[tokio::test]
  async fn delete_entry_confirm_summary_notes_absent_backlinks() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    seed_entry_file(root, "entries/green.md", "緑茶", "誰からも参照されない");

    let (gate, mut rx, pending) = manual_gate();
    let tool = DeleteEntry { root: root.to_path_buf(), gate };
    let task =
      tokio::spawn(async move { tool.call(DeleteEntryArgs { id: "緑茶".into() }).await.unwrap() });

    let Some(crate::agent::StreamProgress::ConfirmRequest { id, summary }) = rx.recv().await else {
      panic!("expected ConfirmRequest event");
    };
    assert!(summary.contains("no other entries link to it"), "was: {summary}");
    crate::workshop::infrastructure::confirm::resolve(&pending, id, false);
    task.await.unwrap();
  }
}
