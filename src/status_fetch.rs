use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;
use tokio::time::timeout;

use crate::status::{
    ActiveIncident, ComponentStatus, IncidentUpdate, OfficialStatusSource, ProviderHealth,
    ProviderStatus, ScheduledMaintenance, StatusProvenance, StatusProviderSeed, StatusSourceMethod,
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
            let incidents_url = format!("{}/api/v2/incidents.json", source.page_url());
            if let Ok(Ok(incidents)) = timeout(Duration::from_secs(3), async {
                let resp = client
                    .get(&incidents_url)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                let text = resp.text().await.map_err(|e| e.to_string())?;
                parse_incidents_json(&text)
            })
            .await
            {
                snapshot.incidents = incidents;
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
            if let Ok(Ok(components)) = timeout(Duration::from_secs(3), async {
                let resp = client
                    .get(&components_url)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                let text = resp.text().await.map_err(|e| e.to_string())?;
                parse_instatus_components(&text)
            })
            .await
            {
                snapshot.components = components;
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

            // Populate per-provider error when both sources failed
            if status.provenance == StatusProvenance::Unavailable {
                if let Some(ref off_err) = official_err {
                    status.error = Some(match &fallback_err {
                        Some(fb_err) => format!("official: {off_err}; fallback: {fb_err}"),
                        None => format!("official: {off_err}"),
                    });
                }
            }

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
    last_checked: Option<String>,
    summary: Option<String>,
    components: Vec<ComponentStatus>,
    incidents: Vec<ActiveIncident>,
    maintenance: Vec<ScheduledMaintenance>,
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

        Self {
            label: product.title.clone(),
            method: StatusSourceMethod::GoogleCloudJson,
            health,
            official_url: format!(
                "https://status.cloud.google.com/products/{}/history",
                product.id
            ),
            last_checked,
            summary,
            components: Vec::new(),
            incidents: Vec::new(),
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

    let components = payload
        .components
        .iter()
        .map(|c| ComponentStatus {
            name: c.name.clone(),
            status: c.status.clone(),
            group_name: c.group_name.clone(),
        })
        .collect();

    let incidents = payload
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

    let maintenance = payload
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
        last_checked: payload.page.updated_at,
        summary: incident_summary.or(Some(payload.status.description)),
        components,
        incidents,
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

    let components = included
        .map(|arr| {
            arr.iter()
                .filter(|item| {
                    item.get("type").and_then(|v| v.as_str()) == Some("status_page_resource")
                })
                .filter_map(|item| {
                    let name = item
                        .pointer("/attributes/resource_name")
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

    Ok(OfficialSnapshot {
        label: source.label().to_string(),
        method: StatusSourceMethod::BetterStack,
        health,
        official_url: source.page_url().to_string(),
        last_checked: None,
        summary,
        components,
        incidents,
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

    let components = result
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

    Ok(OfficialSnapshot {
        label: source.label().to_string(),
        method: StatusSourceMethod::OnlineOrNot,
        health,
        official_url: source.page_url().to_string(),
        last_checked: None,
        summary,
        components,
        incidents,
        maintenance,
    })
}

// ---------------------------------------------------------------------------
// Status.io parser
// ---------------------------------------------------------------------------

fn status_io_code_to_string(code: u64) -> String {
    match code {
        100 => "operational".to_string(),
        300 => "degraded_performance".to_string(),
        500 => "major_outage".to_string(),
        600 => "under_maintenance".to_string(),
        _ => format!("unknown_{code}"),
    }
}

fn status_io_code_to_health(code: u64) -> ProviderHealth {
    match code {
        100 => ProviderHealth::Operational,
        300 => ProviderHealth::Degraded,
        500 => ProviderHealth::Outage,
        600 => ProviderHealth::Maintenance,
        _ => ProviderHealth::Unknown,
    }
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
                    Some(ActiveIncident {
                        name: name.to_string(),
                        status: "investigating".to_string(),
                        impact: "none".to_string(),
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

    let mut maintenance: Vec<ScheduledMaintenance> = Vec::new();
    if let Some(maint) = result.get("maintenance") {
        for key in &["active", "upcoming"] {
            if let Some(arr) = maint.get(*key).and_then(|v| v.as_array()) {
                for m in arr {
                    if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                        maintenance.push(ScheduledMaintenance {
                            name: name.to_string(),
                            status: (*key).to_string(),
                            impact: String::new(),
                            scheduled_for: None,
                            scheduled_until: None,
                            affected_components: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    let summary = incidents
        .first()
        .map(|i| i.name.clone())
        .or_else(|| Some(status_io_code_to_string(overall_code)));

    Ok(OfficialSnapshot {
        label: source.label().to_string(),
        method: StatusSourceMethod::StatusIo,
        health,
        official_url: source.page_url().to_string(),
        last_checked: None,
        summary,
        components,
        incidents,
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

    Ok(OfficialSnapshot {
        label: source.label().to_string(),
        method: StatusSourceMethod::Instatus,
        health,
        official_url: source.page_url().to_string(),
        last_checked: None,
        summary,
        components: Vec::new(),
        incidents,
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
        status.source_label = Some(official.label);
        status.source_method = Some(official.method);
        status.official_url = Some(official.official_url);
        status.last_checked = official.last_checked;
        status.summary = official.summary;
        status.components = official.components;
        status.incidents = official.incidents;
        status.scheduled_maintenances = official.maintenance;

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
        status.source_label = Some(fallback.label);
        status.source_method = Some(StatusSourceMethod::ApiStatusCheck);
        status.official_url = fallback.official_url;
        status.fallback_url = Some(fallback.fallback_url);
        status.last_checked = fallback.last_checked;
        status.summary = fallback.summary;
        return status;
    }

    status.summary = match seed.strategy {
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
    use crate::status::{status_seed_for_provider, StatusProvenance};

    use super::*;

    fn seed(slug: &str) -> StatusProviderSeed {
        status_seed_for_provider(slug)
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
                last_checked: Some("2026-03-11T00:00:00Z".to_string()),
                summary: Some("Partial System Degradation".to_string()),
                components: Vec::new(),
                incidents: Vec::new(),
                maintenance: Vec::new(),
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
    fn unverified_provider_stays_unavailable() {
        let status = resolve_provider_status(&seed("some-nonexistent-provider"), None, None);
        assert_eq!(status.provenance, StatusProvenance::Unavailable);
        assert!(status
            .summary
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
                        "resource_name": "API",
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
                "status_overall": {"status_code": 300, "status": "Minor Service Outage"},
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
                    {"name": "API degradation"}
                ],
                "maintenance": {
                    "active": [{"name": "DB migration"}],
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
        assert_eq!(snapshot.maintenance.len(), 2);
        assert_eq!(snapshot.maintenance[0].name, "DB migration");
        assert_eq!(snapshot.maintenance[0].status, "active");
        assert_eq!(snapshot.maintenance[1].name, "Network upgrade");
        assert_eq!(snapshot.maintenance[1].status, "upcoming");
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
