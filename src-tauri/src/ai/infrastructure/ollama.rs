use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ai::domain::{AiError, AiProvider, StreamProgress, StructureRequest, StructureResult};

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

/// Pass1(思考パス)用プロンプト。JSON を要求せず、推論しながら散文ドラフトを書かせる。
/// format grammar と think を同時に使うと content が壊れるため、思考時は構造化を
/// Pass2 に分離する。ここは"良いドラフト"を出すことだけに集中させる。
const DRAFT_PROMPT: &str = r###"You are a knowledge base editor chatting with the user about the provided Material.
Read the Material and the conversation, think it through, then write the best response in Markdown:
- If the user wants a knowledge entry created or revised, write a clean, well-structured Markdown entry grounded ONLY in the Material.
- Otherwise, reply conversationally.
Do not add facts that are not supported by the Material. Output only the response itself — no JSON, no code fences, no meta commentary."###;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
  pub name: String,
  /// thinking（推論トレース）能力を持つか。/api/show の capabilities で判定。
  pub thinking: bool,
}

pub struct OllamaProvider {
  model: Option<String>,
  base_url: String,
  think: bool,
}

impl OllamaProvider {
  pub fn new() -> Self {
    let model = std::env::var("EXPERTBASE_OLLAMA_MODEL")
      .ok()
      .filter(|s| !s.trim().is_empty())
      .map(|s| s.trim().to_string());
    Self { model, base_url: API_BASE.to_string(), think: false }
  }

  pub fn with_model(model: String) -> Self {
    let selected = model.trim();
    if selected.is_empty() {
      Self::new()
    } else {
      Self { model: Some(selected.to_string()), base_url: API_BASE.to_string(), think: false }
    }
  }

  /// モデル指定 + thinking 有効化。capabilities に thinking がある場合のみ true を渡す。
  pub fn with_model_think(model: String, think: bool) -> Self {
    let mut provider = Self::with_model(model);
    provider.think = think;
    provider
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
    if status.as_u16() != 200 {
      return Err(AiError::Other(format!("Ollama 模型列表读取失败({status}): {text}")));
    }
    let mut models = parse_models_response(&text)?;
    // 各モデルの thinking 能力を /api/show で補う（ローカル・高速）。
    for model in &mut models {
      if let Ok(show) = client
        .post(format!("{API_BASE}/api/show"))
        .json(&json!({ "model": model.name }))
        .send()
        .and_then(|r| r.text())
      {
        model.thinking = show_supports_thinking(&show);
      }
    }
    Ok(models)
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

/// 素材と関連条目は会話全体で固定の文脈。指定プロンプトの後に pin する。
fn system_content_with(prompt: &str, req: &StructureRequest) -> String {
  let mut related = String::new();
  for e in &req.related {
    related.push_str(&format!("- {}: {}\n", e.title, e.excerpt));
  }
  if related.is_empty() {
    related.push_str("(none)\n");
  }
  format!(
    "{prompt}\n\n# Material\n{}\n\n# Related existing entries\n{related}",
    req.source_text
  )
}

/// /api/chat 用メッセージ列: 先頭に system（指示 + 固定文脈）、続けて会話履歴。
fn messages_with(prompt: &str, req: &StructureRequest) -> Value {
  let mut messages = vec![json!({ "role": "system", "content": system_content_with(prompt, req) })];
  for turn in &req.messages {
    messages.push(json!({ "role": turn.role, "content": turn.content }));
  }
  Value::Array(messages)
}

/// 非思考モデル: 単発で format 固定の構造化出力（従来の確実な経路）。
/// think を明示的に false にする（思考可能モデルは think 省略時に既定 ON のため、
/// format と組み合わさると content が壊れる。明示 off で確実に単発 JSON にする）。
fn build_body(model: &str, req: &StructureRequest) -> Value {
  json!({
    "model": model,
    "messages": messages_with(SYSTEM_PROMPT, req),
    "stream": true,
    "think": false,
    "format": output_schema(),
    "options": { "temperature": 0.2 }
  })
}

/// Pass1(思考パス): think 有効・format なし。推論を message.thinking に流しつつ
/// 散文ドラフトを生成する。format grammar と think の同時使用は content を壊す
/// （実機検証済み）ため format を付けない。サンプリングは公式モデルカード推奨値
/// （temp 1.0 / top_p .95 / top_k 64; 低温は推論モデルを退化させる）。num_ctx で
/// 思考+本文の予算を確保し、num_predict で暴走（話痨）を抑える。
fn build_draft_body(model: &str, req: &StructureRequest) -> Value {
  json!({
    "model": model,
    "messages": messages_with(DRAFT_PROMPT, req),
    "stream": true,
    "think": true,
    "options": {
      "temperature": 1.0,
      "top_p": 0.95,
      "top_k": 64,
      "num_ctx": 16384,
      "num_predict": 2048
    }
  })
}

/// Pass2(構造化パス): think 明示 off・format あり。Pass1 の散文ドラフトを確実に
/// StructureResult へ整形する（= 非思考の確実な経路を再利用）。think を省略すると
/// 思考可能モデルは既定 ON になり、Pass2 まで思考して think+format の壊れた組合せに
/// 戻る（実機で 9000+ token 思考 + 出力の崩壊を確認）。必ず false を明示する。
fn build_structure_body(model: &str, draft: &str) -> Value {
  json!({
    "model": model,
    "messages": [
      { "role": "system", "content": SYSTEM_PROMPT },
      { "role": "user", "content": format!("Convert the following draft into the JSON object as specified:\n\n{draft}") }
    ],
    "stream": true,
    "think": false,
    "format": output_schema(),
    "options": { "temperature": 0.2 }
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
    .map(|model| OllamaModel { name: model.name, thinking: false })
    .collect())
}

#[derive(Deserialize)]
struct ShowResponse {
  #[serde(default)]
  capabilities: Vec<String>,
}

/// /api/show のレスポンスから thinking 能力の有無を読む。
fn show_supports_thinking(body: &str) -> bool {
  serde_json::from_str::<ShowResponse>(body)
    .map(|s| s.capabilities.iter().any(|c| c == "thinking"))
    .unwrap_or(false)
}

#[derive(Deserialize)]
struct ChatMessage {
  #[serde(default)]
  content: String,
  #[serde(default)]
  thinking: Option<String>,
}

#[derive(Deserialize)]
struct StreamChunk {
  message: ChatMessage,
  #[serde(default)]
  done: bool,
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

/// NDJSON ストリーム（Ollama /api/chat stream=true）を消費し、message.content を連結して返す。
/// thinking は Thinking として、content の累積文字数は Generating として上報する。
/// JSON への整形は呼び出し側（parse_structure）に分離する（思考パスは散文を返すため）。
fn consume_chat_stream(
  lines: impl Iterator<Item = std::io::Result<String>>,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<String, AiError> {
  let mut content = String::new();
  for line in lines {
    let line = line.map_err(|e| {
      // 本文読み取り中のタイムアウトはモデルのロード/生成が長いケース。提案つきで返す。
      AiError::Network(format!(
        "读取 Ollama 响应失败（模型加载或生成可能超时，可先在终端 `ollama run` 预热）: {e}"
      ))
    })?;
    if line.trim().is_empty() {
      continue;
    }
    let chunk: StreamChunk =
      serde_json::from_str(&line).map_err(|e| AiError::Other(e.to_string()))?;
    // 思考モデルは thinking が content より先に流れる。増分をそのまま上報する。
    if let Some(thinking) = chunk.message.thinking {
      if !thinking.is_empty() {
        on_progress(StreamProgress::Thinking { delta: thinking });
      }
    }
    if !chunk.message.content.is_empty() {
      content.push_str(&chunk.message.content);
      on_progress(StreamProgress::Generating { chars: content.chars().count() });
    }
    if chunk.done {
      break;
    }
  }
  Ok(content)
}

/// モデル出力から JSON オブジェクトを取り出して構造化結果へ。Pass2 は format 固定で
/// 通常そのまま JSON だが、コードフェンスや前後の文字が混じっても拾えるよう、
/// 最初の `{` から最後の `}` までを抽出してから解析する。
fn parse_structure(content: &str) -> Result<StructureResult, AiError> {
  let json = extract_json_object(content).unwrap_or(content);
  let raw: RawResult =
    serde_json::from_str(json).map_err(|e| AiError::Other(e.to_string()))?;
  Ok(StructureResult {
    kind: raw.kind,
    title: raw.title,
    cat: raw.cat,
    body_markdown: raw.body_markdown,
    suggested_links: raw.suggested_links,
  })
}

/// 最初の `{` から最後の `}` までを返す（コードフェンス等の混入に耐える）。
fn extract_json_object(text: &str) -> Option<&str> {
  let start = text.find('{')?;
  let end = text.rfind('}')?;
  (end > start).then(|| &text[start..=end])
}

impl AiProvider for OllamaProvider {
  fn structure(
    &self,
    req: StructureRequest,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError> {
    let model = match &self.model {
      Some(model) => model.clone(),
      None => Self::first_local_model()?
        .ok_or_else(|| AiError::Other("Ollama 没有可用模型，请先下载模型".into()))?,
    };
    // 接続は短く（未起動を即検知）、全体は長く（モデルのロード + 生成を許容）。
    let client = reqwest::blocking::Client::builder()
      .connect_timeout(Duration::from_secs(3))
      .timeout(Duration::from_secs(180))
      .build()
      .map_err(|e| AiError::Network(e.to_string()))?;

    on_progress(StreamProgress::LoadingModel);
    if self.think {
      // 思考モデルは 2 段階。format grammar と think の同時使用が content を壊すため。
      // Pass1: 推論 + 散文ドラフト（think あり・format なし）。推論は面板へ流れる。
      let draft = post_chat(&client, &self.base_url, &model, build_draft_body(&model, &req), on_progress)?;
      // Pass2: ドラフトを構造化（format あり・think なし＝確実な経路）。
      let content =
        post_chat(&client, &self.base_url, &model, build_structure_body(&model, &draft), on_progress)?;
      parse_structure(&content)
    } else {
      let content = post_chat(&client, &self.base_url, &model, build_body(&model, &req), on_progress)?;
      parse_structure(&content)
    }
  }
}

/// /api/chat に 1 リクエストを投げ、ストリームを消費して連結 content を返す。
fn post_chat(
  client: &reqwest::blocking::Client,
  base_url: &str,
  model: &str,
  body: Value,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<String, AiError> {
  let resp = client
    .post(format!("{base_url}/api/chat"))
    .header("content-type", "application/json")
    .json(&body)
    .send()
    .map_err(|e| {
      if e.is_timeout() {
        AiError::Network(format!(
          "等待 Ollama 响应超时（模型可能正在加载，请先在终端执行 `ollama run {model}` 预热，或选择更小的模型）"
        ))
      } else {
        AiError::Network(format!("无法连接 Ollama（请确认 `ollama serve` 正在运行）: {e}"))
      }
    })?;

  let status = resp.status();
  if status.as_u16() == 404 {
    return Err(AiError::Other(format!(
      "Ollama 模型未找到: {model}（请先执行 `ollama pull {model}`）"
    )));
  }
  if !status.is_success() {
    let text = resp.text().unwrap_or_default();
    return Err(AiError::Other(format!("Ollama API 错误({status}): {text}")));
  }

  use std::io::BufRead;
  let reader = std::io::BufReader::new(resp);
  consume_chat_stream(reader.lines(), on_progress)
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
    assert_eq!(body["stream"], true);
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
  fn consume_chat_stream_accumulates_content_and_reports_progress() {
    // Ollama /api/chat stream=true は NDJSON。各行の message.content を連結すると JSON 本体になる。
    let lines = vec![
      Ok(r#"{"message":{"content":"{\"kind\":\"entry\",\"title\":\"緑"},"done":false}"#.to_string()),
      Ok(r#"{"message":{"content":"茶\",\"cat\":\"tea\",\"body_markdown\":\"本文\",\"suggested_links\":[]}"},"done":true}"#.to_string()),
    ];
    let mut events = Vec::new();
    let content = consume_chat_stream(lines.into_iter(), &mut |p| events.push(p)).unwrap();
    // consume は content を連結して返すだけ。整形は parse_structure に分離。
    let result = parse_structure(&content).unwrap();
    assert_eq!(result.kind, "entry");
    assert_eq!(result.title, "緑茶");
    assert_eq!(result.cat, "tea");
    // チャンク毎に Generating（累積文字数）が上報される
    assert!(matches!(events.first(), Some(StreamProgress::Generating { .. })));
    assert_eq!(events.len(), 2);
  }

  #[test]
  fn consume_chat_stream_reports_thinking_then_content() {
    let lines = vec![
      Ok(r#"{"message":{"thinking":"考え中"},"done":false}"#.to_string()),
      Ok(r#"{"message":{"content":"{\"kind\":\"chat\",\"title\":\"\",\"cat\":\"\",\"body_markdown\":\"hi\",\"suggested_links\":[]}"},"done":true}"#.to_string()),
    ];
    let mut events = Vec::new();
    let content = consume_chat_stream(lines.into_iter(), &mut |p| events.push(p)).unwrap();
    assert_eq!(parse_structure(&content).unwrap().body_markdown, "hi");
    assert_eq!(events.first(), Some(&StreamProgress::Thinking { delta: "考え中".into() }));
    assert!(events.iter().any(|e| matches!(e, StreamProgress::Generating { .. })));
  }

  #[test]
  fn draft_body_omits_format_and_enables_think() {
    // Pass1: think あり・format なし（grammar+think の同時使用が content を壊すため）。
    let body = build_draft_body("m", &req());
    assert_eq!(body["think"], true);
    assert_eq!(body.get("format"), None);
    assert_eq!(body["options"]["num_ctx"], 16384);
    assert_eq!(body["options"]["num_predict"], 2048);
    assert_eq!(body["options"]["temperature"], 1.0);
    // system は JSON を要求しない（散文ドラフト）が、素材は文脈として渡す。
    let system = body["messages"][0]["content"].as_str().unwrap();
    assert!(system.contains("緑茶の淹れ方"));
  }

  #[test]
  fn structure_body_disables_think_explicitly() {
    // Pass2: format あり・think は明示 false。省略だと思考可能モデルが既定 ON になり
    // think+format の壊れた組合せに戻る（実機で確認）。
    let body = build_structure_body("m", "杀青の本文");
    assert_eq!(body["format"]["type"], "object");
    assert_eq!(body["think"], false);
    assert!(body["messages"][1]["content"].as_str().unwrap().contains("杀青の本文"));
  }

  #[test]
  fn parse_structure_extracts_json_even_with_code_fence() {
    // コードフェンスや前後テキストが混じっても最初の { 〜 最後の } を拾う。
    let wrapped =
      "```json\n{\"kind\":\"entry\",\"title\":\"T\",\"cat\":\"c\",\"body_markdown\":\"B\",\"suggested_links\":[]}\n```";
    let result = parse_structure(wrapped).unwrap();
    assert_eq!(result.title, "T");
    assert_eq!(result.kind, "entry");
  }

  #[test]
  fn consume_chat_stream_propagates_read_error_with_hint() {
    let lines = vec![Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out"))];
    let err = consume_chat_stream(lines.into_iter(), &mut |_| {}).unwrap_err();
    assert!(matches!(err, AiError::Network(_)));
  }

  #[test]
  fn show_supports_thinking_reads_capabilities() {
    let yes = r#"{"capabilities":["completion","thinking"]}"#;
    let no = r#"{"capabilities":["completion"]}"#;
    assert!(show_supports_thinking(yes));
    assert!(!show_supports_thinking(no));
    assert!(!show_supports_thinking("{}"));
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
        OllamaModel { name: "qwen3:8b".into(), thinking: false },
        OllamaModel { name: "llama3.1:8b".into(), thinking: false },
      ]
    );
  }
}
