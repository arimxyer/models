use serde::Deserialize;

use crate::status::types::{available_detail_state, fetch_failed_detail_state};
use crate::status::types::{
    ActiveIncident, ComponentStatus, IncidentUpdate, OfficialSnapshot, OfficialStatusSource,
    ProviderHealth, ScheduledMaintenance, StatusDetailSource, StatusDetailState,
};

pub(crate) fn parse_statuspage_v2_summary(
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

    // Pass 1: Build group id → name map from group containers
    let group_map: std::collections::HashMap<&str, &str> = payload
        .components
        .iter()
        .filter(|c| c.group)
        .filter_map(|c| {
            let id = c.id.as_deref()?;
            Some((id, c.name.as_str()))
        })
        .collect();

    // Pass 2: Map non-group components, resolving group_id → group_name
    let components: Vec<ComponentStatus> = payload
        .components
        .iter()
        .filter(|c| !c.group)
        .map(|c| {
            let group_name = c
                .group_id
                .as_deref()
                .and_then(|gid| group_map.get(gid))
                .map(|name| (*name).to_string());
            ComponentStatus {
                name: c.name.clone(),
                status: c.status.clone(),
                group_name,
                position: c.position,
                only_show_if_degraded: c.only_show_if_degraded,
            }
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
            shortlink: m.shortlink.clone(),
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
        health: payload
            .status
            .indicator
            .as_deref()
            .map(ProviderHealth::from_indicator)
            .unwrap_or_else(|| ProviderHealth::from_api_status(&payload.status.description)),
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

pub(crate) fn parse_incidents_json(body: &str) -> Result<Vec<ActiveIncident>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|err| err.to_string())?;
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
// Maintenances JSON parser (for dedicated /api/v2/scheduled-maintenances.json)
// ---------------------------------------------------------------------------

pub(crate) fn parse_maintenances_json(body: &str) -> Result<Vec<ScheduledMaintenance>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|err| err.to_string())?;
    let maintenances = v
        .get("scheduled_maintenances")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|m| {
                    // Only keep scheduled/in_progress/verifying — skip completed
                    let status = m.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    status != "completed"
                })
                .map(|m| ScheduledMaintenance {
                    name: m
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    status: m
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    impact: m
                        .get("impact")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    shortlink: m
                        .get("shortlink")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    scheduled_for: m
                        .get("scheduled_for")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    scheduled_until: m
                        .get("scheduled_until")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    affected_components: m
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
    Ok(maintenances)
}

/// Fetches maintenance enrichment from the dedicated endpoint.
/// Returns Ok(maintenances) on success, Err on failure/timeout.
pub(crate) async fn fetch_maintenance_enrichment(
    client: &reqwest::Client,
    page_url: &str,
) -> Result<Vec<ScheduledMaintenance>, String> {
    use std::time::Duration;
    use tokio::time::timeout;

    let url = format!("{}/api/v2/scheduled-maintenances.json", page_url);
    timeout(Duration::from_secs(3), async {
        let text = super::super::fetch::fetch_text(client, &url).await?;
        parse_maintenances_json(&text)
    })
    .await
    .map_err(|_| "Maintenance details timed out after 3s.".to_string())?
}

/// Wraps `parse_statuspage_v2_summary` for incident.io shim sources:
/// clears inline incidents, then enriches from the separate incidents endpoint.
pub(crate) async fn fetch_incident_io_shim(
    client: &reqwest::Client,
    source: OfficialStatusSource,
) -> Result<OfficialSnapshot, String> {
    use crate::status::types::{not_attempted_detail_state, unsupported_detail_state};
    use std::time::Duration;
    use tokio::time::timeout;

    let body = super::super::fetch::fetch_text(client, source.endpoint_url()).await?;
    let mut snapshot = parse_statuspage_v2_summary(source, &body)?;
    // incident.io has no maintenance API — mark as unsupported initially
    snapshot.maintenance.clear();
    snapshot.maintenance_state =
        unsupported_detail_state("incident.io does not expose scheduled maintenance data.");
    snapshot.incidents.clear();
    snapshot.incidents_state = not_attempted_detail_state(
        StatusDetailSource::Enrichment,
        "Incident details require a second incident feed for this source.",
    );
    let incidents_url = format!("{}/api/v2/incidents.json", source.page_url());
    match timeout(Duration::from_secs(3), async {
        let text = super::super::fetch::fetch_text(client, &incidents_url).await?;
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

    // Enrich maintenance from dedicated endpoint (may 404 on incident.io)
    if snapshot.maintenance.is_empty() {
        let maintenance_url = format!("{}/api/v2/scheduled-maintenances.json", source.page_url());
        match timeout(Duration::from_secs(3), async {
            let text = super::super::fetch::fetch_text(client, &maintenance_url).await?;
            parse_maintenances_json(&text)
        })
        .await
        {
            Ok(Ok(maintenances)) => {
                snapshot.maintenance = maintenances;
                snapshot.maintenance_state =
                    available_detail_state(&snapshot.maintenance, StatusDetailSource::Enrichment);
            }
            Ok(Err(_)) | Err(_) => {
                // incident.io may not support this endpoint — silently keep
                // the inline state rather than marking as failed.
            }
        }
    }

    Ok(snapshot)
}

// ---------------------------------------------------------------------------
// Deserialization types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct OfficialSummaryResponse {
    pub page: OfficialPage,
    pub status: OfficialStatus,
    #[serde(default)]
    pub incidents: Vec<OfficialIncident>,
    #[serde(default)]
    pub components: Vec<OfficialComponent>,
    #[serde(default)]
    pub scheduled_maintenances: Vec<OfficialScheduledMaintenance>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OfficialPage {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OfficialStatus {
    pub description: String,
    #[serde(default)]
    pub indicator: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OfficialIncident {
    pub name: String,
    pub status: String,
    pub impact: String,
    #[serde(default)]
    pub shortlink: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub started_at: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub resolved_at: Option<String>,
    #[serde(default)]
    pub incident_updates: Vec<OfficialIncidentUpdate>,
    #[serde(default)]
    pub components: Vec<OfficialComponentRef>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OfficialIncidentUpdate {
    pub status: String,
    pub body: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OfficialComponentRef {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OfficialComponent {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub group: bool,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub position: Option<u16>,
    #[serde(default)]
    pub only_show_if_degraded: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OfficialScheduledMaintenance {
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub impact: Option<String>,
    #[serde(default)]
    pub shortlink: Option<String>,
    #[serde(default)]
    pub scheduled_for: Option<String>,
    #[serde(default)]
    pub scheduled_until: Option<String>,
    #[serde(default)]
    pub components: Vec<OfficialComponentRef>,
}

// Suppress dead code warnings for deserialization fields
impl StatusDetailState {
    // Reuse from types — this import just ensures the struct is accessible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_incident_io_summary_with_incidents() {
        let summary_json = r#"{
            "page": {"name": "OpenAI", "url": "https://status.openai.com", "updated_at": "2026-03-12T00:00:00Z"},
            "status": {"indicator": "none", "description": "All Systems Operational"},
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
    fn incident_io_summary_missing_maintenance_key_parses_as_empty() {
        use crate::status::types::StatusDetailAvailability;
        // incident.io summary.json omits `scheduled_maintenances` entirely
        let json = r#"{
            "page": {"name": "OpenAI", "url": "https://status.openai.com", "updated_at": "2026-03-20T00:00:00Z"},
            "status": {"indicator": "none", "description": "All Systems Operational"},
            "incidents": [],
            "components": []
        }"#;

        let snapshot = parse_statuspage_v2_summary(OfficialStatusSource::OpenAi, json)
            .expect("parses summary without scheduled_maintenances key");

        // Without the shim fixup, this will be NoneReported (wrong for incident.io)
        // The raw parse returns NoneReported for empty vec — the shim overrides to Unsupported
        assert_eq!(snapshot.maintenance.len(), 0);
        assert_eq!(
            snapshot.maintenance_state.availability,
            StatusDetailAvailability::NoneReported,
            "Raw parse (before shim) should report NoneReported for missing key"
        );
    }

    #[test]
    fn parses_maintenance_json() {
        let json = r#"{
            "scheduled_maintenances": [
                {
                    "name": "Database Migration",
                    "status": "scheduled",
                    "impact": "maintenance",
                    "scheduled_for": "2026-03-25T02:00:00Z",
                    "scheduled_until": "2026-03-25T06:00:00Z",
                    "components": [{"name": "API"}, {"name": "Dashboard"}]
                },
                {
                    "name": "Old Maintenance",
                    "status": "completed",
                    "impact": "maintenance",
                    "scheduled_for": "2026-03-01T00:00:00Z",
                    "scheduled_until": "2026-03-01T04:00:00Z",
                    "components": []
                }
            ]
        }"#;

        let maintenances = parse_maintenances_json(json).expect("parses");
        assert_eq!(maintenances.len(), 1, "completed should be filtered out");
        assert_eq!(maintenances[0].name, "Database Migration");
        assert_eq!(maintenances[0].status, "scheduled");
        assert_eq!(
            maintenances[0].affected_components,
            vec!["API", "Dashboard"]
        );
    }

    #[test]
    fn parses_empty_maintenance_json() {
        let json = r#"{"scheduled_maintenances": []}"#;
        let maintenances = parse_maintenances_json(json).expect("parses");
        assert!(maintenances.is_empty());
    }

    #[test]
    fn resolves_component_group_names() {
        let json = r#"{
            "page": {"name": "Cloudflare"},
            "status": {"indicator": "none", "description": "All Systems Operational"},
            "incidents": [],
            "components": [
                {"id": "g1", "name": "Website", "status": "operational", "group": true, "group_id": null},
                {"id": "c1", "name": "CDN", "status": "operational", "group": false, "group_id": "g1"},
                {"id": "c2", "name": "DNS", "status": "degraded_performance", "group": false, "group_id": "g1"},
                {"id": "c3", "name": "API", "status": "operational", "group": false, "group_id": null}
            ],
            "scheduled_maintenances": []
        }"#;

        let snapshot =
            parse_statuspage_v2_summary(OfficialStatusSource::Cloudflare, json).expect("parses");

        // Group container filtered out
        assert_eq!(snapshot.components.len(), 3);

        // Children get group_name from parent
        assert_eq!(snapshot.components[0].name, "CDN");
        assert_eq!(
            snapshot.components[0].group_name.as_deref(),
            Some("Website")
        );

        assert_eq!(snapshot.components[1].name, "DNS");
        assert_eq!(
            snapshot.components[1].group_name.as_deref(),
            Some("Website")
        );

        // Ungrouped component has no group_name
        assert_eq!(snapshot.components[2].name, "API");
        assert_eq!(snapshot.components[2].group_name, None);
    }
}
