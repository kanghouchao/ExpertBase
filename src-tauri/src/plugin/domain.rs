//! plugin ドメイン層。Skill 値構造体、SKILL.md frontmatter の解析（純関数、IO なし）、
//! system prompt の `# Skills` / `# Activated Skills` 節のレンダリング。
//! 永続化 / FS / Tauri / IPC DTO の詳細には依存しない（生テキストと dir_name を受け取るだけ）。

use serde::{Deserialize, Serialize};

/// 技能の由来。同名なら Kb が勝つ（infrastructure::scan の走査順で担保）。
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SkillSource {
  Kb,
  User,
}

/// 発見済み技能 1 件。IPC 境界を越える（`plugin_list_skills` がそのまま返す）。
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
  pub name: String,
  pub description: String,
  /// frontmatter 剥離済みの本文。
  pub body: String,
  /// SKILL.md への絶対パス（表示用）。
  pub location: String,
  pub source: SkillSource,
  /// `scripts/` サブディレクトリの有無（本バージョンは実行しない、UI 注記用）。
  pub has_scripts: bool,
}

/// 技能をスキップする理由。4 条件とも同等に扱い、寛容な解析で走査全体は止めない。
#[derive(Debug, PartialEq)]
pub enum SkipReason {
  MalformedFrontmatter,
  NameMissing,
  DescriptionMissing,
  NameMismatch,
}

const FENCE: &str = "---";

/// frontmatter(YAML) と本文に分割する。`kb::domain::entry::split_frontmatter` と同じ手法
/// （BOM/CRLF 許容、`---` の二重フェンス）だが、plugin は kb の internal 実装へ越境しない
/// （plugin は将来 MCP も同居する業務非依存モジュール）ので独立して持つ。
fn split_frontmatter(raw: &str) -> Option<(String, String)> {
  let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
  let rest = raw
    .strip_prefix(&format!("{FENCE}\n"))
    .or_else(|| raw.strip_prefix(&format!("{FENCE}\r\n")))?;
  let end = rest.find(&format!("\n{FENCE}"))?;
  let yaml = rest[..end].to_string();
  let after = &rest[end + 1 + FENCE.len()..];
  let body = after.strip_prefix("\r\n").or_else(|| after.strip_prefix('\n')).unwrap_or(after);
  let body = body.strip_prefix("\r\n").or_else(|| body.strip_prefix('\n')).unwrap_or(body);
  Some((yaml, body.to_string()))
}

#[derive(Deserialize, Default)]
struct SkillFrontmatter {
  #[serde(default)]
  name: Option<String>,
  #[serde(default)]
  description: Option<String>,
}

/// SKILL.md の生テキストを解析する。純関数（IO なし）。
/// 失敗は 4 条件のいずれか一つに分類する（寛容な解析: 呼び出し側は該当スキルだけ捨てて走査を続ける）。
pub fn parse_skill_frontmatter(
  raw: &str,
  dir_name: &str,
) -> Result<(String, String, String), SkipReason> {
  let (yaml, body) = split_frontmatter(raw).ok_or(SkipReason::MalformedFrontmatter)?;
  let fm: SkillFrontmatter =
    serde_yaml::from_str(&yaml).map_err(|_| SkipReason::MalformedFrontmatter)?;
  let name = fm.name.filter(|s| !s.trim().is_empty()).ok_or(SkipReason::NameMissing)?;
  let description =
    fm.description.filter(|s| !s.trim().is_empty()).ok_or(SkipReason::DescriptionMissing)?;
  if name != dir_name {
    return Err(SkipReason::NameMismatch);
  }
  Ok((name, description, body))
}

/// `# Skills` catalog 節の本文。発見済み全技能の `- name: description` 箇条書き。
/// 0 件なら空文字列（呼び出し側は空文字列のとき節ごと省略する）。
pub fn render_catalog(skills: &[Skill]) -> String {
  skills.iter().map(|s| format!("- {}: {}", s.name, s.description)).collect::<Vec<_>>().join("\n")
}

/// `# Activated Skills` 節の本文。`activated` に列挙された名前に対応する技能の本文を
/// `## name\nbody` で連結する。`activated` に無い/見つからない名前は無視。空なら空文字列。
pub fn render_activated(skills: &[Skill], activated: &[String]) -> String {
  activated
    .iter()
    .filter_map(|name| skills.iter().find(|s| &s.name == name))
    .map(|s| format!("## {}\n{}", s.name, s.body))
    .collect::<Vec<_>>()
    .join("\n\n")
}

#[cfg(test)]
mod tests {
  use super::*;

  fn skill(name: &str, description: &str, body: &str) -> Skill {
    Skill {
      name: name.to_string(),
      description: description.to_string(),
      body: body.to_string(),
      location: format!("/skills/{name}/SKILL.md"),
      source: SkillSource::Kb,
      has_scripts: false,
    }
  }

  #[test]
  fn parse_skill_frontmatter_extracts_name_description_and_body() {
    let raw = "---\nname: tea-brewing\ndescription: 緑茶の淹れ方を案内する\n---\n\n本文の内容";
    let (name, description, body) = parse_skill_frontmatter(raw, "tea-brewing").unwrap();
    assert_eq!(name, "tea-brewing");
    assert_eq!(description, "緑茶の淹れ方を案内する");
    assert_eq!(body, "本文の内容");
  }

  #[test]
  fn parse_skill_frontmatter_reports_malformed_frontmatter() {
    // フェンスが無い。
    assert_eq!(parse_skill_frontmatter("no fence here", "x").unwrap_err(), SkipReason::MalformedFrontmatter);
    // フェンスはあるが YAML が壊れている。
    let raw = "---\nname: [unterminated\n---\n\nbody";
    assert_eq!(parse_skill_frontmatter(raw, "x").unwrap_err(), SkipReason::MalformedFrontmatter);
  }

  #[test]
  fn parse_skill_frontmatter_reports_name_missing() {
    let raw = "---\ndescription: 説明のみ\n---\n\nbody";
    assert_eq!(parse_skill_frontmatter(raw, "x").unwrap_err(), SkipReason::NameMissing);
    let raw_blank = "---\nname: \"\"\ndescription: 説明\n---\n\nbody";
    assert_eq!(parse_skill_frontmatter(raw_blank, "x").unwrap_err(), SkipReason::NameMissing);
  }

  #[test]
  fn parse_skill_frontmatter_reports_description_missing() {
    let raw = "---\nname: x\n---\n\nbody";
    assert_eq!(parse_skill_frontmatter(raw, "x").unwrap_err(), SkipReason::DescriptionMissing);
    let raw_blank = "---\nname: x\ndescription: \"  \"\n---\n\nbody";
    assert_eq!(parse_skill_frontmatter(raw_blank, "x").unwrap_err(), SkipReason::DescriptionMissing);
  }

  #[test]
  fn parse_skill_frontmatter_reports_name_mismatch() {
    let raw = "---\nname: tea-brewing\ndescription: 説明\n---\n\nbody";
    assert_eq!(parse_skill_frontmatter(raw, "coffee-brewing").unwrap_err(), SkipReason::NameMismatch);
  }

  #[test]
  fn parse_skill_frontmatter_tolerates_bom_crlf_and_blank_line_after_frontmatter() {
    let raw = "\u{feff}---\r\nname: x\r\ndescription: y\r\n---\r\n\r\nbody\r\n";
    let (name, description, body) = parse_skill_frontmatter(raw, "x").unwrap();
    assert_eq!((name.as_str(), description.as_str()), ("x", "y"));
    assert_eq!(body, "body\r\n");
  }

  #[test]
  fn render_catalog_lists_all_skills_and_is_empty_for_no_skills() {
    assert_eq!(render_catalog(&[]), "");
    let skills = vec![skill("a", "説明A", "本文A"), skill("b", "説明B", "本文B")];
    let out = render_catalog(&skills);
    assert_eq!(out, "- a: 説明A\n- b: 説明B");
  }

  #[test]
  fn render_activated_joins_bodies_for_activated_names_only() {
    let skills = vec![skill("a", "説明A", "本文A"), skill("b", "説明B", "本文B")];
    assert_eq!(render_activated(&skills, &[]), "");
    let out = render_activated(&skills, &["b".to_string()]);
    assert_eq!(out, "## b\n本文B");
    // 存在しない名前は無視する。
    let out = render_activated(&skills, &["missing".to_string(), "a".to_string()]);
    assert_eq!(out, "## a\n本文A");
  }
}
