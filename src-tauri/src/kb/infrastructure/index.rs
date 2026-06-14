use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::kb::domain::entry::{extract_links, word_count, Entry};

/// インデックスの軽量な条目参照（一覧・バックリンク・孤立で共用）。
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryRef {
  pub path: String,
  pub title: String,
  pub cat: String,
}

/// 検索結果 1 件（スニペット付き）。
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
  pub path: String,
  pub title: String,
  pub excerpt: String,
}

/// ダッシュボード用の統計。
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
  pub entries: usize,
  pub links: usize,
  pub orphans: usize,
}

/// グラフ描画用のノード + 解決済みエッジ。
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphData {
  pub nodes: Vec<EntryRef>,
  /// (src_path, dst_path)。既存条目へ解決できたリンクのみ。
  pub edges: Vec<(String, String)>,
}

/// 受信箱素材の状態（一覧・ワークショップで使用）。
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
  pub path: String,
  #[serde(rename = "type")]
  pub kind: String,
  pub source: String,
  pub status: String,
  pub captured_at: String,
}

/// スキーマを作成する（存在すれば何もしない）。
/// 全文検索は trigram トークナイザを使う。日本語/中国語を含む部分一致が効くが、
/// クエリは 3 文字以上必要（trigram の仕様）。
pub fn ensure_schema(conn: &Connection) -> Result<(), String> {
  conn
    .execute_batch(
      "CREATE TABLE IF NOT EXISTS entries(
         path TEXT PRIMARY KEY,
         type TEXT NOT NULL,
         title TEXT NOT NULL,
         description TEXT NOT NULL DEFAULT '',
         cat TEXT NOT NULL DEFAULT '',
         tags TEXT NOT NULL DEFAULT '',
         updated TEXT NOT NULL DEFAULT '',
         words INTEGER NOT NULL DEFAULT 0
       );
       CREATE TABLE IF NOT EXISTS links(
         src_path TEXT NOT NULL,
         dst_title TEXT NOT NULL
       );
       CREATE INDEX IF NOT EXISTS idx_links_dst ON links(dst_title);
       CREATE INDEX IF NOT EXISTS idx_links_src ON links(src_path);
       CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_title ON entries(title);
       CREATE TABLE IF NOT EXISTS inbox(
         path TEXT PRIMARY KEY,
         type TEXT NOT NULL,
         source TEXT NOT NULL DEFAULT '',
         status TEXT NOT NULL DEFAULT 'pending',
         captured_at TEXT NOT NULL DEFAULT ''
       );
       CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts
         USING fts5(title, body, path UNINDEXED, tokenize='trigram');",
    )
    .map_err(|e| e.to_string())
}

/// `.expertbase/index.sqlite` を開き、スキーマを保証する。
pub fn open_index(root: &Path) -> Result<Connection, String> {
  let dir = root.join(".expertbase");
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let conn = Connection::open(dir.join("index.sqlite")).map_err(|e| e.to_string())?;
  ensure_schema(&conn)?;
  Ok(conn)
}

/// ディスクの entries/ と inbox/ を走査してインデックスを再構築する。
/// 真実のソースは常に Markdown。壊れたインデックスはこれで作り直せる。
pub fn rebuild(conn: &Connection, root: &Path) -> Result<(), String> {
  conn
    .execute_batch(
      "DELETE FROM entries; DELETE FROM links; DELETE FROM entries_fts; DELETE FROM inbox;",
    )
    .map_err(|e| e.to_string())?;

  let entries_dir = root.join("entries");
  if entries_dir.is_dir() {
    for ent in std::fs::read_dir(&entries_dir).map_err(|e| e.to_string())? {
      let path = ent.map_err(|e| e.to_string())?.path();
      if path.extension().and_then(|s| s.to_str()) != Some("md") {
        continue;
      }
      let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
      let entry = crate::kb::domain::entry::parse_entry(&text)?;
      let rel = format!("entries/{}", path.file_name().unwrap().to_string_lossy());
      upsert_entry(conn, &rel, &entry)?;
    }
  }

  let inbox_dir = root.join("inbox");
  if inbox_dir.is_dir() {
    for ent in std::fs::read_dir(&inbox_dir).map_err(|e| e.to_string())? {
      let path = ent.map_err(|e| e.to_string())?.path();
      if path.extension().and_then(|s| s.to_str()) != Some("md") {
        continue;
      }
      let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
      let m = crate::kb::domain::material::parse_material(&text)?;
      let rel = format!("inbox/{}", path.file_name().unwrap().to_string_lossy());
      upsert_inbox(conn, &rel, &m.meta.kind, &m.meta.source, &m.meta.status, &m.meta.captured_at)?;
    }
  }
  Ok(())
}

/// 条目をインデックスへ upsert する（メタ + リンク + FTS）。
pub fn upsert_entry(conn: &Connection, rel_path: &str, entry: &Entry) -> Result<(), String> {
  let duplicate_path = conn
    .query_row(
      "SELECT path FROM entries WHERE title=?1 AND path<>?2 LIMIT 1",
      rusqlite::params![entry.meta.title, rel_path],
      |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| e.to_string())?;
  if let Some(path) = duplicate_path {
    return Err(format!("同名の条目が既に存在します: {}", path));
  }

  delete_entry(conn, rel_path)?;
  let tags = entry.meta.tags.join(",");
  conn
    .execute(
      "INSERT INTO entries(path,type,title,description,cat,tags,updated,words)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
      rusqlite::params![
        rel_path,
        entry.meta.kind,
        entry.meta.title,
        entry.meta.description,
        entry.meta.cat,
        tags,
        entry.meta.updated,
        word_count(&entry.body) as i64,
      ],
    )
    .map_err(|e| e.to_string())?;
  for dst in extract_links(&entry.body) {
    conn
      .execute(
        "INSERT INTO links(src_path,dst_title) VALUES(?1,?2)",
        rusqlite::params![rel_path, dst],
      )
      .map_err(|e| e.to_string())?;
  }
  conn
    .execute(
      "INSERT INTO entries_fts(title,body,path) VALUES(?1,?2,?3)",
      rusqlite::params![entry.meta.title, entry.body, rel_path],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// 条目をインデックスの全テーブルから削除する。
pub fn delete_entry(conn: &Connection, rel_path: &str) -> Result<(), String> {
  conn
    .execute("DELETE FROM entries WHERE path=?1", [rel_path])
    .map_err(|e| e.to_string())?;
  conn
    .execute("DELETE FROM links WHERE src_path=?1", [rel_path])
    .map_err(|e| e.to_string())?;
  conn
    .execute("DELETE FROM entries_fts WHERE path=?1", [rel_path])
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// 指定タイトルを参照している条目（バックリンク）。
pub fn backlinks(conn: &Connection, title: &str) -> Result<Vec<EntryRef>, String> {
  let mut stmt = conn
    .prepare(
      "SELECT e.path,e.title,e.cat FROM links l
         JOIN entries e ON e.path=l.src_path
         WHERE l.dst_title=?1 ORDER BY e.path",
    )
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([title], |r| {
      Ok(EntryRef { path: r.get(0)?, title: r.get(1)?, cat: r.get(2)? })
    })
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// 孤立条目: 他から参照されず(inbound 無し)かつ自身もリンクを持たない(outbound 無し)。
pub fn orphans(conn: &Connection) -> Result<Vec<EntryRef>, String> {
  let mut stmt = conn
    .prepare(
      "SELECT e.path,e.title,e.cat FROM entries e
         WHERE e.path NOT IN (SELECT src_path FROM links)
           AND e.title NOT IN (SELECT dst_title FROM links)
         ORDER BY e.path",
    )
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| {
      Ok(EntryRef { path: r.get(0)?, title: r.get(1)?, cat: r.get(2)? })
    })
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// FTS5 全文検索。マッチ箇所のスニペットを返す。クエリは 3 文字以上を想定。
pub fn search(conn: &Connection, query: &str) -> Result<Vec<SearchHit>, String> {
  if query.trim().is_empty() {
    return Ok(vec![]);
  }
  let mut stmt = conn
    .prepare(
      "SELECT path,title,snippet(entries_fts,1,'[',']','…',12) FROM entries_fts
         WHERE entries_fts MATCH ?1 ORDER BY rank LIMIT 50",
    )
    .map_err(|e| e.to_string())?;
  // クエリ全体をフレーズとして引用し、FTS5 構文記号を無効化する。
  let phrase = format!("\"{}\"", query.trim().replace('"', "\"\""));
  let rows = stmt
    .query_map([phrase], |r| {
      Ok(SearchHit { path: r.get(0)?, title: r.get(1)?, excerpt: r.get(2)? })
    })
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// インデックス全体から条目一覧を返す（更新が新しい順）。
pub fn list_entries(conn: &Connection) -> Result<Vec<EntryRef>, String> {
  let mut stmt = conn
    .prepare("SELECT path,title,cat FROM entries ORDER BY updated DESC, path")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| {
      Ok(EntryRef { path: r.get(0)?, title: r.get(1)?, cat: r.get(2)? })
    })
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// ダッシュボード用の統計。
pub fn stats(conn: &Connection) -> Result<Stats, String> {
  let entries = conn
    .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0))
    .map_err(|e| e.to_string())? as usize;
  let links = conn
    .query_row("SELECT COUNT(*) FROM links", [], |r| r.get::<_, i64>(0))
    .map_err(|e| e.to_string())? as usize;
  let orphans = orphans(conn)?.len();
  Ok(Stats { entries, links, orphans })
}

/// グラフ描画用のノード + 解決済みエッジ（dst_title を既存条目 path へ解決）。
pub fn graph(conn: &Connection) -> Result<GraphData, String> {
  let nodes = list_entries(conn)?;
  let mut stmt = conn
    .prepare("SELECT l.src_path,e.path FROM links l JOIN entries e ON e.title=l.dst_title")
    .map_err(|e| e.to_string())?;
  let edges = stmt
    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
  Ok(GraphData { nodes, edges })
}

/// 受信箱素材をインデックスへ upsert する。
pub fn upsert_inbox(
  conn: &Connection,
  rel_path: &str,
  kind: &str,
  source: &str,
  status: &str,
  captured_at: &str,
) -> Result<(), String> {
  conn
    .execute(
      "INSERT INTO inbox(path,type,source,status,captured_at) VALUES(?1,?2,?3,?4,?5)
         ON CONFLICT(path) DO UPDATE SET type=?2,source=?3,status=?4,captured_at=?5",
      rusqlite::params![rel_path, kind, source, status, captured_at],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// 受信箱の状態を更新する（pending → processed など）。
pub fn set_inbox_status(conn: &Connection, rel_path: &str, status: &str) -> Result<(), String> {
  conn
    .execute(
      "UPDATE inbox SET status=?2 WHERE path=?1",
      rusqlite::params![rel_path, status],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// 受信箱の一覧（取り込み新しい順）。
pub fn list_inbox(conn: &Connection) -> Result<Vec<InboxItem>, String> {
  let mut stmt = conn
    .prepare("SELECT path,type,source,status,captured_at FROM inbox ORDER BY captured_at DESC, path")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| {
      Ok(InboxItem {
        path: r.get(0)?,
        kind: r.get(1)?,
        source: r.get(2)?,
        status: r.get(3)?,
        captured_at: r.get(4)?,
      })
    })
    .map_err(|e| e.to_string())?;
  rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::domain::entry::EntryMeta;

  fn meta(title: &str, cat: &str) -> EntryMeta {
    EntryMeta {
      kind: "Entry".into(),
      title: title.into(),
      description: String::new(),
      cat: cat.into(),
      tags: vec![],
      created: "2026-06-14".into(),
      updated: "2026-06-14".into(),
    }
  }

  fn mem() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    ensure_schema(&conn).unwrap();
    conn
  }

  #[test]
  fn upsert_and_search_round_trip() {
    let conn = mem();
    let entry = Entry { meta: meta("緑茶", "tea"), body: "湯温は70度".into() };
    upsert_entry(&conn, "entries/green.md", &entry).unwrap();
    let hits = search(&conn, "湯温は").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "entries/green.md");
  }

  #[test]
  fn search_supports_cjk_trigram_and_english_terms() {
    let conn = mem();
    upsert_entry(
      &conn,
      "entries/green.md",
      &Entry { meta: meta("緑茶", "tea"), body: "湯温は70度。oxidation is low.".into() },
    )
    .unwrap();

    assert_eq!(search(&conn, "湯温は").unwrap()[0].path, "entries/green.md");
    assert_eq!(search(&conn, "oxidation").unwrap()[0].path, "entries/green.md");
    assert!(search(&conn, "  ").unwrap().is_empty());
  }

  #[test]
  fn upsert_rejects_duplicate_titles_on_different_paths() {
    let conn = mem();
    let first = Entry { meta: meta("緑茶", "tea"), body: "a".into() };
    let second = Entry { meta: meta("緑茶", "tea"), body: "b".into() };
    upsert_entry(&conn, "entries/a.md", &first).unwrap();

    let err = upsert_entry(&conn, "entries/b.md", &second).unwrap_err();

    assert!(err.contains("同名の条目"));
    assert_eq!(list_entries(&conn).unwrap().len(), 1);
  }

  #[test]
  fn backlinks_and_orphans() {
    let conn = mem();
    let a = Entry { meta: meta("A", "x"), body: "see [[B]]".into() };
    let b = Entry { meta: meta("B", "x"), body: "no links".into() };
    let c = Entry { meta: meta("C", "x"), body: "lonely".into() };
    upsert_entry(&conn, "entries/a.md", &a).unwrap();
    upsert_entry(&conn, "entries/b.md", &b).unwrap();
    upsert_entry(&conn, "entries/c.md", &c).unwrap();

    let back = backlinks(&conn, "B").unwrap();
    assert_eq!(
      back.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
      vec!["entries/a.md"]
    );

    // A は B を指す(outbound)、B は A から指される(inbound)、C は孤立。
    let orphan_paths: Vec<_> = orphans(&conn).unwrap().into_iter().map(|r| r.path).collect();
    assert_eq!(orphan_paths, vec!["entries/c.md".to_string()]);
  }

  #[test]
  fn delete_removes_from_all_tables() {
    let conn = mem();
    let a = Entry { meta: meta("A", "x"), body: "reference [[B]]".into() };
    upsert_entry(&conn, "entries/a.md", &a).unwrap();
    assert_eq!(search(&conn, "reference").unwrap().len(), 1);
    delete_entry(&conn, "entries/a.md").unwrap();
    assert_eq!(stats(&conn).unwrap().entries, 0);
    assert_eq!(stats(&conn).unwrap().links, 0);
    assert!(search(&conn, "reference").unwrap().is_empty());
  }

  #[test]
  fn stats_counts_entries_links_orphans() {
    let conn = mem();
    upsert_entry(&conn, "entries/a.md", &Entry { meta: meta("A", "x"), body: "[[B]]".into() })
      .unwrap();
    upsert_entry(&conn, "entries/b.md", &Entry { meta: meta("B", "x"), body: "x".into() })
      .unwrap();
    let s = stats(&conn).unwrap();
    assert_eq!(s.entries, 2);
    assert_eq!(s.links, 1);
    assert_eq!(s.orphans, 0); // A->B なので両者ともリンクに関与
  }

  #[test]
  fn inbox_upsert_list_and_status() {
    let conn = mem();
    upsert_inbox(&conn, "inbox/a.md", "web", "https://x", "pending", "2026-06-14T00:00:00Z")
      .unwrap();
    let items = list_inbox(&conn).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, "pending");
    set_inbox_status(&conn, "inbox/a.md", "processed").unwrap();
    assert_eq!(list_inbox(&conn).unwrap()[0].status, "processed");
  }

  #[test]
  fn graph_resolves_edges_to_existing_paths() {
    let conn = mem();
    upsert_entry(&conn, "entries/a.md", &Entry { meta: meta("A", "x"), body: "[[B]]".into() })
      .unwrap();
    upsert_entry(&conn, "entries/b.md", &Entry { meta: meta("B", "x"), body: "x".into() })
      .unwrap();
    let g = graph(&conn).unwrap();
    assert_eq!(g.nodes.len(), 2);
    assert_eq!(g.edges, vec![("entries/a.md".to_string(), "entries/b.md".to_string())]);
  }
}
