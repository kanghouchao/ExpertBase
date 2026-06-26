//! ai ドメイン層。境界 DTO（会話ターン・進捗）とドメインエラー。
//! エージェントのループ/ツール実体は Rig（infra）が担うので、ここはプロバイダ非依存の
//! 値だけを置く（フロント契約の ChatTurn / 進捗 StreamProgress / AiError）。

use serde::Deserialize;

/// 会話の 1 ターン。フロントが履歴を組み立てて渡す（多輪・記憶あり）。
/// role は "user" / "assistant"。
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatTurn {
  pub role: String,
  pub content: String,
}

/// ストリーミング進捗。Tauri 非依存のドメイン値（interface 層が Channel へ橋渡しする）。
#[derive(Clone, Debug, PartialEq)]
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
}
