use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};

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
    pub fn label(self) -> &'static str {
        match self {
            Self::Official => "Official",
            Self::Fallback => "Fallback",
            Self::Unavailable => "Unavailable",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Official => "OFF",
            Self::Fallback => "FB",
            Self::Unavailable => "MISS",
        }
    }

    pub fn sort_rank(self) -> u8 {
        match self {
            Self::Official => 0,
            Self::Fallback => 1,
            Self::Unavailable => 2,
        }
    }

    pub fn detail_note(self) -> &'static str {
        match self {
            Self::Official => "Official machine-readable provider status feed.",
            Self::Fallback => {
                "Fallback aggregator snapshot. Verify details on the provider status page."
            }
            Self::Unavailable => {
                "No working machine-readable status source is configured or reachable."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum StatusSourceMethod {
    StatuspageV2,
    Feed,
    GoogleCloudJson,
    ApiStatusCheck,
    BetterStack,
    IncidentIoShim,
    OnlineOrNot,
    StatusIo,
    Instatus,
}

impl StatusSourceMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StatuspageV2 => "Statuspage API",
            Self::Feed => "RSS / Atom Feed",
            Self::GoogleCloudJson => "Google JSON",
            Self::ApiStatusCheck => "API Status Check",
            Self::BetterStack => "Better Stack",
            Self::IncidentIoShim => "incident.io",
            Self::OnlineOrNot => "OnlineOrNot",
            Self::StatusIo => "Status.io",
            Self::Instatus => "Instatus",
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
    GitLab,
    Poe,
    NanoGpt,
    Nvidia,
    Vercel,
    Helicone,
    Groq,
    Cohere,
    Cerebras,
    Cloudflare,
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
            Self::GitLab => "GitLab Status",
            Self::Poe => "Poe Status",
            Self::NanoGpt => "NanoGPT Status",
            Self::Nvidia => "NVIDIA NGC Status",
            Self::Vercel => "Vercel Status",
            Self::Helicone => "Helicone Status",
            Self::Groq => "Groq Status",
            Self::Cohere => "Cohere Status",
            Self::Cerebras => "Cerebras Status",
            Self::Cloudflare => "Cloudflare Status",
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
            Self::Anthropic => "https://status.claude.com/api/v2/summary.json",
            Self::OpenRouter => "https://api.onlineornot.com/v1/status_pages/openrouter/summary",
            Self::GoogleGeminiJson => "https://status.cloud.google.com/incidents.json",
            Self::Moonshot => "https://status.moonshot.cn/api/v2/summary.json",
            Self::GitLab => "https://api.status.io/1.0/status/5b36dc6502d06804c08349f7",
            Self::Poe => "https://status.poe.com/api/v2/summary.json",
            Self::NanoGpt => "https://status.nano-gpt.com/index.json",
            Self::Nvidia => "https://status.ngc.nvidia.com/api/v2/summary.json",
            Self::Vercel => "https://www.vercel-status.com/api/v2/summary.json",
            Self::Helicone => "https://status.helicone.ai/index.json",
            Self::Groq => "https://groqstatus.com/api/v2/summary.json",
            Self::Cohere => "https://status.cohere.com/api/v2/summary.json",
            Self::Cerebras => "https://status.cerebras.ai/api/v2/summary.json",
            Self::Cloudflare => "https://www.cloudflarestatus.com/api/v2/summary.json",
            Self::Cursor => "https://status.cursor.com/api/v2/summary.json",
            Self::GitHub => "https://www.githubstatus.com/api/v2/summary.json",
            Self::DeepSeek => "https://status.deepseek.com/api/v2/summary.json",
            Self::Perplexity => "https://status.perplexity.com/summary.json",
            Self::HuggingFace => "https://status.huggingface.co/index.json",
            Self::TogetherAi => "https://status.together.ai/index.json",
        }
    }

    pub fn page_url(&self) -> &'static str {
        match self {
            Self::OpenAi => "https://status.openai.com",
            Self::Anthropic => "https://status.claude.com",
            Self::OpenRouter => "https://status.openrouter.ai",
            Self::GoogleGeminiJson => {
                "https://status.cloud.google.com/products/Z0FZJAMvEB4j3NbCJs6B/history"
            }
            Self::Moonshot => "https://status.moonshot.cn",
            Self::GitLab => "https://status.gitlab.com",
            Self::Poe => "https://status.poe.com",
            Self::NanoGpt => "https://status.nano-gpt.com",
            Self::Nvidia => "https://status.ngc.nvidia.com",
            Self::Vercel => "https://www.vercel-status.com",
            Self::Helicone => "https://status.helicone.ai",
            Self::Groq => "https://groqstatus.com",
            Self::Cohere => "https://status.cohere.com",
            Self::Cerebras => "https://status.cerebras.ai",
            Self::Cloudflare => "https://www.cloudflarestatus.com",
            Self::Cursor => "https://status.cursor.com",
            Self::GitHub => "https://www.githubstatus.com",
            Self::DeepSeek => "https://status.deepseek.com",
            Self::Perplexity => "https://status.perplexity.com",
            Self::HuggingFace => "https://status.huggingface.co",
            Self::TogetherAi => "https://status.together.ai",
        }
    }

    #[allow(dead_code)]
    pub fn source_method(&self) -> StatusSourceMethod {
        match self {
            Self::Anthropic
            | Self::Moonshot
            | Self::Vercel
            | Self::Cerebras
            | Self::Cloudflare
            | Self::Cursor
            | Self::GitHub
            | Self::DeepSeek
            | Self::Nvidia => StatusSourceMethod::StatuspageV2,

            Self::OpenAi | Self::Poe | Self::Groq | Self::Cohere => {
                StatusSourceMethod::IncidentIoShim
            }

            Self::TogetherAi | Self::HuggingFace | Self::Helicone | Self::NanoGpt => {
                StatusSourceMethod::BetterStack
            }

            Self::OpenRouter => StatusSourceMethod::OnlineOrNot,

            Self::GitLab => StatusSourceMethod::StatusIo,

            Self::Perplexity => StatusSourceMethod::Instatus,

            Self::GoogleGeminiJson => StatusSourceMethod::GoogleCloudJson,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSupportTier {
    Required,
    Curated,
    Untracked,
}

impl StatusSupportTier {
    pub fn label(self) -> &'static str {
        match self {
            Self::Required => "Required",
            Self::Curated => "Curated",
            Self::Untracked => "Untracked",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Required => "R",
            Self::Curated => "C",
            Self::Untracked => "U",
        }
    }

    pub fn sort_rank(self) -> u8 {
        match self {
            Self::Required => 0,
            Self::Curated => 1,
            Self::Untracked => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum StatusConfidence {
    High,
    Medium,
    Low,
    #[default]
    None,
}

#[allow(dead_code)]
impl StatusConfidence {
    pub fn label(self) -> &'static str {
        match self {
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
            Self::None => "None",
        }
    }

    pub fn sort_rank(self) -> u8 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => 2,
            Self::None => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum StatusCoverage {
    Full,
    IncidentOnly,
    ComponentOnly,
    SummaryOnly,
    #[default]
    None,
}

#[allow(dead_code)]
impl StatusCoverage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::IncidentOnly => "Incident only",
            Self::ComponentOnly => "Component only",
            Self::SummaryOnly => "Summary only",
            Self::None => "None",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum StatusFreshness {
    Fresh,
    Aging,
    Stale,
    #[default]
    Unknown,
}

#[allow(dead_code)]
impl StatusFreshness {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fresh => "Fresh",
            Self::Aging => "Aging",
            Self::Stale => "Stale",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub enum AffectedSurface {
    Api,
    Chat,
    Auth,
    Console,
    UploadsFiles,
    ModelsInference,
    Unknown,
}

#[allow(dead_code)]
impl AffectedSurface {
    pub fn label(self) -> &'static str {
        match self {
            Self::Api => "API",
            Self::Chat => "Chat",
            Self::Auth => "Auth",
            Self::Console => "Console",
            Self::UploadsFiles => "Uploads / Files",
            Self::ModelsInference => "Models / Inference",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct StatusContradiction {
    pub summary: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ProviderAssessment {
    pub overall_health: ProviderHealth,
    pub confidence: StatusConfidence,
    pub coverage: StatusCoverage,
    pub freshness: StatusFreshness,
    pub active_incident_count: usize,
    pub affected_surfaces: Vec<AffectedSurface>,
    pub assessment_summary: String,
    pub evidence_summary: String,
    pub reconciliation_notes: Vec<String>,
    pub warnings: Vec<String>,
    pub contradictions: Vec<StatusContradiction>,
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
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentStatus {
    pub name: String,
    pub status: String,
    pub group_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentUpdate {
    pub status: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveIncident {
    pub name: String,
    pub status: String,
    pub impact: String,
    pub shortlink: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub latest_update: Option<IncidentUpdate>,
    pub affected_components: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledMaintenance {
    pub name: String,
    pub status: String,
    pub impact: String,
    pub scheduled_for: Option<String>,
    pub scheduled_until: Option<String>,
    pub affected_components: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusProviderSeed {
    pub slug: String,
    pub display_name: String,
    pub source_slug: String,
    pub strategy: StatusStrategy,
    pub support_tier: StatusSupportTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStatus {
    pub slug: String,
    pub display_name: String,
    pub source_slug: String,
    pub support_tier: StatusSupportTier,
    pub health: ProviderHealth,
    pub provenance: StatusProvenance,
    pub source_label: Option<String>,
    pub source_method: Option<StatusSourceMethod>,
    pub official_url: Option<String>,
    pub fallback_url: Option<String>,
    pub last_checked: Option<String>,
    pub summary: Option<String>,
    pub components: Vec<ComponentStatus>,
    pub incidents: Vec<ActiveIncident>,
    pub scheduled_maintenances: Vec<ScheduledMaintenance>,
    pub error: Option<String>,
}

impl ProviderStatus {
    pub fn placeholder(seed: &StatusProviderSeed) -> Self {
        Self {
            slug: seed.slug.clone(),
            display_name: seed.display_name.clone(),
            source_slug: seed.source_slug.clone(),
            support_tier: seed.support_tier,
            health: ProviderHealth::Unknown,
            provenance: StatusProvenance::Unavailable,
            source_label: None,
            source_method: None,
            official_url: None,
            fallback_url: None,
            last_checked: None,
            summary: None,
            components: Vec::new(),
            incidents: Vec::new(),
            scheduled_maintenances: Vec::new(),
            error: None,
        }
    }

    pub fn best_open_url(&self) -> Option<&str> {
        self.official_url
            .as_deref()
            .or(self.fallback_url.as_deref())
    }

    pub fn active_incidents(&self) -> Vec<&ActiveIncident> {
        self.incidents
            .iter()
            .filter(|incident| incident.is_active())
            .collect()
    }

    pub fn user_visible_affected_items(&self) -> Vec<String> {
        let assessment = self.assessment();
        if !assessment.affected_surfaces.is_empty() {
            return assessment
                .affected_surfaces
                .iter()
                .map(|surface| surface.label().to_string())
                .collect();
        }

        self.active_incidents()
            .into_iter()
            .flat_map(|incident| incident.affected_components.iter().cloned())
            .chain(
                self.scheduled_maintenances
                    .iter()
                    .flat_map(|maint| maint.affected_components.iter().cloned()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn user_visible_caveat(&self) -> Option<&'static str> {
        let assessment = self.assessment();
        if self.provenance == StatusProvenance::Unavailable {
            Some("Status unavailable")
        } else if self.error.is_some() || self.provenance == StatusProvenance::Fallback {
            Some("Limited detail available")
        } else if assessment
            .warnings
            .iter()
            .any(|warning| warning.contains("stale") || warning.contains("reliable freshness"))
        {
            Some("Verify details on the official status page")
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn assessment(&self) -> ProviderAssessment {
        let coverage = self.coverage();
        let freshness = self.freshness();
        let active_incidents = self.active_incidents();
        let contradictions = self.contradictions(coverage, freshness, active_incidents.len());
        let confidence = self.confidence(coverage, freshness, &contradictions);
        let affected_surfaces = self.affected_surfaces();
        let mut reconciliation_notes = Vec::new();
        if !self.components.is_empty() {
            reconciliation_notes.push(format!(
                "{} component signal(s) normalized into the app health model.",
                self.components.len()
            ));
        }
        if !active_incidents.is_empty() {
            reconciliation_notes.push(format!(
                "{} active incident(s) contribute to the current assessment.",
                active_incidents.len()
            ));
        }
        if self.provenance == StatusProvenance::Fallback {
            reconciliation_notes.push(
                "Assessment uses fallback aggregator evidence because official data was unavailable."
                    .to_string(),
            );
        }
        if self.provenance == StatusProvenance::Unavailable {
            reconciliation_notes.push(
                "Assessment is constrained by missing machine-readable status data.".to_string(),
            );
        }

        let mut warnings = Vec::new();
        if self.provenance == StatusProvenance::Fallback {
            warnings.push(
                "Fallback data is lower-trust than an official machine-readable feed.".to_string(),
            );
        }
        if matches!(freshness, StatusFreshness::Stale | StatusFreshness::Unknown) {
            warnings.push(match freshness {
                StatusFreshness::Stale => {
                    "Status data appears stale; verify on the provider status page.".to_string()
                }
                StatusFreshness::Unknown => {
                    "Status source did not provide a reliable freshness timestamp.".to_string()
                }
                _ => unreachable!(),
            });
        }
        if coverage == StatusCoverage::None {
            warnings
                .push("No machine-readable coverage is available for this provider.".to_string());
        } else if coverage == StatusCoverage::SummaryOnly {
            warnings.push(
                "Coverage is summary-only; incidents/components may be missing from this view."
                    .to_string(),
            );
        }
        warnings.extend(contradictions.iter().map(|c| c.detail.clone()));
        if let Some(error) = &self.error {
            warnings.push(format!("Fetch error: {error}"));
        }

        let assessment_summary = self.assessment_summary(confidence, coverage, freshness);
        let evidence_summary = self.evidence_summary(coverage, active_incidents.len());

        ProviderAssessment {
            overall_health: self.health,
            confidence,
            coverage,
            freshness,
            active_incident_count: active_incidents.len(),
            affected_surfaces,
            assessment_summary,
            evidence_summary,
            reconciliation_notes,
            warnings,
            contradictions,
        }
    }

    fn coverage(&self) -> StatusCoverage {
        match self.provenance {
            StatusProvenance::Unavailable => StatusCoverage::None,
            StatusProvenance::Fallback => StatusCoverage::SummaryOnly,
            StatusProvenance::Official => match self.source_method {
                Some(
                    StatusSourceMethod::StatuspageV2
                    | StatusSourceMethod::IncidentIoShim
                    | StatusSourceMethod::BetterStack
                    | StatusSourceMethod::OnlineOrNot
                    | StatusSourceMethod::StatusIo,
                ) => StatusCoverage::Full,
                Some(StatusSourceMethod::Instatus) => {
                    if self.components.is_empty() {
                        StatusCoverage::IncidentOnly
                    } else {
                        StatusCoverage::Full
                    }
                }
                Some(StatusSourceMethod::GoogleCloudJson) => StatusCoverage::IncidentOnly,
                Some(StatusSourceMethod::Feed | StatusSourceMethod::ApiStatusCheck) => {
                    StatusCoverage::SummaryOnly
                }
                None => {
                    if self.summary.is_some() {
                        StatusCoverage::SummaryOnly
                    } else {
                        StatusCoverage::None
                    }
                }
            },
        }
    }

    fn freshness(&self) -> StatusFreshness {
        let Some(last_checked) = self.last_checked.as_deref() else {
            return StatusFreshness::Unknown;
        };
        let Some(parsed) = parse_status_timestamp(last_checked) else {
            return StatusFreshness::Unknown;
        };
        let age = Utc::now().signed_duration_since(parsed);
        if age <= Duration::hours(6) {
            StatusFreshness::Fresh
        } else if age <= Duration::hours(24) {
            StatusFreshness::Aging
        } else {
            StatusFreshness::Stale
        }
    }

    fn confidence(
        &self,
        coverage: StatusCoverage,
        freshness: StatusFreshness,
        contradictions: &[StatusContradiction],
    ) -> StatusConfidence {
        let mut confidence = match self.provenance {
            StatusProvenance::Official => match coverage {
                StatusCoverage::Full => StatusConfidence::High,
                StatusCoverage::IncidentOnly
                | StatusCoverage::ComponentOnly
                | StatusCoverage::SummaryOnly => StatusConfidence::Medium,
                StatusCoverage::None => StatusConfidence::None,
            },
            StatusProvenance::Fallback => StatusConfidence::Low,
            StatusProvenance::Unavailable => StatusConfidence::None,
        };

        if matches!(freshness, StatusFreshness::Stale | StatusFreshness::Unknown) {
            confidence = downgrade_confidence(confidence);
        }
        if !contradictions.is_empty() {
            confidence = downgrade_confidence(confidence);
        }
        confidence
    }

    fn affected_surfaces(&self) -> Vec<AffectedSurface> {
        let mut surfaces = BTreeSet::new();
        for text in self
            .components
            .iter()
            .map(|component| component.name.as_str())
            .chain(self.incidents.iter().map(|incident| incident.name.as_str()))
            .chain(
                self.incidents
                    .iter()
                    .flat_map(|incident| incident.affected_components.iter().map(String::as_str)),
            )
            .chain(
                self.scheduled_maintenances
                    .iter()
                    .flat_map(|maint| maint.affected_components.iter().map(String::as_str)),
            )
        {
            surfaces.insert(normalize_surface(text));
        }
        if surfaces.is_empty() {
            surfaces.insert(AffectedSurface::Unknown);
        }
        surfaces.into_iter().collect()
    }

    fn contradictions(
        &self,
        coverage: StatusCoverage,
        freshness: StatusFreshness,
        active_incident_count: usize,
    ) -> Vec<StatusContradiction> {
        let mut contradictions = Vec::new();
        let summary = self.summary.as_deref().unwrap_or_default().to_lowercase();
        let summary_claims_operational = summary.contains("operational")
            || summary.contains("all systems operational")
            || summary.contains("fully operational");
        let degraded_components = self
            .components
            .iter()
            .filter(|component| {
                matches!(
                    component_health(&component.status),
                    ProviderHealth::Degraded | ProviderHealth::Outage
                )
            })
            .count();

        if summary_claims_operational && active_incident_count > 0 {
            contradictions.push(StatusContradiction {
                summary: "Operational summary with active incident".to_string(),
                detail: format!(
                    "Source summary looks operational, but {} active incident(s) remain unresolved.",
                    active_incident_count
                ),
            });
        }
        if summary_claims_operational && degraded_components > 0 {
            contradictions.push(StatusContradiction {
                summary: "Operational summary with degraded components".to_string(),
                detail: format!(
                    "Source summary looks operational, but {} component(s) are degraded or in outage.",
                    degraded_components
                ),
            });
        }
        if self.provenance == StatusProvenance::Fallback
            && self.health == ProviderHealth::Operational
            && coverage == StatusCoverage::SummaryOnly
            && matches!(freshness, StatusFreshness::Stale | StatusFreshness::Unknown)
        {
            contradictions.push(StatusContradiction {
                summary: "Fallback operational snapshot is low-trust".to_string(),
                detail: "Fallback data reports operational status with limited or stale coverage."
                    .to_string(),
            });
        }
        contradictions
    }

    fn assessment_summary(
        &self,
        confidence: StatusConfidence,
        coverage: StatusCoverage,
        freshness: StatusFreshness,
    ) -> String {
        let source = self.source_label.as_deref().unwrap_or("No source");
        let summary = self
            .summary
            .as_deref()
            .unwrap_or("No provider summary was supplied.");
        format!(
            "{} is {} based on {} evidence ({}, {}, {}). {}",
            self.display_name,
            self.health.label().to_lowercase(),
            source,
            confidence.label().to_lowercase(),
            coverage.label().to_lowercase(),
            freshness.label().to_lowercase(),
            summary
        )
    }

    fn evidence_summary(&self, coverage: StatusCoverage, active_incident_count: usize) -> String {
        let component_count = self.components.len();
        let maintenance_count = self.scheduled_maintenances.len();
        match self.provenance {
            StatusProvenance::Official => format!(
                "Official {} feed with {} coverage: {} active incident(s), {} component signal(s), {} scheduled maintenance item(s).",
                self.source_method.map(|method| method.label()).unwrap_or("status"),
                coverage.label().to_lowercase(),
                active_incident_count,
                component_count,
                maintenance_count
            ),
            StatusProvenance::Fallback => format!(
                "Fallback {} snapshot with {} coverage. Raw incidents/components are unavailable in this adapter.",
                self.source_method.map(|method| method.label()).unwrap_or("status"),
                coverage.label().to_lowercase()
            ),
            StatusProvenance::Unavailable => {
                "No usable provider evidence could be loaded for this provider.".to_string()
            }
        }
    }
}

fn downgrade_confidence(confidence: StatusConfidence) -> StatusConfidence {
    match confidence {
        StatusConfidence::High => StatusConfidence::Medium,
        StatusConfidence::Medium => StatusConfidence::Low,
        StatusConfidence::Low | StatusConfidence::None => confidence,
    }
}

fn component_health(status: &str) -> ProviderHealth {
    let normalized = status.trim().to_lowercase();
    if normalized.contains("major_outage") || normalized.contains("outage") {
        ProviderHealth::Outage
    } else if normalized.contains("degraded") || normalized.contains("partial") {
        ProviderHealth::Degraded
    } else if normalized.contains("maint") {
        ProviderHealth::Maintenance
    } else {
        ProviderHealth::Operational
    }
}

fn normalize_surface(text: &str) -> AffectedSurface {
    let normalized = text.trim().to_lowercase();
    if normalized.is_empty() {
        return AffectedSurface::Unknown;
    }
    if normalized.contains("auth") || normalized.contains("login") || normalized.contains("oauth") {
        AffectedSurface::Auth
    } else if normalized.contains("chat") || normalized.contains("assistant") {
        AffectedSurface::Chat
    } else if normalized.contains("upload")
        || normalized.contains("file")
        || normalized.contains("storage")
    {
        AffectedSurface::UploadsFiles
    } else if normalized.contains("model")
        || normalized.contains("inference")
        || normalized.contains("completion")
        || normalized.contains("embedding")
        || normalized.contains("fine-tun")
    {
        AffectedSurface::ModelsInference
    } else if normalized.contains("console")
        || normalized.contains("dashboard")
        || normalized.contains("studio")
        || normalized.contains("portal")
    {
        AffectedSurface::Console
    } else if normalized.contains("api") || normalized.contains("gateway") {
        AffectedSurface::Api
    } else {
        AffectedSurface::Unknown
    }
}

fn parse_status_timestamp(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }

    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

impl ActiveIncident {
    pub fn is_active(&self) -> bool {
        let normalized = self.status.to_lowercase();
        !normalized.contains("resolved")
            && !normalized.contains("postmortem")
            && !normalized.contains("completed")
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
    fn best_open_url_prefers_official() {
        let status = ProviderStatus {
            slug: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            source_slug: "openai".to_string(),
            support_tier: StatusSupportTier::Required,
            health: ProviderHealth::Operational,
            provenance: StatusProvenance::Official,
            source_label: Some("OpenAI Status".to_string()),
            source_method: Some(StatusSourceMethod::StatuspageV2),
            official_url: Some("https://status.openai.com".to_string()),
            fallback_url: Some("https://apistatuscheck.com/api/openai".to_string()),
            last_checked: None,
            summary: None,
            components: Vec::new(),
            incidents: Vec::new(),
            scheduled_maintenances: Vec::new(),
            error: None,
        };

        assert_eq!(status.best_open_url(), Some("https://status.openai.com"));
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

    fn sample_status() -> ProviderStatus {
        ProviderStatus {
            slug: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            source_slug: "openai".to_string(),
            support_tier: StatusSupportTier::Required,
            health: ProviderHealth::Operational,
            provenance: StatusProvenance::Official,
            source_label: Some("OpenAI Status".to_string()),
            source_method: Some(StatusSourceMethod::StatuspageV2),
            official_url: Some("https://status.openai.com".to_string()),
            fallback_url: None,
            last_checked: Some(Utc::now().to_rfc3339()),
            summary: Some("All Systems Operational".to_string()),
            components: Vec::new(),
            incidents: Vec::new(),
            scheduled_maintenances: Vec::new(),
            error: None,
        }
    }

    #[test]
    fn assessment_surfaces_operational_contradictions() {
        let mut status = sample_status();
        status.components.push(ComponentStatus {
            name: "API".to_string(),
            status: "major_outage".to_string(),
            group_name: None,
        });
        status.incidents.push(ActiveIncident {
            name: "API elevated errors".to_string(),
            status: "investigating".to_string(),
            impact: "minor".to_string(),
            shortlink: None,
            created_at: None,
            updated_at: None,
            latest_update: None,
            affected_components: vec!["API".to_string()],
        });

        let assessment = status.assessment();
        assert_eq!(assessment.coverage, StatusCoverage::Full);
        assert_eq!(assessment.confidence, StatusConfidence::Medium);
        assert_eq!(assessment.active_incident_count, 1);
        assert!(assessment
            .contradictions
            .iter()
            .any(|entry| { entry.summary == "Operational summary with active incident" }));
        assert!(assessment.affected_surfaces.contains(&AffectedSurface::Api));
    }

    #[test]
    fn fallback_stale_summary_only_is_low_trust() {
        let mut status = sample_status();
        status.provenance = StatusProvenance::Fallback;
        status.source_method = Some(StatusSourceMethod::ApiStatusCheck);
        status.fallback_url = Some("https://apistatuscheck.com/api/openai".to_string());
        status.last_checked = Some((Utc::now() - Duration::hours(36)).to_rfc3339());

        let assessment = status.assessment();
        assert_eq!(assessment.coverage, StatusCoverage::SummaryOnly);
        assert_eq!(assessment.freshness, StatusFreshness::Stale);
        assert_eq!(assessment.confidence, StatusConfidence::Low);
        assert!(assessment
            .contradictions
            .iter()
            .any(|entry| { entry.summary == "Fallback operational snapshot is low-trust" }));
    }

    #[test]
    fn user_visible_caveat_prefers_simple_messages() {
        let mut fallback = sample_status();
        fallback.provenance = StatusProvenance::Fallback;
        assert_eq!(
            fallback.user_visible_caveat(),
            Some("Limited detail available")
        );

        let unavailable =
            ProviderStatus::placeholder(&status_seed_for_provider("some-unknown-provider"));
        assert_eq!(
            unavailable.user_visible_caveat(),
            Some("Status unavailable")
        );

        let mut stale = sample_status();
        stale.last_checked = Some((Utc::now() - Duration::hours(30)).to_rfc3339());
        assert_eq!(
            stale.user_visible_caveat(),
            Some("Verify details on the official status page")
        );
    }

    #[test]
    fn user_visible_affected_items_prefers_surface_labels() {
        let mut status = sample_status();
        status.incidents.push(ActiveIncident {
            name: "API elevated errors".to_string(),
            status: "investigating".to_string(),
            impact: "minor".to_string(),
            shortlink: None,
            created_at: None,
            updated_at: None,
            latest_update: None,
            affected_components: vec!["API".to_string(), "Auth".to_string()],
        });

        assert_eq!(
            status.user_visible_affected_items(),
            vec!["API".to_string(), "Auth".to_string()]
        );
    }

    #[test]
    fn unavailable_status_reports_missing_coverage() {
        let status =
            ProviderStatus::placeholder(&status_seed_for_provider("some-unknown-provider"));
        let assessment = status.assessment();
        assert_eq!(assessment.coverage, StatusCoverage::None);
        assert_eq!(assessment.confidence, StatusConfidence::None);
        assert!(assessment
            .warnings
            .iter()
            .any(|warning| { warning.contains("No machine-readable coverage") }));
    }
}
