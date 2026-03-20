use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;

use super::adapters::betterstack::parse_better_stack;
use super::adapters::fallback::ApiStatusCheckResponse;
use super::adapters::google::{build_google_snapshot, GoogleIncident, GoogleProductsResponse};
use super::adapters::instatus::fetch_instatus_with_components;
use super::adapters::onlineornot::parse_onlineornot;
use super::adapters::status_io::parse_status_io;
use super::adapters::statuspage::{
    fetch_incident_io_shim, fetch_maintenance_enrichment, parse_statuspage_v2_summary,
};
use super::types::{
    available_detail_state, FallbackSnapshot, OfficialSnapshot, OfficialStatusSource,
    ProviderHealth, ProviderStatus, StatusDetailAvailability, StatusDetailSource,
    StatusDetailState, StatusLoadState, StatusProvenance, StatusProviderSeed, StatusSourceMethod,
    StatusStrategy,
};

const API_STATUS_CHECK_URL: &str = "https://apistatuscheck.com/api/status?api=";
const GOOGLE_PRODUCTS_URL: &str = "https://status.cloud.google.com/products.json";

#[derive(Debug)]
pub enum StatusFetchResult {
    Fresh(Vec<ProviderStatus>),
}

#[derive(Debug, Clone)]
pub struct StatusFetcher {
    client: reqwest::Client,
}

impl StatusFetcher {
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn fetch(&self, seeds: &[StatusProviderSeed]) -> StatusFetchResult {
        let needs_google = seeds.iter().any(|seed| {
            matches!(
                seed.strategy,
                StatusStrategy::OfficialFirst {
                    official: OfficialStatusSource::GoogleGeminiJson,
                    ..
                }
            )
        });

        let google_products: Option<Arc<GoogleProductsResponse>> = if needs_google {
            match timeout(Duration::from_secs(5), fetch_google_products(&self.client)).await {
                Ok(Ok(products)) => Some(Arc::new(products)),
                _ => None,
            }
        } else {
            None
        };

        let mut set = tokio::task::JoinSet::new();
        let mut results: Vec<(usize, ProviderStatus)> = Vec::with_capacity(seeds.len());

        for (i, seed) in seeds.iter().enumerate() {
            // True bounded concurrency: drain a slot if 10 in-flight
            while set.len() >= 10 {
                if let Some(res) = set.join_next().await {
                    match res {
                        Ok(result) => results.push(result),
                        Err(_join_err) => { /* panicked task, skip */ }
                    }
                }
            }
            let client = self.client.clone();
            let seed = seed.clone();
            let google = google_products.clone();
            set.spawn(async move { (i, fetch_single(client, seed, google).await) });
        }
        // Drain remaining
        while let Some(res) = set.join_next().await {
            match res {
                Ok(result) => results.push(result),
                Err(_join_err) => { /* panicked task, skip */ }
            }
        }

        results.sort_by(|a, b| {
            a.1.health
                .sort_rank()
                .cmp(&b.1.health.sort_rank())
                .then_with(|| {
                    a.1.support_tier
                        .sort_rank()
                        .cmp(&b.1.support_tier.sort_rank())
                })
                .then_with(|| a.1.provenance.sort_rank().cmp(&b.1.provenance.sort_rank()))
                .then_with(|| a.1.display_name.cmp(&b.1.display_name))
        });

        let entries: Vec<ProviderStatus> = results.into_iter().map(|(_, s)| s).collect();

        // Return per-provider entries even when all are Unknown — they contain
        // individual error details and URLs that the UI can display.
        StatusFetchResult::Fresh(entries)
    }
}

// ---------------------------------------------------------------------------
// Free async functions (extracted for future JoinSet::spawn compatibility)
// ---------------------------------------------------------------------------

pub(crate) async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    timeout(Duration::from_secs(5), async {
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|err| err.to_string())?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        response.text().await.map_err(|err| err.to_string())
    })
    .await
    .map_err(|_| "timed out after 5s".to_string())?
}

async fn fetch_google_products(client: &reqwest::Client) -> Result<GoogleProductsResponse, String> {
    let body = fetch_text(client, GOOGLE_PRODUCTS_URL).await?;
    serde_json::from_str(&body).map_err(|err| err.to_string())
}

async fn fetch_official(
    client: &reqwest::Client,
    source: OfficialStatusSource,
    google_products: Option<&GoogleProductsResponse>,
) -> Result<OfficialSnapshot, String> {
    match source.source_method() {
        StatusSourceMethod::StatuspageV2 => {
            let body = fetch_text(client, source.endpoint_url()).await?;
            let mut snapshot = parse_statuspage_v2_summary(source, &body)?;

            // Enrich maintenance if summary returned none
            if snapshot.maintenance.is_empty() {
                match fetch_maintenance_enrichment(client, source.page_url()).await {
                    Ok(maintenances) => {
                        snapshot.maintenance = maintenances;
                        snapshot.maintenance_state = available_detail_state(
                            &snapshot.maintenance,
                            StatusDetailSource::Enrichment,
                        );
                    }
                    Err(_) => {
                        // Keep the inline state (NoneReported/Inline) — enrichment
                        // failure is not worth surfacing when inline already succeeded.
                    }
                }
            }

            Ok(snapshot)
        }
        StatusSourceMethod::IncidentIoShim => fetch_incident_io_shim(client, source).await,
        StatusSourceMethod::BetterStack => {
            let body = fetch_text(client, source.endpoint_url()).await?;
            parse_better_stack(source, &body)
        }
        StatusSourceMethod::OnlineOrNot => {
            let body = fetch_text(client, source.endpoint_url()).await?;
            parse_onlineornot(source, &body)
        }
        StatusSourceMethod::StatusIo => {
            let body = fetch_text(client, source.endpoint_url()).await?;
            parse_status_io(source, &body)
        }
        StatusSourceMethod::Instatus => fetch_instatus_with_components(client, source).await,
        StatusSourceMethod::Feed => Err("Feed parsing not supported".to_string()),
        StatusSourceMethod::GoogleCloudJson => {
            let products =
                google_products.ok_or_else(|| "missing google products catalog".to_string())?;
            let product = products
                .products
                .iter()
                .find(|product| product.title == "Vertex Gemini API")
                .ok_or_else(|| {
                    "Vertex Gemini API not found in Google products catalog".to_string()
                })?;

            let body = fetch_text(client, source.endpoint_url()).await?;
            let incidents: Vec<GoogleIncident> =
                serde_json::from_str(&body).map_err(|err| err.to_string())?;
            Ok(build_google_snapshot(product, &incidents))
        }
        StatusSourceMethod::ApiStatusCheck => {
            Err("ApiStatusCheck is not an official source method".to_string())
        }
    }
}

async fn fetch_fallback(
    client: &reqwest::Client,
    source_slug: &str,
) -> Result<FallbackSnapshot, String> {
    timeout(Duration::from_secs(5), async {
        let response = client
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
    })
    .await
    .map_err(|_| "timed out after 5s".to_string())?
}

async fn fetch_single(
    client: reqwest::Client,
    seed: StatusProviderSeed,
    google_products: Option<Arc<GoogleProductsResponse>>,
) -> ProviderStatus {
    match seed.strategy {
        StatusStrategy::OfficialFirst {
            official,
            fallback_source_slug,
        } => {
            let official_result =
                fetch_official(&client, official, google_products.as_deref()).await;

            let official_err = official_result.as_ref().err().cloned();

            let fallback_result = match (&official_result, fallback_source_slug) {
                (Ok(_), _) | (_, None) => Ok(None),
                (Err(_), Some(slug)) => match fetch_fallback(&client, slug).await {
                    Ok(snapshot) => Ok(Some(snapshot)),
                    Err(e) => Err(e),
                },
            };

            let fallback_err = fallback_result.as_ref().err().cloned();

            let mut status = resolve_provider_status(
                &seed,
                official_result.ok(),
                fallback_result.ok().flatten(),
            );

            status.official_error = official_err;
            status.fallback_error = fallback_err;

            status
        }
        StatusStrategy::Unverified => resolve_provider_status(&seed, None, None),
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
        status.load_state = if official.components_state.is_fetch_failed()
            || official.incidents_state.is_fetch_failed()
            || official.maintenance_state.is_fetch_failed()
        {
            StatusLoadState::Partial
        } else {
            StatusLoadState::Loaded
        };
        status.source_label = Some(official.label);
        status.source_method = Some(official.method);
        status.official_url = Some(official.official_url);
        status.source_updated_at = official.source_updated_at;
        status.provider_summary = official.provider_summary;
        status.status_note = official.status_note;
        status.components = official.components;
        status.components_state = official.components_state;
        status.incidents = official.incidents;
        status.incidents_state = official.incidents_state;
        status.scheduled_maintenances = official.maintenance;
        status.scheduled_maintenances_state = official.maintenance_state;

        // Reconcile health: cross-check API-declared status against
        // actual incidents and component statuses. Take the worst.
        let mut reconciled = status.health;

        // Active (unresolved) incidents imply at least Degraded
        let has_active_incident = status.incidents.iter().any(|i| {
            let s = i.status.to_lowercase();
            !s.contains("resolved") && !s.contains("postmortem") && !s.contains("completed")
        });
        if has_active_incident && reconciled == ProviderHealth::Operational {
            reconciled = ProviderHealth::Degraded;
        }

        // Component-aware health: consider severity AND proportion.
        // A single component outage among many healthy ones → Degraded, not Outage.
        if !status.components.is_empty() {
            let total = status.components.len();
            let mut outage_count = 0usize;
            let mut degraded_count = 0usize;
            let mut maintenance_count = 0usize;

            for comp in &status.components {
                match comp.status.as_str() {
                    "major_outage" => outage_count += 1,
                    "partial_outage" | "degraded_performance" => degraded_count += 1,
                    "under_maintenance" => maintenance_count += 1,
                    _ => {}
                }
            }

            let affected = outage_count + degraded_count;
            let worst_component = if outage_count > 0 {
                ProviderHealth::Outage
            } else if degraded_count > 0 {
                ProviderHealth::Degraded
            } else if maintenance_count > 0 {
                ProviderHealth::Maintenance
            } else {
                ProviderHealth::Operational
            };

            // Only promote to full Outage if a significant fraction of
            // components are affected (>= 1/3) or if the majority are in outage.
            // Otherwise cap at Degraded — a single service down among many
            // healthy ones is a partial degradation, not a full outage.
            let component_health =
                if worst_component == ProviderHealth::Outage && affected * 3 < total {
                    ProviderHealth::Degraded
                } else {
                    worst_component
                };

            if component_health.sort_rank() < reconciled.sort_rank() {
                reconciled = component_health;
            }
        }

        status.health = reconciled;
        return status;
    }

    if let Some(fallback) = fallback {
        status.health = fallback.health;
        status.provenance = StatusProvenance::Fallback;
        status.load_state = StatusLoadState::Loaded;
        status.source_label = Some(fallback.label);
        status.source_method = Some(StatusSourceMethod::ApiStatusCheck);
        status.official_url = fallback.official_url;
        status.fallback_url = Some(fallback.fallback_url);
        status.source_updated_at = fallback.source_updated_at;
        status.provider_summary = fallback.provider_summary;
        status.status_note =
            Some("Fallback adapter exposes only provider-level summary status.".to_string());
        status.components_state = StatusDetailState {
            availability: StatusDetailAvailability::Unsupported,
            source: StatusDetailSource::SummaryOnly,
            note: Some("Service details are unavailable from the fallback adapter.".to_string()),
            error: None,
        };
        status.incidents_state = StatusDetailState {
            availability: StatusDetailAvailability::Unsupported,
            source: StatusDetailSource::SummaryOnly,
            note: Some("Incident details are unavailable from the fallback adapter.".to_string()),
            error: None,
        };
        status.scheduled_maintenances_state = StatusDetailState {
            availability: StatusDetailAvailability::Unsupported,
            source: StatusDetailSource::SummaryOnly,
            note: Some(
                "Maintenance details are unavailable from the fallback adapter.".to_string(),
            ),
            error: None,
        };
        return status;
    }

    status.load_state = StatusLoadState::Failed;
    status.status_note = match seed.strategy {
        StatusStrategy::Unverified => Some(
            "No verified machine-readable official or fallback source has been added for this provider yet."
                .to_string(),
        ),
        StatusStrategy::OfficialFirst {
            fallback_source_slug: Some(_),
            ..
        } => Some("Official source unavailable and no fallback data could be loaded.".to_string()),
        StatusStrategy::OfficialFirst {
            fallback_source_slug: None,
            ..
        } => Some("Official source unavailable and no fallback source is configured.".to_string()),
    };
    let unavailable_state = match seed.strategy {
        StatusStrategy::Unverified => StatusDetailState {
            availability: StatusDetailAvailability::Unsupported,
            source: StatusDetailSource::None,
            note: Some(
                "No verified machine-readable source is configured for this provider.".to_string(),
            ),
            error: None,
        },
        StatusStrategy::OfficialFirst { .. } => StatusDetailState {
            availability: StatusDetailAvailability::FetchFailed,
            source: StatusDetailSource::None,
            note: None,
            error: Some(
                "No provider detail could be loaded from the configured status sources."
                    .to_string(),
            ),
        },
    };
    status.components_state = unavailable_state.clone();
    status.incidents_state = unavailable_state.clone();
    status.scheduled_maintenances_state = unavailable_state;
    status
}

#[cfg(test)]
mod tests {
    use crate::status::{
        status_seed_for_provider, StatusDetailAvailability, StatusDetailSource, StatusDetailState,
        StatusLoadState, StatusProvenance,
    };

    use super::*;

    fn seed(slug: &str) -> StatusProviderSeed {
        status_seed_for_provider(slug)
    }

    fn inline_none_state() -> StatusDetailState {
        StatusDetailState {
            availability: StatusDetailAvailability::NoneReported,
            source: StatusDetailSource::Inline,
            note: None,
            error: None,
        }
    }

    #[test]
    fn official_success_wins_over_fallback() {
        let status = resolve_provider_status(
            &seed("openai"),
            Some(OfficialSnapshot {
                label: "OpenAI".to_string(),
                method: StatusSourceMethod::StatuspageV2,
                health: ProviderHealth::Degraded,
                official_url: "https://status.openai.com".to_string(),
                source_updated_at: Some("2026-03-11T00:00:00Z".to_string()),
                provider_summary: Some("Partial System Degradation".to_string()),
                status_note: None,
                components: Vec::new(),
                components_state: inline_none_state(),
                incidents: Vec::new(),
                incidents_state: inline_none_state(),
                maintenance: Vec::new(),
                maintenance_state: inline_none_state(),
            }),
            Some(FallbackSnapshot {
                label: "OpenAI".to_string(),
                health: ProviderHealth::Operational,
                official_url: Some("https://status.openai.com".to_string()),
                fallback_url: "https://apistatuscheck.com/api/openai".to_string(),
                source_updated_at: Some("2026-03-11T00:00:00Z".to_string()),
                provider_summary: Some("Fallback".to_string()),
            }),
        );

        assert_eq!(status.provenance, StatusProvenance::Official);
        assert_eq!(status.load_state, StatusLoadState::Loaded);
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
                source_updated_at: Some("2026-03-11T00:00:00Z".to_string()),
                provider_summary: Some("Fallback".to_string()),
            }),
        );

        assert_eq!(status.provenance, StatusProvenance::Fallback);
        assert_eq!(
            status.components_state.availability,
            StatusDetailAvailability::Unsupported
        );
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
        assert_eq!(status.load_state, StatusLoadState::Failed);
        assert_eq!(status.health, ProviderHealth::Unknown);
    }

    #[test]
    fn unverified_provider_stays_unavailable() {
        let status = resolve_provider_status(&seed("some-nonexistent-provider"), None, None);
        assert_eq!(status.provenance, StatusProvenance::Unavailable);
        assert!(status
            .status_note
            .as_deref()
            .unwrap_or_default()
            .contains("added for this provider yet"));
    }
}
