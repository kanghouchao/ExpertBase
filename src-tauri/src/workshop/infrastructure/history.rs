use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::workshop::domain::{
  conversation_title, WorkshopConversation, WorkshopConversationPage, WorkshopConversationSummary,
  WorkshopMessage,
};

pub fn open(root: &Path) -> Result<Connection, String> {
  let dir = root.join(".expertbase");
  std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
  let conn = Connection::open(dir.join("workshop.sqlite")).map_err(|error| error.to_string())?;
  conn
    .execute_batch(
      "CREATE TABLE IF NOT EXISTS conversations(
         id INTEGER PRIMARY KEY AUTOINCREMENT,
         title TEXT NOT NULL,
         source_ids TEXT NOT NULL,
         messages TEXT NOT NULL,
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
       );
       CREATE INDEX IF NOT EXISTS idx_conversations_updated
         ON conversations(updated_at DESC, id DESC);",
    )
    .map_err(|error| error.to_string())?;
  Ok(conn)
}

pub fn save(
  conn: &Connection,
  id: Option<i64>,
  source_ids: &[String],
  messages: &[WorkshopMessage],
  now: &str,
) -> Result<WorkshopConversation, String> {
  let title = conversation_title(messages);
  let source_ids_json = serde_json::to_string(source_ids).map_err(|error| error.to_string())?;
  let messages_json = serde_json::to_string(messages).map_err(|error| error.to_string())?;
  let id = match id {
    Some(id) => {
      let changed = conn
        .execute(
          "UPDATE conversations
             SET title=?1, source_ids=?2, messages=?3, updated_at=?4
             WHERE id=?5",
          rusqlite::params![title, source_ids_json, messages_json, now, id],
        )
        .map_err(|error| error.to_string())?;
      if changed == 0 {
        return Err(format!("conversation not found: {id}"));
      }
      id
    }
    None => {
      conn
        .execute(
          "INSERT INTO conversations(title,source_ids,messages,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?4)",
          rusqlite::params![title, source_ids_json, messages_json, now],
        )
        .map_err(|error| error.to_string())?;
      conn.last_insert_rowid()
    }
  };
  get(conn, id)
}

pub fn get(conn: &Connection, id: i64) -> Result<WorkshopConversation, String> {
  conn
    .query_row(
      "SELECT id,title,source_ids,messages,created_at,updated_at
         FROM conversations WHERE id=?1",
      [id],
      |row| {
        let source_ids: String = row.get(2)?;
        let messages: String = row.get(3)?;
        Ok((
          row.get(0)?,
          row.get(1)?,
          source_ids,
          messages,
          row.get(4)?,
          row.get(5)?,
        ))
      },
    )
    .optional()
    .map_err(|error| error.to_string())?
    .ok_or_else(|| format!("conversation not found: {id}"))
    .and_then(|(id, title, source_ids, messages, created_at, updated_at)| {
      Ok(WorkshopConversation {
        id,
        title,
        source_ids: serde_json::from_str(&source_ids).map_err(|error| error.to_string())?,
        messages: serde_json::from_str(&messages).map_err(|error| error.to_string())?,
        created_at,
        updated_at,
      })
    })
}

pub fn list(
  conn: &Connection,
  offset: usize,
  limit: usize,
) -> Result<WorkshopConversationPage, String> {
  let mut stmt = conn
    .prepare(
      "SELECT id,title,updated_at FROM conversations
         ORDER BY updated_at DESC, id DESC LIMIT ?1 OFFSET ?2",
    )
    .map_err(|error| error.to_string())?;
  let mut items = stmt
    .query_map(
      rusqlite::params![(limit + 1) as i64, offset as i64],
      |row| {
        Ok(WorkshopConversationSummary {
          id: row.get(0)?,
          title: row.get(1)?,
          updated_at: row.get(2)?,
        })
      },
    )
    .map_err(|error| error.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())?;
  let has_more = items.len() > limit;
  items.truncate(limit);
  Ok(WorkshopConversationPage { items, has_more })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::workshop::domain::WorkshopMessage;

  #[test]
  fn save_get_update_and_paginate_conversations() {
    let tmp = tempfile::tempdir().unwrap();
    let conn = open(tmp.path()).unwrap();
    let first = save(
      &conn,
      None,
      &["/tmp/a.pdf".into()],
      &[WorkshopMessage::user("第一条"), WorkshopMessage::assistant("回答")],
      "2026-06-30T01:00:00.000Z",
    )
    .unwrap();
    let second = save(
      &conn,
      None,
      &[],
      &[WorkshopMessage::user("第二条"), WorkshopMessage::assistant("回答")],
      "2026-06-30T02:00:00.000Z",
    )
    .unwrap();

    let loaded = get(&conn, first.id).unwrap();
    assert_eq!(loaded.source_ids, vec!["/tmp/a.pdf"]);
    assert_eq!(loaded.messages[1].text, "回答");

    let updated = save(
      &conn,
      Some(first.id),
      &["/tmp/a.pdf".into()],
      &[
        WorkshopMessage::user("第一条"),
        WorkshopMessage::assistant("回答"),
        WorkshopMessage::user("继续"),
        WorkshopMessage::assistant("新回答"),
      ],
      "2026-06-30T03:00:00.000Z",
    )
    .unwrap();
    assert_eq!(updated.created_at, "2026-06-30T01:00:00.000Z");

    let page = list(&conn, 0, 1).unwrap();
    assert_eq!(page.items[0].id, first.id);
    assert!(page.has_more);
    let page = list(&conn, 1, 1).unwrap();
    assert_eq!(page.items[0].id, second.id);
    assert!(!page.has_more);
  }

  #[test]
  fn updating_unknown_conversation_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let conn = open(tmp.path()).unwrap();
    let error = save(
      &conn,
      Some(999),
      &[],
      &[WorkshopMessage::user("不存在")],
      "2026-06-30T01:00:00.000Z",
    )
    .unwrap_err();
    assert!(error.contains("conversation not found"));
  }
}
