use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;
use tokio::time::timeout;

use crate::status::{
    ActiveIncident, ComponentStatus, IncidentUpdate, OfficialStatusSource, ProviderHealth,
    ProviderStatus, ScheduledMaintenance, StatusDetailAvailability, StatusDetailSource,
    StatusDetailState, StatusLoadState, StatusProvenance, StatusProviderSeed, StatusSourceMethod,
    StatusStrategy,
};

const API_STATUS_CHECK_URL: &str = "https://apistatuscheck.com/api/status?api=";
const GOOGLE_PRODUCTS_URL: &str = "https://status.cloud.google.com/products.json";

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

        if entries.iter().all(|e| e.health == ProviderHealth::Unknown) {
            return StatusFetchResult::Error("Failed to fetch provider statuses".to_string());
        }

        StatusFetchResult::Fresh(entries)
    }
}

// ---------------------------------------------------------------------------
// Free async functions (extracted for future JoinSet::spawn compatibility)
// ---------------------------------------------------------------------------

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
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
            parse_statuspage_v2_summary(source, &body)
        }
        StatusSourceMethod::IncidentIoShim => {
            let body = fetch_text(client, source.endpoint_url()).await?;
            let mut snapshot = parse_statuspage_v2_summary(source, &body)?;
            snapshot.incidents.clear();
            snapshot.incidents_state = not_attempted_detail_state(
                StatusDetailSource::Enrichment,
                "Incident details require a second incident feed for this source.",
            );
            let incidents_url = format!("{}/api/v2/incidents.json", source.page_url());
            match timeout(Duration::from_secs(3), async {
                let text = fetch_text(client, &incidents_url).await?;
                parse_incidents_json(&text)
            })
            .await
            {
                Ok(Ok(incidents)) => {
                    snapshot.incidents = incidents;
                    snapshot.incidents_state =
                        available_detail_state(&snapshot.incidents, StatusDetailSource::Enrichment);
                }
                Ok(Err(err)) => {
                    snapshot.incidents_state =
                        fetch_failed_detail_state(StatusDetailSource::Enrichment, err);
                }
                Err(_) => {
                    snapshot.incidents_state = fetch_failed_detail_state(
                        StatusDetailSource::Enrichment,
                        "Incident details timed out after 3s.",
                    );
                }
            }
            Ok(snapshot)
        }
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
        StatusSourceMethod::Instatus => {
            let body = fetch_text(client, source.endpoint_url()).await?;
            let mut snapshot = parse_instatus_summary(source, &body)?;
            let components_url = format!("{}/v2/components.json", source.page_url());
            match timeout(Duration::from_secs(3), async {
                let text = fetch_text(client, &components_url).await?;
                parse_instatus_components(&text)
            })
            .await
            {
                Ok(Ok(components)) => {
                    snapshot.components = components;
                    snapshot.components_state = available_detail_state(
                        &snapshot.components,
                        StatusDetailSource::Enrichment,
                    );
                }
                Ok(Err(err)) => {
                    snapshot.components_state =
                        fetch_failed_detail_state(StatusDetailSource::Enrichment, err);
                }
                Err(_) => {
                    snapshot.components_state = fetch_failed_detail_state(
                        StatusDetailSource::Enrichment,
                        "Service details timed out after 3s.",
                    );
                }
            }
            Ok(snapshot)
        }
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
            Ok(OfficialSnapshot::from_google(product, &incidents))
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct OfficialSnapshot {
    label: String,
    method: StatusSourceMethod,
    health: ProviderHealth,
    official_url: String,
    source_updated_at: Option<String>,
    provider_summary: Option<String>,
    status_note: Option<String>,
    components: Vec<ComponentStatus>,
    components_state: StatusDetailState,
    incidents: Vec<ActiveIncident>,
    incidents_state: StatusDetailState,
    maintenance: Vec<ScheduledMaintenance>,
    maintenance_state: StatusDetailState,
}

impl OfficialSnapshot {
    fn from_google(product: &GoogleProduct, incidents: &[GoogleIncident]) -> Self {
        let matching: Vec<_> = incidents
            .iter()
            .filter(|incident| {
                incident
                    .affected_products
                    .iter()
                    .any(|p| p.id == product.id)
            })
            .collect();

        let latest = matching
            .iter()
            .max_by_key(|incident| incident.modified.as_deref().unwrap_or(""));

        let active = matching.iter().any(|incident| incident.end.is_none());
        let health = if active {
            latest
                .map(|incident| {
                    if incident.status_impact == "SERVICE_OUTAGE" || incident.severity == "high" {
                        ProviderHealth::Outage
                    } else {
                        ProviderHealth::Degraded
                    }
                })
                .unwrap_or(ProviderHealth::Degraded)
        } else {
            ProviderHealth::Operational
        };

        let summary = latest.map(|incident| incident.external_desc.clone());
        let last_checked = latest.and_then(|incident| incident.modified.clone());
        let components_state = unsupported_detail_state(
            "Service details are not exposed as component rows by this Google adapter.",
        );
        let incidents_state = not_attempted_detail_state(
            StatusDetailSource::Derived,
            "Raw Google incident details are not preserved by this adapter yet.",
        );
        let maintenance_state = unsupported_detail_state(
            "Scheduled maintenance details are not exposed by this Google adapter.",
        );

        Self {
            label: product.title.clone(),
            method: StatusSourceMethod::GoogleCloudJson,
            health,
            official_url: format!(
                "https://status.cloud.google.com/products/{}/history",
                product.id
            ),
            source_updated_at: last_checked,
            provider_summary: summary,
            status_note: Some(
                "Google Cloud incidents are currently summarized into provider-level status only."
                    .to_string(),
            ),
            components_state,
            components: Vec::new(),
            incidents_state,
            incidents: Vec::new(),
            maintenance_state,
            maintenance: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FallbackSnapshot {
    label: String,
    health: ProviderHealth,
    official_url: Option<String>,
    fallback_url: String,
    source_updated_at: Option<String>,
    provider_summary: Option<String>,
}

impl FallbackSnapshot {
    fn from_api_status(payload: ApiStatusCheckResponse) -> Self {
        Self {
            label: payload.api.name,
            health: ProviderHealth::from_api_status(&payload.api.status),
            official_url: Some(payload.api.status_page_url),
            fallback_url: payload.links.page,
            source_updated_at: payload.api.last_checked,
            provider_summary: Some(payload.api.description),
        }
    }
}

fn available_detail_state<T>(items: &[T], source: StatusDetailSource) -> StatusDetailState {
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

fn unsupported_detail_state(note: impl Into<String>) -> StatusDetailState {
    StatusDetailState {
        availability: StatusDetailAvailability::Unsupported,
        source: StatusDetailSource::None,
        note: Some(note.into()),
        error: None,
    }
}

fn not_attempted_detail_state(
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

fn fetch_failed_detail_state(
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

// ---------------------------------------------------------------------------
// Normalize component status strings from various platforms
// ---------------------------------------------------------------------------

fn normalize_component_status(raw: &str) -> String {
    match raw {
        // Instatus (UPPERCASECONCATENATED)
        "OPERATIONAL" => "operational".to_string(),
        "DEGRADEDPERFORMANCE" => "degraded_performance".to_string(),
        "UNDERMAINTENANCE" => "under_maintenance".to_string(),
        "MAJOROUTAGE" => "major_outage".to_string(),
        "PARTIALOUTAGE" => "partial_outage".to_string(),
        // Better Stack / OnlineOrNot
        "degraded" => "degraded_performance".to_string(),
        "downtime" | "outage" => "major_outage".to_string(),
        // Already normalized or unknown — lowercase passthrough
        other => other.to_lowercase(),
    }
}

// ---------------------------------------------------------------------------
// Statuspage V2 / incident.io parser (summary.json)
// ---------------------------------------------------------------------------

fn parse_statuspage_v2_summary(
    source: OfficialStatusSource,
    body: &str,
) -> Result<OfficialSnapshot, String> {
    let payload: OfficialSummaryResponse =
        serde_json::from_str(body).map_err(|err| err.to_string())?;

    let incident_summary = payload.incidents.first().map(|incident| {
        format!(
            "{} ({}, {})",
            incident.name, incident.impact, incident.status
        )
    });

    let components: Vec<ComponentStatus> = payload
        .components
        .iter()
        .map(|c| ComponentStatus {
            name: c.name.clone(),
            status: c.status.clone(),
            group_name: c.group_name.clone(),
        })
        .collect();

    let incidents: Vec<ActiveIncident> = payload
        .incidents
        .iter()
        .map(|i| ActiveIncident {
            name: i.name.clone(),
            status: i.status.clone(),
            impact: i.impact.clone(),
            shortlink: i.shortlink.clone(),
            created_at: i.created_at.clone(),
            updated_at: i.updated_at.clone(),
            latest_update: i.incident_updates.first().map(|u| IncidentUpdate {
                status: u.status.clone(),
                body: u.body.clone(),
                created_at: u.created_at.clone().unwrap_or_default(),
            }),
            affected_components: i.components.iter().map(|c| c.name.clone()).collect(),
        })
        .collect();

    let maintenance: Vec<ScheduledMaintenance> = payload
        .scheduled_maintenances
        .iter()
        .map(|m| ScheduledMaintenance {
            name: m.name.clone(),
            status: m.status.clone(),
            impact: m.impact.clone().unwrap_or_default(),
            scheduled_for: m.scheduled_for.clone(),
            scheduled_until: m.scheduled_until.clone(),
            affected_components: m.components.iter().map(|c| c.name.clone()).collect(),
        })
        .collect();
    let components_state = available_detail_state(&components, StatusDetailSource::Inline);
    let incidents_state = available_detail_state(&incidents, StatusDetailSource::Inline);
    let maintenance_state = available_detail_state(&maintenance, StatusDetailSource::Inline);

    Ok(OfficialSnapshot {
        label: payload
            .page
            .name
            .or_else(|| Some(source.label().to_string()))
            .unwrap_or_else(|| source.label().to_string()),
        method: source.source_method(),
        health: ProviderHealth::from_api_status(&payload.status.description),
        official_url: payload
            .page
            .url
            .unwrap_or_else(|| source.page_url().to_string()),
        source_updated_at: payload.page.updated_at,
        provider_summary: incident_summary.or(Some(payload.status.description)),
        status_note: None,
        components_state,
        components,
        incidents_state,
        incidents,
        maintenance_state,
        maintenance,
    })
}

// ---------------------------------------------------------------------------
// Incidents JSON parser (for incident.io second call)
// ---------------------------------------------------------------------------

fn parse_incidents_json(body: &str) -> Result<Vec<ActiveIncident>, String> {
    let v: Value = serde_json::from_str(body).map_err(|err| err.to_string())?;
    let incidents = v
        .get("incidents")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|i| ActiveIncident {
                    name: i
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    status: i
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    impact: i
                        .get("impact")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    shortlink: i
                        .get("shortlink")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    created_at: i
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    updated_at: i
                        .get("updated_at")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    latest_update: i
                        .get("incident_updates")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .map(|u| IncidentUpdate {
                            status: u
                                .get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            body: u
                                .get("body")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            created_at: u
                                .get("created_at")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                        }),
                    affected_components: i
                        .get("components")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|c| {
                                    c.get("name").and_then(|v| v.as_str()).map(String::from)
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(incidents)
}

// ---------------------------------------------------------------------------
// Better Stack parser (JSON:API format)
// ---------------------------------------------------------------------------

fn parse_better_stack(
    source: OfficialStatusSource,
    body: &str,
) -> Result<OfficialSnapshot, String> {
    let v: Value = serde_json::from_str(body).map_err(|err| err.to_string())?;

    let aggregate_state = v
        .pointer("/data/attributes/aggregate_state")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let health = match aggregate_state {
        "operational" => ProviderHealth::Operational,
        "degraded" => ProviderHealth::Degraded,
        "downtime" => ProviderHealth::Outage,
        _ => ProviderHealth::Unknown,
    };

    let included = v.get("included").and_then(|v| v.as_array());

    let components: Vec<ComponentStatus> = included
        .map(|arr| {
            arr.iter()
                .filter(|item| {
                    item.get("type").and_then(|v| v.as_str()) == Some("status_page_resource")
                })
                .filter_map(|item| {
                    let name = item
                        .pointer("/attributes/public_name")
                        .or_else(|| item.pointer("/attributes/resource_name"))
                        .and_then(|v| v.as_str())?;
                    let status = item
                        .pointer("/attributes/status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("operational");
                    Some(ComponentStatus {
                        name: name.to_string(),
                        status: normalize_component_status(status),
                        group_name: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let incidents: Vec<ActiveIncident> = included
        .map(|arr| {
            arr.iter()
                .filter(|item| item.get("type").and_then(|v| v.as_str()) == Some("status_report"))
                .filter_map(|item| {
                    let title = item.pointer("/attributes/title").and_then(|v| v.as_str())?;
                    let message = item
                        .pointer("/attributes/status_report_updates")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|u| u.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    Some(ActiveIncident {
                        name: title.to_string(),
                        status: "investigating".to_string(),
                        impact: message.to_string(),
                        shortlink: None,
                        created_at: None,
                        updated_at: None,
                        latest_update: None,
                        affected_components: Vec::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let summary = incidents
        .first()
        .map(|i| i.name.clone())
        .or_else(|| Some(aggregate_state.to_string()));
    let components_state = available_detail_state(&components, StatusDetailSource::Inline);
    let incidents_state = available_detail_state(&incidents, StatusDetailSource::Inline);
    let maintenance_state = unsupported_detail_state(
        "Scheduled maintenance details are not exposed by this Better Stack adapter.",
    );

    Ok(OfficialSnapshot {
        label: source.label().to_string(),
        method: StatusSourceMethod::BetterStack,
        health,
        official_url: source.page_url().to_string(),
        source_updated_at: v
            .pointer("/data/attributes/updated_at")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        provider_summary: summary,
        status_note: None,
        components_state,
        components,
        incidents_state,
        incidents,
        maintenance_state,
        maintenance: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// OnlineOrNot parser
// ---------------------------------------------------------------------------

fn parse_onlineornot(source: OfficialStatusSource, body: &str) -> Result<OfficialSnapshot, String> {
    let v: Value = serde_json::from_str(body).map_err(|err| err.to_string())?;

    let result = v.get("result").ok_or("missing result field")?;

    // status may be a string ("operational") or an object ({"description": "All Systems Operational"})
    let status_str = result
        .get("status")
        .and_then(|v| {
            v.as_str().map(String::from).or_else(|| {
                v.get("description")
                    .and_then(|d| d.as_str())
                    .map(String::from)
            })
        })
        .unwrap_or_else(|| "unknown".to_string());

    let health = ProviderHealth::from_api_status(&status_str);

    let components: Vec<ComponentStatus> = result
        .get("components")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let name = c.get("name").and_then(|v| v.as_str())?;
                    let status = c
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("operational");
                    Some(ComponentStatus {
                        name: name.to_string(),
                        status: normalize_component_status(status),
                        group_name: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let incidents: Vec<ActiveIncident> = result
        .get("active_incidents")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|i| {
                    let name = i.get("name").and_then(|v| v.as_str())?;
                    Some(ActiveIncident {
                        name: name.to_string(),
                        status: i
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        impact: i
                            .get("impact")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        shortlink: None,
                        created_at: None,
                        updated_at: None,
                        latest_update: None,
                        affected_components: Vec::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let maintenance: Vec<ScheduledMaintenance> = result
        .get("scheduled_maintenance")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let name = m.get("name").and_then(|v| v.as_str())?;
                    Some(ScheduledMaintenance {
                        name: name.to_string(),
                        status: m
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        impact: String::new(),
                        scheduled_for: None,
                        scheduled_until: None,
                        affected_components: Vec::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let summary = incidents
        .first()
        .map(|i| i.name.clone())
        .or_else(|| Some(status_str.to_string()));
    let components_state = available_detail_state(&components, StatusDetailSource::Inline);
    let incidents_state = available_detail_state(&incidents, StatusDetailSource::Inline);
    let maintenance_state = available_detail_state(&maintenance, StatusDetailSource::Inline);

    Ok(OfficialSnapshot {
        label: source.label().to_string(),
        method: StatusSourceMethod::OnlineOrNot,
        health,
        official_url: source.page_url().to_string(),
        source_updated_at: None,
        provider_summary: summary,
        status_note: None,
        components_state,
        components,
        incidents_state,
        incidents,
        maintenance_state,
        maintenance,
    })
}

// ---------------------------------------------------------------------------
// Status.io parser
// ---------------------------------------------------------------------------

fn status_io_code_to_string(code: u64) -> String {
    match code {
        100 => "operational".to_string(),
        200 => "under_maintenance".to_string(),
        300 | 400 => "degraded_performance".to_string(),
        500 => "major_outage".to_string(),
        600 => "security_event".to_string(),
        _ => format!("unknown_{code}"),
    }
}

fn status_io_code_to_health(code: u64) -> ProviderHealth {
    match code {
        100 => ProviderHealth::Operational,
        200 => ProviderHealth::Maintenance,
        300 | 400 => ProviderHealth::Degraded,
        500 => ProviderHealth::Outage,
        600 => ProviderHealth::Unknown,
        _ => ProviderHealth::Unknown,
    }
}

fn status_io_code_or_label_to_status(value: &str) -> String {
    if let Ok(code) = value.parse::<u64>() {
        return status_io_code_to_string(code);
    }

    let normalized = value.trim().to_lowercase();
    if normalized.contains("maintenance") {
        "under_maintenance".to_string()
    } else if normalized.contains("security") {
        "security_event".to_string()
    } else if normalized.contains("major") || normalized.contains("disruption") {
        "major_outage".to_string()
    } else if normalized.contains("partial")
        || normalized.contains("minor")
        || normalized.contains("degrad")
    {
        "degraded_performance".to_string()
    } else if normalized.contains("operational") {
        "operational".to_string()
    } else {
        normalize_component_status(value)
    }
}

fn status_io_datetime(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

fn status_io_latest_message(messages: &[Value]) -> Option<&Value> {
    messages.iter().max_by_key(|message| {
        message
            .get("datetime")
            .and_then(|value| value.as_str())
            .and_then(crate::agents::helpers::parse_date)
            .map(|dt| dt.timestamp())
            .unwrap_or(0)
    })
}

fn status_io_collect_names(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("name")
                        .and_then(|value| value.as_str())
                        .or_else(|| item.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn status_io_merge_names(primary: Vec<String>, secondary: Vec<String>) -> Vec<String> {
    let mut merged = primary;
    for name in secondary {
        if !merged.contains(&name) {
            merged.push(name);
        }
    }
    merged
}

fn status_io_message_state(message: &Value) -> Option<String> {
    message
        .get("state")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            message
                .get("status")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
}

fn parse_status_io(source: OfficialStatusSource, body: &str) -> Result<OfficialSnapshot, String> {
    let v: Value = serde_json::from_str(body).map_err(|err| err.to_string())?;

    let result = v.get("result").ok_or("missing result field")?;

    let overall_code = result
        .pointer("/status_overall/status_code")
        .and_then(|v| v.as_u64())
        .unwrap_or(100);

    let health = status_io_code_to_health(overall_code);

    let components: Vec<ComponentStatus> = result
        .get("status")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .flat_map(|status_group| {
                    let group_name = status_group
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    status_group
                        .get("containers")
                        .and_then(|v| v.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(move |c| {
                            let name = c.get("name").and_then(|v| v.as_str())?;
                            let code = c.get("status_code").and_then(|v| v.as_u64()).unwrap_or(100);
                            Some(ComponentStatus {
                                name: name.to_string(),
                                status: status_io_code_to_string(code),
                                group_name: group_name.clone(),
                            })
                        })
                })
                .collect()
        })
        .unwrap_or_default();

    let incidents: Vec<ActiveIncident> = result
        .get("incidents")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|i| {
                    let name = i.get("name").and_then(|v| v.as_str())?;
                    let messages = i
                        .get("messages")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let latest_message = status_io_latest_message(&messages);
                    let affected_components = status_io_merge_names(
                        status_io_collect_names(i.get("components_affected")),
                        status_io_collect_names(i.get("containers_affected")),
                    );
                    Some(ActiveIncident {
                        name: name.to_string(),
                        status: latest_message
                            .and_then(status_io_message_state)
                            .unwrap_or_else(|| "reported".to_string()),
                        impact: latest_message
                            .and_then(|message| {
                                message
                                    .get("status")
                                    .and_then(|value| value.as_str())
                                    .map(status_io_code_or_label_to_status)
                            })
                            .unwrap_or_default(),
                        shortlink: None,
                        created_at: i.get("datetime_open").and_then(status_io_datetime),
                        updated_at: latest_message
                            .and_then(|message| message.get("datetime"))
                            .and_then(status_io_datetime),
                        latest_update: latest_message.map(|message| IncidentUpdate {
                            status: status_io_message_state(message)
                                .unwrap_or_else(|| "reported".to_string()),
                            body: message
                                .get("details")
                                .and_then(|value| value.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            created_at: message
                                .get("datetime")
                                .and_then(status_io_datetime)
                                .unwrap_or_default(),
                        }),
                        affected_components,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut maintenance: Vec<ScheduledMaintenance> = Vec::new();
    if let Some(maint) = result.get("maintenance") {
        for key in &["active", "upcoming"] {
            if let Some(arr) = maint.get(*key).and_then(|v| v.as_array()) {
                for m in arr {
                    if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                        let messages = m
                            .get("messages")
                            .and_then(|value| value.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let latest_message = status_io_latest_message(&messages);
                        let affected_components = status_io_merge_names(
                            status_io_collect_names(m.get("components_affected")),
                            status_io_collect_names(m.get("containers_affected")),
                        );
                        maintenance.push(ScheduledMaintenance {
                            name: name.to_string(),
                            status: latest_message
                                .and_then(status_io_message_state)
                                .unwrap_or_else(|| (*key).to_string()),
                            impact: latest_message
                                .and_then(|message| {
                                    message
                                        .get("status")
                                        .and_then(|value| value.as_str())
                                        .map(status_io_code_or_label_to_status)
                                })
                                .unwrap_or_default(),
                            scheduled_for: m
                                .get("datetime_planned_start")
                                .and_then(status_io_datetime)
                                .or_else(|| m.get("datetime_open").and_then(status_io_datetime)),
                            scheduled_until: m
                                .get("datetime_planned_end")
                                .and_then(status_io_datetime),
                            affected_components,
                        });
                    }
                }
            }
        }
    }

    let summary = incidents
        .first()
        .map(|i| i.name.clone())
        .or_else(|| {
            result
                .pointer("/status_overall/status")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .or_else(|| Some(status_io_code_to_string(overall_code)));
    let components_state = available_detail_state(&components, StatusDetailSource::Inline);
    let incidents_state = available_detail_state(&incidents, StatusDetailSource::Inline);
    let maintenance_state = available_detail_state(&maintenance, StatusDetailSource::Inline);

    Ok(OfficialSnapshot {
        label: source.label().to_string(),
        method: StatusSourceMethod::StatusIo,
        health,
        official_url: source.page_url().to_string(),
        source_updated_at: result
            .pointer("/status_overall/updated")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        provider_summary: summary,
        status_note: None,
        components_state,
        components,
        incidents_state,
        incidents,
        maintenance_state,
        maintenance,
    })
}

// ---------------------------------------------------------------------------
// Instatus parser
// ---------------------------------------------------------------------------

fn instatus_status_to_health(status: &str) -> ProviderHealth {
    match status {
        "UP" | "OPERATIONAL" => ProviderHealth::Operational,
        "HASISSUES" | "DEGRADEDPERFORMANCE" => ProviderHealth::Degraded,
        "MAJOROUTAGE" | "PARTIALOUTAGE" => ProviderHealth::Outage,
        "UNDERMAINTENANCE" => ProviderHealth::Maintenance,
        _ => ProviderHealth::Unknown,
    }
}

fn parse_instatus_summary(
    source: OfficialStatusSource,
    body: &str,
) -> Result<OfficialSnapshot, String> {
    let v: Value = serde_json::from_str(body).map_err(|err| err.to_string())?;

    let page_status = v
        .pointer("/page/status")
        .and_then(|v| v.as_str())
        .unwrap_or("OPERATIONAL");

    let health = instatus_status_to_health(page_status);

    let incidents: Vec<ActiveIncident> = v
        .get("activeIncidents")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|i| {
                    let name = i.get("name").and_then(|v| v.as_str())?;
                    Some(ActiveIncident {
                        name: name.to_string(),
                        status: i
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        impact: i
                            .get("impact")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        shortlink: None,
                        created_at: None,
                        updated_at: None,
                        latest_update: None,
                        affected_components: Vec::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let maintenance: Vec<ScheduledMaintenance> = v
        .get("scheduledMaintenances")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let name = m.get("name").and_then(|v| v.as_str())?;
                    Some(ScheduledMaintenance {
                        name: name.to_string(),
                        status: m
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        impact: String::new(),
                        scheduled_for: None,
                        scheduled_until: None,
                        affected_components: Vec::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let summary = incidents
        .first()
        .map(|i| i.name.clone())
        .or_else(|| Some(normalize_component_status(page_status)));
    let components_state = not_attempted_detail_state(
        StatusDetailSource::Enrichment,
        "Service details require the Instatus components endpoint.",
    );
    let incidents_state = available_detail_state(&incidents, StatusDetailSource::Inline);
    let maintenance_state = available_detail_state(&maintenance, StatusDetailSource::Inline);

    Ok(OfficialSnapshot {
        label: source.label().to_string(),
        method: StatusSourceMethod::Instatus,
        health,
        official_url: source.page_url().to_string(),
        source_updated_at: None,
        provider_summary: summary,
        status_note: None,
        components_state,
        components: Vec::new(),
        incidents_state,
        incidents,
        maintenance_state,
        maintenance,
    })
}

fn parse_instatus_components(body: &str) -> Result<Vec<ComponentStatus>, String> {
    let arr: Vec<Value> = serde_json::from_str(body).map_err(|err| err.to_string())?;
    Ok(arr
        .iter()
        .filter_map(|c| {
            let name = c.get("name").and_then(|v| v.as_str())?;
            let status = c
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("OPERATIONAL");
            Some(ComponentStatus {
                name: name.to_string(),
                status: normalize_component_status(status),
                group_name: None,
            })
        })
        .collect())
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

// ---------------------------------------------------------------------------
// Deserialization types
// ---------------------------------------------------------------------------

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
    #[serde(default)]
    components: Vec<OfficialComponent>,
    #[serde(default)]
    scheduled_maintenances: Vec<OfficialScheduledMaintenance>,
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
    #[serde(default)]
    shortlink: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    incident_updates: Vec<OfficialIncidentUpdate>,
    #[serde(default)]
    components: Vec<OfficialComponentRef>,
}

#[derive(Debug, Deserialize)]
struct OfficialIncidentUpdate {
    status: String,
    body: String,
    #[serde(default)]
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OfficialComponentRef {
    name: String,
}

#[derive(Debug, Deserialize)]
struct OfficialComponent {
    name: String,
    status: String,
    #[serde(default)]
    group_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OfficialScheduledMaintenance {
    name: String,
    status: String,
    #[serde(default)]
    impact: Option<String>,
    #[serde(default)]
    scheduled_for: Option<String>,
    #[serde(default)]
    scheduled_until: Option<String>,
    #[serde(default)]
    components: Vec<OfficialComponentRef>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleProductsResponse {
    products: Vec<GoogleProduct>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleProduct {
    id: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct GoogleIncident {
    external_desc: String,
    #[serde(default)]
    modified: Option<String>,
    #[serde(default)]
    end: Option<String>,
    severity: String,
    status_impact: String,
    #[serde(default)]
    affected_products: Vec<GoogleAffectedProduct>,
}

#[derive(Debug, Deserialize)]
struct GoogleAffectedProduct {
    id: String,
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

    #[test]
    fn builds_google_snapshot_from_incidents() {
        let product = GoogleProduct {
            id: "Z0FZJAMvEB4j3NbCJs6B".to_string(),
            title: "Vertex Gemini API".to_string(),
        };
        let incidents = vec![GoogleIncident {
            external_desc: "Vertex AI Gemini API customers experienced increased error rates"
                .to_string(),
            modified: Some("2026-03-09T05:25:43+00:00".to_string()),
            end: Some("2026-02-27T14:35:00+00:00".to_string()),
            severity: "low".to_string(),
            status_impact: "SERVICE_INFORMATION".to_string(),
            affected_products: vec![GoogleAffectedProduct {
                id: "Z0FZJAMvEB4j3NbCJs6B".to_string(),
            }],
        }];

        let snapshot = OfficialSnapshot::from_google(&product, &incidents);
        assert_eq!(snapshot.method, StatusSourceMethod::GoogleCloudJson);
        assert_eq!(snapshot.health, ProviderHealth::Operational);
        assert_eq!(
            snapshot.incidents_state.availability,
            StatusDetailAvailability::NotAttempted
        );
        assert!(snapshot
            .official_url
            .contains("Z0FZJAMvEB4j3NbCJs6B/history"));
    }

    #[test]
    fn parses_better_stack_index_json() {
        let json = r#"{
            "data": {
                "id": "abc123",
                "type": "status_page",
                "attributes": {
                    "aggregate_state": "degraded"
                }
            },
            "included": [
                {
                    "id": "r1",
                    "type": "status_page_resource",
                    "attributes": {
                        "public_name": "API",
                        "status": "degraded"
                    }
                },
                {
                    "id": "r2",
                    "type": "status_page_resource",
                    "attributes": {
                        "resource_name": "Dashboard",
                        "status": "operational"
                    }
                },
                {
                    "id": "sr1",
                    "type": "status_report",
                    "attributes": {
                        "title": "API latency increase",
                        "status_report_updates": [
                            {"message": "Investigating elevated latency"}
                        ]
                    }
                }
            ]
        }"#;

        let snapshot =
            parse_better_stack(OfficialStatusSource::TogetherAi, json).expect("parses ok");
        assert_eq!(snapshot.method, StatusSourceMethod::BetterStack);
        assert_eq!(snapshot.health, ProviderHealth::Degraded);
        assert_eq!(snapshot.components.len(), 2);
        assert_eq!(snapshot.components[0].name, "API");
        assert_eq!(snapshot.components[0].status, "degraded_performance");
        assert_eq!(snapshot.components[1].status, "operational");
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(snapshot.incidents[0].name, "API latency increase");
    }

    #[test]
    fn parses_incident_io_summary_with_incidents() {
        let summary_json = r#"{
            "page": {"name": "OpenAI", "url": "https://status.openai.com", "updated_at": "2026-03-12T00:00:00Z"},
            "status": {"description": "All Systems Operational"},
            "incidents": [],
            "components": [
                {"name": "API", "status": "operational"},
                {"name": "Dashboard", "status": "operational"}
            ],
            "scheduled_maintenances": []
        }"#;

        let snapshot = parse_statuspage_v2_summary(OfficialStatusSource::OpenAi, summary_json)
            .expect("parses summary");
        assert_eq!(snapshot.health, ProviderHealth::Operational);
        assert_eq!(snapshot.components.len(), 2);

        let incidents_json = r#"{
            "incidents": [
                {"name": "Elevated error rates", "status": "investigating", "impact": "minor"}
            ]
        }"#;

        let incidents = parse_incidents_json(incidents_json).expect("parses incidents");
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].name, "Elevated error rates");
        assert_eq!(incidents[0].status, "investigating");
        assert_eq!(incidents[0].impact, "minor");
    }

    #[test]
    fn parses_onlineornot_summary() {
        let json = r#"{
            "result": {
                "status": "degraded",
                "components": [
                    {"name": "API Gateway", "status": "operational"},
                    {"name": "Inference", "status": "degraded"}
                ],
                "active_incidents": [
                    {"name": "Slow responses", "status": "investigating", "impact": "minor"}
                ],
                "scheduled_maintenance": []
            }
        }"#;

        let snapshot =
            parse_onlineornot(OfficialStatusSource::OpenRouter, json).expect("parses ok");
        assert_eq!(snapshot.method, StatusSourceMethod::OnlineOrNot);
        assert_eq!(snapshot.health, ProviderHealth::Degraded);
        assert_eq!(snapshot.components.len(), 2);
        assert_eq!(snapshot.components[0].name, "API Gateway");
        assert_eq!(snapshot.components[0].status, "operational");
        assert_eq!(snapshot.components[1].status, "degraded_performance");
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(snapshot.incidents[0].name, "Slow responses");
    }

    #[test]
    fn parses_status_io_response() {
        let json = r#"{
            "result": {
                "status_overall": {
                    "status_code": 300,
                    "status": "Minor Service Outage",
                    "updated": "2026-03-17T00:05:00Z"
                },
                "status": [
                    {
                        "id": "g1",
                        "name": "Infrastructure",
                        "containers": [
                            {"name": "Web", "status_code": 100},
                            {"name": "API", "status_code": 300}
                        ]
                    }
                ],
                "incidents": [
                    {
                        "name": "API degradation",
                        "datetime_open": "2026-03-16T23:00:00Z",
                        "components_affected": [{"name": "API"}],
                        "containers_affected": [{"name": "Web"}],
                        "messages": [
                            {
                                "state": "identified",
                                "status": "Minor Service Outage",
                                "details": "API latency is elevated in one region.",
                                "datetime": "2026-03-17T00:04:00Z"
                            }
                        ]
                    }
                ],
                "maintenance": {
                    "active": [{
                        "name": "DB migration",
                        "datetime_open": "2026-03-16T22:00:00Z",
                        "datetime_planned_start": "2026-03-16T22:00:00Z",
                        "datetime_planned_end": "2026-03-17T01:00:00Z",
                        "components_affected": [{"name": "API"}],
                        "messages": [{
                            "state": "active",
                            "status": "Maintenance",
                            "details": "Database migration in progress.",
                            "datetime": "2026-03-16T22:10:00Z"
                        }]
                    }],
                    "upcoming": [{"name": "Network upgrade"}]
                }
            }
        }"#;

        let snapshot = parse_status_io(OfficialStatusSource::GitLab, json).expect("parses ok");
        assert_eq!(snapshot.method, StatusSourceMethod::StatusIo);
        assert_eq!(snapshot.health, ProviderHealth::Degraded);
        assert_eq!(snapshot.components.len(), 2);
        assert_eq!(snapshot.components[0].name, "Web");
        assert_eq!(snapshot.components[0].status, "operational");
        assert_eq!(snapshot.components[1].name, "API");
        assert_eq!(snapshot.components[1].status, "degraded_performance");
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(snapshot.incidents[0].name, "API degradation");
        assert_eq!(snapshot.incidents[0].status, "identified");
        assert_eq!(snapshot.incidents[0].impact, "degraded_performance");
        assert_eq!(
            snapshot.incidents[0].affected_components,
            vec!["API".to_string(), "Web".to_string()]
        );
        assert_eq!(
            snapshot.incidents[0].updated_at.as_deref(),
            Some("2026-03-17T00:04:00Z")
        );
        assert_eq!(
            snapshot.incidents[0]
                .latest_update
                .as_ref()
                .map(|update| update.body.as_str()),
            Some("API latency is elevated in one region.")
        );
        assert_eq!(snapshot.maintenance.len(), 2);
        assert_eq!(snapshot.maintenance[0].name, "DB migration");
        assert_eq!(snapshot.maintenance[0].status, "active");
        assert_eq!(snapshot.maintenance[0].impact, "under_maintenance");
        assert_eq!(
            snapshot.maintenance[0].scheduled_until.as_deref(),
            Some("2026-03-17T01:00:00Z")
        );
        assert_eq!(
            snapshot.provider_summary.as_deref(),
            Some("API degradation")
        );
        assert_eq!(snapshot.maintenance[1].name, "Network upgrade");
        assert_eq!(snapshot.maintenance[1].status, "upcoming");
    }

    #[test]
    fn status_io_code_mappings_cover_maintenance_and_security() {
        assert_eq!(status_io_code_to_string(200), "under_maintenance");
        assert_eq!(status_io_code_to_health(200), ProviderHealth::Maintenance);
        assert_eq!(status_io_code_to_string(600), "security_event");
        assert_eq!(status_io_code_to_health(600), ProviderHealth::Unknown);
    }

    #[test]
    fn parses_instatus_summary() {
        let summary_json = r#"{
            "page": {"status": "HASISSUES"},
            "activeIncidents": [
                {"name": "Search degraded", "status": "INVESTIGATING", "impact": "minor"}
            ],
            "scheduledMaintenances": [
                {"name": "Planned reboot", "status": "SCHEDULED"}
            ]
        }"#;

        let snapshot = parse_instatus_summary(OfficialStatusSource::Perplexity, summary_json)
            .expect("parses ok");
        assert_eq!(snapshot.method, StatusSourceMethod::Instatus);
        assert_eq!(snapshot.health, ProviderHealth::Degraded);
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(snapshot.incidents[0].name, "Search degraded");
        assert_eq!(snapshot.maintenance.len(), 1);
        assert_eq!(snapshot.maintenance[0].name, "Planned reboot");

        let components_json = r#"[
            {"name": "API", "status": "OPERATIONAL"},
            {"name": "Search", "status": "DEGRADEDPERFORMANCE"},
            {"name": "Backend", "status": "MAJOROUTAGE"}
        ]"#;

        let components = parse_instatus_components(components_json).expect("parses ok");
        assert_eq!(components.len(), 3);
        assert_eq!(components[0].status, "operational");
        assert_eq!(components[1].status, "degraded_performance");
        assert_eq!(components[2].status, "major_outage");
    }

    #[test]
    fn normalize_component_status_maps_all_platforms() {
        // Better Stack
        assert_eq!(
            normalize_component_status("degraded"),
            "degraded_performance"
        );
        assert_eq!(normalize_component_status("downtime"), "major_outage");
        // OnlineOrNot
        assert_eq!(normalize_component_status("outage"), "major_outage");
        // Instatus
        assert_eq!(normalize_component_status("OPERATIONAL"), "operational");
        assert_eq!(
            normalize_component_status("DEGRADEDPERFORMANCE"),
            "degraded_performance"
        );
        assert_eq!(
            normalize_component_status("UNDERMAINTENANCE"),
            "under_maintenance"
        );
        assert_eq!(normalize_component_status("MAJOROUTAGE"), "major_outage");
        assert_eq!(
            normalize_component_status("PARTIALOUTAGE"),
            "partial_outage"
        );
        // Already normalized
        assert_eq!(normalize_component_status("operational"), "operational");
        assert_eq!(normalize_component_status("major_outage"), "major_outage");
    }
}
