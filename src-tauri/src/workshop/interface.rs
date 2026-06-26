//! workshop インターフェイス層。Tauri コマンド（IPC アダプタ）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{Manager, State};

use crate::ai::{ChatTurn, StreamProgress};
use crate::kb::material;
use crate::workshop::application;

/// 停止ボタン用の共有中断フラグ。lib.rs で app.manage する。
/// Ollama は直列なので生成は同時に 1 本だけ ＝ 単一フラグで足りる。
/// ponytail: 単飛フラグ。並列生成が要るようになったら per-run の登録表に替える。
#[derive(Default)]
pub struct WorkshopCancel(pub Arc<AtomicBool>);

/// 対話の進捗イベント。フロントの Channel へ送る（思考・本文・ツール呼び出しを会話流へ）。
#[derive(Clone, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum ChatEvent {
  /// 推論トレース（thinking）の増分。
  Thinking { delta: String },
  /// リクエスト送信済み・最初のトークン待ち（モデルのロード中を含む）。
  LoadingModel,
  /// ユーザー向け本文（モデルの返信）の増分。会話に過程テキストを流す。
  Narration { delta: String },
  /// エージェントがツールを呼び出した（検索・書き込みなど）。会話にカードで見せる。args は表示用 JSON 文字列。
  ToolCall { name: String, args: String },
  /// ツール実行結果の要約。呼び出しカードに続けて見せる。
  ToolResult { name: String, summary: String },
}

impl From<StreamProgress> for ChatEvent {
  fn from(p: StreamProgress) -> Self {
    match p {
      StreamProgress::Thinking { delta } => ChatEvent::Thinking { delta },
      StreamProgress::LoadingModel => ChatEvent::LoadingModel,
      StreamProgress::Narration { delta } => ChatEvent::Narration { delta },
      StreamProgress::ToolCall { name, args } => ChatEvent::ToolCall { name, args },
      StreamProgress::ToolResult { name, summary } => ChatEvent::ToolResult { name, summary },
    }
  }
}

/// 複数の受信箱素材 + 会話履歴で対話エージェントを 1 会話分回す。各素材の本文を区切り線で
/// 連結し、1 つの素材文脈として system に pin する。会話の記憶はフロントが組み立てた messages。
/// tools 対応モデルは search_kb / write_entry を使える（書き込みはユーザーが対話で頼んだとき）。
/// 受信箱読み込みはブロッキングなので別スレッドへ。Rig エージェント（async）は spawn し、進捗を
/// mpsc 経由で受けて Channel へ転送する（ストリームのコールバック Send 制約を回避）。
/// 戻り値は最終的な助手の返信本文。
#[tauri::command]
pub async fn workshop_chat(
  app: tauri::AppHandle,
  cancel: State<'_, WorkshopCancel>,
  inbox_paths: Vec<String>,
  messages: Vec<ChatTurn>,
  model: String,
  think: bool,
  tools: bool,
  on_event: Channel<ChatEvent>,
) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  // 新しい生成の開始時にフラグを倒す（前回の停止が残らないように）。
  cancel.0.store(false, Ordering::Relaxed);
  let cancel_flag = cancel.0.clone();

  // 受信箱素材の読み込みはブロッキング IO。root + 連結素材 + inbox_rels を別スレッドで用意する。
  let (root, source_text, inbox_rels) =
    tauri::async_runtime::spawn_blocking(move || -> Result<(PathBuf, String, Vec<String>), String> {
      let (root, _conn) = crate::kb::open_active(&home)?;
      let mut bodies = Vec::with_capacity(inbox_paths.len());
      let mut inbox_rels = Vec::with_capacity(inbox_paths.len());
      for inbox_path in &inbox_paths {
        let inbox_rel = crate::kb::checked_kb_markdown_path(inbox_path, "inbox")?;
        let rel_str = inbox_rel.to_string_lossy().to_string();
        let raw = std::fs::read_to_string(root.join(&inbox_rel)).map_err(|e| e.to_string())?;
        bodies.push(material::parse_material(&raw)?.body);
        inbox_rels.push(rel_str);
      }
      Ok((root, bodies.join("\n\n---\n\n"), inbox_rels))
    })
    .await
    .map_err(|e| e.to_string())??;

  // Rig エージェントを spawn し、進捗 mpsc を Channel へ排出する。tx が drop されると rx が閉じる。
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StreamProgress>();
  let agent = tauri::async_runtime::spawn(application::chat(
    model, think, root, source_text, inbox_rels, messages, tools, cancel_flag, tx,
  ));
  while let Some(p) = rx.recv().await {
    let _ = on_event.send(ChatEvent::from(p));
  }
  agent.await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

/// 進行中の生成を中断する（停止ボタン）。共有フラグを立てるだけ。
/// stream 消費ループが次のチャンク前に検知して打ち切り、接続を drop して Ollama 側も止める。
#[tauri::command]
pub fn workshop_cancel(cancel: State<'_, WorkshopCancel>) {
  cancel.0.store(true, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn chat_event_serializes_with_phase_tag() {
    let load = serde_json::to_value(ChatEvent::from(StreamProgress::LoadingModel)).unwrap();
    assert_eq!(load["phase"], "loadingModel");
    let think =
      serde_json::to_value(ChatEvent::from(StreamProgress::Thinking { delta: "x".into() })).unwrap();
    assert_eq!(think["phase"], "thinking");
    assert_eq!(think["delta"], "x");
    // ナレーションは実テキスト（delta）を運ぶ。
    let narr =
      serde_json::to_value(ChatEvent::from(StreamProgress::Narration { delta: "本文".into() }))
        .unwrap();
    assert_eq!(narr["phase"], "narration");
    assert_eq!(narr["delta"], "本文");
    // ツール呼び出し / 結果。
    let call = serde_json::to_value(ChatEvent::from(StreamProgress::ToolCall {
      name: "write_entry".into(),
      args: "{}".into(),
    }))
    .unwrap();
    assert_eq!(call["phase"], "toolCall");
    assert_eq!(call["name"], "write_entry");
    let res = serde_json::to_value(ChatEvent::from(StreamProgress::ToolResult {
      name: "write_entry".into(),
      summary: "saved 緑茶".into(),
    }))
    .unwrap();
    assert_eq!(res["phase"], "toolResult");
    assert_eq!(res["summary"], "saved 緑茶");
  }
}
