//! workshop インフラ: 対話履歴の永続化。会話 1 件 = JSONL ファイル 1 つ
//! （`<root>/.expertbase/conversations/<id>.jsonl`）。1 行 1 イベントの追記式
//! （Claude Code / Codex と同じ発想）＝毎ターン全文を書き直さず新しい行だけ足す。
//! 先頭行が meta（id・タイトル・作成時刻・素材 id）、以降が 1 メッセージ 1 行。
//! 素材が増えたら meta 行を追記し、読むときは最後の meta を採る（last-wins）。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::workshop::domain::{
  conversation_title, WorkshopConversation, WorkshopConversationPage, WorkshopConversationSummary,
  WorkshopMessage,
};

/// JSONL の 1 行。会話メタ（meta）か、1 メッセージ + 時刻（msg）。
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Line {
  Meta {
    id: i64,
    title: String,
    created_at: String,
    source_ids: Vec<String>,
  },
  Msg {
    at: String,
    message: WorkshopMessage,
  },
}

/// 会話 JSONL の置き場を用意して返す（無ければ作る）。
pub fn open(root: &Path) -> Result<PathBuf, String> {
  let dir = root.join(".expertbase").join("conversations");
  fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
  Ok(dir)
}

fn conv_path(dir: &Path, id: i64) -> PathBuf {
  dir.join(format!("{id}.jsonl"))
}

fn to_line(line: &Line) -> Result<String, String> {
  let mut text = serde_json::to_string(line).map_err(|error| error.to_string())?;
  text.push('\n');
  Ok(text)
}

fn read_lines(path: &Path) -> Result<Vec<Line>, String> {
  let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
  text
    .lines()
    .filter(|line| !line.trim().is_empty())
    .map(|line| serde_json::from_str::<Line>(line).map_err(|error| error.to_string()))
    .collect()
}

/// 次の会話 id ＝ 既存ファイル名（数値）の最大 +1。無ければ 1。
fn next_id(dir: &Path) -> Result<i64, String> {
  let mut max = 0i64;
  for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
    let entry = entry.map_err(|error| error.to_string())?;
    if let Some(id) = entry
      .path()
      .file_stem()
      .and_then(|stem| stem.to_str())
      .and_then(|stem| stem.parse::<i64>().ok())
    {
      max = max.max(id);
    }
  }
  Ok(max + 1)
}

fn first_meta(lines: &[Line]) -> Option<(String, String)> {
  lines.iter().find_map(|line| match line {
    Line::Meta { title, created_at, .. } => Some((title.clone(), created_at.clone())),
    _ => None,
  })
}

fn last_source_ids(lines: &[Line]) -> Option<Vec<String>> {
  lines.iter().rev().find_map(|line| match line {
    Line::Meta { source_ids, .. } => Some(source_ids.clone()),
    _ => None,
  })
}

fn write_full(
  path: &Path,
  id: i64,
  title: String,
  created_at: String,
  source_ids: &[String],
  messages: &[WorkshopMessage],
  now: &str,
) -> Result<(), String> {
  let mut out = to_line(&Line::Meta { id, title, created_at, source_ids: source_ids.to_vec() })?;
  for message in messages {
    out.push_str(&to_line(&Line::Msg { at: now.to_string(), message: message.clone() })?);
  }
  fs::write(path, out).map_err(|error| error.to_string())
}

pub fn save(
  dir: &Path,
  id: Option<i64>,
  source_ids: &[String],
  messages: &[WorkshopMessage],
  now: &str,
) -> Result<WorkshopConversation, String> {
  match id {
    None => {
      let id = next_id(dir)?;
      let title = conversation_title(messages);
      write_full(&conv_path(dir, id), id, title, now.to_string(), source_ids, messages, now)?;
      get(dir, id)
    }
    Some(id) => {
      let path = conv_path(dir, id);
      if !path.exists() {
        return Err(format!("conversation not found: {id}"));
      }
      let lines = read_lines(&path)?;
      let existing_count = lines.iter().filter(|line| matches!(line, Line::Msg { .. })).count();

      // app 層は只增（送信即存盤・失敗も消さない）なので messages は既存以上のはず。
      // ponytail: 縮んだら（想定外）全文書き直しで安全側に倒す。
      if messages.len() < existing_count {
        let (title, created_at) =
          first_meta(&lines).unwrap_or_else(|| (conversation_title(messages), now.to_string()));
        write_full(&path, id, title, created_at, source_ids, messages, now)?;
        return get(dir, id);
      }

      let mut appended = String::new();
      // 素材が変わったら meta 行を追記（読むときは最後の meta ＝ last-wins）。
      if last_source_ids(&lines).as_deref() != Some(source_ids) {
        let (title, created_at) =
          first_meta(&lines).unwrap_or_else(|| (conversation_title(messages), now.to_string()));
        appended.push_str(&to_line(&Line::Meta {
          id,
          title,
          created_at,
          source_ids: source_ids.to_vec(),
        })?);
      }
      // 既存より後ろの新しいメッセージ「だけ」を追記する＝毎ターン全文を書き直さない。
      for message in &messages[existing_count..] {
        appended.push_str(&to_line(&Line::Msg { at: now.to_string(), message: message.clone() })?);
      }
      let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
      file.write_all(appended.as_bytes()).map_err(|error| error.to_string())?;
      get(dir, id)
    }
  }
}

fn build_conversation(id: i64, lines: &[Line]) -> Result<WorkshopConversation, String> {
  // title/createdAt は不変、sourceIds は最新を採りたいので、いずれも最後の meta から取る。
  let (title, created_at, source_ids) = lines
    .iter()
    .rev()
    .find_map(|line| match line {
      Line::Meta { title, created_at, source_ids, .. } => {
        Some((title.clone(), created_at.clone(), source_ids.clone()))
      }
      _ => None,
    })
    .ok_or_else(|| format!("conversation has no meta: {id}"))?;
  let messages = lines
    .iter()
    .filter_map(|line| match line {
      Line::Msg { message, .. } => Some(message.clone()),
      _ => None,
    })
    .collect();
  // updatedAt ＝ 最後のメッセージ行の時刻（無ければ作成時刻）。
  let updated_at = lines
    .iter()
    .rev()
    .find_map(|line| match line {
      Line::Msg { at, .. } => Some(at.clone()),
      _ => None,
    })
    .unwrap_or_else(|| created_at.clone());
  Ok(WorkshopConversation { id, title, source_ids, messages, created_at, updated_at })
}

pub fn get(dir: &Path, id: i64) -> Result<WorkshopConversation, String> {
  let path = conv_path(dir, id);
  if !path.exists() {
    return Err(format!("conversation not found: {id}"));
  }
  build_conversation(id, &read_lines(&path)?)
}

pub fn list(dir: &Path, offset: usize, limit: usize) -> Result<WorkshopConversationPage, String> {
  let mut summaries: Vec<WorkshopConversationSummary> = Vec::new();
  for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
    let entry = entry.map_err(|error| error.to_string())?;
    let path = entry.path();
    if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
      continue;
    }
    let Some(id) = path
      .file_stem()
      .and_then(|stem| stem.to_str())
      .and_then(|stem| stem.parse::<i64>().ok())
    else {
      continue;
    };
    // ponytail: 一覧は会話ごとに全文を読む。件数が爆発したら title/updatedAt を別索引へ。
    let conversation = build_conversation(id, &read_lines(&path)?)?;
    summaries.push(WorkshopConversationSummary {
      id,
      title: conversation.title,
      updated_at: conversation.updated_at,
    });
  }
  // 更新が新しい順、同時刻は id 降順。
  summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(b.id.cmp(&a.id)));
  let has_more = summaries.len() > offset + limit;
  let items = summaries.into_iter().skip(offset).take(limit).collect();
  Ok(WorkshopConversationPage { items, has_more })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::workshop::domain::WorkshopMessage;

  fn line_count(path: &Path) -> usize {
    fs::read_to_string(path).unwrap().lines().filter(|line| !line.trim().is_empty()).count()
  }

  #[test]
  fn saving_an_existing_conversation_only_appends_new_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = open(tmp.path()).unwrap();
    let first = save(
      &dir,
      None,
      &["/a.pdf".into()],
      &[WorkshopMessage::user("問い")],
      "2026-06-30T01:00:00.000Z",
    )
    .unwrap();
    let path = conv_path(&dir, first.id);
    // meta + user = 2 行。
    assert_eq!(line_count(&path), 2);

    let updated = save(
      &dir,
      Some(first.id),
      &["/a.pdf".into()],
      &[WorkshopMessage::user("問い"), WorkshopMessage::assistant("答え")],
      "2026-06-30T02:00:00.000Z",
    )
    .unwrap();
    // ai 行だけ追記＝3 行（全文書き直しではない）。
    assert_eq!(line_count(&path), 3);
    assert_eq!(updated.messages.len(), 2);
    assert_eq!(updated.messages[1].text, "答え");
    assert_eq!(updated.created_at, "2026-06-30T01:00:00.000Z");
    assert_eq!(updated.updated_at, "2026-06-30T02:00:00.000Z");
  }

  #[test]
  fn get_round_trips_sources_and_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = open(tmp.path()).unwrap();
    let saved = save(
      &dir,
      None,
      &["/a.pdf".into()],
      &[WorkshopMessage::user("第一条"), WorkshopMessage::assistant("回答")],
      "2026-06-30T01:00:00.000Z",
    )
    .unwrap();
    let loaded = get(&dir, saved.id).unwrap();
    assert_eq!(loaded.source_ids, vec!["/a.pdf"]);
    assert_eq!(loaded.messages[1].text, "回答");
  }

  #[test]
  fn list_orders_by_last_update_and_paginates() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = open(tmp.path()).unwrap();
    let first = save(&dir, None, &[], &[WorkshopMessage::user("一")], "2026-06-30T01:00:00.000Z")
      .unwrap();
    let second = save(&dir, None, &[], &[WorkshopMessage::user("二")], "2026-06-30T02:00:00.000Z")
      .unwrap();
    // first を後から更新＝最新へ。
    save(
      &dir,
      Some(first.id),
      &[],
      &[WorkshopMessage::user("一"), WorkshopMessage::assistant("回")],
      "2026-06-30T03:00:00.000Z",
    )
    .unwrap();

    let page = list(&dir, 0, 1).unwrap();
    assert_eq!(page.items[0].id, first.id);
    assert!(page.has_more);
    let page = list(&dir, 1, 1).unwrap();
    assert_eq!(page.items[0].id, second.id);
    assert!(!page.has_more);
  }

  #[test]
  fn updating_unknown_conversation_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = open(tmp.path()).unwrap();
    let error = save(
      &dir,
      Some(999),
      &[],
      &[WorkshopMessage::user("不存在")],
      "2026-06-30T01:00:00.000Z",
    )
    .unwrap_err();
    assert!(error.contains("conversation not found"));
  }

  #[test]
  fn changed_sources_are_persisted_last_wins() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = open(tmp.path()).unwrap();
    let first =
      save(&dir, None, &["/a.pdf".into()], &[WorkshopMessage::user("問")], "2026-06-30T01:00:00.000Z")
        .unwrap();
    // mid-conversation で素材を足して送り直す。
    let updated = save(
      &dir,
      Some(first.id),
      &["/a.pdf".into(), "/b.docx".into()],
      &[WorkshopMessage::user("問"), WorkshopMessage::assistant("答")],
      "2026-06-30T02:00:00.000Z",
    )
    .unwrap();
    assert_eq!(updated.source_ids, vec!["/a.pdf", "/b.docx"]);
    assert_eq!(get(&dir, first.id).unwrap().source_ids, vec!["/a.pdf", "/b.docx"]);
  }
}
