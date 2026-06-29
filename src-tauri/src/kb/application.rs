//! kb アプリケーション層。ユースケースを表し、ドメインを編成して
//! インフラ抽象（config_store / index / store）に依存する。

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::kb::domain::entry;
use crate::kb::domain::registry::{self, KbConfig, KbEntry};
use crate::kb::infrastructure::{config_store, index};

/// アクティブなナレッジベースのルートパスを返す。未選択ならエラー。
pub(crate) fn active_kb_root(home: &Path) -> Result<PathBuf, String> {
  let reg = config_store::load_registry(home)?;
  let active = reg.active.ok_or("没有激活的知识库")?;
  Ok(PathBuf::from(active))
}

/// アクティブ KB のルートとインデックス接続をまとめて開く。
pub(crate) fn open_active(home: &Path) -> Result<(PathBuf, Connection), String> {
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
) -> Result<KbEntry, String> {
  let name = name.trim();
  if name.is_empty() {
    return Err("知识库名称不能为空".into());
  }
  let raw_path = raw_path.trim();
  if raw_path.is_empty() {
    return Err("存储位置不能为空".into());
  }
  let path = registry::expand_home(home, raw_path);
  let path_str = path.to_string_lossy().into_owned();

  let mut reg = config_store::load_registry(home)?;
  if reg.knowledge_bases.iter().any(|k| k.path == path_str) {
    return Err("该位置已注册为知识库".into());
  }

  if config_store::kb_config_exists(&path) {
    return Err("该目录已经包含 ExpertBase 知识库，请选择其他位置".into());
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
pub fn set_active(home: &Path, path: &str) -> Result<(), String> {
  let mut reg = config_store::load_registry(home)?;
  if !reg.knowledge_bases.iter().any(|k| k.path == path) {
    return Err("未找到该知识库".into());
  }
  reg.active = Some(path.into());
  config_store::save_registry(home, &reg)
}

/// 登録済みナレッジベースを削除する。ExpertBase のメタデータだけを削除し、
/// 空になった場合に限ってルートも削除してからレジストリを更新する。
pub fn delete_kb(home: &Path, path: &str) -> Result<(), String> {
  let mut reg = config_store::load_registry(home)?;
  let idx = reg
    .knowledge_bases
    .iter()
    .position(|k| k.path == path)
    .ok_or("未找到该知识库")?;
  let root = Path::new(path);
  let metadata = root.join(".expertbase");
  if metadata.exists() {
    if !metadata.is_dir() {
      return Err("知识库元数据不是目录".into());
    }
    fs::remove_dir_all(&metadata).map_err(|e| e.to_string())?;
  }
  match fs::remove_dir(root) {
    Ok(()) => {}
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
    // ponytail: 空でないルートはユーザーデータを含む可能性があるため残す。
    Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
    Err(e) => return Err(e.to_string()),
  }

  reg.knowledge_bases.remove(idx);
  if reg.active.as_deref() == Some(path) {
    reg.active = reg.knowledge_bases.first().map(|k| k.path.clone());
  }
  config_store::save_registry(home, &reg)
}

/// 条目を上書き保存する（frontmatter 検証付き）。保存前に検証し、不正なら書き込まない。
pub fn save_entry(home: &Path, rel_path: &str, content: &str) -> Result<(), String> {
  let (root, conn) = open_active(home)?;
  let rel = registry::checked_kb_markdown_path(rel_path, "entries")?;
  let parsed = entry::parse_entry(content)?;
  fs::write(root.join(&rel), content).map_err(|e| e.to_string())?;
  index::upsert_entry(&conn, &rel.to_string_lossy(), &parsed)
}

/// 条目の生 Markdown を読む。
pub fn read_entry(home: &Path, rel_path: &str) -> Result<String, String> {
  let root = active_kb_root(home)?;
  let rel = registry::checked_kb_markdown_path(rel_path, "entries")?;
  fs::read_to_string(root.join(rel)).map_err(|e| e.to_string())
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

}
