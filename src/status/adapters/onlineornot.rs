use serde_json::Value;

use super::normalize_component_status;
use crate::status::types::{
    available_detail_state, ActiveIncident, ComponentStatus, OfficialSnapshot,
    OfficialStatusSource, ProviderHealth, ScheduledMaintenance, StatusDetailSource,
    StatusSourceMethod,
};

pub(crate) fn parse_onlineornot(
    source: OfficialStatusSource,
    body: &str,
) -> Result<OfficialSnapshot, String> {
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
                        position: None,
                        only_show_if_degraded: false,
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
                        shortlink: None,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
