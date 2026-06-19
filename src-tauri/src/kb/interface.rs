//! kb インターフェイス層。Tauri コマンド（IPC アダプタ）。
//! IPC 入力を解析し、アプリケーション/インフラの関数を呼び、IPC 出力を整形するだけ。

use serde::Serialize;
use tauri::Manager;

use crate::kb::application;
use crate::kb::domain::registry::KbEntry;
use crate::kb::infrastructure::{config_store, index};

/// `kb_list` の応答。フロントの初期化ウィザード向けに既定の親ディレクトリも返す。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KbList {
  pub kbs: Vec<KbEntry>,
  pub active: Option<String>,
  pub default_parent: String,
}

#[tauri::command]
pub fn kb_list(app: tauri::AppHandle) -> Result<KbList, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let registry = config_store::load_registry(&home)?;
  Ok(KbList {
    kbs: registry.knowledge_bases,
    active: registry.active,
    default_parent: home.join("ExpertBase").to_string_lossy().into_owned(),
  })
}

#[tauri::command]
pub fn kb_create(
  app: tauri::AppHandle,
  name: String,
  description: String,
  path: String,
) -> Result<KbEntry, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::create_kb(&home, &name, &description, &path)
}

#[tauri::command]
pub fn kb_set_active(app: tauri::AppHandle, path: String) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::set_active(&home, &path)
}

#[tauri::command]
pub fn kb_rebuild_index(app: tauri::AppHandle) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = application::open_active(&home)?;
  index::rebuild(&conn, &root)
}

#[tauri::command]
pub fn kb_list_entries(app: tauri::AppHandle) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = application::open_active(&home)?;
  index::list_entries(&conn)
}

#[tauri::command]
pub fn kb_search(app: tauri::AppHandle, query: String) -> Result<Vec<index::SearchHit>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = application::open_active(&home)?;
  index::search(&conn, &query)
}

#[tauri::command]
pub fn kb_backlinks(app: tauri::AppHandle, title: String) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = application::open_active(&home)?;
  index::backlinks(&conn, &title)
}

#[tauri::command]
pub fn kb_stats(app: tauri::AppHandle) -> Result<index::Stats, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = application::open_active(&home)?;
  index::stats(&conn)
}

#[tauri::command]
pub fn kb_graph(app: tauri::AppHandle) -> Result<index::GraphData, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = application::open_active(&home)?;
  index::graph(&conn)
}

#[tauri::command]
pub fn kb_orphans(app: tauri::AppHandle) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = application::open_active(&home)?;
  index::orphans(&conn)
}

#[tauri::command]
pub fn kb_read_entry(app: tauri::AppHandle, path: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::read_entry(&home, &path)
}

#[tauri::command]
pub fn kb_read_inbox_material(app: tauri::AppHandle, path: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::read_inbox_material(&home, &path)
}

#[tauri::command]
pub fn kb_save_entry(app: tauri::AppHandle, path: String, content: String) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::save_entry(&home, &path, &content)
}

#[tauri::command]
pub fn kb_delete_inbox_material(app: tauri::AppHandle, path: String) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  application::delete_inbox_material(&home, &path)
}

#[tauri::command]
pub fn kb_list_inbox(app: tauri::AppHandle) -> Result<Vec<index::InboxItem>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = application::open_active(&home)?;
  index::list_inbox(&conn)
}
