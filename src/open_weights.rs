use std::collections::HashMap;

use crate::benchmarks::BenchmarkEntry;
use crate::data::Provider;

/// Minimum Jaro-Winkler similarity to consider a match.
/// 0.85 is tuned to catch reordered tokens (e.g. "llama-3-1-instruct-405b" ↔
/// "llama-3.1-405b-instruct") while rejecting cross-family matches
/// (e.g. "gemma-3-27b" ≠ "gemini-3-pro").
const MIN_SIMILARITY: f64 = 0.85;

/// Normalize a string for matching: lowercase, strip separators.
fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | '.' | ' '))
        .collect()
}

/// Map AA creator slugs to models.dev provider IDs where they differ.
fn creator_to_providers(creator: &str) -> &[&str] {
    match creator {
        "meta" => &["llama"],
        "kimi" => &["moonshotai"],
        // Note: aws→amazon-bedrock and nvidia use org-prefixed model IDs
        // (e.g. "amazon.nova-2-lite-v1:0", "deepseek-ai/deepseek-r1") that
        // don't match AA slugs, but the mapping is kept for partial matches.
        "aws" => &["amazon-bedrock"],
        "azure" => &["azure"],
        "nvidia" => &["nvidia"],
        _ => &[],
    }
}

/// Hardcoded open/closed status for well-known creators that have no
/// models.dev provider. Returns `None` for unknown creators.
fn known_creator_openness(creator: &str) -> Option<bool> {
    match creator {
        // Open weight
        "ai2" => Some(true),           // OLMo, Molmo, Tülu — Allen Institute
        "ibm" => Some(true),           // Granite
        "lg" => Some(true),            // EXAONE
        "nous-research" => Some(true), // Hermes, DeepHermes
        "tii-uae" => Some(true),       // Falcon
        "databricks" => Some(true),    // DBRX
        "snowflake" => Some(true),     // Arctic
        "servicenow" => Some(true),    // Apriel
        "deepcogito" => Some(true),    // Cogito
        // Closed / proprietary API
        "ai21-labs" => Some(false),     // Jamba
        "naver" => Some(false),         // HyperCLOVA
        "korea-telecom" => Some(false), // Mi:dm
        _ => None,
    }
}

/// Build a map from AA benchmark entry slug → open_weights bool.
///
/// Matching strategy:
/// 1. **Creator-scoped**: Map AA creator to models.dev provider(s), then
///    Jaro-Winkler match the slug within those providers
/// 2. **Global fallback**: If no creator-scoped match, search ALL models
///    across ALL providers for a high-confidence slug match
///
/// Both stages require [`MIN_SIMILARITY`] threshold. Unmatched entries
/// are absent from the map — callers show no source label.
pub fn build_open_weights_map(
    providers: &[(String, Provider)],
    entries: &[BenchmarkEntry],
) -> HashMap<String, bool> {
    // Build per-provider lookup: normalized provider ID → [(normalized model ID, open_weights)]
    let provider_set: HashMap<String, ()> = providers
        .iter()
        .map(|(id, _)| (normalize(id), ()))
        .collect();

    let mut model_lookup: HashMap<String, Vec<(String, bool)>> = HashMap::new();
    for (id, provider) in providers {
        let norm_provider = normalize(id);
        let models: Vec<(String, bool)> = provider
            .models
            .iter()
            .map(|(model_id, model)| (normalize(model_id), model.open_weights))
            .collect();
        model_lookup.insert(norm_provider, models);
    }

    // Build global flat list of all models for fallback matching
    let all_models: Vec<(String, bool)> = providers
        .iter()
        .flat_map(|(_, provider)| {
            provider
                .models
                .iter()
                .map(|(model_id, model)| (normalize(model_id), model.open_weights))
        })
        .collect();

    let mut result = HashMap::new();

    for entry in entries {
        if entry.creator.is_empty() || entry.slug.is_empty() {
            continue;
        }

        let norm_creator = normalize(&entry.creator);
        let norm_slug = normalize(&entry.slug);

        // Stage 1: Creator-scoped matching
        let mapped = creator_to_providers(&entry.creator);
        let provider_ids: Vec<String> = if mapped.is_empty() {
            vec![norm_creator.clone()]
        } else {
            mapped.iter().map(|id| normalize(id)).collect()
        };

        let mut best_score: f64 = 0.0;
        let mut best_ow = None;

        for norm_provider_id in &provider_ids {
            if !provider_set.contains_key(norm_provider_id.as_str()) {
                continue;
            }

            if let Some(models) = model_lookup.get(norm_provider_id.as_str()) {
                for (norm_model_id, ow) in models {
                    let score = strsim::jaro_winkler(&norm_slug, norm_model_id);
                    if score > best_score {
                        best_score = score;
                        best_ow = Some(*ow);
                        if (score - 1.0).abs() < f64::EPSILON {
                            break;
                        }
                    }
                }
            }

            if (best_score - 1.0).abs() < f64::EPSILON {
                break;
            }
        }

        // Stage 2: Global fallback — search all models if creator-scoped didn't match
        if best_score < MIN_SIMILARITY {
            for (norm_model_id, ow) in &all_models {
                let score = strsim::jaro_winkler(&norm_slug, norm_model_id);
                if score > best_score {
                    best_score = score;
                    best_ow = Some(*ow);
                    if (score - 1.0).abs() < f64::EPSILON {
                        break;
                    }
                }
            }
        }

        if best_score >= MIN_SIMILARITY {
            if let Some(ow) = best_ow {
                result.insert(entry.slug.clone(), ow);
                continue;
            }
        }

        // Stage 3: Known creator overrides for providers absent from models.dev
        if let Some(ow) = known_creator_openness(&entry.creator) {
            result.insert(entry.slug.clone(), ow);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::BenchmarkEntry;
    use crate::data::{Model, Provider, ProvidersMap};

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
    fn test_reordered_tokens_match() {
        // AA: "llama-3-1-instruct-405b" vs models.dev: "llama-3.1-405b-instruct"
        // These differ in token order but should match via Jaro-Winkler
        let providers = vec![make_provider(
            "llama",
            vec![("llama-3.1-405b-instruct", true)],
        )];
        let entries = vec![make_entry("meta", "llama-3-1-instruct-405b")];

        let map = build_open_weights_map(&providers, &entries);
        assert_eq!(map.get("llama-3-1-instruct-405b"), Some(&true));
    }

    #[test]
    fn test_cross_family_rejected() {
        // "gemma-3-27b" should NOT match "gemini-3-pro" — different model families
        let providers = vec![make_provider(
            "google",
            vec![("gemini-3-pro-preview", false)],
        )];
        let entries = vec![make_entry("google", "gemma-3-27b")];

        let map = build_open_weights_map(&providers, &entries);
        assert!(map.is_empty(), "gemma should not match gemini");
    }

    /// Diagnostic test: runs matching against real benchmarks.json + live models.dev API.
    /// Run manually with: cargo test diagnostic_match_rate -- --ignored --nocapture
    #[test]
    #[ignore]
    fn diagnostic_match_rate() {
        // Load benchmark entries from local data file
        let bench_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/benchmarks.json");
        let bench_data = std::fs::read_to_string(&bench_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", bench_path.display()));
        let entries: Vec<BenchmarkEntry> =
            serde_json::from_str(&bench_data).expect("Failed to parse benchmarks.json");

        // Fetch providers from models.dev API
        let api_url = "https://models.dev/api.json";
        let response = reqwest::blocking::get(api_url).expect("Failed to fetch models.dev API");
        let providers_map: ProvidersMap = response.json().expect("Failed to parse API response");
        let providers: Vec<(String, crate::data::Provider)> = providers_map.into_iter().collect();

        // Run matching
        let map = build_open_weights_map(&providers, &entries);

        // Report stats
        let total = entries.len();
        let matched = map.len();
        let unmatched = total - matched;
        let open_count = map.values().filter(|&&v| v).count();
        let closed_count = map.values().filter(|&&v| !v).count();

        println!("\n=== Open Weights Match Rate ===");
        println!("Total AA entries:  {total}");
        println!(
            "Matched:           {matched} ({:.1}%)",
            matched as f64 / total as f64 * 100.0
        );
        println!("  Open:            {open_count}");
        println!("  Closed:          {closed_count}");
        println!(
            "Unmatched:         {unmatched} ({:.1}%)",
            unmatched as f64 / total as f64 * 100.0
        );

        // Group unmatched by creator
        let mut unmatched_by_creator: HashMap<&str, Vec<&str>> = HashMap::new();
        for entry in &entries {
            if !map.contains_key(&entry.slug) {
                unmatched_by_creator
                    .entry(&entry.creator)
                    .or_default()
                    .push(&entry.slug);
            }
        }
        let mut unmatched_creators: Vec<_> = unmatched_by_creator.iter().collect();
        unmatched_creators.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        // Build provider ID set for checking availability
        let provider_set: HashMap<String, Vec<String>> = providers
            .iter()
            .map(|(id, p)| {
                let model_ids: Vec<String> = p.models.keys().cloned().collect();
                (normalize(id), model_ids)
            })
            .collect();

        println!("\n--- Unmatched by creator ---");
        let mut no_provider_count = 0;
        let mut has_provider_count = 0;

        for &(creator, slugs) in &unmatched_creators {
            let mapped = creator_to_providers(creator);
            let norm_ids: Vec<String> = if mapped.is_empty() {
                vec![normalize(creator)]
            } else {
                mapped.iter().map(|id| normalize(id)).collect()
            };

            let has_provider = norm_ids
                .iter()
                .any(|id| provider_set.contains_key(id.as_str()));
            let status = if has_provider {
                has_provider_count += slugs.len();
                "HAS PROVIDER"
            } else {
                no_provider_count += slugs.len();
                "NO PROVIDER"
            };

            let mapping_note = if mapped.is_empty() {
                format!("(identity: {})", normalize(creator))
            } else {
                format!("(mapped → {:?})", mapped)
            };
            println!(
                "[{status}] {creator} ({} entries) {mapping_note}",
                slugs.len()
            );
            for slug in slugs {
                println!("  - {slug}");
            }

            // Show sample model IDs from the provider for gap analysis
            if has_provider {
                for norm_id in &norm_ids {
                    if let Some(model_ids) = provider_set.get(norm_id.as_str()) {
                        let mut sample: Vec<&str> = model_ids.iter().map(|s| s.as_str()).collect();
                        sample.sort();
                        sample.truncate(10);
                        println!(
                            "  >> models.dev has: {:?}{}",
                            sample,
                            if model_ids.len() > 10 { " ..." } else { "" }
                        );
                    }
                }
            }
        }

        println!("\n--- Summary ---");
        println!("No provider in models.dev:  {no_provider_count} (truly unmatchable)");
        println!("Has provider, slug mismatch: {has_provider_count} (potentially fixable)");
    }
}
