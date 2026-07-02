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

use super::confirm::ConfirmGate;
use super::web_search::{BraveSearchBackend, SearchBackend};

pub(crate) type UsedSources = Arc<Mutex<Vec<String>>>;

fn remember_source(used_sources: &UsedSources, source: &str) {
  let mut refs = used_sources.lock().unwrap_or_else(|e| e.into_inner());
  if !refs.iter().any(|item| item == source) {
    refs.push(source.to_string());
  }
}

/// 工作坊のツール一式を組んで汎用 `agent` へ注入するために返す。
/// read_source・list_kb・search_kb・search_web・read_entry・write_entry・fetch_web を常に登録する。
/// used_sources は read/fetch で読んだ素材を write_entry が entry.sources に残すための共有状態。
/// brave_api_key は search_web の backend へだけ渡し、ツール出力には含めない。
/// gate は破壊的ツール（write_entry / update_entry / delete_entry）が実行前に
/// ユーザー確認を取るための確認ゲート。
pub(crate) fn build_toolset(
  root: &Path,
  sources: &[String],
  brave_api_key: String,
  gate: Arc<ConfirmGate>,
) -> Vec<Box<dyn rig_core::tool::ToolDyn>> {
  let used_sources: UsedSources = Arc::new(Mutex::new(Vec::new()));
  vec![
    Box::new(ReadSource { sources: sources.to_vec(), used_sources: used_sources.clone() }),
    Box::new(ListKb { root: root.to_path_buf() }),
    Box::new(SearchKb { root: root.to_path_buf() }),
    Box::new(SearchWeb { backend: Arc::new(BraveSearchBackend::new(brave_api_key)) }),
    Box::new(ReadEntry { root: root.to_path_buf() }),
    Box::new(WriteEntry { root: root.to_path_buf(), used_sources: used_sources.clone(), gate: gate.clone() }),
    Box::new(UpdateEntry { root: root.to_path_buf(), gate: gate.clone() }),
    Box::new(DeleteEntry { root: root.to_path_buf(), gate }),
    Box::new(FetchWeb { used_sources }),
  ]
}

/// read_source の引数。id（素材識別子）を緩く受ける。
#[derive(Deserialize)]
pub struct ReadArgs {
  #[serde(default)]
  id: String,
}

/// search_kb / search_web の引数。弱いモデルが欠落させても落ちないよう default で緩く受ける。
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

/// Web 検索で候補 URL を返す。本文は fetch_web で選択的に読む。
pub struct SearchWeb {
  pub backend: Arc<dyn SearchBackend>,
}

impl Tool for SearchWeb {
  const NAME: &'static str = "search_web";
  type Error = Infallible;
  type Args = SearchArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Search the web by keywords. Returns a JSON array of results with title, url, and snippet; returns a parenthesized plain-text notice when there are no results or the search fails. Use fetch_web on a selected URL to read the page."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "query": { "type": "string", "description": "Keywords to search for on the web" }
        },
        "required": ["query"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let query = args.query.trim();
    if query.is_empty() {
      return Ok("(search_web needs a non-empty query)".to_string());
    }
    match self.backend.search(query.to_string()).await {
      // 空結果は兄弟ツールと同じ括弧書きの案内で返す（裸の "[]" は弱いモデルが誤読する）。
      Ok(results) if results.is_empty() => Ok("(no web results)".to_string()),
      Ok(results) => Ok(
        serde_json::to_string(&results)
          .unwrap_or_else(|e| format!("(search_web result serialization failed: {e})")),
      ),
      Err(e) => Ok(format!("(search_web error: {e})")),
    }
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
        "List knowledge base entries (title, category, path), newest first. Shows at most 100 entries; if more exist, the output ends with how many were omitted."
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
        "Overwrite the body of an existing knowledge base entry, located by its path (entries/*.md) or exact title. The new body replaces the old one entirely. Call only when the user asks to change a saved entry."
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

/// 既存条目を削除するツール（kb::delete_entry_in へ委譲、ファイル + 索引）。
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
        "Delete an existing knowledge base entry, located by its path (entries/*.md) or exact title. This is irreversible. Call only when the user asks to delete a saved entry."
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

/// id（path / 正確な title）を索引で既存条目へ解決し、entries/*.md へ再検証した相対パスと
/// タイトルを返す（read_entry / update_entry 共用）。失敗は全てモデル向け文字列（ループ継続）。
fn resolve_entry_blocking(root: &Path, id: &str) -> Result<(String, String), String> {
  let conn = index::open_index(root).map_err(|e| format!("(index error: {e:?})"))?;
  let refs = index::list_entries(&conn).map_err(|e| format!("(index error: {e:?})"))?;
  // ponytail: 線形探索。条目一覧はメモリに収まる規模なので専用 SQL は不要。
  let Some(hit) = refs.into_iter().find(|r| r.path == id || r.title == id) else {
    return Err(format!("(no entry found: {id})"));
  };
  // 索引由来のパスでも entries/*.md に限定して再検証する（索引が壊れていても越境を防ぐ）。
  let Ok(rel) = crate::kb::checked_kb_markdown_path(&hit.path, "entries") else {
    return Err(format!("(invalid entry path in index: {})", hit.path));
  };
  Ok((rel.to_string_lossy().into_owned(), hit.title))
}

/// 条目全文読み（ブロッキング）。id（path / title）を索引で解決してから開く。
fn read_entry_blocking(root: &Path, id: &str) -> String {
  let id = id.trim();
  if id.is_empty() {
    return "(read_entry needs a non-empty path or title)".to_string();
  }
  let rel = match resolve_entry_blocking(root, id) {
    Ok((rel, _title)) => rel,
    Err(notice) => return notice,
  };
  match std::fs::read_to_string(root.join(rel)) {
    Ok(text) => text,
    Err(e) => format!("(read error: {e})"),
  }
}

/// update_entry の位置決め（ブロッキング・読み取りのみ）。対象の相対パス・タイトル・
/// 旧本文の文字数（確認カードの差異概要用）を返す。
fn update_prepare(root: &Path, id: &str) -> Result<(String, String, usize), String> {
  let (rel, title) = resolve_entry_blocking(root, id)?;
  let text = std::fs::read_to_string(root.join(&rel)).map_err(|e| format!("(read error: {e})"))?;
  let entry =
    crate::kb::entry::parse_entry(&text).map_err(|e| format!("(parse error: {e:?})"))?;
  // 差異概要の文字数は前後の空白を除いて数える（末尾改行で 1 ずれた数字を出さない）。
  Ok((rel, title, entry.body.trim().chars().count()))
}

/// 条目上書き（ブロッキング）。application::overwrite で本文差し替え + 索引更新。
fn update_blocking(root: &Path, rel: &str, body: &str) -> String {
  let conn = match index::open_index(root) {
    Ok(c) => c,
    Err(e) => return format!("(index error: {e:?})"),
  };
  match crate::workshop::application::overwrite(root, &conn, rel, body) {
    Ok(()) => format!("Updated {rel}"),
    Err(e) => format!("(update error: {e:?})"),
  }
}

/// delete_entry の位置決め（ブロッキング・読み取りのみ）。対象の相対パス・タイトル・
/// 被参照元のタイトル一覧（確認カードの断リンク警告用）を返す。
fn delete_prepare(root: &Path, id: &str) -> Result<(String, String, Vec<String>), String> {
  let (rel, title) = resolve_entry_blocking(root, id)?;
  let conn = index::open_index(root).map_err(|e| format!("(index error: {e:?})"))?;
  let backlinks = index::backlinks(&conn, &title)
    .map_err(|e| format!("(index error: {e:?})"))?
    .into_iter()
    .map(|r| r.title)
    .collect();
  Ok((rel, title, backlinks))
}

/// 条目削除（ブロッキング）。kb::delete_entry_in でファイルと索引を原子的に消す。
fn delete_blocking(root: &Path, rel: &str) -> String {
  let mut conn = match index::open_index(root) {
    Ok(c) => c,
    Err(e) => return format!("(index error: {e:?})"),
  };
  match crate::kb::delete_entry_in(root, &mut conn, rel) {
    Ok(()) => format!("Deleted {rel}"),
    Err(e) => format!("(delete error: {e:?})"),
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
  use crate::workshop::infrastructure::web_search::{SearchBackend, WebSearchResult};
  use futures::future::{BoxFuture, FutureExt};
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::{Arc, Mutex};
  use unicode_normalization::UnicodeNormalization;

  struct FakeSearchBackend {
    calls: Arc<AtomicUsize>,
    result: Result<Vec<WebSearchResult>, String>,
  }

  impl SearchBackend for FakeSearchBackend {
    fn search(&self, _query: String) -> BoxFuture<'static, Result<Vec<WebSearchResult>, String>> {
      self.calls.fetch_add(1, Ordering::Relaxed);
      let result = self.result.clone();
      async move { result }.boxed()
    }
  }

  /// 確認要求へ自動応答するゲート（approve = 許可 / 拒否）。確認そのものが主題でないテスト用。
  fn auto_gate(approve: bool) -> Arc<ConfirmGate> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let gate = Arc::new(ConfirmGate {
      pending: Default::default(),
      tx,
      cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });
    let pending = gate.pending.clone();
    tokio::spawn(async move {
      while let Some(event) = rx.recv().await {
        if let crate::agent::StreamProgress::ConfirmRequest { id, .. } = event {
          crate::workshop::infrastructure::confirm::resolve(&pending, id, approve);
        }
      }
    });
    gate
  }

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

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let gate = Arc::new(ConfirmGate {
      pending: Default::default(),
      tx,
      cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });
    let pending = gate.pending.clone();
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

  /// 更新対象の条目をファイル + 索引の両方へ植える（update/delete 系テスト用）。
  fn seed_entry_file(root: &Path, rel: &str, title: &str, body: &str) {
    std::fs::create_dir_all(root.join("entries")).unwrap();
    let content = format!(
      "---\ntype: Entry\ntitle: {title}\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n{body}\n"
    );
    std::fs::write(root.join(rel), content).unwrap();
    let conn = index::open_index(root).unwrap();
    seed_entry(&conn, rel, title, body);
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

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let gate = Arc::new(ConfirmGate {
      pending: Default::default(),
      tx,
      cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });
    let pending = gate.pending.clone();
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

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let gate = Arc::new(ConfirmGate {
      pending: Default::default(),
      tx,
      cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });
    let pending = gate.pending.clone();
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

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let gate = Arc::new(ConfirmGate {
      pending: Default::default(),
      tx,
      cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });
    let pending = gate.pending.clone();
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

  #[tokio::test]
  async fn search_web_rejects_empty_query_without_calling_backend() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tool = SearchWeb {
      backend: Arc::new(FakeSearchBackend { calls: calls.clone(), result: Ok(vec![]) }),
    };

    let out = tool.call(SearchArgs { query: "  ".into() }).await.unwrap();

    assert_eq!(out, "(search_web needs a non-empty query)");
    assert_eq!(calls.load(Ordering::Relaxed), 0);
  }

  #[tokio::test]
  async fn search_web_returns_structured_results() {
    let tool = SearchWeb {
      backend: Arc::new(FakeSearchBackend {
        calls: Arc::new(AtomicUsize::new(0)),
        result: Ok(vec![WebSearchResult {
          title: "ExpertBase".into(),
          url: "https://example.com/expertbase".into(),
          snippet: "Local-first knowledge base".into(),
        }]),
      }),
    };

    let out = tool.call(SearchArgs { query: "ExpertBase".into() }).await.unwrap();
    let results: serde_json::Value = serde_json::from_str(&out).unwrap();

    assert_eq!(results[0]["title"], "ExpertBase");
    assert_eq!(results[0]["url"], "https://example.com/expertbase");
    assert_eq!(results[0]["snippet"], "Local-first knowledge base");
  }

  #[tokio::test]
  async fn search_web_returns_notice_for_empty_results() {
    let tool = SearchWeb {
      backend: Arc::new(FakeSearchBackend {
        calls: Arc::new(AtomicUsize::new(0)),
        result: Ok(vec![]),
      }),
    };

    let out = tool.call(SearchArgs { query: "ExpertBase".into() }).await.unwrap();

    assert_eq!(out, "(no web results)");
  }

  #[tokio::test]
  async fn search_web_returns_backend_errors_without_failing_tool_loop() {
    let tool = SearchWeb {
      backend: Arc::new(FakeSearchBackend {
        calls: Arc::new(AtomicUsize::new(0)),
        result: Err("Brave Search request failed with HTTP 429".into()),
      }),
    };

    let out = tool.call(SearchArgs { query: "ExpertBase".into() }).await.unwrap();

    assert_eq!(out, "(search_web error: Brave Search request failed with HTTP 429)");
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
