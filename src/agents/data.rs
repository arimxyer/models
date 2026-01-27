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

/// A single release from GitHub
#[derive(Debug, Clone, Default)]
pub struct Release {
    pub version: String,
    pub date: Option<String>,
    pub changelog: Option<String>,
}

/// GitHub API data - fetched live and cached
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct GitHubData {
    pub releases: Vec<Release>,
    pub stars: Option<u64>,
    pub open_issues: Option<u64>,
    pub license: Option<String>,
    pub last_commit: Option<String>,
}

impl GitHubData {
    /// Get the latest release (first in the list)
    pub fn latest_release(&self) -> Option<&Release> {
        self.releases.first()
    }

    /// Get the latest version string
    pub fn latest_version(&self) -> Option<&str> {
        self.latest_release().map(|r| r.version.as_str())
    }
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
        match (&self.installed.version, self.github.latest_version()) {
            (Some(installed), Some(latest)) => {
                // Try semver comparison, fallback to string
                match (
                    semver::Version::parse(installed),
                    semver::Version::parse(latest),
                ) {
                    (Ok(i), Ok(l)) => l > i,
                    _ => latest != installed,
                }
            }
            _ => false,
        }
    }

    /// Find releases between installed version and latest (exclusive of installed)
    pub fn new_releases(&self) -> Vec<&Release> {
        let installed = match &self.installed.version {
            Some(v) => v,
            None => return self.github.releases.iter().collect(), // All releases are "new" if not installed
        };

        self.github
            .releases
            .iter()
            .take_while(|r| r.version != *installed)
            .collect()
    }
}
