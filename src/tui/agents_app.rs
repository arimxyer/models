use std::collections::HashMap;

use ratatui::widgets::ListState;

use crate::agents::{detect_installed, AgentEntry, AgentsFile, FetchStatus, GitHubData};
use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentSortOrder {
    #[default]
    Name,
    Updated,
    Stars,
    Status,
}

impl AgentSortOrder {
    pub fn next(self) -> Self {
        match self {
            AgentSortOrder::Name => AgentSortOrder::Updated,
            AgentSortOrder::Updated => AgentSortOrder::Stars,
            AgentSortOrder::Stars => AgentSortOrder::Status,
            AgentSortOrder::Status => AgentSortOrder::Name,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AgentSortOrder::Name => "name",
            AgentSortOrder::Updated => "updated",
            AgentSortOrder::Stars => "stars",
            AgentSortOrder::Status => "status",
        }
    }
}

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
    List,
    Details,
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
    pub agent_list_state: ListState,
    pub focus: AgentFocus,
    pub filters: AgentFilters,
    pub search_query: String,
    pub sort_order: AgentSortOrder,
    // Picker modal state
    pub show_picker: bool,
    pub picker_selected: usize,
    pub picker_changes: HashMap<String, bool>, // agent_id -> new tracked state
    // Detail panel scroll
    pub detail_scroll: u16,
    // Loading state for async GitHub fetches
    pub loading_github: bool,
    pub pending_github_fetches: usize,
}

impl AgentsApp {
    pub fn new(agents_file: &AgentsFile, config: &Config) -> Self {
        use std::sync::mpsc;
        use std::thread;

        // Collect agents that need version detection (tracked only)
        let agents_to_detect: Vec<_> = agents_file
            .agents
            .iter()
            .filter(|(id, _)| config.is_tracked(id))
            .map(|(id, agent)| (id.clone(), agent.clone()))
            .collect();

        // Run version detection in parallel using threads
        let (tx, rx) = mpsc::channel();
        for (id, agent) in agents_to_detect {
            let tx = tx.clone();
            thread::spawn(move || {
                let installed = detect_installed(&agent);
                let _ = tx.send((id, installed));
            });
        }
        drop(tx); // Close sender so rx.iter() terminates

        // Collect results
        let detected: std::collections::HashMap<String, _> = rx.iter().collect();

        // Build entries with detected versions
        let mut entries: Vec<AgentEntry> = agents_file
            .agents
            .iter()
            .map(|(id, agent)| {
                let tracked = config.is_tracked(id);
                let installed = detected.get(id).cloned().unwrap_or_default();
                AgentEntry {
                    id: id.clone(),
                    agent: agent.clone(),
                    github: GitHubData::default(),
                    installed,
                    tracked,
                    fetch_status: if tracked {
                        FetchStatus::Loading
                    } else {
                        FetchStatus::NotStarted
                    },
                }
            })
            .collect();

        // Add custom agents from config
        for custom in &config.agents.custom {
            let id = custom.name.to_lowercase().replace(' ', "-");
            // Skip if already exists (curated agent takes precedence)
            if entries.iter().any(|e| e.id == id) {
                continue;
            }
            let agent = custom.to_agent();
            let installed = detect_installed(&agent);
            entries.push(AgentEntry {
                id,
                agent,
                github: GitHubData::default(),
                installed,
                tracked: true, // Custom agents are tracked by default
                fetch_status: FetchStatus::Loading, // Tracked, so GitHub fetch will be spawned
            });
        }

        // Sort by name
        entries.sort_by(|a, b| a.agent.name.cmp(&b.agent.name));

        let mut agent_list_state = ListState::default();
        agent_list_state.select(Some(0));

        // Only count tracked agents for pending fetches
        let pending_fetches = entries.iter().filter(|e| e.tracked).count();
        let mut app = Self {
            entries,
            filtered_entries: Vec::new(),
            selected_category: 0,
            selected_agent: 0,
            agent_list_state,
            focus: AgentFocus::default(),
            filters: AgentFilters::default(),
            search_query: String::new(),
            sort_order: AgentSortOrder::default(),
            show_picker: false,
            picker_selected: 0,
            picker_changes: HashMap::new(),
            detail_scroll: 0,
            loading_github: true,
            pending_github_fetches: pending_fetches,
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

                // Tracked agents only (primary filter)
                if !entry.tracked {
                    return false;
                }

                // Additional filters
                let filter_match = (!self.filters.installed_only
                    || entry.installed.version.is_some())
                    && (!self.filters.cli_only
                        || entry.agent.categories.contains(&"cli".to_string()))
                    && (!self.filters.open_source_only || entry.agent.open_source);

                // Search filter
                let search_match = query_lower.is_empty()
                    || entry.agent.name.to_lowercase().contains(&query_lower)
                    || entry.id.to_lowercase().contains(&query_lower);

                category_match && filter_match && search_match
            })
            .map(|(i, _)| i)
            .collect();

        self.apply_sort();

        // Reset selection if out of bounds
        if self.selected_agent >= self.filtered_entries.len() {
            self.selected_agent = 0;
        }
        self.agent_list_state.select(Some(self.selected_agent));
    }

    pub fn cycle_sort(&mut self) {
        self.sort_order = self.sort_order.next();
        self.apply_sort();
    }

    pub fn apply_sort(&mut self) {
        let entries = &self.entries;
        self.filtered_entries.sort_by(|&a, &b| {
            let ea = &entries[a];
            let eb = &entries[b];
            match self.sort_order {
                AgentSortOrder::Name => ea.agent.name.cmp(&eb.agent.name),
                AgentSortOrder::Updated => {
                    let da = ea
                        .github
                        .latest_release()
                        .and_then(|r| r.date.as_deref())
                        .unwrap_or("");
                    let db = eb
                        .github
                        .latest_release()
                        .and_then(|r| r.date.as_deref())
                        .unwrap_or("");
                    db.cmp(da) // Descending (newest first)
                }
                AgentSortOrder::Stars => {
                    let sa = ea.github.stars.unwrap_or(0);
                    let sb = eb.github.stars.unwrap_or(0);
                    sb.cmp(&sa) // Descending (most stars first)
                }
                AgentSortOrder::Status => {
                    let status_a = if ea.update_available() {
                        0
                    } else if ea.installed.version.is_some() {
                        1
                    } else {
                        2
                    };
                    let status_b = if eb.update_available() {
                        0
                    } else if eb.installed.version.is_some() {
                        1
                    } else {
                        2
                    };
                    status_a.cmp(&status_b)
                }
            }
        });
    }

    pub fn current_entry(&self) -> Option<&AgentEntry> {
        self.filtered_entries
            .get(self.selected_agent)
            .and_then(|&i| self.entries.get(i))
    }

    pub fn next_agent(&mut self) {
        if self.selected_agent < self.filtered_entries.len().saturating_sub(1) {
            self.selected_agent += 1;
            self.agent_list_state.select(Some(self.selected_agent));
            self.detail_scroll = 0;
        }
    }

    pub fn prev_agent(&mut self) {
        if self.selected_agent > 0 {
            self.selected_agent -= 1;
            self.agent_list_state.select(Some(self.selected_agent));
            self.detail_scroll = 0;
        }
    }

    pub fn page_down(&mut self, page_size: usize) {
        let last_index = self.filtered_entries.len().saturating_sub(1);
        self.selected_agent = (self.selected_agent + page_size).min(last_index);
        self.agent_list_state.select(Some(self.selected_agent));
        self.detail_scroll = 0;
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected_agent = self.selected_agent.saturating_sub(page_size);
        self.agent_list_state.select(Some(self.selected_agent));
        self.detail_scroll = 0;
    }

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            AgentFocus::List => AgentFocus::Details,
            AgentFocus::Details => AgentFocus::List,
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

    // Picker modal methods
    pub fn open_picker(&mut self) {
        self.show_picker = true;
        self.picker_selected = 0;
        self.picker_changes.clear();
        // Initialize with current tracked states
        for entry in &self.entries {
            self.picker_changes.insert(entry.id.clone(), entry.tracked);
        }
    }

    pub fn close_picker(&mut self) {
        self.show_picker = false;
        self.picker_changes.clear();
    }

    pub fn picker_toggle_current(&mut self) {
        if let Some(entry) = self.entries.get(self.picker_selected) {
            let current = self
                .picker_changes
                .get(&entry.id)
                .copied()
                .unwrap_or(entry.tracked);
            self.picker_changes.insert(entry.id.clone(), !current);
        }
    }

    pub fn picker_next(&mut self) {
        if self.picker_selected < self.entries.len().saturating_sub(1) {
            self.picker_selected += 1;
        }
    }

    pub fn picker_prev(&mut self) {
        if self.picker_selected > 0 {
            self.picker_selected -= 1;
        }
    }

    /// Save picker changes and return list of newly tracked agents (id, repo) for fetching
    pub fn picker_save(&mut self, config: &mut Config) -> Result<Vec<(String, String)>, String> {
        let mut newly_tracked = Vec::new();

        for (agent_id, tracked) in &self.picker_changes {
            config.set_tracked(agent_id, *tracked);
            if let Some(entry) = self.entries.iter_mut().find(|e| e.id == *agent_id) {
                // Track if this is a newly tracked agent (was not tracked, now is)
                if *tracked && !entry.tracked {
                    newly_tracked.push((agent_id.clone(), entry.agent.repo.clone()));
                    entry.fetch_status = FetchStatus::Loading;
                }
                entry.tracked = *tracked;
            }
        }

        if let Err(e) = config.save() {
            self.close_picker();
            return Err(format!("Failed to save config: {}", e));
        }

        self.close_picker();
        self.update_filtered(); // Re-filter in case tracked_only is active
        Ok(newly_tracked)
    }

    /// Format active filters for display in block title
    pub fn format_active_filters(&self) -> String {
        let mut active = Vec::new();

        // Category (if not "All")
        let category = AgentCategory::variants()[self.selected_category];
        if category != AgentCategory::All {
            active.push(category.label().to_lowercase());
        }

        // Additional filters
        if self.filters.installed_only {
            active.push("installed".to_string());
        }
        if self.filters.cli_only {
            active.push("cli".to_string());
        }
        if self.filters.open_source_only {
            active.push("open".to_string());
        }

        if !self.search_query.is_empty() {
            active.push("search".to_string());
        }

        active.join(", ")
    }
}
