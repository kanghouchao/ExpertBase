use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopToolEvent {
  pub name: String,
  pub args: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopMessage {
  pub role: WorkshopMessageRole,
  pub text: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub thinking: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tools: Option<Vec<WorkshopToolEvent>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkshopMessageRole {
  User,
  Ai,
}

impl WorkshopMessage {
  #[cfg(test)]
  pub(crate) fn user(text: &str) -> Self {
    Self {
      role: WorkshopMessageRole::User,
      text: text.into(),
      thinking: None,
      tools: None,
    }
  }

  #[cfg(test)]
  pub(crate) fn assistant(text: &str) -> Self {
    Self {
      role: WorkshopMessageRole::Ai,
      text: text.into(),
      thinking: None,
      tools: None,
    }
  }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopConversationSummary {
  pub id: i64,
  pub title: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopConversationPage {
  pub items: Vec<WorkshopConversationSummary>,
  pub has_more: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopConversation {
  pub id: i64,
  pub title: String,
  pub source_ids: Vec<String>,
  pub messages: Vec<WorkshopMessage>,
  pub created_at: String,
  pub updated_at: String,
}

pub fn conversation_title(messages: &[WorkshopMessage]) -> String {
  messages
    .iter()
    .find(|message| message.role == WorkshopMessageRole::User)
    .map(|message| message.text.split_whitespace().collect::<Vec<_>>().join(" "))
    .filter(|title| !title.is_empty())
    .map(|title| title.chars().take(40).collect())
    .unwrap_or_else(|| "New conversation".into())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn title_uses_first_user_message_and_truncates_by_characters() {
    let messages = vec![
      WorkshopMessage::assistant("先行応答"),
      WorkshopMessage::user("  这是   一段\n包含空白的标题abcdefghijklmnopqrstuvwxyz12  "),
    ];

    assert_eq!(conversation_title(&messages).chars().count(), 40);
    assert!(conversation_title(&messages).starts_with("这是 一段 包含空白的标题"));
  }
}
