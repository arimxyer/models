use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::GitHubData;

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

#[derive(Clone)]
pub struct AsyncGitHubClient {
    client: reqwest::Client,
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    token: Option<String>,
}

impl Default for AsyncGitHubClient {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AsyncGitHubClient {
    pub fn new(token: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("models-tui")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            cache: Arc::new(Mutex::new(HashMap::new())),
            token,
        }
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

        // Fetch repo and release in parallel
        let repo_url = format!("{}/repos/{}", GITHUB_API_BASE, repo);
        let release_url = format!("{}/repos/{}/releases/latest", GITHUB_API_BASE, repo);

        let (repo_result, release_result) = tokio::join!(
            self.get_json::<RepoResponse>(&repo_url),
            self.get_json::<ReleaseResponse>(&release_url),
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

        if let Ok(release) = release_result {
            let version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
            data.latest_version = Some(version.to_string());
            data.release_date = release.published_at.map(|s| format_relative_time(&s));
            data.changelog = release.body;
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

// Sync wrapper for backward compatibility (used by existing TUI code)
pub struct GitHubClient {
    _async_client: AsyncGitHubClient,
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubClient {
    pub fn new() -> Self {
        Self {
            _async_client: AsyncGitHubClient::new(None),
        }
    }

    // Note: These methods will be removed when we fully migrate to async
    pub fn fetch(&self, _repo: &str) -> Result<GitHubData> {
        // Return empty data for now - will be populated by async fetches
        Ok(GitHubData::default())
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
