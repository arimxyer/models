use regex::Regex;
use serde::Deserialize;

use crate::status::{
    OfficialStatusSource, ProviderHealth, ProviderStatus, StatusProvenance, StatusProviderSeed,
    StatusSourceMethod, StatusStrategy,
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

                    let fallback_result = match official_result {
                        Ok(_) => None,
                        Err(_) => match self.fetch_fallback(fallback_source_slug).await {
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
                StatusStrategy::FallbackOnly {
                    fallback_source_slug,
                } => {
                    let fallback_result = match self.fetch_fallback(fallback_source_slug).await {
                        Ok(snapshot) => {
                            successes += 1;
                            Some(snapshot)
                        }
                        Err(err) => {
                            last_error = Some(err);
                            None
                        }
                    };
                    resolve_provider_status(seed, None, fallback_result)
                }
                StatusStrategy::Unsupported => resolve_provider_status(seed, None, None),
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
        match source {
            OfficialStatusSource::OpenAi | OfficialStatusSource::Anthropic => {
                let response = self
                    .client
                    .get(source.summary_url())
                    .send()
                    .await
                    .map_err(|err| err.to_string())?;

                if !response.status().is_success() {
                    return Err(format!("HTTP {}", response.status()));
                }

                let payload: OfficialSummaryResponse =
                    response.json().await.map_err(|err| err.to_string())?;
                Ok(OfficialSnapshot::from_summary(source, payload))
            }
            OfficialStatusSource::OpenRouterRss => {
                let response = self
                    .client
                    .get(source.summary_url())
                    .send()
                    .await
                    .map_err(|err| err.to_string())?;

                if !response.status().is_success() {
                    return Err(format!("HTTP {}", response.status()));
                }

                let body = response.text().await.map_err(|err| err.to_string())?;
                parse_openrouter_rss(&body)
            }
            OfficialStatusSource::GoogleGeminiJson => {
                let products =
                    google_products.ok_or_else(|| "missing google products catalog".to_string())?;
                let product = products
                    .products
                    .iter()
                    .find(|product| product.title == "Vertex Gemini API")
                    .ok_or_else(|| {
                        "Vertex Gemini API not found in Google products catalog".to_string()
                    })?;

                let response = self
                    .client
                    .get(source.summary_url())
                    .send()
                    .await
                    .map_err(|err| err.to_string())?;

                if !response.status().is_success() {
                    return Err(format!("HTTP {}", response.status()));
                }

                let incidents: Vec<GoogleIncident> =
                    response.json().await.map_err(|err| err.to_string())?;
                Ok(OfficialSnapshot::from_google(product, &incidents))
            }
        }
    }

    async fn fetch_google_products(&self) -> Result<GoogleProductsResponse, String> {
        let response = self
            .client
            .get(GOOGLE_PRODUCTS_URL)
            .send()
            .await
            .map_err(|err| err.to_string())?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        response.json().await.map_err(|err| err.to_string())
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
}

impl OfficialSnapshot {
    fn from_summary(source: OfficialStatusSource, payload: OfficialSummaryResponse) -> Self {
        let incident_summary = payload.incidents.first().map(|incident| {
            format!(
                "{} ({}, {})",
                incident.name, incident.impact, incident.status
            )
        });

        Self {
            label: payload
                .page
                .name
                .or_else(|| Some(source.label().to_string()))
                .unwrap_or_else(|| source.label().to_string()),
            method: StatusSourceMethod::StatuspageV2,
            health: ProviderHealth::from_api_status(&payload.status.description),
            official_url: payload
                .page
                .url
                .unwrap_or_else(|| source.summary_url().to_string()),
            last_checked: payload.page.updated_at,
            summary: incident_summary.or(Some(payload.status.description)),
        }
    }

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

fn parse_openrouter_rss(xml: &str) -> Result<OfficialSnapshot, String> {
    let title_re = Regex::new(r"(?s)<title>([^<]+)</title>").map_err(|err| err.to_string())?;
    let build_re =
        Regex::new(r"(?s)<lastBuildDate>([^<]+)</lastBuildDate>").map_err(|err| err.to_string())?;
    let item_re = Regex::new(r"(?s)<item>\s*<title>([^<]+)</title>.*?<description><!\[CDATA\[(.*?)\]\]></description>.*?<link>([^<]+)</link>")
        .map_err(|err| err.to_string())?;

    let channel_title = title_re
        .captures_iter(xml)
        .next()
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "OpenRouter Status".to_string());
    let last_checked = build_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string());
    let item = item_re.captures(xml);

    let (health, summary, official_url) = if let Some(caps) = item {
        let incident_title = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        let description = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let link = caps
            .get(3)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "https://status.openrouter.ai".to_string());
        let upper = description.to_uppercase();
        let health = if upper.contains("RESOLVED") || upper.contains("OPERATIONAL AGAIN") {
            ProviderHealth::Operational
        } else if upper.contains("INVESTIGATING") || upper.contains("IDENTIFIED") {
            ProviderHealth::Degraded
        } else {
            ProviderHealth::Unknown
        };
        (health, Some(incident_title.to_string()), link)
    } else {
        (
            ProviderHealth::Operational,
            Some("No incidents recorded".to_string()),
            "https://status.openrouter.ai".to_string(),
        )
    };

    Ok(OfficialSnapshot {
        label: channel_title,
        method: StatusSourceMethod::RssFeed,
        health,
        official_url,
        last_checked,
        summary,
    })
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
        StatusStrategy::Unsupported => Some(
            "No verified machine-readable official or fallback source is configured for this provider."
                .to_string(),
        ),
        StatusStrategy::OfficialFirst { .. } => {
            Some("Official source unavailable and no fallback data could be loaded.".to_string())
        }
        StatusStrategy::FallbackOnly { .. } => {
            Some("Fallback source unavailable for this provider.".to_string())
        }
    };
    status
}

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
    use crate::status::{
        display_name_for_provider, source_slug_for_provider, strategy_for_provider,
        StatusProvenance,
    };

    use super::*;

    fn seed(slug: &str) -> StatusProviderSeed {
        StatusProviderSeed {
            slug: slug.to_string(),
            display_name: display_name_for_provider(slug),
            source_slug: source_slug_for_provider(slug).to_string(),
            strategy: strategy_for_provider(slug),
        }
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
    fn unsupported_provider_stays_unavailable() {
        let status = resolve_provider_status(&seed("moonshot"), None, None);
        assert_eq!(status.provenance, StatusProvenance::Unavailable);
        assert!(status
            .summary
            .as_deref()
            .unwrap_or_default()
            .contains("No verified machine-readable"));
    }

    #[test]
    fn parses_openrouter_rss_feed() {
        let xml = r#"
        <rss version="2.0">
          <channel>
            <title>OpenRouter Status - Incident History</title>
            <link>https://status.openrouter.ai</link>
            <lastBuildDate>Thu, 12 Mar 2026 00:43:25 GMT</lastBuildDate>
            <item>
              <title>Degraded website login</title>
              <description><![CDATA[<strong>RESOLVED</strong> - <p>This incident has been resolved.</p>]]></description>
              <pubDate>Thu, 19 Feb 2026 16:38:24 GMT</pubDate>
              <link>https://status.openrouter.ai/incidents/lrkj1G0wmMoe</link>
            </item>
          </channel>
        </rss>
        "#;

        let parsed = parse_openrouter_rss(xml).expect("rss parses");
        assert_eq!(parsed.method, StatusSourceMethod::RssFeed);
        assert_eq!(parsed.health, ProviderHealth::Operational);
        assert_eq!(parsed.summary.as_deref(), Some("Degraded website login"));
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
}
