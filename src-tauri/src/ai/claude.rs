use serde::Deserialize;
use serde_json::{json, Value};

use super::{AiError, AiProvider, StructureRequest, StructureResult};

/// 既定モデル。最新かつ最も高性能な Claude を使う。
const DEFAULT_MODEL: &str = "claude-opus-4-8";
const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

/// Anthropic API を直叩きする本番プロバイダ（BYO-key）。
pub struct ClaudeProvider {
  api_key: String,
  model: String,
}

impl ClaudeProvider {
  pub fn new(api_key: String) -> Self {
    Self { api_key, model: DEFAULT_MODEL.to_string() }
  }
}

/// system プロンプト。出力言語は素材に合わせ、リンク候補は既存タイトルから選ばせる。
const SYSTEM_PROMPT: &str = "あなたはナレッジベースの編集者です。与えられた新素材と既存の関連条目を踏まえ、\
構造化された Markdown 条目を作成します。出力は素材と同じ言語で書きます。\
title は簡潔な見出し、cat はカテゴリ（短い英小文字の単語）、\
body_markdown は整理された本文、suggested_links は『既存の関連条目』一覧に実在するタイトルのみから選びます。";

/// StructureResult を強制するための JSON スキーマ（構造化出力）。
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

/// リクエストから user メッセージ本文を組み立てる。
fn user_content(req: &StructureRequest) -> String {
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
    "# 指示\n{instruction}\n\n# 新素材\n{}\n\n# 既存の関連条目\n{related}",
    req.source_text
  )
}

/// Messages API のリクエストボディを構築する（純関数・テスト可能）。
fn build_body(model: &str, req: &StructureRequest) -> Value {
  json!({
    "model": model,
    "max_tokens": 8000,
    "system": SYSTEM_PROMPT,
    "messages": [{ "role": "user", "content": user_content(req) }],
    "output_config": {
      "format": { "type": "json_schema", "schema": output_schema() }
    }
  })
}

#[derive(Deserialize)]
struct RawResult {
  title: String,
  cat: String,
  body_markdown: String,
  suggested_links: Vec<String>,
}

/// API 応答 JSON から StructureResult を取り出す（純関数・テスト可能）。
fn parse_response(body: &str) -> Result<StructureResult, AiError> {
  let value: Value = serde_json::from_str(body).map_err(|e| AiError::Other(e.to_string()))?;
  if value.get("stop_reason").and_then(|s| s.as_str()) == Some("refusal") {
    return Err(AiError::Other("AI が応答を拒否しました".into()));
  }
  // content 配列から最初の text ブロックを取り出す（thinking 等が先行しても拾える）。
  let text = value
    .get("content")
    .and_then(|c| c.as_array())
    .and_then(|blocks| {
      blocks
        .iter()
        .find(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
        .and_then(|b| b.get("text").and_then(|t| t.as_str()))
    })
    .ok_or_else(|| AiError::Other("応答にテキストがありません".into()))?;
  let raw: RawResult = serde_json::from_str(text).map_err(|e| AiError::Other(e.to_string()))?;
  Ok(StructureResult {
    title: raw.title,
    cat: raw.cat,
    body_markdown: raw.body_markdown,
    suggested_links: raw.suggested_links,
  })
}

impl AiProvider for ClaudeProvider {
  fn structure(&self, req: StructureRequest) -> Result<StructureResult, AiError> {
    if self.api_key.trim().is_empty() {
      return Err(AiError::NoKey);
    }
    let body = build_body(&self.model, &req);
    let client = reqwest::blocking::Client::new();
    let resp = client
      .post(API_URL)
      .header("x-api-key", &self.api_key)
      .header("anthropic-version", API_VERSION)
      .header("content-type", "application/json")
      .json(&body)
      .send()
      .map_err(|e| AiError::Network(e.to_string()))?;

    let status = resp.status();
    let text = resp.text().map_err(|e| AiError::Network(e.to_string()))?;
    match status.as_u16() {
      200 => parse_response(&text),
      401 | 403 => Err(AiError::Other("API キーが無効です".into())),
      429 => Err(AiError::RateLimited),
      _ => Err(AiError::Other(format!("API エラー({status}): {text}"))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ai::EntrySummary;

  fn req() -> StructureRequest {
    StructureRequest {
      source_text: "緑茶の淹れ方の素材".into(),
      related: vec![EntrySummary { title: "煎茶".into(), excerpt: "茶葉の一種".into() }],
      instruction: "整理して".into(),
    }
  }

  #[test]
  fn build_body_includes_model_schema_and_inputs() {
    let body = build_body("claude-opus-4-8", &req());
    assert_eq!(body["model"], "claude-opus-4-8");
    assert_eq!(body["output_config"]["format"]["type"], "json_schema");
    let content = body["messages"][0]["content"].as_str().unwrap();
    assert!(content.contains("緑茶の淹れ方の素材"));
    assert!(content.contains("煎茶"));
    assert!(content.contains("整理して"));
  }

  #[test]
  fn parse_response_extracts_structure_result() {
    let sample = r#"{
      "stop_reason": "end_turn",
      "content": [
        {"type": "text", "text": "{\"title\":\"緑茶の淹れ方\",\"cat\":\"tea\",\"body_markdown\":\"湯温は70度\",\"suggested_links\":[\"煎茶\"]}"}
      ]
    }"#;
    let result = parse_response(sample).unwrap();
    assert_eq!(result.title, "緑茶の淹れ方");
    assert_eq!(result.cat, "tea");
    assert_eq!(result.suggested_links, vec!["煎茶".to_string()]);
  }

  #[test]
  fn parse_response_errors_on_refusal() {
    let sample = r#"{"stop_reason":"refusal","content":[]}"#;
    assert_eq!(parse_response(sample), Err(AiError::Other("AI が応答を拒否しました".into())));
  }

  #[test]
  fn parse_response_errors_on_malformed_text() {
    let sample = r#"{"stop_reason":"end_turn","content":[{"type":"text","text":"not json"}]}"#;
    assert!(parse_response(sample).is_err());
  }
}
