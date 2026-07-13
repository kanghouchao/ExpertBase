//! workshop インフラ: Web 系ツール（search_web / fetch_web）。

use std::convert::Infallible;
use std::sync::Arc;

use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::Deserialize;
use serde_json::json;

use crate::extract::{extract_readable, fetch_html};

use super::super::web_search::SearchBackend;
use super::{remember_source, SearchArgs, UsedSources};

/// Web 検索で候補 URL を返す。本文は fetch_web で選択的に読む。
pub struct SearchWeb {
  pub backend: Arc<dyn SearchBackend>,
}

impl Tool for SearchWeb {
  const NAME: &'static str = "search_web";
  type Error = Infallible;
  type Args = SearchArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Search the web by keywords. Returns a JSON array of results with title, url, and snippet; returns a parenthesized plain-text notice when there are no results or the search fails. Use fetch_web on a selected result before relying on or saving its content."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "query": { "type": "string", "description": "Keywords to search for on the web" }
        },
        "required": ["query"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let query = args.query.trim();
    if query.is_empty() {
      return Ok("(search_web needs a non-empty query)".to_string());
    }
    match self.backend.search(query.to_string()).await {
      // 空結果は兄弟ツールと同じ括弧書きの案内で返す（裸の "[]" は弱いモデルが誤読する）。
      Ok(results) if results.is_empty() => Ok("(no web results)".to_string()),
      Ok(results) => Ok(
        serde_json::to_string(&results)
          .unwrap_or_else(|e| format!("(search_web result serialization failed: {e})")),
      ),
      Err(e) => Ok(format!("(search_web error: {e})")),
    }
  }
}

/// fetch_web の引数。URL を緩く受ける。
#[derive(Deserialize)]
pub struct FetchArgs {
  #[serde(default)]
  url: String,
}

/// ユーザーが会話に渡した URL の本文を Markdown で返す読み取りツール。
/// `web::fetch_html`（HTTPS 取得）+ `web::extract_readable`（Readability→Markdown）を再利用する。
/// 単一 URL の本文抽出のみ。許可リスト / SSRF 防御は入れない（local-first・単一ユーザー前提）。
pub struct FetchWeb {
  pub used_sources: UsedSources,
}

impl Tool for FetchWeb {
  const NAME: &'static str = "fetch_web";
  type Error = Infallible;
  type Args = FetchArgs;
  type Output = String;

  async fn definition(&self, _prompt: String) -> ToolDefinition {
    ToolDefinition {
      name: Self::NAME.to_string(),
      description:
        "Fetch a web page the user gave you and return its main text as Markdown. Use it when the user shares a URL to read, summarize, or save."
          .to_string(),
      parameters: json!({
        "type": "object",
        "properties": {
          "url": { "type": "string", "description": "The page URL to fetch" }
        },
        "required": ["url"]
      }),
    }
  }

  async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let url = args.url.trim();
    if url.is_empty() {
      return Ok("(fetch_web needs a non-empty url)".to_string());
    }
    let html = match fetch_html(url).await {
      Ok(h) => h,
      Err(e) => return Ok(format!("(fetch error: {e})")),
    };
    match extract_readable(&html, url) {
      Ok((title, markdown)) => {
        remember_source(&self.used_sources, url);
        Ok(format_web_body(&title, &markdown))
      }
      Err(e) => Ok(format!("(extract error: {e})")),
    }
  }
}

/// タイトルがあれば本文の先頭に `# title` を前置する（無ければ本文のみ）。
fn format_web_body(title: &str, markdown: &str) -> String {
  if title.trim().is_empty() {
    markdown.to_string()
  } else {
    format!("# {}\n\n{}", title.trim(), markdown)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::workshop::infrastructure::web_search::WebSearchResult;
  use futures::future::{BoxFuture, FutureExt};
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::Mutex;

  struct FakeSearchBackend {
    calls: Arc<AtomicUsize>,
    result: Result<Vec<WebSearchResult>, String>,
  }

  impl SearchBackend for FakeSearchBackend {
    fn search(&self, _query: String) -> BoxFuture<'static, Result<Vec<WebSearchResult>, String>> {
      self.calls.fetch_add(1, Ordering::Relaxed);
      let result = self.result.clone();
      async move { result }.boxed()
    }
  }

  #[tokio::test]
  async fn fetch_web_rejects_empty_url() {
    let used_sources = Arc::new(Mutex::new(Vec::new()));
    let out = FetchWeb { used_sources: used_sources.clone() }
      .call(FetchArgs { url: "  ".into() })
      .await
      .unwrap();
    assert!(out.contains("needs a non-empty url"), "was: {out}");
    assert!(used_sources.lock().unwrap().is_empty());
  }

  #[tokio::test]
  async fn search_web_rejects_empty_query_without_calling_backend() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tool = SearchWeb {
      backend: Arc::new(FakeSearchBackend { calls: calls.clone(), result: Ok(vec![]) }),
    };

    let out = tool.call(SearchArgs { query: "  ".into() }).await.unwrap();

    assert_eq!(out, "(search_web needs a non-empty query)");
    assert_eq!(calls.load(Ordering::Relaxed), 0);
  }

  #[tokio::test]
  async fn search_web_returns_structured_results() {
    let tool = SearchWeb {
      backend: Arc::new(FakeSearchBackend {
        calls: Arc::new(AtomicUsize::new(0)),
        result: Ok(vec![WebSearchResult {
          title: "ExpertBase".into(),
          url: "https://example.com/expertbase".into(),
          snippet: "Local-first knowledge base".into(),
        }]),
      }),
    };

    let out = tool.call(SearchArgs { query: "ExpertBase".into() }).await.unwrap();
    let results: serde_json::Value = serde_json::from_str(&out).unwrap();

    assert_eq!(results[0]["title"], "ExpertBase");
    assert_eq!(results[0]["url"], "https://example.com/expertbase");
    assert_eq!(results[0]["snippet"], "Local-first knowledge base");
  }

  #[tokio::test]
  async fn search_web_returns_notice_for_empty_results() {
    let tool = SearchWeb {
      backend: Arc::new(FakeSearchBackend {
        calls: Arc::new(AtomicUsize::new(0)),
        result: Ok(vec![]),
      }),
    };

    let out = tool.call(SearchArgs { query: "ExpertBase".into() }).await.unwrap();

    assert_eq!(out, "(no web results)");
  }

  #[tokio::test]
  async fn search_web_returns_backend_errors_without_failing_tool_loop() {
    let tool = SearchWeb {
      backend: Arc::new(FakeSearchBackend {
        calls: Arc::new(AtomicUsize::new(0)),
        result: Err("Brave Search request failed with HTTP 429".into()),
      }),
    };

    let out = tool.call(SearchArgs { query: "ExpertBase".into() }).await.unwrap();

    assert_eq!(out, "(search_web error: Brave Search request failed with HTTP 429)");
  }

  #[tokio::test]
  async fn fetch_web_formats_extracted_body_with_title() {
    // 実ネットは叩かない。抽出器の出力整形（title を見出しに前置）だけを検証する。
    let body = super::format_web_body("緑茶の淹れ方", "湯温は70度。");
    assert!(body.starts_with("# 緑茶の淹れ方"));
    assert!(body.contains("湯温は70度。"));
    // タイトルが空なら見出しを足さない。
    assert_eq!(super::format_web_body("  ", "本文だけ"), "本文だけ");
  }
}
