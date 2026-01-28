//! Persistent disk cache for GitHub data
//!
//! Stores GitHub API responses to disk to reduce API calls and enable
//! offline access to previously fetched data.

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::{GitHubData, Release};

/// Cache file version for future migration support
const CACHE_VERSION: u32 = 1;

/// Cache file name
const CACHE_FILENAME: &str = "github-cache.json";

/// Serializable version of Release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableRelease {
    pub version: String,
    pub date: Option<String>,
    pub changelog: Option<String>,
}

impl From<&Release> for SerializableRelease {
    fn from(release: &Release) -> Self {
        Self {
            version: release.version.clone(),
            date: release.date.clone(),
            changelog: release.changelog.clone(),
        }
    }
}

impl From<SerializableRelease> for Release {
    fn from(release: SerializableRelease) -> Self {
        Self {
            version: release.version,
            date: release.date,
            changelog: release.changelog,
        }
    }
}

/// Serializable version of GitHubData
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableGitHubData {
    pub releases: Vec<SerializableRelease>,
    pub stars: Option<u64>,
    pub open_issues: Option<u64>,
    pub license: Option<String>,
    pub last_commit: Option<String>,
}

impl From<&GitHubData> for SerializableGitHubData {
    fn from(data: &GitHubData) -> Self {
        Self {
            releases: data
                .releases
                .iter()
                .map(SerializableRelease::from)
                .collect(),
            stars: data.stars,
            open_issues: data.open_issues,
            license: data.license.clone(),
            last_commit: data.last_commit.clone(),
        }
    }
}

impl From<SerializableGitHubData> for GitHubData {
    fn from(data: SerializableGitHubData) -> Self {
        Self {
            releases: data.releases.into_iter().map(Release::from).collect(),
            stars: data.stars,
            open_issues: data.open_issues,
            license: data.license,
            last_commit: data.last_commit,
        }
    }
}

/// A single cached GitHub data entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedGitHubData {
    pub data: SerializableGitHubData,
    pub etag: Option<String>,
    pub fetched_at: i64,
}

/// The root cache structure stored on disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitHubCache {
    pub version: u32,
    pub entries: HashMap<String, CachedGitHubData>,
}

impl GitHubCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: HashMap::new(),
        }
    }

    /// Get the cache file path
    fn cache_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("models").join(CACHE_FILENAME))
    }

    /// Load the cache from disk, returning an empty cache if file doesn't exist or is invalid
    pub fn load() -> Self {
        Self::try_load().unwrap_or_else(|_| Self::new())
    }

    /// Try to load the cache from disk
    fn try_load() -> Result<Self> {
        let path = Self::cache_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(&path)?;
        let cache: GitHubCache = serde_json::from_str(&contents)?;

        // Future: handle version migrations here
        if cache.version != CACHE_VERSION {
            // For now, just return empty cache if version mismatch
            return Ok(Self::new());
        }

        Ok(cache)
    }

    /// Save the cache to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::cache_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;

        Ok(())
    }

    /// Get a cached entry for an agent
    pub fn get(&self, agent_id: &str) -> Option<&CachedGitHubData> {
        self.entries.get(agent_id)
    }

    /// Insert or update a cached entry
    pub fn insert(&mut self, agent_id: String, data: CachedGitHubData) {
        self.entries.insert(agent_id, data);
    }

    /// Remove a cached entry
    pub fn remove(&mut self, agent_id: &str) -> Option<CachedGitHubData> {
        self.entries.remove(agent_id)
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cache() {
        let cache = GitHubCache::new();
        assert_eq!(cache.version, CACHE_VERSION);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_insert_and_get() {
        let mut cache = GitHubCache::new();

        let data = CachedGitHubData {
            data: SerializableGitHubData {
                releases: vec![SerializableRelease {
                    version: "1.0.0".to_string(),
                    date: Some("2024-01-15".to_string()),
                    changelog: None,
                }],
                stars: Some(1000),
                open_issues: Some(10),
                license: Some("MIT".to_string()),
                last_commit: Some("2024-01-15".to_string()),
            },
            etag: Some("abc123".to_string()),
            fetched_at: 1234567890,
        };

        cache.insert("test-agent".to_string(), data);

        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        let retrieved = cache.get("test-agent");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data.stars, Some(1000));
    }

    #[test]
    fn test_remove() {
        let mut cache = GitHubCache::new();

        let data = CachedGitHubData {
            data: SerializableGitHubData {
                releases: vec![],
                stars: None,
                open_issues: None,
                license: None,
                last_commit: None,
            },
            etag: None,
            fetched_at: 0,
        };

        cache.insert("test".to_string(), data);
        assert_eq!(cache.len(), 1);

        cache.remove("test");
        assert!(cache.is_empty());
    }

    #[test]
    fn test_serializable_release_conversion() {
        let release = Release {
            version: "1.0.0".to_string(),
            date: Some("2024-01-15".to_string()),
            changelog: Some("Changes".to_string()),
        };

        let serializable: SerializableRelease = (&release).into();
        assert_eq!(serializable.version, "1.0.0");

        let back: Release = serializable.into();
        assert_eq!(back.version, "1.0.0");
        assert_eq!(back.date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_serializable_github_data_conversion() {
        let data = GitHubData {
            releases: vec![Release {
                version: "1.0.0".to_string(),
                date: None,
                changelog: None,
            }],
            stars: Some(500),
            open_issues: Some(5),
            license: Some("Apache-2.0".to_string()),
            last_commit: Some("2024-01-10".to_string()),
        };

        let serializable: SerializableGitHubData = (&data).into();
        assert_eq!(serializable.stars, Some(500));
        assert_eq!(serializable.releases.len(), 1);

        let back: GitHubData = serializable.into();
        assert_eq!(back.stars, Some(500));
        assert_eq!(back.releases[0].version, "1.0.0");
    }
}
