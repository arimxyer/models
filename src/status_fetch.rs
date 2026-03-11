use std::collections::HashMap;

use serde::Deserialize;

use crate::status::{ProviderHealth, ProviderStatus, StatusProviderSeed};

const API_URL: &str = "https://apistatuscheck.com/api/status?api=";

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
        let mut statuses: HashMap<String, ProviderStatus> = seeds
            .iter()
            .map(|seed| (seed.slug.clone(), ProviderStatus::placeholder(seed)))
            .collect();

        let mut by_source: HashMap<&str, Vec<&StatusProviderSeed>> = HashMap::new();
        for seed in seeds {
            by_source
                .entry(seed.source_slug.as_str())
                .or_default()
                .push(seed);
        }

        let mut success_count = 0usize;
        let mut last_error: Option<String> = None;

        for (source_slug, mapped_seeds) in by_source {
            let url = format!("{API_URL}{source_slug}");
            let response = match self.client.get(&url).send().await {
                Ok(response) => response,
                Err(err) => {
                    last_error = Some(err.to_string());
                    continue;
                }
            };

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                continue;
            }

            if !response.status().is_success() {
                last_error = Some(format!("HTTP {}", response.status()));
                continue;
            }

            let payload: ApiStatusResponse = match response.json().await {
                Ok(payload) => payload,
                Err(err) => {
                    last_error = Some(err.to_string());
                    continue;
                }
            };

            success_count += 1;

            for seed in mapped_seeds {
                if let Some(entry) = statuses.get_mut(&seed.slug) {
                    entry.source_name = Some(payload.api.name.clone());
                    entry.category = Some(payload.api.category.clone());
                    entry.description = Some(payload.api.description.clone());
                    entry.health = ProviderHealth::from_api_status(&payload.api.status);
                    entry.last_checked = payload.api.last_checked.clone();
                    entry.status_page_url = Some(payload.api.status_page_url.clone());
                    entry.docs_url = Some(payload.api.docs_url.clone());
                    entry.source_page_url = Some(payload.links.page.clone());
                    entry.history_url = Some(payload.links.history.clone());
                }
            }
        }

        if success_count == 0 {
            return StatusFetchResult::Error(
                last_error.unwrap_or_else(|| "Failed to fetch provider statuses".to_string()),
            );
        }

        let mut entries: Vec<_> = statuses.into_values().collect();
        entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        StatusFetchResult::Fresh(entries)
    }
}

#[derive(Debug, Deserialize)]
struct ApiStatusResponse {
    api: ApiStatusApi,
    links: ApiStatusLinks,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiStatusApi {
    name: String,
    category: String,
    description: String,
    status_page_url: String,
    docs_url: String,
    status: String,
    #[serde(default)]
    last_checked: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiStatusLinks {
    page: String,
    history: String,
}
