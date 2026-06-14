use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

pub mod entry;
pub mod index;
pub mod store;

/// グローバル設定ディレクトリ（ユーザーホーム直下）。
const CONFIG_DIR: &str = ".expertBase";
/// グローバル設定ファイル名。登録済みナレッジベースの一覧を保持する。
const CONFIG_FILE: &str = "config.toml";
/// 各ナレッジベース内の設定ディレクトリ（ドット始まりで一般ユーザーには不可視）。
const KB_DIR: &str = ".expertbase";
/// ナレッジベース個別の設定ファイル名。
const KB_FILE: &str = "kb.toml";

/// 登録済みナレッジベースの 1 件。パスを一意な識別子として扱う。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KbEntry {
  pub name: String,
  pub path: String,
}

/// `~/.expertBase/config.toml` の内容。
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Registry {
  /// 現在アクティブなナレッジベースのパス。
  pub active: Option<String>,
  #[serde(default)]
  pub knowledge_bases: Vec<KbEntry>,
}

/// `<ナレッジベース>/.expertbase/kb.toml` の内容。
#[derive(Serialize, Deserialize, Debug)]
pub struct KbConfig {
  pub name: String,
  #[serde(default)]
  pub description: String,
}

fn config_path(home: &Path) -> PathBuf {
  home.join(CONFIG_DIR).join(CONFIG_FILE)
}

/// グローバル設定を読み込む。ファイルが無ければ空の Registry を返す。
pub fn load_registry(home: &Path) -> Result<Registry, String> {
  let path = config_path(home);
  if !path.exists() {
    return Ok(Registry::default());
  }
  let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  toml::from_str(&text).map_err(|e| e.to_string())
}

/// グローバル設定を書き込む。`.expertBase` ディレクトリが無ければ作成する。
pub fn save_registry(home: &Path, registry: &Registry) -> Result<(), String> {
  let path = config_path(home);
  if let Some(dir) = path.parent() {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
  }
  let text = toml::to_string_pretty(registry).map_err(|e| e.to_string())?;
  fs::write(&path, text).map_err(|e| e.to_string())
}

/// `~` / `~/` 始まりのパスをホームディレクトリ配下に展開する。
fn expand_home(home: &Path, raw: &str) -> PathBuf {
  if raw == "~" {
    home.to_path_buf()
  } else if let Some(rest) = raw.strip_prefix("~/") {
    home.join(rest)
  } else {
    PathBuf::from(raw)
  }
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
  let path = expand_home(home, raw_path);
  let path_str = path.to_string_lossy().into_owned();

  let mut registry = load_registry(home)?;
  if registry.knowledge_bases.iter().any(|k| k.path == path_str) {
    return Err("该位置已注册为知识库".into());
  }

  let kb_dir = path.join(KB_DIR);
  let kb_config_path = kb_dir.join(KB_FILE);
  if kb_config_path.exists() {
    return Err("该目录已经包含 ExpertBase 知识库，请选择其他位置".into());
  }
  fs::create_dir_all(&kb_dir).map_err(|e| e.to_string())?;
  let config = KbConfig {
    name: name.into(),
    description: description.trim().into(),
  };
  let text = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
  fs::write(kb_config_path, text).map_err(|e| e.to_string())?;

  let entry = KbEntry {
    name: name.into(),
    path: path_str.clone(),
  };
  registry.knowledge_bases.push(entry.clone());
  registry.active = Some(path_str);
  save_registry(home, &registry)?;
  Ok(entry)
}

/// 登録済みナレッジベースをアクティブに切り替える。
pub fn set_active(home: &Path, path: &str) -> Result<(), String> {
  let mut registry = load_registry(home)?;
  if !registry.knowledge_bases.iter().any(|k| k.path == path) {
    return Err("未找到该知识库".into());
  }
  registry.active = Some(path.into());
  save_registry(home, &registry)
}

/// `kb_list` の応答。フロントの初期化ウィザード向けに既定の親ディレクトリも返す。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KbList {
  pub kbs: Vec<KbEntry>,
  pub active: Option<String>,
  pub default_parent: String,
}

#[tauri::command]
pub fn kb_list(app: tauri::AppHandle) -> Result<KbList, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let registry = load_registry(&home)?;
  Ok(KbList {
    kbs: registry.knowledge_bases,
    active: registry.active,
    default_parent: home.join("ExpertBase").to_string_lossy().into_owned(),
  })
}

#[tauri::command]
pub fn kb_create(
  app: tauri::AppHandle,
  name: String,
  description: String,
  path: String,
) -> Result<KbEntry, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  create_kb(&home, &name, &description, &path)
}

#[tauri::command]
pub fn kb_set_active(app: tauri::AppHandle, path: String) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  set_active(&home, &path)
}

/// アクティブなナレッジベースのルートパスを返す。未選択ならエラー。
pub(crate) fn active_kb_root(home: &Path) -> Result<PathBuf, String> {
  let registry = load_registry(home)?;
  let active = registry.active.ok_or("没有激活的知识库")?;
  Ok(PathBuf::from(active))
}

/// アクティブ KB のルートとインデックス接続をまとめて開く。
pub(crate) fn open_active(home: &Path) -> Result<(PathBuf, rusqlite::Connection), String> {
  let root = active_kb_root(home)?;
  let conn = index::open_index(&root)?;
  Ok((root, conn))
}

/// IPC から受け取る KB 内パスを、許可された直下 Markdown ファイルに限定する。
pub(crate) fn checked_kb_markdown_path(rel_path: &str, dir: &str) -> Result<PathBuf, String> {
  let path = Path::new(rel_path);
  if path.is_absolute() {
    return Err("知识库路径必须是相对路径".into());
  }
  let parts = path.components().collect::<Vec<_>>();
  match parts.as_slice() {
    [Component::Normal(prefix), Component::Normal(file)]
      if prefix.to_string_lossy() == dir
        && !file.to_string_lossy().is_empty()
        && Path::new(file).extension().and_then(|s| s.to_str()) == Some("md") =>
    {
      Ok(PathBuf::from(dir).join(file))
    }
    _ => Err("知识库路径不在允许的 Markdown 目录内".into()),
  }
}

#[tauri::command]
pub fn kb_rebuild_index(app: tauri::AppHandle) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = open_active(&home)?;
  index::rebuild(&conn, &root)
}

#[tauri::command]
pub fn kb_list_entries(app: tauri::AppHandle) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::list_entries(&conn)
}

#[tauri::command]
pub fn kb_search(app: tauri::AppHandle, query: String) -> Result<Vec<index::SearchHit>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::search(&conn, &query)
}

#[tauri::command]
pub fn kb_backlinks(app: tauri::AppHandle, title: String) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::backlinks(&conn, &title)
}

#[tauri::command]
pub fn kb_stats(app: tauri::AppHandle) -> Result<index::Stats, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::stats(&conn)
}

#[tauri::command]
pub fn kb_graph(app: tauri::AppHandle) -> Result<index::GraphData, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::graph(&conn)
}

#[tauri::command]
pub fn kb_orphans(app: tauri::AppHandle) -> Result<Vec<index::EntryRef>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::orphans(&conn)
}

#[tauri::command]
pub fn kb_read_entry(app: tauri::AppHandle, path: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let root = active_kb_root(&home)?;
  let rel = checked_kb_markdown_path(&path, "entries")?;
  fs::read_to_string(root.join(rel)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kb_read_inbox_material(app: tauri::AppHandle, path: String) -> Result<String, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let root = active_kb_root(&home)?;
  let rel = checked_kb_markdown_path(&path, "inbox")?;
  fs::read_to_string(root.join(rel)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kb_save_entry(app: tauri::AppHandle, path: String, content: String) -> Result<(), String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (root, conn) = open_active(&home)?;
  let rel = checked_kb_markdown_path(&path, "entries")?;
  // 保存前に frontmatter を検証する（不正なら書き込まない）。
  let parsed = entry::parse_entry(&content)?;
  fs::write(root.join(&rel), &content).map_err(|e| e.to_string())?;
  index::upsert_entry(&conn, &rel.to_string_lossy(), &parsed)
}

#[tauri::command]
pub fn kb_list_inbox(app: tauri::AppHandle) -> Result<Vec<index::InboxItem>, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let (_root, conn) = open_active(&home)?;
  index::list_inbox(&conn)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn load_registry_returns_default_when_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = load_registry(tmp.path()).unwrap();
    assert!(registry.knowledge_bases.is_empty());
    assert!(registry.active.is_none());
  }

  #[test]
  fn create_kb_writes_registry_and_kb_config() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let kb_path = home.join("ExpertBase").join("tea");

    let entry = create_kb(home, "茶語", "制茶笔记", kb_path.to_str().unwrap()).unwrap();
    assert_eq!(entry.name, "茶語");
    assert_eq!(entry.path, kb_path.to_string_lossy());

    // グローバル設定に登録され、アクティブになっていること
    let registry = load_registry(home).unwrap();
    assert_eq!(registry.knowledge_bases, vec![entry]);
    assert_eq!(registry.active.as_deref(), Some(kb_path.to_str().unwrap()));

    // ナレッジベース内にドット始まりの設定が生成されること
    let text = fs::read_to_string(kb_path.join(KB_DIR).join(KB_FILE)).unwrap();
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
    assert!(home.join("ExpertBase/kb").join(KB_DIR).is_dir());
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
    let config_path = kb_path.join(KB_DIR).join(KB_FILE);
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, "name = \"original\"\n").unwrap();

    let err = create_kb(home, "new", "desc", kb_path.to_str().unwrap());

    assert!(err.is_err());
    assert_eq!(fs::read_to_string(config_path).unwrap(), "name = \"original\"\n");
    let registry = load_registry(home).unwrap();
    assert!(registry.knowledge_bases.is_empty());
    assert!(registry.active.is_none());
  }

  #[test]
  fn set_active_switches_between_registered_kbs() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let first = create_kb(home, "a", "", home.join("a").to_str().unwrap()).unwrap();
    let second = create_kb(home, "b", "", home.join("b").to_str().unwrap()).unwrap();
    assert_eq!(load_registry(home).unwrap().active, Some(second.path.clone()));

    set_active(home, &first.path).unwrap();
    assert_eq!(load_registry(home).unwrap().active, Some(first.path));
  }

  #[test]
  fn checked_kb_markdown_path_accepts_expected_dirs_only() {
    assert_eq!(
      checked_kb_markdown_path("inbox/a.md", "inbox").unwrap(),
      PathBuf::from("inbox/a.md")
    );
    assert!(checked_kb_markdown_path("inbox/a.md", "entries").is_err());
    assert!(checked_kb_markdown_path("inbox/nested/a.md", "inbox").is_err());
    assert!(checked_kb_markdown_path("../inbox/a.md", "inbox").is_err());
  }

  #[test]
  fn set_active_rejects_unknown_path() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(set_active(tmp.path(), "/nowhere").is_err());
  }

  #[test]
  fn checked_kb_markdown_path_rejects_escape_paths() {
    assert_eq!(
      checked_kb_markdown_path("entries/a.md", "entries").unwrap(),
      PathBuf::from("entries/a.md")
    );
    assert!(checked_kb_markdown_path("../secret.md", "entries").is_err());
    assert!(checked_kb_markdown_path("entries/../secret.md", "entries").is_err());
    assert!(checked_kb_markdown_path("/tmp/secret.md", "entries").is_err());
    assert!(checked_kb_markdown_path("inbox/a.md", "entries").is_err());
    assert!(checked_kb_markdown_path("entries/nested/a.md", "entries").is_err());
    assert!(checked_kb_markdown_path("entries/a.txt", "entries").is_err());
  }
}
