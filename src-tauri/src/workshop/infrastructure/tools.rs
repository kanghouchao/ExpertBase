//! workshop インフラ: Rig の `Tool` トレイト実装（KB 操作の AI ツール）。
//! search_kb は読み取り（FTS 検索）、write_entry は書き込み（application::confirm へ委譲）。
//! `Tool::call` は async だが sqlite/FS はブロッキングなので spawn_blocking で橋渡しし、
//! 各呼び出しで root から索引を開き直す（`Connection` は Sync ではないため共有しない）。

use std::convert::Infallible;
use std::path::{Path, PathBuf};

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::Deserialize;
use serde_json::json;

use crate::capture::{extract_docx, extract_pdf};
use crate::kb::index;

/// read_source の引数。id（素材識別子）を緩く受ける。
#[derive(Deserialize)]
pub struct ReadArgs {
  #[serde(default)]
  id: String,
}

/// search_kb の引数。弱いモデルが欠落させても落ちないよう default で緩く受ける。
#[derive(Deserialize)]
pub struct SearchArgs {
  #[serde(default)]
  query: String,
}

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

/// 添付素材を id で読む読み取りツール（inbox の内部素材 / ローカルの外部ファイル）。
/// sources は許可された素材 id の集合＝モデルが任意のパスを読むのを防ぐ。
/// 外部ファイルは読み取りのみ・KB へ落とさない。
pub struct ReadSource {
  pub root: PathBuf,
  pub sources: Vec<String>,
}

impl Tool for ReadSource {
  const NAME: &'static str = "read_source";
  type Error = Infallible;
  type Args = ReadArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Read the full text of an attached source material by its id (see the # Sources list). Read a source before translating, rewriting, summarizing, or answering questions about it."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "id": { "type": "string", "description": "Source id from the # Sources list" }
        },
        "required": ["id"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let root = self.root.clone();
    let sources = self.sources.clone();
    let out = tokio::task::spawn_blocking(move || read_blocking(&root, &sources, &args.id))
      .await
      .unwrap_or_else(|e| format!("(read task failed: {e})"));
    Ok(out)
  }
}

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
        "Search existing knowledge base entries by keyword. Returns matching entry titles and excerpts."
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

/// 新しい条目を KB へ書き込むツール（application::confirm へ委譲、source の inbox を processed に）。
pub struct WriteEntry {
  pub root: PathBuf,
  pub inbox_rels: Vec<String>,
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
        "Save a new entry into the knowledge base. Call only when the user asks to save or store the content."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "title": { "type": "string", "description": "Concise entry heading" },
          "cat": { "type": "string", "description": "Short lowercase English category, e.g. tea, finance" },
          "body": { "type": "string", "description": "Entry body in Markdown" }
        },
        "required": ["title", "body"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let root = self.root.clone();
    let inbox_rels = self.inbox_rels.clone();
    let out = tokio::task::spawn_blocking(move || write_blocking(&root, &inbox_rels, args))
      .await
      .unwrap_or_else(|e| format!("(write task failed: {e})"));
    Ok(out)
  }
}

/// 素材読み取り（ブロッキング）。id を許可集合で検証してから、接頭辞で内部 / 外部に振り分ける。
/// エラーは全てモデル向け文字列で返す（ループ継続）。外部ファイルは読み取りのみ・KB へ落とさない。
fn read_blocking(root: &Path, sources: &[String], id: &str) -> String {
  let id = id.trim();
  if id.is_empty() {
    return "(read_source needs a non-empty id)".to_string();
  }
  // 許可された素材だけ読む（モデルが任意パスを読むのを防ぐ）。
  if !sources.iter().any(|s| s == id) {
    return format!("(unknown source id: {id})");
  }
  let text = if id.starts_with("inbox/") {
    // 内部素材: KB の inbox を frontmatter ごと parse_material で読み、本文を返す。
    match crate::kb::checked_kb_markdown_path(id, "inbox") {
      Ok(rel) => match std::fs::read_to_string(root.join(&rel)) {
        Ok(raw) => match crate::kb::material::parse_material(&raw) {
          Ok(m) => Ok(m.body),
          Err(e) => Err(format!("parse error: {e}")),
        },
        Err(e) => Err(format!("read error: {e}")),
      },
      Err(e) => Err(format!("invalid source: {e}")),
    }
  } else {
    // 外部素材: ローカルファイルを拡張子で振り分け（pdf/docx は抽出、その他はテキスト）。
    let path = Path::new(id);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
      "pdf" => extract_pdf(path),
      "docx" => extract_docx(path),
      _ => std::fs::read_to_string(path).map_err(|e| e.to_string()),
    }
    .map_err(|e| format!("read error: {e}"))
  };
  match text {
    Ok(body) if body.trim().is_empty() => format!("(source {id} is empty)"),
    Ok(body) => body,
    Err(e) => format!("({e})"),
  }
}

/// FTS 検索（ブロッキング）。空クエリ・無ヒット・エラーもモデル向け文字列で返す（ループ継続）。
fn search_blocking(root: &Path, query: &str) -> String {
  let query = query.trim();
  if query.is_empty() {
    return "(empty query)".to_string();
  }
  let conn = match index::open_index(root) {
    Ok(c) => c,
    Err(e) => return format!("(index error: {e})"),
  };
  match index::search(&conn, query) {
    Ok(hits) if !hits.is_empty() => {
      let shown = hits.len().min(5);
      hits
        .iter()
        .take(shown)
        .map(|h| format!("- {}: {}", h.title, h.excerpt))
        .collect::<Vec<_>>()
        .join("\n")
    }
    Ok(_) => "(no matching entries)".to_string(),
    Err(e) => format!("(search error: {e})"),
  }
}

/// 条目書き込み（ブロッキング）。title/body を検証 → confirm で確定し inbox を processed に。
fn write_blocking(root: &Path, inbox_rels: &[String], args: WriteArgs) -> String {
  let title = args.title.trim();
  let body = args.body.trim();
  if title.is_empty() || body.is_empty() {
    return "(write_entry needs a non-empty title and body)".to_string();
  }
  let conn = match index::open_index(root) {
    Ok(c) => c,
    Err(e) => return format!("(index error: {e})"),
  };
  match crate::workshop::application::confirm(root, &conn, title, args.cat.trim(), body, inbox_rels) {
    Ok(rel) => format!("Saved entry to {rel}"),
    Err(e) => format!("(write error: {e})"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::entry::{Entry, EntryMeta};

  fn seed_entry(conn: &rusqlite::Connection, path: &str, title: &str, body: &str) {
    let entry = Entry {
      meta: EntryMeta {
        kind: "Entry".into(),
        title: title.into(),
        description: String::new(),
        cat: "x".into(),
        tags: vec![],
        sources: vec![],
        created: "2026-06-14".into(),
        updated: "2026-06-14".into(),
      },
      body: body.into(),
    };
    index::upsert_entry(conn, path, &entry).unwrap();
  }

  #[tokio::test]
  async fn read_source_reads_inbox_material_body() {
    use crate::kb::material::{serialize_material, Material, MaterialMeta};
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("inbox")).unwrap();
    let m = Material {
      meta: MaterialMeta {
        kind: "text".into(),
        source: "paste".into(),
        status: "pending".into(),
        attachment: String::new(),
        captured_at: "2026-06-14T00:00:00Z".into(),
      },
      body: "受信箱の本文テキスト".into(),
    };
    std::fs::write(root.join("inbox/m.md"), serialize_material(&m).unwrap()).unwrap();

    let tool = ReadSource { root: root.to_path_buf(), sources: vec!["inbox/m.md".into()] };
    let out = tool.call(ReadArgs { id: "inbox/m.md".into() }).await.unwrap();

    assert!(out.contains("受信箱の本文テキスト"));
  }

  #[tokio::test]
  async fn read_source_reads_external_local_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("外部メモ.md");
    std::fs::write(&file, "外部ファイルの内容").unwrap();
    let id = file.to_string_lossy().to_string();

    let tool = ReadSource { root: root.to_path_buf(), sources: vec![id.clone()] };
    let out = tool.call(ReadArgs { id }).await.unwrap();

    assert!(out.contains("外部ファイルの内容"));
  }

  #[tokio::test]
  async fn read_source_rejects_unknown_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let tool = ReadSource { root: root.to_path_buf(), sources: vec![] };

    let out = tool.call(ReadArgs { id: "inbox/secret.md".into() }).await.unwrap();
    assert!(out.contains("unknown source id"));
  }

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
  async fn write_entry_tool_persists_and_marks_inbox() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    index::upsert_inbox(&conn, "inbox/m.md", "text", "paste", "pending", "2026-06-14T00:00:00Z")
      .unwrap();

    let tool = WriteEntry { root: root.to_path_buf(), inbox_rels: vec!["inbox/m.md".into()] };
    let out = tool
      .call(WriteArgs { title: "緑茶".into(), cat: "tea".into(), body: "湯温は [[煎茶]] で70度".into() })
      .await
      .unwrap();

    assert!(out.starts_with("Saved entry to"));
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
    let inbox = index::list_inbox(&conn).unwrap();
    assert!(inbox.iter().all(|m| m.status == "processed"));
  }

  #[tokio::test]
  async fn write_entry_tool_rejects_missing_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    index::open_index(root).unwrap();
    let tool = WriteEntry { root: root.to_path_buf(), inbox_rels: vec![] };

    let out = tool
      .call(WriteArgs { title: "  ".into(), cat: "tea".into(), body: "x".into() })
      .await
      .unwrap();
    assert!(out.contains("needs a non-empty title and body"));
  }
}
