use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

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

        let google_products = if seeds.iter().any(|seed| {
            matches!(
                seed.strategy,
                StatusStrategy::OfficialFirst {
                    official: OfficialStatusSource::GoogleGeminiJson,
                    ..
                }
            )
        }) {
            match self.fetch_google_products().await {
                Ok(products) => Some(products),
                Err(err) => {
                    last_error = Some(err);
                    None
                }
            }
        } else {
            None
        };

        for seed in seeds {
            let resolved = match seed.strategy {
                StatusStrategy::OfficialFirst {
                    official,
                    fallback_source_slug,
                } => {
                    let official_result = self
                        .fetch_official(official, google_products.as_ref())
                        .await;
                    if official_result.is_ok() {
                        successes += 1;
                    } else if let Err(err) = &official_result {
                        last_error = Some(err.clone());
                    }

                    let fallback_result = match (official_result.as_ref(), fallback_source_slug) {
                        (Ok(_), _) | (_, None) => None,
                        (Err(_), Some(source_slug)) => match self.fetch_fallback(source_slug).await
                        {
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
                StatusStrategy::Unverified => resolve_provider_status(seed, None, None),
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
        google_products: Option<&GoogleProductsResponse>,
    ) -> Result<OfficialSnapshot, String> {
        match source.source_method() {
            StatusSourceMethod::StatuspageV2 => {
                let body = self.fetch_text(source.endpoint_url()).await?;
                parse_statuspage_v2_summary(source, &body)
            }
            StatusSourceMethod::IncidentIoShim => {
                let body = self.fetch_text(source.endpoint_url()).await?;
                let mut snapshot = parse_statuspage_v2_summary(source, &body)?;
                let incidents_url = format!("{}/api/v2/incidents.json", source.page_url());
                if let Ok(resp) = self.client.get(&incidents_url).send().await {
                    if let Ok(text) = resp.text().await {
                        if let Ok(incidents) = parse_incidents_json(&text) {
                            snapshot.incidents = incidents;
                        }
                    }
                }
                Ok(snapshot)
            }
            StatusSourceMethod::BetterStack => {
                let body = self.fetch_text(source.endpoint_url()).await?;
                parse_better_stack(source, &body)
            }
            StatusSourceMethod::OnlineOrNot => {
                let body = self.fetch_text(source.endpoint_url()).await?;
                parse_onlineornot(source, &body)
            }
            StatusSourceMethod::StatusIo => {
                let body = self.fetch_text(source.endpoint_url()).await?;
                parse_status_io(source, &body)
            }
            StatusSourceMethod::Instatus => {
                let body = self.fetch_text(source.endpoint_url()).await?;
                let mut snapshot = parse_instatus_summary(source, &body)?;
                let components_url = format!("{}/v2/components.json", source.page_url());
                if let Ok(resp) = self.client.get(&components_url).send().await {
                    if let Ok(text) = resp.text().await {
                        if let Ok(components) = parse_instatus_components(&text) {
                            snapshot.components = components;
                        }
                    }
                }
                Ok(snapshot)
            }
            StatusSourceMethod::Feed => {
                let body = self.fetch_text(source.endpoint_url()).await?;
                parse_feed(source, &body)
            }
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

                let body = self.fetch_text(source.endpoint_url()).await?;
                let incidents: Vec<GoogleIncident> =
                    serde_json::from_str(&body).map_err(|err| err.to_string())?;
                Ok(OfficialSnapshot::from_google(product, &incidents))
            }
            StatusSourceMethod::ApiStatusCheck => {
                Err("ApiStatusCheck is not an official source method".to_string())
            }
        }
    }

    async fn fetch_text(&self, url: &str) -> Result<String, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| err.to_string())?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        response.text().await.map_err(|err| err.to_string())
    }

    async fn fetch_google_products(&self) -> Result<GoogleProductsResponse, String> {
        let body = self.fetch_text(GOOGLE_PRODUCTS_URL).await?;
        serde_json::from_str(&body).map_err(|err| err.to_string())
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

    let status_str = result
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let health = match status_str {
        "operational" | "up" => ProviderHealth::Operational,
        "degraded" => ProviderHealth::Degraded,
        "outage" | "down" => ProviderHealth::Outage,
        _ => ProviderHealth::Unknown,
    };

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

// ---------------------------------------------------------------------------
// RSS / Atom feed parsers
// ---------------------------------------------------------------------------

fn parse_feed(source: OfficialStatusSource, xml: &str) -> Result<OfficialSnapshot, String> {
    parse_atom_feed(source, xml).or_else(|_| parse_rss_feed(source, xml))
}

fn parse_rss_feed(source: OfficialStatusSource, xml: &str) -> Result<OfficialSnapshot, String> {
    let title_re =
        Regex::new(r"(?s)<channel>.*?<title>(.*?)</title>").map_err(|err| err.to_string())?;
    let build_re =
        Regex::new(r"(?s)<lastBuildDate>(.*?)</lastBuildDate>").map_err(|err| err.to_string())?;
    let item_re = Regex::new(r"(?s)<item>(.*?)</item>").map_err(|err| err.to_string())?;
    let item_title_re = Regex::new(r"(?s)<title>(.*?)</title>").map_err(|err| err.to_string())?;
    let item_description_re =
        Regex::new(r"(?s)<description>(.*?)</description>").map_err(|err| err.to_string())?;
    let channel_title = title_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string())
        .unwrap_or_else(|| source.label().to_string());
    let last_checked = build_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string());
    let item = item_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str());

    let official_url = source.page_url().to_string();

    let (health, summary) = if let Some(item_block) = item {
        let title = item_title_re
            .captures(item_block)
            .and_then(|caps| caps.get(1))
            .map(|m| strip_markup(&decode_xml(m.as_str())))
            .unwrap_or_default();
        let description = item_description_re
            .captures(item_block)
            .and_then(|caps| caps.get(1))
            .map(|m| strip_markup(&decode_xml(m.as_str())))
            .unwrap_or_default();
        let combined = format!("{title} {description}");
        (
            health_from_feed_text(&combined),
            Some(prefer_summary(&title, &description)),
        )
    } else {
        (
            ProviderHealth::Operational,
            Some("No incidents recorded".to_string()),
        )
    };

    Ok(OfficialSnapshot {
        label: channel_title,
        method: StatusSourceMethod::Feed,
        health,
        official_url,
        last_checked,
        summary,
        components: Vec::new(),
        incidents: Vec::new(),
        maintenance: Vec::new(),
    })
}

fn parse_atom_feed(source: OfficialStatusSource, xml: &str) -> Result<OfficialSnapshot, String> {
    if !xml.contains("<feed") {
        return Err("not an atom feed".to_string());
    }

    let title_re =
        Regex::new(r"(?s)<feed[^>]*>.*?<title>(.*?)</title>").map_err(|err| err.to_string())?;
    let updated_re = Regex::new(r"(?s)<updated>(.*?)</updated>").map_err(|err| err.to_string())?;
    let entry_re = Regex::new(r"(?s)<entry>(.*?)</entry>").map_err(|err| err.to_string())?;
    let entry_title_re = Regex::new(r"(?s)<title>(.*?)</title>").map_err(|err| err.to_string())?;
    let entry_body_re = Regex::new(r"(?s)<(?:content|summary)[^>]*>(.*?)</(?:content|summary)>")
        .map_err(|err| err.to_string())?;
    let entry_link_re =
        Regex::new(r#"(?s)<link[^>]*href="(.*?)"[^>]*/?>"#).map_err(|err| err.to_string())?;

    let feed_title = title_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string())
        .unwrap_or_else(|| source.label().to_string());
    let last_checked = updated_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string());
    let entry_block = entry_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| "atom feed entry missing".to_string())?;

    let title = entry_title_re
        .captures(entry_block)
        .and_then(|caps| caps.get(1))
        .map(|m| strip_markup(&decode_xml(m.as_str())))
        .unwrap_or_default();
    let body = entry_body_re
        .captures(entry_block)
        .and_then(|caps| caps.get(1))
        .map(|m| strip_markup(&decode_xml(m.as_str())))
        .unwrap_or_default();
    let official_url = entry_link_re
        .captures(entry_block)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string())
        .unwrap_or_else(|| source.page_url().to_string());
    let combined = format!("{title} {body}");

    Ok(OfficialSnapshot {
        label: feed_title,
        method: StatusSourceMethod::Feed,
        health: health_from_feed_text(&combined),
        official_url,
        last_checked,
        summary: Some(prefer_summary(&title, &body)),
        components: Vec::new(),
        incidents: Vec::new(),
        maintenance: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn health_from_feed_text(text: &str) -> ProviderHealth {
    let upper = text.to_uppercase();
    if upper.contains("RECOVERED")
        || upper.contains("RESOLVED")
        || upper.contains("OPERATIONAL AGAIN")
        || upper.contains("ALL SYSTEMS OPERATIONAL")
    {
        ProviderHealth::Operational
    } else if upper.contains("OUTAGE") || upper.contains("WENT DOWN") || upper.contains("DOWN") {
        ProviderHealth::Outage
    } else if upper.contains("INVESTIGATING")
        || upper.contains("IDENTIFIED")
        || upper.contains("DEGRADED")
        || upper.contains("PARTIAL")
        || upper.contains("MINOR")
    {
        ProviderHealth::Degraded
    } else {
        ProviderHealth::Unknown
    }
}

fn prefer_summary(title: &str, body: &str) -> String {
    if body.is_empty() || body == title {
        title.to_string()
    } else if title.is_empty() {
        body.to_string()
    } else {
        format!("{title} — {body}")
    }
}

fn decode_xml(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn strip_markup(text: &str) -> String {
    let without_cdata = text.replace("<![CDATA[", "").replace("]]>", "");
    let no_tags = Regex::new(r"(?s)<[^>]+>")
        .expect("valid regex")
        .replace_all(&without_cdata, " ");
    no_tags.split_whitespace().collect::<Vec<_>>().join(" ")
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

#[derive(Debug, Deserialize)]
struct GoogleProductsResponse {
    products: Vec<GoogleProduct>,
}

#[derive(Debug, Deserialize)]
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
    fn parses_xai_rss_feed() {
        let xml = r#"
        <rss version="2.0">
          <channel>
            <title>xAI Status - Incident History</title>
            <link>https://status.x.ai</link>
            <lastBuildDate>Thu, 12 Mar 2026 00:43:25 GMT</lastBuildDate>
            <item>
              <title>Degraded website login</title>
              <description><![CDATA[<strong>RESOLVED</strong> - <p>This incident has been resolved.</p>]]></description>
              <pubDate>Thu, 19 Feb 2026 16:38:24 GMT</pubDate>
              <link>https://status.x.ai/incidents/lrkj1G0wmMoe</link>
            </item>
          </channel>
        </rss>
        "#;

        let parsed = parse_feed(OfficialStatusSource::Xai, xml).expect("rss parses");
        assert_eq!(parsed.method, StatusSourceMethod::Feed);
        assert_eq!(parsed.health, ProviderHealth::Operational);
        assert_eq!(
            parsed.summary.as_deref(),
            Some("Degraded website login — RESOLVED - This incident has been resolved.")
        );
    }

    #[test]
    fn parses_atom_feed() {
        let xml = r#"
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Azure Status - Incident history</title>
          <updated>2026-02-16T21:35:20.315+00:00</updated>
          <entry>
            <title>Service incident</title>
            <updated>2026-02-16T21:35:20.315+00:00</updated>
            <link rel="alternate" type="text/html" href="https://azure.status.microsoft/en-us/status/incident/test"/>
            <content type="html"><![CDATA[<p><strong>Investigating</strong> - We are currently investigating this incident.</p>]]></content>
          </entry>
        </feed>
        "#;

        let parsed = parse_feed(OfficialStatusSource::Azure, xml).expect("atom parses");
        assert_eq!(parsed.method, StatusSourceMethod::Feed);
        assert_eq!(parsed.health, ProviderHealth::Degraded);
        assert_eq!(
            parsed.official_url,
            "https://azure.status.microsoft/en-us/status/incident/test"
        );
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
