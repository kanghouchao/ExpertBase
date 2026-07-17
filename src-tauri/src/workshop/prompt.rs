//! 工作坊エージェントの「指示」層＝system プロンプト。KB とツールを語る業務固有の内容なので、
//! 汎用 agent には置かず workshop が持つ。ツールの契約文（説明 + 引数）は infra の各ツールの
//! `definition()` が唯一の真源で、# Tools 節はそこから生成された本文を受け取って収めるだけ
//! （ここに個別ツールの説明を書くと二重管理・分岐が再発する）。
//! ここは役割 + 言語方針 + 素材の扱いだけを担う。

/// エージェント経路の system プロンプト前文（英語）。知識ベース管理アシスタントとして、必要な
/// ときに適切なツールを呼び、素材の加工・整理・文書作成・KB 管理を手伝う。KB は Markdown +
/// 双方向リンク。言語方針: ユーザーの直近メッセージと同じ言語で返す。
pub const AGENT_SYSTEM: &str = r###"You are a knowledge base management assistant. You help the user process and organize source material, draft documents, and manage their knowledge base, calling the appropriate tools when needed. The knowledge base is written in Markdown, with [[title]] wikilinks connecting related notes.

Always reply in the same language as the user's latest message.

When you need a tool, call it directly. Never announce or describe a tool call in prose — do not write things like "I will call read_source" or "(I will read the file)". Just call the tool. After it returns, continue based on the result."###;

/// AGENT_SYSTEM に # Tools（toolset の definition() から生成された契約文）と # Sources（素材 id
/// の目録）を付けて system プロンプトを組む。素材は本文を注入せず、id だけ並べて
/// `read_source(id)` で AI 自身に読ませる＝「AI が読んだ内容」と「我々のプロンプト」を構造的に分離。
/// 素材が無ければ `# Sources` 節ごと、契約文が空（tools 非対応モデル＝空 toolset）なら
/// `# Tools` 節ごと省略する。
///
/// `skills_catalog`（`# Skills`）と `activated_skills_section`（`# Activated Skills`）は
/// 互いに独立な節（plugin::render_catalog / render_activated が内容の唯一の真源、ここは
/// 「どこに置くか」だけを持つ）。空文字列ならそれぞれ節ごと省略する。catalog は tools 能力が
/// 無いモデルには渡さない（呼び出し側が空文字列を渡す）が、activated は tools 能力に関わらず
/// 常に評価する（明示発動は tools 能力に依存しない、要求4）。
pub fn agent_system_with(
  tools_section: &str,
  source_ids: &[String],
  skills_catalog: &str,
  activated_skills_section: &str,
) -> String {
  let mut s = AGENT_SYSTEM.to_string();
  if !tools_section.is_empty() {
    s.push_str(&format!("\n\n# Tools\n{tools_section}"));
  }
  if !source_ids.is_empty() {
    let list = source_ids.iter().map(|id| format!("- {id}")).collect::<Vec<_>>().join("\n");
    s.push_str(&format!(
      "\n\n# Sources\nThe following source materials are attached. Call read_source(id) to read one before working on it.\n{list}"
    ));
  }
  if !skills_catalog.is_empty() {
    s.push_str(&format!("\n\n# Skills\n{skills_catalog}"));
  }
  if !activated_skills_section.is_empty() {
    s.push_str(&format!("\n\n# Activated Skills\n{activated_skills_section}"));
  }
  s
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_system_embeds_tools_section_and_lists_source_ids() {
    let s = agent_system_with(
      "- read_source(id): Read a source.",
      &["inbox/a.md".into(), "/abs/b.pdf".into()],
      "",
      "",
    );
    // # Tools は infra が definition() から生成した本文をそのまま節に収める。
    assert!(s.contains("# Tools\n- read_source(id): Read a source."), "was: {s}");
    // 本文ではなく id の目録を並べ、read_source で読ませる。
    assert!(s.contains("# Sources"));
    assert!(s.contains("source materials are attached"));
    assert!(s.contains("inbox/a.md"));
    assert!(s.contains("/abs/b.pdf"));
  }

  #[test]
  fn agent_system_omits_sources_section_when_empty() {
    // 素材なしなら # Sources 節ごと省略する。
    let s = agent_system_with("- x(): y", &[], "", "");
    assert!(s.contains("# Tools\n- x(): y"), "was: {s}");
    assert!(!s.contains("# Sources"));
    assert!(!s.contains("source materials are attached"));
  }

  #[test]
  fn agent_system_omits_tools_section_when_empty() {
    // tools 非対応モデル(空 toolset)では # Tools 節ごと省略する＝呼べないツールを語らない。
    let s = agent_system_with("", &[], "", "");
    assert!(!s.contains("# Tools"));
  }

  #[test]
  fn agent_system_includes_skills_catalog_when_non_empty() {
    let s = agent_system_with("- x(): y", &[], "- tea-brewing: 緑茶の淹れ方", "");
    assert!(s.contains("# Skills\n- tea-brewing: 緑茶の淹れ方"), "was: {s}");
    assert!(!s.contains("# Activated Skills"));
  }

  #[test]
  fn agent_system_omits_skills_catalog_when_empty() {
    let s = agent_system_with("- x(): y", &[], "", "");
    assert!(!s.contains("# Skills"));
  }

  #[test]
  fn agent_system_includes_activated_skills_section_when_non_empty() {
    let s = agent_system_with("- x(): y", &[], "", "## tea-brewing\n本文");
    assert!(s.contains("# Activated Skills\n## tea-brewing\n本文"), "was: {s}");
    assert!(!s.contains("# Skills\n"));
  }

  #[test]
  fn agent_system_omits_activated_skills_section_when_empty() {
    let s = agent_system_with("- x(): y", &[], "", "");
    assert!(!s.contains("# Activated Skills"));
  }

  #[test]
  fn agent_system_shows_skills_catalog_and_activated_skills_independently_when_both_present() {
    // 要求3(catalog)と要求4(明示発動)は独立の節。両方同時に出うる。
    let s = agent_system_with(
      "- x(): y",
      &[],
      "- tea-brewing: 緑茶の淹れ方",
      "## coffee-brewing\nコーヒーの淹れ方本文",
    );
    assert!(s.contains("# Skills\n- tea-brewing: 緑茶の淹れ方"), "was: {s}");
    assert!(s.contains("# Activated Skills\n## coffee-brewing\nコーヒーの淹れ方本文"), "was: {s}");
  }

  #[test]
  fn agent_system_does_not_hardcode_tool_descriptions() {
    // ツール契約は definition() が唯一の真源＝前文に個別ツールの説明を再び書くと分岐が再発する。
    // （"read_source" の語自体はツール呼び出しの禁止例と # Sources 案内に残る）
    assert!(!AGENT_SYSTEM.contains("Tools:"));
    assert!(!AGENT_SYSTEM.contains("list_kb"));
    assert!(!AGENT_SYSTEM.contains("write_entry"));
  }

  #[test]
  fn agent_system_forbids_narrating_tool_calls() {
    // ツール呼び出しを散文で予告せず直接呼ぶよう縛る（narration だけして呼ばない退行を防ぐ）。
    assert!(AGENT_SYSTEM.contains("call it directly"));
    assert!(AGENT_SYSTEM.contains("Never announce or describe a tool call"));
  }
}
