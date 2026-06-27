//! workshop インターフェイス層。Tauri コマンド（IPC アダプタ）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{Manager, State};

use crate::ai::{ChatTurn, StreamProgress};
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

/// 添付素材 + 会話履歴で対話エージェントを 1 会話分回す。素材は本文を注入せず、検証済みの id
/// 一覧（sources）として渡し、AI が read_source で自分で読む。会話の記憶はフロントが組み立てた
/// messages。tools 対応モデル必須で read_source / search_kb / write_entry を使える（書き込みは
/// ユーザーが対話で頼んだとき）。素材 id の検証はブロッキングなので別スレッドへ。Rig エージェント
/// （async）は spawn し、進捗を mpsc 経由で受けて Channel へ転送する（ストリームのコールバック
/// Send 制約を回避）。戻り値は最終的な助手の返信本文。
/// `tools` は前端の互換のため受けるが分岐しない（Phase 3 で前端が tools モデルを必須化する）。
#[tauri::command]
pub async fn workshop_chat(
  app: tauri::AppHandle,
  cancel: State<'_, WorkshopCancel>,
  source_ids: Vec<String>,
  messages: Vec<ChatTurn>,
  model: String,
  think: bool,
  tools: bool,
  on_event: Channel<ChatEvent>,
) -> Result<String, String> {
  let _ = tools;
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  // 新しい生成の開始時にフラグを倒す（前回の停止が残らないように）。
  cancel.0.store(false, Ordering::Relaxed);
  let cancel_flag = cancel.0.clone();

  // 素材 id の検証はブロッキング寄り。root + 検証済み sources を別スレッドで用意。
  // 本文はここでは読まない＝AI が read_source で個別に読む。id は inbox 相対 | ファイル絶対の混在：
  // 絶対パスはユーザーがダイアログで選んだ外部ファイルとしてそのまま許可、それ以外は inbox 内に限定検証。
  let (root, sources) =
    tauri::async_runtime::spawn_blocking(move || -> Result<(PathBuf, Vec<String>), String> {
      let (root, _conn) = crate::kb::open_active(&home)?;
      let mut sources = Vec::with_capacity(source_ids.len());
      for id in &source_ids {
        if std::path::Path::new(id).is_absolute() {
          sources.push(id.clone());
        } else {
          let inbox_rel = crate::kb::checked_kb_markdown_path(id, "inbox")?;
          sources.push(inbox_rel.to_string_lossy().to_string());
        }
      }
      Ok((root, sources))
    })
    .await
    .map_err(|e| e.to_string())??;

  // Rig エージェントを spawn し、進捗 mpsc を Channel へ排出する。tx が drop されると rx が閉じる。
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StreamProgress>();
  let agent = tauri::async_runtime::spawn(application::chat(
    model, think, root, sources, messages, cancel_flag, tx,
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
