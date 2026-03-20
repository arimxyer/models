use crate::status::{ProviderHealth, ProviderStatus, StatusLoadState};

pub struct AgentServiceMapping {
    pub agent_id: &'static str,
    pub provider_slug: &'static str,
    /// Exact component name to match (case-insensitive). None = use overall provider health.
    pub component_name: Option<&'static str>,
}

pub static AGENT_SERVICE_MAPPINGS: &[AgentServiceMapping] = &[
    AgentServiceMapping {
        agent_id: "claude-code",
        provider_slug: "anthropic",
        component_name: Some("Claude Code"),
    },
    AgentServiceMapping {
        agent_id: "codex",
        provider_slug: "openai",
        component_name: Some("Codex"),
    },
    AgentServiceMapping {
        agent_id: "cursor",
        provider_slug: "cursor",
        component_name: Some("Chat - Agent Mode"),
    },
    AgentServiceMapping {
        agent_id: "gemini-cli",
        provider_slug: "google",
        component_name: None,
    },
    AgentServiceMapping {
        agent_id: "kimi-cli",
        provider_slug: "moonshot",
        component_name: None,
    },
    AgentServiceMapping {
        agent_id: "zed",
        provider_slug: "zed",
        component_name: Some("Agent"),
    },
];

pub fn service_mapping_for_agent(agent_id: &str) -> Option<&'static AgentServiceMapping> {
    AGENT_SERVICE_MAPPINGS
        .iter()
        .find(|m| m.agent_id == agent_id)
}

pub struct ResolvedHealth {
    pub health: ProviderHealth,
    pub provider_name: String,
    pub component_name: Option<String>,
}

/// Resolve an agent's service health from status data.
/// Returns None if no mapping exists or status data is unavailable.
pub fn resolve_agent_service_health(
    agent_id: &str,
    status_entries: &[ProviderStatus],
) -> Option<ResolvedHealth> {
    let mapping = service_mapping_for_agent(agent_id)?;

    let provider = status_entries
        .iter()
        .find(|p| p.slug == mapping.provider_slug)?;

    if provider.load_state == StatusLoadState::Placeholder {
        return Some(ResolvedHealth {
            health: ProviderHealth::Unknown,
            provider_name: provider.display_name.clone(),
            component_name: None,
        });
    }

    match mapping.component_name {
        Some(pattern) => {
            let pattern_lower = pattern.to_lowercase();
            let component = provider
                .components
                .iter()
                .find(|c| c.name.to_lowercase() == pattern_lower);

            match component {
                Some(c) => Some(ResolvedHealth {
                    health: ProviderHealth::from_api_status(&c.status),
                    provider_name: provider.display_name.clone(),
                    component_name: Some(c.name.clone()),
                }),
                None => Some(ResolvedHealth {
                    health: provider.health,
                    provider_name: provider.display_name.clone(),
                    component_name: None,
                }),
            }
        }
        None => Some(ResolvedHealth {
            health: provider.health,
            provider_name: provider.display_name.clone(),
            component_name: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapped_agents_have_entries() {
        assert!(service_mapping_for_agent("claude-code").is_some());
        assert!(service_mapping_for_agent("codex").is_some());
        assert!(service_mapping_for_agent("cursor").is_some());
        assert!(service_mapping_for_agent("gemini-cli").is_some());
        assert!(service_mapping_for_agent("kimi-cli").is_some());
    }

    #[test]
    fn unmapped_agents_return_none() {
        assert!(service_mapping_for_agent("aider").is_none());
        assert!(service_mapping_for_agent("opencode").is_none());
    }

    #[test]
    fn resolve_returns_none_for_unmapped() {
        assert!(resolve_agent_service_health("aider", &[]).is_none());
    }

    #[test]
    fn resolve_returns_none_when_provider_missing() {
        assert!(resolve_agent_service_health("claude-code", &[]).is_none());
    }
}
