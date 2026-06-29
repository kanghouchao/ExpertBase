//! workshop インフラ: Rig による Ollama エージェントの駆動（手書きループの置き換え）。
//! Ollama クライアント → preamble + tools + additional_params(num_ctx/think) でエージェントを組み、
//! `stream_chat().multi_turn().await` の `MultiTurnStreamItem` を `StreamProgress` へ写像して
//! mpsc で interface へ流す。最終本文は `FinalResponse::response()`。
//! 中断は共有 `AtomicBool` をチャンク間で確認し、立っていれば stream を drop して返す
//! （drop により reqwest 接続が切れ、Ollama 側の生成も止まる）。

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use serde_json::json;
use tokio::sync::mpsc::UnboundedSender;

use rig_core::agent::MultiTurnStreamItem;
use rig_core::client::{CompletionClient, Nothing};
use rig_core::message::{Message, Text, ToolResultContent};
use rig_core::providers::ollama;
use rig_core::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingChat};
use rig_core::tool::ToolDyn;
use rig_core::OneOrMany;

use super::tools::{FetchWeb, ReadSource, SearchKb, WriteEntry};
use crate::ai::{AiError, ChatTurn, StreamProgress};

/// エージェントの暴走（無限ツール呼び出し）を抑える反復上限。
const MAX_TURNS: usize = 6;

/// Ollama エージェントを 1 会話分回す。system + 履歴 + 直近プロンプトを渡し、
/// 思考・本文・ツール呼び出し/結果を tx へ流しつつ、最終本文を返す。
pub(crate) async fn run(
  model: &str,
  think: bool,
  system: &str,
  root: &Path,
  sources: &[String],
  inbox_rels: &[String],
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: &UnboundedSender<StreamProgress>,
) -> Result<String, AiError> {
  // 直近のユーザー発話をプロンプト、それ以前を履歴に分ける。
  let Some((last, rest)) = messages.split_last() else {
    return Err(AiError::Other("対話メッセージが空です".into()));
  };

  let client = ollama::Client::new(Nothing).map_err(|e| AiError::Network(e.to_string()))?;

  // 工作坊は tools 対応モデル必須。read_source（素材読み取り）・search_kb・write_entry を常に登録する。
  let tools: Vec<Box<dyn ToolDyn>> = vec![
    Box::new(ReadSource { root: root.to_path_buf(), sources: sources.to_vec() }),
    Box::new(SearchKb { root: root.to_path_buf() }),
    Box::new(WriteEntry { root: root.to_path_buf(), inbox_rels: inbox_rels.to_vec() }),
    Box::new(FetchWeb),
  ];

  // num_ctx は options へ、think は最上位へ（Ollama provider が additional_params を仕分ける）。
  let agent = client
    .agent(model)
    .preamble(system)
    .temperature(0.6)
    .additional_params(json!({ "num_ctx": 16384, "think": think }))
    .tools(tools)
    .build();

  let history: Vec<Message> = rest.iter().map(turn_to_message).collect();
  let prompt = turn_to_message(last);

  // 送信済み・最初のトークン待ち（モデルのロード中を含む）。
  let _ = tx.send(StreamProgress::LoadingModel);

  let mut stream = agent.stream_chat(prompt, history).multi_turn(MAX_TURNS).await;

  // internal_call_id → ツール名。ToolResult はツール名を持たないので呼び出し時に対応付ける。
  let mut tool_names: HashMap<String, String> = HashMap::new();
  let mut final_text = String::new();

  while let Some(item) = stream.next().await {
    // 停止ボタン: 各チャンク前に確認。立っていれば stream を drop して即返す。
    if cancel.load(Ordering::Relaxed) {
      return Err(AiError::Cancelled);
    }
    match item {
      Ok(MultiTurnStreamItem::StreamAssistantItem(content)) => match content {
        StreamedAssistantContent::Text(Text { text, .. }) => {
          let _ = tx.send(StreamProgress::Narration { delta: text });
        }
        StreamedAssistantContent::Reasoning(reasoning) => {
          let _ = tx.send(StreamProgress::Thinking { delta: reasoning.display_text() });
        }
        StreamedAssistantContent::ReasoningDelta { reasoning, .. } => {
          let _ = tx.send(StreamProgress::Thinking { delta: reasoning });
        }
        StreamedAssistantContent::ToolCall { tool_call, internal_call_id } => {
          let name = tool_call.function.name.clone();
          tool_names.insert(internal_call_id, name.clone());
          let _ = tx.send(StreamProgress::ToolCall {
            name,
            args: tool_call.function.arguments.to_string(),
          });
        }
        _ => {}
      },
      Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult {
        tool_result,
        internal_call_id,
      })) => {
        let name = tool_names.get(&internal_call_id).cloned().unwrap_or_default();
        let _ = tx.send(StreamProgress::ToolResult {
          name,
          summary: first_text(&tool_result.content),
        });
      }
      Ok(MultiTurnStreamItem::FinalResponse(res)) => {
        final_text = res.response().to_string();
        break;
      }
      Ok(_) => {}
      Err(e) => return Err(AiError::Other(e.to_string())),
    }
  }

  Ok(final_text)
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
