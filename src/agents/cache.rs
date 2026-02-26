//! Persistent disk cache for GitHub data
//!
//! Stores GitHub API responses to disk to reduce API calls and enable
//! offline access to previously fetched data.

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

impl SerializableGitHubData {
    pub fn to_github_data(&self) -> GitHubData {
        GitHubData {
            releases: self
                .releases
                .iter()
                .map(|r| Release {
                    version: r.version.clone(),
                    date: r.date.clone(),
                    changelog: r.changelog.clone(),
                })
                .collect(),
            stars: self.stars,
            open_issues: self.open_issues,
            license: self.license.clone(),
            last_commit: self.last_commit.clone(),
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
        let Some(path) = Self::cache_path() else {
            return Self::new();
        };
        Self::load_from_path(&path)
    }

    /// Try to load the cache from disk
    fn try_load() -> Result<Self> {
        let path = Self::cache_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Self::try_load_from_path(&path)
    }

    /// Load cache from an explicit file path, returning an empty cache if file doesn't exist or is invalid
    fn load_from_path(path: &Path) -> Self {
        Self::try_load_from_path(path).unwrap_or_else(|_| Self::new())
    }

    /// Try to load cache from an explicit file path
    fn try_load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(path)?;
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
        self.save_to_path(&path)
    }

    /// Save the cache to an explicit file path
    fn save_to_path(&self, path: &Path) -> Result<()> {
        // Create the directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;

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
    use std::env;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let unique = format!(
                "{}-{}-{}",
                prefix,
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let path = env::temp_dir().join(unique);
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_cached_data() -> CachedGitHubData {
        CachedGitHubData {
            data: SerializableGitHubData {
                releases: vec![SerializableRelease {
                    version: "1.2.3".to_string(),
                    date: Some("2024-01-15T00:00:00Z".to_string()),
                    changelog: Some("notes".to_string()),
                }],
                stars: Some(42),
                open_issues: Some(7),
                license: Some("MIT".to_string()),
                last_commit: Some("2024-01-16T01:02:03Z".to_string()),
            },
            etag: Some("\"etag-123\"".to_string()),
            fetched_at: 1_700_000_000,
        }
    }

    fn temp_cache_path(prefix: &str) -> (TempDirGuard, PathBuf) {
        let temp_dir = TempDirGuard::new(prefix);
        let path = temp_dir.path().join(CACHE_FILENAME);
        (temp_dir, path)
    }

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

    #[test]
    fn test_save_and_load_round_trip_preserves_repo_key_and_data() {
        let (temp_dir, cache_path) = temp_cache_path("github-cache-roundtrip");

        let repo_key = "owner/repo-name";
        let mut cache = GitHubCache::new();
        cache.insert(repo_key.to_string(), sample_cached_data());
        cache.save_to_path(&cache_path).unwrap();

        let loaded = GitHubCache::load_from_path(&cache_path);
        let entry = loaded.get(repo_key).expect("repo key should round-trip");

        assert_eq!(loaded.version, CACHE_VERSION);
        assert_eq!(loaded.len(), 1);
        assert_eq!(entry.etag.as_deref(), Some("\"etag-123\""));
        assert_eq!(entry.fetched_at, 1_700_000_000);
        assert_eq!(entry.data.releases[0].version, "1.2.3");
        assert_eq!(
            entry.data.last_commit.as_deref(),
            Some("2024-01-16T01:02:03Z")
        );
        assert!(cache_path.exists());
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_load_version_mismatch_returns_empty_cache() {
        let (_temp_dir, cache_path) = temp_cache_path("github-cache-version-mismatch");
        fs::write(
            &cache_path,
            r#"{
  "version": 999,
  "entries": {
    "owner/repo": {
      "data": {
        "releases": [],
        "stars": 1,
        "open_issues": 2,
        "license": "MIT",
        "last_commit": "2024-01-01T00:00:00Z"
      },
      "etag": "abc",
      "fetched_at": 123
    }
  }
}"#,
        )
        .unwrap();

        let loaded = GitHubCache::load_from_path(&cache_path);
        assert_eq!(loaded.version, CACHE_VERSION);
        assert!(loaded.is_empty());
    }
}
