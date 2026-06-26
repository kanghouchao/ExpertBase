//! workshop アプリケーション層。対話エージェントの編成と条目確定のユースケース。
//! kb（検索・索引・FS）と Rig エージェント（infra）を編成する。ループ/ツール実体は Rig が持つ。

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::mpsc::UnboundedSender;

use crate::ai::agent::agent_system_with;
use crate::ai::{AiError, ChatTurn, StreamProgress};
use crate::kb::entry::{Entry, EntryMeta};
use crate::kb::{index, store};

use super::infrastructure::rig_agent;

/// 対話エージェント経路。素材を system に pin し、Rig で 1 会話分回す。進捗（思考・本文・
/// ツール呼び出し/結果）は tx へ流し、最終的な助手の返信本文を返す。書き込みは write_entry
/// ツール経由で「ユーザーが保存を頼んだとき」だけ起きる＝確定の主導権はユーザー。
#[allow(clippy::too_many_arguments)]
pub async fn chat(
  model: String,
  think: bool,
  root: PathBuf,
  source_text: String,
  inbox_rels: Vec<String>,
  messages: Vec<ChatTurn>,
  with_tools: bool,
  cancel: Arc<AtomicBool>,
  tx: UnboundedSender<StreamProgress>,
) -> Result<String, AiError> {
  let system = agent_system_with(&source_text);
  rig_agent::run(&model, think, &system, &root, &inbox_rels, with_tools, messages, cancel, &tx).await
}

/// 承認された内容を `entries/` に確定し、インデックス更新 + source の受信箱を全て processed にする。
/// write_entry ツール（infra）経由で呼ばれる（書き込みの実体）。複数素材でも同じ経路を通る。
pub fn confirm(
  root: &Path,
  conn: &Connection,
  title: &str,
  cat: &str,
  body: &str,
  inbox_rels: &[String],
) -> Result<String, String> {
  let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
  let entry = Entry {
    meta: EntryMeta {
      kind: "Entry".into(),
      title: title.to_string(),
      description: String::new(),
      cat: cat.to_string(),
      tags: vec![],
      sources: inbox_rels.to_vec(),
      created: today.clone(),
      updated: today,
    },
    body: body.to_string(),
  };
  let rel = store::write_entry(root, &entry)?;
  index::upsert_entry(conn, &rel, &entry)?;
  for inbox_rel in inbox_rels {
    index::set_inbox_status(conn, inbox_rel, "processed")?;
  }
  Ok(rel)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn confirm_records_inbox_rels_as_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = crate::kb::index::open_index(root).unwrap();
    std::fs::create_dir_all(root.join("inbox")).unwrap();

    let rel = confirm(root, &conn, "緑茶", "tea", "本文", &["inbox/a.md".into()]).unwrap();

    let saved = std::fs::read_to_string(root.join(&rel)).unwrap();
    let entry = crate::kb::entry::parse_entry(&saved).unwrap();
    assert_eq!(entry.meta.sources, vec!["inbox/a.md".to_string()]);
  }

  #[test]
  fn confirm_writes_one_entry_and_marks_all_source_inboxes_processed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    index::upsert_inbox(&conn, "inbox/m.md", "text", "paste", "pending", "2026-06-14T00:00:00Z")
      .unwrap();
    index::upsert_inbox(&conn, "inbox/n.md", "text", "paste", "pending", "2026-06-14T00:00:00Z")
      .unwrap();

    // 複数素材を 1 条目に合成する。確定後、source の inbox は全て processed になる。
    let rels = vec!["inbox/m.md".to_string(), "inbox/n.md".to_string()];
    let rel = confirm(root, &conn, "緑茶", "tea", "湯温は [[煎茶]] で70度", &rels).unwrap();
    assert!(root.join(&rel).is_file());
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
    let inbox = index::list_inbox(&conn).unwrap();
    assert!(inbox.iter().all(|m| m.status == "processed"));
  }
}
