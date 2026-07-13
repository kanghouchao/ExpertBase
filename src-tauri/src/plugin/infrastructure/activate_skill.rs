//! plugin インフラ: モデル自主発動用の `activate_skill` ツール（要求5の補助経路）。
//! `name` 引数は発見済み技能名の enum で制約する（幻覚防止）。単発生成内（同一 `chat()` 実行内、
//! `multi_turn` の中で複数回呼ばれる場合）の重複排除は `activated_this_call` が担う
//! （`tools/mod.rs` の `used_sources` と同型のパターン）。会話をまたぐ重複排除は行わない
//! （フロントの `activatedSkillNames` に一本化する、設計の確定した決定10）。

use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::Deserialize;
use serde_json::json;

use crate::plugin::Skill;

#[derive(Deserialize)]
pub(crate) struct ActivateSkillArgs {
  #[serde(default)]
  name: String,
}

pub(crate) struct ActivateSkill {
  pub skills: Vec<Skill>,
  pub activated_this_call: Arc<Mutex<Vec<String>>>,
}

impl Tool for ActivateSkill {
  const NAME: &'static str = "activate_skill";
  type Error = Infallible;
  type Args = ActivateSkillArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    let names: Vec<String> = self.skills.iter().map(|s| s.name.clone()).collect();
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Activate a discovered skill by name and receive its full instructions. Call this when the user's request matches a skill listed in the # Skills catalog."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "description": "Skill name from the # Skills catalog",
            "enum": names
          }
        },
        "required": ["name"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let name = args.name.trim();
    if name.is_empty() {
      return Ok("(activate_skill needs a non-empty name)".to_string());
    }
    let Some(skill) = self.skills.iter().find(|s| s.name == name) else {
      return Ok(format!("(no skill found: {name})"));
    };
    let mut activated = self.activated_this_call.lock().unwrap_or_else(|e| e.into_inner());
    if activated.iter().any(|n| n == name) {
      return Ok(format!("(skill already activated this turn: {name})"));
    }
    activated.push(name.to_string());
    Ok(skill.body.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plugin::SkillSource;

  fn skill(name: &str, body: &str) -> Skill {
    Skill {
      name: name.to_string(),
      description: format!("{name} の説明"),
      body: body.to_string(),
      location: format!("/skills/{name}/SKILL.md"),
      source: SkillSource::Kb,
      has_scripts: false,
    }
  }

  fn tool(skills: Vec<Skill>) -> ActivateSkill {
    ActivateSkill { skills, activated_this_call: Arc::new(Mutex::new(Vec::new())) }
  }

  #[tokio::test]
  async fn call_returns_body_of_existing_skill() {
    let t = tool(vec![skill("tea-brewing", "緑茶の淹れ方本文")]);
    let out = t.call(ActivateSkillArgs { name: "tea-brewing".into() }).await.unwrap();
    assert_eq!(out, "緑茶の淹れ方本文");
  }

  #[tokio::test]
  async fn call_reports_unknown_skill_name() {
    let t = tool(vec![skill("tea-brewing", "本文")]);
    let out = t.call(ActivateSkillArgs { name: "coffee-brewing".into() }).await.unwrap();
    assert!(out.contains("no skill found: coffee-brewing"), "was: {out}");
  }

  #[tokio::test]
  async fn call_reports_empty_name() {
    let t = tool(vec![skill("tea-brewing", "本文")]);
    let out = t.call(ActivateSkillArgs { name: "  ".into() }).await.unwrap();
    assert!(out.contains("non-empty"), "was: {out}");
  }

  #[tokio::test]
  async fn call_deduplicates_repeated_activation_within_same_instance() {
    let t = tool(vec![skill("tea-brewing", "本文")]);
    let first = t.call(ActivateSkillArgs { name: "tea-brewing".into() }).await.unwrap();
    let second = t.call(ActivateSkillArgs { name: "tea-brewing".into() }).await.unwrap();
    assert_eq!(first, "本文");
    assert!(second.contains("already activated this turn"), "was: {second}");
  }

  #[tokio::test]
  async fn definition_enum_constrains_name_to_discovered_skill_names() {
    let t = tool(vec![skill("tea-brewing", "本文A"), skill("coffee-brewing", "本文B")]);
    let def = t.definition(String::new()).await;
    let names = def.parameters["properties"]["name"]["enum"].as_array().unwrap();
    let names: Vec<&str> = names.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names, vec!["tea-brewing", "coffee-brewing"]);
  }
}
