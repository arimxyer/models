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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AgentsConfig {
    #[serde(default)]
    pub tracked: HashSet<String>,
    #[serde(default)]
    pub excluded: HashSet<String>,
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
        self.agents.tracked.is_empty() || self.agents.tracked.contains(agent_id)
    }

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
        assert!(config.agents.tracked.is_empty());
    }

    #[test]
    fn test_is_tracked_empty() {
        let config = Config::default();
        // Empty tracked list means all are tracked by default
        assert!(config.is_tracked("claude-code"));
    }

    #[test]
    fn test_is_tracked_excluded() {
        let mut config = Config::default();
        config.agents.excluded.insert("aider".to_string());
        assert!(!config.is_tracked("aider"));
        assert!(config.is_tracked("claude-code"));
    }
}
