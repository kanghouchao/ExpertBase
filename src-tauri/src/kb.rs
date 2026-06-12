use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KbStatus {
  pub root: String,
  pub initialized: bool,
}

/// アプリデータディレクトリ内にナレッジベースのルートを作成する。
pub fn ensure_kb_root(base_dir: &Path) -> std::io::Result<PathBuf> {
  let root = base_dir.join("knowledge-base");
  std::fs::create_dir_all(&root)?;
  Ok(root)
}

#[tauri::command]
pub fn kb_status(app: tauri::AppHandle) -> Result<KbStatus, String> {
  let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
  let root = ensure_kb_root(&base).map_err(|e| e.to_string())?;
  Ok(KbStatus {
    root: root.to_string_lossy().into_owned(),
    initialized: true,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ensure_kb_root_creates_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let root = ensure_kb_root(tmp.path()).unwrap();
    assert!(root.is_dir());
    assert!(root.ends_with("knowledge-base"));
  }

  #[test]
  fn ensure_kb_root_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let first = ensure_kb_root(tmp.path()).unwrap();
    let second = ensure_kb_root(tmp.path()).unwrap();
    assert_eq!(first, second);
  }
}
