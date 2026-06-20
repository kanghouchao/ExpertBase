use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ai::domain::{AiError, AiProvider, StructureRequest, StructureResult};

/// kind が欠落したモデル出力への安全な既定値（信頼境界の堅牢化）。
fn default_kind() -> String {
  "entry".to_string()
}

const API_BASE: &str = "http://127.0.0.1:11434";

/// Ollama に渡す system プロンプト。出力は JSON スキーマで固定する。
const SYSTEM_PROMPT: &str = r###"You are a knowledge base editor chatting with the user about the provided Material. Every reply MUST be a single JSON object with the fields below.

Fields:
- kind: "entry" or "chat". Use "entry" when the user asks you to produce or revise a knowledge entry from the Material. Use "chat" when the user is greeting, asking a question, or just talking and has not asked for an entry yet.
- title: a concise heading; empty string when kind is "chat".
- cat: a category as a short lowercase English word, e.g. tea, finance, privacy; empty string when kind is "chat".
- body_markdown: for "entry", the reorganized entry body in valid Markdown; for "chat", your conversational reply.
- suggested_links: for "entry", titles chosen ONLY from the Related existing entries that are genuinely relevant; [] when none are relevant or the list is empty. Always [] for "chat".

Example (chat)
User: Hi, what is this material about?
Output:
{"kind":"chat","title":"","cat":"","body_markdown":"It is an interview with a tea master about pan-firing temperature. Want me to turn it into an entry?","suggested_links":[]}

Example (entry)
User: Organize this into a clean entry.
Material: Oolong tea is semi-oxidized. Steep at 90-95C for about one minute; it can be re-steeped several times.
Related existing entries:
- Green tea brewing: steeping notes for green tea
Output:
{"kind":"entry","title":"Oolong tea brewing","cat":"tea","body_markdown":"## Overview\nOolong is a semi-oxidized tea.\n\n## Brewing\n- Water: 90-95C\n- Time: about 1 minute\n- Can be re-steeped several times","suggested_links":["Green tea brewing"]}

Do not:
- Do not add facts that are not supported by the Material.
- Do not output anything outside the JSON object (no explanations, no code fences).
- Do not link unrelated entries just to fill suggested_links; use [] instead.
- Do not leave broken Markdown such as unmatched ** markers.

Reply with JSON only."###;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
  pub name: String,
}

pub struct OllamaProvider {
  model: Option<String>,
  base_url: String,
}

impl OllamaProvider {
  pub fn new() -> Self {
    let model = std::env::var("EXPERTBASE_OLLAMA_MODEL")
      .ok()
      .filter(|s| !s.trim().is_empty())
      .map(|s| s.trim().to_string());
    Self { model, base_url: API_BASE.to_string() }
  }

  pub fn with_model(model: String) -> Self {
    let selected = model.trim();
    if selected.is_empty() {
      Self::new()
    } else {
      Self { model: Some(selected.to_string()), base_url: API_BASE.to_string() }
    }
  }

  pub fn available() -> bool {
    reqwest::blocking::Client::builder()
      .timeout(Duration::from_millis(800))
      .build()
      .and_then(|client| client.get(format!("{API_BASE}/api/version")).send())
      .map(|resp| resp.status().is_success())
      .unwrap_or(false)
  }

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
    match status.as_u16() {
      200 => parse_models_response(&text),
      _ => Err(AiError::Other(format!("Ollama 模型列表读取失败({status}): {text}"))),
    }
  }

  pub fn first_local_model() -> Result<Option<String>, AiError> {
    Ok(Self::list_models()?.into_iter().next().map(|model| model.name))
  }
}

fn output_schema() -> Value {
  json!({
    "type": "object",
    "properties": {
      "kind": { "type": "string", "enum": ["entry", "chat"] },
      "title": { "type": "string" },
      "cat": { "type": "string" },
      "body_markdown": { "type": "string" },
      "suggested_links": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["kind", "title", "cat", "body_markdown", "suggested_links"],
    "additionalProperties": false
  })
}

/// 素材と関連条目は会話全体で固定の文脈。system メッセージに pin する。
fn system_content(req: &StructureRequest) -> String {
  let mut related = String::new();
  for e in &req.related {
    related.push_str(&format!("- {}: {}\n", e.title, e.excerpt));
  }
  if related.is_empty() {
    related.push_str("(none)\n");
  }
  format!(
    "{SYSTEM_PROMPT}\n\n# Material\n{}\n\n# Related existing entries\n{related}",
    req.source_text
  )
}

/// /api/chat 用メッセージ列: 先頭に system（指示 + 固定文脈）、続けて会話履歴。
fn chat_messages(req: &StructureRequest) -> Value {
  let mut messages = vec![json!({ "role": "system", "content": system_content(req) })];
  for turn in &req.messages {
    messages.push(json!({ "role": turn.role, "content": turn.content }));
  }
  Value::Array(messages)
}

fn build_body(model: &str, req: &StructureRequest) -> Value {
  json!({
    "model": model,
    "messages": chat_messages(req),
    "stream": false,
    "format": output_schema(),
    "options": {
      "temperature": 0.2
    }
  })
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
    .map(|model| OllamaModel { name: model.name })
    .collect())
}

#[derive(Deserialize)]
struct ChatResponse {
  message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
  content: String,
}

#[derive(Deserialize)]
struct RawResult {
  #[serde(default = "default_kind")]
  kind: String,
  title: String,
  cat: String,
  body_markdown: String,
  suggested_links: Vec<String>,
}

fn parse_response(body: &str) -> Result<StructureResult, AiError> {
  let chat: ChatResponse =
    serde_json::from_str(body).map_err(|e| AiError::Other(e.to_string()))?;
  let raw: RawResult =
    serde_json::from_str(&chat.message.content).map_err(|e| AiError::Other(e.to_string()))?;
  Ok(StructureResult {
    kind: raw.kind,
    title: raw.title,
    cat: raw.cat,
    body_markdown: raw.body_markdown,
    suggested_links: raw.suggested_links,
  })
}

impl AiProvider for OllamaProvider {
  fn structure(&self, req: StructureRequest) -> Result<StructureResult, AiError> {
    let model = match &self.model {
      Some(model) => model.clone(),
      None => Self::first_local_model()?
        .ok_or_else(|| AiError::Other("Ollama 没有可用模型，请先下载模型".into()))?,
    };
    let body = build_body(&model, &req);
    let client = reqwest::blocking::Client::builder()
      .timeout(Duration::from_secs(120))
      .build()
      .map_err(|e| AiError::Network(e.to_string()))?;
    let resp = client
      .post(format!("{}/api/chat", self.base_url))
      .header("content-type", "application/json")
      .json(&body)
      .send()
      .map_err(|e| AiError::Network(e.to_string()))?;

    let status = resp.status();
    let text = resp.text().map_err(|e| AiError::Network(e.to_string()))?;
    match status.as_u16() {
      200 => parse_response(&text),
      404 => Err(AiError::Other(format!("Ollama 模型未找到: {model}"))),
      _ => Err(AiError::Other(format!("Ollama API 错误({status}): {text}"))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ai::{ChatTurn, EntrySummary};

  fn req() -> StructureRequest {
    StructureRequest {
      source_text: "緑茶の淹れ方".into(),
      related: vec![EntrySummary { title: "煎茶".into(), excerpt: "茶葉".into() }],
      messages: vec![ChatTurn { role: "user".into(), content: "整理して".into() }],
    }
  }

  #[test]
  fn build_body_uses_ollama_chat_contract() {
    let body = build_body("llama3.2", &req());
    assert_eq!(body["model"], "llama3.2");
    assert_eq!(body["stream"], false);
    assert_eq!(body["format"]["type"], "object");
    let messages = body["messages"].as_array().unwrap();
    // 先頭は system（指示 + 固定文脈）、続けて会話履歴。
    assert_eq!(messages[0]["role"], "system");
    assert!(messages[0]["content"].as_str().unwrap().contains("緑茶の淹れ方"));
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "整理して");
  }

  #[test]
  fn system_prompt_handles_chat_kind_and_relevant_links() {
    // kind（entry/chat）の判別・例示・関連性ルール・禁止事項を含む
    let system =
      build_body("llama3.2", &req())["messages"][0]["content"].as_str().unwrap().to_lowercase();
    assert!(system.contains("kind"));
    assert!(system.contains("chat"));
    assert!(system.contains("entry"));
    assert!(system.contains("example"));
    assert!(system.contains("relevant"));
    assert!(system.contains("do not"));
  }

  #[test]
  fn with_model_preserves_explicit_model_name() {
    let provider = OllamaProvider::with_model("llama3.2".into());
    assert_eq!(provider.model.as_deref(), Some("llama3.2"));
  }

  #[test]
  fn parse_response_extracts_entry_result() {
    let sample = r#"{"message":{"content":"{\"kind\":\"entry\",\"title\":\"緑茶\",\"cat\":\"tea\",\"body_markdown\":\"本文\",\"suggested_links\":[\"煎茶\"]}"}}"#;
    let result = parse_response(sample).unwrap();
    assert_eq!(result.kind, "entry");
    assert_eq!(result.title, "緑茶");
    assert_eq!(result.suggested_links, vec!["煎茶".to_string()]);
  }

  #[test]
  fn parse_response_extracts_chat_reply() {
    let sample = r#"{"message":{"content":"{\"kind\":\"chat\",\"title\":\"\",\"cat\":\"\",\"body_markdown\":\"こんにちは\",\"suggested_links\":[]}"}}"#;
    let result = parse_response(sample).unwrap();
    assert_eq!(result.kind, "chat");
    assert_eq!(result.body_markdown, "こんにちは");
    assert!(result.suggested_links.is_empty());
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
        OllamaModel { name: "qwen3:8b".into() },
        OllamaModel { name: "llama3.1:8b".into() },
      ]
    );
  }
}
