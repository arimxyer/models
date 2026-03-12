use std::collections::BTreeMap;
use std::collections::HashMap;

use ratatui::widgets::ListState;

use crate::agents::AgentsFile;
use crate::status::{
    canonical_status_slug, status_seed_for_provider, ProviderHealth, ProviderStatus,
    StatusProviderSeed, STATUS_REGISTRY,
};

const PAGE_SIZE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFocus {
    #[default]
    List,
    Details,
}

pub struct StatusApp {
    pub entries: Vec<ProviderStatus>,
    pub filtered_entries: Vec<usize>,
    pub selected: usize,
    pub list_state: ListState,
    pub focus: StatusFocus,
    pub search_query: String,
    pub detail_scroll: u16,
    pub loading: bool,
    pub last_error: Option<String>,
    pub related_agents: HashMap<String, Vec<String>>,
}

impl StatusApp {
    pub fn new(agents_file: &AgentsFile) -> Self {
        let mut by_slug: BTreeMap<String, StatusProviderSeed> = BTreeMap::new();
        let mut related_agents: HashMap<String, Vec<String>> = HashMap::new();

        for entry in STATUS_REGISTRY {
            by_slug.insert(
                entry.slug.to_string(),
                StatusProviderSeed {
                    slug: entry.slug.to_string(),
                    display_name: entry.display_name.to_string(),
                    source_slug: entry.source_slug.to_string(),
                    strategy: entry.strategy,
                },
            );
        }

        for agent in agents_file.agents.values() {
            for slug in &agent.supported_providers {
                let canonical = canonical_status_slug(slug).to_string();
                by_slug
                    .entry(canonical.clone())
                    .or_insert_with(|| status_seed_for_provider(slug));
                related_agents
                    .entry(canonical)
                    .or_default()
                    .push(agent.name.clone());
            }
        }

        let entries: Vec<_> = by_slug.values().map(ProviderStatus::placeholder).collect();

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut app = Self {
            entries,
            filtered_entries: Vec::new(),
            selected: 0,
            list_state,
            focus: StatusFocus::default(),
            search_query: String::new(),
            detail_scroll: 0,
            loading: true,
            last_error: None,
            related_agents,
        };
        app.update_filtered();
        app
    }

    pub fn fetch_seeds(&self) -> Vec<StatusProviderSeed> {
        self.entries
            .iter()
            .map(|entry| status_seed_for_provider(&entry.slug))
            .collect()
    }

    pub fn apply_fetch(&mut self, mut entries: Vec<ProviderStatus>) {
        entries.sort_by(|a, b| {
            a.health
                .sort_rank()
                .cmp(&b.health.sort_rank())
                .then_with(|| a.display_name.cmp(&b.display_name))
        });
        self.entries = entries;
        self.loading = false;
        self.last_error = None;
        self.update_filtered();
    }

    pub fn apply_error(&mut self, error: String) {
        self.loading = false;
        self.last_error = Some(error);
    }

    pub fn update_filtered(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                query.is_empty()
                    || entry.display_name.to_lowercase().contains(&query)
                    || entry.slug.to_lowercase().contains(&query)
                    || entry
                        .source_label
                        .as_ref()
                        .is_some_and(|name| name.to_lowercase().contains(&query))
                    || entry
                        .summary
                        .as_ref()
                        .is_some_and(|summary| summary.to_lowercase().contains(&query))
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.selected >= self.filtered_entries.len() {
            self.selected = 0;
        }
        self.list_state.select(Some(self.selected));
    }

    pub fn current_entry(&self) -> Option<&ProviderStatus> {
        self.filtered_entries
            .get(self.selected)
            .and_then(|&idx| self.entries.get(idx))
    }

    pub fn related_agents_for(&self, slug: &str) -> &[String] {
        self.related_agents
            .get(slug)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn next(&mut self) {
        if self.filtered_entries.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.filtered_entries.len().saturating_sub(1));
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn select_first(&mut self) {
        self.selected = 0;
        self.list_state.select(Some(0));
        self.detail_scroll = 0;
    }

    pub fn select_last(&mut self) {
        if self.filtered_entries.is_empty() {
            return;
        }
        self.selected = self.filtered_entries.len() - 1;
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn page_down(&mut self) {
        if self.filtered_entries.is_empty() {
            return;
        }
        self.selected = (self.selected + PAGE_SIZE).min(self.filtered_entries.len() - 1);
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(PAGE_SIZE);
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn health_counts(&self) -> (usize, usize, usize, usize) {
        let mut op = 0;
        let mut deg = 0;
        let mut out = 0;
        let mut other = 0;
        for entry in &self.entries {
            match entry.health {
                ProviderHealth::Operational => op += 1,
                ProviderHealth::Degraded => deg += 1,
                ProviderHealth::Outage => out += 1,
                _ => other += 1,
            }
        }
        (op, deg, out, other)
    }

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            StatusFocus::List => StatusFocus::Details,
            StatusFocus::Details => StatusFocus::List,
        };
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::agents::{Agent, AgentsFile};

    use super::*;

    #[test]
    fn builds_unique_provider_entries_from_catalog() {
        let mut agents = HashMap::new();
        agents.insert(
            "a".to_string(),
            Agent {
                name: "A".to_string(),
                repo: "owner/a".to_string(),
                categories: vec![],
                installation_method: None,
                pricing: None,
                supported_providers: vec!["openai".to_string(), "google".to_string()],
                platform_support: vec![],
                open_source: true,
                cli_binary: None,
                alt_binaries: vec![],
                version_command: vec![],
                version_regex: None,
                config_files: vec![],
                homepage: None,
                docs: None,
            },
        );
        agents.insert(
            "b".to_string(),
            Agent {
                name: "B".to_string(),
                repo: "owner/b".to_string(),
                categories: vec![],
                installation_method: None,
                pricing: None,
                supported_providers: vec!["openai".to_string()],
                platform_support: vec![],
                open_source: true,
                cli_binary: None,
                alt_binaries: vec![],
                version_command: vec![],
                version_regex: None,
                config_files: vec![],
                homepage: None,
                docs: None,
            },
        );

        let app = StatusApp::new(&AgentsFile {
            schema_version: 1,
            last_scraped: None,
            scrape_source: None,
            agents,
        });

        let slugs: Vec<_> = app
            .entries
            .iter()
            .map(|entry| entry.slug.as_str())
            .collect();
        assert!(slugs.contains(&"google"));
        assert!(slugs.contains(&"openai"));
        assert!(slugs.contains(&"openrouter"));
        assert!(slugs.contains(&"cursor"));
        assert_eq!(
            app.entries
                .iter()
                .find(|entry| entry.slug == "google")
                .map(|entry| entry.source_slug.as_str()),
            Some("gemini")
        );
        assert_eq!(
            app.fetch_seeds()
                .iter()
                .find(|seed| seed.slug == "google")
                .map(|seed| seed.source_slug.as_str()),
            Some("gemini")
        );
    }

    #[test]
    fn health_counts_tallies_all_entries() {
        let mut agents = HashMap::new();
        agents.insert(
            "a".to_string(),
            Agent {
                name: "A".to_string(),
                repo: "owner/a".to_string(),
                categories: vec![],
                installation_method: None,
                pricing: None,
                supported_providers: vec![],
                platform_support: vec![],
                open_source: true,
                cli_binary: None,
                alt_binaries: vec![],
                version_command: vec![],
                version_regex: None,
                config_files: vec![],
                homepage: None,
                docs: None,
            },
        );

        let app = StatusApp::new(&AgentsFile {
            schema_version: 1,
            last_scraped: None,
            scrape_source: None,
            agents,
        });

        // All entries start as Unknown health (from placeholders)
        let (op, deg, out, other) = app.health_counts();
        assert_eq!(op, 0);
        assert_eq!(deg, 0);
        assert_eq!(out, 0);
        assert!(other > 0); // all Unknown = other
    }
}
