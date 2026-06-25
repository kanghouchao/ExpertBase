use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ai::agent::{output_schema, DRAFT_PROMPT, SYSTEM_PROMPT};
use crate::ai::{
  AgentMsg, AiError, AiProvider, StreamProgress, StructureRequest, StructureResult, ToolCall,
  ToolDef, TurnOutcome,
};

/// kind が欠落したモデル出力への安全な既定値（信頼境界の堅牢化）。
fn default_kind() -> String {
  "entry".to_string()
}

/// 既定の中断フラグ（決して立たない）。テスト・単発呼び出し用。
fn never_cancel() -> Arc<AtomicBool> {
  Arc::new(AtomicBool::new(false))
}

const API_BASE: &str = "http://127.0.0.1:11434";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
  pub name: String,
  /// thinking（推論トレース）能力を持つか。/api/show の capabilities で判定。
  pub thinking: bool,
  /// tools（関数呼び出し）能力を持つか。これが true のモデルだけ agent ループに乗せる。
  pub tools: bool,
}

pub struct OllamaProvider {
  model: Option<String>,
  base_url: String,
  think: bool,
  /// 停止ボタン用の中断フラグ。stream 消費中に true になったら生成を打ち切る。
  /// 既定は never-cancel（テスト・単発呼び出し用）。workshop_draft が実フラグを注入する。
  cancel: Arc<AtomicBool>,
}

impl OllamaProvider {
  pub fn new() -> Self {
    let model = std::env::var("EXPERTBASE_OLLAMA_MODEL")
      .ok()
      .filter(|s| !s.trim().is_empty())
      .map(|s| s.trim().to_string());
    Self { model, base_url: API_BASE.to_string(), think: false, cancel: never_cancel() }
  }

  pub fn with_model(model: String) -> Self {
    let selected = model.trim();
    if selected.is_empty() {
      Self::new()
    } else {
      Self {
        model: Some(selected.to_string()),
        base_url: API_BASE.to_string(),
        think: false,
        cancel: never_cancel(),
      }
    }
  }

  /// モデル指定 + thinking 有効化。capabilities に thinking がある場合のみ true を渡す。
  pub fn with_model_think(model: String, think: bool) -> Self {
    let mut provider = Self::with_model(model);
    provider.think = think;
    provider
  }

  /// 中断フラグを注入する（停止ボタンと共有する Arc<AtomicBool>）。
  pub fn with_cancel(mut self, cancel: Arc<AtomicBool>) -> Self {
    self.cancel = cancel;
    self
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

  pub fn first_local_model() -> Result<Option<String>, AiError> {
    Ok(Self::list_models()?.into_iter().next().map(|model| model.name))
  }

  /// 使用モデル名を解決する（明示指定が無ければローカル先頭）。
  fn resolve_model(&self) -> Result<String, AiError> {
    match &self.model {
      Some(model) => Ok(model.clone()),
      None => Self::first_local_model()?
        .ok_or_else(|| AiError::Other("Ollama 没有可用模型，请先下载模型".into())),
    }
  }

  /// 生成用クライアント。接続は短く（未起動を即検知）、全体は長く（ロード + 生成を許容）。
  fn build_client() -> Result<reqwest::blocking::Client, AiError> {
    reqwest::blocking::Client::builder()
      .connect_timeout(Duration::from_secs(3))
      .timeout(Duration::from_secs(180))
      .build()
      .map_err(|e| AiError::Network(e.to_string()))
  }
}

/// エージェント 1 ターンのリクエスト本体。system + tools + 会話を渡す。think はモデル能力に従う。
/// num_predict で 1 ターンの暴走を抑える（ツール呼び出し or ドラフトのどちらでも十分な予算）。
fn build_agent_body(
  model: &str,
  system: &str,
  tools: &[ToolDef],
  messages: &[AgentMsg],
  think: bool,
) -> Value {
  let mut msgs = vec![json!({ "role": "system", "content": system })];
  for m in messages {
    msgs.push(render_agent_msg(m));
  }
  json!({
    "model": model,
    "messages": msgs,
    "tools": render_tools(tools),
    "stream": true,
    "think": think,
    "options": { "temperature": 0.6, "num_ctx": 16384, "num_predict": 2048 }
  })
}

/// 中立な ToolDef 列を Ollama（OpenAI 互換）の tools wire 形式へ包む。wire 形状はここに閉じる。
fn render_tools(tools: &[ToolDef]) -> Value {
  Value::Array(
    tools
      .iter()
      .map(|t| {
        json!({
          "type": "function",
          "function": {
            "name": t.name,
            "description": t.description,
            "parameters": t.parameters
          }
        })
      })
      .collect(),
  )
}

/// AgentMsg を Ollama の /api/chat メッセージ形式へ変換する（wire 形式はここに閉じる）。
fn render_agent_msg(m: &AgentMsg) -> Value {
  match m {
    AgentMsg::User(content) => json!({ "role": "user", "content": content }),
    AgentMsg::Assistant { content, tool_calls } => json!({
      "role": "assistant",
      "content": content,
      "tool_calls": tool_calls
        .iter()
        .map(|tc| json!({ "function": { "name": tc.name, "arguments": tc.args } }))
        .collect::<Vec<_>>()
    }),
    AgentMsg::Tool { name, content } => {
      json!({ "role": "tool", "tool_name": name, "content": content })
    }
  }
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
/// num_predict で暴走を抑える: format 文法のみだと上限がなく、空/巨大ドラフトや
/// モデルの癖で JSON を閉じずに 180s 超時まで生成し続ける（＝「整理中…停まらない」）。
/// Pass1 の本文上限 2048 token を JSON で包み直す余裕として 4096 を取る。
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
    "options": { "temperature": 0.2, "num_predict": 4096 }
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

#[derive(Deserialize)]
struct ChatMessage {
  #[serde(default)]
  content: String,
  #[serde(default)]
  thinking: Option<String>,
  #[serde(default)]
  tool_calls: Vec<RawToolCall>,
}

/// Ollama のツール呼び出し（message.tool_calls[]）。arguments はオブジェクト（文字列ではない）。
#[derive(Deserialize)]
struct RawToolCall {
  function: RawToolFn,
}

#[derive(Deserialize)]
struct RawToolFn {
  name: String,
  #[serde(default)]
  arguments: Value,
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
/// thinking は Thinking として上報する。content の各チャンクは content_event で上報する
/// （引数は (新着分 delta, 累積文字数)）。段ごとに表現を変える: Pass1＝Narration（実テキスト）、
/// Pass2＝Structuring（数字）、非思考単発＝Generating（数字）。
/// JSON への整形は呼び出し側（parse_structure）に分離する（思考パスは散文を返すため）。
fn consume_chat_stream(
  lines: impl Iterator<Item = std::io::Result<String>>,
  content_event: &dyn Fn(&str, usize) -> StreamProgress,
  should_cancel: &dyn Fn() -> bool,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<String, AiError> {
  let mut content = String::new();
  for line in lines {
    // 停止ボタン: 各チャンクの前に確認。立っていれば読み取りを止めて即返す。
    // ここで return すると呼び出し側で reader（＝接続）が drop され、Ollama 側の生成も中断される。
    if should_cancel() {
      return Err(AiError::Cancelled);
    }
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
      on_progress(content_event(&chunk.message.content, content.chars().count()));
    }
    if chunk.done {
      break;
    }
  }
  Ok(content)
}

/// エージェント 1 ターンの NDJSON ストリームを消費する。thinking→Thinking、content→Narration を
/// 流しつつ、message.tool_calls を集めて (本文, ツール呼び出し列) を返す。
fn consume_agent_stream(
  lines: impl Iterator<Item = std::io::Result<String>>,
  should_cancel: &dyn Fn() -> bool,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<(String, Vec<ToolCall>), AiError> {
  let mut content = String::new();
  let mut tool_calls: Vec<ToolCall> = Vec::new();
  for line in lines {
    if should_cancel() {
      return Err(AiError::Cancelled);
    }
    let line = line.map_err(|e| {
      AiError::Network(format!(
        "读取 Ollama 响应失败（模型加载或生成可能超时，可先在终端 `ollama run` 预热）: {e}"
      ))
    })?;
    if line.trim().is_empty() {
      continue;
    }
    let chunk: StreamChunk =
      serde_json::from_str(&line).map_err(|e| AiError::Other(e.to_string()))?;
    if let Some(thinking) = chunk.message.thinking {
      if !thinking.is_empty() {
        on_progress(StreamProgress::Thinking { delta: thinking });
      }
    }
    if !chunk.message.content.is_empty() {
      content.push_str(&chunk.message.content);
      on_progress(StreamProgress::Narration { delta: chunk.message.content.clone() });
    }
    for tc in chunk.message.tool_calls {
      tool_calls.push(ToolCall { name: tc.function.name, args: tc.function.arguments });
    }
    if chunk.done {
      break;
    }
  }
  Ok((content, tool_calls))
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
    let model = self.resolve_model()?;
    let client = Self::build_client()?;

    on_progress(StreamProgress::LoadingModel);
    // Pass1（思考モデルの散文）は実テキストを Narration で流す＝過程を見せる。
    let narrating = |delta: &str, _c: usize| StreamProgress::Narration { delta: delta.to_string() };
    // 非思考単発は JSON を起こす段。表示用テキストは無いので数字で済ます。
    let generating = |_d: &str, c: usize| StreamProgress::Generating { chars: c };
    let should_cancel = || self.cancel.load(Ordering::Relaxed);
    if self.think {
      // 思考モデルは 2 段階。format grammar と think の同時使用が content を壊すため。
      // Pass1: 推論 + 散文ドラフト（think あり・format なし）。散文は Narration として会話へ流れる。
      let draft = post_chat(
        &client,
        &self.base_url,
        &model,
        build_draft_body(&model, &req),
        &narrating,
        &should_cancel,
        on_progress,
      )?;
      // Pass2: ドラフトを構造化（format あり・think なし＝確実な経路）＝整理。
      self.structure_draft(&draft, on_progress)
    } else {
      // 非思考は 1 段で直接 JSON を起こす＝生成。
      let content = post_chat(
        &client,
        &self.base_url,
        &model,
        build_body(&model, &req),
        &generating,
        &should_cancel,
        on_progress,
      )?;
      parse_structure(&content)
    }
  }

  fn agent_turn(
    &self,
    system: &str,
    tools: &[ToolDef],
    messages: &[AgentMsg],
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<TurnOutcome, AiError> {
    let model = self.resolve_model()?;
    let client = Self::build_client()?;
    on_progress(StreamProgress::LoadingModel);
    let should_cancel = || self.cancel.load(Ordering::Relaxed);
    let body = build_agent_body(&model, system, tools, messages, self.think);
    let (content, tool_calls) =
      post_agent(&client, &self.base_url, &model, body, &should_cancel, on_progress)?;
    Ok(TurnOutcome { content, tool_calls })
  }

  fn structure_draft(
    &self,
    draft: &str,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError> {
    let model = self.resolve_model()?;
    let client = Self::build_client()?;
    let should_cancel = || self.cancel.load(Ordering::Relaxed);
    // 整理段（Pass2 を再利用）。format 固定で散文ドラフトを StructureResult へ。
    let structuring = |_d: &str, c: usize| StreamProgress::Structuring { chars: c };
    let content = post_chat(
      &client,
      &self.base_url,
      &model,
      build_structure_body(&model, draft),
      &structuring,
      &should_cancel,
      on_progress,
    )?;
    parse_structure(&content)
  }
}

/// /api/chat に POST してストリーミング応答（status 検証済み）を返す。
fn send_chat(
  client: &reqwest::blocking::Client,
  base_url: &str,
  model: &str,
  body: Value,
) -> Result<reqwest::blocking::Response, AiError> {
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
  Ok(resp)
}

/// /api/chat に 1 リクエストを投げ、ストリームを消費して連結 content を返す。
fn post_chat(
  client: &reqwest::blocking::Client,
  base_url: &str,
  model: &str,
  body: Value,
  content_event: &dyn Fn(&str, usize) -> StreamProgress,
  should_cancel: &dyn Fn() -> bool,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<String, AiError> {
  let resp = send_chat(client, base_url, model, body)?;
  use std::io::BufRead;
  let reader = std::io::BufReader::new(resp);
  consume_chat_stream(reader.lines(), content_event, should_cancel, on_progress)
}

/// /api/chat（tools 付き）に 1 リクエストを投げ、本文 + ツール呼び出しを返す。
fn post_agent(
  client: &reqwest::blocking::Client,
  base_url: &str,
  model: &str,
  body: Value,
  should_cancel: &dyn Fn() -> bool,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<(String, Vec<ToolCall>), AiError> {
  let resp = send_chat(client, base_url, model, body)?;
  use std::io::BufRead;
  let reader = std::io::BufReader::new(resp);
  consume_agent_stream(reader.lines(), should_cancel, on_progress)
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
    let gen = |_: &str, c: usize| StreamProgress::Generating { chars: c };
    let content =
      consume_chat_stream(lines.into_iter(), &gen, &|| false, &mut |p| events.push(p)).unwrap();
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
  fn consume_chat_stream_uses_content_event_for_structuring() {
    // Pass2 は Structuring 閉包を渡す＝整理段として区別できる。
    let lines = vec![Ok(
      r#"{"message":{"content":"{\"kind\":\"entry\",\"title\":\"t\",\"cat\":\"\",\"body_markdown\":\"b\",\"suggested_links\":[]}"},"done":true}"#
        .to_string(),
    )];
    let mut events = Vec::new();
    let structuring = |_: &str, c: usize| StreamProgress::Structuring { chars: c };
    consume_chat_stream(lines.into_iter(), &structuring, &|| false, &mut |p| events.push(p)).unwrap();
    assert!(events.iter().all(|e| matches!(e, StreamProgress::Structuring { .. })));
    assert!(!events.is_empty());
  }

  #[test]
  fn consume_chat_stream_narration_carries_delta_text() {
    // Pass1（散文）は各チャンクの実テキスト（delta）を Narration として流す＝過程表示の素。
    let lines = vec![
      Ok(r#"{"message":{"content":"こん"},"done":false}"#.to_string()),
      Ok(r#"{"message":{"content":"にちは"},"done":true}"#.to_string()),
    ];
    let mut events = Vec::new();
    let narrating =
      |delta: &str, _c: usize| StreamProgress::Narration { delta: delta.to_string() };
    consume_chat_stream(lines.into_iter(), &narrating, &|| false, &mut |p| events.push(p)).unwrap();
    let deltas: Vec<String> = events
      .iter()
      .filter_map(|e| match e {
        StreamProgress::Narration { delta } => Some(delta.clone()),
        _ => None,
      })
      .collect();
    assert_eq!(deltas, vec!["こん".to_string(), "にちは".to_string()]);
  }

  #[test]
  fn render_tools_wraps_tooldef_in_function_envelope() {
    // 中立 ToolDef → Ollama/OpenAI 互換の {type:function, function:{...}} へ包む（wire 形状は infra）。
    let tools = vec![ToolDef {
      name: "search_kb".into(),
      description: "find entries".into(),
      parameters: json!({ "type": "object", "properties": { "query": { "type": "string" } } }),
    }];
    let wire = render_tools(&tools);
    assert_eq!(wire[0]["type"], "function");
    assert_eq!(wire[0]["function"]["name"], "search_kb");
    assert_eq!(wire[0]["function"]["description"], "find entries");
    assert_eq!(wire[0]["function"]["parameters"]["properties"]["query"]["type"], "string");
  }

  #[test]
  fn consume_agent_stream_collects_content_and_tool_calls() {
    // Ollama の tool_calls wire 形式（message.tool_calls[].function.{name,arguments}）を正しく拾う。
    // arguments はオブジェクト（文字列ではない）。thinking→Thinking、content→Narration も流す。
    let lines = vec![
      Ok(r#"{"message":{"thinking":"考え中","content":""},"done":false}"#.to_string()),
      Ok(
        r#"{"message":{"content":"検索します","tool_calls":[{"function":{"name":"search_kb","arguments":{"query":"茶"}}}]},"done":false}"#
          .to_string(),
      ),
      Ok(r#"{"message":{"content":""},"done":true}"#.to_string()),
    ];
    let mut events = Vec::new();
    let (content, tool_calls) =
      consume_agent_stream(lines.into_iter(), &|| false, &mut |p| events.push(p)).unwrap();
    assert_eq!(content, "検索します");
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name, "search_kb");
    assert_eq!(tool_calls[0].args["query"], "茶");
    assert!(events.iter().any(|e| matches!(e, StreamProgress::Thinking { .. })));
    assert!(events.iter().any(|e| matches!(e, StreamProgress::Narration { .. })));
  }

  #[test]
  fn consume_agent_stream_stops_when_cancelled() {
    // 停止ボタン: エージェントターンも次チャンク前に中断できる。
    let lines = vec![
      Ok(r#"{"message":{"content":"a"},"done":false}"#.to_string()),
      Ok(r#"{"message":{"content":"b"},"done":false}"#.to_string()),
    ];
    let calls = std::cell::Cell::new(0);
    let cancel = || {
      calls.set(calls.get() + 1);
      calls.get() > 1
    };
    let result = consume_agent_stream(lines.into_iter(), &cancel, &mut |_| {});
    assert!(matches!(result, Err(AiError::Cancelled)));
  }

  #[test]
  fn consume_chat_stream_stops_when_cancelled() {
    // 停止ボタン: should_cancel が途中で true になったら、残りの行は消費せず Cancelled で返す。
    let lines = vec![
      Ok(r#"{"message":{"content":"a"},"done":false}"#.to_string()),
      Ok(r#"{"message":{"content":"b"},"done":false}"#.to_string()),
    ];
    let mut events = Vec::new();
    let gen = |_: &str, c: usize| StreamProgress::Generating { chars: c };
    // 1 回目の確認は false（1 行目を処理）、2 回目で true（2 行目の前で中断）。
    let calls = std::cell::Cell::new(0);
    let cancel = || {
      calls.set(calls.get() + 1);
      calls.get() > 1
    };
    let result = consume_chat_stream(lines.into_iter(), &gen, &cancel, &mut |p| events.push(p));
    assert!(matches!(result, Err(AiError::Cancelled)));
    // 2 行目は処理されない（進捗は 1 件のみ）。
    assert_eq!(events.len(), 1);
  }

  #[test]
  fn consume_chat_stream_reports_thinking_then_content() {
    let lines = vec![
      Ok(r#"{"message":{"thinking":"考え中"},"done":false}"#.to_string()),
      Ok(r#"{"message":{"content":"{\"kind\":\"chat\",\"title\":\"\",\"cat\":\"\",\"body_markdown\":\"hi\",\"suggested_links\":[]}"},"done":true}"#.to_string()),
    ];
    let mut events = Vec::new();
    let gen = |_: &str, c: usize| StreamProgress::Generating { chars: c };
    let content =
      consume_chat_stream(lines.into_iter(), &gen, &|| false, &mut |p| events.push(p)).unwrap();
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
    // num_predict で整理段の暴走を抑える（format 文法のみだと上限なしで 180s 超時まで生成しうる）。
    assert_eq!(body["options"]["num_predict"], 4096);
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
    let gen = |_: &str, c: usize| StreamProgress::Generating { chars: c };
    let err = consume_chat_stream(lines.into_iter(), &gen, &|| false, &mut |_| {}).unwrap_err();
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
