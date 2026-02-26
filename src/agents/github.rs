use anyhow::{anyhow, Result};
use regex::Regex;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::cache::{CachedGitHubData, GitHubCache, SerializableGitHubData};
use super::{GitHubData, Release};

const GITHUB_API_BASE: &str = "https://api.github.com";

/// Detect a GitHub token for authenticated API access (5,000 req/hr vs 60).
/// Tries `gh auth token` first (works if user has gh CLI installed and logged in),
/// then falls back to the `GITHUB_TOKEN` environment variable.
pub fn detect_github_token() -> Option<String> {
    // Try gh CLI first
    if let Ok(output) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
    {
        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }

    // Fall back to GITHUB_TOKEN env var
    std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty())
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

fn map_release_response(r: ReleaseResponse) -> Release {
    Release {
        version: extract_version(&r.tag_name),
        date: r.published_at,
        changelog: r.body,
    }
}

fn build_github_data(repo_info: RepoResponse, releases: Vec<ReleaseResponse>) -> GitHubData {
    GitHubData {
        stars: Some(repo_info.stargazers_count),
        open_issues: Some(repo_info.open_issues_count),
        license: repo_info
            .license
            .and_then(|l| l.spdx_id)
            .filter(|s| s != "NOASSERTION"),
        last_commit: repo_info.pushed_at,
        releases: releases.into_iter().map(map_release_response).collect(),
    }
}

fn build_github_data_with_cached_releases(
    repo_info: &RepoResponse,
    cached: &CachedGitHubData,
) -> GitHubData {
    let mut data: GitHubData = cached.data.clone().into();
    data.stars = Some(repo_info.stargazers_count);
    data.open_issues = Some(repo_info.open_issues_count);
    data.license = repo_info
        .license
        .as_ref()
        .and_then(|l| l.spdx_id.clone())
        .filter(|s| s != "NOASSERTION");
    data.last_commit = repo_info.pushed_at.clone();
    data
}

fn releases_match_cached(cached: &CachedGitHubData, releases: &[ReleaseResponse]) -> bool {
    if cached.data.releases.len() != releases.len() {
        return false;
    }

    cached
        .data
        .releases
        .iter()
        .zip(releases.iter())
        .all(|(cached_release, fetched_release)| {
            cached_release.version == extract_version(&fetched_release.tag_name)
                && cached_release.date == fetched_release.published_at
                && cached_release.changelog == fetched_release.body
        })
}

fn resolve_repo_not_modified_with_releases(
    cached: Option<&CachedGitHubData>,
    cached_etag: Option<String>,
    releases_fetch: std::result::Result<Vec<ReleaseResponse>, ()>,
) -> ConditionalFetchResult {
    let Some(cached) = cached else {
        return ConditionalFetchResult::NotModified;
    };

    let Ok(releases) = releases_fetch else {
        return ConditionalFetchResult::NotModified;
    };

    if releases_match_cached(cached, &releases) {
        return ConditionalFetchResult::NotModified;
    }

    let mut data: GitHubData = cached.data.clone().into();
    data.releases = releases.into_iter().map(map_release_response).collect();
    ConditionalFetchResult::Fresh(data, cached_etag)
}

fn etag_for_releases_failure(
    cached_etag: Option<String>,
    new_repo_etag: Option<String>,
) -> Option<String> {
    cached_etag.or(new_repo_etag)
}

/// Result of a conditional fetch operation
#[derive(Debug, Clone)]
pub enum ConditionalFetchResult {
    /// New data fetched, with optional ETag for future conditional requests
    Fresh(GitHubData, Option<String>),
    /// Cached data is still valid (304 Not Modified)
    NotModified,
    /// Fetch failed with error message
    Error(String),
}

/// Extract version number from various tag formats.
/// Handles: "v1.2.3", "1.2.3", "rust-v0.92.0", "release-1.2.3", etc.
fn extract_version(tag: &str) -> String {
    // Try to find a semver-like pattern (X.Y.Z with optional pre-release)
    let re = Regex::new(r"(\d+\.\d+\.\d+(?:-[\w.]+)?)").unwrap();
    if let Some(captures) = re.captures(tag) {
        if let Some(m) = captures.get(1) {
            return m.as_str().to_string();
        }
    }
    // Fallback: strip common prefixes
    tag.strip_prefix('v')
        .or_else(|| tag.strip_prefix("release-"))
        .unwrap_or(tag)
        .to_string()
}

#[derive(Clone)]
pub struct AsyncGitHubClient {
    client: reqwest::Client,
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
            disk_cache,
            token,
        }
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

    /// Fetch only releases for a repo (single API call, no repo metadata).
    /// Used by CLI commands that don't need stars/issues/license.
    pub async fn fetch_releases_only(&self, repo: &str) -> ConditionalFetchResult {
        let releases_url = format!("{}/repos/{}/releases", GITHUB_API_BASE, repo);
        match self.get_json::<Vec<ReleaseResponse>>(&releases_url).await {
            Ok(releases) => {
                let data = GitHubData {
                    releases: releases.into_iter().map(map_release_response).collect(),
                    stars: None,
                    open_issues: None,
                    license: None,
                    last_commit: None,
                };

                // Update disk cache with releases data
                {
                    let mut cache = self.disk_cache.write().await;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    // Merge with existing cached repo metadata if available
                    let existing = cache.get(repo).cloned();
                    let merged_data = if let Some(existing) = &existing {
                        let mut merged: GitHubData = existing.data.clone().into();
                        merged.releases = data.releases.clone();
                        merged
                    } else {
                        data.clone()
                    };

                    cache.insert(
                        repo.to_string(),
                        CachedGitHubData {
                            data: SerializableGitHubData::from(&merged_data),
                            etag: existing.and_then(|e| e.etag),
                            fetched_at: now,
                        },
                    );
                }

                ConditionalFetchResult::Fresh(data, None)
            }
            Err(e) => {
                // Fall back to cached releases
                let cache = self.disk_cache.read().await;
                if cache.get(repo).is_some() {
                    ConditionalFetchResult::NotModified
                } else {
                    ConditionalFetchResult::Error(e.to_string())
                }
            }
        }
    }

    /// Fetch GitHub data conditionally using ETag-based caching.
    ///
    /// If we have cached data with an ETag for this repo, we send an If-None-Match header.
    /// If GitHub returns 304 Not Modified, we know our cached data is still valid.
    pub async fn fetch_conditional(&self, repo: &str) -> ConditionalFetchResult {
        // Check disk cache for existing ETag
        let (cached_etag, cached_entry) = {
            let cache = self.disk_cache.read().await;
            let entry = cache.get(repo).cloned();
            let etag = entry.as_ref().and_then(|e| e.etag.clone());
            (etag, entry)
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
            let releases_url = format!("{}/repos/{}/releases", GITHUB_API_BASE, repo);
            let releases_fetch = self
                .get_json::<Vec<ReleaseResponse>>(&releases_url)
                .await
                .map_err(|_| ());

            let result = resolve_repo_not_modified_with_releases(
                cached_entry.as_ref(),
                cached_etag.clone(),
                releases_fetch,
            );

            if let ConditionalFetchResult::Fresh(ref data, ref etag) = result {
                let mut cache = self.disk_cache.write().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                cache.insert(
                    repo.to_string(),
                    CachedGitHubData {
                        data: SerializableGitHubData::from(data),
                        etag: etag.clone(),
                        fetched_at: now,
                    },
                );
            }

            return result;
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

        // Fetch releases separately
        // If fetch fails, try to preserve cached releases but keep the OLD ETag
        // to ensure we re-fetch releases on the next attempt
        let releases_url = format!("{}/repos/{}/releases", GITHUB_API_BASE, repo);
        let mut etag_for_cache = new_etag.clone();
        let data = match self.get_json::<Vec<ReleaseResponse>>(&releases_url).await {
            Ok(releases) => build_github_data(repo_info, releases),
            Err(_) => {
                // Fetch failed - try to preserve cached releases
                if let Some(cached) = cached_entry.as_ref() {
                    etag_for_cache =
                        etag_for_releases_failure(cached_etag.clone(), new_etag.clone());
                    build_github_data_with_cached_releases(&repo_info, cached)
                } else {
                    // No cached releases to preserve. Keep the repo ETag; a repo 304 will still
                    // fetch /releases and recover once the transient error clears.
                    etag_for_cache = etag_for_releases_failure(None, new_etag.clone());
                    build_github_data(repo_info, Vec::new())
                }
            }
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
                    etag: etag_for_cache.clone(),
                    fetched_at: now,
                },
            );
        }

        ConditionalFetchResult::Fresh(data, etag_for_cache)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn release_response(tag: &str, published_at: &str) -> ReleaseResponse {
        ReleaseResponse {
            tag_name: tag.to_string(),
            published_at: Some(published_at.to_string()),
            body: None,
        }
    }

    fn cached_entry_with_releases(releases: Vec<(&str, &str)>) -> CachedGitHubData {
        let releases = releases
            .into_iter()
            .map(|(version, date)| super::super::cache::SerializableRelease {
                version: version.to_string(),
                date: Some(date.to_string()),
                changelog: None,
            })
            .collect();

        CachedGitHubData {
            data: SerializableGitHubData {
                releases,
                stars: Some(42),
                open_issues: Some(7),
                license: Some("MIT".to_string()),
                last_commit: Some("2024-01-01T00:00:00Z".to_string()),
            },
            etag: Some("\"repo-etag\"".to_string()),
            fetched_at: 123,
        }
    }

    fn repo_response(stars: u64, license: Option<&str>, pushed_at: &str) -> RepoResponse {
        RepoResponse {
            stargazers_count: stars,
            open_issues_count: 11,
            license: Some(LicenseResponse {
                spdx_id: license.map(str::to_string),
            }),
            pushed_at: Some(pushed_at.to_string()),
        }
    }

    #[test]
    fn test_format_stars() {
        assert_eq!(format_stars(0), "0");
        assert_eq!(format_stars(999), "999");
        assert_eq!(format_stars(1000), "1.0k");
        assert_eq!(format_stars(1234567), "1.2m");
    }

    #[test]
    fn test_extract_version() {
        // Standard v-prefixed
        assert_eq!(extract_version("v1.2.3"), "1.2.3");
        // No prefix
        assert_eq!(extract_version("1.2.3"), "1.2.3");
        // Rust-prefixed (like openai/codex)
        assert_eq!(extract_version("rust-v0.92.0"), "0.92.0");
        // Release-prefixed
        assert_eq!(extract_version("release-2.0.0"), "2.0.0");
        // With prerelease
        assert_eq!(extract_version("v1.0.0-beta.1"), "1.0.0-beta.1");
    }

    #[test]
    fn test_repo_304_with_changed_releases_returns_fresh() {
        let cached = cached_entry_with_releases(vec![("1.0.0", "2024-01-01T00:00:00Z")]);

        let result = resolve_repo_not_modified_with_releases(
            Some(&cached),
            cached.etag.clone(),
            Ok(vec![release_response("v1.1.0", "2024-02-01T00:00:00Z")]),
        );

        match result {
            ConditionalFetchResult::Fresh(data, etag) => {
                assert_eq!(etag, cached.etag);
                assert_eq!(data.stars, Some(42));
                assert_eq!(data.license.as_deref(), Some("MIT"));
                assert_eq!(data.latest_version(), Some("1.1.0"));
                assert_eq!(data.releases.len(), 1);
            }
            other => panic!("expected Fresh, got {other:?}"),
        }
    }

    #[test]
    fn test_repo_304_with_unchanged_releases_returns_not_modified() {
        let cached = cached_entry_with_releases(vec![("1.0.0", "2024-01-01T00:00:00Z")]);

        let result = resolve_repo_not_modified_with_releases(
            Some(&cached),
            cached.etag.clone(),
            Ok(vec![release_response("v1.0.0", "2024-01-01T00:00:00Z")]),
        );

        assert!(matches!(result, ConditionalFetchResult::NotModified));
    }

    #[test]
    fn test_repo_304_with_releases_fetch_error_returns_not_modified() {
        let cached = cached_entry_with_releases(vec![("1.0.0", "2024-01-01T00:00:00Z")]);

        let result =
            resolve_repo_not_modified_with_releases(Some(&cached), cached.etag.clone(), Err(()));

        assert!(matches!(result, ConditionalFetchResult::NotModified));
    }

    #[test]
    fn test_partial_releases_failure_with_cache_preserves_releases_and_old_etag() {
        let cached = cached_entry_with_releases(vec![("1.0.0", "2024-01-01T00:00:00Z")]);
        let repo_info = repo_response(99, Some("Apache-2.0"), "2024-03-01T00:00:00Z");

        let data = build_github_data_with_cached_releases(&repo_info, &cached);
        let etag =
            etag_for_releases_failure(cached.etag.clone(), Some("\"new-repo-etag\"".to_string()));

        assert_eq!(etag, cached.etag);
        assert_eq!(data.stars, Some(99));
        assert_eq!(data.license.as_deref(), Some("Apache-2.0"));
        assert_eq!(data.last_commit.as_deref(), Some("2024-03-01T00:00:00Z"));
        assert_eq!(data.latest_version(), Some("1.0.0"));
    }

    #[test]
    fn test_partial_releases_failure_without_cache_keeps_new_repo_etag() {
        let new_repo_etag = Some("\"new-repo-etag\"".to_string());
        let etag = etag_for_releases_failure(None, new_repo_etag.clone());
        assert_eq!(etag, new_repo_etag);
    }
}
