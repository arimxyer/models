use ratatui::widgets::ListState;

use crate::agents::{Agent, AgentEntry, AgentsFile, GitHubData, InstalledInfo, detect_installed};
use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentCategory {
    #[default]
    All,
    Installed,
    Cli,
    Ide,
    OpenSource,
}

impl AgentCategory {
    pub fn label(&self) -> &'static str {
        match self {
            AgentCategory::All => "All",
            AgentCategory::Installed => "Installed",
            AgentCategory::Cli => "CLI Tools",
            AgentCategory::Ide => "IDEs",
            AgentCategory::OpenSource => "Open Source",
        }
    }

    pub fn variants() -> &'static [AgentCategory] {
        &[
            AgentCategory::All,
            AgentCategory::Installed,
            AgentCategory::Cli,
            AgentCategory::Ide,
            AgentCategory::OpenSource,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentFocus {
    #[default]
    Categories,
    Agents,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AgentFilters {
    pub installed_only: bool,
    pub cli_only: bool,
    pub open_source_only: bool,
}

pub struct AgentsApp {
    pub entries: Vec<AgentEntry>,
    pub filtered_entries: Vec<usize>, // indices into entries
    pub selected_category: usize,
    pub selected_agent: usize,
    pub category_list_state: ListState,
    pub agent_list_state: ListState,
    pub focus: AgentFocus,
    pub filters: AgentFilters,
    pub search_query: String,
}

impl AgentsApp {
    pub fn new(agents_file: &AgentsFile, config: &Config) -> Self {
        let mut entries: Vec<AgentEntry> = agents_file
            .agents
            .iter()
            .map(|(id, agent)| {
                let installed = detect_installed(agent);
                AgentEntry {
                    id: id.clone(),
                    agent: agent.clone(),
                    github: GitHubData::default(),
                    installed,
                    tracked: config.is_tracked(id),
                }
            })
            .collect();

        // Sort by name
        entries.sort_by(|a, b| a.agent.name.cmp(&b.agent.name));

        let mut category_list_state = ListState::default();
        category_list_state.select(Some(0));
        let mut agent_list_state = ListState::default();
        agent_list_state.select(Some(0));

        let mut app = Self {
            entries,
            filtered_entries: Vec::new(),
            selected_category: 0,
            selected_agent: 0,
            category_list_state,
            agent_list_state,
            focus: AgentFocus::default(),
            filters: AgentFilters::default(),
            search_query: String::new(),
        };

        app.update_filtered();
        app
    }

    pub fn update_filtered(&mut self) {
        let category = AgentCategory::variants()[self.selected_category];
        let query_lower = self.search_query.to_lowercase();

        self.filtered_entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                // Category filter
                let category_match = match category {
                    AgentCategory::All => true,
                    AgentCategory::Installed => entry.installed.version.is_some(),
                    AgentCategory::Cli => entry.agent.categories.contains(&"cli".to_string()),
                    AgentCategory::Ide => entry.agent.categories.contains(&"ide".to_string()),
                    AgentCategory::OpenSource => entry.agent.open_source,
                };

                // Additional filters
                let filter_match = (!self.filters.installed_only || entry.installed.version.is_some())
                    && (!self.filters.cli_only || entry.agent.categories.contains(&"cli".to_string()))
                    && (!self.filters.open_source_only || entry.agent.open_source);

                // Search filter
                let search_match = query_lower.is_empty()
                    || entry.agent.name.to_lowercase().contains(&query_lower)
                    || entry.id.to_lowercase().contains(&query_lower);

                category_match && filter_match && search_match
            })
            .map(|(i, _)| i)
            .collect();

        // Reset selection if out of bounds
        if self.selected_agent >= self.filtered_entries.len() {
            self.selected_agent = 0;
        }
        self.agent_list_state.select(Some(self.selected_agent));
    }

    pub fn current_entry(&self) -> Option<&AgentEntry> {
        self.filtered_entries
            .get(self.selected_agent)
            .and_then(|&i| self.entries.get(i))
    }

    pub fn category_count(&self, category: AgentCategory) -> usize {
        self.entries
            .iter()
            .filter(|e| match category {
                AgentCategory::All => true,
                AgentCategory::Installed => e.installed.version.is_some(),
                AgentCategory::Cli => e.agent.categories.contains(&"cli".to_string()),
                AgentCategory::Ide => e.agent.categories.contains(&"ide".to_string()),
                AgentCategory::OpenSource => e.agent.open_source,
            })
            .count()
    }

    pub fn next_category(&mut self) {
        let max = AgentCategory::variants().len() - 1;
        if self.selected_category < max {
            self.selected_category += 1;
            self.category_list_state.select(Some(self.selected_category));
            self.selected_agent = 0;
            self.update_filtered();
        }
    }

    pub fn prev_category(&mut self) {
        if self.selected_category > 0 {
            self.selected_category -= 1;
            self.category_list_state.select(Some(self.selected_category));
            self.selected_agent = 0;
            self.update_filtered();
        }
    }

    pub fn next_agent(&mut self) {
        if self.selected_agent < self.filtered_entries.len().saturating_sub(1) {
            self.selected_agent += 1;
            self.agent_list_state.select(Some(self.selected_agent));
        }
    }

    pub fn prev_agent(&mut self) {
        if self.selected_agent > 0 {
            self.selected_agent -= 1;
            self.agent_list_state.select(Some(self.selected_agent));
        }
    }

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            AgentFocus::Categories => AgentFocus::Agents,
            AgentFocus::Agents => AgentFocus::Categories,
        };
    }

    pub fn toggle_installed_filter(&mut self) {
        self.filters.installed_only = !self.filters.installed_only;
        self.selected_agent = 0;
        self.update_filtered();
    }

    pub fn toggle_cli_filter(&mut self) {
        self.filters.cli_only = !self.filters.cli_only;
        self.selected_agent = 0;
        self.update_filtered();
    }

    pub fn toggle_open_source_filter(&mut self) {
        self.filters.open_source_only = !self.filters.open_source_only;
        self.selected_agent = 0;
        self.update_filtered();
    }
}
