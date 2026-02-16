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

/// Build a map from AA benchmark entry slug → open_weights bool.
///
/// For each AA entry, we try to find the corresponding model in models.dev
/// by matching the entry's `creator` to a provider and then matching the
/// entry's `slug` against model IDs within that provider.
///
/// Unmatched entries are simply absent from the returned map — callers
/// should fall back to `CreatorOpenness` for those.
pub fn build_open_weights_map(
    providers: &[(String, Provider)],
    entries: &[BenchmarkEntry],
) -> HashMap<String, bool> {
    // Build a lookup: normalized provider ID → &Provider
    let provider_lookup: HashMap<String, &Provider> =
        providers.iter().map(|(id, p)| (normalize(id), p)).collect();

    // For each provider, build normalized model ID → open_weights
    // This is a nested map: normalized_provider_id → (normalized_model_id → open_weights)
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

        // Check if creator maps to a known provider
        if !provider_lookup.contains_key(&norm_creator) {
            continue;
        }

        let norm_slug = normalize(&entry.slug);

        // Try to find a matching model in that provider's model list
        if let Some(models) = model_lookup.get(&norm_creator) {
            // Strategy 1: Direct match on normalized slug
            if let Some(&ow) = models.get(&norm_slug) {
                result.insert(entry.slug.clone(), ow);
                continue;
            }

            // Strategy 2: Check if any model ID contains the slug or vice versa
            let mut best_match: Option<bool> = None;
            for (norm_model_id, &ow) in models {
                if norm_model_id.contains(&norm_slug) || norm_slug.contains(norm_model_id.as_str())
                {
                    best_match = Some(ow);
                    break;
                }
            }

            if let Some(ow) = best_match {
                result.insert(entry.slug.clone(), ow);
            }
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
            "meta",
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
}
