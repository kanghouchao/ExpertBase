//! workshop インターフェイス層。Tauri コマンド（IPC アダプタ）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{Manager, State};

use crate::ai::{ChatTurn, StreamProgress, StructureResult};
use crate::kb::material;
use crate::workshop::application;

/// 停止ボタン用の共有中断フラグ。lib.rs で app.manage する。
/// Ollama は直列なので生成は同時に 1 本だけ ＝ 単一フラグで足りる。
/// ponytail: 単飛フラグ。並列生成が要るようになったら per-run の登録表に替える。
#[derive(Default)]
pub struct WorkshopCancel(pub Arc<AtomicBool>);

/// 草稿生成の進捗イベント。フロントの Channel へ送る（右側 status バーのフェーズ表示）。
#[derive(Clone, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum DraftEvent {
  /// 関連既存条目を FTS で検索中。
  Retrieving,
  /// 推論トレース（thinking）の増分。
  Thinking { delta: String },
  /// リクエスト送信済み・最初のトークン待ち（モデルのロード中を含む）。
  LoadingModel,
  /// 起草（Pass1）のトークン受信中。chars は累積文字数。
  Generating { chars: usize },
  /// 整理（Pass2）のトークン受信中。起草と区別してフェーズ表示する。
  Structuring { chars: usize },
  /// ユーザー向けナレーションの増分（思考モデルの散文）。会話に過程テキストを流す。
  Narration { delta: String },
}

impl From<StreamProgress> for DraftEvent {
  fn from(p: StreamProgress) -> Self {
    match p {
      StreamProgress::Retrieving => DraftEvent::Retrieving,
      StreamProgress::Thinking { delta } => DraftEvent::Thinking { delta },
      StreamProgress::LoadingModel => DraftEvent::LoadingModel,
      StreamProgress::Generating { chars } => DraftEvent::Generating { chars },
      StreamProgress::Structuring { chars } => DraftEvent::Structuring { chars },
      StreamProgress::Narration { delta } => DraftEvent::Narration { delta },
    }
  }
}

/// 複数の受信箱素材 + 会話履歴から AI 構造化（草稿 or 会話返信）を生成する。
/// 各素材の本文を区切り線で連結し、1 つの source として加工する。会話の記憶は
/// フロントが組み立てた messages（多輪）で渡る。
/// KB 読み込み・FTS・Ollama 呼び出しはブロッキングなので別スレッドへ。進捗は on_event で上報。
#[tauri::command]
pub async fn workshop_draft(
  app: tauri::AppHandle,
  cancel: State<'_, WorkshopCancel>,
  inbox_paths: Vec<String>,
  messages: Vec<ChatTurn>,
  model: String,
  think: bool,
  on_event: Channel<DraftEvent>,
) -> Result<StructureResult, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  // 新しい生成の開始時にフラグを倒す（前回の停止が残らないように）。
  cancel.0.store(false, Ordering::Relaxed);
  let cancel_flag = cancel.0.clone();
  let joined = tauri::async_runtime::spawn_blocking(move || -> Result<StructureResult, String> {
    let (root, conn) = crate::kb::open_active(&home)?;
    let mut bodies = Vec::with_capacity(inbox_paths.len());
    for inbox_path in &inbox_paths {
      let inbox_rel = crate::kb::checked_kb_markdown_path(inbox_path, "inbox")?;
      let raw = std::fs::read_to_string(root.join(inbox_rel)).map_err(|e| e.to_string())?;
      bodies.push(material::parse_material(&raw)?.body);
    }
    let source_text = bodies.join("\n\n---\n\n");
    let provider =
      crate::ai::ollama::OllamaProvider::with_model_think(model, think).with_cancel(cancel_flag);
    let mut on_progress = |p: StreamProgress| {
      let _ = on_event.send(DraftEvent::from(p));
    };
    application::draft(&provider, &conn, &source_text, messages, &mut on_progress)
      .map_err(|e| e.to_string())
  })
  .await;

  match joined {
    Ok(inner) => inner,
    Err(e) => Err(e.to_string()),
  }
}

/// 進行中の生成を中断する（停止ボタン）。共有フラグを立てるだけ。
/// stream 消費ループが次のチャンク前に検知して打ち切り、接続を drop して Ollama 側も止める。
#[tauri::command]
pub fn workshop_cancel(cancel: State<'_, WorkshopCancel>) {
  cancel.0.store(true, Ordering::Relaxed);
}

/// 承認内容を条目として確定する（UI で手編集済みの値を受け取る）。
/// source の受信箱は全て processed になる。
#[tauri::command]
pub fn workshop_confirm(
  app: tauri::AppHandle,
  inbox_paths: Vec<String>,
  title: String,
  cat: String,
  body: String,
) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = crate::kb::open_active(&home)?;
  let mut inbox_rels = Vec::with_capacity(inbox_paths.len());
  for inbox_path in &inbox_paths {
    let rel = crate::kb::checked_kb_markdown_path(inbox_path, "inbox")?;
    inbox_rels.push(rel.to_string_lossy().to_string());
  }
  application::confirm(&root, &conn, &title, &cat, &body, &inbox_rels)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn draft_event_serializes_with_phase_tag() {
    let gen = serde_json::to_value(DraftEvent::from(StreamProgress::Generating { chars: 7 })).unwrap();
    assert_eq!(gen["phase"], "generating");
    assert_eq!(gen["chars"], 7);
    let load = serde_json::to_value(DraftEvent::from(StreamProgress::LoadingModel)).unwrap();
    assert_eq!(load["phase"], "loadingModel");
    let retr = serde_json::to_value(DraftEvent::from(StreamProgress::Retrieving)).unwrap();
    assert_eq!(retr["phase"], "retrieving");
    let think =
      serde_json::to_value(DraftEvent::from(StreamProgress::Thinking { delta: "x".into() })).unwrap();
    assert_eq!(think["phase"], "thinking");
    assert_eq!(think["delta"], "x");
    // Pass2（整理）は Structuring として上報し、起草と区別できる。
    let struc =
      serde_json::to_value(DraftEvent::from(StreamProgress::Structuring { chars: 5 })).unwrap();
    assert_eq!(struc["phase"], "structuring");
    assert_eq!(struc["chars"], 5);
    // ナレーションは実テキスト（delta）を運ぶ。
    let narr =
      serde_json::to_value(DraftEvent::from(StreamProgress::Narration { delta: "本文".into() }))
        .unwrap();
    assert_eq!(narr["phase"], "narration");
    assert_eq!(narr["delta"], "本文");
  }
}
