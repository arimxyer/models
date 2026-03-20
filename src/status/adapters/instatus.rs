use serde_json::Value;

use super::normalize_component_status;
use crate::status::types::{
    available_detail_state, fetch_failed_detail_state, not_attempted_detail_state, ActiveIncident,
    ComponentStatus, OfficialSnapshot, OfficialStatusSource, ProviderHealth, ScheduledMaintenance,
    StatusDetailSource, StatusSourceMethod,
};

fn instatus_status_to_health(status: &str) -> ProviderHealth {
    match status {
        "UP" | "OPERATIONAL" => ProviderHealth::Operational,
        "HASISSUES" | "DEGRADEDPERFORMANCE" => ProviderHealth::Degraded,
        "MAJOROUTAGE" | "PARTIALOUTAGE" => ProviderHealth::Outage,
        "UNDERMAINTENANCE" => ProviderHealth::Maintenance,
        _ => ProviderHealth::Unknown,
    }
}

pub(crate) fn parse_instatus_summary(
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
                        shortlink: i
                            .get("url")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        created_at: i
                            .get("started")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
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
                        // Instatus uses "start" (not "scheduled_for") for the maintenance start time
                        scheduled_for: m
                            .get("start")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
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

pub(crate) fn parse_instatus_components(body: &str) -> Result<Vec<ComponentStatus>, String> {
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

/// Fetches Instatus summary and enriches with component data from a second endpoint.
pub(crate) async fn fetch_instatus_with_components(
    client: &reqwest::Client,
    source: OfficialStatusSource,
) -> Result<OfficialSnapshot, String> {
    use std::time::Duration;
    use tokio::time::timeout;

    let body = super::super::fetch::fetch_text(client, source.endpoint_url()).await?;
    let mut snapshot = parse_instatus_summary(source, &body)?;
    let components_url = format!("{}/v2/components.json", source.page_url());
    match timeout(Duration::from_secs(3), async {
        let text = super::super::fetch::fetch_text(client, &components_url).await?;
        parse_instatus_components(&text)
    })
    .await
    {
        Ok(Ok(components)) => {
            snapshot.components = components;
            snapshot.components_state =
                available_detail_state(&snapshot.components, StatusDetailSource::Enrichment);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_instatus_summary() {
        let summary_json = r#"{
            "page": {"status": "HASISSUES"},
            "activeIncidents": [
                {
                    "name": "Search degraded",
                    "status": "INVESTIGATING",
                    "impact": "MAJOROUTAGE",
                    "started": "Sat Jun 11 2022 18:55:50 GMT+0000 (Coordinated Universal Time)",
                    "url": "https://status.perplexity.com/incident/cl4a8n307"
                }
            ],
            "scheduledMaintenances": [
                {
                    "name": "Planned reboot",
                    "status": "NOTSTARTEDYET",
                    "start": "Sat Jun 11 2022 20:00:00 GMT+0000 (Coordinated Universal Time)",
                    "duration": "60"
                }
            ]
        }"#;

        let snapshot = parse_instatus_summary(OfficialStatusSource::Perplexity, summary_json)
            .expect("parses ok");
        assert_eq!(snapshot.method, StatusSourceMethod::Instatus);
        assert_eq!(snapshot.health, ProviderHealth::Degraded);
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(snapshot.incidents[0].name, "Search degraded");
        assert_eq!(
            snapshot.incidents[0].created_at.as_deref(),
            Some("Sat Jun 11 2022 18:55:50 GMT+0000 (Coordinated Universal Time)")
        );
        assert_eq!(
            snapshot.incidents[0].shortlink.as_deref(),
            Some("https://status.perplexity.com/incident/cl4a8n307")
        );
        assert_eq!(snapshot.maintenance.len(), 1);
        assert_eq!(snapshot.maintenance[0].name, "Planned reboot");
        assert_eq!(
            snapshot.maintenance[0].scheduled_for.as_deref(),
            Some("Sat Jun 11 2022 20:00:00 GMT+0000 (Coordinated Universal Time)")
        );

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
}
