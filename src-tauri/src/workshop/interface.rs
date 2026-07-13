//! workshop インターフェイス層。Tauri コマンド（IPC アダプタ）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{Manager, State};

use crate::agent::{ChatTurn, StreamProgress};
use crate::error::AppError;
use crate::workshop::application;
use crate::workshop::domain::{WorkshopConversation, WorkshopConversationPage, WorkshopMessage};
use crate::workshop::infrastructure::confirm;

/// 停止ボタン用の共有中断フラグ。lib.rs で app.manage する。
/// Ollama は直列なので生成は同時に 1 本だけ ＝ 単一フラグで足りる。
/// ponytail: 単飛フラグ。並列生成が要るようになったら per-run の登録表に替える。
#[derive(Default)]
pub struct WorkshopCancel(pub Arc<AtomicBool>);

/// 未応答の確認要求（id → 応答チャネル）の共有表。lib.rs で app.manage する。
#[derive(Default)]
pub struct WorkshopConfirm(pub confirm::PendingConfirms);

/// 確認要求へのユーザー応答を回填する（許可 / 拒否）。未知 id は無視（取消・超時済み）。
#[tauri::command]
pub fn workshop_confirm(confirms: State<'_, WorkshopConfirm>, id: u64, approved: bool) {
  confirm::resolve(&confirms.0, id, approved);
}

/// 完了済みの対話をアクティブ KB の履歴へ保存する。
#[tauri::command]
pub async fn workshop_save_conversation(
  app: tauri::AppHandle,
  kb_path: String,
  id: Option<i64>,
  source_ids: Vec<String>,
  messages: Vec<WorkshopMessage>,
) -> Result<WorkshopConversation, AppError> {
  let home = app.path().home_dir().map_err(AppError::generic)?;
  tauri::async_runtime::spawn_blocking(move || {
    application::save_active_conversation(&home, &kb_path, id, source_ids, messages)
  })
  .await
  .map_err(AppError::generic)?
}

/// アクティブ KB から指定した対話を取得する。
#[tauri::command]
pub async fn workshop_get_conversation(
  app: tauri::AppHandle,
  id: i64,
) -> Result<WorkshopConversation, AppError> {
  let home = app.path().home_dir().map_err(AppError::generic)?;
  tauri::async_runtime::spawn_blocking(move || {
    let root = crate::kb::active_kb_root(&home)?;
    application::get_conversation(&root, id)
  })
  .await
  .map_err(AppError::generic)?
}

/// アクティブ KB の対話履歴を更新日時の降順で取得する。
#[tauri::command]
pub async fn workshop_list_conversations(
  app: tauri::AppHandle,
  offset: usize,
) -> Result<WorkshopConversationPage, AppError> {
  let home = app.path().home_dir().map_err(AppError::generic)?;
  tauri::async_runtime::spawn_blocking(move || {
    let root = crate::kb::active_kb_root(&home)?;
    application::list_conversations(&root, offset)
  })
  .await
  .map_err(AppError::generic)?
}

/// 添付素材 + 会話履歴で対話エージェントを 1 会話分回す。素材は本文を注入せず、検証済みの id
/// 一覧（sources）として渡し、AI が read_source で自分で読む。会話の記憶はフロントが組み立てた
/// messages。素材 id の検証 + 技能発見はブロッキングなので別スレッドへ。Rig エージェント
/// （async）は spawn し、進捗を mpsc 経由で受けて Channel へ転送する（ストリームのコールバック
/// Send 制約を回避）。戻り値は最終的な助手の返信本文。
/// `tools`（前端が算出したモデルの tools 能力）は skills catalog / `activate_skill` 登録の
/// 能力ゲートとして使う（tools 非対応モデルには載せない）。`activated_skill_names` は
/// フロントが管理する「この会話で発動済みの技能名」（ボタン発動・モデル自動発動を問わず一本化）で、
/// 対応する技能本文は tools 能力に関わらず system prompt へ注入する（明示発動は tools 能力に依存しない）。
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn workshop_chat(
  app: tauri::AppHandle,
  cancel: State<'_, WorkshopCancel>,
  confirms: State<'_, WorkshopConfirm>,
  source_ids: Vec<String>,
  messages: Vec<ChatTurn>,
  model: String,
  think: bool,
  tools: bool,
  activated_skill_names: Vec<String>,
  on_event: Channel<StreamProgress>,
) -> Result<String, AppError> {
  let home = app.path().home_dir().map_err(AppError::generic)?;
  // 新しい生成の開始時にフラグを倒す（前回の停止が残らないように）。
  cancel.0.store(false, Ordering::Relaxed);
  let cancel_flag = cancel.0.clone();

  // 素材 id の検証 + Agent 設定の読み込み + 技能発見はブロッキング寄り。
  // 本文はここでは読まない＝AI が read_source で個別に読む。id は外部ファイルの絶対パスのみ。
  // 設定はグローバル（前端の設定画面で編集）。model は会話ごとに前端が渡す。
  let (root, sources, settings, skills) = tauri::async_runtime::spawn_blocking(
    move || -> Result<
      (PathBuf, Vec<String>, crate::agent::AiSettings, Vec<crate::plugin::Skill>),
      AppError,
    > {
      let (root, _conn) = crate::kb::open_active(&home)?;
      let mut sources = Vec::with_capacity(source_ids.len());
      for id in &source_ids {
        if std::path::Path::new(id).is_absolute() {
          sources.push(id.clone());
        } else {
          return Err(AppError::param("err.workshop.sourceMustBeAbsolute", "id", id));
        }
      }
      let settings = crate::agent::settings_store::load(&home)?;
      let skills = crate::plugin::discover_skills(Some(&root), &home);
      Ok((root, sources, settings, skills))
    },
  )
  .await
  .map_err(AppError::generic)??;

  // Rig エージェントを spawn し、進捗 mpsc を Channel へ排出する。tx が drop されると rx が閉じる。
  let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StreamProgress>();
  let pending = confirms.0.clone();
  let agent = tauri::async_runtime::spawn(application::chat(
    settings,
    model,
    think,
    tools,
    root,
    sources,
    skills,
    activated_skill_names,
    messages,
    cancel_flag,
    tx,
    pending,
  ));
  while let Some(p) = rx.recv().await {
    if on_event.send(p).is_err() {
      // UI チャネル切断＝確認カードは誰にも見えない。未応答の確認を即拒否して塞がない。
      confirm::deny_all(&confirms.0);
    }
  }
  agent.await.map_err(AppError::generic)?
}

/// 進行中の生成を中断する（停止ボタン）。共有フラグを立てるだけ。
/// stream 消費ループが次のチャンク前に検知して打ち切り、接続を drop して Ollama 側も止める。
#[tauri::command]
pub fn workshop_cancel(cancel: State<'_, WorkshopCancel>) {
  cancel.0.store(true, Ordering::Relaxed);
}
