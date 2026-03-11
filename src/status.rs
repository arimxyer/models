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
    ApiStatusCheck,
}

impl StatusSourceMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StatuspageV2 => "Statuspage API",
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
}

impl OfficialStatusSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI Status",
            Self::Anthropic => "Claude Status",
        }
    }

    pub fn summary_url(&self) -> &'static str {
        match self {
            Self::OpenAi => "https://status.openai.com/api/v2/summary.json",
            Self::Anthropic => "https://status.anthropic.com/api/v2/summary.json",
        }
    }
}

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
    match slug {
        "openai" => "OpenAI".to_string(),
        "anthropic" => "Anthropic".to_string(),
        "openrouter" => "OpenRouter".to_string(),
        "google" => "Google / Gemini".to_string(),
        "ollama" => "Ollama".to_string(),
        "qwen" => "Qwen".to_string(),
        "moonshot" => "Moonshot".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => other.to_string(),
            }
        }
    }
}

pub fn strategy_for_provider(slug: &str) -> StatusStrategy {
    match slug {
        "openai" => StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::OpenAi,
            fallback_source_slug: "openai",
        },
        "anthropic" => StatusStrategy::OfficialFirst {
            official: OfficialStatusSource::Anthropic,
            fallback_source_slug: "anthropic",
        },
        "openrouter" => StatusStrategy::FallbackOnly {
            fallback_source_slug: "openrouter",
        },
        "google" => StatusStrategy::FallbackOnly {
            fallback_source_slug: "gemini",
        },
        "moonshot" | "ollama" | "qwen" => StatusStrategy::Unsupported,
        _ => StatusStrategy::Unsupported,
    }
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
            StatusStrategy::FallbackOnly { .. }
        ));
        assert!(matches!(
            strategy_for_provider("google"),
            StatusStrategy::FallbackOnly { .. }
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
}
