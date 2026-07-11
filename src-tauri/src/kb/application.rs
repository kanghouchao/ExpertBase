//! kb アプリケーション層。ユースケースを表し、ドメインを編成して
//! インフラ抽象（config_store / index / store）に依存する。

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::AppError;
use crate::kb::domain::entry::{self, Entry, EntryMeta};
use crate::kb::domain::registry::{self, KbConfig, KbEntry};
use crate::kb::infrastructure::{config_store, index, store};

/// アクティブなナレッジベースのルートパスを返す。未選択ならエラー。
pub(crate) fn active_kb_root(home: &Path) -> Result<PathBuf, AppError> {
  let reg = config_store::load_registry(home)?;
  let active = reg.active.ok_or_else(|| AppError::code("err.kb.noActiveKb"))?;
  Ok(PathBuf::from(active))
}

/// アクティブ KB のルートとインデックス接続をまとめて開く。
pub(crate) fn open_active(home: &Path) -> Result<(PathBuf, Connection), AppError> {
  let root = active_kb_root(home)?;
  let conn = index::open_index(&root)?;
  Ok((root, conn))
}

/// ナレッジベースを新規作成して登録し、アクティブに切り替える。
pub fn create_kb(
  home: &Path,
  name: &str,
  description: &str,
  raw_path: &str,
) -> Result<KbEntry, AppError> {
  let name = name.trim();
  if name.is_empty() {
    return Err(AppError::code("err.kb.nameRequired"));
  }
  let raw_path = raw_path.trim();
  if raw_path.is_empty() {
    return Err(AppError::code("err.kb.pathRequired"));
  }
  let path = registry::expand_home(home, raw_path);
  let path_str = path.to_string_lossy().into_owned();

  let mut reg = config_store::load_registry(home)?;
  if reg.knowledge_bases.iter().any(|k| k.path == path_str) {
    return Err(AppError::code("err.kb.pathAlreadyRegistered"));
  }

  if config_store::kb_config_exists(&path) {
    return Err(AppError::code("err.kb.pathAlreadyHasKb"));
  }
  config_store::write_kb_config(
    &path,
    &KbConfig {
      name: name.into(),
      description: description.trim().into(),
    },
  )?;

  let entry = KbEntry {
    name: name.into(),
    path: path_str.clone(),
  };
  reg.knowledge_bases.push(entry.clone());
  reg.active = Some(path_str);
  config_store::save_registry(home, &reg)?;
  Ok(entry)
}

/// 登録済みナレッジベースをアクティブに切り替える。
pub fn set_active(home: &Path, path: &str) -> Result<(), AppError> {
  let mut reg = config_store::load_registry(home)?;
  if !reg.knowledge_bases.iter().any(|k| k.path == path) {
    return Err(AppError::code("err.kb.notFound"));
  }
  reg.active = Some(path.into());
  config_store::save_registry(home, &reg)
}

/// 登録済みナレッジベースを削除する。ExpertBase のメタデータだけを削除し、
/// 空になった場合に限ってルートも削除してからレジストリを更新する。
pub fn delete_kb(home: &Path, path: &str) -> Result<(), AppError> {
  let mut reg = config_store::load_registry(home)?;
  let idx = reg
    .knowledge_bases
    .iter()
    .position(|k| k.path == path)
    .ok_or_else(|| AppError::code("err.kb.notFound"))?;
  let root = Path::new(path);
  let metadata = root.join(".expertbase");
  if metadata.exists() {
    if !metadata.is_dir() {
      return Err(AppError::code("err.kb.metaNotDirectory"));
    }
    fs::remove_dir_all(&metadata).map_err(AppError::generic)?;
  }
  match fs::remove_dir(root) {
    Ok(()) => {}
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
    // ponytail: 空でないルートはユーザーデータを含む可能性があるため残す。
    Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
    Err(e) => return Err(AppError::generic(e)),
  }

  reg.knowledge_bases.remove(idx);
  if reg.active.as_deref() == Some(path) {
    reg.active = reg.knowledge_bases.first().map(|k| k.path.clone());
  }
  config_store::save_registry(home, &reg)
}

/// 条目を上書き保存する（frontmatter 検証付き）。保存前に検証し、不正なら書き込まない。
pub fn save_entry(home: &Path, rel_path: &str, content: &str) -> Result<(), AppError> {
  let (root, conn) = open_active(home)?;
  let rel = registry::checked_kb_markdown_path(rel_path, "entries")?;
  let parsed = entry::parse_entry(content)?;
  fs::write(root.join(&rel), content).map_err(AppError::generic)?;
  index::upsert_entry(&conn, &rel.to_string_lossy(), &parsed)
}

/// 新しい条目を確定する（条目持久化）。kind は Entry 固定、created / updated は当日。
/// source_refs は実際に参照した素材の引用文字列（外部絶対パス / URL）。KB へは複製しない。
/// ファイルが真源：索引更新に失敗しても書いた条目は残り、索引は rebuild で復元できる。
pub fn create_entry(
  root: &Path,
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
  let conn = index::open_index(root)?;
  let rel = store::write_entry(root, &entry)?;
  index::upsert_entry(&conn, &rel, &entry)?;
  Ok(rel)
}

/// 既存条目の本文だけを差し替える（条目持久化）。メタ（title / cat / sources / created 等）は
/// 維持し、updated だけ当日へ進める。ファイルが真源：索引更新に失敗しても差し替えは残る。
pub fn update_entry_body(root: &Path, rel_path: &str, body: &str) -> Result<(), AppError> {
  let rel = registry::checked_kb_markdown_path(rel_path, "entries")?;
  let conn = index::open_index(root)?;
  let text = fs::read_to_string(root.join(&rel)).map_err(AppError::generic)?;
  let mut entry = entry::parse_entry(&text)?;
  entry.body = body.to_string();
  entry.meta.updated = chrono::Utc::now().format("%Y-%m-%d").to_string();
  store::save_entry(root, &rel.to_string_lossy(), &entry)?;
  index::upsert_entry(&conn, &rel.to_string_lossy(), &entry)
}

/// 条目を削除する（ファイル + 索引）。アクティブ KB を解決して delete_entry_in へ委譲する。
pub fn delete_entry(home: &Path, rel_path: &str) -> Result<(), AppError> {
  let (root, mut conn) = open_active(home)?;
  delete_entry_in(&root, &mut conn, rel_path)
}

/// root 直指版の削除（条目持久化）。索引接続は内部で開く。
pub fn delete_entry_at(root: &Path, rel_path: &str) -> Result<(), AppError> {
  let mut conn = index::open_index(root)?;
  delete_entry_in(root, &mut conn, rel_path)
}

/// 削除の実体（delete_entry / delete_entry_at から呼ばれる）。
/// 存在しない条目は err.kb.entryNotFound。
/// 事前検査はせず最初の rename が存在判定を兼ねる（TOCTOU 回避）。ファイルを一時名へ退避 →
/// 索引を事務で清掃 → 成功時に一時ファイルを削除、失敗時は復元する＝どの段階で失敗しても
/// 「ファイルだけ消えて索引に残る」幽霊状態を作らない。
fn delete_entry_in(root: &Path, conn: &mut Connection, rel_path: &str) -> Result<(), AppError> {
  let rel = registry::checked_kb_markdown_path(rel_path, "entries")?;
  let abs = root.join(&rel);
  let tmp = abs.with_extension("md.deleting");
  match fs::rename(&abs, &tmp) {
    Ok(()) => {}
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      return Err(AppError::param("err.kb.entryNotFound", "path", rel_path));
    }
    Err(e) => return Err(AppError::generic(e)),
  }
  let cleaned = (|| {
    let tx = conn.transaction().map_err(AppError::generic)?;
    index::delete_entry(&tx, &rel.to_string_lossy())?;
    tx.commit().map_err(AppError::generic)
  })();
  match cleaned {
    Ok(()) => fs::remove_file(&tmp).map_err(AppError::generic),
    Err(e) => {
      // 索引清掃に失敗＝ファイルを元へ戻す（復元自体の失敗は元エラーを優先して返す）。
      let _ = fs::rename(&tmp, &abs);
      Err(e)
    }
  }
}

/// 条目の生 Markdown を読む。
pub fn read_entry(home: &Path, rel_path: &str) -> Result<String, AppError> {
  let root = active_kb_root(home)?;
  let rel = registry::checked_kb_markdown_path(rel_path, "entries")?;
  fs::read_to_string(root.join(rel)).map_err(AppError::generic)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::kb::infrastructure::config_store;

  #[test]
  fn create_kb_writes_registry_and_kb_config() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("ExpertBase").join("tea");

    let entry = create_kb(home, "茶語", "制茶笔记", kb_path.to_str().unwrap()).unwrap();
    assert_eq!(entry.name, "茶語");
    assert_eq!(entry.path, kb_path.to_string_lossy());

    // グローバル設定に登録され、アクティブになっていること
    let reg = config_store::load_registry(home).unwrap();
    assert_eq!(reg.knowledge_bases, vec![entry]);
    assert_eq!(reg.active.as_deref(), Some(kb_path.to_str().unwrap()));

    // ナレッジベース内にドット始まりの設定が生成されること
    let text = fs::read_to_string(config_store::kb_config_path(&kb_path)).unwrap();
    let config: KbConfig = toml::from_str(&text).unwrap();
    assert_eq!(config.name, "茶語");
    assert_eq!(config.description, "制茶笔记");
  }

  #[test]
  fn create_kb_expands_home_prefix() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let entry = create_kb(home, "kb", "", "~/ExpertBase/kb").unwrap();
    assert_eq!(entry.path, home.join("ExpertBase/kb").to_string_lossy());
    assert!(config_store::kb_config_exists(&home.join("ExpertBase/kb")));
  }

  #[test]
  fn create_kb_rejects_empty_name_and_path() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(create_kb(tmp.path(), "  ", "", "/tmp/x").is_err());
    assert!(create_kb(tmp.path(), "kb", "", "  ").is_err());
  }

  #[test]
  fn create_kb_rejects_duplicate_path() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("kb");
    create_kb(home, "a", "", kb_path.to_str().unwrap()).unwrap();
    let err = create_kb(home, "b", "", kb_path.to_str().unwrap());
    assert!(err.is_err());
  }

  #[test]
  fn create_kb_rejects_existing_kb_config_without_overwriting() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("existing");
    let config_path = config_store::kb_config_path(&kb_path);
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, "name = \"original\"\n").unwrap();

    let err = create_kb(home, "new", "desc", kb_path.to_str().unwrap());

    assert!(err.is_err());
    assert_eq!(fs::read_to_string(config_path).unwrap(), "name = \"original\"\n");
    let reg = config_store::load_registry(home).unwrap();
    assert!(reg.knowledge_bases.is_empty());
    assert!(reg.active.is_none());
  }

  #[test]
  fn set_active_switches_between_registered_kbs() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let first = create_kb(home, "a", "", home.join("a").to_str().unwrap()).unwrap();
    let second = create_kb(home, "b", "", home.join("b").to_str().unwrap()).unwrap();
    assert_eq!(config_store::load_registry(home).unwrap().active, Some(second.path.clone()));

    set_active(home, &first.path).unwrap();
    assert_eq!(config_store::load_registry(home).unwrap().active, Some(first.path));
  }

  #[test]
  fn set_active_rejects_unknown_path() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(set_active(tmp.path(), "/nowhere").is_err());
  }

  #[test]
  fn delete_kb_removes_empty_application_created_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("kb");
    create_kb(home, "k", "", kb_path.to_str().unwrap()).unwrap();
    assert!(kb_path.exists());

    delete_kb(home, kb_path.to_str().unwrap()).unwrap();

    assert!(!kb_path.exists());
    let reg = config_store::load_registry(home).unwrap();
    assert!(reg.knowledge_bases.is_empty());
    assert!(reg.active.is_none());
  }

  #[test]
  fn delete_kb_preserves_unrelated_files_in_existing_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("existing");
    fs::create_dir_all(&kb_path).unwrap();
    fs::write(kb_path.join("notes.txt"), "user data").unwrap();
    create_kb(home, "k", "", kb_path.to_str().unwrap()).unwrap();

    delete_kb(home, kb_path.to_str().unwrap()).unwrap();

    assert_eq!(
      fs::read_to_string(kb_path.join("notes.txt")).unwrap(),
      "user data"
    );
    assert!(!config_store::kb_config_path(&kb_path).exists());
  }

  #[test]
  fn delete_kb_keeps_registry_when_metadata_deletion_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("kb");
    let entry = create_kb(home, "k", "", kb_path.to_str().unwrap()).unwrap();
    fs::remove_dir_all(kb_path.join(".expertbase")).unwrap();
    fs::write(kb_path.join(".expertbase"), "not a directory").unwrap();

    assert!(delete_kb(home, kb_path.to_str().unwrap()).is_err());

    let reg = config_store::load_registry(home).unwrap();
    assert_eq!(reg.knowledge_bases, vec![entry.clone()]);
    assert_eq!(reg.active.as_deref(), Some(entry.path.as_str()));
  }

  #[test]
  fn delete_kb_reassigns_active_to_remaining() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let first = create_kb(home, "a", "", home.join("a").to_str().unwrap()).unwrap();
    let second = create_kb(home, "b", "", home.join("b").to_str().unwrap()).unwrap();
    // 直近作成の second がアクティブ。これを消すと first へ付け替わる。
    delete_kb(home, &second.path).unwrap();

    let reg = config_store::load_registry(home).unwrap();
    assert_eq!(reg.knowledge_bases, vec![first.clone()]);
    assert_eq!(reg.active.as_deref(), Some(first.path.as_str()));
  }

  #[test]
  fn delete_kb_rejects_unknown_path() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(delete_kb(tmp.path(), "/nowhere").is_err());
  }

  #[test]
  fn delete_entry_removes_file_and_index() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("kb");
    create_kb(home, "k", "", kb_path.to_str().unwrap()).unwrap();
    fs::create_dir_all(kb_path.join("entries")).unwrap();
    let content =
      "---\ntype: Entry\ntitle: 緑茶\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n湯温は70度 [[煎茶]]\n";
    save_entry(home, "entries/green.md", content).unwrap();
    let conn = index::open_index(&kb_path).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);

    delete_entry(home, "entries/green.md").unwrap();

    assert!(!kb_path.join("entries/green.md").exists());
    assert_eq!(index::stats(&conn).unwrap().entries, 0);
    assert_eq!(index::stats(&conn).unwrap().links, 0);
    assert!(index::search(&conn, "湯温は").unwrap().is_empty());
  }

  #[test]
  fn delete_entry_restores_file_when_index_cleanup_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("kb");
    create_kb(home, "k", "", kb_path.to_str().unwrap()).unwrap();
    fs::create_dir_all(kb_path.join("entries")).unwrap();
    let content =
      "---\ntype: Entry\ntitle: 緑茶\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n本文\n";
    save_entry(home, "entries/green.md", content).unwrap();
    // entries_fts を通常表（path 列なし）に差し替えて索引清掃を失敗させる
    // （ensure_schema は IF NOT EXISTS なので作り直されない）。
    let conn = index::open_index(&kb_path).unwrap();
    conn
      .execute_batch("DROP TABLE entries_fts; CREATE TABLE entries_fts(dummy);")
      .unwrap();
    drop(conn);

    let result = delete_entry(home, "entries/green.md");

    // 失敗を返し、ファイルは復元される＝「ファイルだけ消えて索引に残る」幽霊状態を作らない。
    assert!(result.is_err());
    assert!(kb_path.join("entries/green.md").exists());
    assert!(!kb_path.join("entries/green.md.deleting").exists());
  }

  #[test]
  fn delete_entry_maps_missing_file_to_entry_not_found_without_precheck() {
    // 連続削除（並行削除の直列化と同型）: 2 回目は err.generic ではなく entryNotFound。
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("kb");
    create_kb(home, "k", "", kb_path.to_str().unwrap()).unwrap();
    fs::create_dir_all(kb_path.join("entries")).unwrap();
    let content =
      "---\ntype: Entry\ntitle: 緑茶\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\n本文\n";
    save_entry(home, "entries/green.md", content).unwrap();

    delete_entry(home, "entries/green.md").unwrap();
    let err = delete_entry(home, "entries/green.md").unwrap_err();

    assert_eq!(err.code, "err.kb.entryNotFound");
  }

  #[test]
  fn delete_entry_rejects_missing_entry_and_bad_path() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("kb");
    create_kb(home, "k", "", kb_path.to_str().unwrap()).unwrap();

    let err = delete_entry(home, "entries/nope.md").unwrap_err();
    assert_eq!(err.code, "err.kb.entryNotFound");
    // KB 外パスは checked_kb_markdown_path で拒否される。
    assert!(delete_entry(home, "../secret.md").is_err());
  }

  #[test]
  fn create_entry_records_source_refs_as_entry_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let rel = create_entry(root, "緑茶", "tea", "本文", &["/abs/report.pdf".into()]).unwrap();

    let saved = fs::read_to_string(root.join(&rel)).unwrap();
    let parsed = entry::parse_entry(&saved).unwrap();
    assert_eq!(parsed.meta.sources, vec!["/abs/report.pdf".to_string()]);
  }

  #[test]
  fn create_entry_writes_one_entry_and_indexes_links() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // 複数素材を 1 条目に合成する。引用は外部パスの文字列として残す。
    let refs = vec!["/abs/a.pdf".to_string(), "/abs/b.docx".to_string()];
    let rel = create_entry(root, "緑茶", "tea", "湯温は [[煎茶]] で70度", &refs).unwrap();

    assert!(root.join(&rel).is_file());
    let conn = index::open_index(root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
    let saved = fs::read_to_string(root.join(&rel)).unwrap();
    let parsed = entry::parse_entry(&saved).unwrap();
    assert_eq!(parsed.meta.sources, refs);
  }

  #[test]
  fn update_entry_body_replaces_body_keeps_meta_and_reindexes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let rel = create_entry(root, "緑茶", "tea", "湯温は70度", &["/abs/a.pdf".into()]).unwrap();

    update_entry_body(root, &rel, "湯温は80度 [[煎茶]]").unwrap();

    // メタ（title / cat / sources / created）は維持、本文差し替え、updated は当日へ。
    let saved = fs::read_to_string(root.join(&rel)).unwrap();
    let parsed = entry::parse_entry(&saved).unwrap();
    assert_eq!(parsed.body, "湯温は80度 [[煎茶]]");
    assert_eq!(parsed.meta.title, "緑茶");
    assert_eq!(parsed.meta.cat, "tea");
    assert_eq!(parsed.meta.sources, vec!["/abs/a.pdf".to_string()]);
    assert_eq!(parsed.meta.updated, chrono::Utc::now().format("%Y-%m-%d").to_string());
    // 索引も新本文で引ける（旧本文は消える）。
    let conn = index::open_index(root).unwrap();
    assert_eq!(index::search(&conn, "湯温は80度").unwrap().len(), 1);
    assert!(index::search(&conn, "湯温は70度").unwrap().is_empty());
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
  }

  #[test]
  fn update_entry_body_rejects_escaping_path() {
    let tmp = tempfile::tempdir().unwrap();
    // KB 外パスは checked_kb_markdown_path で拒否される（delete と同じ防御）。
    assert!(update_entry_body(tmp.path(), "../secret.md", "x").is_err());
  }

  #[test]
  fn delete_entry_at_removes_file_and_index_with_root_only() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let rel = create_entry(root, "緑茶", "tea", "湯温は70度 [[煎茶]]", &[]).unwrap();

    delete_entry_at(root, &rel).unwrap();

    assert!(!root.join(&rel).exists());
    let conn = index::open_index(root).unwrap();
    assert_eq!(index::stats(&conn).unwrap().entries, 0);
    assert_eq!(index::stats(&conn).unwrap().links, 0);
  }

  #[test]
  fn create_entry_keeps_file_when_index_update_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // entries_fts を通常表に差し替えて索引更新を失敗させる
    // （ensure_schema は IF NOT EXISTS なので作り直されない）。
    let conn = index::open_index(root).unwrap();
    conn
      .execute_batch("DROP TABLE entries_fts; CREATE TABLE entries_fts(dummy);")
      .unwrap();
    drop(conn);

    let result = create_entry(root, "緑茶", "tea", "本文", &[]);

    // ファイルが真源：索引更新に失敗しても書いた条目は残る（索引は rebuild で復元可能）。
    assert!(result.is_err());
    assert!(root.join("entries/緑茶.md").is_file());
  }
}
