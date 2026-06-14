use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[cfg(test)]
use super::entry::parse_entry;
use super::entry::{serialize_entry, split_frontmatter, Entry};

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
pub fn write_entry(root: &Path, entry: &Entry) -> Result<String, String> {
  let dir = root.join("entries");
  fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let base = slug(&entry.meta.title);
  let mut rel = format!("entries/{base}.md");
  let mut n = 2;
  while root.join(&rel).exists() {
    rel = format!("entries/{base}-{n}.md");
    n += 1;
  }
  fs::write(root.join(&rel), serialize_entry(entry)?).map_err(|e| e.to_string())?;
  Ok(rel)
}

/// 既存条目を相対パスから読む。
#[cfg(test)]
pub fn read_entry(root: &Path, rel_path: &str) -> Result<Entry, String> {
  let text = fs::read_to_string(root.join(rel_path)).map_err(|e| e.to_string())?;
  parse_entry(&text)
}

/// 既存条目を上書き保存する（相対パス指定）。
#[cfg(test)]
pub fn save_entry(root: &Path, rel_path: &str, entry: &Entry) -> Result<(), String> {
  fs::write(root.join(rel_path), serialize_entry(entry)?).map_err(|e| e.to_string())
}

fn default_status() -> String {
  "pending".to_string()
}

/// 受信箱素材（inbox/*.md）の frontmatter。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MaterialMeta {
  /// text/web/pdf/doc/audio/video/image
  #[serde(rename = "type")]
  pub kind: String,
  #[serde(default)]
  pub source: String,
  /// pending/processed
  #[serde(default = "default_status")]
  pub status: String,
  /// 添付（attachments/ への相対パス）。任意。
  #[serde(default)]
  pub attachment: String,
  #[serde(default)]
  pub captured_at: String,
}

/// 受信箱素材 = frontmatter + 本文（抽出テキストまたはユーザー説明文、任意）。
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
  pub meta: MaterialMeta,
  pub body: String,
}

/// 受信箱素材を直列化する（条目と同じフェンス規約）。
pub fn serialize_material(material: &Material) -> Result<String, String> {
  let yaml = serde_yaml::to_string(&material.meta).map_err(|e| e.to_string())?;
  Ok(format!("---\n{yaml}---\n\n{}", material.body))
}

/// 受信箱素材を解析する（条目と同じフェンス規約）。
pub fn parse_material(raw: &str) -> Result<Material, String> {
  let (yaml, body) = split_frontmatter(raw)?;
  let meta: MaterialMeta = serde_yaml::from_str(&yaml).map_err(|e| e.to_string())?;
  Ok(Material { meta, body })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::entry::EntryMeta;
  use crate::kb::index;

  fn meta(title: &str) -> EntryMeta {
    EntryMeta {
      kind: "Entry".into(),
      title: title.into(),
      description: String::new(),
      cat: "x".into(),
      tags: vec![],
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
  fn rebuild_scans_entries_and_inbox_from_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("entries")).unwrap();
    fs::create_dir_all(root.join("inbox")).unwrap();
    fs::write(
      root.join("entries/a.md"),
      "---\ntype: Entry\ntitle: A\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n[[B]]\n",
    )
    .unwrap();
    fs::write(
      root.join("inbox/m.md"),
      "---\ntype: web\nsource: https://x\nstatus: pending\ncaptured_at: 2026-06-14T00:00:00Z\n---\n\ntext\n",
    )
    .unwrap();

    let conn = index::open_index(root).unwrap();
    index::rebuild(&conn, root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::list_inbox(&conn).unwrap().len(), 1);
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

  #[test]
  fn material_round_trips() {
    let raw = "---\ntype: web\nsource: https://x\nstatus: pending\ncaptured_at: 2026-06-14T00:00:00Z\n---\n\n本文テキスト\n";
    let m = parse_material(raw).unwrap();
    assert_eq!(m.meta.kind, "web");
    assert_eq!(m.meta.status, "pending");
    let again = parse_material(&serialize_material(&m).unwrap()).unwrap();
    assert_eq!(again, m);
  }
}
