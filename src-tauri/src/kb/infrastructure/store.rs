use std::fs;
use std::path::Path;

use crate::error::AppError;
#[cfg(test)]
use crate::kb::domain::entry::parse_entry;
use crate::kb::domain::entry::{serialize_entry, Entry};

/// タイトルから安全なファイル名(slug)を作る。日本語等はそのまま、パス区切り等のみ除去。
fn slug(title: &str) -> String {
  let cleaned: String = title
    .chars()
    .map(|c| if "/\\:*?\"<>|".contains(c) { '-' } else { c })
    .collect();
  let cleaned = cleaned.trim().replace(' ', "-");
  if cleaned.is_empty() {
    "untitled".to_string()
  } else {
    cleaned
  }
}

/// 条目を `entries/<slug>.md` に書き出し、相対パスを返す。重複時は連番を付ける。
pub fn write_entry(root: &Path, entry: &Entry) -> Result<String, AppError> {
  let dir = root.join("entries");
  fs::create_dir_all(&dir).map_err(AppError::generic)?;
  let base = slug(&entry.meta.title);
  let mut rel = format!("entries/{base}.md");
  let mut n = 2;
  while root.join(&rel).exists() {
    rel = format!("entries/{base}-{n}.md");
    n += 1;
  }
  fs::write(root.join(&rel), serialize_entry(entry)?).map_err(AppError::generic)?;
  Ok(rel)
}

/// 既存条目を相対パスから読む。
#[cfg(test)]
pub fn read_entry(root: &Path, rel_path: &str) -> Result<Entry, AppError> {
  let text = fs::read_to_string(root.join(rel_path)).map_err(AppError::generic)?;
  parse_entry(&text)
}

/// 既存条目を上書き保存する（相対パス指定）。
#[cfg(test)]
pub fn save_entry(root: &Path, rel_path: &str, entry: &Entry) -> Result<(), AppError> {
  fs::write(root.join(rel_path), serialize_entry(entry)?).map_err(AppError::generic)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::domain::entry::EntryMeta;
  use crate::kb::index;

  fn meta(title: &str) -> EntryMeta {
    EntryMeta {
      kind: "Entry".into(),
      title: title.into(),
      description: String::new(),
      cat: "x".into(),
      tags: vec![],
      sources: vec![],
      created: "2026-06-14".into(),
      updated: "2026-06-14".into(),
    }
  }

  #[test]
  fn write_entry_creates_file_and_rebuild_indexes_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let entry = Entry { meta: meta("緑茶"), body: "[[煎茶]] が大事".into() };
    let rel = write_entry(root, &entry).unwrap();
    assert!(root.join(&rel).is_file());

    let conn = index::open_index(root).unwrap();
    index::rebuild(&conn, root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
  }

  #[test]
  fn rebuild_scans_entries_from_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("entries")).unwrap();
    fs::write(
      root.join("entries/a.md"),
      "---\ntype: Entry\ntitle: A\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n[[B]]\n",
    )
    .unwrap();

    let conn = index::open_index(root).unwrap();
    index::rebuild(&conn, root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "B").unwrap().len(), 1);
  }

  #[test]
  fn read_entry_round_trips_written_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let entry = Entry { meta: meta("緑茶"), body: "本文".into() };
    let rel = write_entry(root, &entry).unwrap();
    let read = read_entry(root, &rel).unwrap();
    assert_eq!(read.meta.title, "緑茶");
  }

  #[test]
  fn save_entry_overwrites_existing_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let original = Entry { meta: meta("緑茶"), body: "本文".into() };
    let rel = write_entry(root, &original).unwrap();
    let updated = Entry { meta: meta("緑茶"), body: "更新後".into() };

    save_entry(root, &rel, &updated).unwrap();

    assert_eq!(read_entry(root, &rel).unwrap().body, "更新後");
  }
}
