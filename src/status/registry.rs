use super::types::{OfficialStatusSource, StatusProviderSeed, StatusStrategy, StatusSupportTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusRegistryEntry {
    pub slug: &'static str,
    pub display_name: &'static str,
    pub source_slug: &'static str,
    pub strategy: StatusStrategy,
    pub support_tier: StatusSupportTier,
}

pub const STATUS_SOURCE_ALIASES: &[(&str, &str)] = &[
    ("github-copilot", "github"),
    ("github-models", "github"),
    ("moonshotai", "moonshot"),
    ("moonshotai-cn", "moonshot"),
    ("kimi-for-coding", "moonshot"),
    ("google-vertex", "google"),
    ("google-vertex-anthropic", "google"),
    ("perplexity-agent", "perplexity"),
    ("cloudflare-ai-gateway", "cloudflare"),
    ("cloudflare-workers-ai", "cloudflare"),
];

pub const STATUS_REGISTRY: &[StatusRegistryEntry] = &[
    StatusRegistryEntry {
        slug: "openai",
        display_name: "OpenAI",
        source_slug: "openai",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::OpenAi,
            fallback_source_slug: Some("openai"),
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "anthropic",
        display_name: "Anthropic",
        source_slug: "anthropic",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Anthropic,
            fallback_source_slug: Some("anthropic"),
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "openrouter",
        display_name: "OpenRouter",
        source_slug: "openrouter",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::OpenRouter,
            fallback_source_slug: Some("openrouter"),
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "google",
        display_name: "Google / Gemini",
        source_slug: "gemini",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::GoogleGeminiJson,
            fallback_source_slug: Some("gemini"),
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "moonshot",
        display_name: "Moonshot",
        source_slug: "moonshot",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Moonshot,
            fallback_source_slug: Some("moonshot"),
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "github",
        display_name: "GitHub",
        source_slug: "github",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::GitHub,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "cursor",
        display_name: "Cursor",
        source_slug: "cursor",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Cursor,
            fallback_source_slug: Some("cursor"),
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "perplexity",
        display_name: "Perplexity",
        source_slug: "perplexity",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Perplexity,
            fallback_source_slug: Some("perplexity"),
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "deepseek",
        display_name: "DeepSeek",
        source_slug: "deepseek",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::DeepSeek,
            fallback_source_slug: Some("deepseek"),
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "gitlab",
        display_name: "GitLab",
        source_slug: "gitlab",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::GitLab,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "poe",
        display_name: "Poe",
        source_slug: "poe",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Poe,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "nano-gpt",
        display_name: "NanoGPT",
        source_slug: "nano-gpt",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::NanoGpt,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "nvidia",
        display_name: "NVIDIA",
        source_slug: "nvidia",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Nvidia,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "vercel",
        display_name: "Vercel",
        source_slug: "vercel",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Vercel,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "helicone",
        display_name: "Helicone",
        source_slug: "helicone",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Helicone,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "groq",
        display_name: "Groq",
        source_slug: "groq",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Groq,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "cohere",
        display_name: "Cohere",
        source_slug: "cohere",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Cohere,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "cerebras",
        display_name: "Cerebras",
        source_slug: "cerebras",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Cerebras,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "cloudflare",
        display_name: "Cloudflare",
        source_slug: "cloudflare",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Cloudflare,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "together-ai",
        display_name: "Together AI",
        source_slug: "together-ai",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::TogetherAi,
            fallback_source_slug: Some("together-ai"),
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "huggingface",
        display_name: "Hugging Face",
        source_slug: "huggingface",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::HuggingFace,
            fallback_source_slug: Some("huggingface"),
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "zed",
        display_name: "Zed",
        source_slug: "zed",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Zed,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
];

pub fn canonical_status_slug(slug: &str) -> &str {
    STATUS_SOURCE_ALIASES
        .iter()
        .find_map(|(alias, canonical)| (*alias == slug).then_some(*canonical))
        .unwrap_or(slug)
}

pub fn status_seed_for_provider(slug: &str) -> StatusProviderSeed {
    let canonical = canonical_status_slug(slug);
    let entry = status_registry_entry(canonical);
    StatusProviderSeed {
        slug: slug.to_string(),
        display_name: entry
            .map(|entry| entry.display_name.to_string())
            .unwrap_or_else(|| slug.to_string()),
        source_slug: entry
            .map(|entry| entry.source_slug.to_string())
            .unwrap_or_else(|| canonical.to_string()),
        strategy: strategy_for_provider(slug),
        support_tier: entry
            .map(|entry| entry.support_tier)
            .unwrap_or(StatusSupportTier::Untracked),
    }
}

pub fn strategy_for_provider(slug: &str) -> StatusStrategy {
    status_registry_entry(canonical_status_slug(slug))
        .map(|entry| entry.strategy)
        .unwrap_or(StatusStrategy::Unverified)
}

pub fn status_registry_entry(slug: &str) -> Option<&'static StatusRegistryEntry> {
    STATUS_REGISTRY.iter().find(|entry| entry.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_maps_to_gemini_source_slug() {
        assert_eq!(
            status_registry_entry("google").map(|entry| entry.source_slug),
            Some("gemini")
        );
        assert_eq!(
            status_registry_entry("openai").map(|entry| entry.source_slug),
            Some("openai")
        );
    }

    #[test]
    fn unknown_provider_defaults_to_untracked_support() {
        let seed = status_seed_for_provider("some-nonexistent-provider");
        assert_eq!(seed.support_tier, StatusSupportTier::Untracked);
        assert_eq!(seed.source_slug, "some-nonexistent-provider");
    }

    #[test]
    fn provider_strategy_table_matches_current_scope() {
        assert!(matches!(
            strategy_for_provider("openai"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("anthropic"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("openrouter"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("google"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("moonshot"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("moonshotai"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("github-copilot"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("gitlab"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("poe"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("nano-gpt"),
            StatusStrategy::OfficialFirst { .. }
        ));
        assert!(matches!(
            strategy_for_provider("nvidia"),
            StatusStrategy::OfficialFirst { .. }
        ));
    }

    #[test]
    fn registry_contains_curated_sources() {
        assert!(status_registry_entry("github").is_some());
        assert!(status_registry_entry("cursor").is_some());
        assert!(status_registry_entry("perplexity").is_some());
        assert!(status_registry_entry("deepseek").is_some());
        assert!(status_registry_entry("gitlab").is_some());
        assert!(status_registry_entry("poe").is_some());
        assert!(status_registry_entry("nano-gpt").is_some());
        assert!(status_registry_entry("nvidia").is_some());
        assert!(status_registry_entry("vercel").is_some());
        assert!(status_registry_entry("helicone").is_some());
        assert!(status_registry_entry("groq").is_some());
        assert!(status_registry_entry("cohere").is_some());
        assert!(status_registry_entry("cerebras").is_some());
        assert!(status_registry_entry("cloudflare").is_some());
        assert!(status_registry_entry("together-ai").is_some());
        assert!(status_registry_entry("huggingface").is_some());
    }

    #[test]
    fn aliases_map_to_canonical_registry_entries() {
        assert_eq!(canonical_status_slug("github-copilot"), "github");
        assert_eq!(canonical_status_slug("github-models"), "github");
        assert_eq!(canonical_status_slug("moonshotai"), "moonshot");
        assert_eq!(canonical_status_slug("cloudflare-workers-ai"), "cloudflare");
    }
}
