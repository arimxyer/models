use serde::{Deserialize, Serialize};

use crate::benchmark_cache::BenchmarkCache;

const BENCHMARKS_JSON: &str = include_str!("../data/benchmarks.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchmarkEntry {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub creator: String,
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
    pub aime_25: Option<f64>,
    pub output_tps: Option<f64>,
    pub ttft: Option<f64>,
    pub price_input: Option<f64>,
    pub price_output: Option<f64>,
    pub price_blended: Option<f64>,
}

pub struct BenchmarkStore {
    entries: Vec<BenchmarkEntry>,
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
        if cache.has_entries() {
            Self::from_entries(cache.entries.clone())
        } else {
            Self::load()
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
}
