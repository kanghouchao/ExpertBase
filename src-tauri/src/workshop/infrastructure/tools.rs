//! workshop インフラ: Rig の `Tool` トレイト実装（KB 操作の AI ツール）。
//! search_kb は読み取り（FTS 検索）、write_entry は書き込み（application::confirm へ委譲）。
//! `Tool::call` は async だが sqlite/FS はブロッキングなので spawn_blocking で橋渡しし、
//! 各呼び出しで root から索引を開き直す（`Connection` は Sync ではないため共有しない）。

use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use unicode_normalization::UnicodeNormalization;

use crate::extract::{extract_docx, extract_pdf, extract_readable, fetch_html};
use crate::kb::index;

pub(crate) type UsedSources = Arc<Mutex<Vec<String>>>;

fn remember_source(used_sources: &UsedSources, source: &str) {
  let mut refs = used_sources.lock().unwrap_or_else(|e| e.into_inner());
  if !refs.iter().any(|item| item == source) {
    refs.push(source.to_string());
  }
}

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

/// 添付素材を id で読む読み取りツール（外部絶対パスのローカルファイルのみ）。
/// sources は許可された素材 id の集合＝モデルが任意のパスを読むのを防ぐ。
/// 読み取りのみ・KB へ落とさない。
pub struct ReadSource {
  pub sources: Vec<String>,
  pub used_sources: UsedSources,
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
    let sources = self.sources.clone();
    let used_sources = self.used_sources.clone();
    let out = tokio::task::spawn_blocking(move || read_blocking(&sources, &used_sources, &args.id))
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

/// 新しい条目を KB へ書き込むツール（application::confirm へ委譲）。
/// 同じ実行内で正常に読んだファイル / URL を entry.sources に記録する。
pub struct WriteEntry {
  pub root: PathBuf,
  pub used_sources: UsedSources,
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
    let source_refs = self.used_sources.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let out = tokio::task::spawn_blocking(move || write_blocking(&root, &source_refs, args))
      .await
      .unwrap_or_else(|e| format!("(write task failed: {e})"));
    Ok(out)
  }
}

/// 素材読み取り（ブロッキング）。id を許可集合で検証してから、拡張子で抽出器を選ぶ。
/// source は外部絶対パスのみ（pdf/docx は抽出、その他はテキスト読み）。
/// エラーは全てモデル向け文字列で返す（ループ継続）。読み取りのみ・KB へ落とさない。
fn read_blocking(sources: &[String], used_sources: &UsedSources, id: &str) -> String {
  let id = id.trim();
  if id.is_empty() {
    return "(read_source needs a non-empty id)".to_string();
  }
  // 許可された素材だけ読む（モデルが任意パスを読むのを防ぐ）。
  // macOS はファイル名を NFD で返し、モデルは NFC で打ち直すため、Unicode 正規化（NFC）で照合する
  // （バイト一致だと日本語名の素材を必ず取りこぼす）。読み取りは検証済みの保存側パスで開く。
  let want: String = id.nfc().collect();
  let Some(source) = sources.iter().find(|s| s.nfc().collect::<String>() == want) else {
    return format!("(unknown source id: {id})");
  };
  let path = Path::new(source);
  let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
  let text = match ext.as_str() {
    "pdf" => extract_pdf(path),
    "docx" => extract_docx(path),
    _ => std::fs::read_to_string(path).map_err(|e| e.to_string()),
  }
  .map_err(|e| format!("read error: {e}"));
  match text {
    Ok(body) => {
      remember_source(used_sources, source);
      if body.trim().is_empty() {
        format!("(source {id} is empty)")
      } else {
        body
      }
    }
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

/// fetch_web の引数。URL を緩く受ける。
#[derive(Deserialize)]
pub struct FetchArgs {
  #[serde(default)]
  url: String,
}

/// ユーザーが会話に渡した URL の本文を Markdown で返す読み取りツール。
/// `web::fetch_html`（HTTPS 取得）+ `web::extract_readable`（Readability→Markdown）を再利用する。
/// 単一 URL の本文抽出のみ。許可リスト / SSRF 防御は入れない（local-first・単一ユーザー前提）。
pub struct FetchWeb {
  pub used_sources: UsedSources,
}

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
      Ok((title, markdown)) => {
        remember_source(&self.used_sources, url);
        Ok(format_web_body(&title, &markdown))
      }
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::entry::{Entry, EntryMeta};
  use std::sync::{Arc, Mutex};
  use unicode_normalization::UnicodeNormalization;

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
  async fn read_source_reads_external_local_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("外部メモ.md");
    std::fs::write(&file, "外部ファイルの内容").unwrap();
    let id = file.to_string_lossy().to_string();

    let used_sources = Arc::new(Mutex::new(Vec::new()));
    let tool = ReadSource { sources: vec![id.clone()], used_sources: used_sources.clone() };
    let out = tool.call(ReadArgs { id }).await.unwrap();

    assert!(out.contains("外部ファイルの内容"));
    assert_eq!(*used_sources.lock().unwrap(), vec![file.to_string_lossy().to_string()]);
  }

  #[tokio::test]
  async fn read_source_matches_across_unicode_normalization() {
    // macOS のファイルダイアログはファイル名を NFD（分解）で返すが、モデルは tool 引数を
    // NFC（合成）で打ち直す。バイト一致だと日本語名の素材を必ず取りこぼすため、正規化して照合する。
    let tmp = tempfile::tempdir().unwrap();
    let nfd_name: String = "五分プレゼン.md".nfd().collect();
    let file = tmp.path().join(&nfd_name);
    std::fs::write(&file, "テキスト本文").unwrap();
    // sources にはディスク由来の NFD パス、モデルが渡すのは NFC パス。
    let nfd_id = file.to_string_lossy().to_string();
    let nfc_id: String = nfd_id.nfc().collect();
    assert_ne!(nfd_id, nfc_id, "前提: NFD と NFC でバイトが異なる");

    let tool = ReadSource {
      sources: vec![nfd_id],
      used_sources: Arc::new(Mutex::new(Vec::new())),
    };
    let out = tool.call(ReadArgs { id: nfc_id }).await.unwrap();

    assert!(out.contains("テキスト本文"), "was: {out}");
  }

  #[tokio::test]
  async fn read_source_rejects_unknown_id() {
    let used_sources = Arc::new(Mutex::new(Vec::new()));
    let tool = ReadSource { sources: vec![], used_sources: used_sources.clone() };
    let out = tool.call(ReadArgs { id: "/abs/secret.md".into() }).await.unwrap();
    assert!(out.contains("unknown source id"));
    assert!(used_sources.lock().unwrap().is_empty());
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
  async fn write_entry_tool_persists_and_records_source_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();

    let used_sources = Arc::new(Mutex::new(vec!["/abs/report.pdf".into()]));
    let tool = WriteEntry { root: root.to_path_buf(), used_sources };
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
    };

    let out = tool
      .call(WriteArgs { title: "  ".into(), cat: "tea".into(), body: "x".into() })
      .await
      .unwrap();
    assert!(out.contains("needs a non-empty title and body"));
  }

  #[tokio::test]
  async fn fetch_web_rejects_empty_url() {
    let used_sources = Arc::new(Mutex::new(Vec::new()));
    let out = FetchWeb { used_sources: used_sources.clone() }
      .call(FetchArgs { url: "  ".into() })
      .await
      .unwrap();
    assert!(out.contains("needs a non-empty url"), "was: {out}");
    assert!(used_sources.lock().unwrap().is_empty());
  }

  #[test]
  fn source_tracking_deduplicates_urls() {
    let used_sources = Arc::new(Mutex::new(Vec::new()));
    remember_source(&used_sources, "https://example.com/article");
    remember_source(&used_sources, "https://example.com/article");

    assert_eq!(
      *used_sources.lock().unwrap(),
      vec!["https://example.com/article".to_string()]
    );
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
}
