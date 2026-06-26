//! AI エージェントの「指示」層。プロバイダ非依存（Ollama にも将来の API にも共通）。
//! ＝ AI の振る舞いを決める system プロンプト。ツールの wire 定義は infra（Rig の Tool 実装）が
//! 持つので、ここはプロンプト本文（役割 + ツールの説明 + 言語方針 + 素材の扱い）だけを担う。

/// エージェント経路の system プロンプト（英語）。知識ベース管理アシスタントとして、必要なときに
/// 適切なツールを呼び、素材の加工・整理・文書作成・KB 管理を手伝う。KB は Markdown + 双方向リンク。
/// 言語方針: ユーザーの直近メッセージと同じ言語で返す。書き込みは「ユーザーが頼んだときだけ」。
pub const AGENT_SYSTEM: &str = r###"You are a knowledge base management assistant. You help the user process and organize source material, draft documents, and manage their knowledge base, calling the appropriate tools when needed. The knowledge base is written in Markdown, with [[title]] wikilinks connecting related notes.

Always reply in the same language as the user's latest message.

Tools:
- search_kb(query): Search existing entries by keyword; returns matching titles and excerpts. Use it to find related notes and avoid duplicates.
- write_entry(title, cat, body): Save a new entry into the knowledge base. Call only when the user asks to save or store the content.
  - title: a concise heading.
  - cat: a short lowercase English category, e.g. tea, finance, privacy.
  - body: the entry body in Markdown, using [[title]] links to related notes."###;

/// AGENT_SYSTEM に素材を pin する（会話全体で固定の文脈）。役割 + ツールの後に素材を置き、
/// その後に会話（ユーザーの指示）が続く＝役割 → ツール → 素材 → ユーザー入力の順。素材は
/// 「参考資料」と性格づけ、概括・改写を勝手にしないよう一文だけ添える（ユーザーの指示が任務）。
pub fn agent_system_with(source_text: &str) -> String {
  format!(
    "{AGENT_SYSTEM}\n\n# Sources\nThe following are reference materials for this conversation. Follow the user's instruction; do not summarize or rewrite them unless asked.\n{source_text}"
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_system_pins_material_and_mentions_tools() {
    let s = agent_system_with("緑茶の淹れ方");
    assert!(s.contains("緑茶の淹れ方"));
    assert!(s.contains("search_kb"));
    assert!(s.contains("write_entry"));
    // 素材は「# Sources」セクションとして pin される。
    assert!(s.contains("# Sources"));
  }

  #[test]
  fn agent_system_gates_write_on_user_request() {
    // 書き込みは「ユーザーが頼んだときだけ」をプロンプトで縛る（最小限の門控は残す）。
    assert!(AGENT_SYSTEM.contains("Call only when the user asks to save"));
  }
}
