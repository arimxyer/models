use serde_json::Value;

use super::normalize_component_status;
use crate::status::types::{
    available_detail_state, ActiveIncident, ComponentStatus, IncidentUpdate, OfficialSnapshot,
    OfficialStatusSource, ProviderHealth, ScheduledMaintenance, StatusDetailSource,
    StatusSourceMethod,
};

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

pub(crate) fn parse_status_io(
    source: OfficialStatusSource,
    body: &str,
) -> Result<OfficialSnapshot, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
