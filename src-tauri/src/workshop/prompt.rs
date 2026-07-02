//! 工作坊エージェントの「指示」層＝system プロンプト。KB とツール（read_source /
//! search_kb / write_entry / fetch_web）を語る業務固有の内容なので、汎用 agent には置かず
//! workshop が持つ。ツールの wire 定義は infra（Rig の Tool 実装）が持ち、ここは本文
//! （役割 + ツールの説明 + 言語方針 + 素材の扱い）だけを担う。

/// エージェント経路の system プロンプト（英語）。知識ベース管理アシスタントとして、必要なときに
/// 適切なツールを呼び、素材の加工・整理・文書作成・KB 管理を手伝う。KB は Markdown + 双方向リンク。
/// 言語方針: ユーザーの直近メッセージと同じ言語で返す。書き込みは「ユーザーが頼んだときだけ」。
pub const AGENT_SYSTEM: &str = r###"You are a knowledge base management assistant. You help the user process and organize source material, draft documents, and manage their knowledge base, calling the appropriate tools when needed. The knowledge base is written in Markdown, with [[title]] wikilinks connecting related notes.

Always reply in the same language as the user's latest message.

When you need a tool, call it directly. Never announce or describe a tool call in prose — do not write things like "I will call read_source" or "(I will read the file)". Just call the tool. After it returns, continue based on the result.

Tools:
- read_source(id): Read the full text of an attached source by its id (see the # Sources list). Read a source before translating, rewriting, summarizing, or answering questions about it. Do not summarize or rewrite a source unless the user asks.
- search_kb(query): Search existing entries by keyword; returns matching titles and excerpts. Use it to find related notes and avoid duplicates.
- fetch_web(url): Fetch a web page the user gave you and return its main text as Markdown. Use it when the user shares a URL to read, summarize, or save.
- write_entry(title, cat, body): Save a new entry into the knowledge base. Call only when the user asks to save or store the content. The user is asked to approve the save before it happens; if they deny it, do not retry unless asked.
  - title: a concise heading.
  - cat: a short lowercase English category, e.g. tea, finance, privacy.
  - body: the entry body in Markdown, using [[title]] links to related notes."###;

/// AGENT_SYSTEM に素材の目録（id 一覧）を付ける。本文は注入せず、id だけ並べて
/// `read_source(id)` で AI 自身に読ませる＝「AI が読んだ内容」と「我々のプロンプト」を構造的に分離。
/// 素材が無ければ `# Sources` 節ごと省略する。
pub fn agent_system_with(source_ids: &[String]) -> String {
  if source_ids.is_empty() {
    return AGENT_SYSTEM.to_string();
  }
  let list = source_ids.iter().map(|id| format!("- {id}")).collect::<Vec<_>>().join("\n");
  format!(
    "{AGENT_SYSTEM}\n\n# Sources\nThe following source materials are attached. Call read_source(id) to read one before working on it.\n{list}"
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_system_lists_source_ids_and_mentions_tools() {
    let s = agent_system_with(&["inbox/a.md".into(), "/abs/b.pdf".into()]);
    // 本文ではなく id の目録を並べ、read_source で読ませる。
    assert!(s.contains("source materials are attached"));
    assert!(s.contains("inbox/a.md"));
    assert!(s.contains("/abs/b.pdf"));
    assert!(s.contains("read_source"));
    assert!(s.contains("search_kb"));
    assert!(s.contains("write_entry"));
    assert!(s.contains("fetch_web"));
  }

  #[test]
  fn agent_system_omits_sources_section_when_empty() {
    // 素材なしなら節を足さない＝preamble そのもの（read_source 説明文の「# Sources list」言及は残る）。
    let s = agent_system_with(&[]);
    assert_eq!(s, AGENT_SYSTEM);
    assert!(!s.contains("source materials are attached"));
  }

  #[test]
  fn agent_system_gates_write_on_user_request() {
    // 書き込みは「ユーザーが頼んだときだけ」をプロンプトで縛る（最小限の門控は残す）。
    assert!(AGENT_SYSTEM.contains("Call only when the user asks to save"));
  }

  #[test]
  fn agent_system_forbids_narrating_tool_calls() {
    // ツール呼び出しを散文で予告せず直接呼ぶよう縛る（narration だけして呼ばない退行を防ぐ）。
    assert!(AGENT_SYSTEM.contains("call it directly"));
    assert!(AGENT_SYSTEM.contains("Never announce or describe a tool call"));
  }
}
