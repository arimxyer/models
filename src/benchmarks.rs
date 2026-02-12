use serde::Deserialize;

const BENCHMARKS_JSON: &str = include_str!("../data/benchmarks.json");

#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkEntry {
    pub name: String,
    pub slug: String,
    pub intelligence_index: Option<f64>,
    pub coding_index: Option<f64>,
    pub math_index: Option<f64>,
    pub mmlu_pro: Option<f64>,
    pub gpqa: Option<f64>,
    pub hle: Option<f64>,
    pub livecodebench: Option<f64>,
    pub output_tps: Option<f64>,
    pub ttft: Option<f64>,
}

impl BenchmarkEntry {
    /// Returns true if the entry has at least one benchmark score.
    pub fn has_any_score(&self) -> bool {
        self.intelligence_index.is_some()
            || self.coding_index.is_some()
            || self.math_index.is_some()
            || self.mmlu_pro.is_some()
            || self.gpqa.is_some()
    }
}

pub struct BenchmarkStore {
    entries: Vec<BenchmarkEntry>,
}

impl BenchmarkStore {
    pub fn load() -> Self {
        let entries: Vec<BenchmarkEntry> =
            serde_json::from_str(BENCHMARKS_JSON).unwrap_or_default();
        Self { entries }
    }

    /// Find benchmark data for a model by matching against model ID and name.
    pub fn find_for_model(&self, model_id: &str, model_name: &str) -> Option<&BenchmarkEntry> {
        let normalized_id = normalize(model_id);
        let normalized_name = normalize(model_name);

        // Try exact slug match first (most reliable)
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| normalize(&e.slug) == normalized_id)
        {
            return Some(entry);
        }

        // Try matching normalized model name against entry name
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| normalize(&e.name) == normalized_name)
        {
            return Some(entry);
        }

        // Try matching normalized model ID against entry name
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| normalize(&e.name) == normalized_id)
        {
            return Some(entry);
        }

        // Try slug substring matching (model ID contains the slug).
        // Require slug to be at least 4 chars to avoid spurious matches.
        if let Some(entry) = self.entries.iter().find(|e| {
            let norm_slug = normalize(&e.slug);
            norm_slug.len() >= 4 && normalized_id.contains(&norm_slug)
        }) {
            return Some(entry);
        }

        None
    }
}

/// Normalize a string for fuzzy matching: lowercase, strip common separators,
/// remove version date suffixes like "20241022".
fn normalize(s: &str) -> String {
    let base = s.to_lowercase().replace(['-', '_', '.', ' '], "");
    // Only strip trailing digits if they look like a date (8+ digits)
    let trailing_digits = base
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .count();
    if trailing_digits >= 8 {
        base[..base.len() - trailing_digits].to_string()
    } else {
        base
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
    fn test_find_gpt4o() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("gpt-4o", "GPT-4o");
        assert!(result.is_some());
        let entry = result.unwrap();
        assert!(entry.name.contains("GPT-4o"));
    }

    #[test]
    fn test_find_gpt4o_with_date() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("gpt-4o-2024-08-06", "GPT-4o");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_claude() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet (New)");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_nonexistent() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("some-unknown-model", "Unknown Model");
        assert!(result.is_none());
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("GPT-4o"), "gpt4o");
        assert_eq!(normalize("claude-3-5-sonnet-20241022"), "claude35sonnet");
        assert_eq!(normalize("Llama 3.1 405B"), "llama31405b");
        // Short trailing digits should NOT be stripped (model identifiers like o1, o3)
        assert_eq!(normalize("o1"), "o1");
        assert_eq!(normalize("o3-mini"), "o3mini");
    }
}
