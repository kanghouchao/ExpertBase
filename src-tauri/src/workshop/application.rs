//! workshop アプリケーション層。対話エージェントの編成と条目確定のユースケース。
//! kb（検索・索引・FS）と汎用 agent を編成する。ツールは infra（tools）で組んで注入し、
//! ループ/推論は agent が持つ。

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use chrono::SecondsFormat;
use rusqlite::Connection;
use tokio::sync::mpsc::UnboundedSender;

use crate::agent::{ChatTurn, Provider, StreamProgress};
use crate::error::AppError;

use super::prompt::agent_system_with;
use crate::kb::entry::{Entry, EntryMeta};
use crate::kb::{index, store};

use super::domain::{WorkshopConversation, WorkshopConversationPage, WorkshopMessage};
use super::infrastructure::{confirm, history, tools};

const HISTORY_PAGE_SIZE: usize = 20;

fn ensure_active_kb(active_root: &Path, expected_kb_path: &str) -> Result<(), AppError> {
  if active_root != Path::new(expected_kb_path) {
    return Err(AppError::code("err.workshop.kbSwitchedDuringSave"));
  }
  Ok(())
}

pub fn save_active_conversation(
  home: &Path,
  expected_kb_path: &str,
  id: Option<i64>,
  source_ids: Vec<String>,
  messages: Vec<WorkshopMessage>,
) -> Result<WorkshopConversation, AppError> {
  let root = crate::kb::active_kb_root(home)?;
  ensure_active_kb(&root, expected_kb_path)?;
  save_conversation(&root, id, source_ids, messages)
}

pub fn save_conversation(
  root: &Path,
  id: Option<i64>,
  source_ids: Vec<String>,
  messages: Vec<WorkshopMessage>,
) -> Result<WorkshopConversation, AppError> {
  let dir = history::open(root)?;
  let now = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
  history::save(&dir, id, &source_ids, &messages, &now)
}

pub fn get_conversation(root: &Path, id: i64) -> Result<WorkshopConversation, AppError> {
  history::get(&history::open(root)?, id)
}

pub fn list_conversations(root: &Path, offset: usize) -> Result<WorkshopConversationPage, AppError> {
  history::list(&history::open(root)?, offset, HISTORY_PAGE_SIZE)
}

/// 対話エージェント経路。素材は本文を注入せず id の目録だけ system に置き、AI が read_source で
/// 自分で読む。Rig で 1 会話分回し、進捗（思考・本文・ツール呼び出し/結果）を tx へ流して、
/// 最終的な助手の返信本文を返す。書き込みは write_entry ツール経由で「ユーザーが保存を頼んだとき」
/// だけ起きる＝確定の主導権はユーザー。素材は全て外部絶対パスで、KB へは複製しない。
#[allow(clippy::too_many_arguments)]
pub async fn chat(
  provider: Provider,
  base_url: String,
  brave_api_key: String,
  model: String,
  think: bool,
  root: PathBuf,
  sources: Vec<String>,
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: UnboundedSender<StreamProgress>,
  pending: confirm::PendingConfirms,
) -> Result<String, AppError> {
  let system = agent_system_with(&sources);
  // 破壊的ツール用の確認ゲート。進捗 tx へ確認要求を流し、workshop_confirm の回填を待つ。
  let gate = Arc::new(confirm::ConfirmGate { pending, tx: tx.clone(), cancel: cancel.clone() });
  let toolset = tools::build_toolset(&root, &sources, brave_api_key, gate);
  // base_url は設定の生値（空欄可）。空欄→provider 既定への解決は agent::run が担う。
  crate::agent::run(provider, &base_url, &model, think, &system, toolset, messages, cancel, &tx).await
}

/// 承認された内容を `entries/` に確定し、インデックスを更新する。
/// write_entry ツール（infra）経由で呼ばれる（書き込みの実体）。複数素材でも同じ経路を通る。
/// source_refs は実際に読んだ素材の引用文字列（外部絶対パス / URL）。
/// KB へは複製せず文字列だけ残す。
pub fn confirm(
  root: &Path,
  conn: &Connection,
  title: &str,
  cat: &str,
  body: &str,
  source_refs: &[String],
) -> Result<String, AppError> {
  let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
  let entry = Entry {
    meta: EntryMeta {
      kind: "Entry".into(),
      title: title.to_string(),
      description: String::new(),
      cat: cat.to_string(),
      tags: vec![],
      sources: source_refs.to_vec(),
      created: today.clone(),
      updated: today,
    },
    body: body.to_string(),
  };
  let rel = store::write_entry(root, &entry)?;
  index::upsert_entry(conn, &rel, &entry)?;
  Ok(rel)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::workshop::domain::{WorkshopMessage, WorkshopMessageRole};

  #[test]
  fn conversation_use_cases_round_trip_in_active_root() {
    let tmp = tempfile::tempdir().unwrap();
    let messages = vec![
      WorkshopMessage {
        role: WorkshopMessageRole::User,
        text: "整理这份资料".into(),
        thinking: None,
        tools: None,
      },
      WorkshopMessage {
        role: WorkshopMessageRole::Ai,
        text: "完成".into(),
        thinking: None,
        tools: None,
      },
    ];

    let saved = save_conversation(tmp.path(), None, vec![], messages).unwrap();
    assert_eq!(
      get_conversation(tmp.path(), saved.id).unwrap().title,
      "整理这份资料"
    );
    assert_eq!(
      list_conversations(tmp.path(), 0).unwrap().items[0].id,
      saved.id
    );
  }

  #[test]
  fn saving_rejects_a_conversation_from_a_different_active_kb() {
    let tmp = tempfile::tempdir().unwrap();
    let first_path = tmp.path().join("first");
    let second_path = tmp.path().join("second");

    let error = ensure_active_kb(&second_path, first_path.to_str().unwrap()).unwrap_err();

    assert_eq!(error.code, "err.workshop.kbSwitchedDuringSave");
  }

  #[test]
  fn confirm_records_source_refs_as_entry_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = crate::kb::index::open_index(root).unwrap();

    let rel = confirm(root, &conn, "緑茶", "tea", "本文", &["/abs/report.pdf".into()]).unwrap();

    let saved = std::fs::read_to_string(root.join(&rel)).unwrap();
    let entry = crate::kb::entry::parse_entry(&saved).unwrap();
    assert_eq!(entry.meta.sources, vec!["/abs/report.pdf".to_string()]);
  }

  #[test]
  fn confirm_writes_one_entry_and_indexes_links() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();

    // 複数素材を 1 条目に合成する。引用は外部パスの文字列として残す。
    let refs = vec!["/abs/a.pdf".to_string(), "/abs/b.docx".to_string()];
    let rel = confirm(root, &conn, "緑茶", "tea", "湯温は [[煎茶]] で70度", &refs).unwrap();
    assert!(root.join(&rel).is_file());
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
    let saved = std::fs::read_to_string(root.join(&rel)).unwrap();
    let entry = crate::kb::entry::parse_entry(&saved).unwrap();
    assert_eq!(entry.meta.sources, refs);
  }
}
