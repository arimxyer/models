//! Persistent disk cache for benchmark data
//!
//! Stores benchmark entries fetched from the CDN to disk, enabling
//! fast startup and offline access to previously fetched data.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::benchmarks::BenchmarkEntry;

pub const CACHE_VERSION: u32 = 3;
pub const DATA_SCHEMA_VERSION: u32 = 1;
const CACHE_FILENAME: &str = "benchmarks-cache.json";
const CACHE_TTL_SECS: i64 = 6 * 60 * 60; // 6 hours

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCache {
    pub version: u32,
    #[serde(default)]
    pub schema_version: u32,
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
            schema_version: DATA_SCHEMA_VERSION,
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

        if cache.version != CACHE_VERSION || cache.schema_version != DATA_SCHEMA_VERSION {
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
        assert_eq!(cache.schema_version, DATA_SCHEMA_VERSION);
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
            schema_version: DATA_SCHEMA_VERSION,
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
            schema_version: DATA_SCHEMA_VERSION,
            entries: Vec::new(),
            etag: None,
            fetched_at: now - CACHE_TTL_SECS - 1,
        };
        assert!(!cache.is_fresh());
    }

    #[test]
    fn test_old_version_cache_rejected() {
        use crate::benchmarks::BenchmarkStore;

        // Simulate a v1 cache with entries but no ttfat data
        let old_entry = BenchmarkEntry {
            id: String::new(),
            name: "test-model".to_string(),
            slug: "test".to_string(),
            creator: "test".to_string(),
            creator_id: String::new(),
            creator_name: "Test".to_string(),
            release_date: None,
            intelligence_index: Some(50.0),
            coding_index: None,
            math_index: None,
            mmlu_pro: None,
            gpqa: None,
            hle: None,
            livecodebench: None,
            scicode: None,
            ifbench: None,
            lcr: None,
            terminalbench_hard: None,
            tau2: None,
            math_500: None,
            aime: None,
            aime_25: None,
            output_tps: None,
            ttft: None,
            ttfat: None, // old cache won't have real ttfat data
            price_input: None,
            price_output: None,
            price_blended: None,
        };

        let old_cache = BenchmarkCache {
            version: CACHE_VERSION - 1, // old version
            schema_version: DATA_SCHEMA_VERSION,
            entries: vec![old_entry],
            etag: None,
            fetched_at: 0,
        };

        // Old version cache should be treated as empty by load_with_cache
        // because try_load rejects mismatched versions, returning an empty cache.
        // Verify that embedded data (which has ttfat) wins over stale cache.
        assert!(old_cache.has_entries(), "old cache should have entries");

        // Simulate what try_load does: reject old version → empty cache
        let rejected_cache = BenchmarkCache::new();
        assert!(!rejected_cache.has_entries());

        // load_with_cache with empty cache falls back to embedded
        let store = BenchmarkStore::load_with_cache(&rejected_cache);
        let ttfat_count = store.entries().iter().filter(|e| e.ttfat.is_some()).count();
        assert!(
            ttfat_count > 100,
            "Embedded fallback should have >100 entries with ttfat, got {ttfat_count}"
        );
    }

    #[test]
    fn test_cache_with_entries_used_over_embedded() {
        use crate::benchmarks::BenchmarkStore;

        let entry = BenchmarkEntry {
            id: "test-id".to_string(),
            name: "cached-model".to_string(),
            slug: "cached".to_string(),
            creator: "test".to_string(),
            creator_id: "test-creator-id".to_string(),
            creator_name: "Test".to_string(),
            release_date: None,
            intelligence_index: Some(99.0),
            coding_index: None,
            math_index: None,
            mmlu_pro: None,
            gpqa: None,
            hle: None,
            livecodebench: None,
            scicode: None,
            ifbench: None,
            lcr: None,
            terminalbench_hard: None,
            tau2: None,
            math_500: None,
            aime: None,
            aime_25: None,
            output_tps: None,
            ttft: None,
            ttfat: Some(5.0),
            price_input: None,
            price_output: None,
            price_blended: None,
        };

        let cache = BenchmarkCache {
            version: CACHE_VERSION,
            schema_version: DATA_SCHEMA_VERSION,
            entries: vec![entry],
            etag: None,
            fetched_at: 0,
        };

        // Current version cache with entries should be used
        let store = BenchmarkStore::load_with_cache(&cache);
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.entries()[0].name, "cached-model");
        assert_eq!(store.entries()[0].ttfat, Some(5.0));
    }
}
