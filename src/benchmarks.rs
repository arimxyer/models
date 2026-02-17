use serde::{Deserialize, Serialize};

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

impl BenchmarkStore {
    pub fn entries(&self) -> &[BenchmarkEntry] {
        &self.entries
    }

    /// Create an empty store (no benchmark data loaded yet).
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create a store from runtime-fetched or cached entries.
    pub fn from_entries(entries: Vec<BenchmarkEntry>) -> Self {
        Self { entries }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(overrides: impl FnOnce(&mut BenchmarkEntry)) -> BenchmarkEntry {
        let mut entry = BenchmarkEntry {
            id: String::new(),
            name: "test".to_string(),
            slug: "test".to_string(),
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
        };
        overrides(&mut entry);
        entry
    }

    #[test]
    fn test_empty_store() {
        let store = BenchmarkStore::empty();
        assert!(store.entries().is_empty());
    }

    #[test]
    fn test_from_entries() {
        let entries = vec![make_entry(|e| {
            e.name = "cached-model".to_string();
            e.ttfat = Some(5.0);
        })];
        let store = BenchmarkStore::from_entries(entries);
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.entries()[0].name, "cached-model");
        assert_eq!(store.entries()[0].ttfat, Some(5.0));
    }
}
