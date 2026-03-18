use serde::Deserialize;

use crate::status::types::{FallbackSnapshot, ProviderHealth};

impl FallbackSnapshot {
    pub(crate) fn from_api_status(payload: ApiStatusCheckResponse) -> Self {
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

// ---------------------------------------------------------------------------
// Deserialization types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct ApiStatusCheckResponse {
    pub api: ApiStatusCheckApi,
    pub links: ApiStatusCheckLinks,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiStatusCheckApi {
    pub name: String,
    pub description: String,
    pub status_page_url: String,
    pub status: String,
    #[serde(default)]
    pub last_checked: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApiStatusCheckLinks {
    pub page: String,
}
