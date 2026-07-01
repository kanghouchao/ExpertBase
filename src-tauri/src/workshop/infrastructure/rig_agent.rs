//! workshop インフラ: 工作坊のツールを構築して汎用 `agent` へ注入する薄いアダプタ。
//! ツールループ/ストリーミング/provider 分岐は `agent::run`（汎用）が持つので、ここは
//! read_source / search_kb / write_entry / fetch_web を組んで注入し委譲するだけ。

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use rig_core::tool::ToolDyn;
use tokio::sync::mpsc::UnboundedSender;

use super::tools::{FetchWeb, ReadSource, SearchKb, WriteEntry};
use crate::agent::{self, AiError, ChatTurn, Provider, StreamProgress};

/// 工作坊のツールを注入して 1 会話分回す。素材は本文を注入せず id の目録だけ system に置き、
/// AI が read_source で自分で読む。書き込みは write_entry ツール経由で「ユーザーが頼んだとき」だけ起きる。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run(
  provider: Provider,
  base_url: Option<&str>,
  model: &str,
  think: bool,
  system: &str,
  root: &Path,
  sources: &[String],
  messages: Vec<ChatTurn>,
  cancel: Arc<AtomicBool>,
  tx: &UnboundedSender<StreamProgress>,
) -> Result<String, AiError> {
  let used_sources = Arc::new(Mutex::new(Vec::new()));
  // 工作坊は tools 対応モデル必須。read_source（素材読み取り）・search_kb・write_entry・fetch_web を常に注入する。
  let tools: Vec<Box<dyn ToolDyn>> = vec![
    Box::new(ReadSource { sources: sources.to_vec(), used_sources: used_sources.clone() }),
    Box::new(SearchKb { root: root.to_path_buf() }),
    Box::new(WriteEntry { root: root.to_path_buf(), used_sources: used_sources.clone() }),
    Box::new(FetchWeb { used_sources }),
  ];
  agent::run(provider, base_url, model, think, system, tools, messages, cancel, tx).await
}
