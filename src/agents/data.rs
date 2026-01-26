use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentsFile {
    pub schema_version: u32,
    #[serde(default)]
    pub last_scraped: Option<String>,
    #[serde(default)]
    pub scrape_source: Option<String>,
    pub agents: HashMap<String, Agent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Agent {
    pub name: String,
    pub repo: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub installation_method: Option<String>,
    #[serde(default)]
    pub pricing: Option<Pricing>,
    #[serde(default)]
    pub supported_providers: Vec<String>,
    #[serde(default)]
    pub platform_support: Vec<String>,
    #[serde(default)]
    pub open_source: bool,
    #[serde(default)]
    pub cli_binary: Option<String>,
    #[serde(default)]
    pub version_command: Vec<String>,
    #[serde(default)]
    pub version_regex: Option<String>,
    #[serde(default)]
    pub config_files: Vec<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub docs: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pricing {
    pub model: String,
    #[serde(default)]
    pub subscription_price: Option<f64>,
    #[serde(default)]
    pub subscription_period: Option<String>,
    #[serde(default)]
    pub free_tier: bool,
    #[serde(default)]
    pub usage_notes: Option<String>,
}

/// GitHub API data - fetched live and cached (fields for future use)
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct GitHubData {
    pub latest_version: Option<String>,
    pub release_date: Option<String>,
    pub changelog: Option<String>,
    pub stars: Option<u64>,
    pub open_issues: Option<u64>,
    pub license: Option<String>,
    pub last_commit: Option<String>,
}

/// Installed CLI info - path field for future use
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct InstalledInfo {
    pub version: Option<String>,
    pub path: Option<String>,
}

/// Agent entry combining static and runtime data
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AgentEntry {
    pub id: String,
    pub agent: Agent,
    pub github: GitHubData,
    pub installed: InstalledInfo,
    pub tracked: bool,
}

impl AgentEntry {
    pub fn update_available(&self) -> bool {
        match (&self.installed.version, &self.github.latest_version) {
            (Some(installed), Some(latest)) => {
                // Try semver comparison, fallback to string
                match (semver::Version::parse(installed), semver::Version::parse(latest)) {
                    (Ok(i), Ok(l)) => l > i,
                    _ => latest != installed,
                }
            }
            _ => false,
        }
    }

    pub fn status_str(&self) -> &'static str {
        if self.installed.version.is_none() {
            "Not Inst"
        } else if self.update_available() {
            "⬆ Update"
        } else {
            "✓ Latest"
        }
    }
}
