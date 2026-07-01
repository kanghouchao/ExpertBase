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

/// AI プロバイダの選択。本 MVP はローカル端点のみ（クラウド・API キーなし）。
/// どちらも OpenAI 互換/HTTP のローカル端点で、base_url は設定から（空欄は既定へフォールバック）。
/// 追加プロバイダは runner の match に arm を足すだけ（縫い目はここと domain）。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
  #[default]
  Ollama,
  LlamaApp,
}

/// Ollama の既定端点（ローカル常駐サービス）。
pub const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434";
/// llama.app（= llama.cpp の `llama serve`）の既定端点。OpenAI 互換 `/v1`、既定ポート 8080。
pub const DEFAULT_LLAMA_APP_URL: &str = "http://127.0.0.1:8080/v1";

impl Provider {
  /// プロバイダ既定の base URL（設定が空欄のときのフォールバック先）。
  pub fn default_base_url(self) -> &'static str {
    match self {
      Provider::Ollama => DEFAULT_OLLAMA_URL,
      Provider::LlamaApp => DEFAULT_LLAMA_APP_URL,
    }
  }
}

/// 設定の生 URL を解決する。前後空白を除き、空なら provider 既定へフォールバック。
/// 「URL は設定可能だが既定値を持つ」という要件をここ 1 箇所で表す（呼び出し側は生値を渡すだけ）。
pub fn resolve_base_url(provider: Provider, raw: &str) -> String {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    provider.default_base_url().to_string()
  } else {
    trimmed.to_string()
  }
}

/// AI 設定（`~/.expertBase/ai.toml` に永続化、前端の設定画面で編集）。
/// provider はグローバル選択、model は既定モデル、ollama_url / llama_app_url は各端点（空欄=既定）。
/// provider にも `#[serde(default)]` を付け、旧 ai.toml / 手編集で欠落しても既定へ倒れるようにする。
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
  #[serde(default)]
  pub provider: Provider,
  #[serde(default)]
  pub model: String,
  #[serde(default)]
  pub ollama_url: String,
  #[serde(default)]
  pub llama_app_url: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn provider_defaults_to_ollama_and_serializes_camel_case() {
    // 既定はローカルの Ollama。ワイヤ形式は camelCase（前端契約と一致）。
    assert_eq!(Provider::default(), Provider::Ollama);
    assert_eq!(serde_json::to_value(Provider::LlamaApp).unwrap(), "llamaApp");
    assert_eq!(
      serde_json::from_value::<Provider>(serde_json::json!("ollama")).unwrap(),
      Provider::Ollama
    );
  }

  #[test]
  fn ai_settings_defaults_when_provider_field_missing() {
    // provider 欠落（旧 ai.toml / 手編集）でも #[serde(default)] で Ollama に倒れる。
    let s: AiSettings = toml::from_str("model = \"qwen3:8b\"").unwrap();
    assert_eq!(s.provider, Provider::Ollama);
    assert_eq!(s.model, "qwen3:8b");
  }

  #[test]
  fn resolve_base_url_falls_back_to_provider_default_when_blank() {
    // 空欄・空白は provider 既定へ。非空は trim して採用。
    assert_eq!(resolve_base_url(Provider::Ollama, "  "), DEFAULT_OLLAMA_URL);
    assert_eq!(resolve_base_url(Provider::LlamaApp, ""), DEFAULT_LLAMA_APP_URL);
    assert_eq!(Provider::LlamaApp.default_base_url(), "http://127.0.0.1:8080/v1");
    assert_eq!(resolve_base_url(Provider::LlamaApp, " http://x:8080/v1 "), "http://x:8080/v1");
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
