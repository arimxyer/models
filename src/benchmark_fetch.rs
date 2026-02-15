//! Async HTTP client for fetching benchmark data from jsDelivr CDN.

use crate::benchmarks::BenchmarkEntry;

const CDN_URL: &str = "https://cdn.jsdelivr.net/gh/arimxyer/models@main/data/benchmarks.json";

/// Result of a conditional fetch operation.
#[derive(Debug)]
pub enum BenchmarkFetchResult {
    /// New data fetched with optional ETag.
    Fresh(Vec<BenchmarkEntry>, Option<String>),
    /// Cached data is still valid (304 Not Modified).
    NotModified,
    /// Fetch failed.
    Error,
}

pub struct BenchmarkFetcher {
    client: reqwest::Client,
}

impl BenchmarkFetcher {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("models-tui")
            .build()
            .expect("Failed to build HTTP client");

        Self { client }
    }

    /// Fetch benchmark data, using ETag for conditional requests.
    pub async fn fetch_conditional(&self, etag: Option<&str>) -> BenchmarkFetchResult {
        let mut request = self.client.get(CDN_URL);

        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag);
        }

        let response = match request.send().await {
            Ok(resp) => resp,
            Err(_) => return BenchmarkFetchResult::Error,
        };

        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return BenchmarkFetchResult::NotModified;
        }

        if !response.status().is_success() {
            return BenchmarkFetchResult::Error;
        }

        let new_etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let entries: Vec<BenchmarkEntry> = match response.json().await {
            Ok(e) => e,
            Err(_) => return BenchmarkFetchResult::Error,
        };

        BenchmarkFetchResult::Fresh(entries, new_etag)
    }
}
