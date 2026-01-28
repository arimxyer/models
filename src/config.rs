use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub config_version: u32,
    #[serde(default)]
    pub agents: AgentsConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomAgent {
    pub name: String,
    pub repo: String,
    #[serde(default)]
    pub agent_type: Option<String>, // "cli" or "ide"
    #[serde(default)]
    pub binary: Option<String>,
    #[serde(default)]
    pub version_command: Option<Vec<String>>,
}

impl CustomAgent {
    pub fn to_agent(&self) -> crate::agents::Agent {
        crate::agents::Agent {
            name: self.name.clone(),
            repo: self.repo.clone(),
            categories: self
                .agent_type
                .as_ref()
                .map(|t| vec![t.clone()])
                .unwrap_or_default(),
            cli_binary: self.binary.clone(),
            version_command: self.version_command.clone().unwrap_or_default(),
            installation_method: self.agent_type.clone(),
            pricing: None,
            supported_providers: vec![],
            platform_support: vec![],
            open_source: true,
            version_regex: None,
            config_files: vec![],
            homepage: None,
            docs: None,
        }
    }
}

/// Default starter agents for new users
fn default_tracked_agents() -> HashSet<String> {
    ["claude-code", "codex", "gemini-cli", "opencode"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentsConfig {
    #[serde(default = "default_tracked_agents")]
    pub tracked: HashSet<String>,
    #[serde(default)]
    pub excluded: HashSet<String>,
    #[serde(default)]
    pub custom: Vec<CustomAgent>,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            tracked: default_tracked_agents(),
            excluded: HashSet::new(),
            custom: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    #[serde(default = "default_github_ttl")]
    pub github_ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            github_ttl_seconds: default_github_ttl(),
        }
    }
}

fn default_github_ttl() -> u64 {
    3600
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DisplayConfig {
    #[serde(default)]
    pub default_tab: Option<String>,
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("models").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Ok(Self::default()),
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;

        toml::from_str(&content).context("Failed to parse config.toml")
    }

    /// Save config to disk (for future add/remove picker)
    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        let path = match Self::config_path() {
            Some(p) => p,
            None => anyhow::bail!("Could not determine config directory"),
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;

        Ok(())
    }

    pub fn is_tracked(&self, agent_id: &str) -> bool {
        if self.agents.excluded.contains(agent_id) {
            return false;
        }
        self.agents.tracked.contains(agent_id)
    }

    /// Set agent tracking status (for future add/remove picker)
    #[allow(dead_code)]
    pub fn set_tracked(&mut self, agent_id: &str, tracked: bool) {
        if tracked {
            self.agents.tracked.insert(agent_id.to_string());
            self.agents.excluded.remove(agent_id);
        } else {
            self.agents.tracked.remove(agent_id);
            self.agents.excluded.insert(agent_id.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.cache.github_ttl_seconds, 3600);
        // Default includes starter agents
        assert_eq!(config.agents.tracked.len(), 4);
        assert!(config.agents.tracked.contains("claude-code"));
        assert!(config.agents.tracked.contains("codex"));
        assert!(config.agents.tracked.contains("gemini-cli"));
        assert!(config.agents.tracked.contains("opencode"));
    }

    #[test]
    fn test_is_tracked_default() {
        let config = Config::default();
        // Default tracked agents
        assert!(config.is_tracked("claude-code"));
        assert!(config.is_tracked("codex"));
        // Not in default list
        assert!(!config.is_tracked("aider"));
        assert!(!config.is_tracked("cursor"));
    }

    #[test]
    fn test_is_tracked_excluded() {
        let mut config = Config::default();
        config.agents.excluded.insert("claude-code".to_string());
        // Excluded even though in tracked list
        assert!(!config.is_tracked("claude-code"));
        // Still tracked
        assert!(config.is_tracked("codex"));
    }
}
