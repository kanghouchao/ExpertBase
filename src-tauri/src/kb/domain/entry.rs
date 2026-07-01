use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 通常条目の `type` 既定値（OKF 互換の必須フィールド）。
fn default_entry_type() -> String {
  "Entry".to_string()
}

/// 条目（entries/*.md）の frontmatter。宣言順が YAML 出力順になる。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EntryMeta {
  #[serde(rename = "type", default = "default_entry_type")]
  pub kind: String,
  pub title: String,
  #[serde(default)]
  pub description: String,
  #[serde(default)]
  pub cat: String,
  #[serde(default)]
  pub tags: Vec<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub sources: Vec<String>,
  pub created: String,
  pub updated: String,
}

/// frontmatter + 本文。本文中の `[[タイトル]]` がリンク。
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
  pub meta: EntryMeta,
  pub body: String,
}

const FENCE: &str = "---";

/// frontmatter(YAML) と本文に分割する共通ヘルパ。条目と受信箱素材で共用する。
/// 返り値: (YAML 文字列, 本文文字列)。
pub fn split_frontmatter(raw: &str) -> Result<(String, String), AppError> {
  // 先頭 BOM と CR を許容する。
  let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
  let rest = raw
    .strip_prefix(&format!("{FENCE}\n"))
    .or_else(|| raw.strip_prefix(&format!("{FENCE}\r\n")))
    .ok_or_else(|| AppError::code("err.kb.entryFrontmatterMissing"))?;
  // 終端フェンス（行頭の `\n---`）を探す。
  let end = rest
    .find(&format!("\n{FENCE}"))
    .ok_or_else(|| AppError::code("err.kb.entryFrontmatterUnterminated"))?;
  let yaml = rest[..end].to_string();
  // 終端フェンス行とその後の改行を飛ばして本文を取り出す。
  let after = &rest[end + 1 + FENCE.len()..];
  let body = after
    .strip_prefix("\r\n")
    .or_else(|| after.strip_prefix('\n'))
    .unwrap_or(after);
  // frontmatter と本文の間の空行を 1 つ許容する。
  let body = body
    .strip_prefix("\r\n")
    .or_else(|| body.strip_prefix('\n'))
    .unwrap_or(body);
  Ok((yaml, body.to_string()))
}

/// frontmatter 付き Markdown を Entry に解析する。
pub fn parse_entry(raw: &str) -> Result<Entry, AppError> {
  let (yaml, body) = split_frontmatter(raw)?;
  let meta: EntryMeta = serde_yaml::from_str(&yaml).map_err(AppError::generic)?;
  Ok(Entry { meta, body })
}

/// Entry を frontmatter 付き Markdown 文字列へ直列化する。
pub fn serialize_entry(entry: &Entry) -> Result<String, AppError> {
  let yaml = serde_yaml::to_string(&entry.meta).map_err(AppError::generic)?;
  Ok(format!("{FENCE}\n{yaml}{FENCE}\n\n{}", entry.body))
}

fn link_re() -> &'static Regex {
  static RE: OnceLock<Regex> = OnceLock::new();
  RE.get_or_init(|| Regex::new(r"\[\[([^\]\[]+)\]\]").unwrap())
}

/// 本文中の `[[タイトル]]` を出現順・重複排除で抽出する。
pub fn extract_links(body: &str) -> Vec<String> {
  let mut seen = std::collections::HashSet::new();
  let mut out = Vec::new();
  for cap in link_re().captures_iter(body) {
    let title = cap[1].trim().to_string();
    if seen.insert(title.clone()) {
      out.push(title);
    }
  }
  out
}

/// 空白区切りの語数。MVP の統計用の素朴な実装。
pub fn word_count(body: &str) -> usize {
  body.split_whitespace().count()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample() -> Entry {
    Entry {
      meta: EntryMeta {
        kind: "Entry".into(),
        title: "緑茶の淹れ方".into(),
        description: "基本手順".into(),
        cat: "tea".into(),
        tags: vec!["howto".into(), "tea".into()],
        sources: vec![],
        created: "2026-06-14".into(),
        updated: "2026-06-14".into(),
      },
      body: "湯温は [[煎茶]] で 70 度。\n\n参考: [[水質]]。\n".into(),
    }
  }

  #[test]
  fn parse_then_serialize_round_trips() {
    let entry = sample();
    let text = serialize_entry(&entry).unwrap();
    let parsed = parse_entry(&text).unwrap();
    assert_eq!(parsed, entry);
  }

  #[test]
  fn parse_defaults_type_when_missing() {
    let raw = "---\ntitle: t\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\nbody\n";
    let entry = parse_entry(raw).unwrap();
    assert_eq!(entry.meta.kind, "Entry");
    assert_eq!(entry.body, "body\n");
  }

  #[test]
  fn parse_rejects_text_without_frontmatter() {
    assert!(parse_entry("no frontmatter here").is_err());
  }

  #[test]
  fn extract_links_dedupes_in_order() {
    assert_eq!(
      extract_links("a [[X]] b [[Y]] c [[X]]"),
      vec!["X".to_string(), "Y".to_string()]
    );
  }

  #[test]
  fn word_count_counts_whitespace_separated_tokens() {
    assert_eq!(word_count("hello  world\nthree"), 3);
  }

  #[test]
  fn parse_then_serialize_round_trips_with_sources() {
    let mut entry = sample();
    entry.meta.sources = vec!["inbox/foo.md".into(), "inbox/bar.md".into()];
    let text = serialize_entry(&entry).unwrap();
    assert!(text.contains("sources:"));
    assert_eq!(parse_entry(&text).unwrap(), entry);
  }

  #[test]
  fn parse_defaults_sources_when_missing() {
    let raw = "---\ntitle: t\ncreated: 2026-06-14\nupdated: 2026-06-14\n---\n\nbody\n";
    assert!(parse_entry(raw).unwrap().meta.sources.is_empty());
  }

  #[test]
  fn serialize_omits_empty_sources() {
    // 既存 entries の出力を変えないこと（空 sources は出力しない）。
    assert!(!serialize_entry(&sample()).unwrap().contains("sources:"));
  }
}
