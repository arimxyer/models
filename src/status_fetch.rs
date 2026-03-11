use serde::Deserialize;

use crate::status::{
    OfficialStatusSource, ProviderHealth, ProviderStatus, StatusProvenance, StatusProviderSeed,
    StatusSourceMethod, StatusStrategy,
};

const API_STATUS_CHECK_URL: &str = "https://apistatuscheck.com/api/status?api=";

#[derive(Debug)]
pub enum StatusFetchResult {
    Fresh(Vec<ProviderStatus>),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct StatusFetcher {
    client: reqwest::Client,
}

impl StatusFetcher {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("models-tui")
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }

    pub async fn fetch(&self, seeds: &[StatusProviderSeed]) -> StatusFetchResult {
        let mut entries = Vec::with_capacity(seeds.len());
        let mut successes = 0usize;
        let mut last_error: Option<String> = None;

        for seed in seeds {
            let resolved = match seed.strategy {
                StatusStrategy::OfficialFirst {
                    official,
                    fallback_source_slug,
                } => {
                    let official_result = self.fetch_official(official).await;
                    if official_result.is_ok() {
                        successes += 1;
                    } else if let Err(err) = &official_result {
                        last_error = Some(err.clone());
                    }

                    let fallback_result = match official_result {
                        Ok(_) => None,
                        Err(_) => match self.fetch_fallback(fallback_source_slug).await {
                            Ok(snapshot) => {
                                successes += 1;
                                Some(snapshot)
                            }
                            Err(err) => {
                                last_error = Some(err);
                                None
                            }
                        },
                    };

                    resolve_provider_status(seed, official_result.ok(), fallback_result)
                }
                StatusStrategy::FallbackOnly {
                    fallback_source_slug,
                } => {
                    let fallback_result = match self.fetch_fallback(fallback_source_slug).await {
                        Ok(snapshot) => {
                            successes += 1;
                            Some(snapshot)
                        }
                        Err(err) => {
                            last_error = Some(err);
                            None
                        }
                    };
                    resolve_provider_status(seed, None, fallback_result)
                }
                StatusStrategy::Unsupported => resolve_provider_status(seed, None, None),
            };

            entries.push(resolved);
        }

        entries.sort_by(|a, b| {
            a.health
                .sort_rank()
                .cmp(&b.health.sort_rank())
                .then_with(|| a.display_name.cmp(&b.display_name))
        });

        if successes == 0 && last_error.is_some() {
            return StatusFetchResult::Error(
                last_error.unwrap_or_else(|| "Failed to fetch provider statuses".to_string()),
            );
        }

        StatusFetchResult::Fresh(entries)
    }

    async fn fetch_official(
        &self,
        source: OfficialStatusSource,
    ) -> Result<OfficialSnapshot, String> {
        let response = self
            .client
            .get(source.summary_url())
            .send()
            .await
            .map_err(|err| err.to_string())?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        let payload: OfficialSummaryResponse =
            response.json().await.map_err(|err| err.to_string())?;
        Ok(OfficialSnapshot::from_summary(source, payload))
    }

    async fn fetch_fallback(&self, source_slug: &str) -> Result<FallbackSnapshot, String> {
        let response = self
            .client
            .get(format!("{API_STATUS_CHECK_URL}{source_slug}"))
            .send()
            .await
            .map_err(|err| err.to_string())?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err("not found".to_string());
        }

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        let payload: ApiStatusCheckResponse =
            response.json().await.map_err(|err| err.to_string())?;
        Ok(FallbackSnapshot::from_api_status(payload))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OfficialSnapshot {
    label: String,
    health: ProviderHealth,
    official_url: String,
    last_checked: Option<String>,
    summary: Option<String>,
}

impl OfficialSnapshot {
    fn from_summary(source: OfficialStatusSource, payload: OfficialSummaryResponse) -> Self {
        let incident_summary = payload.incidents.first().map(|incident| {
            format!(
                "{} ({}, {})",
                incident.name, incident.impact, incident.status
            )
        });

        Self {
            label: payload
                .page
                .name
                .or_else(|| Some(source.label().to_string()))
                .unwrap_or_else(|| source.label().to_string()),
            health: ProviderHealth::from_api_status(&payload.status.description),
            official_url: payload
                .page
                .url
                .unwrap_or_else(|| source.summary_url().to_string()),
            last_checked: payload.page.updated_at,
            summary: incident_summary.or(Some(payload.status.description)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FallbackSnapshot {
    label: String,
    health: ProviderHealth,
    official_url: Option<String>,
    fallback_url: String,
    last_checked: Option<String>,
    summary: Option<String>,
}

impl FallbackSnapshot {
    fn from_api_status(payload: ApiStatusCheckResponse) -> Self {
        Self {
            label: payload.api.name,
            health: ProviderHealth::from_api_status(&payload.api.status),
            official_url: Some(payload.api.status_page_url),
            fallback_url: payload.links.page,
            last_checked: payload.api.last_checked,
            summary: Some(payload.api.description),
        }
    }
}

fn resolve_provider_status(
    seed: &StatusProviderSeed,
    official: Option<OfficialSnapshot>,
    fallback: Option<FallbackSnapshot>,
) -> ProviderStatus {
    let mut status = ProviderStatus::placeholder(seed);

    if let Some(official) = official {
        status.health = official.health;
        status.provenance = StatusProvenance::Official;
        status.source_label = Some(official.label);
        status.source_method = Some(StatusSourceMethod::StatuspageV2);
        status.official_url = Some(official.official_url);
        status.last_checked = official.last_checked;
        status.summary = official.summary;
        return status;
    }

    if let Some(fallback) = fallback {
        status.health = fallback.health;
        status.provenance = StatusProvenance::Fallback;
        status.source_label = Some(fallback.label);
        status.source_method = Some(StatusSourceMethod::ApiStatusCheck);
        status.official_url = fallback.official_url;
        status.fallback_url = Some(fallback.fallback_url);
        status.last_checked = fallback.last_checked;
        status.summary = fallback.summary;
        return status;
    }

    status.summary = match seed.strategy {
        StatusStrategy::Unsupported => Some(
            "No verified machine-readable official or fallback source is configured for this provider.".to_string(),
        ),
        StatusStrategy::OfficialFirst { .. } => Some(
            "Official source unavailable and no fallback data could be loaded.".to_string(),
        ),
        StatusStrategy::FallbackOnly { .. } => {
            Some("Fallback source unavailable for this provider.".to_string())
        }
    };
    status
}

#[derive(Debug, Deserialize)]
struct ApiStatusCheckResponse {
    api: ApiStatusCheckApi,
    links: ApiStatusCheckLinks,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiStatusCheckApi {
    name: String,
    description: String,
    status_page_url: String,
    status: String,
    #[serde(default)]
    last_checked: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiStatusCheckLinks {
    page: String,
}

#[derive(Debug, Deserialize)]
struct OfficialSummaryResponse {
    page: OfficialPage,
    status: OfficialStatus,
    #[serde(default)]
    incidents: Vec<OfficialIncident>,
}

#[derive(Debug, Deserialize)]
struct OfficialPage {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OfficialStatus {
    description: String,
}

#[derive(Debug, Deserialize)]
struct OfficialIncident {
    name: String,
    status: String,
    impact: String,
}

#[cfg(test)]
mod tests {
    use crate::status::{
        display_name_for_provider, source_slug_for_provider, strategy_for_provider,
    };

    use super::*;

    fn seed(slug: &str) -> StatusProviderSeed {
        StatusProviderSeed {
            slug: slug.to_string(),
            display_name: display_name_for_provider(slug),
            source_slug: source_slug_for_provider(slug).to_string(),
            strategy: strategy_for_provider(slug),
        }
    }

    #[test]
    fn official_success_wins_over_fallback() {
        let status = resolve_provider_status(
            &seed("openai"),
            Some(OfficialSnapshot {
                label: "OpenAI".to_string(),
                health: ProviderHealth::Degraded,
                official_url: "https://status.openai.com".to_string(),
                last_checked: Some("2026-03-11T00:00:00Z".to_string()),
                summary: Some("Partial System Degradation".to_string()),
            }),
            Some(FallbackSnapshot {
                label: "OpenAI".to_string(),
                health: ProviderHealth::Operational,
                official_url: Some("https://status.openai.com".to_string()),
                fallback_url: "https://apistatuscheck.com/api/openai".to_string(),
                last_checked: Some("2026-03-11T00:00:00Z".to_string()),
                summary: Some("Fallback".to_string()),
            }),
        );

        assert_eq!(status.provenance, StatusProvenance::Official);
        assert_eq!(status.health, ProviderHealth::Degraded);
        assert_eq!(status.fallback_url, None);
    }

    #[test]
    fn official_failure_downgrades_to_fallback() {
        let status = resolve_provider_status(
            &seed("openai"),
            None,
            Some(FallbackSnapshot {
                label: "OpenAI".to_string(),
                health: ProviderHealth::Operational,
                official_url: Some("https://status.openai.com".to_string()),
                fallback_url: "https://apistatuscheck.com/api/openai".to_string(),
                last_checked: Some("2026-03-11T00:00:00Z".to_string()),
                summary: Some("Fallback".to_string()),
            }),
        );

        assert_eq!(status.provenance, StatusProvenance::Fallback);
        assert_eq!(status.best_open_url(), Some("https://status.openai.com"));
        assert_eq!(
            status.fallback_url.as_deref(),
            Some("https://apistatuscheck.com/api/openai")
        );
    }

    #[test]
    fn both_fail_stays_unavailable() {
        let status = resolve_provider_status(&seed("openai"), None, None);
        assert_eq!(status.provenance, StatusProvenance::Unavailable);
        assert_eq!(status.health, ProviderHealth::Unknown);
    }

    #[test]
    fn unsupported_provider_stays_unavailable() {
        let status = resolve_provider_status(&seed("moonshot"), None, None);
        assert_eq!(status.provenance, StatusProvenance::Unavailable);
        assert!(status
            .summary
            .as_deref()
            .unwrap_or_default()
            .contains("No verified machine-readable"));
    }
}
