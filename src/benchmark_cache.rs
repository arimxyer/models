//! Persistent disk cache for benchmark data
//!
//! Stores benchmark entries fetched from the CDN to disk, enabling
//! fast startup and offline access to previously fetched data.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::benchmarks::BenchmarkEntry;

const CACHE_VERSION: u32 = 1;
const CACHE_FILENAME: &str = "benchmarks-cache.json";
const CACHE_TTL_SECS: i64 = 6 * 60 * 60; // 6 hours

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCache {
    pub version: u32,
    pub entries: Vec<BenchmarkEntry>,
    pub etag: Option<String>,
    pub fetched_at: i64,
}

impl Default for BenchmarkCache {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkCache {
    pub fn new() -> Self {
        Self {
            version: CACHE_VERSION,
            entries: Vec::new(),
            etag: None,
            fetched_at: 0,
        }
    }

    fn cache_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("models").join(CACHE_FILENAME))
    }

    pub fn load() -> Self {
        Self::try_load().unwrap_or_default()
    }

    fn try_load() -> Result<Self> {
        let path =
            Self::cache_path().ok_or_else(|| anyhow::anyhow!("Could not determine config dir"))?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(&path)?;
        let cache: BenchmarkCache = serde_json::from_str(&contents)?;

        if cache.version != CACHE_VERSION {
            return Ok(Self::new());
        }

        Ok(cache)
    }

    pub fn save(&self) -> Result<()> {
        let path =
            Self::cache_path().ok_or_else(|| anyhow::anyhow!("Could not determine config dir"))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string(self)?;
        fs::write(&path, contents)?;

        Ok(())
    }

    pub fn is_fresh(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        now - self.fetched_at < CACHE_TTL_SECS
    }

    pub fn has_entries(&self) -> bool {
        !self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cache() {
        let cache = BenchmarkCache::new();
        assert_eq!(cache.version, CACHE_VERSION);
        assert!(cache.entries.is_empty());
        assert!(!cache.is_fresh());
    }

    #[test]
    fn test_fresh_cache() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cache = BenchmarkCache {
            version: CACHE_VERSION,
            entries: Vec::new(),
            etag: None,
            fetched_at: now,
        };
        assert!(cache.is_fresh());
    }

    #[test]
    fn test_stale_cache() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cache = BenchmarkCache {
            version: CACHE_VERSION,
            entries: Vec::new(),
            etag: None,
            fetched_at: now - CACHE_TTL_SECS - 1,
        };
        assert!(!cache.is_fresh());
    }
}
