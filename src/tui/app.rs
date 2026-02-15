use ratatui::widgets::ListState;

use super::agents_app::AgentsApp;
use super::benchmarks_app::BenchmarksApp;

/// Page size for page up/down navigation
const PAGE_SIZE: usize = 10;
use crate::agents::{AgentsFile, FetchStatus, GitHubData};
use crate::benchmarks::{BenchmarkEntry, BenchmarkStore};
use crate::config::Config;
use crate::data::{Model, Provider, ProvidersMap};
use crate::provider_category::{provider_category, ProviderCategory};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Providers,
    Models,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Default,
    ReleaseDate,
    Cost,
    Context,
}

impl SortOrder {
    pub fn next(self) -> Self {
        match self {
            SortOrder::Default => SortOrder::ReleaseDate,
            SortOrder::ReleaseDate => SortOrder::Cost,
            SortOrder::Cost => SortOrder::Context,
            SortOrder::Context => SortOrder::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Models,
    Agents,
    Benchmarks,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Models => Tab::Agents,
            Tab::Agents => Tab::Benchmarks,
            Tab::Benchmarks => Tab::Models,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::Models => Tab::Benchmarks,
            Tab::Agents => Tab::Models,
            Tab::Benchmarks => Tab::Agents,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Filters {
    pub reasoning: bool,
    pub tools: bool,
    pub open_weights: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderListItem {
    All,
    CategoryHeader(ProviderCategory),
    Provider(usize), // Index into self.providers
}

#[derive(Debug)]
pub enum Message {
    Quit,
    NextProvider,
    PrevProvider,
    NextModel,
    PrevModel,
    SelectFirstProvider,
    SelectLastProvider,
    SelectFirstModel,
    SelectLastModel,
    PageDownProvider,
    PageUpProvider,
    PageDownModel,
    PageUpModel,
    SwitchFocus,
    EnterSearch,
    ExitSearch,
    SearchInput(char),
    SearchBackspace,
    ClearSearch,
    CopyFull,          // Copy provider/model-id
    CopyModelId,       // Copy just model-id
    CopyProviderDoc,   // Copy provider documentation URL
    CopyProviderApi,   // Copy provider API URL
    OpenProviderDoc,   // Open provider documentation URL in browser
    CycleSort,         // Cycle through sort options
    ToggleReasoning,   // Toggle reasoning filter
    ToggleTools,       // Toggle tools filter
    ToggleOpenWeights, // Toggle open weights filter
    ToggleHelp,        // Toggle help popup
    ScrollHelpUp,      // Scroll help popup up
    ScrollHelpDown,    // Scroll help popup down
    NextTab,
    PrevTab,
    // Agents tab messages
    NextAgent,
    PrevAgent,
    PageDownAgent,
    PageUpAgent,
    SwitchAgentFocus,
    ToggleInstalledFilter,
    ToggleCliFilter,
    ToggleOpenSourceFilter,
    OpenAgentRepo,
    OpenAgentDocs,
    CopyAgentName,
    // Picker modal messages
    OpenPicker,
    ClosePicker,
    PickerNext,
    PickerPrev,
    PickerToggle,
    PickerSave,
    // Detail panel scrolling
    ScrollDetailUp,
    ScrollDetailDown,
    PageScrollDetailUp,
    PageScrollDetailDown,
    // Agent sort
    CycleAgentSort,
    // Provider categories
    CycleProviderCategory,
    ToggleGrouping,
    // Benchmarks tab messages
    NextBenchmark,
    PrevBenchmark,
    PageDownBenchmark,
    PageUpBenchmark,
    NextBenchmarkCreator,
    PrevBenchmarkCreator,
    PageDownBenchmarkCreator,
    PageUpBenchmarkCreator,
    SwitchBenchmarkFocus,
    CycleBenchmarkOpenness,
    CycleBenchmarkRegion,
    CycleBenchmarkType,
    CycleBenchmarkSort,
    ToggleBenchmarkSortDir,
    QuickSortIntelligence,
    QuickSortDate,
    QuickSortSpeed,
    CopyBenchmarkName,
    OpenBenchmarkUrl,
    // Async data messages
    GitHubDataReceived(String, GitHubData),
    GitHubFetchFailed(String, String), // (agent_id, error_message)
    // Benchmark data messages
    BenchmarkDataReceived(Vec<BenchmarkEntry>),
    BenchmarkFetchFailed,
}

#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub id: String,
    pub model: Model,
    pub provider_id: String,
}

pub struct App {
    pub providers: Vec<(String, Provider)>,
    /// Index into provider_list_items
    pub selected_provider: usize,
    pub selected_model: usize,
    pub provider_list_state: ListState,
    pub model_list_state: ListState,
    pub focus: Focus,
    pub mode: Mode,
    pub sort_order: SortOrder,
    pub filters: Filters,
    pub search_query: String,
    pub status_message: Option<String>,
    pub show_help: bool,
    pub help_scroll: u16,
    pub current_tab: Tab,
    pub agents_app: Option<AgentsApp>,
    pub config: Config,
    filtered_models: Vec<ModelEntry>,
    /// Agents newly tracked that need GitHub fetches (agent_id, repo)
    pub pending_fetches: Vec<(String, String)>,
    pub provider_category_filter: ProviderCategory,
    pub group_by_category: bool,
    pub provider_list_items: Vec<ProviderListItem>,
    pub benchmark_store: BenchmarkStore,
    pub benchmarks_app: BenchmarksApp,
}

impl App {
    pub fn new(
        providers_map: ProvidersMap,
        agents_file: Option<&AgentsFile>,
        config: Option<Config>,
        benchmark_store: BenchmarkStore,
    ) -> Self {
        let mut providers: Vec<(String, Provider)> = providers_map.into_iter().collect();
        providers.sort_by(|a, b| a.0.cmp(&b.0));

        let mut provider_list_state = ListState::default();
        provider_list_state.select(Some(0));
        let mut model_list_state = ListState::default();
        model_list_state.select(Some(1)); // +1 for header row

        let config = config.unwrap_or_default();
        let agents_app = agents_file.map(|af| AgentsApp::new(af, &config));
        let benchmarks_app = BenchmarksApp::new(&benchmark_store);

        let mut app = Self {
            providers,
            selected_provider: 0, // Start with "All"
            selected_model: 0,
            provider_list_state,
            model_list_state,
            focus: Focus::Providers,
            mode: Mode::Normal,
            sort_order: SortOrder::Default,
            filters: Filters::default(),
            search_query: String::new(),
            status_message: None,
            show_help: false,
            help_scroll: 0,
            current_tab: Tab::default(),
            agents_app,
            config,
            filtered_models: Vec::new(),
            pending_fetches: Vec::new(),
            provider_category_filter: ProviderCategory::All,
            group_by_category: false,
            provider_list_items: Vec::new(),
            benchmark_store,
            benchmarks_app,
        };

        app.update_provider_list();
        app.update_filtered_models();
        app
    }

    pub fn is_all_selected(&self) -> bool {
        matches!(
            self.provider_list_items.get(self.selected_provider),
            Some(ProviderListItem::All)
        )
    }

    /// Returns the number of items in the provider list
    pub fn provider_list_len(&self) -> usize {
        self.provider_list_items.len()
    }

    /// Get the selected provider data (id, Provider) if a provider is selected
    pub fn selected_provider_data(&self) -> Option<&(String, Provider)> {
        match self.provider_list_items.get(self.selected_provider) {
            Some(ProviderListItem::Provider(idx)) => self.providers.get(*idx),
            _ => None,
        }
    }

    /// Rebuild the provider_list_items based on current filter and grouping
    pub fn update_provider_list(&mut self) {
        self.provider_list_items.clear();
        self.provider_list_items.push(ProviderListItem::All);

        if self.group_by_category {
            // Group by category, sorted by display_order then alphabetical within
            let categories = [
                ProviderCategory::Origin,
                ProviderCategory::Cloud,
                ProviderCategory::Inference,
                ProviderCategory::Gateway,
                ProviderCategory::Tool,
            ];

            for cat in &categories {
                if self.provider_category_filter != ProviderCategory::All
                    && self.provider_category_filter != *cat
                {
                    continue;
                }

                let mut indices: Vec<usize> = self
                    .providers
                    .iter()
                    .enumerate()
                    .filter(|(_, (id, _))| provider_category(id) == *cat)
                    .map(|(idx, _)| idx)
                    .collect();

                if indices.is_empty() {
                    continue;
                }

                indices.sort_by(|a, b| self.providers[*a].0.cmp(&self.providers[*b].0));

                self.provider_list_items
                    .push(ProviderListItem::CategoryHeader(*cat));
                for idx in indices {
                    self.provider_list_items
                        .push(ProviderListItem::Provider(idx));
                }
            }
        } else {
            // Flat list, filtered by category
            for (idx, (id, _)) in self.providers.iter().enumerate() {
                if self.provider_category_filter != ProviderCategory::All
                    && provider_category(id) != self.provider_category_filter
                {
                    continue;
                }
                self.provider_list_items
                    .push(ProviderListItem::Provider(idx));
            }
        }
    }

    /// Find the next selectable index (skipping CategoryHeader items)
    fn find_selectable_index(&self, from: usize, forward: bool) -> usize {
        let len = self.provider_list_items.len();
        if len == 0 {
            return 0;
        }

        let mut idx = from;
        loop {
            if !matches!(
                self.provider_list_items.get(idx),
                Some(ProviderListItem::CategoryHeader(_))
            ) {
                return idx;
            }
            if forward {
                if idx >= len - 1 {
                    // Can't go further forward, search backwards from original
                    return self.find_selectable_index(from.saturating_sub(1), false);
                }
                idx += 1;
            } else {
                if idx == 0 {
                    return 0;
                }
                idx -= 1;
            }
        }
    }

    pub fn update(&mut self, msg: Message) -> bool {
        match msg {
            Message::Quit => return false,
            Message::NextProvider => {
                if self.selected_provider < self.provider_list_len().saturating_sub(1) {
                    let next = self.find_selectable_index(self.selected_provider + 1, true);
                    if next != self.selected_provider {
                        self.select_provider_at_index(next);
                    }
                }
            }
            Message::PrevProvider => {
                if self.selected_provider > 0 {
                    let prev = self.find_selectable_index(self.selected_provider - 1, false);
                    if prev != self.selected_provider {
                        self.select_provider_at_index(prev);
                    }
                }
            }
            Message::NextModel => {
                if self.selected_model < self.filtered_models.len().saturating_sub(1) {
                    self.selected_model += 1;
                    self.model_list_state.select(Some(self.selected_model + 1));
                    // +1 for header
                }
            }
            Message::PrevModel => {
                if self.selected_model > 0 {
                    self.selected_model -= 1;
                    self.model_list_state.select(Some(self.selected_model + 1));
                    // +1 for header
                }
            }
            Message::SelectFirstProvider => {
                let first = self.find_selectable_index(0, true);
                if first != self.selected_provider {
                    self.select_provider_at_index(first);
                }
            }
            Message::SelectLastProvider => {
                let last_raw = self.provider_list_len().saturating_sub(1);
                let last = self.find_selectable_index(last_raw, false);
                if last != self.selected_provider {
                    self.select_provider_at_index(last);
                }
            }
            Message::SelectFirstModel => {
                if self.selected_model > 0 {
                    self.selected_model = 0;
                    self.model_list_state.select(Some(self.selected_model + 1));
                }
            }
            Message::SelectLastModel => {
                if self.selected_model < self.filtered_models.len().saturating_sub(1) {
                    self.selected_model = self.filtered_models.len().saturating_sub(1);
                    self.model_list_state.select(Some(self.selected_model + 1));
                }
            }
            Message::PageDownProvider => {
                let last_index = self.provider_list_len().saturating_sub(1);
                let raw = (self.selected_provider + PAGE_SIZE).min(last_index);
                let next = self.find_selectable_index(raw, true);
                if next != self.selected_provider {
                    self.select_provider_at_index(next);
                }
            }
            Message::PageUpProvider => {
                let raw = self.selected_provider.saturating_sub(PAGE_SIZE);
                let next = self.find_selectable_index(raw, false);
                if next != self.selected_provider {
                    self.select_provider_at_index(next);
                }
            }
            Message::PageDownModel => {
                let page_size = PAGE_SIZE;
                let last_index = self.filtered_models.len().saturating_sub(1);
                let next = (self.selected_model + page_size).min(last_index);
                if next != self.selected_model {
                    self.selected_model = next;
                    self.model_list_state.select(Some(self.selected_model + 1));
                }
            }
            Message::PageUpModel => {
                let page_size = PAGE_SIZE;
                let next = self.selected_model.saturating_sub(page_size);
                if next != self.selected_model {
                    self.selected_model = next;
                    self.model_list_state.select(Some(self.selected_model + 1));
                }
            }
            Message::SwitchFocus => {
                self.focus = match self.focus {
                    Focus::Providers => Focus::Models,
                    Focus::Models => Focus::Providers,
                };
            }
            Message::EnterSearch => {
                self.mode = Mode::Search;
            }
            Message::ExitSearch => {
                self.mode = Mode::Normal;
            }
            Message::SearchInput(c) => {
                match self.current_tab {
                    Tab::Models => {
                        self.search_query.push(c);
                        self.selected_model = 0;
                        self.update_filtered_models();
                        self.model_list_state.select(Some(self.selected_model + 1));
                        // +1 for header
                    }
                    Tab::Agents => {
                        if let Some(ref mut agents_app) = self.agents_app {
                            agents_app.search_query.push(c);
                            agents_app.selected_agent = 0;
                            agents_app.update_filtered();
                        }
                    }
                    Tab::Benchmarks => {
                        self.benchmarks_app.search_query.push(c);
                        self.benchmarks_app.selected = 0;
                        self.benchmarks_app.update_filtered(&self.benchmark_store);
                    }
                }
            }
            Message::SearchBackspace => {
                match self.current_tab {
                    Tab::Models => {
                        self.search_query.pop();
                        self.selected_model = 0;
                        self.update_filtered_models();
                        self.model_list_state.select(Some(self.selected_model + 1));
                        // +1 for header
                    }
                    Tab::Agents => {
                        if let Some(ref mut agents_app) = self.agents_app {
                            agents_app.search_query.pop();
                            agents_app.selected_agent = 0;
                            agents_app.update_filtered();
                        }
                    }
                    Tab::Benchmarks => {
                        self.benchmarks_app.search_query.pop();
                        self.benchmarks_app.selected = 0;
                        self.benchmarks_app.update_filtered(&self.benchmark_store);
                    }
                }
            }
            Message::ClearSearch => {
                match self.current_tab {
                    Tab::Models => {
                        self.search_query.clear();
                        self.selected_model = 0;
                        self.update_filtered_models();
                        self.model_list_state.select(Some(self.selected_model + 1));
                        // +1 for header
                    }
                    Tab::Agents => {
                        if let Some(ref mut agents_app) = self.agents_app {
                            agents_app.search_query.clear();
                            agents_app.selected_agent = 0;
                            agents_app.update_filtered();
                        }
                    }
                    Tab::Benchmarks => {
                        self.benchmarks_app.search_query.clear();
                        self.benchmarks_app.selected = 0;
                        self.benchmarks_app.update_filtered(&self.benchmark_store);
                    }
                }
            }
            // Copy and open messages are handled in the main loop
            Message::CopyFull
            | Message::CopyModelId
            | Message::CopyProviderDoc
            | Message::CopyProviderApi
            | Message::OpenProviderDoc => {}
            Message::CycleSort => {
                self.sort_order = self.sort_order.next();
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1)); // +1 for header
            }
            Message::ToggleReasoning => {
                self.filters.reasoning = !self.filters.reasoning;
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1));
            }
            Message::ToggleTools => {
                self.filters.tools = !self.filters.tools;
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1));
            }
            Message::ToggleOpenWeights => {
                self.filters.open_weights = !self.filters.open_weights;
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1));
            }
            Message::ToggleHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.help_scroll = 0; // Reset scroll when opening
                }
            }
            Message::ScrollHelpUp => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            Message::ScrollHelpDown => {
                // Help content lines, cap scroll to prevent scrolling past content
                const HELP_LINES: u16 = 49;
                const MIN_VISIBLE: u16 = 5;
                let max_scroll = HELP_LINES.saturating_sub(MIN_VISIBLE);
                if self.help_scroll < max_scroll {
                    self.help_scroll = self.help_scroll.saturating_add(1);
                }
            }
            Message::NextTab => {
                self.current_tab = self.current_tab.next();
            }
            Message::PrevTab => {
                self.current_tab = self.current_tab.prev();
            }
            Message::NextAgent => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.next_agent();
                }
            }
            Message::PrevAgent => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.prev_agent();
                }
            }
            Message::PageDownAgent => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.page_down(PAGE_SIZE);
                }
            }
            Message::PageUpAgent => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.page_up(PAGE_SIZE);
                }
            }
            Message::SwitchAgentFocus => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.switch_focus();
                }
            }
            Message::ToggleInstalledFilter => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.toggle_installed_filter();
                }
            }
            Message::ToggleCliFilter => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.toggle_cli_filter();
                }
            }
            Message::ToggleOpenSourceFilter => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.toggle_open_source_filter();
                }
            }
            Message::OpenAgentRepo | Message::OpenAgentDocs | Message::CopyAgentName => {
                // Handled in main loop
            }
            Message::OpenPicker => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.open_picker();
                }
            }
            Message::ClosePicker => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.close_picker();
                }
            }
            Message::PickerNext => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.picker_next();
                }
            }
            Message::PickerPrev => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.picker_prev();
                }
            }
            Message::PickerToggle => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.picker_toggle_current();
                }
            }
            Message::PickerSave => {
                if let Some(ref mut agents_app) = self.agents_app {
                    match agents_app.picker_save(&mut self.config) {
                        Ok(newly_tracked) => {
                            if newly_tracked.is_empty() {
                                self.set_status("Tracked agents saved".to_string());
                            } else {
                                self.set_status(format!(
                                    "Tracked agents saved, fetching {} new...",
                                    newly_tracked.len()
                                ));
                                self.pending_fetches = newly_tracked;
                            }
                        }
                        Err(e) => {
                            self.set_status(e);
                        }
                    }
                }
            }
            Message::ScrollDetailUp => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.detail_scroll = agents_app.detail_scroll.saturating_sub(1);
                }
            }
            Message::ScrollDetailDown => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.detail_scroll = agents_app.detail_scroll.saturating_add(1);
                }
            }
            Message::PageScrollDetailUp => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.detail_scroll =
                        agents_app.detail_scroll.saturating_sub(PAGE_SIZE as u16);
                }
            }
            Message::PageScrollDetailDown => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.detail_scroll =
                        agents_app.detail_scroll.saturating_add(PAGE_SIZE as u16);
                }
            }
            Message::CycleAgentSort => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.cycle_sort();
                }
            }
            Message::CycleProviderCategory => {
                self.provider_category_filter = self.provider_category_filter.next();
                self.update_provider_list();
                self.selected_provider = self.find_selectable_index(0, true);
                self.provider_list_state
                    .select(Some(self.selected_provider));
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1));
            }
            Message::ToggleGrouping => {
                self.group_by_category = !self.group_by_category;
                self.update_provider_list();
                self.selected_provider = self.find_selectable_index(0, true);
                self.provider_list_state
                    .select(Some(self.selected_provider));
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1));
            }
            // Benchmarks tab messages
            Message::NextBenchmark => {
                self.benchmarks_app.next();
            }
            Message::PrevBenchmark => {
                self.benchmarks_app.prev();
            }
            Message::PageDownBenchmark => {
                self.benchmarks_app.page_down();
            }
            Message::PageUpBenchmark => {
                self.benchmarks_app.page_up();
            }
            Message::NextBenchmarkCreator => {
                self.benchmarks_app.next_creator();
                self.benchmarks_app.update_filtered(&self.benchmark_store);
            }
            Message::PrevBenchmarkCreator => {
                self.benchmarks_app.prev_creator();
                self.benchmarks_app.update_filtered(&self.benchmark_store);
            }
            Message::PageDownBenchmarkCreator => {
                self.benchmarks_app.page_down_creator();
                self.benchmarks_app.update_filtered(&self.benchmark_store);
            }
            Message::PageUpBenchmarkCreator => {
                self.benchmarks_app.page_up_creator();
                self.benchmarks_app.update_filtered(&self.benchmark_store);
            }
            Message::SwitchBenchmarkFocus => {
                self.benchmarks_app.switch_focus();
            }
            Message::CycleBenchmarkOpenness => {
                self.benchmarks_app
                    .cycle_openness_filter(&self.benchmark_store);
            }
            Message::CycleBenchmarkRegion => {
                self.benchmarks_app
                    .cycle_region_filter(&self.benchmark_store);
            }
            Message::CycleBenchmarkType => {
                self.benchmarks_app.cycle_type_filter(&self.benchmark_store);
            }
            Message::CycleBenchmarkSort => {
                self.benchmarks_app.cycle_sort(&self.benchmark_store);
            }
            Message::ToggleBenchmarkSortDir => {
                self.benchmarks_app
                    .toggle_sort_direction(&self.benchmark_store);
            }
            Message::QuickSortIntelligence => {
                self.benchmarks_app.quick_sort(
                    super::benchmarks_app::BenchmarkSortColumn::Intelligence,
                    &self.benchmark_store,
                );
            }
            Message::QuickSortDate => {
                self.benchmarks_app.quick_sort(
                    super::benchmarks_app::BenchmarkSortColumn::ReleaseDate,
                    &self.benchmark_store,
                );
            }
            Message::QuickSortSpeed => {
                self.benchmarks_app.quick_sort(
                    super::benchmarks_app::BenchmarkSortColumn::Speed,
                    &self.benchmark_store,
                );
            }
            Message::CopyBenchmarkName | Message::OpenBenchmarkUrl => {
                // Handled in main loop
            }
            Message::GitHubDataReceived(agent_id, data) => {
                if let Some(ref mut agents_app) = self.agents_app {
                    if let Some(entry) = agents_app.entries.iter_mut().find(|e| e.id == agent_id) {
                        entry.github = data;
                        entry.fetch_status = FetchStatus::Loaded;
                    }
                    agents_app.apply_sort(); // Re-sort after data arrives

                    // Decrement pending fetches and clear loading flag when all complete
                    agents_app.pending_github_fetches =
                        agents_app.pending_github_fetches.saturating_sub(1);
                    if agents_app.pending_github_fetches == 0 {
                        agents_app.loading_github = false;
                    }
                }
            }
            Message::GitHubFetchFailed(agent_id, error) => {
                if let Some(ref mut agents_app) = self.agents_app {
                    if let Some(entry) = agents_app.entries.iter_mut().find(|e| e.id == agent_id) {
                        entry.fetch_status = FetchStatus::Failed(error);
                    }

                    // Decrement pending fetches and clear loading flag when all complete
                    agents_app.pending_github_fetches =
                        agents_app.pending_github_fetches.saturating_sub(1);
                    if agents_app.pending_github_fetches == 0 {
                        agents_app.loading_github = false;
                    }
                }
            }
            Message::BenchmarkDataReceived(entries) => {
                self.benchmark_store = BenchmarkStore::from_entries(entries);
                self.benchmarks_app.rebuild(&self.benchmark_store);
            }
            Message::BenchmarkFetchFailed => {
                // Silently keep existing data
            }
        }
        true
    }

    fn passes_filters(&self, model: &Model) -> bool {
        if self.filters.reasoning && !model.reasoning {
            return false;
        }
        if self.filters.tools && !model.tool_call {
            return false;
        }
        if self.filters.open_weights && !model.open_weights {
            return false;
        }
        true
    }

    fn update_filtered_models(&mut self) {
        let query_lower = self.search_query.to_lowercase();
        let cat_filter = self.provider_category_filter;

        self.filtered_models = if self.is_all_selected() {
            // Show all models from providers matching the category filter
            let mut entries: Vec<ModelEntry> = self
                .providers
                .iter()
                .filter(|(id, _)| {
                    cat_filter == ProviderCategory::All || provider_category(id) == cat_filter
                })
                .flat_map(|(provider_id, provider)| {
                    provider.models.iter().filter_map(|(model_id, model)| {
                        let search_matches = query_lower.is_empty()
                            || model_id.to_lowercase().contains(&query_lower)
                            || model.name.to_lowercase().contains(&query_lower)
                            || provider_id.to_lowercase().contains(&query_lower);

                        if search_matches && self.passes_filters(model) {
                            Some(ModelEntry {
                                id: model_id.clone(),
                                model: model.clone(),
                                provider_id: provider_id.clone(),
                            })
                        } else {
                            None
                        }
                    })
                })
                .collect();

            self.sort_entries(&mut entries);
            entries
        } else {
            // Show models for selected provider only
            let provider_data = self.selected_provider_data().cloned();
            if let Some((provider_id, provider)) = provider_data {
                let mut entries: Vec<ModelEntry> = provider
                    .models
                    .iter()
                    .filter_map(|(model_id, model)| {
                        let search_matches = query_lower.is_empty()
                            || model_id.to_lowercase().contains(&query_lower)
                            || model.name.to_lowercase().contains(&query_lower);

                        if search_matches && self.passes_filters(model) {
                            Some(ModelEntry {
                                id: model_id.clone(),
                                model: model.clone(),
                                provider_id: provider_id.clone(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                self.sort_entries(&mut entries);
                entries
            } else {
                Vec::new()
            }
        };
    }

    fn sort_entries(&self, entries: &mut [ModelEntry]) {
        match self.sort_order {
            SortOrder::Default => {
                // Sort by provider, then model id (alphabetical)
                entries.sort_by(|a, b| a.provider_id.cmp(&b.provider_id).then(a.id.cmp(&b.id)));
            }
            SortOrder::ReleaseDate => {
                // Sort by release date descending (newest first), None values last
                entries.sort_by(
                    |a, b| match (&b.model.release_date, &a.model.release_date) {
                        (Some(b_date), Some(a_date)) => b_date.cmp(a_date),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.id.cmp(&b.id),
                    },
                );
            }
            SortOrder::Cost => {
                // Sort by input cost ascending (cheapest first), None values last
                entries.sort_by(|a, b| {
                    let a_cost = a.model.cost.as_ref().and_then(|c| c.input);
                    let b_cost = b.model.cost.as_ref().and_then(|c| c.input);
                    match (a_cost, b_cost) {
                        (Some(a_val), Some(b_val)) => a_val
                            .partial_cmp(&b_val)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.id.cmp(&b.id),
                    }
                });
            }
            SortOrder::Context => {
                // Sort by context size descending (largest first), None values last
                entries.sort_by(|a, b| {
                    let a_ctx = a.model.limit.as_ref().and_then(|l| l.context);
                    let b_ctx = b.model.limit.as_ref().and_then(|l| l.context);
                    match (b_ctx, a_ctx) {
                        (Some(b_val), Some(a_val)) => b_val.cmp(&a_val),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.id.cmp(&b.id),
                    }
                });
            }
        }
    }

    fn select_provider_at_index(&mut self, index: usize) {
        self.selected_provider = index;
        self.selected_model = 0;
        self.provider_list_state
            .select(Some(self.selected_provider));
        self.update_filtered_models();
        self.model_list_state.select(Some(self.selected_model + 1));
        // +1 for header
    }

    pub fn current_model(&self) -> Option<&ModelEntry> {
        self.filtered_models.get(self.selected_model)
    }

    pub fn filtered_models(&self) -> &[ModelEntry] {
        &self.filtered_models
    }

    pub fn total_model_count(&self) -> usize {
        self.providers.iter().map(|(_, p)| p.models.len()).sum()
    }

    /// Model count respecting the active category filter
    pub fn filtered_model_count(&self) -> usize {
        if self.provider_category_filter == ProviderCategory::All {
            self.total_model_count()
        } else {
            self.providers
                .iter()
                .filter(|(id, _)| provider_category(id) == self.provider_category_filter)
                .map(|(_, p)| p.models.len())
                .sum()
        }
    }

    /// Get the full provider/model-id string for copying
    pub fn get_copy_full(&self) -> Option<String> {
        self.current_model()
            .map(|entry| format!("{}/{}", entry.provider_id, entry.id))
    }

    /// Get just the model-id for copying
    pub fn get_copy_model_id(&self) -> Option<String> {
        self.current_model().map(|entry| entry.id.clone())
    }

    /// Get the provider documentation URL for copying
    pub fn get_provider_doc(&self) -> Option<String> {
        self.current_model().and_then(|entry| {
            self.providers
                .iter()
                .find(|(id, _)| id == &entry.provider_id)
                .and_then(|(_, provider)| provider.doc.clone())
        })
    }

    /// Get the provider API URL for copying
    pub fn get_provider_api(&self) -> Option<String> {
        self.current_model().and_then(|entry| {
            self.providers
                .iter()
                .find(|(id, _)| id == &entry.provider_id)
                .and_then(|(_, provider)| provider.api.clone())
        })
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }
}
