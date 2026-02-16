use serde::{Deserialize, Serialize};

use crate::benchmark_cache::BenchmarkCache;

const BENCHMARKS_JSON: &str = include_str!("../data/benchmarks.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchmarkEntry {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub creator: String,
    #[serde(default)]
    pub creator_id: String,
    #[serde(default)]
    pub creator_name: String,
    #[serde(default)]
    pub release_date: Option<String>,
    pub intelligence_index: Option<f64>,
    pub coding_index: Option<f64>,
    pub math_index: Option<f64>,
    pub mmlu_pro: Option<f64>,
    pub gpqa: Option<f64>,
    pub hle: Option<f64>,
    pub livecodebench: Option<f64>,
    pub scicode: Option<f64>,
    pub ifbench: Option<f64>,
    pub lcr: Option<f64>,
    pub terminalbench_hard: Option<f64>,
    pub tau2: Option<f64>,
    pub math_500: Option<f64>,
    #[serde(default)]
    pub aime: Option<f64>,
    pub aime_25: Option<f64>,
    pub output_tps: Option<f64>,
    pub ttft: Option<f64>,
    #[serde(default)]
    pub ttfat: Option<f64>,
    pub price_input: Option<f64>,
    pub price_output: Option<f64>,
    pub price_blended: Option<f64>,
}

pub struct BenchmarkStore {
    entries: Vec<BenchmarkEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchmarkSchemaCoverage {
    pub has_ttfat: bool,
    pub has_ids: bool,
}

impl BenchmarkSchemaCoverage {
    pub fn from_entries(entries: &[BenchmarkEntry]) -> Self {
        Self {
            has_ttfat: entries.iter().any(|e| e.ttfat.is_some()),
            has_ids: entries
                .iter()
                .any(|e| !e.id.is_empty() && !e.creator_id.is_empty()),
        }
    }
}

/// Returns true when `candidate` satisfies the schema capabilities required by `baseline`.
/// This protects against stale payloads that deserialize but miss newly required fields.
pub fn benchmark_entries_compatible(
    candidate: &[BenchmarkEntry],
    baseline: &[BenchmarkEntry],
) -> bool {
    let candidate_cov = BenchmarkSchemaCoverage::from_entries(candidate);
    let baseline_cov = BenchmarkSchemaCoverage::from_entries(baseline);

    let ttfat_ok = !baseline_cov.has_ttfat || candidate_cov.has_ttfat;
    let ids_ok = !baseline_cov.has_ids || candidate_cov.has_ids;
    ttfat_ok && ids_ok
}

impl BenchmarkStore {
    pub fn entries(&self) -> &[BenchmarkEntry] {
        &self.entries
    }

    pub fn load() -> Self {
        let entries: Vec<BenchmarkEntry> =
            serde_json::from_str(BENCHMARKS_JSON).unwrap_or_default();
        Self { entries }
    }

    /// Create a store from runtime-fetched entries.
    pub fn from_entries(entries: Vec<BenchmarkEntry>) -> Self {
        Self { entries }
    }

    /// Load using cache if it has entries, otherwise fall back to embedded data.
    pub fn load_with_cache(cache: &BenchmarkCache) -> Self {
        if !cache.has_entries() {
            Self::load()
        } else {
            let embedded = Self::load();
            if !benchmark_entries_compatible(&cache.entries, &embedded.entries) {
                embedded
            } else {
                Self::from_entries(cache.entries.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_benchmarks() {
        let store = BenchmarkStore::load();
        assert!(!store.entries.is_empty());
    }

    #[test]
    fn test_ttfat_data_exists() {
        let store = BenchmarkStore::load();
        let count = store.entries().iter().filter(|e| e.ttfat.is_some()).count();
        assert!(
            count > 100,
            "Expected >100 entries with TTFAT data, got {count}"
        );
    }

    #[test]
    fn test_aime_data_exists() {
        let store = BenchmarkStore::load();
        let count = store.entries().iter().filter(|e| e.aime.is_some()).count();
        assert!(count > 0, "Expected some entries with AIME data, got 0");
    }

    #[test]
    fn test_ids_present() {
        let store = BenchmarkStore::load();
        let entry = &store.entries()[0];
        assert!(!entry.id.is_empty(), "Expected first entry to have an id");
        assert!(
            !entry.creator_id.is_empty(),
            "Expected first entry to have a creator_id"
        );
    }

    #[test]
    fn test_ttfat_survives_serialize_roundtrip() {
        // Simulate CDN JSON → deserialize → cache save → cache load
        let json = r#"[{
            "id": "abc-123",
            "name": "test-model",
            "slug": "test",
            "creator": "openai",
            "creator_id": "def-456",
            "creator_name": "OpenAI",
            "release_date": "2025-01-01",
            "intelligence_index": 50.0,
            "coding_index": null,
            "math_index": null,
            "mmlu_pro": null,
            "gpqa": null,
            "hle": null,
            "livecodebench": null,
            "scicode": null,
            "ifbench": null,
            "lcr": null,
            "terminalbench_hard": null,
            "tau2": null,
            "math_500": null,
            "aime": null,
            "aime_25": null,
            "output_tps": 100.0,
            "ttft": 0.5,
            "ttfat": 6.7,
            "price_input": 1.0,
            "price_output": 2.0,
            "price_blended": 1.25
        }]"#;

        // Step 1: Deserialize from CDN JSON (like benchmark_fetch.rs does)
        let entries: Vec<BenchmarkEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(
            entries[0].ttfat,
            Some(6.7),
            "ttfat lost during CDN deserialize"
        );

        // Step 2: Serialize to cache (like mod.rs cache.save() does)
        let cached = serde_json::to_string(&entries).unwrap();

        // Step 3: Deserialize from cache (like benchmark_cache.rs load does)
        let restored: Vec<BenchmarkEntry> = serde_json::from_str(&cached).unwrap();
        assert_eq!(
            restored[0].ttfat,
            Some(6.7),
            "ttfat lost during cache roundtrip"
        );
    }

    #[test]
    fn test_ttfat_from_embedded_data_first_entry() {
        // Directly verify the first entry in embedded data
        let store = BenchmarkStore::load();
        let first = &store.entries()[0];
        // The first entry (gpt-oss-120B) should have ttfat=6.748
        assert!(
            first.ttfat.is_some(),
            "First entry '{}' should have ttfat but got None",
            first.name
        );
    }

    #[test]
    fn test_load_with_cache_rejects_stale_schema_without_ttfat() {
        let cache = BenchmarkCache {
            version: crate::benchmark_cache::CACHE_VERSION,
            schema_version: crate::benchmark_cache::DATA_SCHEMA_VERSION,
            entries: vec![BenchmarkEntry {
                id: String::new(),
                name: "stale-cache-entry".to_string(),
                slug: "stale-cache-entry".to_string(),
                creator: "openai".to_string(),
                creator_id: String::new(),
                creator_name: "OpenAI".to_string(),
                release_date: Some("2025-01-01".to_string()),
                intelligence_index: Some(42.0),
                coding_index: Some(40.0),
                math_index: Some(39.0),
                mmlu_pro: Some(0.8),
                gpqa: Some(0.7),
                hle: Some(0.1),
                livecodebench: Some(0.6),
                scicode: Some(0.3),
                ifbench: Some(0.5),
                lcr: Some(0.4),
                terminalbench_hard: Some(0.2),
                tau2: Some(0.6),
                math_500: None,
                aime: None,
                aime_25: Some(0.4),
                output_tps: Some(120.0),
                ttft: Some(0.7),
                ttfat: None,
                price_input: Some(1.0),
                price_output: Some(2.0),
                price_blended: Some(1.25),
            }],
            etag: None,
            fetched_at: 0,
        };

        let store = BenchmarkStore::load_with_cache(&cache);
        assert!(
            store.entries().iter().any(|e| e.ttfat.is_some()),
            "Expected fallback to embedded data with TTFAT values"
        );
        assert_ne!(store.entries()[0].name, "stale-cache-entry");
    }

    #[test]
    fn test_load_with_cache_rejects_stale_schema_without_ids() {
        let cache = BenchmarkCache {
            version: crate::benchmark_cache::CACHE_VERSION,
            schema_version: crate::benchmark_cache::DATA_SCHEMA_VERSION,
            entries: vec![BenchmarkEntry {
                id: String::new(),
                name: "stale-cache-entry-without-ids".to_string(),
                slug: "stale-cache-entry-without-ids".to_string(),
                creator: "openai".to_string(),
                creator_id: String::new(),
                creator_name: "OpenAI".to_string(),
                release_date: Some("2025-01-01".to_string()),
                intelligence_index: Some(42.0),
                coding_index: Some(40.0),
                math_index: Some(39.0),
                mmlu_pro: Some(0.8),
                gpqa: Some(0.7),
                hle: Some(0.1),
                livecodebench: Some(0.6),
                scicode: Some(0.3),
                ifbench: Some(0.5),
                lcr: Some(0.4),
                terminalbench_hard: Some(0.2),
                tau2: Some(0.6),
                math_500: None,
                aime: None,
                aime_25: Some(0.4),
                output_tps: Some(120.0),
                ttft: Some(0.7),
                ttfat: Some(6.7),
                price_input: Some(1.0),
                price_output: Some(2.0),
                price_blended: Some(1.25),
            }],
            etag: None,
            fetched_at: 0,
        };

        let store = BenchmarkStore::load_with_cache(&cache);
        assert!(
            store
                .entries()
                .iter()
                .any(|e| !e.id.is_empty() && !e.creator_id.is_empty()),
            "Expected fallback to embedded data with id/creator_id values"
        );
        assert_ne!(store.entries()[0].name, "stale-cache-entry-without-ids");
    }

    #[test]
    fn test_benchmark_entries_compatible() {
        let baseline = vec![BenchmarkEntry {
            id: "id-1".to_string(),
            name: "baseline".to_string(),
            slug: "baseline".to_string(),
            creator: "openai".to_string(),
            creator_id: "creator-1".to_string(),
            creator_name: "OpenAI".to_string(),
            release_date: Some("2025-01-01".to_string()),
            intelligence_index: Some(1.0),
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
            ttfat: Some(1.2),
            price_input: None,
            price_output: None,
            price_blended: None,
        }];

        let candidate_missing_new_fields = vec![BenchmarkEntry {
            id: String::new(),
            name: "stale".to_string(),
            slug: "stale".to_string(),
            creator: "openai".to_string(),
            creator_id: String::new(),
            creator_name: "OpenAI".to_string(),
            release_date: Some("2025-01-01".to_string()),
            intelligence_index: Some(1.0),
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
            ttfat: None,
            price_input: None,
            price_output: None,
            price_blended: None,
        }];

        assert!(!benchmark_entries_compatible(
            &candidate_missing_new_fields,
            &baseline
        ));
        assert!(benchmark_entries_compatible(&baseline, &baseline));
    }
}
