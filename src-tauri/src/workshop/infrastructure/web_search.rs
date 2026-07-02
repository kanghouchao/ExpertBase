use std::time::Duration;

use futures::future::{BoxFuture, FutureExt};
use serde::{Deserialize, Serialize};

const BRAVE_SEARCH_URL: &str = "https://api.search.brave.com/res/v1/web/search";
const RESULT_COUNT: &str = "5";

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
      let response = client
        .get(BRAVE_SEARCH_URL)
        .header("X-Subscription-Token", api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .query(&[("q", query), ("count", RESULT_COUNT.to_string()), ("safesearch", "strict".into())])
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Brave Search request failed: {e}"))?;
      if !response.status().is_success() {
        return Err(format!("Brave Search request failed with HTTP {}", response.status().as_u16()));
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

#[derive(Deserialize, Default)]
struct BraveResponse {
  #[serde(default)]
  web: BraveWeb,
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
      .results
      .into_iter()
      .take(5)
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

  #[tokio::test]
  async fn missing_api_key_returns_before_request() {
    let error = BraveSearchBackend::new("  ".into()).search("ExpertBase".into()).await.unwrap_err();

    assert_eq!(error, "Brave Search API key is not configured");
  }
}
