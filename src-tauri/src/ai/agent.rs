//! AI エージェントの「指示」層。プロバイダ非依存（Ollama にも将来の API にも共通）。
//! ＝ AI の振る舞いを決めるプロンプトと出力スキーマ。ここを transport（ollama.rs 等）から
//! 切り離すことで、別の AI ツール/API を後から差し込んでも指示は使い回せる。
//!
//! ponytail: 現状は KB エディタ・タスク 1 種なので定数 + 関数で足りる。ツール定義を
//! 束ねる AgentSpec 構造体は、ツール呼び出し（Stage 3）が実際に要るまで作らない（YAGNI）。

use serde_json::{json, Value};

use crate::ai::domain::ToolDef;

/// 構造化パス（非思考の単発 / 思考モデルの Pass2）用の system プロンプト。
/// 出力を JSON スキーマで固定する。
pub const SYSTEM_PROMPT: &str = r###"You are a knowledge base editor chatting with the user about the provided Material. Every reply MUST be a single JSON object with the fields below.

Fields:
- kind: "entry" or "chat". Use "entry" when the user asks you to produce or revise a knowledge entry from the Material. Use "chat" when the user is greeting, asking a question, or just talking and has not asked for an entry yet.
- title: a concise heading; empty string when kind is "chat".
- cat: a category as a short lowercase English word, e.g. tea, finance, privacy; empty string when kind is "chat".
- body_markdown: for "entry", the reorganized entry body in valid Markdown; for "chat", your conversational reply.
- suggested_links: for "entry", titles chosen ONLY from the Related existing entries that are genuinely relevant; [] when none are relevant or the list is empty. Always [] for "chat".

Example (chat)
User: Hi, what is this material about?
Output:
{"kind":"chat","title":"","cat":"","body_markdown":"It is an interview with a tea master about pan-firing temperature. Want me to turn it into an entry?","suggested_links":[]}

Example (entry)
User: Organize this into a clean entry.
Material: Oolong tea is semi-oxidized. Steep at 90-95C for about one minute; it can be re-steeped several times.
Related existing entries:
- Green tea brewing: steeping notes for green tea
Output:
{"kind":"entry","title":"Oolong tea brewing","cat":"tea","body_markdown":"## Overview\nOolong is a semi-oxidized tea.\n\n## Brewing\n- Water: 90-95C\n- Time: about 1 minute\n- Can be re-steeped several times","suggested_links":["Green tea brewing"]}

Do not:
- Do not add facts that are not supported by the Material.
- Do not output anything outside the JSON object (no explanations, no code fences).
- Do not link unrelated entries just to fill suggested_links; use [] instead.
- Do not leave broken Markdown such as unmatched ** markers.

Reply with JSON only."###;

/// Pass1(思考パス)用プロンプト。JSON を要求せず、推論しながら散文ドラフトを書かせる。
/// format grammar と think を同時に使うと content が壊れるため、思考時は構造化を
/// Pass2 に分離する。ここは"良いドラフト"を出すことだけに集中させる。
pub const DRAFT_PROMPT: &str = r###"You are a knowledge base editor chatting with the user about the provided Material.
Read the Material and the conversation, think it through, then write the best response in Markdown:
- If the user wants a knowledge entry created or revised, write a clean, well-structured Markdown entry grounded ONLY in the Material.
- Otherwise, reply conversationally.
Do not add facts that are not supported by the Material. Output only the response itself — no JSON, no code fences, no meta commentary."###;

/// エージェント経路（tools 対応モデル）の system プロンプト。検索ツールで既存条目を調べてから
/// 散文ドラフトを書かせる。最終的な JSON 整形は別段（structure_draft）に分離するので、
/// ここでは JSON を要求しない＝「良いドラフト＋適切なリンク調査」に集中させる。
pub const AGENT_SYSTEM: &str = r###"You are a knowledge base editor helping the user turn the provided Material into a clean entry.

You can call tools to research the existing knowledge base before writing:
- search_kb(query): search existing entries by keyword; returns matching titles and excerpts.

Use search_kb to find genuinely related existing entries so you can reference them and avoid duplicating content. Search a few focused queries when useful. When you have enough context, STOP calling tools and write the final response in Markdown:
- If the user wants a knowledge entry created or revised, write a clean, well-structured Markdown entry grounded ONLY in the Material. Mention genuinely related existing entries inline as [[Title]] links.
- Otherwise, reply conversationally.

Do not add facts that are not supported by the Material. Output only the response itself — no JSON, no code fences, no meta commentary."###;

/// エージェントへ公開するツール定義（provider 非依存の中立形）。wire 形式（OpenAI 互換 function
/// ラッパ）への変換は infra（ollama）が行う。v1 は読み取り専用の search_kb のみ。書き込みは
/// 確認門控（既存 confirm）に残すため出さない。
pub fn agent_tools() -> Vec<ToolDef> {
  vec![ToolDef {
    name: "search_kb".into(),
    description:
      "Search existing knowledge base entries by keyword. Returns matching entry titles and excerpts."
        .into(),
    parameters: json!({
      "type": "object",
      "properties": {
        "query": { "type": "string", "description": "Keywords to search for in existing entries" }
      },
      "required": ["query"]
    }),
  }]
}

/// AGENT_SYSTEM に素材を pin する（会話全体で固定の文脈）。
pub fn agent_system_with(source_text: &str) -> String {
  format!("{AGENT_SYSTEM}\n\n# Material\n{source_text}")
}

/// 構造化結果の JSON スキーマ（最終出力の固定形）。プロバイダの format/structured-output に渡す。
pub fn output_schema() -> Value {
  json!({
    "type": "object",
    "properties": {
      "kind": { "type": "string", "enum": ["entry", "chat"] },
      "title": { "type": "string" },
      "cat": { "type": "string" },
      "body_markdown": { "type": "string" },
      "suggested_links": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["kind", "title", "cat", "body_markdown", "suggested_links"],
    "additionalProperties": false
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn system_prompt_defines_kind_examples_and_rules() {
    let p = SYSTEM_PROMPT.to_lowercase();
    // kind（entry/chat）の判別・例示・関連性ルール・禁止事項を含む。
    assert!(p.contains("kind"));
    assert!(p.contains("entry"));
    assert!(p.contains("chat"));
    assert!(p.contains("example"));
    assert!(p.contains("relevant"));
    assert!(p.contains("do not"));
  }

  #[test]
  fn draft_prompt_asks_for_prose_not_json() {
    // 思考パスは散文。JSON を要求しない。
    let p = DRAFT_PROMPT.to_lowercase();
    assert!(p.contains("markdown"));
    assert!(p.contains("no json"));
  }

  #[test]
  fn agent_tools_exposes_readonly_search_kb() {
    let tools = agent_tools();
    // v1 は読み取り専用 search_kb のみ（書き込み系は出さない）。wire ラッパは持たない中立形。
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "search_kb");
    let required = tools[0].parameters["required"].as_array().unwrap();
    assert!(required.iter().any(|f| f == "query"));
  }

  #[test]
  fn agent_system_pins_material_and_mentions_search() {
    let s = agent_system_with("緑茶の淹れ方");
    assert!(s.contains("緑茶の淹れ方"));
    assert!(s.contains("search_kb"));
    assert!(s.to_lowercase().contains("material"));
  }

  #[test]
  fn output_schema_requires_all_fields() {
    let schema = output_schema();
    assert_eq!(schema["type"], "object");
    let required = schema["required"].as_array().unwrap();
    for field in ["kind", "title", "cat", "body_markdown", "suggested_links"] {
      assert!(
        required.iter().any(|f| f == field),
        "missing required: {field}"
      );
    }
  }
}
