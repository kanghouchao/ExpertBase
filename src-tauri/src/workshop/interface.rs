//! workshop インターフェイス層。Tauri コマンド（IPC アダプタ）。

use tauri::Manager;

use crate::ai::StructureResult;
use crate::kb::material;
use crate::workshop::application;

/// 受信箱素材 + 指示文から AI 構造化草稿を生成する。
#[tauri::command]
pub fn workshop_draft(
  app: tauri::AppHandle,
  inbox_path: String,
  instruction: String,
  model: String,
) -> Result<StructureResult, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = crate::kb::open_active(&home)?;
  let inbox_rel = crate::kb::checked_kb_markdown_path(&inbox_path, "inbox")?;
  let raw = std::fs::read_to_string(root.join(inbox_rel)).map_err(|e| e.to_string())?;
  let material = material::parse_material(&raw)?;
  let provider = crate::ai::ollama::OllamaProvider::with_model(model);
  application::draft(&provider, &conn, &material.body, &instruction).map_err(|e| e.to_string())
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
  application::confirm(&root, &conn, &title, &cat, &body, &inbox_rel.to_string_lossy())
}
