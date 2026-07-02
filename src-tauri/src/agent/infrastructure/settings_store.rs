//! agent インフラ: AI 設定の永続化（`~/.expertBase/ai.toml`）。
//! kb の config_store と同じ toml + ホーム直下方式。欠落時は既定値を返す。

use std::fs;
use std::path::{Path, PathBuf};

use crate::agent::domain::AiSettings;
use crate::error::AppError;

/// グローバル設定ディレクトリ（ユーザーホーム直下、kb と共用）。
const CONFIG_DIR: &str = ".expertBase";
/// AI 設定ファイル名。
const SETTINGS_FILE: &str = "ai.toml";

fn settings_path(home: &Path) -> PathBuf {
  home.join(CONFIG_DIR).join(SETTINGS_FILE)
}

/// AI 設定を読み込む。ファイルが無ければ既定値（Ollama）を返す。
pub fn load(home: &Path) -> Result<AiSettings, AppError> {
  let path = settings_path(home);
  if !path.exists() {
    return Ok(AiSettings::default());
  }
  let text = fs::read_to_string(&path).map_err(AppError::generic)?;
  toml::from_str(&text).map_err(AppError::generic)
}

/// AI 設定を書き込む。`.expertBase` ディレクトリが無ければ作成する。
/// brave_api_key（秘密情報）を含むため、unix では所有者のみ読み書き（0600）へ絞る。
pub fn save(home: &Path, settings: &AiSettings) -> Result<(), AppError> {
  let path = settings_path(home);
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir).map_err(AppError::generic)?;
  }
  let text = toml::to_string_pretty(settings).map_err(AppError::generic)?;
  fs::write(&path, text).map_err(AppError::generic)?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(AppError::generic)?;
  }
  Ok(())
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
      brave_api_key: "brave-secret".into(),
    };
    save(tmp.path(), &settings).unwrap();
    assert_eq!(load(tmp.path()).unwrap(), settings);
  }

  #[cfg(unix)]
  #[test]
  fn save_restricts_file_permissions_to_owner() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();

    save(tmp.path(), &AiSettings::default()).unwrap();

    let mode = fs::metadata(settings_path(tmp.path())).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
  }

  #[test]
  fn save_then_load_round_trips_brave_api_key() {
    let tmp = tempfile::tempdir().unwrap();
    let settings: AiSettings = toml::from_str(
      "provider = \"ollama\"\nmodel = \"qwen3:8b\"\nbraveApiKey = \"brave-secret\"\n",
    )
    .unwrap();

    save(tmp.path(), &settings).unwrap();
    let saved = serde_json::to_value(load(tmp.path()).unwrap()).unwrap();

    assert_eq!(saved["braveApiKey"], "brave-secret");
  }
}
