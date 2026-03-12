#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProviderHealth {
    Operational,
    Degraded,
    Outage,
    Maintenance,
    #[default]
    Unknown,
}

impl ProviderHealth {
    pub fn from_api_status(status: &str) -> Self {
        let normalized = status.trim().to_lowercase();
        if normalized.is_empty() {
            return Self::Unknown;
        }
        if normalized.contains("operational") || normalized.contains("all systems operational") {
            return Self::Operational;
        }
        if normalized.contains("maint") {
            return Self::Maintenance;
        }
        if normalized.contains("degrad")
            || normalized.contains("partial")
            || normalized.contains("minor")
        {
            return Self::Degraded;
        }
        if normalized.contains("outage")
            || normalized.contains("major")
            || normalized.contains("down")
        {
            return Self::Outage;
        }
        Self::Unknown
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Operational => "Operational",
            Self::Degraded => "Degraded",
            Self::Outage => "Outage",
            Self::Maintenance => "Maintenance",
            Self::Unknown => "Unknown",
        }
    }

    pub fn sort_rank(&self) -> u8 {
        match self {
            Self::Outage => 0,
            Self::Degraded => 1,
            Self::Maintenance => 2,
            Self::Unknown => 3,
            Self::Operational => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusProvenance {
    Official,
    Fallback,
    #[default]
    Unavailable,
}

impl StatusProvenance {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Official => "Official",
            Self::Fallback => "Fallback",
            Self::Unavailable => "Unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSourceMethod {
    StatuspageV2,
    RssFeed,
    GoogleCloudJson,
    ApiStatusCheck,
}

impl StatusSourceMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StatuspageV2 => "Statuspage API",
            Self::RssFeed => "RSS Feed",
            Self::GoogleCloudJson => "Google JSON",
            Self::ApiStatusCheck => "API Status Check",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusStrategy {
    OfficialFirst {
        official: OfficialStatusSource,
        fallback_source_slug: &'static str,
    },
    FallbackOnly {
        fallback_source_slug: &'static str,
    },
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficialStatusSource {
    OpenAi,
    Anthropic,
    OpenRouterRss,
    GoogleGeminiJson,
}

impl OfficialStatusSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI Status",
            Self::Anthropic => "Claude Status",
            Self::OpenRouterRss => "OpenRouter Status",
            Self::GoogleGeminiJson => "Google Cloud Service Health",
        }
    }

    pub fn summary_url(&self) -> &'static str {
        match self {
            Self::OpenAi => "https://status.openai.com/api/v2/summary.json",
            Self::Anthropic => "https://status.anthropic.com/api/v2/summary.json",
            Self::OpenRouterRss => "https://status.openrouter.ai/incidents.rss",
            Self::GoogleGeminiJson => "https://status.cloud.google.com/incidents.json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSupportTier {
    Required,
    Curated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusRegistryEntry {
    pub slug: &'static str,
    pub display_name: &'static str,
    pub source_slug: &'static str,
    pub strategy: StatusStrategy,
    pub support_tier: StatusSupportTier,
}

pub const STATUS_REGISTRY: &[StatusRegistryEntry] = &[
    StatusRegistryEntry {
        slug: "openai",
        display_name: "OpenAI",
        source_slug: "openai",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::OpenAi,
            fallback_source_slug: "openai",
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "anthropic",
        display_name: "Anthropic",
        source_slug: "anthropic",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Anthropic,
            fallback_source_slug: "anthropic",
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "openrouter",
        display_name: "OpenRouter",
        source_slug: "openrouter",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::OpenRouterRss,
            fallback_source_slug: "openrouter",
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "google",
        display_name: "Google / Gemini",
        source_slug: "gemini",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::GoogleGeminiJson,
            fallback_source_slug: "gemini",
        },
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "moonshot",
        display_name: "Moonshot",
        source_slug: "moonshot",
        strategy: StatusStrategy::Unsupported,
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "ollama",
        display_name: "Ollama",
        source_slug: "ollama",
        strategy: StatusStrategy::Unsupported,
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "qwen",
        display_name: "Qwen",
        source_slug: "qwen",
        strategy: StatusStrategy::Unsupported,
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "github-copilot",
        display_name: "GitHub Copilot",
        source_slug: "github-copilot",
        strategy: StatusStrategy::FallbackOnly {
            fallback_source_slug: "github-copilot",
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "cursor",
        display_name: "Cursor",
        source_slug: "cursor",
        strategy: StatusStrategy::FallbackOnly {
            fallback_source_slug: "cursor",
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "perplexity",
        display_name: "Perplexity",
        source_slug: "perplexity",
        strategy: StatusStrategy::FallbackOnly {
            fallback_source_slug: "perplexity",
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "deepseek",
        display_name: "DeepSeek",
        source_slug: "deepseek",
        strategy: StatusStrategy::FallbackOnly {
            fallback_source_slug: "deepseek",
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "together-ai",
        display_name: "Together AI",
        source_slug: "together-ai",
        strategy: StatusStrategy::FallbackOnly {
            fallback_source_slug: "together-ai",
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "huggingface",
        display_name: "Hugging Face",
        source_slug: "huggingface",
        strategy: StatusStrategy::FallbackOnly {
            fallback_source_slug: "huggingface",
        },
        support_tier: StatusSupportTier::Curated,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusProviderSeed {
    pub slug: String,
    pub display_name: String,
    pub source_slug: String,
    pub strategy: StatusStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStatus {
    pub slug: String,
    pub display_name: String,
    pub source_slug: String,
    pub health: ProviderHealth,
    pub provenance: StatusProvenance,
    pub source_label: Option<String>,
    pub source_method: Option<StatusSourceMethod>,
    pub official_url: Option<String>,
    pub fallback_url: Option<String>,
    pub last_checked: Option<String>,
    pub summary: Option<String>,
}

impl ProviderStatus {
    pub fn placeholder(seed: &StatusProviderSeed) -> Self {
        Self {
            slug: seed.slug.clone(),
            display_name: seed.display_name.clone(),
            source_slug: seed.source_slug.clone(),
            health: ProviderHealth::Unknown,
            provenance: StatusProvenance::Unavailable,
            source_label: None,
            source_method: None,
            official_url: None,
            fallback_url: None,
            last_checked: None,
            summary: None,
        }
    }

    pub fn best_open_url(&self) -> Option<&str> {
        self.official_url
            .as_deref()
            .or(self.fallback_url.as_deref())
    }
}

pub fn source_slug_for_provider(slug: &str) -> &str {
    match slug {
        "google" => "gemini",
        other => other,
    }
}

pub fn display_name_for_provider(slug: &str) -> String {
    status_registry_entry(slug)
        .map(|entry| entry.display_name.to_string())
        .unwrap_or_else(|| {
            let mut chars = slug.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => slug.to_string(),
            }
        })
}

pub fn strategy_for_provider(slug: &str) -> StatusStrategy {
    status_registry_entry(slug)
        .map(|entry| entry.strategy)
        .unwrap_or(StatusStrategy::Unsupported)
}

pub fn status_registry_entry(slug: &str) -> Option<&'static StatusRegistryEntry> {
    STATUS_REGISTRY.iter().find(|entry| entry.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_health_maps_api_statuses() {
        assert_eq!(
            ProviderHealth::from_api_status("operational"),
            ProviderHealth::Operational
        );
        assert_eq!(
            ProviderHealth::from_api_status("Partial System Degradation"),
            ProviderHealth::Degraded
        );
        assert_eq!(
            ProviderHealth::from_api_status("major outage"),
            ProviderHealth::Outage
        );
        assert_eq!(
            ProviderHealth::from_api_status("scheduled maintenance"),
            ProviderHealth::Maintenance
        );
    }

    #[test]
    fn google_maps_to_gemini_source_slug() {
        assert_eq!(source_slug_for_provider("google"), "gemini");
        assert_eq!(source_slug_for_provider("openai"), "openai");
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
        assert_eq!(
            strategy_for_provider("moonshot"),
            StatusStrategy::Unsupported
        );
        assert_eq!(strategy_for_provider("ollama"), StatusStrategy::Unsupported);
        assert_eq!(strategy_for_provider("qwen"), StatusStrategy::Unsupported);
    }

    #[test]
    fn best_open_url_prefers_official() {
        let status = ProviderStatus {
            slug: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            source_slug: "openai".to_string(),
            health: ProviderHealth::Operational,
            provenance: StatusProvenance::Official,
            source_label: Some("OpenAI Status".to_string()),
            source_method: Some(StatusSourceMethod::StatuspageV2),
            official_url: Some("https://status.openai.com".to_string()),
            fallback_url: Some("https://apistatuscheck.com/api/openai".to_string()),
            last_checked: None,
            summary: None,
        };

        assert_eq!(status.best_open_url(), Some("https://status.openai.com"));
    }

    #[test]
    fn registry_contains_curated_sources() {
        assert!(status_registry_entry("github-copilot").is_some());
        assert!(status_registry_entry("cursor").is_some());
        assert!(status_registry_entry("perplexity").is_some());
        assert!(status_registry_entry("deepseek").is_some());
        assert!(status_registry_entry("together-ai").is_some());
        assert!(status_registry_entry("huggingface").is_some());
    }
}
