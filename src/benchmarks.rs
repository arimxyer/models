use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
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

/// Regex to strip reasoning/thinking variant suffixes for fuzzy tokenization.
static RE_VARIANT_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[-\s](?:non[-\s]?reasoning|reasoning|thinking|adaptive)\b").unwrap()
});

/// Brand tokens used for cross-brand filtering in fuzzy matching.
/// If both query and entry contain brand tokens, at least one must overlap.
static BRAND_TOKENS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    HashSet::from([
        "claude",
        "gpt",
        "gemini",
        "gemma",
        "llama",
        "mistral",
        "qwen",
        "deepseek",
        "grok",
        "phi",
        "nova",
        "command",
        "nemotron",
        "glm",
        "jamba",
        "dbrx",
        "falcon",
        "yi",
        "internlm",
        "minimax",
        "aya",
        "granite",
        "starcoder",
        "codestral",
        "devstral",
        "pixtral",
        "magistral",
        "ministral",
        "o1",
        "o3",
        "o4",
        "kimi",
        "mimo",
        "exaone",
        "mixtral",
        "titan",
    ])
});

/// Maps AA `creator` field to the brand tokens that creator owns.
/// Used for authoritative cross-brand filtering in Tier 5 fuzzy matching.
static CREATOR_BRANDS: LazyLock<HashMap<&str, &[&str]>> = LazyLock::new(|| {
    HashMap::from([
        ("openai", ["gpt", "o1", "o3", "o4"].as_slice()),
        ("anthropic", &["claude"]),
        ("google", &["gemini", "gemma"]),
        ("meta", &["llama"]),
        (
            "mistral",
            &[
                "mistral",
                "codestral",
                "devstral",
                "pixtral",
                "magistral",
                "ministral",
                "mixtral",
            ],
        ),
        ("alibaba", &["qwen"]),
        ("xai", &["grok"]),
        ("deepseek", &["deepseek"]),
        ("microsoft", &["phi"]),
        ("aws", &["nova", "titan"]),
        ("cohere", &["command", "aya"]),
        ("nvidia", &["nemotron"]),
        ("ai21-labs", &["jamba"]),
        ("databricks", &["dbrx"]),
        ("tii-uae", &["falcon"]),
        ("kimi", &["kimi"]),
        ("xiaomi", &["mimo"]),
        ("lg", &["exaone"]),
        ("ibm", &["granite"]),
        ("minimax", &["minimax"]),
    ])
});

/// Tokens that indicate a non-LLM model (embeddings, code-gen, image-gen).
/// If the query contains any of these and the AA entry does not, skip the match.
/// Defense-in-depth for cases where modalities data is missing or incorrect.
static NON_LLM_TOKENS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    HashSet::from([
        "embed",
        "embedding",
        "bge",
        "e5",
        "gte",
        "nomic",
        "rerank",
        "reranker",
    ])
});

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
    #[serde(default)]
    pub creator: String,
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
    /// Tier 5 (fuzzy): IDF weights for each token in the AA corpus
    idf: HashMap<String, f64>,
    /// Tier 5 (fuzzy): pre-tokenized AA entries
    aa_tokens: Vec<HashSet<String>>,
    /// Tier 5 (fuzzy): pre-computed L2 norms for each AA entry's IDF vector
    aa_norms: Vec<f64>,
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

        // Tier 5: Build TF-IDF index from AA corpus
        let aa_tokens: Vec<HashSet<String>> = entries
            .iter()
            .map(|e| {
                fuzzy_tokenize(&e.name)
                    .union(&fuzzy_tokenize(&e.slug))
                    .cloned()
                    .collect()
            })
            .collect();

        let n = entries.len() as f64;
        let mut df: HashMap<String, usize> = HashMap::new();
        for tokens in &aa_tokens {
            for t in tokens {
                *df.entry(t.clone()).or_default() += 1;
            }
        }
        let idf: HashMap<String, f64> = df
            .into_iter()
            .map(|(t, count)| (t, (n / count as f64).ln()))
            .collect();

        let aa_norms: Vec<f64> = aa_tokens
            .iter()
            .map(|toks| {
                toks.iter()
                    .filter_map(|t| idf.get(t))
                    .map(|w| w * w)
                    .sum::<f64>()
                    .sqrt()
            })
            .collect();

        Self {
            entries,
            by_slug,
            by_name,
            by_stripped,
            by_sorted,
            idf,
            aa_tokens,
            aa_norms,
        }
    }

    /// Find benchmark data for a text-output model. Returns `None` for non-text
    /// models (image gen, video gen, embeddings) without attempting any matching.
    pub fn find_for_text_model(
        &self,
        model_id: &str,
        model: &crate::data::Model,
    ) -> Option<&BenchmarkEntry> {
        if !model.is_text_model() {
            return None;
        }
        self.find_for_model(model_id, &model.name, model.reasoning)
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

        // Tier 5: Brand-anchored TF-IDF cosine similarity (fuzzy fallback)
        if let Some(entry) = self.find_fuzzy(model_id, model_name, reasoning) {
            return Some(entry);
        }

        fallback
    }

    /// Tier 5: Find the best fuzzy match using TF-IDF cosine similarity.
    /// Filters out cross-brand matches to avoid false positives.
    fn find_fuzzy(
        &self,
        model_id: &str,
        model_name: &str,
        reasoning: bool,
    ) -> Option<&BenchmarkEntry> {
        const THRESHOLD: f64 = 0.65;

        let query_tokens: HashSet<String> = fuzzy_tokenize(model_id)
            .union(&fuzzy_tokenize(model_name))
            .cloned()
            .collect();

        // Early exit: if query contains non-LLM tokens, skip fuzzy matching entirely
        if query_tokens
            .iter()
            .any(|t| NON_LLM_TOKENS.contains(t.as_str()))
        {
            return None;
        }

        let query_norm: f64 = query_tokens
            .iter()
            .filter_map(|t| self.idf.get(t))
            .map(|w| w * w)
            .sum::<f64>()
            .sqrt();

        if query_norm == 0.0 {
            return None;
        }

        let mut best: Option<(f64, usize)> = None;

        for (idx, aa_toks) in self.aa_tokens.iter().enumerate() {
            if self.aa_norms[idx] == 0.0 {
                continue;
            }

            // Creator-anchored filter: use structured AA creator data when available,
            // fall back to heuristic brand token matching otherwise.
            let entry_creator = &self.entries[idx].creator;
            if !creator_compatible(&query_tokens, entry_creator, aa_toks) {
                continue;
            }

            // Scaled shared-token minimum: require at least 2 shared tokens, or half
            // the smaller set size, whichever is greater. Prevents sparse matches on
            // long model names where 2 tokens is insufficient.
            let shared_count = query_tokens.intersection(aa_toks).count();
            let min_shared = 2.max(query_tokens.len().min(aa_toks.len()) / 2);
            if shared_count < min_shared {
                continue;
            }

            let dot: f64 = query_tokens
                .intersection(aa_toks)
                .filter_map(|t| self.idf.get(t.as_str()))
                .map(|w| w * w)
                .sum();
            let cos = dot / (query_norm * self.aa_norms[idx]);

            if cos >= THRESHOLD {
                if let Some((best_score, _)) = best {
                    if cos > best_score {
                        best = Some((cos, idx));
                    }
                } else {
                    best = Some((cos, idx));
                }
            }
        }

        let (_, best_idx) = best?;
        let entry = &self.entries[best_idx];

        // If the best match's reasoning flag matches, return it directly.
        // Otherwise, check if there's a variant of the same model with matching reasoning.
        if entry.is_reasoning_variant() == reasoning {
            return Some(entry);
        }

        // Look for a sibling variant with matching reasoning (same stripped name)
        let stripped = strip_qualifiers(&entry.name);
        if let Some(indices) = self.by_stripped.get(&stripped) {
            if indices.len() > 1 {
                return self.pick_variant(indices, reasoning);
            }
        }

        Some(entry)
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

/// Tokenize a string for TF-IDF similarity: lowercase, strip provider prefixes,
/// parenthetical content, variant suffixes, and date-like tokens (6+ digits).
/// Regex to extract tokens while preserving version numbers (e.g. "3.5", "4.1").
/// Matches either a digit-dot-digit sequence or a run of non-separator characters.
static RE_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d+(?:\.\d+)+|[^\-_.\s:/]+").unwrap());

fn fuzzy_tokenize(s: &str) -> HashSet<String> {
    let lower = s.to_lowercase();
    let stripped = RE_PROVIDER_PREFIX.replace(&lower, "");
    let stripped = RE_PARENS.replace_all(&stripped, "");
    let stripped = RE_VARIANT_SUFFIX.replace_all(&stripped, "");
    RE_TOKEN
        .find_iter(&stripped)
        .map(|m| m.as_str())
        .filter(|t| !t.is_empty())
        .filter(|t| !(t.len() >= 6 && t.chars().all(|c| c.is_ascii_digit() || c == '.')))
        .map(|t| t.to_string())
        .collect()
}

/// Returns true if a query is brand-compatible with an AA entry.
/// Uses the structured `creator` field when available (authoritative),
/// falling back to heuristic brand-token overlap otherwise.
fn creator_compatible(
    query: &HashSet<String>,
    creator: &str,
    entry_tokens: &HashSet<String>,
) -> bool {
    let q_brands: HashSet<&str> = query
        .iter()
        .filter(|t| BRAND_TOKENS.contains(t.as_str()))
        .map(|s| s.as_str())
        .collect();

    // If query has no recognized brand tokens, allow match (unknown model)
    if q_brands.is_empty() {
        return true;
    }

    // Prefer structured creator data when available
    if let Some(creator_brand_list) = CREATOR_BRANDS.get(creator) {
        // Query must contain at least one brand token owned by this creator
        return q_brands.iter().any(|b| creator_brand_list.contains(b));
    }

    // Fallback: heuristic brand-token overlap (for creators not in CREATOR_BRANDS)
    let e_brands: HashSet<&str> = entry_tokens
        .iter()
        .filter(|t| BRAND_TOKENS.contains(t.as_str()))
        .map(|s| s.as_str())
        .collect();
    if e_brands.is_empty() {
        return true;
    }
    !q_brands.is_disjoint(&e_brands)
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
    fn test_fuzzy_fallback() {
        let store = BenchmarkStore::load();
        // Model with extra qualifiers that exact tiers won't match,
        // but fuzzy should find via token similarity
        let result = store.find_for_model(
            "llama-4-maverick-17b-128e-instruct-fp8",
            "Llama 4 Maverick 17B 128E Instruct FP8",
            false,
        );
        assert!(
            result.is_some(),
            "fuzzy should match Llama 4 Maverick variant"
        );
        let entry = result.unwrap();
        assert!(
            entry.name.contains("Llama 4 Maverick"),
            "expected Llama 4 Maverick, got: {}",
            entry.name
        );
    }

    #[test]
    fn test_fuzzy_rejects_cross_brand() {
        let store = BenchmarkStore::load();
        // "BGE Large EN v1.5" should NOT match "Mistral Large" via fuzzy
        let result = store.find_for_model("bge-large-en-v1.5", "BGE Large EN v1.5", false);
        if let Some(entry) = result {
            assert!(
                !entry.name.contains("Mistral"),
                "cross-brand match should be prevented, got: {}",
                entry.name
            );
        }
    }

    /// Dump all Tier 5 (fuzzy) matches from live models.dev data for manual review.
    /// Run with: cargo test verify_fuzzy_matches -- --ignored --nocapture
    #[test]
    #[ignore]
    fn verify_fuzzy_matches() {
        let store = BenchmarkStore::load();
        let providers: crate::data::ProvidersMap =
            reqwest::blocking::get("https://models.dev/api.json")
                .unwrap()
                .json()
                .unwrap();

        // Build a set of models matched by exact tiers (1-4) only,
        // by temporarily creating a store without fuzzy capability.
        // Instead, we detect fuzzy matches by checking if exact tiers miss but full pipeline hits.
        let mut fuzzy_matches: Vec<(&str, &str, &str, &BenchmarkEntry, f64)> = Vec::new();
        let mut exact_count = 0;
        let mut fuzzy_count = 0;
        let mut total = 0;

        for (provider_id, provider) in &providers {
            for (model_id, model) in &provider.models {
                total += 1;

                // Check if full pipeline (with fuzzy) finds a match
                let full_match = store.find_for_model(model_id, &model.name, model.reasoning);
                if full_match.is_none() {
                    continue;
                }
                let entry = full_match.unwrap();

                // Check if exact tiers alone would have found it
                let normalized_id = normalize(model_id);
                let normalized_name = normalize(&model.name);
                let stripped_name = strip_qualifiers(&model.name);
                let stripped_id = strip_qualifiers(model_id);
                let sorted_name = sorted_tokens(&model.name);
                let sorted_id = sorted_tokens(model_id);

                let exact_keys: Vec<(&HashMap<String, Vec<usize>>, &str)> = vec![
                    (&store.by_slug, &normalized_id),
                    (&store.by_name, &normalized_name),
                    (&store.by_name, &normalized_id),
                    (&store.by_stripped, &stripped_name),
                    (&store.by_stripped, &stripped_id),
                    (&store.by_sorted, &sorted_name),
                    (&store.by_sorted, &sorted_id),
                ];

                let found_exact = exact_keys
                    .iter()
                    .any(|(index, key)| index.contains_key(*key));
                if found_exact {
                    exact_count += 1;
                } else {
                    fuzzy_count += 1;

                    // Compute the cosine score for this match
                    let query_tokens: HashSet<String> = fuzzy_tokenize(model_id)
                        .union(&fuzzy_tokenize(&model.name))
                        .cloned()
                        .collect();
                    let query_norm: f64 = query_tokens
                        .iter()
                        .filter_map(|t| store.idf.get(t))
                        .map(|w| w * w)
                        .sum::<f64>()
                        .sqrt();

                    let aa_idx = store
                        .entries
                        .iter()
                        .position(|e| std::ptr::eq(e, entry))
                        .unwrap_or(0);

                    let cos = if query_norm > 0.0 && store.aa_norms[aa_idx] > 0.0 {
                        let dot: f64 = query_tokens
                            .intersection(&store.aa_tokens[aa_idx])
                            .filter_map(|t| store.idf.get(t))
                            .map(|w| w * w)
                            .sum();
                        dot / (query_norm * store.aa_norms[aa_idx])
                    } else {
                        0.0
                    };

                    fuzzy_matches.push((provider_id, model_id, &model.name, entry, cos));
                }
            }
        }

        println!(
            "\n=== FUZZY MATCH VERIFICATION ({} total, {} exact, {} fuzzy) ===\n",
            total, exact_count, fuzzy_count
        );

        // Group by AA entry
        let mut by_aa: HashMap<String, Vec<(&str, &str, &str, f64)>> = HashMap::new();
        for &(provider, model_id, model_name, entry, cos) in &fuzzy_matches {
            by_aa
                .entry(format!("{} ({})", entry.name, entry.slug))
                .or_default()
                .push((provider, model_id, model_name, cos));
        }

        // Sort groups by count descending
        let mut groups: Vec<_> = by_aa.iter().collect();
        groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        let mut obvious_bad = 0;
        for (aa_label, models) in &groups {
            println!("AA: {} <- {} models", aa_label, models.len());
            for (i, &(provider, model_id, model_name, cos)) in models.iter().enumerate() {
                if i >= 8 {
                    println!("  ... and {} more", models.len() - 8);
                    break;
                }
                let flag = if cos < 0.70 { " <<<" } else { "" };
                if cos < 0.70 {
                    obvious_bad += 1;
                }
                println!(
                    "  {}/{} \"{}\" [cos={:.2}]{}",
                    provider, model_id, model_name, cos, flag
                );
            }
        }

        println!(
            "\nSummary: {} fuzzy matches, {} with cos < 0.70 (flagged <<<)",
            fuzzy_count, obvious_bad
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

    /// Compare multiple matching algorithms against live models.dev data.
    /// Run with: cargo test algorithm_comparison -- --ignored --nocapture
    #[test]
    #[ignore]
    fn algorithm_comparison() {
        let store = BenchmarkStore::load();
        let providers: crate::data::ProvidersMap =
            reqwest::blocking::get("https://models.dev/api.json")
                .unwrap()
                .json()
                .unwrap();

        // Collect all models.dev entries (provider_id, model_id, name, reasoning, is_text)
        let mut all_models: Vec<(&str, &str, &str, bool, bool)> = Vec::new();
        for (provider_id, provider) in &providers {
            for (model_id, model) in &provider.models {
                all_models.push((
                    provider_id,
                    model_id,
                    &model.name,
                    model.reasoning,
                    model.is_text_model(),
                ));
            }
        }
        let total = all_models.len();
        let text_total = all_models.iter().filter(|m| m.4).count();
        let non_text_total = total - text_total;

        // Pre-tokenize all AA entries
        let aa_tokens: Vec<HashSet<String>> = store
            .entries
            .iter()
            .map(|e| {
                fuzzy_tokenize(&e.name)
                    .union(&fuzzy_tokenize(&e.slug))
                    .cloned()
                    .collect()
            })
            .collect();

        // --- Build IDF table from AA corpus ---
        let n = store.entries.len() as f64;
        let mut df: HashMap<String, usize> = HashMap::new();
        for tokens in &aa_tokens {
            for t in tokens {
                *df.entry(t.clone()).or_default() += 1;
            }
        }
        let idf: HashMap<String, f64> = df
            .iter()
            .map(|(t, &count)| (t.clone(), (n / count as f64).ln()))
            .collect();

        // Pre-compute entry norms for TF-IDF cosine
        let aa_norms: Vec<f64> = aa_tokens
            .iter()
            .map(|toks| {
                toks.iter()
                    .filter_map(|t| idf.get(t))
                    .map(|w| w * w)
                    .sum::<f64>()
                    .sqrt()
            })
            .collect();

        // --- Brand tokens for brand-anchored filtering ---
        let brand_tokens: HashSet<&str> = HashSet::from([
            "claude",
            "gpt",
            "gemini",
            "gemma",
            "llama",
            "mistral",
            "qwen",
            "deepseek",
            "grok",
            "phi",
            "nova",
            "command",
            "nemotron",
            "glm",
            "jamba",
            "dbrx",
            "falcon",
            "yi",
            "internlm",
            "minimax",
            "aya",
            "granite",
            "starcoder",
            "codestral",
            "devstral",
            "pixtral",
            "magistral",
            "ministral",
            "o1",
            "o3",
            "o4",
            "kimi",
            "mimo",
            "exaone",
            "mixtral",
            "titan",
        ]);

        fn brands_compatible(
            query: &HashSet<String>,
            entry: &HashSet<String>,
            brand_tokens: &HashSet<&str>,
        ) -> bool {
            let q_brands: HashSet<&str> = query
                .iter()
                .filter(|t| brand_tokens.contains(t.as_str()))
                .map(|s| s.as_str())
                .collect();
            let e_brands: HashSet<&str> = entry
                .iter()
                .filter(|t| brand_tokens.contains(t.as_str()))
                .map(|s| s.as_str())
                .collect();
            if q_brands.is_empty() || e_brands.is_empty() {
                return true;
            }
            !q_brands.is_disjoint(&e_brands)
        }

        println!(
            "\n=== ALGORITHM COMPARISON ({} models.dev models, {} AA entries) ===",
            total,
            store.entries.len()
        );
        println!(
            "  Text models: {}  |  Non-text (image/video/embed): {}  |  Skipped by filter: {}\n",
            text_total, non_text_total, non_text_total
        );

        // --- Baseline: Current approach (text models only) ---
        let mut current_matched = 0;
        let mut current_matched_text = 0;
        let mut current_aa_hit: HashSet<usize> = HashSet::new();
        let mut non_text_would_match = 0;
        for &(_, model_id, model_name, reasoning, is_text) in &all_models {
            if let Some(bench) = store.find_for_model(model_id, model_name, reasoning) {
                current_matched += 1;
                if is_text {
                    current_matched_text += 1;
                } else {
                    non_text_would_match += 1;
                }
                if let Some(idx) = store.entries.iter().position(|e| std::ptr::eq(e, bench)) {
                    current_aa_hit.insert(idx);
                }
            }
        }

        println!("BASELINE: Current production pipeline (tiers 0-5)");
        println!(
            "  All models:       {}/{} ({:.1}%)",
            current_matched,
            total,
            pct(current_matched, total)
        );
        println!(
            "  Text models only: {}/{} ({:.1}%)  <- effective rate after modalities filter",
            current_matched_text,
            text_total,
            pct(current_matched_text, text_total)
        );
        println!(
            "  Non-text matched: {}/{} (false positives blocked by UI filter)",
            non_text_would_match, non_text_total
        );
        println!(
            "  AA entries hit:   {}/{} ({:.1}%)\n",
            current_aa_hit.len(),
            store.entries.len(),
            pct(current_aa_hit.len(), store.entries.len())
        );

        // --- Standalone algorithm comparison at various thresholds ---
        let thresholds = [0.50, 0.55, 0.60, 0.65, 0.70, 0.75, 0.80, 0.85, 0.90];

        println!("=== STANDALONE ALGORITHMS (text models only, no Current baseline) ===");
        println!(
            "{:>5}  {:>14}  {:>14}  {:>14}  {:>14}  {:>14}",
            "Thr", "Jaccard", "Dice", "TF-IDF Cos", "Wt. Jaccard", "Brand+TF-IDF"
        );

        for &threshold in &thresholds {
            let mut j_m = 0usize;
            let mut d_m = 0usize;
            let mut tfidf_m = 0usize;
            let mut wj_m = 0usize;
            let mut bt_m = 0usize;

            for &(_, model_id, model_name, _, is_text) in &all_models {
                if !is_text {
                    continue;
                }
                let query_tokens: HashSet<String> = fuzzy_tokenize(model_id)
                    .union(&fuzzy_tokenize(model_name))
                    .cloned()
                    .collect();

                let query_norm: f64 = query_tokens
                    .iter()
                    .filter_map(|t| idf.get(t))
                    .map(|w| w * w)
                    .sum::<f64>()
                    .sqrt();

                let mut best_j = 0.0_f64;
                let mut best_d = 0.0_f64;
                let mut best_tfidf = 0.0_f64;
                let mut best_wj = 0.0_f64;
                let mut best_bt = 0.0_f64;

                for (idx, aa_toks) in aa_tokens.iter().enumerate() {
                    let intersection = query_tokens.intersection(aa_toks).count() as f64;
                    let union = query_tokens.union(aa_toks).count() as f64;
                    let sum_size = (query_tokens.len() + aa_toks.len()) as f64;

                    // Jaccard
                    if union > 0.0 {
                        let j = intersection / union;
                        if j > best_j {
                            best_j = j;
                        }
                    }

                    // Dice
                    if sum_size > 0.0 {
                        let d = 2.0 * intersection / sum_size;
                        if d > best_d {
                            best_d = d;
                        }
                    }

                    // TF-IDF Cosine
                    if query_norm > 0.0 && aa_norms[idx] > 0.0 {
                        let dot: f64 = query_tokens
                            .intersection(aa_toks)
                            .filter_map(|t| idf.get(t))
                            .map(|w| w * w)
                            .sum();
                        let cos = dot / (query_norm * aa_norms[idx]);
                        if cos > best_tfidf {
                            best_tfidf = cos;
                        }

                        // Brand-anchored TF-IDF: same cosine but skip incompatible brands
                        if brands_compatible(&query_tokens, aa_toks, &brand_tokens) && cos > best_bt
                        {
                            best_bt = cos;
                        }
                    }

                    // Weighted Jaccard (IDF-weighted)
                    let i_weight: f64 = query_tokens
                        .intersection(aa_toks)
                        .filter_map(|t| idf.get(t))
                        .sum();
                    let u_weight: f64 =
                        query_tokens.union(aa_toks).filter_map(|t| idf.get(t)).sum();
                    if u_weight > 0.0 {
                        let wj = i_weight / u_weight;
                        if wj > best_wj {
                            best_wj = wj;
                        }
                    }
                }

                if best_j >= threshold {
                    j_m += 1;
                }
                if best_d >= threshold {
                    d_m += 1;
                }
                if best_tfidf >= threshold {
                    tfidf_m += 1;
                }
                if best_wj >= threshold {
                    wj_m += 1;
                }
                if best_bt >= threshold {
                    bt_m += 1;
                }
            }

            println!(
                "{:>4}%  {:>5}/{} {:>5.1}%  {:>5}/{} {:>5.1}%  {:>5}/{} {:>5.1}%  {:>5}/{} {:>5.1}%  {:>5}/{} {:>5.1}%",
                (threshold * 100.0) as u32,
                j_m, text_total, pct(j_m, text_total),
                d_m, text_total, pct(d_m, text_total),
                tfidf_m, text_total, pct(tfidf_m, text_total),
                wj_m, text_total, pct(wj_m, text_total),
                bt_m, text_total, pct(bt_m, text_total),
            );
        }

        // --- Hybrid approaches: Current first, then fuzzy fallback (text models only) ---
        let current_hits: HashSet<usize> = all_models
            .iter()
            .enumerate()
            .filter(|(_, &(_, _, _, _, is_text))| is_text)
            .filter_map(|(i, &(_, model_id, model_name, reasoning, _))| {
                store
                    .find_for_model(model_id, model_name, reasoning)
                    .map(|_| i)
            })
            .collect();

        // For each unmatched text model, compute all algorithm scores
        struct FuzzyScores {
            jaccard: (f64, usize),
            dice: (f64, usize),
            tfidf_cosine: (f64, usize),
            weighted_jaccard: (f64, usize),
            brand_tfidf: (f64, usize),
        }

        let unmatched_scores: Vec<(usize, FuzzyScores)> = all_models
            .iter()
            .enumerate()
            .filter(|(_, &(_, _, _, _, is_text))| is_text)
            .filter(|(i, _)| !current_hits.contains(i))
            .map(|(i, &(_, model_id, model_name, _, _))| {
                let query_tokens: HashSet<String> = fuzzy_tokenize(model_id)
                    .union(&fuzzy_tokenize(model_name))
                    .cloned()
                    .collect();

                let query_norm: f64 = query_tokens
                    .iter()
                    .filter_map(|t| idf.get(t))
                    .map(|w| w * w)
                    .sum::<f64>()
                    .sqrt();

                let mut best_j = (0.0_f64, 0usize);
                let mut best_d = (0.0_f64, 0usize);
                let mut best_tfidf = (0.0_f64, 0usize);
                let mut best_wj = (0.0_f64, 0usize);
                let mut best_bt = (0.0_f64, 0usize);

                for (idx, aa_toks) in aa_tokens.iter().enumerate() {
                    let intersection = query_tokens.intersection(aa_toks).count() as f64;
                    let union = query_tokens.union(aa_toks).count() as f64;
                    let sum_size = (query_tokens.len() + aa_toks.len()) as f64;

                    if union > 0.0 {
                        let j = intersection / union;
                        if j > best_j.0 {
                            best_j = (j, idx);
                        }
                    }
                    if sum_size > 0.0 {
                        let d = 2.0 * intersection / sum_size;
                        if d > best_d.0 {
                            best_d = (d, idx);
                        }
                    }
                    if query_norm > 0.0 && aa_norms[idx] > 0.0 {
                        let dot: f64 = query_tokens
                            .intersection(aa_toks)
                            .filter_map(|t| idf.get(t))
                            .map(|w| w * w)
                            .sum();
                        let cos = dot / (query_norm * aa_norms[idx]);
                        if cos > best_tfidf.0 {
                            best_tfidf = (cos, idx);
                        }
                        if brands_compatible(&query_tokens, aa_toks, &brand_tokens)
                            && cos > best_bt.0
                        {
                            best_bt = (cos, idx);
                        }
                    }
                    let i_weight: f64 = query_tokens
                        .intersection(aa_toks)
                        .filter_map(|t| idf.get(t))
                        .sum();
                    let u_weight: f64 =
                        query_tokens.union(aa_toks).filter_map(|t| idf.get(t)).sum();
                    if u_weight > 0.0 {
                        let wj = i_weight / u_weight;
                        if wj > best_wj.0 {
                            best_wj = (wj, idx);
                        }
                    }
                }

                (
                    i,
                    FuzzyScores {
                        jaccard: best_j,
                        dice: best_d,
                        tfidf_cosine: best_tfidf,
                        weighted_jaccard: best_wj,
                        brand_tfidf: best_bt,
                    },
                )
            })
            .collect();

        println!("\n=== HYBRID: Current + fuzzy fallback (text models only) ===");
        println!(
            "Baseline: Current alone = {}/{} ({:.1}%)\n",
            current_hits.len(),
            text_total,
            pct(current_hits.len(), text_total)
        );

        type ScoreFn = fn(&FuzzyScores) -> (f64, usize);
        let get_jaccard: ScoreFn = |s| s.jaccard;
        let get_dice: ScoreFn = |s| s.dice;
        let get_tfidf: ScoreFn = |s| s.tfidf_cosine;
        let get_wj: ScoreFn = |s| s.weighted_jaccard;
        let get_bt: ScoreFn = |s| s.brand_tfidf;

        let configs: Vec<(&str, Vec<(&str, ScoreFn, f64)>)> = vec![
            // Single fallbacks — existing algos
            ("Current + Jaccard@0.80", vec![("J", get_jaccard, 0.80)]),
            ("Current + Dice@0.80", vec![("D", get_dice, 0.80)]),
            ("Current + Dice@0.75", vec![("D", get_dice, 0.75)]),
            // Single fallbacks — new algos
            ("Current + TF-IDF@0.70", vec![("T", get_tfidf, 0.70)]),
            ("Current + TF-IDF@0.65", vec![("T", get_tfidf, 0.65)]),
            ("Current + TF-IDF@0.60", vec![("T", get_tfidf, 0.60)]),
            ("Current + Wt.Jaccard@0.70", vec![("W", get_wj, 0.70)]),
            ("Current + Wt.Jaccard@0.65", vec![("W", get_wj, 0.65)]),
            ("Current + Wt.Jaccard@0.60", vec![("W", get_wj, 0.60)]),
            ("Current + Brand+TF-IDF@0.65", vec![("B", get_bt, 0.65)]),
            ("Current + Brand+TF-IDF@0.60", vec![("B", get_bt, 0.60)]),
            ("Current + Brand+TF-IDF@0.55", vec![("B", get_bt, 0.55)]),
            ("Current + Brand+TF-IDF@0.50", vec![("B", get_bt, 0.50)]),
            // Two-stage combos
            (
                "Current + Jaccard@0.80 + TF-IDF@0.65",
                vec![("J", get_jaccard, 0.80), ("T", get_tfidf, 0.65)],
            ),
            (
                "Current + Jaccard@0.80 + Brand+TF-IDF@0.60",
                vec![("J", get_jaccard, 0.80), ("B", get_bt, 0.60)],
            ),
            (
                "Current + Jaccard@0.80 + Wt.Jaccard@0.65",
                vec![("J", get_jaccard, 0.80), ("W", get_wj, 0.65)],
            ),
            (
                "Current + Dice@0.80 + Brand+TF-IDF@0.60",
                vec![("D", get_dice, 0.80), ("B", get_bt, 0.60)],
            ),
            (
                "Current + Brand+TF-IDF@0.65 + Dice@0.75",
                vec![("B", get_bt, 0.65), ("D", get_dice, 0.75)],
            ),
            // Three-stage combos
            (
                "Current + J@0.80 + Brand+TF-IDF@0.65 + D@0.80",
                vec![
                    ("J", get_jaccard, 0.80),
                    ("B", get_bt, 0.65),
                    ("D", get_dice, 0.80),
                ],
            ),
            (
                "Current + J@0.80 + Brand+TF-IDF@0.60 + D@0.75",
                vec![
                    ("J", get_jaccard, 0.80),
                    ("B", get_bt, 0.60),
                    ("D", get_dice, 0.75),
                ],
            ),
        ];

        let mut best_hybrid = ("", 0usize, 0usize);

        let current_text_matched = current_hits.len();
        for (label, steps) in &configs {
            let mut matched = current_text_matched;
            let mut aa_hit = current_aa_hit.clone();

            for &(_, ref scores) in &unmatched_scores {
                for &(_, score_fn, threshold) in steps {
                    let (score, aa_idx) = score_fn(scores);
                    if score >= threshold {
                        matched += 1;
                        aa_hit.insert(aa_idx);
                        break;
                    }
                }
            }

            let gained = matched - current_text_matched;
            println!(
                "  {:50} models: {}/{} ({:.1}%)  AA: {}/{}  (+{})",
                label,
                matched,
                text_total,
                pct(matched, text_total),
                aa_hit.len(),
                store.entries.len(),
                gained,
            );

            if matched > best_hybrid.1 || (matched == best_hybrid.1 && aa_hit.len() > best_hybrid.2)
            {
                best_hybrid = (label, matched, aa_hit.len());
            }
        }

        println!(
            "\n  BEST: {} -> {}/{} ({:.1}%) models, {} AA entries",
            best_hybrid.0,
            best_hybrid.1,
            text_total,
            pct(best_hybrid.1, text_total),
            best_hybrid.2,
        );

        // --- Sample new matches from best new algo (Brand+TF-IDF@0.60) ---
        println!("\n=== SAMPLE NEW MATCHES: Brand+TF-IDF@0.60 (first 40) ===\n");
        let mut samples = 0;
        for &(model_idx, ref scores) in &unmatched_scores {
            if samples >= 40 {
                break;
            }
            let (score, aa_idx) = scores.brand_tfidf;
            if score >= 0.60 {
                let (provider, model_id, model_name, _, _) = all_models[model_idx];
                let aa = &store.entries[aa_idx];
                let (j_score, _) = scores.jaccard;
                println!(
                    "  {}/{} \"{}\" -> \"{}\" [BT={:.2}, J={:.2}]",
                    provider, model_id, model_name, aa.name, score, j_score
                );
                samples += 1;
            }
        }

        // --- Show IDF distribution for context ---
        println!("\n=== TOP 20 HIGHEST IDF TOKENS (most distinctive) ===\n");
        let mut idf_sorted: Vec<_> = idf.iter().collect();
        idf_sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        for (t, w) in idf_sorted.iter().take(20) {
            println!("  {:20} idf={:.2}  (df={})", t, w, df[*t]);
        }

        println!("\n=== TOP 20 LOWEST IDF TOKENS (most common / least distinctive) ===\n");
        idf_sorted.reverse();
        for (t, w) in idf_sorted.iter().take(20) {
            println!("  {:20} idf={:.2}  (df={})", t, w, df[*t]);
        }
    }

    fn pct(n: usize, total: usize) -> f64 {
        n as f64 / total as f64 * 100.0
    }
}
