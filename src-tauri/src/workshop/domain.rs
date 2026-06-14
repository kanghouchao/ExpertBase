//! workshop ドメイン層。RAG 検索の候補語抽出など外部依存のない純ロジック。

/// 素材本文から検索候補語を取り出す（空白・句読点で分割、3 文字以上）。
/// trigram FTS の制約上 3 文字未満は使えない。空白の無い CJK は recall が限られる（MVP の既知の制限）。
pub(crate) fn candidate_terms(source: &str, max: usize) -> Vec<String> {
  let mut seen = std::collections::HashSet::new();
  let mut terms = Vec::new();
  for raw in source.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation()) {
    let t = raw.trim();
    if t.chars().count() >= 3 && seen.insert(t.to_string()) {
      terms.push(t.to_string());
      if terms.len() >= max {
        break;
      }
    }
  }
  terms
}
