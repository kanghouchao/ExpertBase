//! ai ドメイン層。AI プロバイダのポート（trait）と境界 DTO、ドメインエラー。
//! 具体的な HTTP/プロバイダ実装には依存しない。

use serde::{Deserialize, Serialize};

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
  /// 本文（content）受信中。chars は累積文字数。
  Generating { chars: usize },
}

/// AI エラー。UI で区別して表示し、手動パスへ退避できるようにする。
#[derive(Debug, PartialEq)]
pub enum AiError {
  /// ネットワーク障害。
  Network(String),
  /// その他（API エラー応答・解析失敗など）。
  Other(String),
}

impl std::fmt::Display for AiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AiError::Network(m) => write!(f, "网络错误: {m}"),
      AiError::Other(m) => write!(f, "{m}"),
    }
  }
}

/// AI プロバイダ接合面。ワークショップはこの trait の裏でのみ AI を呼ぶ。
/// 将来のローカル LLM / マルチモーダルは別実装として差し込む（下流は変更不要）。
pub trait AiProvider {
  fn structure(
    &self,
    req: StructureRequest,
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
