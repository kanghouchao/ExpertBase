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

/// 工作坊のツール一式を組んで汎用 `agent` へ注入するために返す。
/// read_source（素材読み取り）・list_kb・search_kb・read_entry・write_entry・fetch_web を常に登録する。
/// used_sources は read/fetch で読んだ素材を write_entry が entry.sources に残すための共有状態。
pub(crate) fn build_toolset(root: &Path, sources: &[String]) -> Vec<Box<dyn rig_core::tool::ToolDyn>> {
  let used_sources: UsedSources = Arc::new(Mutex::new(Vec::new()));
  vec![
    Box::new(ReadSource { sources: sources.to_vec(), used_sources: used_sources.clone() }),
    Box::new(ListKb { root: root.to_path_buf() }),
    Box::new(SearchKb { root: root.to_path_buf() }),
    Box::new(ReadEntry { root: root.to_path_buf() }),
    Box::new(WriteEntry { root: root.to_path_buf(), used_sources: used_sources.clone() }),
    Box::new(FetchWeb { used_sources }),
  ]
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
        "List all knowledge base entries (title, category, path), newest first. Use it to get an overview of what the knowledge base contains."
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
        "Read the full Markdown text of an existing knowledge base entry by its path (entries/*.md) or exact title. Use list_kb or search_kb first to find the entry."
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
    Err(e) => return format!("(index error: {e:?})"),
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
    Err(e) => format!("(search error: {e:?})"),
  }
}

/// 条目一覧（ブロッキング）。コンテキスト保護のため先頭 100 件で打ち切り、残数を添える。
fn list_blocking(root: &Path) -> String {
  let conn = match index::open_index(root) {
    Ok(c) => c,
    Err(e) => return format!("(index error: {e:?})"),
  };
  match index::list_entries(&conn) {
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
      lines.join("\n")
    }
    Ok(_) => "(no entries)".to_string(),
    Err(e) => format!("(list error: {e:?})"),
  }
}

/// 条目全文読み（ブロッキング）。id（path / title）を索引で解決してから開く。
fn read_entry_blocking(root: &Path, id: &str) -> String {
  let id = id.trim();
  if id.is_empty() {
    return "(read_entry needs a non-empty path or title)".to_string();
  }
  let conn = match index::open_index(root) {
    Ok(c) => c,
    Err(e) => return format!("(index error: {e:?})"),
  };
  let refs = match index::list_entries(&conn) {
    Ok(r) => r,
    Err(e) => return format!("(index error: {e:?})"),
  };
  // ponytail: 線形探索。条目一覧はメモリに収まる規模なので専用 SQL は不要。
  let Some(hit) = refs.iter().find(|r| r.path == id || r.title == id) else {
    return format!("(no entry found: {id})");
  };
  match std::fs::read_to_string(root.join(&hit.path)) {
    Ok(text) => text,
    Err(e) => format!("(read error: {e})"),
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
    Err(e) => return format!("(index error: {e:?})"),
  };
  match crate::workshop::application::confirm(root, &conn, title, args.cat.trim(), body, source_refs) {
    Ok(rel) => format!("Saved entry to {rel}"),
    Err(e) => format!("(write error: {e:?})"),
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
}
