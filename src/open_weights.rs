use std::collections::HashMap;

use crate::benchmarks::BenchmarkEntry;
use crate::data::Provider;

/// Normalize a string for fuzzy matching: lowercase, strip separators.
fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | '.' | ' '))
        .collect()
}

/// Map AA creator slugs to models.dev provider IDs where they differ.
/// Returns one or more provider IDs to search (some creators span multiple providers).
fn creator_to_providers(creator: &str) -> &[&str] {
    match creator {
        "meta" => &["llama"],
        "kimi" => &["moonshotai"],
        "aws" => &["amazon-bedrock"],
        "azure" => &["azure"],
        "nvidia" => &["nvidia"],
        "ibm" => &["nova"],
        // These creators match their models.dev provider ID directly
        _ => &[],
    }
}

/// Score how well a normalized AA slug matches a normalized models.dev model ID.
/// Higher is better. Returns 0 for no match.
fn match_score(norm_slug: &str, norm_model_id: &str) -> usize {
    if norm_slug == norm_model_id {
        return usize::MAX; // perfect match
    }

    // Check containment — prefer the longer overlap
    if norm_model_id.contains(norm_slug) {
        // slug is a prefix/subset of model ID (e.g. "claude35sonnet" in "claude35sonnet20241022")
        return norm_slug.len() * 2;
    }
    if norm_slug.contains(norm_model_id) {
        // model ID is a prefix/subset of slug (e.g. "o3mini" in "o3minihigh")
        return norm_model_id.len();
    }

    0
}

/// Build a map from AA benchmark entry slug → open_weights bool.
///
/// For each AA entry, we try to find the corresponding model in models.dev
/// by matching the entry's `creator` to a provider and then matching the
/// entry's `slug` against model IDs within that provider.
///
/// Uses a creator→provider translation table for known naming differences,
/// and best-score matching (not first-match) for slug resolution.
///
/// Unmatched entries are simply absent from the returned map — callers
/// should display no source label for those.
pub fn build_open_weights_map(
    providers: &[(String, Provider)],
    entries: &[BenchmarkEntry],
) -> HashMap<String, bool> {
    // Build a lookup: normalized provider ID → &Provider
    let provider_lookup: HashMap<String, &Provider> =
        providers.iter().map(|(id, p)| (normalize(id), p)).collect();

    // For each provider, build normalized model ID → open_weights
    let mut model_lookup: HashMap<String, HashMap<String, bool>> = HashMap::new();
    for (id, provider) in providers {
        let norm_provider = normalize(id);
        let models: HashMap<String, bool> = provider
            .models
            .iter()
            .map(|(model_id, model)| (normalize(model_id), model.open_weights))
            .collect();
        model_lookup.insert(norm_provider, models);
    }

    let mut result = HashMap::new();

    for entry in entries {
        if entry.creator.is_empty() || entry.slug.is_empty() {
            continue;
        }

        let norm_creator = normalize(&entry.creator);
        let norm_slug = normalize(&entry.slug);

        // Determine which providers to search: explicit mapping first, then identity
        let mapped = creator_to_providers(&entry.creator);
        let provider_ids: Vec<String> = if mapped.is_empty() {
            // Identity: try the creator slug directly
            vec![norm_creator.clone()]
        } else {
            mapped.iter().map(|id| normalize(id)).collect()
        };

        // Find the best match across all candidate providers
        let mut best_score = 0usize;
        let mut best_ow = None;

        for norm_provider_id in &provider_ids {
            if !provider_lookup.contains_key(norm_provider_id.as_str()) {
                continue;
            }

            if let Some(models) = model_lookup.get(norm_provider_id.as_str()) {
                for (norm_model_id, &ow) in models {
                    let score = match_score(&norm_slug, norm_model_id);
                    if score > best_score {
                        best_score = score;
                        best_ow = Some(ow);
                        if score == usize::MAX {
                            break; // perfect match, no need to keep looking
                        }
                    }
                }
            }

            if best_score == usize::MAX {
                break; // perfect match found
            }
        }

        if let Some(ow) = best_ow {
            result.insert(entry.slug.clone(), ow);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::BenchmarkEntry;
    use crate::data::{Model, Provider};

    fn make_provider(id: &str, models: Vec<(&str, bool)>) -> (String, Provider) {
        let mut model_map = HashMap::new();
        for (model_id, open_weights) in models {
            model_map.insert(
                model_id.to_string(),
                Model {
                    id: model_id.to_string(),
                    name: model_id.to_string(),
                    open_weights,
                    ..default_model()
                },
            );
        }
        (
            id.to_string(),
            Provider {
                id: id.to_string(),
                name: id.to_string(),
                npm: None,
                env: Vec::new(),
                doc: None,
                api: None,
                models: model_map,
            },
        )
    }

    fn default_model() -> Model {
        Model {
            id: String::new(),
            name: String::new(),
            family: None,
            reasoning: false,
            tool_call: false,
            attachment: false,
            temperature: false,
            modalities: None,
            cost: None,
            limit: None,
            release_date: None,
            last_updated: None,
            knowledge: None,
            open_weights: false,
            status: None,
        }
    }

    fn make_entry(creator: &str, slug: &str) -> BenchmarkEntry {
        BenchmarkEntry {
            id: String::new(),
            name: slug.to_string(),
            slug: slug.to_string(),
            creator: creator.to_string(),
            creator_id: String::new(),
            creator_name: String::new(),
            release_date: None,
            intelligence_index: None,
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
        }
    }

    #[test]
    fn test_direct_match() {
        let providers = vec![make_provider(
            "llama",
            vec![("llama-3.1-70b", true), ("llama-3.1-8b", true)],
        )];
        let entries = vec![make_entry("meta", "llama-3.1-70b")];

        let map = build_open_weights_map(&providers, &entries);
        assert_eq!(map.get("llama-3.1-70b"), Some(&true));
    }

    #[test]
    fn test_closed_model() {
        let providers = vec![make_provider(
            "openai",
            vec![("gpt-4o", false), ("gpt-4o-mini", false)],
        )];
        let entries = vec![make_entry("openai", "gpt-4o")];

        let map = build_open_weights_map(&providers, &entries);
        assert_eq!(map.get("gpt-4o"), Some(&false));
    }

    #[test]
    fn test_unmatched_creator_not_in_map() {
        let providers = vec![make_provider("openai", vec![("gpt-4o", false)])];
        let entries = vec![make_entry("unknown-lab", "some-model")];

        let map = build_open_weights_map(&providers, &entries);
        assert!(map.is_empty());
    }

    #[test]
    fn test_substring_match() {
        let providers = vec![make_provider(
            "mistral",
            vec![("mistral-large-2411", false)],
        )];
        let entries = vec![make_entry("mistral", "mistral-large")];

        let map = build_open_weights_map(&providers, &entries);
        assert_eq!(map.get("mistral-large"), Some(&false));
    }

    #[test]
    fn test_creator_to_provider_mapping() {
        // meta → llama
        let providers = vec![make_provider(
            "llama",
            vec![("llama-3.1-405b", true), ("llama-3.2-1b", true)],
        )];
        let entries = vec![
            make_entry("meta", "llama-3.1-405b"),
            make_entry("meta", "llama-3.2-1b"),
        ];

        let map = build_open_weights_map(&providers, &entries);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("llama-3.1-405b"), Some(&true));
        assert_eq!(map.get("llama-3.2-1b"), Some(&true));
    }

    #[test]
    fn test_best_score_picks_closest() {
        // Given two models, pick the one that matches best
        let providers = vec![make_provider(
            "anthropic",
            vec![
                ("claude-3-5-sonnet-20240620", false),
                ("claude-3-5-sonnet-20241022", false),
                ("claude-3-5-haiku-20241022", false),
            ],
        )];
        // "claude-35-sonnet" should match both sonnet models (not haiku)
        let entries = vec![make_entry("anthropic", "claude-35-sonnet")];

        let map = build_open_weights_map(&providers, &entries);
        // Should match one of the sonnet models (both are closed)
        assert_eq!(map.get("claude-35-sonnet"), Some(&false));
    }

    #[test]
    fn test_best_score_prefers_longer_slug_overlap() {
        // "gemini-2-5-pro" should match "gemini-2.5-pro" over "gemini-2.5-pro-preview"
        let providers = vec![make_provider(
            "google",
            vec![
                ("gemini-2.5-pro", false),
                ("gemini-2.5-pro-preview-05-06", false),
            ],
        )];
        let entries = vec![make_entry("google", "gemini-2-5-pro")];

        let map = build_open_weights_map(&providers, &entries);
        assert_eq!(map.get("gemini-2-5-pro"), Some(&false));
    }

    #[test]
    fn test_match_score_exact() {
        assert_eq!(match_score("gpt4o", "gpt4o"), usize::MAX);
    }

    #[test]
    fn test_match_score_slug_in_model() {
        // slug "claude35sonnet" in model "claude35sonnet20241022"
        let score = match_score("claude35sonnet", "claude35sonnet20241022");
        assert_eq!(score, "claude35sonnet".len() * 2);
    }

    #[test]
    fn test_match_score_model_in_slug() {
        // model "o3mini" in slug "o3minihigh"
        let score = match_score("o3minihigh", "o3mini");
        assert_eq!(score, "o3mini".len());
    }

    #[test]
    fn test_match_score_no_match() {
        assert_eq!(match_score("gpt4o", "claude35sonnet"), 0);
    }
}
