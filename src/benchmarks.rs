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
    fn test_ttfat_survives_serialize_roundtrip() {
        // Simulate CDN JSON -> deserialize -> cache save -> cache load
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

        let entries: Vec<BenchmarkEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(entries[0].ttfat, Some(6.7), "ttfat lost during deserialize");

        let cached = serde_json::to_string(&entries).unwrap();
        let restored: Vec<BenchmarkEntry> = serde_json::from_str(&cached).unwrap();
        assert_eq!(
            restored[0].ttfat,
            Some(6.7),
            "ttfat lost during cache roundtrip"
        );
    }

    #[test]
    fn test_compatible_rejects_candidate_missing_ttfat() {
        let baseline = vec![make_entry(|e| {
            e.id = "id-1".to_string();
            e.creator_id = "creator-1".to_string();
            e.ttfat = Some(1.2);
        })];
        let candidate = vec![make_entry(|_| {})]; // no ttfat, no ids

        assert!(!benchmark_entries_compatible(&candidate, &baseline));
    }

    #[test]
    fn test_compatible_rejects_candidate_missing_ids() {
        let baseline = vec![make_entry(|e| {
            e.id = "id-1".to_string();
            e.creator_id = "creator-1".to_string();
        })];
        let candidate = vec![make_entry(|_| {})]; // no ids

        assert!(!benchmark_entries_compatible(&candidate, &baseline));
    }

    #[test]
    fn test_compatible_accepts_matching_schema() {
        let baseline = vec![make_entry(|e| {
            e.id = "id-1".to_string();
            e.creator_id = "creator-1".to_string();
            e.ttfat = Some(1.2);
        })];

        assert!(benchmark_entries_compatible(&baseline, &baseline));
    }

    #[test]
    fn test_compatible_accepts_anything_with_empty_baseline() {
        // Empty baseline = no requirements = accept any candidate.
        // This is the first-launch scenario (no cache, no embedded data).
        let empty: Vec<BenchmarkEntry> = vec![];
        let candidate = vec![make_entry(|_| {})]; // incomplete but better than nothing

        assert!(benchmark_entries_compatible(&candidate, &empty));
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
