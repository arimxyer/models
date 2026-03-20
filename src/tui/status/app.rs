use std::collections::BTreeMap;
use std::time::Instant;

use ratatui::widgets::ListState;

use crate::agents::AgentsFile;
use crate::status::{
    status_seed_for_provider, ProviderHealth, ProviderStatus, ScheduledMaintenance,
    StatusProvenance, StatusProviderSeed, STATUS_REGISTRY,
};
use crate::tui::widgets::ScrollOffset;

const PAGE_SIZE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFocus {
    #[default]
    List,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverallPanelFocus {
    #[default]
    Incidents,
    Degradation,
    Maintenance,
}

pub struct StatusApp {
    pub entries: Vec<ProviderStatus>,
    pub filtered_entries: Vec<usize>,
    pub selected: usize,
    pub list_state: ListState,
    pub focus: StatusFocus,
    pub overall_panel_focus: OverallPanelFocus,
    pub search_query: String,
    pub detail_scroll: ScrollOffset,
    pub overall_incidents_scroll: ScrollOffset,
    pub overall_degradation_scroll: ScrollOffset,
    pub overall_maintenance_scroll: ScrollOffset,
    pub services_expanded: bool,
    pub services_scroll: ScrollOffset,
    pub loading: bool,
    pub last_refreshed: Option<Instant>,
    pub last_error: Option<String>,
}

impl StatusApp {
    pub fn new(_agents_file: &AgentsFile) -> Self {
        let mut by_slug: BTreeMap<String, StatusProviderSeed> = BTreeMap::new();

        for entry in STATUS_REGISTRY {
            by_slug.insert(
                entry.slug.to_string(),
                StatusProviderSeed {
                    slug: entry.slug.to_string(),
                    display_name: entry.display_name.to_string(),
                    source_slug: entry.source_slug.to_string(),
                    strategy: entry.strategy,
                    support_tier: entry.support_tier,
                },
            );
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
            overall_panel_focus: OverallPanelFocus::default(),
            search_query: String::new(),
            detail_scroll: ScrollOffset::default(),
            overall_incidents_scroll: ScrollOffset::default(),
            overall_degradation_scroll: ScrollOffset::default(),
            overall_maintenance_scroll: ScrollOffset::default(),
            services_expanded: false,
            services_scroll: ScrollOffset::default(),
            loading: true,
            last_refreshed: None,
            last_error: None,
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
                .then_with(|| a.support_tier.sort_rank().cmp(&b.support_tier.sort_rank()))
                .then_with(|| a.provenance.sort_rank().cmp(&b.provenance.sort_rank()))
                .then_with(|| a.display_name.cmp(&b.display_name))
        });
        self.entries = entries;
        self.loading = false;
        self.last_refreshed = Some(Instant::now());
        self.last_error = None;
        self.normalize_overall_panel_focus();
        self.update_filtered();
    }

    pub fn apply_error(&mut self, error: String) {
        self.loading = false;
        self.last_error = Some(error);
    }

    /// `selected` is a display index: 0 = Overall, 1+ = provider at `filtered_entries[selected - 1]`.
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
                        .provider_summary_text()
                        .is_some_and(|summary| summary.to_lowercase().contains(&query))
                    || entry
                        .status_note_text()
                        .is_some_and(|note| note.to_lowercase().contains(&query))
            })
            .map(|(idx, _)| idx)
            .collect();

        self.normalize_overall_panel_focus();

        // If current provider selection is out of range, reset to Overall
        if self.selected > self.filtered_entries.len() {
            self.selected = 0;
        }
        self.list_state.select(Some(self.selected));
    }

    pub fn is_overall_selected(&self) -> bool {
        self.selected == 0
    }

    /// Returns the selected provider, or `None` when Overall (index 0) is selected.
    pub fn current_entry(&self) -> Option<&ProviderStatus> {
        if self.selected == 0 {
            return None;
        }
        self.filtered_entries
            .get(self.selected - 1)
            .and_then(|&idx| self.entries.get(idx))
    }

    pub fn next(&mut self) {
        if self.filtered_entries.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.filtered_entries.len());
        self.list_state.select(Some(self.selected));
        self.detail_scroll.jump_top();
    }

    pub fn prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.list_state.select(Some(self.selected));
        self.detail_scroll.jump_top();
    }

    pub fn select_first(&mut self) {
        self.selected = 0;
        self.list_state.select(Some(0));
        self.detail_scroll.jump_top();
    }

    pub fn select_last(&mut self) {
        self.selected = self.filtered_entries.len(); // last provider (0 = Overall)
        self.list_state.select(Some(self.selected));
        self.detail_scroll.jump_top();
    }

    pub fn page_down(&mut self) {
        self.selected = (self.selected + PAGE_SIZE).min(self.filtered_entries.len());
        self.list_state.select(Some(self.selected));
        self.detail_scroll.jump_top();
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(PAGE_SIZE);
        self.list_state.select(Some(self.selected));
        self.detail_scroll.jump_top();
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn provenance_counts(&self) -> (usize, usize, usize) {
        let mut official = 0;
        let mut fallback = 0;
        let mut unavailable = 0;
        for entry in &self.entries {
            match entry.provenance {
                StatusProvenance::Official => official += 1,
                StatusProvenance::Fallback => fallback += 1,
                StatusProvenance::Unavailable => unavailable += 1,
            }
        }
        (official, fallback, unavailable)
    }

    /// All scheduled maintenances across all providers, as (display_name, maintenance) pairs.
    pub fn all_maintenances(&self) -> Vec<(&str, &ScheduledMaintenance)> {
        self.entries
            .iter()
            .flat_map(|entry| {
                entry
                    .scheduled_maintenances
                    .iter()
                    .map(move |m| (entry.display_name.as_str(), m))
            })
            .collect()
    }

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            StatusFocus::List => StatusFocus::Details,
            StatusFocus::Details => StatusFocus::List,
        };
    }

    fn visible_overall_panels(&self) -> [OverallPanelFocus; 3] {
        [
            OverallPanelFocus::Incidents,
            OverallPanelFocus::Degradation,
            OverallPanelFocus::Maintenance,
        ]
    }

    pub fn maintenance_panel_visible(&self) -> bool {
        !self.all_maintenances().is_empty()
    }

    pub fn normalize_overall_panel_focus(&mut self) {
        if self.overall_panel_focus == OverallPanelFocus::Maintenance
            && !self.maintenance_panel_visible()
        {
            self.overall_panel_focus = OverallPanelFocus::Incidents;
        }
    }

    pub fn select_prev_overall_panel(&mut self) {
        let panels = self.visible_overall_panels();
        let visible_count = if self.maintenance_panel_visible() {
            3
        } else {
            2
        };
        let current = panels[..visible_count]
            .iter()
            .position(|panel| *panel == self.overall_panel_focus)
            .unwrap_or(0);
        let prev = if current == 0 {
            visible_count - 1
        } else {
            current - 1
        };
        self.overall_panel_focus = panels[prev];
    }

    pub fn select_next_overall_panel(&mut self) {
        let panels = self.visible_overall_panels();
        let visible_count = if self.maintenance_panel_visible() {
            3
        } else {
            2
        };
        let current = panels[..visible_count]
            .iter()
            .position(|panel| *panel == self.overall_panel_focus)
            .unwrap_or(0);
        self.overall_panel_focus = panels[(current + 1) % visible_count];
    }

    pub fn active_overall_scroll(&self) -> &ScrollOffset {
        match self.overall_panel_focus {
            OverallPanelFocus::Incidents => &self.overall_incidents_scroll,
            OverallPanelFocus::Degradation => &self.overall_degradation_scroll,
            OverallPanelFocus::Maintenance => &self.overall_maintenance_scroll,
        }
    }

    pub fn scroll_active_overall_panel_up(&self) {
        self.active_overall_scroll().decrement(1);
    }

    pub fn scroll_active_overall_panel_down(&self) {
        self.active_overall_scroll().increment(1);
    }

    pub fn scroll_active_overall_panel_top(&self) {
        self.active_overall_scroll().jump_top();
    }

    pub fn scroll_active_overall_panel_bottom(&self) {
        self.active_overall_scroll().jump_bottom();
    }

    pub fn page_scroll_active_overall_panel_up(&self) {
        self.active_overall_scroll().decrement(PAGE_SIZE as u16);
    }

    pub fn page_scroll_active_overall_panel_down(&self) {
        self.active_overall_scroll().increment(PAGE_SIZE as u16);
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

    #[test]
    fn provenance_counts_tallies_all_entries() {
        let app = StatusApp::new(&AgentsFile {
            schema_version: 1,
            last_scraped: None,
            scrape_source: None,
            agents: HashMap::new(),
        });

        let (official, fallback, unavailable) = app.provenance_counts();
        assert_eq!(official, 0);
        assert_eq!(fallback, 0);
        assert!(unavailable > 0);
    }

    #[test]
    fn overall_panel_focus_skips_maintenance_when_hidden() {
        let mut app = StatusApp::new(&AgentsFile {
            schema_version: 1,
            last_scraped: None,
            scrape_source: None,
            agents: HashMap::new(),
        });

        app.overall_panel_focus = OverallPanelFocus::Incidents;
        app.select_next_overall_panel();
        assert_eq!(app.overall_panel_focus, OverallPanelFocus::Degradation);

        app.select_next_overall_panel();
        assert_eq!(app.overall_panel_focus, OverallPanelFocus::Incidents);
    }

    #[test]
    fn overall_panel_focus_includes_maintenance_when_visible() {
        let mut app = StatusApp::new(&AgentsFile {
            schema_version: 1,
            last_scraped: None,
            scrape_source: None,
            agents: HashMap::new(),
        });

        if let Some(entry) = app.entries.first_mut() {
            entry.scheduled_maintenances.push(ScheduledMaintenance {
                name: "DB maintenance".to_string(),
                status: "scheduled".to_string(),
                impact: "none".to_string(),
                scheduled_for: Some("2026-03-18T12:00:00Z".to_string()),
                scheduled_until: None,
                affected_components: vec!["API".to_string()],
            });
        }

        app.overall_panel_focus = OverallPanelFocus::Incidents;
        app.select_next_overall_panel();
        assert_eq!(app.overall_panel_focus, OverallPanelFocus::Degradation);

        app.select_next_overall_panel();
        assert_eq!(app.overall_panel_focus, OverallPanelFocus::Maintenance);

        app.select_next_overall_panel();
        assert_eq!(app.overall_panel_focus, OverallPanelFocus::Incidents);
    }
}
