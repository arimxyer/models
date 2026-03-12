use regex::Regex;
use serde::Deserialize;

use crate::status::{
    ActiveIncident, ComponentStatus, IncidentUpdate, OfficialStatusSource, ProviderHealth,
    ProviderStatus, ScheduledMaintenance, StatusProvenance, StatusProviderSeed, StatusSourceMethod,
    StatusStrategy,
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

                    let fallback_result = match (official_result.as_ref(), fallback_source_slug) {
                        (Ok(_), _) | (_, None) => None,
                        (Err(_), Some(source_slug)) => match self.fetch_fallback(source_slug).await
                        {
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
                StatusStrategy::Unverified => resolve_provider_status(seed, None, None),
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
            OfficialStatusSource::OpenAi
            | OfficialStatusSource::Anthropic
            | OfficialStatusSource::Moonshot
            | OfficialStatusSource::Vercel
            | OfficialStatusSource::Groq
            | OfficialStatusSource::Cohere
            | OfficialStatusSource::Cerebras
            | OfficialStatusSource::Cloudflare
            | OfficialStatusSource::Cursor
            | OfficialStatusSource::GitHub
            | OfficialStatusSource::DeepSeek => {
                let response = self
                    .client
                    .get(source.endpoint_url())
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
            OfficialStatusSource::OpenRouter
            | OfficialStatusSource::Xai
            | OfficialStatusSource::GitLab
            | OfficialStatusSource::Poe
            | OfficialStatusSource::NanoGpt
            | OfficialStatusSource::Nvidia
            | OfficialStatusSource::Perplexity
            | OfficialStatusSource::HuggingFace
            | OfficialStatusSource::TogetherAi
            | OfficialStatusSource::Helicone
            | OfficialStatusSource::Aws
            | OfficialStatusSource::Azure => {
                let response = self
                    .client
                    .get(source.endpoint_url())
                    .send()
                    .await
                    .map_err(|err| err.to_string())?;

                if !response.status().is_success() {
                    return Err(format!("HTTP {}", response.status()));
                }

                let body = response.text().await.map_err(|err| err.to_string())?;
                parse_feed(source, &body)
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
                    .get(source.endpoint_url())
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
    components: Vec<ComponentStatus>,
    incidents: Vec<ActiveIncident>,
    scheduled_maintenances: Vec<ScheduledMaintenance>,
}

impl OfficialSnapshot {
    fn from_summary(source: OfficialStatusSource, payload: OfficialSummaryResponse) -> Self {
        let incident_summary = payload.incidents.first().map(|incident| {
            format!(
                "{} ({}, {})",
                incident.name, incident.impact, incident.status
            )
        });

        let health = match payload.status.indicator.as_deref() {
            Some("none") => ProviderHealth::Operational,
            Some("minor") => ProviderHealth::Degraded,
            Some("major") | Some("critical") => ProviderHealth::Outage,
            Some("maintenance") => ProviderHealth::Maintenance,
            _ => ProviderHealth::from_api_status(&payload.status.description),
        };

        // Build group name lookup: components where group==true are group headers
        let group_names: Vec<(&str, &str)> = payload
            .components
            .iter()
            .filter(|c| c.group == Some(true))
            .filter_map(|c| c.id.as_deref().map(|id| (id, c.name.as_str())))
            .collect();

        let components: Vec<ComponentStatus> = payload
            .components
            .iter()
            .filter(|c| c.group != Some(true))
            .map(|c| {
                let group_name = c.group_id.as_deref().and_then(|gid| {
                    group_names
                        .iter()
                        .find(|(id, _)| *id == gid)
                        .map(|(_, name)| (*name).to_string())
                });
                ComponentStatus {
                    name: c.name.clone(),
                    status: c.status.clone(),
                    group_name,
                }
            })
            .collect();

        let incidents: Vec<ActiveIncident> = payload
            .incidents
            .iter()
            .map(|inc| {
                let latest_update = inc.incident_updates.first().map(|u| IncidentUpdate {
                    status: u.status.clone(),
                    body: u.body.clone(),
                    created_at: u.created_at.clone(),
                });
                ActiveIncident {
                    name: inc.name.clone(),
                    status: inc.status.clone(),
                    impact: inc.impact.clone(),
                    shortlink: inc.shortlink.clone(),
                    created_at: inc.created_at.clone(),
                    updated_at: inc.updated_at.clone(),
                    latest_update,
                    affected_components: inc.components.iter().map(|c| c.name.clone()).collect(),
                }
            })
            .collect();

        let scheduled_maintenances: Vec<ScheduledMaintenance> = payload
            .scheduled_maintenances
            .iter()
            .map(|m| ScheduledMaintenance {
                name: m.name.clone(),
                status: m.status.clone(),
                impact: m.impact.clone(),
                scheduled_for: m.scheduled_for.clone(),
                scheduled_until: m.scheduled_until.clone(),
                affected_components: m.components.iter().map(|c| c.name.clone()).collect(),
            })
            .collect();

        Self {
            label: payload
                .page
                .name
                .or_else(|| Some(source.label().to_string()))
                .unwrap_or_else(|| source.label().to_string()),
            method: StatusSourceMethod::StatuspageV2,
            health,
            official_url: payload
                .page
                .url
                .unwrap_or_else(|| source.page_url().to_string()),
            last_checked: payload.page.updated_at,
            summary: incident_summary.or(Some(payload.status.description)),
            components,
            incidents,
            scheduled_maintenances,
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

        let active_incidents: Vec<ActiveIncident> = matching
            .iter()
            .map(|incident| {
                let impact = match incident.severity.as_str() {
                    "high" => "major".to_string(),
                    "medium" => "minor".to_string(),
                    _ => "none".to_string(),
                };
                let status = if incident.end.is_none() {
                    "active".to_string()
                } else {
                    "resolved".to_string()
                };
                let shortlink = incident
                    .uri
                    .as_ref()
                    .map(|uri| format!("https://status.cloud.google.com{uri}"));
                ActiveIncident {
                    name: incident.external_desc.clone(),
                    status,
                    impact,
                    shortlink,
                    created_at: incident.begin.clone(),
                    updated_at: incident.end.clone(),
                    latest_update: None,
                    affected_components: vec![],
                }
            })
            .collect();

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
            components: Vec::new(),
            incidents: active_incidents,
            scheduled_maintenances: Vec::new(),
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

fn parse_feed(source: OfficialStatusSource, xml: &str) -> Result<OfficialSnapshot, String> {
    parse_atom_feed(source, xml).or_else(|_| parse_rss_feed(source, xml))
}

fn infer_status_from_feed(text: &str) -> String {
    let upper = text.to_uppercase();
    if upper.contains("RESOLVED") || upper.contains("RECOVERED") {
        "resolved".to_string()
    } else if upper.contains("MONITORING") {
        "monitoring".to_string()
    } else if upper.contains("IDENTIFIED") {
        "identified".to_string()
    } else if upper.contains("INVESTIGATING") {
        "investigating".to_string()
    } else {
        "unknown".to_string()
    }
}

fn infer_impact_from_feed(text: &str) -> String {
    let upper = text.to_uppercase();
    if upper.contains("MAJOR") || upper.contains("OUTAGE") {
        "major".to_string()
    } else if upper.contains("MINOR") || upper.contains("DEGRADED") || upper.contains("PARTIAL") {
        "minor".to_string()
    } else {
        "none".to_string()
    }
}

fn parse_rss_feed(source: OfficialStatusSource, xml: &str) -> Result<OfficialSnapshot, String> {
    let title_re =
        Regex::new(r"(?s)<channel>.*?<title>(.*?)</title>").map_err(|err| err.to_string())?;
    let build_re =
        Regex::new(r"(?s)<lastBuildDate>(.*?)</lastBuildDate>").map_err(|err| err.to_string())?;
    let item_re = Regex::new(r"(?s)<item>(.*?)</item>").map_err(|err| err.to_string())?;
    let item_title_re = Regex::new(r"(?s)<title>(.*?)</title>").map_err(|err| err.to_string())?;
    let item_description_re =
        Regex::new(r"(?s)<description>(.*?)</description>").map_err(|err| err.to_string())?;
    let item_link_re = Regex::new(r"(?s)<link>(.*?)</link>").map_err(|err| err.to_string())?;
    let item_pubdate_re =
        Regex::new(r"(?s)<pubDate>(.*?)</pubDate>").map_err(|err| err.to_string())?;

    let channel_title = title_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string())
        .unwrap_or_else(|| source.label().to_string());
    let last_checked = build_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string());

    let mut incidents = Vec::new();
    let mut first_health = None;
    let mut first_summary = None;
    let mut first_url = None;

    for item_caps in item_re.captures_iter(xml) {
        let item_block = item_caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let title = item_title_re
            .captures(item_block)
            .and_then(|caps| caps.get(1))
            .map(|m| strip_markup(&decode_xml(m.as_str())))
            .unwrap_or_default();
        let description = item_description_re
            .captures(item_block)
            .and_then(|caps| caps.get(1))
            .map(|m| strip_markup(&decode_xml(m.as_str())))
            .unwrap_or_default();
        let link = item_link_re
            .captures(item_block)
            .and_then(|caps| caps.get(1))
            .map(|m| decode_xml(m.as_str()).trim().to_string())
            .unwrap_or_else(|| source.page_url().to_string());
        let pubdate = item_pubdate_re
            .captures(item_block)
            .and_then(|caps| caps.get(1))
            .map(|m| decode_xml(m.as_str()).trim().to_string());

        let combined = format!("{title} {description}");

        if first_health.is_none() {
            first_health = Some(health_from_feed_text(&combined));
            first_summary = Some(prefer_summary(&title, &description));
            first_url = Some(link.clone());
        }

        let latest_update = if !description.is_empty() {
            Some(IncidentUpdate {
                status: infer_status_from_feed(&description),
                body: description,
                created_at: pubdate.clone().unwrap_or_default(),
            })
        } else {
            None
        };

        incidents.push(ActiveIncident {
            name: title,
            status: infer_status_from_feed(&combined),
            impact: infer_impact_from_feed(&combined),
            shortlink: Some(link),
            created_at: pubdate,
            updated_at: None,
            latest_update,
            affected_components: vec![],
        });
    }

    let (health, summary, official_url) = if incidents.is_empty() {
        (
            ProviderHealth::Operational,
            Some("No incidents recorded".to_string()),
            source.page_url().to_string(),
        )
    } else {
        (
            first_health.unwrap_or(ProviderHealth::Operational),
            first_summary,
            first_url.unwrap_or_else(|| source.page_url().to_string()),
        )
    };

    Ok(OfficialSnapshot {
        label: channel_title,
        method: StatusSourceMethod::Feed,
        health,
        official_url,
        last_checked,
        summary,
        components: Vec::new(),
        incidents,
        scheduled_maintenances: Vec::new(),
    })
}

fn parse_atom_feed(source: OfficialStatusSource, xml: &str) -> Result<OfficialSnapshot, String> {
    if !xml.contains("<feed") {
        return Err("not an atom feed".to_string());
    }

    let title_re =
        Regex::new(r"(?s)<feed[^>]*>.*?<title>(.*?)</title>").map_err(|err| err.to_string())?;
    let updated_re = Regex::new(r"(?s)<updated>(.*?)</updated>").map_err(|err| err.to_string())?;
    let entry_re = Regex::new(r"(?s)<entry>(.*?)</entry>").map_err(|err| err.to_string())?;
    let entry_title_re = Regex::new(r"(?s)<title>(.*?)</title>").map_err(|err| err.to_string())?;
    let entry_body_re = Regex::new(r"(?s)<(?:content|summary)[^>]*>(.*?)</(?:content|summary)>")
        .map_err(|err| err.to_string())?;
    let entry_link_re =
        Regex::new(r#"(?s)<link[^>]*href="(.*?)"[^>]*/?>"#).map_err(|err| err.to_string())?;

    let feed_title = title_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string())
        .unwrap_or_else(|| source.label().to_string());
    let last_checked = updated_re
        .captures(xml)
        .and_then(|caps| caps.get(1))
        .map(|m| decode_xml(m.as_str()).trim().to_string());

    let mut incidents = Vec::new();
    let mut first_health = None;
    let mut first_summary = None;
    let mut first_url = None;

    for entry_caps in entry_re.captures_iter(xml) {
        let entry_block = entry_caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let title = entry_title_re
            .captures(entry_block)
            .and_then(|caps| caps.get(1))
            .map(|m| strip_markup(&decode_xml(m.as_str())))
            .unwrap_or_default();
        let body = entry_body_re
            .captures(entry_block)
            .and_then(|caps| caps.get(1))
            .map(|m| strip_markup(&decode_xml(m.as_str())))
            .unwrap_or_default();
        let link = entry_link_re
            .captures(entry_block)
            .and_then(|caps| caps.get(1))
            .map(|m| decode_xml(m.as_str()).trim().to_string())
            .unwrap_or_else(|| source.page_url().to_string());
        let entry_updated = updated_re
            .captures(entry_block)
            .and_then(|caps| caps.get(1))
            .map(|m| decode_xml(m.as_str()).trim().to_string());

        let combined = format!("{title} {body}");

        if first_health.is_none() {
            first_health = Some(health_from_feed_text(&combined));
            first_summary = Some(prefer_summary(&title, &body));
            first_url = Some(link.clone());
        }

        let latest_update = if !body.is_empty() {
            Some(IncidentUpdate {
                status: infer_status_from_feed(&body),
                body: body.clone(),
                created_at: entry_updated.clone().unwrap_or_default(),
            })
        } else {
            None
        };

        incidents.push(ActiveIncident {
            name: title,
            status: infer_status_from_feed(&combined),
            impact: infer_impact_from_feed(&combined),
            shortlink: Some(link),
            created_at: entry_updated,
            updated_at: None,
            latest_update,
            affected_components: vec![],
        });
    }

    if incidents.is_empty() {
        return Err("atom feed entry missing".to_string());
    }

    Ok(OfficialSnapshot {
        label: feed_title,
        method: StatusSourceMethod::Feed,
        health: first_health.unwrap_or(ProviderHealth::Unknown),
        official_url: first_url.unwrap_or_else(|| source.page_url().to_string()),
        last_checked,
        summary: first_summary,
        components: Vec::new(),
        incidents,
        scheduled_maintenances: Vec::new(),
    })
}

fn health_from_feed_text(text: &str) -> ProviderHealth {
    let upper = text.to_uppercase();
    if upper.contains("RECOVERED")
        || upper.contains("RESOLVED")
        || upper.contains("OPERATIONAL AGAIN")
        || upper.contains("ALL SYSTEMS OPERATIONAL")
    {
        ProviderHealth::Operational
    } else if upper.contains("OUTAGE") || upper.contains("WENT DOWN") || upper.contains("DOWN") {
        ProviderHealth::Outage
    } else if upper.contains("INVESTIGATING")
        || upper.contains("IDENTIFIED")
        || upper.contains("DEGRADED")
        || upper.contains("PARTIAL")
        || upper.contains("MINOR")
    {
        ProviderHealth::Degraded
    } else {
        ProviderHealth::Unknown
    }
}

fn prefer_summary(title: &str, body: &str) -> String {
    if body.is_empty() || body == title {
        title.to_string()
    } else if title.is_empty() {
        body.to_string()
    } else {
        format!("{title} — {body}")
    }
}

fn decode_xml(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn strip_markup(text: &str) -> String {
    let without_cdata = text.replace("<![CDATA[", "").replace("]]>", "");
    let no_tags = Regex::new(r"(?s)<[^>]+>")
        .expect("valid regex")
        .replace_all(&without_cdata, " ");
    no_tags.split_whitespace().collect::<Vec<_>>().join(" ")
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
        status.components = official.components;
        status.incidents = official.incidents;
        status.scheduled_maintenances = official.scheduled_maintenances;
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
        StatusStrategy::Unverified => Some(
            "No verified machine-readable official or fallback source has been added for this provider yet."
                .to_string(),
        ),
        StatusStrategy::OfficialFirst {
            fallback_source_slug: Some(_),
            ..
        } => Some("Official source unavailable and no fallback data could be loaded.".to_string()),
        StatusStrategy::OfficialFirst {
            fallback_source_slug: None,
            ..
        } => Some("Official source unavailable and no fallback source is configured.".to_string()),
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
    #[serde(default)]
    components: Vec<OfficialComponent>,
    #[serde(default)]
    scheduled_maintenances: Vec<OfficialScheduledMaintenance>,
}

#[derive(Debug, Deserialize)]
struct OfficialComponent {
    #[serde(default)]
    id: Option<String>,
    name: String,
    status: String,
    #[serde(default)]
    group_id: Option<String>,
    #[serde(default)]
    group: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OfficialScheduledMaintenance {
    name: String,
    status: String,
    impact: String,
    #[serde(default)]
    scheduled_for: Option<String>,
    #[serde(default)]
    scheduled_until: Option<String>,
    #[serde(default)]
    components: Vec<OfficialIncidentComponent>,
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
    #[serde(default)]
    indicator: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OfficialIncident {
    name: String,
    status: String,
    impact: String,
    #[serde(default)]
    shortlink: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    incident_updates: Vec<OfficialIncidentUpdate>,
    #[serde(default)]
    components: Vec<OfficialIncidentComponent>,
}

#[derive(Debug, Deserialize)]
struct OfficialIncidentUpdate {
    status: String,
    body: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct OfficialIncidentComponent {
    name: String,
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
    begin: Option<String>,
    #[serde(default)]
    end: Option<String>,
    #[serde(default)]
    uri: Option<String>,
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
    use crate::status::{status_seed_for_provider, StatusProvenance};

    use super::*;

    fn seed(slug: &str) -> StatusProviderSeed {
        status_seed_for_provider(slug)
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
                components: Vec::new(),
                incidents: Vec::new(),
                scheduled_maintenances: Vec::new(),
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
    fn unverified_provider_stays_unavailable() {
        let status = resolve_provider_status(&seed("qwen"), None, None);
        assert_eq!(status.provenance, StatusProvenance::Unavailable);
        assert!(status
            .summary
            .as_deref()
            .unwrap_or_default()
            .contains("added for this provider yet"));
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
            <item>
              <title>API latency issues</title>
              <description><![CDATA[<strong>Investigating</strong> - We are investigating increased latency.]]></description>
              <pubDate>Wed, 18 Feb 2026 10:00:00 GMT</pubDate>
              <link>https://status.openrouter.ai/incidents/abc123</link>
            </item>
          </channel>
        </rss>
        "#;

        let parsed = parse_feed(OfficialStatusSource::OpenRouter, xml).expect("rss parses");
        assert_eq!(parsed.method, StatusSourceMethod::Feed);
        assert_eq!(parsed.health, ProviderHealth::Operational);
        assert_eq!(
            parsed.summary.as_deref(),
            Some("Degraded website login — RESOLVED - This incident has been resolved.")
        );
        assert_eq!(parsed.incidents.len(), 2);
        assert_eq!(parsed.incidents[0].name, "Degraded website login");
        assert_eq!(parsed.incidents[0].status, "resolved");
        assert_eq!(parsed.incidents[1].name, "API latency issues");
        assert_eq!(parsed.incidents[1].status, "investigating");
        assert_eq!(
            parsed.incidents[1].shortlink.as_deref(),
            Some("https://status.openrouter.ai/incidents/abc123")
        );
    }

    #[test]
    fn parses_atom_feed() {
        let xml = r#"
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Perplexity Status - Incident history</title>
          <updated>2026-02-16T21:35:20.315+00:00</updated>
          <entry>
            <title>Sonar API incident</title>
            <updated>2026-02-16T21:35:20.315+00:00</updated>
            <link rel="alternate" type="text/html" href="https://status.perplexity.com/incident/test"/>
            <content type="html"><![CDATA[<p><strong>Investigating</strong> - We are currently investigating this incident affecting Sonar API.</p>]]></content>
          </entry>
          <entry>
            <title>Search downtime</title>
            <updated>2026-02-15T10:00:00.000+00:00</updated>
            <link rel="alternate" type="text/html" href="https://status.perplexity.com/incident/older"/>
            <content type="html"><![CDATA[<p><strong>Resolved</strong> - This issue has been resolved.</p>]]></content>
          </entry>
        </feed>
        "#;

        let parsed = parse_feed(OfficialStatusSource::Perplexity, xml).expect("atom parses");
        assert_eq!(parsed.method, StatusSourceMethod::Feed);
        assert_eq!(parsed.health, ProviderHealth::Degraded);
        assert_eq!(
            parsed.official_url,
            "https://status.perplexity.com/incident/test"
        );
        assert_eq!(parsed.incidents.len(), 2);
        assert_eq!(parsed.incidents[0].name, "Sonar API incident");
        assert_eq!(parsed.incidents[0].status, "investigating");
        assert_eq!(parsed.incidents[1].name, "Search downtime");
        assert_eq!(parsed.incidents[1].status, "resolved");
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
            begin: Some("2026-02-27T10:00:00+00:00".to_string()),
            end: Some("2026-02-27T14:35:00+00:00".to_string()),
            uri: Some("/incidents/RiFm4GRdELxBfnY7qRAG".to_string()),
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
        assert_eq!(snapshot.incidents.len(), 1);
        assert_eq!(snapshot.incidents[0].status, "resolved");
        assert_eq!(snapshot.incidents[0].impact, "none");
        assert_eq!(
            snapshot.incidents[0].shortlink.as_deref(),
            Some("https://status.cloud.google.com/incidents/RiFm4GRdELxBfnY7qRAG")
        );
        assert_eq!(
            snapshot.incidents[0].created_at.as_deref(),
            Some("2026-02-27T10:00:00+00:00")
        );
    }
}
