use std::path::Path;

use serde::Serialize;
use tauri::Manager;

pub mod ollama;

/// API キーの保存場所（ユーザーホーム直下、ドット始まりの設定ディレクトリ内）。
fn key_path(home: &Path) -> std::path::PathBuf {
  home.join(".expertBase").join("anthropic.key")
}

/// BYO API キーを保存する。UI はキーを保持せず、Rust 側にのみ置く。
pub fn set_api_key(home: &Path, key: &str) -> Result<(), String> {
  let path = key_path(home);
  std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
  std::fs::write(path, key.trim()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_set_key(app: tauri::AppHandle, key: String) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  set_api_key(&home, &key)
}

#[tauri::command]
pub fn ai_has_key(app: tauri::AppHandle) -> Result<bool, String> {
  let _home = app.path().home_dir().map_err(|e| e.to_string())?;
  Ok(ollama::OllamaProvider::available())
}

#[tauri::command]
pub fn ai_list_ollama_models(
  app: tauri::AppHandle,
) -> Result<Vec<ollama::OllamaModel>, String> {
  let _home = app.path().home_dir().map_err(|e| e.to_string())?;
  ollama::OllamaProvider::list_models().map_err(|e| e.to_string())
}

/// FTS で引いた関連既存条目の要約（title + excerpt）。
#[derive(Clone, Debug)]
pub struct EntrySummary {
  pub title: String,
  pub excerpt: String,
}

/// 構造化リクエスト（ワークショップが組み立てる）。
#[derive(Clone, Debug)]
pub struct StructureRequest {
  /// 新素材の本文（文字を持つもののみ）。
  pub source_text: String,
  /// FTS で引いた関連既存条目。
  pub related: Vec<EntrySummary>,
  /// ユーザーの指示文。
  pub instruction: String,
}

/// 構造化結果。
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StructureResult {
  pub title: String,
  pub cat: String,
  pub body_markdown: String,
  /// 既存条目タイトルから選ばれたリンク候補。
  pub suggested_links: Vec<String>,
}

/// AI エラー。UI で区別して表示し、手動パスへ退避できるようにする。
#[derive(Debug, PartialEq)]
pub enum AiError {
  /// ネットワーク障害。
  Network(String),
  /// その他（API エラー応答・解析失敗など）。
  Other(String),
}

impl std::fmt::Display for AiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AiError::Network(m) => write!(f, "网络错误: {m}"),
      AiError::Other(m) => write!(f, "{m}"),
    }
  }
}

/// AI プロバイダ接合面。ワークショップはこの trait の裏でのみ AI を呼ぶ。
/// 将来のローカル LLM / マルチモーダルは別実装として差し込む（下流は変更不要）。
pub trait AiProvider {
  fn structure(&self, req: StructureRequest) -> Result<StructureResult, AiError>;
}

/// テスト用の決定的プロバイダ（ネットワーク不要）。
#[cfg(test)]
pub struct FakeProvider;

#[cfg(test)]
impl AiProvider for FakeProvider {
  fn structure(&self, req: StructureRequest) -> Result<StructureResult, AiError> {
    let title = req.source_text.lines().next().unwrap_or("").trim().to_string();
    let suggested_links = req.related.iter().take(3).map(|e| e.title.clone()).collect();
    Ok(StructureResult {
      title: if title.is_empty() { "無題".into() } else { title },
      cat: "uncategorized".into(),
      body_markdown: req.source_text.clone(),
      suggested_links,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fake_provider_echoes_and_suggests_related_links() {
    let req = StructureRequest {
      source_text: "緑茶の淹れ方\n\n本文".into(),
      related: vec![EntrySummary { title: "煎茶".into(), excerpt: "...".into() }],
      instruction: "整理して".into(),
    };
    let res = FakeProvider.structure(req).unwrap();
    assert_eq!(res.title, "緑茶の淹れ方");
    assert_eq!(res.suggested_links, vec!["煎茶".to_string()]);
  }

  #[test]
  fn ai_error_displays_messages() {
    assert_eq!(AiError::Other("x".into()).to_string(), "x");
  }
}
