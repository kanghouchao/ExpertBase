//! workshop アプリケーション層。対話エージェントの編成と会話履歴のユースケース。
//! kb と汎用 agent を編成する。ツールは infra（tools）で組んで注入し、ループ/推論は agent が
//! 持つ。条目の確定・上書き・削除は kb の条目持久化用例（create_entry 等）へ委譲する。

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use chrono::SecondsFormat;
use tokio::sync::mpsc::UnboundedSender;

use crate::agent::{AiSettings, ChatTurn, Provider, StreamProgress};
use crate::error::AppError;

use super::prompt::agent_system_with;

use super::domain::{WorkshopConversation, WorkshopConversationPage, WorkshopMessage};
use super::infrastructure::{confirm, history, tools};

const HISTORY_PAGE_SIZE: usize = 20;

fn ensure_active_kb(active_root: &Path, expected_kb_path: &str) -> Result<(), AppError> {
  if active_root != Path::new(expected_kb_path) {
    return Err(AppError::code("err.workshop.kbSwitchedDuringSave"));
  }
  Ok(())
}

pub fn save_active_conversation(
  home: &Path,
  expected_kb_path: &str,
  id: Option<i64>,
  source_ids: Vec<String>,
  messages: Vec<WorkshopMessage>,
) -> Result<WorkshopConversation, AppError> {
  let root = crate::kb::active_kb_root(home)?;
  ensure_active_kb(&root, expected_kb_path)?;
  save_conversation(&root, id, source_ids, messages)
}

pub fn save_conversation(
  root: &Path,
  id: Option<i64>,
  source_ids: Vec<String>,
  messages: Vec<WorkshopMessage>,
) -> Result<WorkshopConversation, AppError> {
  let dir = history::open(root)?;
  let now = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
  history::save(&dir, id, &source_ids, &messages, &now)
}

pub fn get_conversation(root: &Path, id: i64) -> Result<WorkshopConversation, AppError> {
  history::get(&history::open(root)?, id)
}

pub fn list_conversations(root: &Path, offset: usize) -> Result<WorkshopConversationPage, AppError> {
  history::list(&history::open(root)?, offset, HISTORY_PAGE_SIZE)
}

/// 対話エージェント経路。素材は本文を注入せず id の目録だけ system に置き、AI が read_source で
/// 自分で読む。Rig で 1 会話分回し、進捗（思考・本文・ツール呼び出し/結果）を tx へ流して、
/// 最終的な助手の返信本文を返す。書き込みは write_entry ツール経由で「ユーザーが保存を頼んだとき」
/// だけ起きる＝確定の主導権はユーザー。素材は全て外部絶対パスで、KB へは複製しない。
/// skills は呼び出し側（interface）が discover_skills 済みの一覧。tools_capable のときだけ
/// catalog を出し `activate_skill` を登録する（要求3）。activated_skill_names に対応する技能の
/// 本文は tools_capable に関わらず常に # Activated Skills へ注入する（要求4、明示発動は
/// tools 能力に依存しない）。
#[allow(clippy::too_many_arguments)]
pub async fn chat(
  settings: AiSettings,
  model: String,
  think: bool,
  tools_capable: bool,
  root: PathBuf,
  sources: Vec<String>,
  skills: Vec<crate::plugin::Skill>,
  activated_skill_names: Vec<String>,
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: UnboundedSender<StreamProgress>,
  pending: confirm::PendingConfirms,
) -> Result<String, AppError> {
  // 破壊的ツール用の確認ゲート。進捗 tx へ確認要求を流し、workshop_confirm の回填を待つ。
  let gate = Arc::new(confirm::ConfirmGate { pending, tx: tx.clone(), cancel: cancel.clone() });
  let provider = settings.provider;
  // provider ごとの生 URL（空欄可）。空欄→provider 既定への解決は agent::run が担う。
  let base_url = match provider {
    Provider::LlamaApp => settings.llama_app_url,
    Provider::Ollama => settings.ollama_url,
  };
  let toolset = tools::build_toolset(
    &root,
    &sources,
    settings.brave_api_key,
    gate,
    &skills,
    tools_capable,
    &activated_skill_names,
  );
  // system の # Tools 節は toolset の definition() から生成する（ツール契約文の唯一の真源）。
  // # Skills / # Activated Skills も同じ「内容は plugin が唯一の真源、ここは配置だけ」パターン。
  let catalog = if tools_capable { crate::plugin::render_catalog(&skills) } else { String::new() };
  let activated_section = crate::plugin::render_activated(&skills, &activated_skill_names);
  let system = agent_system_with(
    &tools::render_tools_section(&toolset).await,
    &sources,
    &catalog,
    &activated_section,
  );
  crate::agent::run(provider, &base_url, &model, think, &system, toolset, messages, cancel, &tx).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::workshop::domain::{WorkshopMessage, WorkshopMessageRole};

  #[test]
  fn conversation_use_cases_round_trip_in_active_root() {
    let tmp = tempfile::tempdir().unwrap();
    let messages = vec![
      WorkshopMessage {
        role: WorkshopMessageRole::User,
        text: "整理这份资料".into(),
        thinking: None,
        tools: None,
      },
      WorkshopMessage {
        role: WorkshopMessageRole::Ai,
        text: "完成".into(),
        thinking: None,
        tools: None,
      },
    ];

    let saved = save_conversation(tmp.path(), None, vec![], messages).unwrap();
    assert_eq!(
      get_conversation(tmp.path(), saved.id).unwrap().title,
      "整理这份资料"
    );
    assert_eq!(
      list_conversations(tmp.path(), 0).unwrap().items[0].id,
      saved.id
    );
  }

  #[test]
  fn saving_rejects_a_conversation_from_a_different_active_kb() {
    let tmp = tempfile::tempdir().unwrap();
    let first_path = tmp.path().join("first");
    let second_path = tmp.path().join("second");

    let error = ensure_active_kb(&second_path, first_path.to_str().unwrap()).unwrap_err();

    assert_eq!(error.code, "err.workshop.kbSwitchedDuringSave");
  }

}
