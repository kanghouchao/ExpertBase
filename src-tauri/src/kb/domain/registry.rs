use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

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

/// `~` / `~/` 始まりのパスをホームディレクトリ配下に展開する。
pub(crate) fn expand_home(home: &Path, raw: &str) -> PathBuf {
  if raw == "~" {
    home.to_path_buf()
  } else if let Some(rest) = raw.strip_prefix("~/") {
    home.join(rest)
  } else {
    PathBuf::from(raw)
  }
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

#[cfg(test)]
mod tests {
  use super::*;

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
