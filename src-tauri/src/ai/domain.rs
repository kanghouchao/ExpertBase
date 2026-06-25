//! ai ドメイン層。AI プロバイダのポート（trait）と境界 DTO、ドメインエラー。
//! 具体的な HTTP/プロバイダ実装には依存しない。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// FTS で引いた関連既存条目の要約（title + excerpt）。
#[derive(Clone, Debug)]
pub struct EntrySummary {
  pub title: String,
  pub excerpt: String,
}

/// 会話の 1 ターン。フロントが履歴を組み立てて渡す（多輪・記憶あり）。
/// role は "user" / "assistant"。
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatTurn {
  pub role: String,
  pub content: String,
}

/// 構造化リクエスト（ワークショップが組み立てる）。
#[derive(Clone, Debug)]
pub struct StructureRequest {
  /// 新素材の本文（文字を持つもののみ）。
  pub source_text: String,
  /// FTS で引いた関連既存条目。
  pub related: Vec<EntrySummary>,
  /// ユーザーとの会話履歴（最後が最新のユーザー発話）。
  pub messages: Vec<ChatTurn>,
}

/// 構造化結果。kind が "chat" のときは body_markdown に会話返信が入り、
/// title/cat/suggested_links は空になる。
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StructureResult {
  /// "entry"（条目草稿）か "chat"（会話返信）か。
  pub kind: String,
  pub title: String,
  pub cat: String,
  pub body_markdown: String,
  /// 既存条目タイトルから選ばれたリンク候補。
  pub suggested_links: Vec<String>,
}

/// ストリーミング進捗。Tauri 非依存のドメイン値（interface 層が Channel へ橋渡しする）。
#[derive(Clone, Debug, PartialEq)]
pub enum StreamProgress {
  /// 関連既存条目を FTS で検索中（モデル呼び出し前の確定的な段）。
  Retrieving,
  /// 推論トレース（thinking）の増分。思考モデルのみ。
  Thinking { delta: String },
  /// リクエスト送信済み・最初のトークン待ち（モデルのロード中を含む）。
  LoadingModel,
  /// 起草（Pass1）の本文受信中。chars は累積文字数。
  Generating { chars: usize },
  /// 整理（Pass2）の本文受信中。format 固定でドラフトを構造化する段。
  Structuring { chars: usize },
  /// ユーザー向けナレーションの増分（思考モデルの Pass1 散文＝「AI が今書いている本文」）。
  /// 数字ではなく実テキストを流し、会話で過程を見せる。delta は新着分のみ。
  Narration { delta: String },
  /// エージェントがツールを呼び出した（検索など）。会話にカードで見せる。args は表示用 JSON 文字列。
  ToolCall { name: String, args: String },
  /// ツール実行結果の要約（件数など）。呼び出しカードに続けて見せる。
  ToolResult { name: String, summary: String },
}

/// エージェント会話の 1 メッセージ（プロバイダ非依存）。provider が各 API の wire 形式へ変換する。
#[derive(Clone, Debug, PartialEq)]
pub enum AgentMsg {
  User(String),
  /// モデルの応答（本文 + ツール呼び出し）。ツール結果を返す前に履歴へ積む。
  Assistant { content: String, tool_calls: Vec<ToolCall> },
  /// ツール実行結果。name はどのツールか、content は結果テキスト。
  Tool { name: String, content: String },
}

/// モデルが要求したツール呼び出し。args はツールの引数（JSON オブジェクト）。
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
  pub name: String,
  pub args: Value,
}

/// agent_turn の出力。thinking / narration は on_progress で流し済み。
/// tool_calls が空＝モデルが最終応答（draft 本文）を返した、と解釈する。
#[derive(Clone, Debug, PartialEq)]
pub struct TurnOutcome {
  pub content: String,
  pub tool_calls: Vec<ToolCall>,
}

/// プロバイダ非依存のツール定義。各 API の wire 形式（OpenAI 互換 function ラッパ等）へ
/// 包むのは infra の責務。parameters は引数の JSON スキーマ（中立な記述言語なので domain に置く）。
#[derive(Clone, Debug)]
pub struct ToolDef {
  pub name: String,
  pub description: String,
  pub parameters: Value,
}

/// AI エラー。UI で区別して表示し、手動パスへ退避できるようにする。
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

/// AI プロバイダ接合面。ワークショップはこの trait の裏でのみ AI を呼ぶ。
/// 将来のローカル LLM / マルチモーダルは別実装として差し込む（下流は変更不要）。
pub trait AiProvider {
  /// 非エージェント経路（ツール非対応モデルのフォールバック）。素材から直接構造化する。
  fn structure(
    &self,
    req: StructureRequest,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError>;

  /// エージェントの 1 ターン。system + tools + 会話を渡し、本文 + ツール呼び出しを返す。
  /// tools は中立な ToolDef 列で渡し、wire 形式への変換は実装（infra）が行う。
  /// thinking / narration は on_progress で流す。中断は実装側が内部フラグで処理する。
  fn agent_turn(
    &self,
    system: &str,
    tools: &[ToolDef],
    messages: &[AgentMsg],
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<TurnOutcome, AiError>;

  /// エージェントが書いた散文ドラフトを最終 StructureResult へ整形する（＝ Pass2 を再利用）。
  fn structure_draft(
    &self,
    draft: &str,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError>;
}

/// テスト用の決定的プロバイダ（ネットワーク不要）。
#[cfg(test)]
pub struct FakeProvider;

#[cfg(test)]
impl AiProvider for FakeProvider {
  fn structure(
    &self,
    req: StructureRequest,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError> {
    on_progress(StreamProgress::LoadingModel);
    let title = req.source_text.lines().next().unwrap_or("").trim().to_string();
    on_progress(StreamProgress::Generating { chars: req.source_text.chars().count() });
    let suggested_links = req.related.iter().take(3).map(|e| e.title.clone()).collect();
    Ok(StructureResult {
      kind: "entry".into(),
      title: if title.is_empty() { "無題".into() } else { title },
      cat: "uncategorized".into(),
      body_markdown: req.source_text.clone(),
      suggested_links,
    })
  }

  /// 既定では即座に最終応答を返す（ツールを使わない）。ループ自体の検証は
  /// workshop テストの脚本化プロバイダで行う。content は直近 User 発話のエコー。
  fn agent_turn(
    &self,
    _system: &str,
    _tools: &[ToolDef],
    messages: &[AgentMsg],
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<TurnOutcome, AiError> {
    on_progress(StreamProgress::LoadingModel);
    let content = messages
      .iter()
      .rev()
      .find_map(|m| match m {
        AgentMsg::User(text) => Some(text.clone()),
        _ => None,
      })
      .unwrap_or_default();
    Ok(TurnOutcome { content, tool_calls: vec![] })
  }

  fn structure_draft(
    &self,
    draft: &str,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError> {
    on_progress(StreamProgress::Structuring { chars: draft.chars().count() });
    let title = draft.lines().next().unwrap_or("").trim().to_string();
    Ok(StructureResult {
      kind: "entry".into(),
      title: if title.is_empty() { "無題".into() } else { title },
      cat: "uncategorized".into(),
      body_markdown: draft.to_string(),
      suggested_links: vec![],
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fake_provider_echoes_and_suggests_related_links() {
    let req = StructureRequest {
      source_text: "緑茶の淹れ方\n\n本文".into(),
      related: vec![EntrySummary { title: "煎茶".into(), excerpt: "...".into() }],
      messages: vec![ChatTurn { role: "user".into(), content: "整理して".into() }],
    };
    let res = FakeProvider.structure(req, &mut |_| {}).unwrap();
    assert_eq!(res.kind, "entry");
    assert_eq!(res.title, "緑茶の淹れ方");
    assert_eq!(res.suggested_links, vec!["煎茶".to_string()]);
  }

  #[test]
  fn fake_provider_reports_loading_then_generating() {
    let req = StructureRequest {
      source_text: "abc".into(),
      related: vec![],
      messages: vec![],
    };
    let mut events = Vec::new();
    FakeProvider.structure(req, &mut |p| events.push(p)).unwrap();
    assert_eq!(events.first(), Some(&StreamProgress::LoadingModel));
    assert!(matches!(events.last(), Some(StreamProgress::Generating { chars: 3 })));
  }

  #[test]
  fn ai_error_displays_messages() {
    assert_eq!(AiError::Other("x".into()).to_string(), "x");
  }
}
