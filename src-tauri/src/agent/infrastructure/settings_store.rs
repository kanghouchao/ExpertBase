//! agent インフラ: AI 設定の永続化（`~/.expertBase/ai.toml`）。
//! kb の config_store と同じ toml + ホーム直下方式。欠落時は既定値を返す。

use std::fs;
use std::path::{Path, PathBuf};

use crate::agent::domain::AiSettings;

/// グローバル設定ディレクトリ（ユーザーホーム直下、kb と共用）。
const CONFIG_DIR: &str = ".expertBase";
/// AI 設定ファイル名。
const SETTINGS_FILE: &str = "ai.toml";

fn settings_path(home: &Path) -> PathBuf {
  home.join(CONFIG_DIR).join(SETTINGS_FILE)
}

/// AI 設定を読み込む。ファイルが無ければ既定値（Ollama）を返す。
pub fn load(home: &Path) -> Result<AiSettings, String> {
  let path = settings_path(home);
  if !path.exists() {
    return Ok(AiSettings::default());
  }
  let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  toml::from_str(&text).map_err(|e| e.to_string())
}

/// AI 設定を書き込む。`.expertBase` ディレクトリが無ければ作成する。
pub fn save(home: &Path, settings: &AiSettings) -> Result<(), String> {
  let path = settings_path(home);
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
  }
  let text = toml::to_string_pretty(settings).map_err(|e| e.to_string())?;
  fs::write(path, text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::agent::Provider;

  #[test]
  fn load_returns_default_when_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let settings = load(tmp.path()).unwrap();
    assert_eq!(settings, AiSettings::default());
    assert_eq!(settings.provider, Provider::Ollama);
  }

  #[test]
  fn save_then_load_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let settings = AiSettings {
      provider: Provider::LlamaApp,
      model: "qwen2.5".into(),
      ollama_url: "http://127.0.0.1:11434".into(),
      llama_app_url: "http://127.0.0.1:8080/v1".into(),
    };
    save(tmp.path(), &settings).unwrap();
    assert_eq!(load(tmp.path()).unwrap(), settings);
  }
}
