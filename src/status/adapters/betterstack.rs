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

    let status_note = v
        .pointer("/data/attributes/announcement")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let health = match aggregate_state {
        "operational" => ProviderHealth::Operational,
        "degraded" => ProviderHealth::Degraded,
        "downtime" => ProviderHealth::Outage,
        _ => ProviderHealth::Unknown,
    };

    let included = v.get("included").and_then(|v| v.as_array());

    // Build section_id → section_name map from status_page_section entries.
    // Section IDs in the API are numeric but resources reference them as numbers too.
    let section_map: std::collections::HashMap<u64, String> = included
        .map(|arr| {
            arr.iter()
                .filter(|item| {
                    item.get("type").and_then(|v| v.as_str()) == Some("status_page_section")
                })
                .filter_map(|item| {
                    let id = item
                        .get("id")
                        .and_then(|v| v.as_str())?
                        .parse::<u64>()
                        .ok()?;
                    let name = item
                        .pointer("/attributes/name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    if name.is_empty() {
                        return None;
                    }
                    Some((id, name.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    let components: Vec<ComponentStatus> = included
        .map(|arr| {
            arr.iter()
                .filter(|item| {
                    item.get("type").and_then(|v| v.as_str()) == Some("status_page_resource")
                })
                .filter_map(|item| {
                    let name = item
                        .pointer("/attributes/public_name")
                        .and_then(|v| v.as_str())?;
                    let status = item
                        .pointer("/attributes/status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("operational");
                    if status == "not_monitored" {
                        return None;
                    }
                    let group_name = item
                        .pointer("/attributes/status_page_section_id")
                        .and_then(|v| v.as_u64())
                        .and_then(|id| section_map.get(&id))
                        .cloned();
                    Some(ComponentStatus {
                        name: name.to_string(),
                        status: normalize_component_status(status),
                        group_name,
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
                    let updates = item
                        .pointer("/attributes/status_report_updates")
                        .and_then(|v| v.as_array());
                    let message = updates
                        .and_then(|arr| arr.first())
                        .and_then(|u| u.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let created_at = item
                        .pointer("/attributes/created_at")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    // updated_at: prefer the latest update's created_at, fall back to report's updated_at
                    let updated_at = updates
                        .and_then(|arr| arr.first())
                        .and_then(|u| u.get("created_at"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .or_else(|| {
                            item.pointer("/attributes/updated_at")
                                .and_then(|v| v.as_str())
                                .map(str::to_string)
                        });
                    Some(ActiveIncident {
                        name: title.to_string(),
                        status: "investigating".to_string(),
                        impact: message.to_string(),
                        shortlink: None,
                        created_at,
                        updated_at,
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
        status_note,
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
                        "public_name": "Dashboard",
                        "status": "operational"
                    }
                },
                {
                    "id": "sr1",
                    "type": "status_report",
                    "attributes": {
                        "title": "API latency increase",
                        "created_at": "2026-03-19T10:00:00Z",
                        "updated_at": "2026-03-19T10:30:00Z",
                        "status_report_updates": [
                            {
                                "message": "Investigating elevated latency",
                                "created_at": "2026-03-19T10:15:00Z"
                            }
                        ]
                    }
                }
            ]
        }"#;

        let snapshot =
            parse_better_stack(OfficialStatusSource::TogetherAi, json).expect("parses ok");
        assert_eq!(snapshot.method, StatusSourceMethod::BetterStack);
        assert_eq!(snapshot.health, ProviderHealth::Degraded);
        assert_eq!(
            snapshot.components.len(),
            2,
            "should include API and Dashboard"
        );
        assert_eq!(snapshot.components[0].name, "API");
        assert_eq!(snapshot.components[0].status, "degraded_performance");
        assert_eq!(snapshot.components[1].status, "operational");
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(snapshot.incidents[0].name, "API latency increase");
        assert_eq!(
            snapshot.incidents[0].created_at.as_deref(),
            Some("2026-03-19T10:00:00Z")
        );
        // updated_at should be the first update's created_at, not the report's updated_at
        assert_eq!(
            snapshot.incidents[0].updated_at.as_deref(),
            Some("2026-03-19T10:15:00Z")
        );
    }

    #[test]
    fn incident_updated_at_falls_back_to_report_updated_at() {
        let json = r#"{
            "data": {
                "id": "abc",
                "type": "status_page",
                "attributes": { "aggregate_state": "degraded" }
            },
            "included": [
                {
                    "id": "sr1",
                    "type": "status_report",
                    "attributes": {
                        "title": "Outage",
                        "created_at": "2026-03-19T08:00:00Z",
                        "updated_at": "2026-03-19T09:00:00Z",
                        "status_report_updates": [
                            {"message": "We are investigating"}
                        ]
                    }
                }
            ]
        }"#;

        let snapshot =
            parse_better_stack(OfficialStatusSource::TogetherAi, json).expect("parses ok");
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(
            snapshot.incidents[0].created_at.as_deref(),
            Some("2026-03-19T08:00:00Z")
        );
        // No created_at on the update, so falls back to report's updated_at
        assert_eq!(
            snapshot.incidents[0].updated_at.as_deref(),
            Some("2026-03-19T09:00:00Z")
        );
    }

    #[test]
    fn incident_timestamps_absent_when_not_provided() {
        let json = r#"{
            "data": {
                "id": "abc",
                "type": "status_page",
                "attributes": { "aggregate_state": "degraded" }
            },
            "included": [
                {
                    "id": "sr1",
                    "type": "status_report",
                    "attributes": {
                        "title": "Outage",
                        "status_report_updates": [
                            {"message": "Investigating"}
                        ]
                    }
                }
            ]
        }"#;

        let snapshot = parse_better_stack(OfficialStatusSource::Helicone, json).expect("parses ok");
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(snapshot.incidents[0].created_at, None);
        assert_eq!(snapshot.incidents[0].updated_at, None);
    }

    #[test]
    fn populates_group_name_from_sections() {
        let json = r#"{
            "data": {
                "id": "abc",
                "type": "status_page",
                "attributes": { "aggregate_state": "operational" }
            },
            "included": [
                {
                    "id": "231960",
                    "type": "status_page_section",
                    "attributes": { "name": "Website" }
                },
                {
                    "id": "231963",
                    "type": "status_page_section",
                    "attributes": { "name": "Inference - Chat" }
                },
                {
                    "id": "r1",
                    "type": "status_page_resource",
                    "attributes": {
                        "public_name": "Playground",
                        "status": "operational",
                        "status_page_section_id": 231960
                    }
                },
                {
                    "id": "r2",
                    "type": "status_page_resource",
                    "attributes": {
                        "public_name": "Llama 3.3 70B",
                        "status": "degraded",
                        "status_page_section_id": 231963
                    }
                },
                {
                    "id": "r3",
                    "type": "status_page_resource",
                    "attributes": {
                        "public_name": "Orphan Resource",
                        "status": "operational"
                    }
                }
            ]
        }"#;

        let snapshot =
            parse_better_stack(OfficialStatusSource::TogetherAi, json).expect("parses ok");
        assert_eq!(snapshot.components.len(), 3);

        let playground = &snapshot.components[0];
        assert_eq!(playground.name, "Playground");
        assert_eq!(playground.group_name.as_deref(), Some("Website"));

        let llama = &snapshot.components[1];
        assert_eq!(llama.name, "Llama 3.3 70B");
        assert_eq!(llama.group_name.as_deref(), Some("Inference - Chat"));

        let orphan = &snapshot.components[2];
        assert_eq!(orphan.name, "Orphan Resource");
        assert_eq!(orphan.group_name, None);
    }

    #[test]
    fn ignores_empty_named_sections() {
        let json = r#"{
            "data": {
                "id": "abc",
                "type": "status_page",
                "attributes": { "aggregate_state": "operational" }
            },
            "included": [
                {
                    "id": "150464",
                    "type": "status_page_section",
                    "attributes": { "name": "" }
                },
                {
                    "id": "r1",
                    "type": "status_page_resource",
                    "attributes": {
                        "public_name": "api.hconeai.com",
                        "status": "operational",
                        "status_page_section_id": 150464
                    }
                }
            ]
        }"#;

        let snapshot = parse_better_stack(OfficialStatusSource::Helicone, json).expect("parses ok");
        assert_eq!(snapshot.components.len(), 1);
        assert_eq!(
            snapshot.components[0].group_name, None,
            "empty section name should not set group_name"
        );
    }

    #[test]
    fn filters_out_not_monitored_resources() {
        let json = r#"{
            "data": {
                "id": "abc",
                "type": "status_page",
                "attributes": { "aggregate_state": "operational" }
            },
            "included": [
                {
                    "id": "r1",
                    "type": "status_page_resource",
                    "attributes": {
                        "public_name": "helicone.ai",
                        "status": "not_monitored"
                    }
                },
                {
                    "id": "r2",
                    "type": "status_page_resource",
                    "attributes": {
                        "public_name": "api.hconeai.com",
                        "status": "operational"
                    }
                }
            ]
        }"#;

        let snapshot = parse_better_stack(OfficialStatusSource::Helicone, json).expect("parses ok");
        assert_eq!(
            snapshot.components.len(),
            1,
            "not_monitored should be filtered out"
        );
        assert_eq!(snapshot.components[0].name, "api.hconeai.com");
    }

    #[test]
    fn surfaces_announcement_as_status_note() {
        let json = r#"{
            "data": {
                "id": "abc",
                "type": "status_page",
                "attributes": {
                    "aggregate_state": "operational",
                    "announcement": "Scheduled maintenance on March 25th"
                }
            },
            "included": []
        }"#;

        let snapshot =
            parse_better_stack(OfficialStatusSource::TogetherAi, json).expect("parses ok");
        assert_eq!(
            snapshot.status_note.as_deref(),
            Some("Scheduled maintenance on March 25th")
        );
    }

    #[test]
    fn null_announcement_produces_no_status_note() {
        let json = r#"{
            "data": {
                "id": "abc",
                "type": "status_page",
                "attributes": {
                    "aggregate_state": "operational",
                    "announcement": null
                }
            },
            "included": []
        }"#;

        let snapshot =
            parse_better_stack(OfficialStatusSource::TogetherAi, json).expect("parses ok");
        assert_eq!(snapshot.status_note, None);
    }
}
