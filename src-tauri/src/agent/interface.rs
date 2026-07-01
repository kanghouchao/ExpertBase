//! agent インターフェイス層。Tauri コマンド（IPC アダプタ）。

use tauri::Manager;

use crate::agent::domain::AiSettings;
use crate::agent::infrastructure::{ollama, openai_compat, settings_store};
use crate::agent::{resolve_base_url, Provider};

/// 保存済み設定を読む共通処理（ホーム直下 ai.toml、欠落時は既定）。
fn load_settings(app: &tauri::AppHandle) -> Result<AiSettings, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  settings_store::load(&home)
}

/// Ollama が起動しているか（設定の ollama_url、空欄は既定へ解決）。
#[tauri::command]
pub fn ai_has_key(app: tauri::AppHandle) -> Result<bool, String> {
  let settings = load_settings(&app)?;
  let base = resolve_base_url(Provider::Ollama, &settings.ollama_url);
  Ok(ollama::available(&base))
}

/// Ollama のモデル一覧（設定の ollama_url を使う。workshop の作曲欄が使う）。
#[tauri::command]
pub fn ai_list_ollama_models(app: tauri::AppHandle) -> Result<Vec<ollama::OllamaModel>, String> {
  let settings = load_settings(&app)?;
  let base = resolve_base_url(Provider::Ollama, &settings.ollama_url);
  ollama::list_models(&base).map_err(|e| e.to_string())
}

/// 指定 provider + URL のモデル一覧（設定画面の「検証」用。URL は未保存の入力値をそのまま受ける）。
/// 空欄なら provider 既定へ解決。成功＝端点が生きていて列挙できた＝検証 OK。
#[tauri::command]
pub fn ai_list_models(provider: Provider, base_url: String) -> Result<Vec<ollama::OllamaModel>, String> {
  let base = resolve_base_url(provider, &base_url);
  match provider {
    Provider::Ollama => ollama::list_models(&base).map_err(|e| e.to_string()),
    Provider::LlamaApp => openai_compat::list_models(&base).map_err(|e| e.to_string()),
  }
}

/// 保存済みの AI 設定を読む（無ければ既定値）。
#[tauri::command]
pub fn ai_get_settings(app: tauri::AppHandle) -> Result<AiSettings, String> {
  load_settings(&app)
}

/// AI 設定を保存する。
#[tauri::command]
pub fn ai_set_settings(app: tauri::AppHandle, settings: AiSettings) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  settings_store::save(&home, &settings)
}
