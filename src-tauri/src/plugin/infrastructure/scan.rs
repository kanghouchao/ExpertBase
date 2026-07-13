//! plugin インフラ: 二段ディレクトリ走査でスキルを発見する。
//! user スコープ（`~/.agents/skills/`）→ KB スコープ（`<kb ルート>/skills/`）の順に走査し、
//! 同名は後勝ち（KB が勝つ）で HashMap に upsert する。常駐状態・キャッシュ・ファイル監視は無し、
//! 呼び出しごとに毎回走査する（設計の確定した決定1）。

use std::collections::HashMap;
use std::path::Path;

use crate::plugin::domain::parse_skill_frontmatter;
use crate::plugin::{Skill, SkillSource};

/// KB 内 `skills/` と `~/.agents/skills/` を走査し、決定的順序（name 昇順）の技能一覧を返す。
/// `kb_root` が `None`（アクティブ KB が無い、例: 設定ダイアログを KB 未選択で開いた場合）なら
/// KB 側の走査を省略し、user 側だけを返す（呼び出し側をエラーにしない）。
pub(crate) fn discover_skills(kb_root: Option<&Path>, home: &Path) -> Vec<Skill> {
  let mut by_name: HashMap<String, Skill> = HashMap::new();
  for skill in scan_dir(&home.join(".agents/skills"), SkillSource::User) {
    by_name.insert(skill.name.clone(), skill);
  }
  if let Some(kb_root) = kb_root {
    for skill in scan_dir(&kb_root.join("skills"), SkillSource::Kb) {
      by_name.insert(skill.name.clone(), skill); // 同名は KB が勝つ（後勝ち）。
    }
  }
  let mut skills: Vec<Skill> = by_name.into_values().collect();
  skills.sort_by(|a, b| a.name.cmp(&b.name));
  skills
}

/// 1 スコープ分の走査。ディレクトリが無ければ空 Vec（エラーにしない）。
/// 各サブディレクトリの `SKILL.md` を読み、解析失敗は `log::warn!` して当該スキルだけ捨てる
/// （寛容な解析、走査全体は継続）。`SKILL.md` 自体が無いサブディレクトリはスキルではないので無視。
fn scan_dir(dir: &Path, source: SkillSource) -> Vec<Skill> {
  let mut skills = Vec::new();
  let Ok(entries) = std::fs::read_dir(dir) else {
    return skills;
  };
  for entry in entries.flatten() {
    let path = entry.path();
    if !path.is_dir() {
      continue;
    }
    let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else {
      continue;
    };
    let skill_md = path.join("SKILL.md");
    let Ok(raw) = std::fs::read_to_string(&skill_md) else {
      continue;
    };
    match parse_skill_frontmatter(&raw, dir_name) {
      Ok((name, description, body)) => {
        skills.push(Skill {
          name,
          description,
          body,
          location: skill_md.to_string_lossy().into_owned(),
          source,
          has_scripts: path.join("scripts").is_dir(),
        });
      }
      Err(reason) => {
        log::warn!("skill skipped: {dir_name} ({reason:?})");
      }
    }
  }
  skills
}

#[cfg(test)]
mod tests {
  use super::*;

  fn write_skill(dir: &Path, name: &str, frontmatter_name: &str, description: &str, body: &str) {
    let skill_dir = dir.join(name);
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
      skill_dir.join("SKILL.md"),
      format!("---\nname: {frontmatter_name}\ndescription: {description}\n---\n\n{body}"),
    )
    .unwrap();
  }

  #[test]
  fn discover_skills_returns_empty_when_neither_directory_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let skills = discover_skills(Some(&tmp.path().join("kb")), &tmp.path().join("home"));
    assert!(skills.is_empty());
  }

  #[test]
  fn discover_skills_finds_kb_only_skills() {
    let tmp = tempfile::tempdir().unwrap();
    let kb_root = tmp.path().join("kb");
    write_skill(&kb_root.join("skills"), "tea-brewing", "tea-brewing", "緑茶の淹れ方", "本文");

    let skills = discover_skills(Some(&kb_root), &tmp.path().join("home"));

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "tea-brewing");
    assert_eq!(skills[0].source, SkillSource::Kb);
  }

  #[test]
  fn discover_skills_finds_user_only_skills() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    write_skill(&home.join(".agents/skills"), "coffee-brewing", "coffee-brewing", "コーヒーの淹れ方", "本文");

    let skills = discover_skills(Some(&tmp.path().join("kb")), &home);

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "coffee-brewing");
    assert_eq!(skills[0].source, SkillSource::User);
  }

  #[test]
  fn discover_skills_returns_user_skills_when_no_active_kb() {
    // アクティブ KB が無い（設定ダイアログを KB 未選択で開いた等）場合でも panic せず、
    // user 側だけ返す。
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    write_skill(&home.join(".agents/skills"), "coffee-brewing", "coffee-brewing", "コーヒーの淹れ方", "本文");

    let skills = discover_skills(None, &home);

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "coffee-brewing");
    assert_eq!(skills[0].source, SkillSource::User);
  }

  #[test]
  fn discover_skills_merges_both_scopes_and_kb_wins_on_same_name() {
    let tmp = tempfile::tempdir().unwrap();
    let kb_root = tmp.path().join("kb");
    let home = tmp.path().join("home");
    write_skill(&home.join(".agents/skills"), "shared", "shared", "user 側の説明", "user 側の本文");
    write_skill(&kb_root.join("skills"), "shared", "shared", "kb 側の説明", "kb 側の本文");
    write_skill(&home.join(".agents/skills"), "user-only", "user-only", "user 専用", "本文");

    let skills = discover_skills(Some(&kb_root), &home);

    // 決定的順序（name 昇順）: shared, user-only。
    assert_eq!(skills.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), vec!["shared", "user-only"]);
    let shared = skills.iter().find(|s| s.name == "shared").unwrap();
    assert_eq!(shared.source, SkillSource::Kb);
    assert_eq!(shared.body, "kb 側の本文");
  }

  #[test]
  fn discover_skills_skips_malformed_skill_but_keeps_others() {
    let tmp = tempfile::tempdir().unwrap();
    let kb_root = tmp.path().join("kb");
    write_skill(&kb_root.join("skills"), "good", "good", "正常な技能", "本文");
    // description 欠落＝スキップ対象。
    let broken_dir = kb_root.join("skills").join("broken");
    std::fs::create_dir_all(&broken_dir).unwrap();
    std::fs::write(broken_dir.join("SKILL.md"), "---\nname: broken\n---\n\n本文").unwrap();
    // SKILL.md が無いサブディレクトリはそもそもスキルではない（無視、ログも出さない）。
    std::fs::create_dir_all(kb_root.join("skills").join("not-a-skill")).unwrap();

    let skills = discover_skills(Some(&kb_root), &tmp.path().join("home"));

    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "good");
  }

  #[test]
  fn discover_skills_reports_has_scripts_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let kb_root = tmp.path().join("kb");
    write_skill(&kb_root.join("skills"), "with-scripts", "with-scripts", "説明", "本文");
    std::fs::create_dir_all(kb_root.join("skills/with-scripts/scripts")).unwrap();
    write_skill(&kb_root.join("skills"), "without-scripts", "without-scripts", "説明", "本文");

    let skills = discover_skills(Some(&kb_root), &tmp.path().join("home"));

    let with = skills.iter().find(|s| s.name == "with-scripts").unwrap();
    let without = skills.iter().find(|s| s.name == "without-scripts").unwrap();
    assert!(with.has_scripts);
    assert!(!without.has_scripts);
  }
}
