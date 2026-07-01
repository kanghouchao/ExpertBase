//! agent インターフェイス層。Tauri コマンド（IPC アダプタ）。

use tauri::Manager;

use crate::agent::domain::AiSettings;
use crate::agent::infrastructure::{ollama, settings_store};

#[tauri::command]
pub fn ai_has_key() -> Result<bool, String> {
  Ok(ollama::available())
}

#[tauri::command]
pub fn ai_list_ollama_models() -> Result<Vec<ollama::OllamaModel>, String> {
  ollama::list_models().map_err(|e| e.to_string())
}

/// 保存済みの AI 設定を読む（無ければ既定値）。
#[tauri::command]
pub fn ai_get_settings(app: tauri::AppHandle) -> Result<AiSettings, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  settings_store::load(&home)
}

/// AI 設定を保存する。
#[tauri::command]
pub fn ai_set_settings(app: tauri::AppHandle, settings: AiSettings) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  settings_store::save(&home, &settings)
}
