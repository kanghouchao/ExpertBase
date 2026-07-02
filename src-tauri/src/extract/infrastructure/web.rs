use dom_smoothie::Readability;

/// 出站 HTTP の自己識別 UA（fetch_web / search_web で共用）。
pub const APP_USER_AGENT: &str = "ExpertBase/0.1 (+local capture)";

/// URL から HTML を取得する。取得は HTTPS、原始 HTML はクラウドへ送らない（抽出はローカル）。
pub async fn fetch_html(url: &str) -> Result<String, String> {
  reqwest::Client::new()
    .get(url)
    .header("User-Agent", APP_USER_AGENT)
    .send()
    .await
    .map_err(|e| e.to_string())?
    .text()
    .await
    .map_err(|e| e.to_string())
}

/// HTML から本文を抽出し、(タイトル, Markdown 本文) を返す。
/// Readability（dom_smoothie）でナビ/フッタ等のノイズを除き、htmd で HTML→Markdown へ変換する。
pub fn extract_readable(html: &str, url: &str) -> Result<(String, String), String> {
  let mut readability =
    Readability::new(html, Some(url), None).map_err(|e| e.to_string())?;
  let article = readability.parse().map_err(|e| e.to_string())?;
  let markdown = htmd::convert(article.content.as_ref()).map_err(|e| e.to_string())?;
  Ok((article.title, markdown))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn extract_readable_pulls_article_and_drops_chrome() {
    // Readability は十分な分量の本文を要求するため、段落を長めにする。
    let html = r#"<html><head><title>緑茶の淹れ方</title></head><body>
      <nav>メニュー ホーム 会社概要 お問い合わせ ログイン</nav>
      <article>
        <h1>緑茶の淹れ方</h1>
        <p>緑茶をおいしく淹れるには湯温が最も重要です。煎茶の場合はおよそ70度から80度のお湯を使います。
        熱湯をそのまま注ぐと渋み成分のカテキンが過剰に抽出され、苦く渋い味になってしまいます。
        まず沸騰させたお湯を湯冷ましや別の器に移して適温まで下げ、茶葉の量は一人あたり約2グラムを目安にします。</p>
        <p>抽出時間も味を左右します。煎茶であれば60秒前後でゆっくりと旨味成分のテアニンが溶け出します。
        最後の一滴まで注ぎ切ることで、二煎目以降も同じ茶葉でおいしく楽しむことができます。
        玉露はさらに低い温度でじっくりと、ほうじ茶や玄米茶は高温でさっと淹れるのが基本です。</p>
      </article>
      <footer>著作権表示 プライバシーポリシー</footer>
    </body></html>"#;
    let (title, md) = extract_readable(html, "https://example.com/green-tea").unwrap();
    assert!(title.contains("緑茶"), "title was: {title}");
    assert!(md.contains("湯温"), "markdown was: {md}");
    // ナビ/フッタのノイズが落ちていること。
    assert!(!md.contains("プライバシーポリシー"), "chrome leaked: {md}");
  }
}
