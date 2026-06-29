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

/// 対話エージェント経路。素材は本文を注入せず id の目録だけ system に置き、AI が read_source で
/// 自分で読む。Rig で 1 会話分回し、進捗（思考・本文・ツール呼び出し/結果）を tx へ流して、
/// 最終的な助手の返信本文を返す。書き込みは write_entry ツール経由で「ユーザーが保存を頼んだとき」
/// だけ起きる＝確定の主導権はユーザー。素材は全て外部絶対パスで、KB へは複製しない。
#[allow(clippy::too_many_arguments)]
pub async fn chat(
  model: String,
  think: bool,
  root: PathBuf,
  sources: Vec<String>,
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: UnboundedSender<StreamProgress>,
) -> Result<String, AiError> {
  let system = agent_system_with(&sources);
  rig_agent::run(&model, think, &system, &root, &sources, messages, cancel, &tx).await
}

/// 承認された内容を `entries/` に確定し、インデックスを更新する。
/// write_entry ツール（infra）経由で呼ばれる（書き込みの実体）。複数素材でも同じ経路を通る。
/// source_refs は添付素材の引用文字列（外部絶対パス）。KB へは複製せず文字列だけ残す。
pub fn confirm(
  root: &Path,
  conn: &Connection,
  title: &str,
  cat: &str,
  body: &str,
  source_refs: &[String],
) -> Result<String, String> {
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
