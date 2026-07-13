//! workshop インフラ: KB 読み取り系ツール（search_kb / list_kb / read_entry）。

use std::convert::Infallible;
use std::path::{Path, PathBuf};

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::Deserialize;
use serde_json::json;

use crate::kb::index;

use super::{resolve_entry, with_index, SearchArgs};

/// 既存条目をキーワード検索する読み取りツール（root から索引を開く）。
pub struct SearchKb {
  pub root: PathBuf,
}

impl Tool for SearchKb {
  const NAME: &'static str = "search_kb";
  type Error = Infallible;
  type Args = SearchArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Search existing knowledge base entries by keyword. Returns matching entry titles and excerpts. Use it to find related notes and avoid duplicates."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "query": { "type": "string", "description": "Keywords to search for in existing entries" }
        },
        "required": ["query"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let root = self.root.clone();
    let out = tokio::task::spawn_blocking(move || search_blocking(&root, &args.query))
      .await
      .unwrap_or_else(|e| format!("(search task failed: {e})"));
    Ok(out)
  }
}

/// list_kb の引数。引数なし（空オブジェクトを緩く受ける）。
#[derive(Deserialize)]
pub struct ListArgs {}

/// KB の条目一覧（title / cat / path）を返す読み取りツール。
pub struct ListKb {
  pub root: PathBuf,
}

impl Tool for ListKb {
  const NAME: &'static str = "list_kb";
  type Error = Infallible;
  type Args = ListArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "List knowledge base entries (title, category, path), newest first. Shows at most 100 entries; if more exist, the output ends with how many were omitted. Use it to get an overview of what the knowledge base contains."
          .to_string(),
      parameters: json!({ "type": "object", "properties": {} }),
    }
  }

  async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
    let root = self.root.clone();
    let out = tokio::task::spawn_blocking(move || list_blocking(&root))
      .await
      .unwrap_or_else(|e| format!("(list task failed: {e})"));
    Ok(out)
  }
}

/// read_entry の引数。path / 正確な title を 1 つの id として緩く受ける。
#[derive(Deserialize)]
pub struct ReadEntryArgs {
  #[serde(default)]
  id: String,
}

/// 既存条目の全文（生 Markdown）を読む読み取りツール。
/// id は索引に載っている path / title だけに解決する＝モデルの任意パス読みを防ぐ。
pub struct ReadEntry {
  pub root: PathBuf,
}

impl Tool for ReadEntry {
  const NAME: &'static str = "read_entry";
  type Error = Infallible;
  type Args = ReadEntryArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Read the full Markdown text of an existing knowledge base entry by its path (entries/*.md) or exact title. Use list_kb or search_kb first to find the entry. Read an entry before answering questions about it or building on it."
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
    let root = self.root.clone();
    let out = tokio::task::spawn_blocking(move || read_entry_blocking(&root, &args.id))
      .await
      .unwrap_or_else(|e| format!("(read task failed: {e})"));
    Ok(out)
  }
}

/// FTS 検索（ブロッキング）。空クエリ・無ヒット・エラーもモデル向け文字列で返す（ループ継続）。
fn search_blocking(root: &Path, query: &str) -> String {
  let query = query.trim();
  if query.is_empty() {
    return "(empty query)".to_string();
  }
  with_index(root, |conn| match index::search(conn, query) {
    Ok(hits) if !hits.is_empty() => {
      let shown = hits.len().min(5);
      Ok(
        hits
          .iter()
          .take(shown)
          .map(|h| format!("- {}: {}", h.title, h.excerpt))
          .collect::<Vec<_>>()
          .join("\n"),
      )
    }
    Ok(_) => Ok("(no matching entries)".to_string()),
    Err(e) => Err(format!("(search error: {e:?})")),
  })
  .unwrap_or_else(|notice| notice)
}

/// 条目一覧（ブロッキング）。コンテキスト保護のため先頭 100 件で打ち切り、残数を添える。
fn list_blocking(root: &Path) -> String {
  with_index(root, |conn| match index::list_entries(conn) {
    Ok(refs) if !refs.is_empty() => {
      let total = refs.len();
      let mut lines: Vec<String> = refs
        .iter()
        .take(100)
        .map(|r| {
          if r.cat.is_empty() {
            format!("- {} — {}", r.title, r.path)
          } else {
            format!("- {} [{}] — {}", r.title, r.cat, r.path)
          }
        })
        .collect();
      if total > 100 {
        lines.push(format!("(and {} more entries)", total - 100));
      }
      Ok(lines.join("\n"))
    }
    Ok(_) => Ok("(no entries)".to_string()),
    Err(e) => Err(format!("(list error: {e:?})")),
  })
  .unwrap_or_else(|notice| notice)
}

/// 条目全文読み（ブロッキング）。id（path / title）を索引で解決してから開く。
fn read_entry_blocking(root: &Path, id: &str) -> String {
  let id = id.trim();
  if id.is_empty() {
    return "(read_entry needs a non-empty path or title)".to_string();
  }
  let rel = match with_index(root, |conn| resolve_entry(conn, id)) {
    Ok((rel, _title)) => rel,
    Err(notice) => return notice,
  };
  match std::fs::read_to_string(root.join(rel)) {
    Ok(text) => text,
    Err(e) => format!("(read error: {e})"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use super::super::tests::seed_entry;

  #[tokio::test]
  async fn search_kb_tool_returns_matching_titles() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    seed_entry(&conn, "entries/green.md", "緑茶の淹れ方", "湯温は70度。淹れ方の基本。");

    let tool = SearchKb { root: root.to_path_buf() };
    let out = tool.call(SearchArgs { query: "淹れ方".into() }).await.unwrap();

    assert!(out.contains("緑茶の淹れ方"));
  }

  #[tokio::test]
  async fn search_kb_tool_reports_index_open_failure() {
    // root がファイルだと .expertbase/ が作れず索引を開けない＝(index error) を返す
    // （with_index が受け持つ「索引エラー → モデル向け文字列」の特性テスト）。
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("not-a-dir");
    std::fs::write(&file, "x").unwrap();

    let tool = SearchKb { root: file };
    let out = tool.call(SearchArgs { query: "茶".into() }).await.unwrap();

    assert!(out.contains("(index error"), "was: {out}");
  }

  #[tokio::test]
  async fn search_kb_tool_reports_no_match_and_empty_query() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    index::open_index(root).unwrap();
    let tool = SearchKb { root: root.to_path_buf() };

    assert_eq!(tool.call(SearchArgs { query: "  ".into() }).await.unwrap(), "(empty query)");
    let none = tool.call(SearchArgs { query: "存在しない".into() }).await.unwrap();
    assert_eq!(none, "(no matching entries)");
  }

  #[tokio::test]
  async fn list_kb_tool_lists_titles_and_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    seed_entry(&conn, "entries/green.md", "緑茶", "湯温は70度");
    seed_entry(&conn, "entries/black.md", "紅茶", "発酵茶");

    let out = ListKb { root: root.to_path_buf() }.call(ListArgs {}).await.unwrap();

    assert!(out.contains("緑茶"), "was: {out}");
    assert!(out.contains("entries/green.md"), "was: {out}");
    assert!(out.contains("紅茶"), "was: {out}");
  }

  #[tokio::test]
  async fn list_kb_tool_reports_empty_kb() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    index::open_index(root).unwrap();
    let out = ListKb { root: root.to_path_buf() }.call(ListArgs {}).await.unwrap();
    assert_eq!(out, "(no entries)");
  }

  #[tokio::test]
  async fn read_entry_tool_reads_by_path_and_title() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    std::fs::create_dir_all(root.join("entries")).unwrap();
    std::fs::write(root.join("entries/green.md"), "---\ntitle: 緑茶\n---\n\n湯温は70度").unwrap();
    seed_entry(&conn, "entries/green.md", "緑茶", "湯温は70度");

    let tool = ReadEntry { root: root.to_path_buf() };
    let by_path = tool.call(ReadEntryArgs { id: "entries/green.md".into() }).await.unwrap();
    let by_title = tool.call(ReadEntryArgs { id: "緑茶".into() }).await.unwrap();

    assert!(by_path.contains("湯温は70度"), "was: {by_path}");
    assert_eq!(by_path, by_title);
  }

  #[tokio::test]
  async fn read_entry_tool_reports_unknown_and_empty_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    index::open_index(root).unwrap();
    let tool = ReadEntry { root: root.to_path_buf() };

    let empty = tool.call(ReadEntryArgs { id: "  ".into() }).await.unwrap();
    assert!(empty.contains("non-empty"), "was: {empty}");
    let out = tool.call(ReadEntryArgs { id: "無い".into() }).await.unwrap();
    assert!(out.contains("no entry found"), "was: {out}");
  }

  #[tokio::test]
  async fn read_entry_tool_refuses_out_of_tree_paths_from_corrupted_index() {
    // 索引が壊れて entries/ 外のパスを含んでも、読み出しは拒否する（防御的再検証）。
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("kb");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(tmp.path().join("secret.md"), "機密").unwrap();
    let conn = index::open_index(&root).unwrap();
    seed_entry(&conn, "../secret.md", "機密メモ", "x");

    let tool = ReadEntry { root: root.clone() };
    let out = tool.call(ReadEntryArgs { id: "機密メモ".into() }).await.unwrap();

    assert!(out.contains("invalid entry path"), "was: {out}");
    assert!(!out.contains("機密"), "must not leak file content: {out}");
  }
}
