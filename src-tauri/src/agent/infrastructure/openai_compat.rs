//! OpenAI 互換ローカル端点（llama.app = llama.cpp `llama serve` 等）のモデル発見。
//! `GET {base_url}/models`（base_url は末尾 `/v1` まで含む前提）で id 一覧を得るだけの薄い HTTP 層。
//! Ollama と違い能力（tools/thinking）は返らないので、tools=true / thinking=false 固定で返し、
//! 「どのモデルを使うか」は利用者に委ねる（工作坊は tools 対応モデルを前提とするため）。

use std::time::Duration;

use serde::Deserialize;

use super::ollama::OllamaModel;
use crate::error::AppError;

/// モデル一覧を返す。base_url は解決済み（例: `http://127.0.0.1:8080/v1`）。
pub fn list_models(base_url: &str) -> Result<Vec<OllamaModel>, AppError> {
  let client = reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(3))
    .build()
    .map_err(|e| AppError::param("err.agent.network", "detail", e))?;
  let resp = client
    .get(format!("{base_url}/models"))
    .send()
    .map_err(|e| AppError::param("err.agent.network", "detail", e))?;
  let status = resp.status();
  let text = resp.text().map_err(|e| AppError::param("err.agent.network", "detail", e))?;
  if status.as_u16() != 200 {
    return Err(AppError::params(
      "err.agent.modelListFailed",
      [("status", status.to_string()), ("detail", text)],
    ));
  }
  parse_models(&text)
}

#[derive(Deserialize)]
struct ModelsResponse {
  #[serde(default)]
  data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
  id: String,
}

/// OpenAI 互換 `/models` レスポンスから id を取り出す。能力は不明なので tools=true / thinking=false。
fn parse_models(body: &str) -> Result<Vec<OllamaModel>, AppError> {
  let parsed: ModelsResponse = serde_json::from_str(body).map_err(AppError::generic)?;
  Ok(
    parsed
      .data
      .into_iter()
      .filter(|m| !m.id.trim().is_empty())
      .map(|m| OllamaModel { name: m.id, thinking: false, tools: true })
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_models_reads_ids_as_tool_capable() {
    // llama.cpp/OpenAI 互換の典型レスポンス。id だけ取り、tools=true 扱い。
    let sample = r#"{
      "object": "list",
      "data": [
        {"id": "qwen2.5-7b-instruct", "object": "model"},
        {"id": "llama-3.1-8b", "object": "model"}
      ]
    }"#;
    let models = parse_models(sample).unwrap();
    assert_eq!(
      models,
      vec![
        OllamaModel { name: "qwen2.5-7b-instruct".into(), thinking: false, tools: true },
        OllamaModel { name: "llama-3.1-8b".into(), thinking: false, tools: true },
      ]
    );
  }

  #[test]
  fn parse_models_tolerates_missing_data() {
    // data 欠落でも空リストで返す（壊れた/最小レスポンスに耐える）。
    assert_eq!(parse_models("{}").unwrap(), vec![]);
  }
}
