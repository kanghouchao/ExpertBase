//! ai ドメイン層。境界 DTO（会話ターン・進捗）とドメインエラー。
//! エージェントのループ/ツール実体は Rig（infra）が担うので、ここはプロバイダ非依存の
//! 値だけを置く（フロント契約の ChatTurn / 進捗 StreamProgress / AiError）。

use serde::{Deserialize, Serialize};

/// 会話の 1 ターン。フロントが履歴を組み立てて渡す（多輪・記憶あり）。
/// role は "user" / "assistant"。
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatTurn {
  pub role: String,
  pub content: String,
}

/// ストリーミング進捗。Tauri 非依存のドメイン値だが、フロント契約（[`ChatTurn`] と同様）でもあるので
/// IPC Channel へそのまま流せるよう Serialize を持つ（phase タグ + camelCase でフロントと一致）。
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum StreamProgress {
  /// 推論トレース（thinking）の増分。思考モデルのみ。
  Thinking { delta: String },
  /// リクエスト送信済み・最初のトークン待ち（モデルのロード中を含む）。
  LoadingModel,
  /// ユーザー向け本文（モデルの返信）の増分。会話に過程テキストを流す。delta は新着分のみ。
  Narration { delta: String },
  /// エージェントがツールを呼び出した（検索・書き込みなど）。会話にカードで見せる。args は表示用 JSON 文字列。
  ToolCall { name: String, args: String },
  /// ツール実行結果の要約（件数・保存先など）。呼び出しカードに続けて見せる。
  ToolResult { name: String, summary: String },
}

/// AI エラー。UI で区別して表示できるようにする。
#[derive(Debug, PartialEq)]
pub enum AiError {
  /// ネットワーク障害。
  Network(String),
  /// その他（API エラー応答・解析失敗など）。
  Other(String),
  /// ユーザーが生成を中断した（停止ボタン）。UI はエラー表示せず idle へ戻す。
  Cancelled,
}

impl std::fmt::Display for AiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AiError::Network(m) => write!(f, "网络错误: {m}"),
      AiError::Other(m) => write!(f, "{m}"),
      AiError::Cancelled => write!(f, "已取消"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ai_error_displays_messages() {
    assert_eq!(AiError::Other("x".into()).to_string(), "x");
  }

  #[test]
  fn stream_progress_serializes_with_phase_tag() {
    // フロント契約のワイヤ形式（phase タグ + camelCase）。interface 層はこれをそのまま Channel へ流す。
    let load = serde_json::to_value(StreamProgress::LoadingModel).unwrap();
    assert_eq!(load["phase"], "loadingModel");
    let think = serde_json::to_value(StreamProgress::Thinking { delta: "x".into() }).unwrap();
    assert_eq!(think["phase"], "thinking");
    assert_eq!(think["delta"], "x");
    // ナレーションは実テキスト（delta）を運ぶ。
    let narr = serde_json::to_value(StreamProgress::Narration { delta: "本文".into() }).unwrap();
    assert_eq!(narr["phase"], "narration");
    assert_eq!(narr["delta"], "本文");
    // ツール呼び出し / 結果。
    let call = serde_json::to_value(StreamProgress::ToolCall {
      name: "write_entry".into(),
      args: "{}".into(),
    })
    .unwrap();
    assert_eq!(call["phase"], "toolCall");
    assert_eq!(call["name"], "write_entry");
    let res = serde_json::to_value(StreamProgress::ToolResult {
      name: "write_entry".into(),
      summary: "saved 緑茶".into(),
    })
    .unwrap();
    assert_eq!(res["phase"], "toolResult");
    assert_eq!(res["summary"], "saved 緑茶");
  }
}
