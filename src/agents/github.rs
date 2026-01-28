use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

use super::cache::{CachedGitHubData, GitHubCache, SerializableGitHubData};
use super::{GitHubData, Release};

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);
const GITHUB_API_BASE: &str = "https://api.github.com";

struct CacheEntry {
    data: GitHubData,
    fetched_at: Instant,
}

#[derive(Debug, Deserialize)]
pub struct RepoResponse {
    pub stargazers_count: u64,
    pub open_issues_count: u64,
    pub license: Option<LicenseResponse>,
    pub pushed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LicenseResponse {
    pub spdx_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseResponse {
    pub tag_name: String,
    pub published_at: Option<String>,
    pub body: Option<String>,
}

/// Result of a conditional fetch operation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ConditionalFetchResult {
    /// New data fetched, with optional ETag for future conditional requests
    Fresh(GitHubData, Option<String>),
    /// Cached data is still valid (304 Not Modified)
    NotModified,
    /// Fetch failed with error message
    Error(String),
}

#[derive(Clone)]
pub struct AsyncGitHubClient {
    client: reqwest::Client,
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    #[allow(dead_code)]
    disk_cache: Arc<RwLock<GitHubCache>>,
    token: Option<String>,
}

impl Default for AsyncGitHubClient {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AsyncGitHubClient {
    pub fn new(token: Option<String>) -> Self {
        Self::with_disk_cache(token, Arc::new(RwLock::new(GitHubCache::new())))
    }

    pub fn with_disk_cache(token: Option<String>, disk_cache: Arc<RwLock<GitHubCache>>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("models-tui")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            cache: Arc::new(Mutex::new(HashMap::new())),
            disk_cache,
            token,
        }
    }

    /// Get a reference to the disk cache
    #[allow(dead_code)]
    pub fn disk_cache(&self) -> &Arc<RwLock<GitHubCache>> {
        &self.disk_cache
    }

    pub async fn fetch(&self, repo: &str) -> Result<GitHubData> {
        // Check cache
        {
            let cache = self.cache.lock().await;
            if let Some(entry) = cache.get(repo) {
                if entry.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(entry.data.clone());
                }
            }
        }

        let data = self.fetch_fresh(repo).await?;

        // Update cache
        {
            let mut cache = self.cache.lock().await;
            cache.insert(
                repo.to_string(),
                CacheEntry {
                    data: data.clone(),
                    fetched_at: Instant::now(),
                },
            );
        }

        Ok(data)
    }

    pub async fn fetch_fresh(&self, repo: &str) -> Result<GitHubData> {
        let mut data = GitHubData::default();

        // Fetch repo and releases in parallel
        let repo_url = format!("{}/repos/{}", GITHUB_API_BASE, repo);
        let releases_url = format!("{}/repos/{}/releases", GITHUB_API_BASE, repo);

        let (repo_result, releases_result) = tokio::join!(
            self.get_json::<RepoResponse>(&repo_url),
            self.get_json::<Vec<ReleaseResponse>>(&releases_url),
        );

        if let Ok(repo_info) = repo_result {
            data.stars = Some(repo_info.stargazers_count);
            data.open_issues = Some(repo_info.open_issues_count);
            data.license = repo_info
                .license
                .and_then(|l| l.spdx_id)
                .filter(|s| s != "NOASSERTION");
            data.last_commit = repo_info.pushed_at.map(|s| format_relative_time(&s));
        }

        if let Ok(releases) = releases_result {
            data.releases = releases
                .into_iter()
                .map(|r| {
                    let version = r
                        .tag_name
                        .strip_prefix('v')
                        .unwrap_or(&r.tag_name)
                        .to_string();
                    Release {
                        version,
                        date: r.published_at.map(|s| format_relative_time(&s)),
                        changelog: r.body,
                    }
                })
                .collect();
        }

        Ok(data)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut request = self.client.get(url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if response.status() == 403 {
            return Err(anyhow!("GitHub API rate limit exceeded"));
        }

        if !response.status().is_success() {
            return Err(anyhow!("GitHub API error: {}", response.status()));
        }

        Ok(response.json().await?)
    }

    /// Fetch GitHub data conditionally using ETag-based caching.
    ///
    /// If we have cached data with an ETag for this repo, we send an If-None-Match header.
    /// If GitHub returns 304 Not Modified, we know our cached data is still valid.
    #[allow(dead_code)]
    pub async fn fetch_conditional(&self, repo: &str) -> ConditionalFetchResult {
        // Check disk cache for existing ETag
        let cached_etag = {
            let cache = self.disk_cache.read().await;
            cache.get(repo).and_then(|entry| entry.etag.clone())
        };

        // Build the request with conditional headers
        let repo_url = format!("{}/repos/{}", GITHUB_API_BASE, repo);
        let mut request = self.client.get(&repo_url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Add If-None-Match header if we have a cached ETag
        if let Some(ref etag) = cached_etag {
            request = request.header("If-None-Match", etag);
        }

        // Send the request
        let response = match request.send().await {
            Ok(resp) => resp,
            Err(e) => return ConditionalFetchResult::Error(e.to_string()),
        };

        // Handle 304 Not Modified
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return ConditionalFetchResult::NotModified;
        }

        // Handle rate limit
        if response.status() == 403 {
            return ConditionalFetchResult::Error("GitHub API rate limit exceeded".to_string());
        }

        // Handle other errors
        if !response.status().is_success() {
            return ConditionalFetchResult::Error(format!(
                "GitHub API error: {}",
                response.status()
            ));
        }

        // Extract ETag from response headers
        let new_etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Parse the response body
        let repo_info: RepoResponse = match response.json().await {
            Ok(info) => info,
            Err(e) => return ConditionalFetchResult::Error(e.to_string()),
        };

        // Fetch releases separately (they have their own ETag, but for simplicity we bundle them)
        let releases_url = format!("{}/repos/{}/releases", GITHUB_API_BASE, repo);
        let releases: Vec<ReleaseResponse> = self
            .get_json::<Vec<ReleaseResponse>>(&releases_url)
            .await
            .unwrap_or_default();

        // Build the GitHubData
        let data = GitHubData {
            stars: Some(repo_info.stargazers_count),
            open_issues: Some(repo_info.open_issues_count),
            license: repo_info
                .license
                .and_then(|l| l.spdx_id)
                .filter(|s| s != "NOASSERTION"),
            last_commit: repo_info.pushed_at.map(|s| format_relative_time(&s)),
            releases: releases
                .into_iter()
                .map(|r| {
                    let version = r
                        .tag_name
                        .strip_prefix('v')
                        .unwrap_or(&r.tag_name)
                        .to_string();
                    Release {
                        version,
                        date: r.published_at.map(|s| format_relative_time(&s)),
                        changelog: r.body,
                    }
                })
                .collect(),
        };

        // Update disk cache with new data and ETag
        {
            let mut cache = self.disk_cache.write().await;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            cache.insert(
                repo.to_string(),
                CachedGitHubData {
                    data: SerializableGitHubData::from(&data),
                    etag: new_etag.clone(),
                    fetched_at: now,
                },
            );
        }

        ConditionalFetchResult::Fresh(data, new_etag)
    }
}

/// Format star count for display (e.g., 12345 -> "12.3k")
pub fn format_stars(stars: u64) -> String {
    if stars >= 1_000_000 {
        format!("{:.1}m", stars as f64 / 1_000_000.0)
    } else if stars >= 1_000 {
        format!("{:.1}k", stars as f64 / 1_000.0)
    } else {
        stars.to_string()
    }
}

/// Format ISO date string to relative time
pub fn format_relative_time(iso_date: &str) -> String {
    if let Some(date) = iso_date.split('T').next() {
        date.to_string()
    } else {
        iso_date.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_stars() {
        assert_eq!(format_stars(0), "0");
        assert_eq!(format_stars(999), "999");
        assert_eq!(format_stars(1000), "1.0k");
        assert_eq!(format_stars(1234567), "1.2m");
    }

    #[test]
    fn test_format_relative_time() {
        assert_eq!(format_relative_time("2024-01-15T10:30:00Z"), "2024-01-15");
    }
}
