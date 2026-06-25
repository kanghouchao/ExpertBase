//! workshop アプリケーション層。RAG 編成と条目確定のユースケース。
//! kb（検索・索引・FS）と ai（プロバイダ）を編成する。AI は ai の trait の裏でのみ呼ぶ。

use std::path::Path;

use rusqlite::Connection;
use serde_json::Value;

use crate::ai::agent::{agent_system_with, agent_tools};
use crate::ai::{
  AgentMsg, AiError, AiProvider, ChatTurn, EntrySummary, StreamProgress, StructureRequest,
  StructureResult,
};
use crate::kb::entry::{Entry, EntryMeta};
use crate::kb::{index, store};
use crate::workshop::domain::candidate_terms;

/// エージェントの暴走（無限ツール呼び出し）を抑える反復上限。超えたら今ある本文で整形へ進む。
const MAX_AGENT_ITERS: usize = 6;

/// 新素材に関連する既存条目を FTS で引く（RAG の検索段）。上位 n 件の title + excerpt。
pub fn related_entries(
  conn: &Connection,
  source: &str,
  n: usize,
) -> Result<Vec<EntrySummary>, String> {
  let mut seen_paths = std::collections::HashSet::new();
  let mut out = Vec::new();
  for term in candidate_terms(source, 12) {
    for hit in index::search(conn, &term)? {
      if seen_paths.insert(hit.path.clone()) {
        out.push(EntrySummary { title: hit.title, excerpt: hit.excerpt });
        if out.len() >= n {
          return Ok(out);
        }
      }
    }
  }
  Ok(out)
}

/// RAG 編成: 関連条目を引き、会話履歴つきで AiProvider に構造化（草稿 or 会話返信）を依頼する。
pub fn draft<P: AiProvider>(
  provider: &P,
  conn: &Connection,
  source_text: &str,
  messages: Vec<ChatTurn>,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<StructureResult, AiError> {
  on_progress(StreamProgress::Retrieving);
  let related = related_entries(conn, source_text, 5).map_err(AiError::Other)?;
  provider.structure(
    StructureRequest { source_text: source_text.to_string(), related, messages },
    on_progress,
  )
}

/// エージェント経路（tools 対応モデル）。検索ツールで既存条目を調べながら散文ドラフトを書かせ、
/// 最後に structure_draft で StructureResult へ整形する。読み取りツール（search_kb）はループ内で
/// 即実行・逐次上報する。書き込み系ツールは公開しない＝確定（書き込み）は既存 confirm に残る。
pub fn agent_draft<P: AiProvider>(
  provider: &P,
  conn: &Connection,
  source_text: &str,
  messages: Vec<ChatTurn>,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<StructureResult, AiError> {
  let system = agent_system_with(source_text);
  let tools = agent_tools();
  let mut convo: Vec<AgentMsg> = messages
    .into_iter()
    .map(|t| match t.role.as_str() {
      "user" => AgentMsg::User(t.content),
      _ => AgentMsg::Assistant { content: t.content, tool_calls: vec![] },
    })
    .collect();
  let mut draft = String::new();
  for _ in 0..MAX_AGENT_ITERS {
    let outcome = provider.agent_turn(&system, &tools, &convo, on_progress)?;
    draft = outcome.content.clone();
    if outcome.tool_calls.is_empty() {
      break;
    }
    // モデルの応答（ツール呼び出し含む）を履歴へ積み、各ツールを実行して結果を返す。
    convo.push(AgentMsg::Assistant {
      content: outcome.content,
      tool_calls: outcome.tool_calls.clone(),
    });
    for tc in &outcome.tool_calls {
      on_progress(StreamProgress::ToolCall { name: tc.name.clone(), args: tc.args.to_string() });
      let (result, summary) = exec_kb_tool(conn, &tc.name, &tc.args);
      on_progress(StreamProgress::ToolResult { name: tc.name.clone(), summary });
      convo.push(AgentMsg::Tool { name: tc.name.clone(), content: result });
    }
  }
  provider.structure_draft(&draft, on_progress)
}

/// 読み取り専用 KB ツールを実行する。戻り値は (モデルへ返す結果, 表示用サマリ)。
/// v1 は search_kb のみ（FTS 検索）。未知ツールは安全に「unknown」を返す（信頼境界の堅牢化）。
fn exec_kb_tool(conn: &Connection, name: &str, args: &Value) -> (String, String) {
  match name {
    "search_kb" => {
      let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("").trim();
      if query.is_empty() {
        return ("(empty query)".into(), "empty query".into());
      }
      match index::search(conn, query) {
        Ok(hits) if !hits.is_empty() => {
          let shown = hits.len().min(5);
          let body = hits
            .iter()
            .take(shown)
            .map(|h| format!("- {}: {}", h.title, h.excerpt))
            .collect::<Vec<_>>()
            .join("\n");
          (body, format!("{shown} results"))
        }
        Ok(_) => ("(no matching entries)".into(), "no matches".into()),
        Err(e) => (format!("(search error: {e})"), "error".into()),
      }
    }
    other => (format!("(unknown tool: {other})"), "unknown tool".into()),
  }
}

/// 承認された内容を `entries/` に確定し、インデックス更新 + source の受信箱を全て processed にする。
/// AI 草稿でも手動入力でも、単一・複数素材でも同じ経路を通る。
pub fn confirm(
  root: &Path,
  conn: &Connection,
  title: &str,
  cat: &str,
  body: &str,
  inbox_rels: &[String],
) -> Result<String, String> {
  let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
  let entry = Entry {
    meta: EntryMeta {
      kind: "Entry".into(),
      title: title.to_string(),
      description: String::new(),
      cat: cat.to_string(),
      tags: vec![],
      sources: inbox_rels.to_vec(),
      created: today.clone(),
      updated: today,
    },
    body: body.to_string(),
  };
  let rel = store::write_entry(root, &entry)?;
  index::upsert_entry(conn, &rel, &entry)?;
  for inbox_rel in inbox_rels {
    index::set_inbox_status(conn, inbox_rel, "processed")?;
  }
  Ok(rel)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ai::{FakeProvider, ToolCall, ToolDef, TurnOutcome};
  use crate::kb::entry::EntryMeta;

  /// 脚本化エージェント: 1 ターン目に search_kb を呼び、2 ターン目に最終ドラフトを返す。
  /// ループ（ツール実行 → 履歴追加 → 再呼び出し → 整形）の検証用。
  struct ScriptedAgent {
    turn: std::cell::Cell<u32>,
  }

  impl AiProvider for ScriptedAgent {
    fn structure(
      &self,
      _req: StructureRequest,
      _on: &mut dyn FnMut(StreamProgress),
    ) -> Result<StructureResult, AiError> {
      unreachable!("agent path uses agent_turn, not structure")
    }

    fn agent_turn(
      &self,
      _system: &str,
      _tools: &[ToolDef],
      _messages: &[AgentMsg],
      on: &mut dyn FnMut(StreamProgress),
    ) -> Result<TurnOutcome, AiError> {
      on(StreamProgress::LoadingModel);
      let n = self.turn.get();
      self.turn.set(n + 1);
      if n == 0 {
        Ok(TurnOutcome {
          content: "let me search".into(),
          tool_calls: vec![ToolCall {
            name: "search_kb".into(),
            args: serde_json::json!({ "query": "淹れ方" }),
          }],
        })
      } else {
        Ok(TurnOutcome { content: "# お茶\n本文だよ".into(), tool_calls: vec![] })
      }
    }

    fn structure_draft(
      &self,
      draft: &str,
      on: &mut dyn FnMut(StreamProgress),
    ) -> Result<StructureResult, AiError> {
      on(StreamProgress::Structuring { chars: draft.chars().count() });
      Ok(StructureResult {
        kind: "entry".into(),
        title: "お茶".into(),
        cat: "tea".into(),
        body_markdown: draft.into(),
        suggested_links: vec![],
      })
    }
  }

  fn seed_entry(conn: &Connection, path: &str, title: &str, body: &str) {
    let entry = Entry {
      meta: EntryMeta {
        kind: "Entry".into(),
        title: title.into(),
        description: String::new(),
        cat: "x".into(),
        tags: vec![],
        sources: vec![],
        created: "2026-06-14".into(),
        updated: "2026-06-14".into(),
      },
      body: body.into(),
    };
    index::upsert_entry(conn, path, &entry).unwrap();
  }

  #[test]
  fn confirm_records_inbox_rels_as_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = crate::kb::index::open_index(root).unwrap();
    std::fs::create_dir_all(root.join("inbox")).unwrap();

    let rel = confirm(root, &conn, "緑茶", "tea", "本文", &["inbox/a.md".into()]).unwrap();

    let saved = std::fs::read_to_string(root.join(&rel)).unwrap();
    let entry = crate::kb::entry::parse_entry(&saved).unwrap();
    assert_eq!(entry.meta.sources, vec!["inbox/a.md".to_string()]);
  }

  #[test]
  fn related_entries_finds_keyword_matches() {
    let conn = Connection::open_in_memory().unwrap();
    index::ensure_schema(&conn).unwrap();
    // 検索語は 3 文字以上が必要（trigram）。「淹れ方」(3 文字) で一致させる。
    seed_entry(&conn, "entries/green.md", "緑茶の淹れ方", "湯温は70度。淹れ方の基本。");
    let related = related_entries(&conn, "新しい 淹れ方 のメモ", 5).unwrap();
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].title, "緑茶の淹れ方");
  }

  #[test]
  fn draft_with_fake_provider_returns_result() {
    let conn = Connection::open_in_memory().unwrap();
    index::ensure_schema(&conn).unwrap();
    seed_entry(&conn, "entries/green.md", "緑茶の淹れ方", "湯温は70度。淹れ方の基本。");
    let messages = vec![ChatTurn { role: "user".into(), content: "整理して".into() }];
    let result = draft(&FakeProvider, &conn, "新しい 淹れ方 の本文", messages, &mut |_| {}).unwrap();
    assert_eq!(result.kind, "entry");
    assert_eq!(result.title, "新しい 淹れ方 の本文");
    // FakeProvider は関連条目のタイトルをリンク候補にする。
    assert_eq!(result.suggested_links, vec!["緑茶の淹れ方".to_string()]);
  }

  #[test]
  fn draft_forwards_streaming_progress() {
    let conn = Connection::open_in_memory().unwrap();
    index::ensure_schema(&conn).unwrap();
    let mut events = Vec::new();
    draft(&FakeProvider, &conn, "本文", vec![], &mut |p| events.push(p)).unwrap();
    // 検索 → モデルロード の順。確定的な検索段が先に上報される。
    assert_eq!(events.first(), Some(&StreamProgress::Retrieving));
    assert!(events.contains(&StreamProgress::LoadingModel));
  }

  #[test]
  fn agent_draft_runs_search_tool_then_structures_draft() {
    let conn = Connection::open_in_memory().unwrap();
    index::ensure_schema(&conn).unwrap();
    seed_entry(&conn, "entries/green.md", "緑茶の淹れ方", "湯温は70度。淹れ方の基本。");
    let provider = ScriptedAgent { turn: std::cell::Cell::new(0) };
    let messages = vec![ChatTurn { role: "user".into(), content: "整理して".into() }];
    let mut events = Vec::new();
    let result =
      agent_draft(&provider, &conn, "新しい本文", messages, &mut |p| events.push(p)).unwrap();
    // ループは読み取りツール search_kb を実行し、ToolCall / ToolResult を逐次上報した。
    assert!(events
      .iter()
      .any(|e| matches!(e, StreamProgress::ToolCall { name, .. } if name == "search_kb")));
    assert!(events.iter().any(|e| matches!(e, StreamProgress::ToolResult { .. })));
    // 2 ターン目の最終ドラフトを structure_draft で整形して返した。
    assert_eq!(result.kind, "entry");
    assert!(result.body_markdown.contains("本文"));
  }

  #[test]
  fn confirm_writes_one_entry_and_marks_all_source_inboxes_processed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let conn = index::open_index(root).unwrap();
    index::upsert_inbox(&conn, "inbox/m.md", "text", "paste", "pending", "2026-06-14T00:00:00Z")
      .unwrap();
    index::upsert_inbox(&conn, "inbox/n.md", "text", "paste", "pending", "2026-06-14T00:00:00Z")
      .unwrap();

    // 複数素材を 1 条目に合成する。確定後、source の inbox は全て processed になる。
    let rels = vec!["inbox/m.md".to_string(), "inbox/n.md".to_string()];
    let rel = confirm(root, &conn, "緑茶", "tea", "湯温は [[煎茶]] で70度", &rels).unwrap();
    assert!(root.join(&rel).is_file());
    assert_eq!(index::stats(&conn).unwrap().entries, 1);
    assert_eq!(index::backlinks(&conn, "煎茶").unwrap().len(), 1);
    let inbox = index::list_inbox(&conn).unwrap();
    assert!(inbox.iter().all(|m| m.status == "processed"));
  }
}
