//! workshop インターフェイス層。Tauri コマンド（IPC アダプタ）。

use tauri::Manager;

use crate::ai::{ChatTurn, StructureResult};
use crate::kb::material;
use crate::workshop::application;

/// 複数の受信箱素材 + 会話履歴から AI 構造化（草稿 or 会話返信）を生成する。
/// 各素材の本文を区切り線で連結し、1 つの source として加工する。会話の記憶は
/// フロントが組み立てた messages（多輪）で渡る。
#[tauri::command]
pub fn workshop_draft(
  app: tauri::AppHandle,
  inbox_paths: Vec<String>,
  messages: Vec<ChatTurn>,
  model: String,
) -> Result<StructureResult, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = crate::kb::open_active(&home)?;
  let mut bodies = Vec::with_capacity(inbox_paths.len());
  for inbox_path in &inbox_paths {
    let inbox_rel = crate::kb::checked_kb_markdown_path(inbox_path, "inbox")?;
    let raw = std::fs::read_to_string(root.join(inbox_rel)).map_err(|e| e.to_string())?;
    bodies.push(material::parse_material(&raw)?.body);
  }
  let source_text = bodies.join("\n\n---\n\n");
  let provider = crate::ai::ollama::OllamaProvider::with_model(model);
  application::draft(&provider, &conn, &source_text, messages, &mut |_| {}).map_err(|e| e.to_string())
}

/// 承認内容を条目として確定する（UI で手編集済みの値を受け取る）。
/// source の受信箱は全て processed になる。
#[tauri::command]
pub fn workshop_confirm(
  app: tauri::AppHandle,
  inbox_paths: Vec<String>,
  title: String,
  cat: String,
  body: String,
) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = crate::kb::open_active(&home)?;
  let mut inbox_rels = Vec::with_capacity(inbox_paths.len());
  for inbox_path in &inbox_paths {
    let rel = crate::kb::checked_kb_markdown_path(inbox_path, "inbox")?;
    inbox_rels.push(rel.to_string_lossy().to_string());
  }
  application::confirm(&root, &conn, &title, &cat, &body, &inbox_rels)
}
