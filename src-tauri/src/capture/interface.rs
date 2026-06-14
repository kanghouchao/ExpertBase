//! capture インターフェイス層。Tauri コマンド（IPC アダプタ）。

use tauri::Manager;

use crate::capture::application;

/// テキスト/Markdown の貼り付けを受信箱へ取り込む。
#[tauri::command]
pub fn capture_text(
  app: tauri::AppHandle,
  content: String,
  source: String,
) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::ingest_text(&home, &content, &source)
}

/// ローカルファイルを受信箱へ取り込む。
#[tauri::command]
pub fn capture_file(app: tauri::AppHandle, path: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::ingest_file(&home, &path)
}

/// Web ページを取り込む。
#[tauri::command]
pub async fn capture_web(app: tauri::AppHandle, url: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::ingest_web(&home, &url).await
}
