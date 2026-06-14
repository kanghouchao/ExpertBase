use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use super::{AiError, AiProvider, StructureRequest, StructureResult};

const DEFAULT_MODEL: &str = "llama3.2";
const API_BASE: &str = "http://127.0.0.1:11434";

/// Ollama に渡す system プロンプト。出力は JSON スキーマで固定する。
const SYSTEM_PROMPT: &str = "あなたはナレッジベースの編集者です。与えられた新素材と既存の関連条目を踏まえ、\
構造化された Markdown 条目を作成します。出力は素材と同じ言語で書きます。\
title は簡潔な見出し、cat はカテゴリ（短い英小文字の単語）、\
body_markdown は整理された本文、suggested_links は『既存の関連条目』一覧に実在するタイトルのみから選びます。\
必ず JSON のみを返してください。";

pub struct OllamaProvider {
  model: String,
  base_url: String,
}

impl OllamaProvider {
  pub fn new() -> Self {
    let model = std::env::var("EXPERTBASE_OLLAMA_MODEL")
      .ok()
      .filter(|s| !s.trim().is_empty())
      .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    Self { model, base_url: API_BASE.to_string() }
  }

  pub fn available() -> bool {
    reqwest::blocking::Client::builder()
      .timeout(Duration::from_millis(800))
      .build()
      .and_then(|client| client.get(format!("{API_BASE}/api/version")).send())
      .map(|resp| resp.status().is_success())
      .unwrap_or(false)
  }
}

fn output_schema() -> Value {
  json!({
    "type": "object",
    "properties": {
      "title": { "type": "string" },
      "cat": { "type": "string" },
      "body_markdown": { "type": "string" },
      "suggested_links": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["title", "cat", "body_markdown", "suggested_links"],
    "additionalProperties": false
  })
}

fn prompt(req: &StructureRequest) -> String {
  let instruction = if req.instruction.trim().is_empty() {
    "（特になし）"
  } else {
    req.instruction.trim()
  };
  let mut related = String::new();
  for e in &req.related {
    related.push_str(&format!("- {}: {}\n", e.title, e.excerpt));
  }
  if related.is_empty() {
    related.push_str("（なし）\n");
  }
  format!(
    "# 指示\n{instruction}\n\n# 新素材\n{}\n\n# 既存の関連条目\n{related}\nJSON のみで回答してください。",
    req.source_text
  )
}

fn build_body(model: &str, req: &StructureRequest) -> Value {
  json!({
    "model": model,
    "system": SYSTEM_PROMPT,
    "prompt": prompt(req),
    "stream": false,
    "format": output_schema(),
    "options": {
      "temperature": 0.2
    }
  })
}

#[derive(Deserialize)]
struct GenerateResponse {
  response: String,
}

#[derive(Deserialize)]
struct RawResult {
  title: String,
  cat: String,
  body_markdown: String,
  suggested_links: Vec<String>,
}

fn parse_response(body: &str) -> Result<StructureResult, AiError> {
  let generated: GenerateResponse =
    serde_json::from_str(body).map_err(|e| AiError::Other(e.to_string()))?;
  let raw: RawResult =
    serde_json::from_str(&generated.response).map_err(|e| AiError::Other(e.to_string()))?;
  Ok(StructureResult {
    title: raw.title,
    cat: raw.cat,
    body_markdown: raw.body_markdown,
    suggested_links: raw.suggested_links,
  })
}

impl AiProvider for OllamaProvider {
  fn structure(&self, req: StructureRequest) -> Result<StructureResult, AiError> {
    let body = build_body(&self.model, &req);
    let client = reqwest::blocking::Client::builder()
      .timeout(Duration::from_secs(120))
      .build()
      .map_err(|e| AiError::Network(e.to_string()))?;
    let resp = client
      .post(format!("{}/api/generate", self.base_url))
      .header("content-type", "application/json")
      .json(&body)
      .send()
      .map_err(|e| AiError::Network(e.to_string()))?;

    let status = resp.status();
    let text = resp.text().map_err(|e| AiError::Network(e.to_string()))?;
    match status.as_u16() {
      200 => parse_response(&text),
      404 => Err(AiError::Other(format!("Ollama 模型未找到: {}", self.model))),
      _ => Err(AiError::Other(format!("Ollama API 错误({status}): {text}"))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ai::EntrySummary;

  fn req() -> StructureRequest {
    StructureRequest {
      source_text: "緑茶の淹れ方".into(),
      related: vec![EntrySummary { title: "煎茶".into(), excerpt: "茶葉".into() }],
      instruction: "整理して".into(),
    }
  }

  #[test]
  fn build_body_uses_ollama_generate_contract() {
    let body = build_body("llama3.2", &req());
    assert_eq!(body["model"], "llama3.2");
    assert_eq!(body["stream"], false);
    assert_eq!(body["format"]["type"], "object");
    assert!(body["prompt"].as_str().unwrap().contains("緑茶の淹れ方"));
  }

  #[test]
  fn parse_response_extracts_structure_result() {
    let sample = r#"{"response":"{\"title\":\"緑茶\",\"cat\":\"tea\",\"body_markdown\":\"本文\",\"suggested_links\":[\"煎茶\"]}"}"#;
    let result = parse_response(sample).unwrap();
    assert_eq!(result.title, "緑茶");
    assert_eq!(result.suggested_links, vec!["煎茶".to_string()]);
  }
}
