//! ai インターフェイス層。Tauri コマンド（IPC アダプタ）。

use crate::agent::infrastructure::ollama;

#[tauri::command]
pub fn ai_has_key() -> Result<bool, String> {
  Ok(ollama::available())
}

#[tauri::command]
pub fn ai_list_ollama_models() -> Result<Vec<ollama::OllamaModel>, String> {
  ollama::list_models().map_err(|e| e.to_string())
}
