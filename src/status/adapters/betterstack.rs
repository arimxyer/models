use serde_json::Value;

use super::normalize_component_status;
use crate::status::types::{
    available_detail_state, unsupported_detail_state, ActiveIncident, ComponentStatus,
    OfficialSnapshot, OfficialStatusSource, ProviderHealth, StatusDetailSource, StatusSourceMethod,
};

pub(crate) fn parse_better_stack(
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
