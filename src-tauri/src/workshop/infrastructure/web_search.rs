use std::time::Duration;

use futures::future::{BoxFuture, FutureExt};
use serde::{Deserialize, Serialize};

const BRAVE_SEARCH_URL: &str = "https://api.search.brave.com/res/v1/web/search";
/// 要求件数。count クエリと parse 側の上限で同じ定数を共有する。
const RESULT_COUNT: usize = 5;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct WebSearchResult {
  pub title: String,
  pub url: String,
  pub snippet: String,
}

pub(crate) trait SearchBackend: Send + Sync {
  fn search(&self, query: String) -> BoxFuture<'static, Result<Vec<WebSearchResult>, String>>;
}

pub(crate) struct BraveSearchBackend {
  api_key: String,
  client: reqwest::Client,
}

impl BraveSearchBackend {
  pub(crate) fn new(api_key: String) -> Self {
    Self { api_key, client: reqwest::Client::new() }
  }
}

impl SearchBackend for BraveSearchBackend {
  fn search(&self, query: String) -> BoxFuture<'static, Result<Vec<WebSearchResult>, String>> {
    let api_key = self.api_key.trim().to_string();
    let client = self.client.clone();
    async move {
      if api_key.is_empty() {
        return Err("Brave Search API key is not configured".to_string());
      }
      // 貼り付け由来の制御文字入り key は header 構築段階で落ちるため、先に明確なエラーへ変える。
      let api_key = reqwest::header::HeaderValue::from_str(&api_key)
        .map_err(|_| "Brave Search API key contains invalid characters".to_string())?;
      let count = RESULT_COUNT.to_string();
      let response = client
        .get(BRAVE_SEARCH_URL)
        .header("X-Subscription-Token", api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::USER_AGENT, crate::extract::APP_USER_AGENT)
        // text_decorations=false: title/description への HTML 強調タグ混入を抑止する。
        // safesearch=moderate（Brave の既定）: strict は医学・法律・歴史等の専門的な検索結果を
        // 無言で落とすため、露骨なメディアだけ弾く moderate に留める。
        .query(&[
          ("q", query.as_str()),
          ("count", count.as_str()),
          ("safesearch", "moderate"),
          ("text_decorations", "false"),
        ])
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Brave Search request failed: {e}"))?;
      let status = response.status();
      if !status.is_success() {
        // 状態コードだけでは原因（key 失効等）が分からないため、応答体の先頭を添える。
        let mut message = format!("Brave Search request failed with HTTP {}", status.as_u16());
        let detail: String =
          response.text().await.unwrap_or_default().chars().take(200).collect();
        let detail = detail.trim();
        if !detail.is_empty() {
          message.push_str(": ");
          message.push_str(detail);
        }
        return Err(message);
      }
      let body = response
        .text()
        .await
        .map_err(|e| format!("Brave Search response read failed: {e}"))?;
      parse_brave_response(&body)
    }
    .boxed()
  }
}

#[derive(Deserialize)]
struct BraveResponse {
  // キー欠落だけでなく明示的な `"web": null` も空結果として扱うため Option で受ける。
  #[serde(default)]
  web: Option<BraveWeb>,
}

#[derive(Deserialize, Default)]
struct BraveWeb {
  #[serde(default)]
  results: Vec<BraveResult>,
}

#[derive(Deserialize)]
struct BraveResult {
  #[serde(default)]
  title: String,
  #[serde(default)]
  url: String,
  #[serde(default)]
  description: String,
}

fn parse_brave_response(body: &str) -> Result<Vec<WebSearchResult>, String> {
  let response: BraveResponse =
    serde_json::from_str(body).map_err(|e| format!("Brave Search returned invalid JSON: {e}"))?;
  Ok(
    response
      .web
      .unwrap_or_default()
      .results
      .into_iter()
      .take(RESULT_COUNT)
      .map(|item| WebSearchResult { title: item.title, url: item.url, snippet: item.description })
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_brave_web_results() {
    let results = parse_brave_response(
      r#"{
        "web": {
          "results": [
            {
              "title": "ExpertBase",
              "url": "https://example.com/expertbase",
              "description": "Local-first knowledge base"
            }
          ]
        }
      }"#,
    )
    .unwrap();

    assert_eq!(
      results,
      vec![WebSearchResult {
        title: "ExpertBase".into(),
        url: "https://example.com/expertbase".into(),
        snippet: "Local-first knowledge base".into(),
      }]
    );
  }

  #[test]
  fn missing_web_results_returns_empty_list() {
    assert!(parse_brave_response("{}").unwrap().is_empty());
  }

  #[test]
  fn explicit_null_web_returns_empty_list() {
    assert!(parse_brave_response(r#"{"web": null}"#).unwrap().is_empty());
  }

  #[tokio::test]
  async fn api_key_with_control_chars_returns_clear_error() {
    let error =
      BraveSearchBackend::new("bad\nkey".into()).search("ExpertBase".into()).await.unwrap_err();

    assert_eq!(error, "Brave Search API key contains invalid characters");
  }

  #[tokio::test]
  async fn missing_api_key_returns_before_request() {
    let error = BraveSearchBackend::new("  ".into()).search("ExpertBase".into()).await.unwrap_err();

    assert_eq!(error, "Brave Search API key is not configured");
  }
}
