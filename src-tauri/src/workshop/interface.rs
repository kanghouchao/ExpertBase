//! workshop インターフェイス層。Tauri コマンド（IPC アダプタ）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{Manager, State};

use crate::agent::{ChatTurn, Provider, StreamProgress};
use crate::workshop::application;
use crate::workshop::domain::{WorkshopConversation, WorkshopConversationPage, WorkshopMessage};

/// 停止ボタン用の共有中断フラグ。lib.rs で app.manage する。
/// Ollama は直列なので生成は同時に 1 本だけ ＝ 単一フラグで足りる。
/// ponytail: 単飛フラグ。並列生成が要るようになったら per-run の登録表に替える。
#[derive(Default)]
pub struct WorkshopCancel(pub Arc<AtomicBool>);

/// 完了済みの対話をアクティブ KB の履歴へ保存する。
#[tauri::command]
pub async fn workshop_save_conversation(
  app: tauri::AppHandle,
  kb_path: String,
  id: Option<i64>,
  source_ids: Vec<String>,
  messages: Vec<WorkshopMessage>,
) -> Result<WorkshopConversation, String> {
  let home = app.path().home_dir().map_err(|error| error.to_string())?;
  tauri::async_runtime::spawn_blocking(move || {
    application::save_active_conversation(&home, &kb_path, id, source_ids, messages)
  })
  .await
  .map_err(|error| error.to_string())?
}

/// アクティブ KB から指定した対話を取得する。
#[tauri::command]
pub async fn workshop_get_conversation(
  app: tauri::AppHandle,
  id: i64,
) -> Result<WorkshopConversation, String> {
  let home = app.path().home_dir().map_err(|error| error.to_string())?;
  tauri::async_runtime::spawn_blocking(move || {
    let root = crate::kb::active_kb_root(&home)?;
    application::get_conversation(&root, id)
  })
  .await
  .map_err(|error| error.to_string())?
}

/// アクティブ KB の対話履歴を更新日時の降順で取得する。
#[tauri::command]
pub async fn workshop_list_conversations(
  app: tauri::AppHandle,
  offset: usize,
) -> Result<WorkshopConversationPage, String> {
  let home = app.path().home_dir().map_err(|error| error.to_string())?;
  tauri::async_runtime::spawn_blocking(move || {
    let root = crate::kb::active_kb_root(&home)?;
    application::list_conversations(&root, offset)
  })
  .await
  .map_err(|error| error.to_string())?
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
  on_event: Channel<StreamProgress>,
) -> Result<String, String> {
  let _ = tools;
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  // 新しい生成の開始時にフラグを倒す（前回の停止が残らないように）。
  cancel.0.store(false, Ordering::Relaxed);
  let cancel_flag = cancel.0.clone();

  // 素材 id の検証 + AI 設定の読み込みはブロッキング寄り。root + 検証済み sources + provider を別スレッドで用意。
  // 本文はここでは読まない＝AI が read_source で個別に読む。id は外部ファイルの絶対パスのみ。
  // provider はグローバル設定（前端の設定画面で選択）。model は会話ごとに前端が渡す。
  let (root, sources, provider, base_url) = tauri::async_runtime::spawn_blocking(
    move || -> Result<(PathBuf, Vec<String>, Provider, Option<String>), String> {
      let (root, _conn) = crate::kb::open_active(&home)?;
      let mut sources = Vec::with_capacity(source_ids.len());
      for id in &source_ids {
        if std::path::Path::new(id).is_absolute() {
          sources.push(id.clone());
        } else {
          return Err(format!("source must be an absolute path: {id}"));
        }
      }
      let settings = crate::agent::settings_store::load(&home)?;
      let base_url = match settings.provider {
        Provider::LlamaApp => Some(settings.llama_app_url),
        Provider::Ollama => None,
      };
      Ok((root, sources, settings.provider, base_url))
    },
  )
  .await
  .map_err(|e| e.to_string())??;

  // Rig エージェントを spawn し、進捗 mpsc を Channel へ排出する。tx が drop されると rx が閉じる。
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StreamProgress>();
  let agent = tauri::async_runtime::spawn(application::chat(
    provider, base_url, model, think, root, sources, messages, cancel_flag, tx,
  ));
  while let Some(p) = rx.recv().await {
    let _ = on_event.send(p);
  }
  agent.await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

/// 進行中の生成を中断する（停止ボタン）。共有フラグを立てるだけ。
/// stream 消費ループが次のチャンク前に検知して打ち切り、接続を drop して Ollama 側も止める。
#[tauri::command]
pub fn workshop_cancel(cancel: State<'_, WorkshopCancel>) {
  cancel.0.store(true, Ordering::Relaxed);
}
