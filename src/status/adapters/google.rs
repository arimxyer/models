use serde::Deserialize;

use crate::status::types::{
    not_attempted_detail_state, unsupported_detail_state, OfficialSnapshot, ProviderHealth,
    StatusDetailSource, StatusSourceMethod,
};

pub(crate) fn build_google_snapshot(
    product: &GoogleProduct,
    incidents: &[GoogleIncident],
) -> OfficialSnapshot {
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

    OfficialSnapshot {
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

// ---------------------------------------------------------------------------
// Deserialization types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GoogleProductsResponse {
    pub products: Vec<GoogleProduct>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GoogleProduct {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleIncident {
    pub external_desc: String,
    #[serde(default)]
    pub modified: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    pub severity: String,
    pub status_impact: String,
    #[serde(default)]
    pub affected_products: Vec<GoogleAffectedProduct>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleAffectedProduct {
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::types::StatusDetailAvailability;

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

        let snapshot = build_google_snapshot(&product, &incidents);
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
}
