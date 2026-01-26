use anyhow::{Context, Result};
use std::path::Path;

use super::data::AgentsFile;

const EMBEDDED_AGENTS: &str = include_str!("../../data/agents.json");

pub fn load_agents() -> Result<AgentsFile> {
    serde_json::from_str(EMBEDDED_AGENTS).context("Failed to parse embedded agents.json")
}

pub fn load_agents_from_file(path: &Path) -> Result<AgentsFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read agents file: {}", path.display()))?;
    serde_json::from_str(&content).context("Failed to parse agents.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_embedded_agents() {
        let agents = load_agents().expect("Should load embedded agents");
        assert!(agents.schema_version >= 1);
        assert!(!agents.agents.is_empty());
        assert!(agents.agents.contains_key("claude-code"));
        assert!(agents.agents.contains_key("aider"));
    }
}
