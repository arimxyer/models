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
    /// Map Statuspage/incident.io `indicator` enum to health.
    pub fn from_indicator(indicator: &str) -> Self {
        match indicator {
            "none" => Self::Operational,
            "minor" => Self::Degraded,
            "major" => Self::Outage,
            "critical" => Self::Outage,
            "maintenance" => Self::Maintenance,
            _ => Self::Unknown,
        }
    }

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
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Self::Official => "Official",
            Self::Fallback => "Fallback",
            Self::Unavailable => "Unavailable",
        }
    }

    pub fn sort_rank(self) -> u8 {
        match self {
            Self::Official => 0,
            Self::Fallback => 1,
            Self::Unavailable => 2,
        }
    }

    #[allow(dead_code)]
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
    Zed,
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
            Self::Zed => "Zed Status",
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
            Self::Zed => "https://status.zed.dev/summary.json",
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
            Self::Zed => "https://status.zed.dev",
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

            Self::Perplexity | Self::Zed => StatusSourceMethod::Instatus,

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
    pub fn sort_rank(self) -> u8 {
        match self {
            Self::Required => 0,
            Self::Curated => 1,
            Self::Untracked => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusLoadState {
    #[default]
    Placeholder,
    Loaded,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusDetailAvailability {
    Available,
    NoneReported,
    Unsupported,
    FetchFailed,
    #[default]
    NotAttempted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusDetailSource {
    Inline,
    Enrichment,
    SummaryOnly,
    Derived,
    #[default]
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusDetailState {
    pub availability: StatusDetailAvailability,
    pub source: StatusDetailSource,
    pub note: Option<String>,
    pub error: Option<String>,
}

impl StatusDetailState {
    pub fn is_available(&self) -> bool {
        matches!(
            self.availability,
            StatusDetailAvailability::Available | StatusDetailAvailability::NoneReported
        )
    }

    pub fn is_none_reported(&self) -> bool {
        self.availability == StatusDetailAvailability::NoneReported
    }

    pub fn is_fetch_failed(&self) -> bool {
        self.availability == StatusDetailAvailability::FetchFailed
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentStatus {
    pub name: String,
    pub status: String,
    pub group_name: Option<String>,
    pub position: Option<u16>,
    pub only_show_if_degraded: bool,
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
    pub shortlink: Option<String>,
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
    pub load_state: StatusLoadState,
    pub source_label: Option<String>,
    pub source_method: Option<StatusSourceMethod>,
    pub official_url: Option<String>,
    pub fallback_url: Option<String>,
    pub source_updated_at: Option<String>,
    pub provider_summary: Option<String>,
    pub status_note: Option<String>,
    pub components: Vec<ComponentStatus>,
    pub components_state: StatusDetailState,
    pub incidents: Vec<ActiveIncident>,
    pub incidents_state: StatusDetailState,
    pub scheduled_maintenances: Vec<ScheduledMaintenance>,
    pub scheduled_maintenances_state: StatusDetailState,
    pub official_error: Option<String>,
    pub fallback_error: Option<String>,
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
            load_state: StatusLoadState::Placeholder,
            source_label: None,
            source_method: None,
            official_url: None,
            fallback_url: None,
            source_updated_at: None,
            provider_summary: None,
            status_note: None,
            components: Vec::new(),
            components_state: StatusDetailState::default(),
            incidents: Vec::new(),
            incidents_state: StatusDetailState::default(),
            scheduled_maintenances: Vec::new(),
            scheduled_maintenances_state: StatusDetailState::default(),
            official_error: None,
            fallback_error: None,
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

    pub fn component_detail_available(&self) -> bool {
        self.components_state.is_available()
    }

    pub fn incident_detail_available(&self) -> bool {
        self.incidents_state.is_available()
    }

    pub fn maintenance_detail_available(&self) -> bool {
        self.scheduled_maintenances_state.is_available()
    }

    pub fn confirmed_no_components(&self) -> bool {
        self.components_state.is_none_reported()
    }

    pub fn confirmed_no_incidents(&self) -> bool {
        self.incidents_state.is_none_reported()
    }

    #[allow(dead_code)]
    pub fn confirmed_no_maintenance(&self) -> bool {
        self.scheduled_maintenances_state.is_none_reported()
    }

    pub fn has_detail_fetch_failures(&self) -> bool {
        self.components_state.is_fetch_failed()
            || self.incidents_state.is_fetch_failed()
            || self.scheduled_maintenances_state.is_fetch_failed()
    }

    pub fn has_partial_data(&self) -> bool {
        self.load_state == StatusLoadState::Partial || self.has_detail_fetch_failures()
    }

    pub fn provider_summary_text(&self) -> Option<&str> {
        self.provider_summary.as_deref()
    }

    pub fn status_note_text(&self) -> Option<&str> {
        self.status_note.as_deref()
    }

    pub fn error_summary(&self) -> Option<String> {
        match (&self.official_error, &self.fallback_error) {
            (Some(official), Some(fallback)) => {
                Some(format!("official: {official}; fallback: {fallback}"))
            }
            (Some(official), None) => Some(format!("official: {official}")),
            (None, Some(fallback)) => Some(format!("fallback: {fallback}")),
            (None, None) => None,
        }
    }

    pub fn detail_state_message(&self, state: &StatusDetailState, label: &str) -> Option<String> {
        match state.availability {
            StatusDetailAvailability::Available | StatusDetailAvailability::NoneReported => None,
            StatusDetailAvailability::Unsupported => Some(
                state
                    .note
                    .clone()
                    .unwrap_or_else(|| format!("{label} unavailable")),
            ),
            StatusDetailAvailability::FetchFailed => Some(
                state
                    .error
                    .clone()
                    .unwrap_or_else(|| format!("{label} failed to load")),
            ),
            StatusDetailAvailability::NotAttempted => Some(
                state
                    .note
                    .clone()
                    .unwrap_or_else(|| format!("{label} not loaded")),
            ),
        }
    }

    /// Count of non-operational components (outage, degraded, or maintenance).
    /// Combined issue count for display badges: active incidents or degraded
    /// components (excluding maintenance — planned work is not an issue).
    pub fn issue_count(&self) -> usize {
        let incidents = if self.incident_detail_available() {
            self.active_incidents().len()
        } else {
            0
        };
        // Only count degraded/outage components as issues, not maintenance
        let degraded_components = if self.component_detail_available() {
            self.components
                .iter()
                .filter(|c| {
                    let s = c.status.to_lowercase();
                    !s.contains("operational")
                        && !s.contains("maint")
                        && s != "unknown"
                        && !s.is_empty()
                })
                .count()
        } else {
            0
        };
        // If there are incidents, they typically cover the component issues.
        // Show the larger of the two to avoid double-counting.
        incidents.max(degraded_components)
    }
}

// ---------------------------------------------------------------------------
// Snapshot types (moved from status_fetch.rs per Option A)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OfficialSnapshot {
    pub label: String,
    pub method: StatusSourceMethod,
    pub health: ProviderHealth,
    pub official_url: String,
    pub source_updated_at: Option<String>,
    pub provider_summary: Option<String>,
    pub status_note: Option<String>,
    pub components: Vec<ComponentStatus>,
    pub components_state: StatusDetailState,
    pub incidents: Vec<ActiveIncident>,
    pub incidents_state: StatusDetailState,
    pub maintenance: Vec<ScheduledMaintenance>,
    pub maintenance_state: StatusDetailState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FallbackSnapshot {
    pub label: String,
    pub health: ProviderHealth,
    pub official_url: Option<String>,
    pub fallback_url: String,
    pub source_updated_at: Option<String>,
    pub provider_summary: Option<String>,
}

// ---------------------------------------------------------------------------
// Detail state constructors (moved from status_fetch.rs per Option A)
// ---------------------------------------------------------------------------

pub(crate) fn available_detail_state<T>(
    items: &[T],
    source: StatusDetailSource,
) -> StatusDetailState {
    StatusDetailState {
        availability: if items.is_empty() {
            StatusDetailAvailability::NoneReported
        } else {
            StatusDetailAvailability::Available
        },
        source,
        note: None,
        error: None,
    }
}

pub(crate) fn unsupported_detail_state(note: impl Into<String>) -> StatusDetailState {
    StatusDetailState {
        availability: StatusDetailAvailability::Unsupported,
        source: StatusDetailSource::None,
        note: Some(note.into()),
        error: None,
    }
}

pub(crate) fn not_attempted_detail_state(
    source: StatusDetailSource,
    note: impl Into<String>,
) -> StatusDetailState {
    StatusDetailState {
        availability: StatusDetailAvailability::NotAttempted,
        source,
        note: Some(note.into()),
        error: None,
    }
}

pub(crate) fn fetch_failed_detail_state(
    source: StatusDetailSource,
    error: impl Into<String>,
) -> StatusDetailState {
    StatusDetailState {
        availability: StatusDetailAvailability::FetchFailed,
        source,
        note: None,
        error: Some(error.into()),
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
    fn best_open_url_prefers_official() {
        let status = ProviderStatus {
            slug: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            source_slug: "openai".to_string(),
            support_tier: StatusSupportTier::Required,
            health: ProviderHealth::Operational,
            provenance: StatusProvenance::Official,
            load_state: StatusLoadState::Loaded,
            source_label: Some("OpenAI Status".to_string()),
            source_method: Some(StatusSourceMethod::StatuspageV2),
            official_url: Some("https://status.openai.com".to_string()),
            fallback_url: Some("https://apistatuscheck.com/api/openai".to_string()),
            source_updated_at: None,
            provider_summary: None,
            status_note: None,
            components: Vec::new(),
            components_state: StatusDetailState {
                availability: StatusDetailAvailability::NoneReported,
                source: StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            incidents: Vec::new(),
            incidents_state: StatusDetailState {
                availability: StatusDetailAvailability::NoneReported,
                source: StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            scheduled_maintenances: Vec::new(),
            scheduled_maintenances_state: StatusDetailState {
                availability: StatusDetailAvailability::NoneReported,
                source: StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            official_error: None,
            fallback_error: None,
        };

        assert_eq!(status.best_open_url(), Some("https://status.openai.com"));
    }
}
