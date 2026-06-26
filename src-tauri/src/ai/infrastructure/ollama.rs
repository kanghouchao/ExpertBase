//! Ollama インフラ: モデル発見（/api/version・/api/tags・/api/show）。
//! エージェントのループ/ストリーミング/ツールは Rig（workshop::infrastructure）が担うので、
//! ここは「どのモデルが居て、thinking / tools 能力を持つか」を答えるだけの薄い HTTP 層。

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ai::AiError;

const API_BASE: &str = "http://127.0.0.1:11434";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
  pub name: String,
  /// thinking（推論トレース）能力を持つか。/api/show の capabilities で判定。
  pub thinking: bool,
  /// tools（関数呼び出し）能力を持つか。これが true のモデルだけ KB 操作ツールに乗せる。
  pub tools: bool,
}

/// Ollama が起動しているか（/api/version への短時間 ping）。
pub fn available() -> bool {
  reqwest::blocking::Client::builder()
    .timeout(Duration::from_millis(800))
    .build()
    .and_then(|client| client.get(format!("{API_BASE}/api/version")).send())
    .map(|resp| resp.status().is_success())
    .unwrap_or(false)
}

/// ローカルのモデル一覧を返す（各モデルの thinking / tools 能力を /api/show で補う）。
pub fn list_models() -> Result<Vec<OllamaModel>, AiError> {
  let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(3))
    .build()
    .map_err(|e| AiError::Network(e.to_string()))?;
  let resp = client
    .get(format!("{API_BASE}/api/tags"))
    .send()
    .map_err(|e| AiError::Network(e.to_string()))?;
  let status = resp.status();
  let text = resp.text().map_err(|e| AiError::Network(e.to_string()))?;
  if status.as_u16() != 200 {
    return Err(AiError::Other(format!("Ollama 模型列表读取失败({status}): {text}")));
  }
  let mut models = parse_models_response(&text)?;
  // 各モデルの thinking / tools 能力を /api/show で補う（ローカル・高速、1 リクエストで両方）。
  for model in &mut models {
    if let Ok(show) = client
      .post(format!("{API_BASE}/api/show"))
      .json(&json!({ "model": model.name }))
      .send()
      .and_then(|r| r.text())
    {
      model.thinking = show_supports_thinking(&show);
      model.tools = show_supports_tools(&show);
    }
  }
  Ok(models)
}

#[derive(Deserialize)]
struct TagsResponse {
  models: Vec<TagModel>,
}

#[derive(Deserialize)]
struct TagModel {
  name: String,
}

fn parse_models_response(body: &str) -> Result<Vec<OllamaModel>, AiError> {
  let tags: TagsResponse = serde_json::from_str(body).map_err(|e| AiError::Other(e.to_string()))?;
  Ok(tags
    .models
    .into_iter()
    .filter(|model| !model.name.trim().is_empty())
    .map(|model| OllamaModel { name: model.name, thinking: false, tools: false })
    .collect())
}

#[derive(Deserialize)]
struct ShowResponse {
  #[serde(default)]
  capabilities: Vec<String>,
}

/// /api/show のレスポンスから thinking 能力の有無を読む。
fn show_supports_thinking(body: &str) -> bool {
  show_has_capability(body, "thinking")
}

/// /api/show のレスポンスから tools（関数呼び出し）能力の有無を読む。
fn show_supports_tools(body: &str) -> bool {
  show_has_capability(body, "tools")
}

fn show_has_capability(body: &str, cap: &str) -> bool {
  serde_json::from_str::<ShowResponse>(body)
    .map(|s| s.capabilities.iter().any(|c| c == cap))
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn show_supports_thinking_reads_capabilities() {
    let yes = r#"{"capabilities":["completion","thinking"]}"#;
    let no = r#"{"capabilities":["completion"]}"#;
    assert!(show_supports_thinking(yes));
    assert!(!show_supports_thinking(no));
    assert!(!show_supports_thinking("{}"));
  }

  #[test]
  fn show_supports_tools_reads_capabilities() {
    let yes = r#"{"capabilities":["completion","tools"]}"#;
    let no = r#"{"capabilities":["completion","thinking"]}"#;
    assert!(show_supports_tools(yes));
    assert!(!show_supports_tools(no));
    assert!(!show_supports_tools("{}"));
  }

  #[test]
  fn parse_models_response_returns_downloaded_model_names() {
    let sample = r#"{
      "models": [
        {"name": "qwen3:8b", "modified_at": "2026-06-14T00:00:00Z"},
        {"name": "llama3.1:8b", "modified_at": "2026-06-14T00:00:00Z"}
      ]
    }"#;
    let models = parse_models_response(sample).unwrap();
    assert_eq!(
      models,
      vec![
        OllamaModel { name: "qwen3:8b".into(), thinking: false, tools: false },
        OllamaModel { name: "llama3.1:8b".into(), thinking: false, tools: false },
      ]
    );
  }
}
