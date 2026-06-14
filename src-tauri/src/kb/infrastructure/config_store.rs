use std::fs;
use std::path::{Path, PathBuf};

use crate::kb::domain::registry::{KbConfig, Registry};

/// グローバル設定ディレクトリ（ユーザーホーム直下）。
const CONFIG_DIR: &str = ".expertBase";
/// グローバル設定ファイル名。登録済みナレッジベースの一覧を保持する。
const CONFIG_FILE: &str = "config.toml";
/// 各ナレッジベース内の設定ディレクトリ（ドット始まりで一般ユーザーには不可視）。
const KB_DIR: &str = ".expertbase";
/// ナレッジベース個別の設定ファイル名。
const KB_FILE: &str = "kb.toml";

fn config_path(home: &Path) -> PathBuf {
  home.join(CONFIG_DIR).join(CONFIG_FILE)
}

/// ナレッジベース個別設定（kb.toml）の絶対パス。
pub fn kb_config_path(kb_root: &Path) -> PathBuf {
  kb_root.join(KB_DIR).join(KB_FILE)
}

/// グローバル設定を読み込む。ファイルが無ければ空の Registry を返す。
pub fn load_registry(home: &Path) -> Result<Registry, String> {
  let path = config_path(home);
  if !path.exists() {
    return Ok(Registry::default());
  }
  let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  toml::from_str(&text).map_err(|e| e.to_string())
}

/// グローバル設定を書き込む。`.expertBase` ディレクトリが無ければ作成する。
pub fn save_registry(home: &Path, registry: &Registry) -> Result<(), String> {
  let path = config_path(home);
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
  }
  let text = toml::to_string_pretty(registry).map_err(|e| e.to_string())?;
  fs::write(&path, text).map_err(|e| e.to_string())
}

/// 指定 KB ルートに既に kb.toml が存在するか。
pub fn kb_config_exists(kb_root: &Path) -> bool {
  kb_config_path(kb_root).exists()
}

/// KB ルート直下に `.expertbase/kb.toml` を作成する（`.expertbase` が無ければ作る）。
pub fn write_kb_config(kb_root: &Path, config: &KbConfig) -> Result<(), String> {
  let path = kb_config_path(kb_root);
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
  }
  let text = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
  fs::write(path, text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn load_registry_returns_default_when_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = load_registry(tmp.path()).unwrap();
    assert!(registry.knowledge_bases.is_empty());
    assert!(registry.active.is_none());
  }
}
