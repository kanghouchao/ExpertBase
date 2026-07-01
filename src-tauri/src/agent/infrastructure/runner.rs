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
use rig_core::client::{CompletionClient, Nothing};
use rig_core::completion::{CompletionModel, GetTokenUsage};
use rig_core::message::{Message, Text, ToolResultContent};
use rig_core::providers::{ollama, openai};
use rig_core::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingChat};
use rig_core::tool::ToolDyn;
use rig_core::OneOrMany;

use crate::agent::{AiError, ChatTurn, Provider, StreamProgress};

/// エージェントの暴走（無限ツール呼び出し）を抑える反復上限。
const MAX_TURNS: usize = 6;

/// 注入されたツールで 1 会話分回す。system + 履歴 + 直近プロンプトを渡し、思考・本文・
/// ツール呼び出し/結果を tx へ流しつつ最終本文を返す。provider ごとにクライアント構築だけ分岐する。
#[allow(clippy::too_many_arguments)]
pub async fn run(
  provider: Provider,
  base_url: Option<&str>,
  model: &str,
  think: bool,
  system: &str,
  tools: Vec<Box<dyn ToolDyn>>,
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: &UnboundedSender<StreamProgress>,
) -> Result<String, AiError> {
  // 直近のユーザー発話をプロンプト、それ以前を履歴に分ける。
  let Some((last, rest)) = messages.split_last() else {
    return Err(AiError::Other("対話メッセージが空です".into()));
  };
  let history: Vec<Message> = rest.iter().map(turn_to_message).collect();
  let prompt = turn_to_message(last);

  // 送信済み・最初のトークン待ち（モデルのロード中を含む）。
  let _ = tx.send(StreamProgress::LoadingModel);

  match provider {
    Provider::Ollama => {
      let client = ollama::Client::new(Nothing).map_err(|e| AiError::Network(e.to_string()))?;
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
      // ponytail: llama.app は OpenAI 互換のローカル端点と仮定。独自プロトコルならこの arm 本体だけ差し替える。
      // interface は未入力でも Some("") を渡す（None にならない）ので、空白も未設定として弾く。
      let base_url = base_url.map(str::trim).filter(|s| !s.is_empty());
      let Some(base_url) = base_url else {
        return Err(AiError::Other("llama.app の URL が未設定です".into()));
      };
      let client = openai::Client::builder()
        .api_key("expertbase-local") // ローカル端点は key 不要。ダミーを渡す。
        .base_url(base_url)
        .build()
        .map_err(|e| AiError::Network(e.to_string()))?;
      let agent = client.agent(model).preamble(system).temperature(0.6).tools(tools).build();
      drive(agent, prompt, history, cancel, tx).await
    }
  }
}

/// 組み上げた `Agent<M>` を回す共通ループ（provider 非依存、provider ごとに単態化）。
async fn drive<M>(
  agent: Agent<M>,
  prompt: Message,
  history: Vec<Message>,
  cancel: Arc<AtomicBool>,
  tx: &UnboundedSender<StreamProgress>,
) -> Result<String, AiError>
where
  M: CompletionModel + 'static,
  M::StreamingResponse: GetTokenUsage,
{
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

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn run_errors_when_llama_app_url_missing() {
    // llama.app 選択で URL 未設定なら、サーバへ触れず即エラー（縫い目のガード）。
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let err = run(
      Provider::LlamaApp,
      None,
      "some-model",
      false,
      "system",
      vec![],
      vec![ChatTurn { role: "user".into(), content: "hi".into() }],
      cancel,
      &tx,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, AiError::Other(_)));
  }

  #[tokio::test]
  async fn run_errors_when_llama_app_url_blank() {
    // URL が空文字（前端が未入力のまま送る実経路。interface は Some("") を渡す）でも、
    // None と同様に即エラーにする＝ base_url None ガードだけでは実運用の未設定を捕らえられない。
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let err = run(
      Provider::LlamaApp,
      Some("   "),
      "some-model",
      false,
      "system",
      vec![],
      vec![ChatTurn { role: "user".into(), content: "hi".into() }],
      cancel,
      &tx,
    )
    .await
    .unwrap_err();
    // variant ではなく明示メッセージで判定する（drive はどんな流エラーも Other に包むため、
    // variant だけでは「未設定ガード」と「サーバ接続失敗」を区別できない）。
    assert!(err.to_string().contains("未設定"), "空 URL は未設定ガードに落ちるべき: {err}");
  }

  #[tokio::test]
  async fn run_errors_on_empty_messages() {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let err = run(Provider::Ollama, None, "m", false, "s", vec![], vec![], cancel, &tx)
      .await
      .unwrap_err();
    assert!(matches!(err, AiError::Other(_)));
  }
}
