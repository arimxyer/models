use regex::Regex;
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

        // Try matching with stripped names (removes provider prefixes, parenthetical
        // suffixes, etc.) — prefer non-reasoning variants as the default match.
        let stripped_name = strip_qualifiers(model_name);
        let stripped_id = strip_qualifiers(model_id);

        let stripped_matches = |e: &BenchmarkEntry| {
            let s_name = strip_qualifiers(&e.name);
            let s_slug = strip_qualifiers(&e.slug);
            s_name == stripped_name
                || s_name == stripped_id
                || s_slug == stripped_name
                || s_slug == stripped_id
        };

        // First pass: prefer non-reasoning entries
        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| !e.is_reasoning_variant() && stripped_matches(e))
        {
            return Some(entry);
        }

        // Second pass: accept reasoning entries if no non-reasoning match
        if let Some(entry) = self.entries.iter().find(|e| stripped_matches(e)) {
            return Some(entry);
        }

        // Final pass: sorted-token matching handles word order differences
        // e.g., "Claude Sonnet 4" (models.dev) vs "Claude 4 Sonnet" (AA)
        let sorted_name = sorted_tokens(model_name);
        let sorted_id = sorted_tokens(model_id);

        let sorted_matches = |e: &BenchmarkEntry| {
            let s_name = sorted_tokens(&e.name);
            let s_slug = sorted_tokens(&e.slug);
            s_name == sorted_name
                || s_name == sorted_id
                || s_slug == sorted_name
                || s_slug == sorted_id
        };

        if let Some(entry) = self
            .entries
            .iter()
            .find(|e| !e.is_reasoning_variant() && sorted_matches(e))
        {
            return Some(entry);
        }

        if let Some(entry) = self.entries.iter().find(|e| sorted_matches(e)) {
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

/// Strip qualifiers, split into tokens, sort alphabetically, and rejoin.
/// Handles word order differences: "Claude Sonnet 4" and "Claude 4 Sonnet" both
/// become "4 claude sonnet".
fn sorted_tokens(s: &str) -> String {
    let lower = s.to_lowercase();
    let re = Regex::new(
        r"^(?:anthropic|openai|google|meta|mistral|cohere|xai|amazon|microsoft|nvidia)\s*[:/]?\s*",
    )
    .unwrap();
    let stripped = re.replace(&lower, "");
    let re_parens = Regex::new(r"\s*\([^)]*\)").unwrap();
    let stripped = re_parens.replace_all(&stripped, "");
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

    // Strip provider prefixes like "Anthropic: ", "OpenAI: ", "OpenAI "
    let re = Regex::new(
        r"^(?:anthropic|openai|google|meta|mistral|cohere|xai|amazon|microsoft|nvidia)\s*[:/]?\s*",
    )
    .unwrap();
    let stripped = re.replace(&lower, "");

    // Strip all parenthetical content: "(latest)", "(US)", "(Non-reasoning)", etc.
    let re_parens = Regex::new(r"\s*\([^)]*\)").unwrap();
    let stripped = re_parens.replace_all(&stripped, "");

    // Strip trailing qualifiers that aren't in parens
    let stripped = stripped.trim();

    // Normalize: strip separators and trailing dates
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
    fn test_find_with_provider_prefix() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("claude-opus-4-5", "Anthropic: Claude Opus 4.5");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_with_region_suffix() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("claude-opus-4-6", "Claude Opus 4.6 (US)");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_prefers_non_reasoning() {
        let store = BenchmarkStore::load();
        let result = store.find_for_model("claude-sonnet-4", "Claude Sonnet 4");
        assert!(result.is_some());
        // Should match non-reasoning variant
        assert!(
            !result.unwrap().name.contains("Reasoning"),
            "Should prefer non-reasoning variant, got: {}",
            result.unwrap().name
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
