use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

const BENCHMARKS_JSON: &str = include_str!("../data/benchmarks.json");

/// Regex to strip provider prefixes like "Anthropic: ", "OpenAI: ", etc.
static RE_PROVIDER_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:anthropic|openai|google|meta|mistral|cohere|xai|amazon|microsoft|nvidia)\s*[:/]?\s*",
    )
    .unwrap()
});

/// Regex to strip parenthetical content like "(latest)", "(US)", etc.
static RE_PARENS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\([^)]*\)").unwrap());

/// Manual overrides for edge cases where algorithmic matching fails.
/// Maps normalized model_id -> AA entry slug.
static MANUAL_OVERRIDES: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    HashMap::from([
        // Example: ("some-edge-case-id", "correct-aa-slug"),
    ])
});

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
    pub scicode: Option<f64>,
    pub ifbench: Option<f64>,
    pub lcr: Option<f64>,
    pub terminalbench_hard: Option<f64>,
    pub tau2: Option<f64>,
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

    /// Returns true if this is a reasoning/thinking variant.
    fn is_reasoning_variant(&self) -> bool {
        let lower = self.name.to_lowercase();
        (lower.contains("reasoning") && !lower.contains("non-reasoning"))
            || lower.contains("thinking")
            || lower.contains("adaptive")
    }
}

pub struct BenchmarkStore {
    entries: Vec<BenchmarkEntry>,
    /// Tier 1: normalized slug -> entry indices
    by_slug: HashMap<String, Vec<usize>>,
    /// Tier 2: normalized name -> entry indices
    by_name: HashMap<String, Vec<usize>>,
    /// Tier 3: stripped qualifiers -> entry indices (may have multiple variants)
    by_stripped: HashMap<String, Vec<usize>>,
    /// Tier 4: sorted tokens -> entry indices (handles word order differences)
    by_sorted: HashMap<String, Vec<usize>>,
}

impl BenchmarkStore {
    pub fn load() -> Self {
        let entries: Vec<BenchmarkEntry> =
            serde_json::from_str(BENCHMARKS_JSON).unwrap_or_default();

        let mut by_slug: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_stripped: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_sorted: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, entry) in entries.iter().enumerate() {
            // Tier 1: normalized slug
            by_slug.entry(normalize(&entry.slug)).or_default().push(idx);

            // Tier 2: normalized name
            by_name.entry(normalize(&entry.name)).or_default().push(idx);

            // Tier 3: stripped qualifiers (both name and slug, deduped)
            let stripped_name = strip_qualifiers(&entry.name);
            let stripped_slug = strip_qualifiers(&entry.slug);
            by_stripped
                .entry(stripped_name.clone())
                .or_default()
                .push(idx);
            if stripped_slug != stripped_name {
                by_stripped.entry(stripped_slug).or_default().push(idx);
            }

            // Tier 4: sorted tokens (both name and slug, deduped)
            let sorted_name = sorted_tokens(&entry.name);
            let sorted_slug = sorted_tokens(&entry.slug);
            by_sorted.entry(sorted_name.clone()).or_default().push(idx);
            if sorted_slug != sorted_name {
                by_sorted.entry(sorted_slug).or_default().push(idx);
            }
        }

        Self {
            entries,
            by_slug,
            by_name,
            by_stripped,
            by_sorted,
        }
    }

    /// Find benchmark data for a model by matching against model ID and name.
    /// The `reasoning` flag (from models.dev) selects the appropriate AA variant
    /// when both reasoning and non-reasoning entries exist for the same model.
    pub fn find_for_model(
        &self,
        model_id: &str,
        model_name: &str,
        reasoning: bool,
    ) -> Option<&BenchmarkEntry> {
        // Tier 0: Manual overrides (checked first, for known edge cases)
        if let Some(&slug) = MANUAL_OVERRIDES.get(model_id) {
            let key = normalize(slug);
            if let Some(indices) = self.by_slug.get(&key) {
                if indices.len() > 1 {
                    return self.pick_variant(indices, reasoning);
                }
                return Some(&self.entries[indices[0]]);
            }
        }

        let normalized_id = normalize(model_id);
        let normalized_name = normalize(model_name);
        let stripped_name = strip_qualifiers(model_name);
        let stripped_id = strip_qualifiers(model_id);
        let sorted_name = sorted_tokens(model_name);
        let sorted_id = sorted_tokens(model_id);

        // Search keys in priority order, paired with their index map.
        // When a tier has multiple candidates, pick_variant selects the right one.
        // When a tier has a single candidate whose reasoning flag doesn't match,
        // save it as fallback and keep searching — a lower tier may group both
        // reasoning variants under the same key and select correctly.
        let searches: &[(&HashMap<String, Vec<usize>>, &str)] = &[
            (&self.by_slug, &normalized_id),
            (&self.by_name, &normalized_name),
            (&self.by_name, &normalized_id),
            (&self.by_stripped, &stripped_name),
            (&self.by_stripped, &stripped_id),
            (&self.by_sorted, &sorted_name),
            (&self.by_sorted, &sorted_id),
        ];

        let mut fallback: Option<&BenchmarkEntry> = None;

        for (index, key) in searches {
            if let Some(indices) = index.get(*key) {
                if indices.len() > 1 {
                    // Multiple candidates — pick_variant handles reasoning selection
                    return self.pick_variant(indices, reasoning);
                }
                // Single candidate
                let entry = &self.entries[indices[0]];
                if entry.is_reasoning_variant() == reasoning {
                    return Some(entry);
                }
                // Reasoning mismatch — save as fallback, keep searching
                if fallback.is_none() {
                    fallback = Some(entry);
                }
            }
        }

        fallback
    }

    /// From a set of candidate entry indices, pick the best variant based on the
    /// `reasoning` flag from models.dev.
    fn pick_variant(&self, indices: &[usize], reasoning: bool) -> Option<&BenchmarkEntry> {
        if indices.len() == 1 {
            return Some(&self.entries[indices[0]]);
        }

        // Try to match the reasoning preference first
        if let Some(&idx) = indices
            .iter()
            .find(|&&idx| self.entries[idx].is_reasoning_variant() == reasoning)
        {
            return Some(&self.entries[idx]);
        }

        // Fall back to non-reasoning (the "standard" variant)
        if let Some(&idx) = indices
            .iter()
            .find(|&&idx| !self.entries[idx].is_reasoning_variant())
        {
            return Some(&self.entries[idx]);
        }

        // Last resort: return the first
        Some(&self.entries[indices[0]])
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

/// Strip qualifiers, split into tokens, sort alphabetically, and rejoin.
/// Handles word order differences: "Claude Sonnet 4" and "Claude 4 Sonnet" both
/// become "4 claude sonnet".
fn sorted_tokens(s: &str) -> String {
    let lower = s.to_lowercase();
    let stripped = RE_PROVIDER_PREFIX.replace(&lower, "");
    let stripped = RE_PARENS.replace_all(&stripped, "");
    let mut tokens: Vec<&str> = stripped
        .split(['-', '_', '.', ' '])
        .filter(|t| !t.is_empty())
        .collect();
    tokens.sort();
    tokens.join(" ")
}

/// Strip provider prefixes, parenthetical suffixes, and normalize for matching.
/// "Anthropic: Claude Opus 4.5 (latest)" -> "claudeopus45"
/// "Claude 3.5 Sonnet (Oct '24)" -> "claude35sonnet"
/// "Claude 4 Opus (Non-reasoning)" -> "claude4opus"
fn strip_qualifiers(s: &str) -> String {
    let lower = s.to_lowercase();
    let stripped = RE_PROVIDER_PREFIX.replace(&lower, "");
    let stripped = RE_PARENS.replace_all(&stripped, "");
    let stripped = stripped.trim();
    normalize(stripped)
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
    fn test_index_is_populated() {
        let store = BenchmarkStore::load();
        assert!(!store.by_slug.is_empty());
        assert!(!store.by_name.is_empty());
        assert!(!store.by_stripped.is_empty());
        assert!(!store.by_sorted.is_empty());
    }

    #[test]
    fn test_find_gpt4o() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("gpt-4o", "GPT-4o", false);
        assert!(result.is_some());
        let entry = result.unwrap();
        assert!(entry.name.contains("GPT-4o"));
    }

    #[test]
    fn test_find_gpt4o_with_date() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("gpt-4o-2024-08-06", "GPT-4o", false);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_claude() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model(
            "claude-3-5-sonnet-20241022",
            "Claude 3.5 Sonnet (New)",
            false,
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_find_nonexistent() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("some-unknown-model", "Unknown Model", false);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_with_provider_prefix() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("claude-opus-4-5", "Anthropic: Claude Opus 4.5", false);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_with_region_suffix() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("claude-opus-4-6", "Claude Opus 4.6 (US)", false);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_prefers_non_reasoning() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("claude-sonnet-4", "Claude Sonnet 4", false);
        assert!(result.is_some());
        let entry = result.unwrap();
        assert!(
            !entry.name.contains("Reasoning"),
            "Should prefer non-reasoning variant, got: {}",
            entry.name
        );
    }

    #[test]
    fn test_find_reasoning_variant_when_requested() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("claude-sonnet-4", "Claude Sonnet 4", true);
        assert!(result.is_some());
        let entry = result.unwrap();
        assert!(
            entry.is_reasoning_variant(),
            "Should prefer reasoning variant when reasoning=true, got: {}",
            entry.name
        );
    }

    #[test]
    fn test_slug_match_respects_reasoning_flag() {
        let store = BenchmarkStore::load();
        // Same model_id (exact slug match), different reasoning flag
        let non_reasoning = store.find_for_model("claude-opus-4-6", "Claude Opus 4.6", false);
        let reasoning = store.find_for_model("claude-opus-4-6", "Claude Opus 4.6", true);
        assert!(non_reasoning.is_some());
        assert!(reasoning.is_some());
        // They should resolve to different AA entries
        assert_ne!(
            non_reasoning.unwrap().name,
            reasoning.unwrap().name,
            "reasoning=true and reasoning=false should return different variants"
        );
        assert!(
            !non_reasoning.unwrap().is_reasoning_variant(),
            "reasoning=false should get non-reasoning variant, got: {}",
            non_reasoning.unwrap().name
        );
        assert!(
            reasoning.unwrap().is_reasoning_variant(),
            "reasoning=true should get reasoning variant, got: {}",
            reasoning.unwrap().name
        );
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("GPT-4o"), "gpt4o");
        assert_eq!(normalize("claude-3-5-sonnet-20241022"), "claude35sonnet");
        assert_eq!(normalize("Llama 3.1 405B"), "llama31405b");
        assert_eq!(normalize("o1"), "o1");
        assert_eq!(normalize("o3-mini"), "o3mini");
    }

    #[test]
    fn test_strip_qualifiers() {
        assert_eq!(
            strip_qualifiers("Anthropic: Claude Opus 4.5 (latest)"),
            "claudeopus45"
        );
        assert_eq!(
            strip_qualifiers("Claude 3.5 Sonnet (Oct '24)"),
            "claude35sonnet"
        );
        assert_eq!(strip_qualifiers("OpenAI GPT-4o"), "gpt4o");
        assert_eq!(strip_qualifiers("Claude Opus 4.6 (US)"), "claudeopus46");
    }
}
