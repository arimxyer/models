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
        if normalized.contains("operational") || normalized.contains("ok") {
            return Self::Operational;
        }
        if normalized.contains("maint") {
            return Self::Maintenance;
        }
        if normalized.contains("degrad") || normalized.contains("partial") {
            return Self::Degraded;
        }
        if normalized.contains("outage") || normalized.contains("down") {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusProviderSeed {
    pub slug: String,
    pub display_name: String,
    pub source_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStatus {
    pub slug: String,
    pub display_name: String,
    pub source_slug: String,
    pub source_name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub health: ProviderHealth,
    pub last_checked: Option<String>,
    pub status_page_url: Option<String>,
    pub docs_url: Option<String>,
    pub source_page_url: Option<String>,
    pub history_url: Option<String>,
}

impl ProviderStatus {
    pub fn placeholder(seed: &StatusProviderSeed) -> Self {
        Self {
            slug: seed.slug.clone(),
            display_name: seed.display_name.clone(),
            source_slug: seed.source_slug.clone(),
            source_name: None,
            category: None,
            description: None,
            health: ProviderHealth::Unknown,
            last_checked: None,
            status_page_url: None,
            docs_url: None,
            source_page_url: None,
            history_url: None,
        }
    }
}

pub fn source_slug_for_provider(slug: &str) -> &str {
    match slug {
        // In the agent catalog, "google" means Gemini/Google AI usage rather than generic Google services.
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
            ProviderHealth::from_api_status("degraded performance"),
            ProviderHealth::Degraded
        );
        assert_eq!(
            ProviderHealth::from_api_status("partial outage"),
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
}
