//! agent インフラ: Rig によるエージェントの駆動（プロバイダ非依存）。
//! `run` が provider を選んで該当クライアントで `Agent<M>` を組み、共通の `drive` で
//! `stream_chat().multi_turn()` を回して `MultiTurnStreamItem` を `StreamProgress` へ写像し
//! mpsc で流す。ツールは呼び出し側が注入する（本モジュールは業務を知らない）。
//! 中断は共有 `AtomicBool` をチャンク間で確認し、立っていれば stream を drop して返す。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use serde_json::json;
use tokio::sync::mpsc::UnboundedSender;

use rig_core::agent::{Agent, MultiTurnStreamItem};
use rig_core::client::CompletionClient;
use rig_core::completion::{CompletionModel, GetTokenUsage};
use rig_core::message::{Message, Text, ToolResultContent};
use rig_core::providers::ollama::OllamaApiKey;
use rig_core::providers::{ollama, openai};
use rig_core::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingChat};
use rig_core::tool::ToolDyn;
use rig_core::OneOrMany;

use crate::agent::{resolve_base_url, ChatTurn, Provider, StreamProgress};
use crate::error::AppError;

/// エージェントの暴走（無限ツール呼び出し）を抑える反復上限。
const MAX_TURNS: usize = 6;

/// 注入されたツールで 1 会話分回す。system + 履歴 + 直近プロンプトを渡し、思考・本文・
/// ツール呼び出し/結果を tx へ流しつつ最終本文を返す。provider ごとにクライアント構築だけ分岐する。
#[allow(clippy::too_many_arguments)]
pub async fn run(
  provider: Provider,
  base_url: &str,
  model: &str,
  think: bool,
  system: &str,
  tools: Vec<Box<dyn ToolDyn>>,
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: &UnboundedSender<StreamProgress>,
) -> Result<String, AppError> {
  // 直近のユーザー発話をプロンプト、それ以前を履歴に分ける。
  let Some((last, rest)) = messages.split_last() else {
    return Err(AppError::code("err.agent.emptyConversation"));
  };
  let history: Vec<Message> = rest.iter().map(turn_to_message).collect();
  let prompt = turn_to_message(last);

  // 送信済み・最初のトークン待ち（モデルのロード中を含む）。
  let _ = tx.send(StreamProgress::LoadingModel);

  // 設定の生 URL を解決（空欄は provider 既定へ。「設定可能だが既定値を持つ」）。
  let base_url = resolve_base_url(provider, base_url);

  match provider {
    Provider::Ollama => {
      // base_url を明示指定（rig の既定 localhost に頼らず、設定した remote Ollama も効く）。ローカルは no-auth。
      let client = ollama::Client::builder()
        .api_key(OllamaApiKey::default())
        .base_url(&base_url)
        .build()
        .map_err(|e| AppError::param("err.agent.network", "detail", e))?;
      // num_ctx は options へ、think は最上位へ（Ollama provider が additional_params を仕分ける）。
      let agent = client
        .agent(model)
        .preamble(system)
        .temperature(0.6)
        .additional_params(json!({ "num_ctx": 16384, "think": think }))
        .tools(tools)
        .build();
      drive(agent, prompt, history, cancel, tx).await
    }
    Provider::LlamaApp => {
      // ponytail: llama.app = llama.cpp の `llama serve`（OpenAI 互換ローカル端点）と仮定。
      // 独自プロトコルならこの arm 本体だけ差し替える（縫い目は不変）。ローカルは key 不要。
      let client = openai::Client::builder()
        .api_key("expertbase-local") // ローカル端点は key 不要。ダミーを渡す。
        .base_url(&base_url)
        .build()
        .map_err(|e| AppError::param("err.agent.network", "detail", e))?;
      let agent = client.agent(model).preamble(system).temperature(0.6).tools(tools).build();
      drive(agent, prompt, history, cancel, tx).await
    }
  }
}

/// 組み上げた `Agent<M>` を回す共通ループ（provider 非依存、provider ごとに単態化）。
/// Rig の流を decode（無状態写像）で自前イベントへ落とし、状態機は pump が一手に持つ。
async fn drive<M>(
  agent: Agent<M>,
  prompt: Message,
  history: Vec<Message>,
  cancel: Arc<AtomicBool>,
  tx: &UnboundedSender<StreamProgress>,
) -> Result<String, AppError>
where
  M: CompletionModel + 'static,
  M::StreamingResponse: GetTokenUsage,
{
  let stream = agent.stream_chat(prompt, history).multi_turn(MAX_TURNS).await;
  let events = stream.filter_map(|item| async move {
    match item {
      Ok(item) => decode(item).map(Ok),
      Err(e) => Some(Err(AppError::generic(e))),
    }
  });
  pump(events, cancel, tx).await
}

/// 解読済みの流イベント（自前・非泛型）。decode の出力で pump の入力。
/// Rig の泛型（`M::StreamingResponse`）をここで断ち切り、pump を脚本列でテスト可能にする。
#[derive(Debug)]
enum StreamEvent {
  /// ユーザー向け本文の増分。
  Text(String),
  /// 推論トレースの増分。
  Reasoning(String),
  /// ツール呼び出し（id は結果との対応付け用の internal_call_id）。
  ToolCall { id: String, name: String, args: String },
  /// ツール実行結果（id で呼び出し時の名前を引く）。
  ToolResult { id: String, summary: String },
  /// 最終返信本文。
  Final(String),
}

/// `MultiTurnStreamItem` → `StreamEvent` の無状態写像。関心外の項は None（読み飛ばし）。
fn decode<R>(item: MultiTurnStreamItem<R>) -> Option<StreamEvent> {
  match item {
    MultiTurnStreamItem::StreamAssistantItem(content) => match content {
      StreamedAssistantContent::Text(Text { text, .. }) => Some(StreamEvent::Text(text)),
      StreamedAssistantContent::Reasoning(reasoning) => {
        Some(StreamEvent::Reasoning(reasoning.display_text()))
      }
      StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
        Some(StreamEvent::Reasoning(reasoning))
      }
      StreamedAssistantContent::ToolCall { tool_call, internal_call_id } => {
        Some(StreamEvent::ToolCall {
          id: internal_call_id,
          name: tool_call.function.name,
          args: tool_call.function.arguments.to_string(),
        })
      }
      _ => None,
    },
    MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult {
      tool_result,
      internal_call_id,
    }) => Some(StreamEvent::ToolResult {
      id: internal_call_id,
      summary: first_text(&tool_result.content),
    }),
    MultiTurnStreamItem::FinalResponse(res) => Some(StreamEvent::Final(res.response().to_string())),
    _ => None,
  }
}

/// イベント列を消費する状態機（ツール名の対応付け・最終本文・中断・上流エラーの一手持ち）。
/// 中断は各イベント処理前に共有 `AtomicBool` を確認し、立っていれば残りを消費せず即返す。
/// Final 不在の自然終端は Ok("")（既定語義）。Rig 非依存＝脚本列で全語義をテストできる。
async fn pump(
  events: impl futures::Stream<Item = Result<StreamEvent, AppError>>,
  cancel: Arc<AtomicBool>,
  tx: &UnboundedSender<StreamProgress>,
) -> Result<String, AppError> {
  let mut events = std::pin::pin!(events);
  // internal_call_id → ツール名。ToolResult はツール名を持たないので呼び出し時に対応付ける。
  let mut tool_names: HashMap<String, String> = HashMap::new();

  while let Some(event) = events.next().await {
    // 停止ボタン: 各イベント前に確認。立っていれば stream を drop して即返す。
    if cancel.load(Ordering::Relaxed) {
      return Err(AppError::code("err.agent.cancelled"));
    }
    match event? {
      StreamEvent::Text(delta) => {
        let _ = tx.send(StreamProgress::Narration { delta });
      }
      StreamEvent::Reasoning(delta) => {
        let _ = tx.send(StreamProgress::Thinking { delta });
      }
      StreamEvent::ToolCall { id, name, args } => {
        tool_names.insert(id, name.clone());
        let _ = tx.send(StreamProgress::ToolCall { name, args });
      }
      StreamEvent::ToolResult { id, summary } => {
        let name = tool_names.get(&id).cloned().unwrap_or_default();
        let _ = tx.send(StreamProgress::ToolResult { name, summary });
      }
      StreamEvent::Final(text) => return Ok(text),
    }
  }

  Ok(String::new())
}

/// ChatTurn を Rig の Message へ。role が user 以外は assistant 扱い。
fn turn_to_message(turn: &ChatTurn) -> Message {
  if turn.role == "user" {
    Message::user(turn.content.as_str())
  } else {
    Message::assistant(turn.content.as_str())
  }
}

/// ツール結果の最初のテキスト断片を取り出す（表示用サマリ）。
fn first_text(content: &OneOrMany<ToolResultContent>) -> String {
  content
    .iter()
    .find_map(|c| match c {
      ToolResultContent::Text(t) => Some(t.text().to_string()),
      _ => None,
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::*;

  // URL 解決（空欄→provider 既定）は domain::resolve_base_url で単体テスト済み。
  // 空欄はもはやエラーにせず既定端点へ倒すため、旧「URL 未設定でエラー」テストは廃止。
  #[tokio::test]
  async fn run_errors_on_empty_messages() {
    // 空メッセージはネットワークに触れる前に即エラー（base_url は空でも解決前に返る）。
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let err = run(Provider::Ollama, "", "m", false, "s", vec![], vec![], cancel, &tx)
      .await
      .unwrap_err();
    assert_eq!(err.code, "err.agent.emptyConversation");
  }

  // ---- decode（Rig 流項 → StreamEvent の無状態写像）。 ----

  #[test]
  fn decode_extracts_fields_from_rig_stream_items() {
    use rig_core::agent::FinalResponse;
    use rig_core::completion::Usage;
    use rig_core::message::{AssistantContent, ToolCall, ToolFunction, ToolResult};

    // 本文・推論増分は素通し。
    assert!(matches!(
      decode::<()>(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
        Text { text: "本文".into(), additional_params: None },
      ))),
      Some(StreamEvent::Text(t)) if t == "本文"
    ));
    assert!(matches!(
      decode::<()>(MultiTurnStreamItem::StreamAssistantItem(
        StreamedAssistantContent::ReasoningDelta { id: None, reasoning: "思考".into() },
      )),
      Some(StreamEvent::Reasoning(t)) if t == "思考"
    ));

    // ツール呼び出しは internal_call_id（provider id ではない）を対応付けキーに採る。
    let call = MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::<()>::ToolCall {
      tool_call: ToolCall {
        id: "prov-1".into(),
        call_id: None,
        function: ToolFunction {
          name: "search_kb".into(),
          arguments: serde_json::json!({"query": "茶"}),
        },
        signature: None,
        additional_params: None,
      },
      internal_call_id: "c1".into(),
    });
    let Some(StreamEvent::ToolCall { id, name, args }) = decode(call) else {
      panic!("ToolCall が写像されない");
    };
    assert_eq!((id.as_str(), name.as_str()), ("c1", "search_kb"));
    assert_eq!(args, r#"{"query":"茶"}"#);

    // ツール結果は最初のテキスト断片をサマリに採る（first_text）。
    let result: MultiTurnStreamItem<()> =
      MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult {
        tool_result: ToolResult {
          id: "prov-1".into(),
          call_id: None,
          content: OneOrMany::one(ToolResultContent::text("2 hits")),
        },
        internal_call_id: "c1".into(),
      });
    assert!(matches!(
      decode(result),
      Some(StreamEvent::ToolResult { id, summary }) if id == "c1" && summary == "2 hits"
    ));

    // 最終返信は本文テキストへ。
    let fin = FinalResponse::new(OneOrMany::one(AssistantContent::text("完成")), Usage::new(), None);
    assert!(matches!(
      decode::<()>(MultiTurnStreamItem::FinalResponse(fin)),
      Some(StreamEvent::Final(t)) if t == "完成"
    ));
  }

  // ---- pump（解読済みイベント列の状態機）。脚本列（stream::iter）で全語義を断言する。 ----

  /// pump 完了後に送信済み進捗を全部吸い出す。
  fn drain(rx: &mut tokio::sync::mpsc::UnboundedReceiver<StreamProgress>) -> Vec<StreamProgress> {
    let mut got = Vec::new();
    while let Ok(p) = rx.try_recv() {
      got.push(p);
    }
    got
  }

  /// 全て Ok のイベント列を作る。
  fn ok_events(
    events: Vec<StreamEvent>,
  ) -> impl futures::Stream<Item = Result<StreamEvent, AppError>> {
    futures::stream::iter(events.into_iter().map(Ok))
  }

  #[tokio::test]
  async fn pump_maps_each_event_to_progress_in_order() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let events = ok_events(vec![
      StreamEvent::Reasoning("考え中".into()),
      StreamEvent::Text("本文".into()),
      StreamEvent::ToolCall { id: "c1".into(), name: "search_kb".into(), args: "{}".into() },
      StreamEvent::ToolResult { id: "c1".into(), summary: "2 hits".into() },
      StreamEvent::Final("完成".into()),
    ]);

    let text = pump(events, cancel, &tx).await.unwrap();

    assert_eq!(text, "完成");
    assert_eq!(
      drain(&mut rx),
      vec![
        StreamProgress::Thinking { delta: "考え中".into() },
        StreamProgress::Narration { delta: "本文".into() },
        StreamProgress::ToolCall { name: "search_kb".into(), args: "{}".into() },
        StreamProgress::ToolResult { name: "search_kb".into(), summary: "2 hits".into() },
      ]
    );
  }

  #[tokio::test]
  async fn pump_correlates_interleaved_tool_results_by_call_id() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let events = ok_events(vec![
      StreamEvent::ToolCall { id: "a".into(), name: "search_kb".into(), args: "{}".into() },
      StreamEvent::ToolCall { id: "b".into(), name: "read_entry".into(), args: "{}".into() },
      StreamEvent::ToolResult { id: "b".into(), summary: "本文".into() },
      StreamEvent::ToolResult { id: "a".into(), summary: "3 hits".into() },
      StreamEvent::Final(String::new()),
    ]);

    pump(events, cancel, &tx).await.unwrap();

    // 交差した結果でも internal_call_id で各自の名前に紐づく。
    assert_eq!(
      drain(&mut rx),
      vec![
        StreamProgress::ToolCall { name: "search_kb".into(), args: "{}".into() },
        StreamProgress::ToolCall { name: "read_entry".into(), args: "{}".into() },
        StreamProgress::ToolResult { name: "read_entry".into(), summary: "本文".into() },
        StreamProgress::ToolResult { name: "search_kb".into(), summary: "3 hits".into() },
      ]
    );
  }

  #[tokio::test]
  async fn pump_uses_empty_name_for_unknown_call_id() {
    // 既定語義（現状維持）: 未登録 id の結果は名前空欄で流す（落とさない）。
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let events = ok_events(vec![
      StreamEvent::ToolResult { id: "ghost".into(), summary: "x".into() },
      StreamEvent::Final(String::new()),
    ]);

    pump(events, cancel, &tx).await.unwrap();

    assert_eq!(
      drain(&mut rx),
      vec![StreamProgress::ToolResult { name: String::new(), summary: "x".into() }]
    );
  }

  #[tokio::test]
  async fn pump_cancels_before_processing_next_event() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(true));
    let events = ok_events(vec![StreamEvent::Text("捨てられる".into())]);

    let err = pump(events, cancel, &tx).await.unwrap_err();

    // 各イベント処理前に確認＝立っていれば何も発しない。
    assert_eq!(err.code, "err.agent.cancelled");
    assert!(drain(&mut rx).is_empty());
  }

  #[tokio::test]
  async fn pump_returns_upstream_error_after_emitting_prior_deltas() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let events = futures::stream::iter(vec![
      Ok(StreamEvent::Text("前半".into())),
      Err(AppError::generic("boom")),
      Ok(StreamEvent::Text("後半".into())),
    ]);

    let err = pump(events, cancel, &tx).await.unwrap_err();

    // 上流エラーは即返し、以降のイベントは消費しない。発出済みの増分はそのまま。
    assert_eq!(err.code, "err.generic");
    assert_eq!(drain(&mut rx), vec![StreamProgress::Narration { delta: "前半".into() }]);
  }

  #[tokio::test]
  async fn pump_stops_consuming_after_final_response() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let events = ok_events(vec![
      StreamEvent::Final("done".into()),
      StreamEvent::Text("after".into()),
    ]);

    let text = pump(events, cancel, &tx).await.unwrap();

    // Final で打ち切り＝以降のイベントは発出されない。
    assert_eq!(text, "done");
    assert!(drain(&mut rx).is_empty());
  }

  #[tokio::test]
  async fn pump_returns_empty_text_when_stream_ends_without_final() {
    // 既定語義（現状維持）: Final 不在の自然終端は Ok("")。
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let events = ok_events(vec![StreamEvent::Text("a".into())]);

    let text = pump(events, cancel, &tx).await.unwrap();

    assert_eq!(text, "");
    assert_eq!(drain(&mut rx), vec![StreamProgress::Narration { delta: "a".into() }]);
  }
}
