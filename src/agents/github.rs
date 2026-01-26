use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::GitHubData;

/// Cache time-to-live: 1 hour
#[allow(dead_code)]
const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

/// Cache entry with data and timestamp
#[allow(dead_code)]
struct CacheEntry {
    data: GitHubData,
    fetched_at: Instant,
}

/// GitHub API response for repository info (used internally)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RepoResponse {
    pub stargazers_count: u64,
    pub open_issues_count: u64,
    pub license: Option<LicenseResponse>,
    pub pushed_at: Option<String>,
}

/// License info from GitHub API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LicenseResponse {
    pub spdx_id: Option<String>,
}

/// GitHub API response for latest release
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReleaseResponse {
    pub tag_name: String,
    pub published_at: Option<String>,
    pub body: Option<String>,
}

/// GitHub API client with caching
#[allow(dead_code)]
pub struct GitHubClient {
    cache: Mutex<HashMap<String, CacheEntry>>,
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl GitHubClient {
    /// Create a new GitHub client
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Fetch GitHub data, using cache if available and not expired
    pub fn fetch(&self, repo: &str) -> Result<GitHubData> {
        // Check cache first
        {
            let cache = self.cache.lock().map_err(|e| anyhow!("Cache lock error: {}", e))?;
            if let Some(entry) = cache.get(repo) {
                if entry.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(entry.data.clone());
                }
            }
        }

        // Fetch fresh data
        let data = self.fetch_fresh(repo)?;

        // Update cache
        {
            let mut cache = self.cache.lock().map_err(|e| anyhow!("Cache lock error: {}", e))?;
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

    /// Fetch fresh data from GitHub, bypassing cache
    pub fn fetch_fresh(&self, repo: &str) -> Result<GitHubData> {
        let mut data = GitHubData::default();

        // Fetch repo info
        if let Ok(repo_info) = self.fetch_repo(repo) {
            data.stars = Some(repo_info.stargazers_count);
            data.open_issues = Some(repo_info.open_issues_count);
            data.license = repo_info
                .license
                .and_then(|l| l.spdx_id)
                .filter(|s| s != "NOASSERTION");
            data.last_commit = repo_info.pushed_at.map(|s| format_relative_time(&s));
        }

        // Fetch latest release
        if let Ok(release) = self.fetch_latest_release(repo) {
            // Strip 'v' prefix from version tag
            let version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
            data.latest_version = Some(version.to_string());
            data.release_date = release.published_at.map(|s| format_relative_time(&s));
            data.changelog = release.body;
        }

        Ok(data)
    }

    /// Fetch repository info from GitHub API
    pub fn fetch_repo(&self, repo: &str) -> Result<RepoResponse> {
        let output = Command::new("gh")
            .args(["api", &format!("repos/{}", repo)])
            .output()
            .map_err(|e| anyhow!("Failed to run gh command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("gh api failed: {}", stderr));
        }

        let response: RepoResponse = serde_json::from_slice(&output.stdout)
            .map_err(|e| anyhow!("Failed to parse repo response: {}", e))?;

        Ok(response)
    }

    /// Fetch latest release from GitHub API
    pub fn fetch_latest_release(&self, repo: &str) -> Result<ReleaseResponse> {
        let output = Command::new("gh")
            .args(["api", &format!("repos/{}/releases/latest", repo)])
            .output()
            .map_err(|e| anyhow!("Failed to run gh command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("gh api failed: {}", stderr));
        }

        let response: ReleaseResponse = serde_json::from_slice(&output.stdout)
            .map_err(|e| anyhow!("Failed to parse release response: {}", e))?;

        Ok(response)
    }
}

/// Format star count for display (e.g., 12345 -> "12.3k")
#[allow(dead_code)]
pub fn format_stars(stars: u64) -> String {
    if stars >= 1_000_000 {
        format!("{:.1}m", stars as f64 / 1_000_000.0)
    } else if stars >= 1_000 {
        format!("{:.1}k", stars as f64 / 1_000.0)
    } else {
        stars.to_string()
    }
}

/// Format ISO date string to relative time (for now, just extract date portion)
#[allow(dead_code)]
pub fn format_relative_time(iso_date: &str) -> String {
    // ISO format: "2024-01-15T10:30:00Z"
    // Extract just the date part for now
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
        assert_eq!(format_stars(1), "1");
        assert_eq!(format_stars(999), "999");
        assert_eq!(format_stars(1000), "1.0k");
        assert_eq!(format_stars(1234), "1.2k");
        assert_eq!(format_stars(12345), "12.3k");
        assert_eq!(format_stars(123456), "123.5k");
        assert_eq!(format_stars(1000000), "1.0m");
        assert_eq!(format_stars(1234567), "1.2m");
    }

    #[test]
    fn test_format_relative_time() {
        assert_eq!(format_relative_time("2024-01-15T10:30:00Z"), "2024-01-15");
        assert_eq!(format_relative_time("2024-12-25"), "2024-12-25");
        assert_eq!(format_relative_time("invalid"), "invalid");
    }
}
