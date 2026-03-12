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
    Feed,
    GoogleCloudJson,
    ApiStatusCheck,
}

impl StatusSourceMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StatuspageV2 => "Statuspage API",
            Self::Feed => "RSS / Atom Feed",
            Self::GoogleCloudJson => "Google JSON",
            Self::ApiStatusCheck => "API Status Check",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusStrategy {
    OfficialFirst {
        official: OfficialStatusSource,
        fallback_source_slug: Option<&'static str>,
    },
    Unverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficialStatusSource {
    OpenAi,
    Anthropic,
    OpenRouter,
    GoogleGeminiJson,
    Moonshot,
    Vercel,
    Helicone,
    Groq,
    Cohere,
    Cerebras,
    Cloudflare,
    Aws,
    Azure,
    Cursor,
    GitHub,
    DeepSeek,
    Perplexity,
    HuggingFace,
    TogetherAi,
}

impl OfficialStatusSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI Status",
            Self::Anthropic => "Claude Status",
            Self::OpenRouter => "OpenRouter Status",
            Self::GoogleGeminiJson => "Google Cloud Service Health",
            Self::Moonshot => "Moonshot AI Status",
            Self::Vercel => "Vercel Status",
            Self::Helicone => "Helicone Status",
            Self::Groq => "Groq Status",
            Self::Cohere => "Cohere Status",
            Self::Cerebras => "Cerebras Status",
            Self::Cloudflare => "Cloudflare Status",
            Self::Aws => "AWS Service Health",
            Self::Azure => "Azure Status",
            Self::Cursor => "Cursor Status",
            Self::GitHub => "GitHub Status",
            Self::DeepSeek => "DeepSeek Status",
            Self::Perplexity => "Perplexity Status",
            Self::HuggingFace => "Hugging Face Status",
            Self::TogetherAi => "Together AI Status",
        }
    }

    pub fn endpoint_url(&self) -> &'static str {
        match self {
            Self::OpenAi => "https://status.openai.com/api/v2/summary.json",
            Self::Anthropic => "https://status.anthropic.com/api/v2/summary.json",
            Self::OpenRouter => "https://status.openrouter.ai/incidents.rss",
            Self::GoogleGeminiJson => "https://status.cloud.google.com/incidents.json",
            Self::Moonshot => "https://status.moonshot.cn/api/v2/summary.json",
            Self::Vercel => "https://www.vercel-status.com/api/v2/summary.json",
            Self::Helicone => "https://status.helicone.ai/feed.rss",
            Self::Groq => "https://groqstatus.com/api/v2/summary.json",
            Self::Cohere => "https://status.cohere.com/api/v2/summary.json",
            Self::Cerebras => "https://status.cerebras.ai/api/v2/summary.json",
            Self::Cloudflare => "https://www.cloudflarestatus.com/api/v2/summary.json",
            Self::Aws => "https://status.aws.amazon.com/rss/all.rss",
            Self::Azure => "https://azure.status.microsoft/en-us/status/feed/",
            Self::Cursor => "https://status.cursor.com/api/v2/summary.json",
            Self::GitHub => "https://www.githubstatus.com/api/v2/summary.json",
            Self::DeepSeek => "https://status.deepseek.com/api/v2/summary.json",
            Self::Perplexity => "https://status.perplexity.com/feed",
            Self::HuggingFace => "https://status.huggingface.co/feed.rss",
            Self::TogetherAi => "https://status.together.ai/feed.rss",
        }
    }

    pub fn page_url(&self) -> &'static str {
        match self {
            Self::OpenAi => "https://status.openai.com",
            Self::Anthropic => "https://status.anthropic.com",
            Self::OpenRouter => "https://status.openrouter.ai",
            Self::GoogleGeminiJson => {
                "https://status.cloud.google.com/products/Z0FZJAMvEB4j3NbCJs6B/history"
            }
            Self::Moonshot => "https://status.moonshot.cn",
            Self::Vercel => "https://www.vercel-status.com",
            Self::Helicone => "https://status.helicone.ai",
            Self::Groq => "https://groqstatus.com",
            Self::Cohere => "https://status.cohere.com",
            Self::Cerebras => "https://status.cerebras.ai",
            Self::Cloudflare => "https://www.cloudflarestatus.com",
            Self::Aws => "https://status.aws.amazon.com",
            Self::Azure => "https://azure.status.microsoft/en-us/status",
            Self::Cursor => "https://status.cursor.com",
            Self::GitHub => "https://www.githubstatus.com",
            Self::DeepSeek => "https://status.deepseek.com",
            Self::Perplexity => "https://status.perplexity.com",
            Self::HuggingFace => "https://status.huggingface.co",
            Self::TogetherAi => "https://status.together.ai",
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

pub const STATUS_SOURCE_ALIASES: &[(&str, &str)] = &[
    ("github-copilot", "github"),
    ("github-models", "github"),
    ("moonshotai", "moonshot"),
    ("moonshotai-cn", "moonshot"),
    ("kimi-for-coding", "moonshot"),
    ("google-vertex", "google"),
    ("google-vertex-anthropic", "google"),
    ("perplexity-agent", "perplexity"),
    ("amazon-bedrock", "aws"),
    ("azure-cognitive-services", "azure"),
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
        slug: "ollama",
        display_name: "Ollama",
        source_slug: "ollama",
        strategy: StatusStrategy::Unverified,
        support_tier: StatusSupportTier::Required,
    },
    StatusRegistryEntry {
        slug: "qwen",
        display_name: "Qwen",
        source_slug: "qwen",
        strategy: StatusStrategy::Unverified,
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
        slug: "aws",
        display_name: "AWS",
        source_slug: "aws",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Aws,
            fallback_source_slug: None,
        },
        support_tier: StatusSupportTier::Curated,
    },
    StatusRegistryEntry {
        slug: "azure",
        display_name: "Azure",
        source_slug: "azure",
        strategy: StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Azure,
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
        assert_eq!(strategy_for_provider("ollama"), StatusStrategy::Unverified);
        assert_eq!(strategy_for_provider("qwen"), StatusStrategy::Unverified);
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
        assert!(status_registry_entry("github").is_some());
        assert!(status_registry_entry("cursor").is_some());
        assert!(status_registry_entry("perplexity").is_some());
        assert!(status_registry_entry("deepseek").is_some());
        assert!(status_registry_entry("vercel").is_some());
        assert!(status_registry_entry("helicone").is_some());
        assert!(status_registry_entry("groq").is_some());
        assert!(status_registry_entry("cohere").is_some());
        assert!(status_registry_entry("cerebras").is_some());
        assert!(status_registry_entry("cloudflare").is_some());
        assert!(status_registry_entry("aws").is_some());
        assert!(status_registry_entry("azure").is_some());
        assert!(status_registry_entry("together-ai").is_some());
        assert!(status_registry_entry("huggingface").is_some());
    }

    #[test]
    fn aliases_map_to_canonical_registry_entries() {
        assert_eq!(canonical_status_slug("github-copilot"), "github");
        assert_eq!(canonical_status_slug("github-models"), "github");
        assert_eq!(canonical_status_slug("moonshotai"), "moonshot");
        assert_eq!(canonical_status_slug("amazon-bedrock"), "aws");
        assert_eq!(canonical_status_slug("azure-cognitive-services"), "azure");
        assert_eq!(canonical_status_slug("cloudflare-workers-ai"), "cloudflare");
    }
}
