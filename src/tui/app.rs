use ratatui::widgets::ListState;

use crate::data::{Model, Provider, ProvidersMap};

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

#[derive(Debug, Clone, Copy, Default)]
pub struct Filters {
    pub reasoning: bool,
    pub tools: bool,
    pub open_weights: bool,
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
    CycleSort,         // Cycle through sort options
    ToggleReasoning,   // Toggle reasoning filter
    ToggleTools,       // Toggle tools filter
    ToggleOpenWeights, // Toggle open weights filter
    ToggleHelp,        // Toggle help popup
    ScrollHelpUp,      // Scroll help popup up
    ScrollHelpDown,    // Scroll help popup down
}

#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub id: String,
    pub model: Model,
    pub provider_id: String,
}

pub struct App {
    pub providers: Vec<(String, Provider)>,
    /// 0 = "All", 1+ = actual provider index + 1
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
    filtered_models: Vec<ModelEntry>,
}

impl App {
    pub fn new(providers_map: ProvidersMap) -> Self {
        let mut providers: Vec<(String, Provider)> = providers_map.into_iter().collect();
        providers.sort_by(|a, b| a.0.cmp(&b.0));

        let mut provider_list_state = ListState::default();
        provider_list_state.select(Some(0));
        let mut model_list_state = ListState::default();
        model_list_state.select(Some(1)); // +1 for header row

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
            filtered_models: Vec::new(),
        };

        app.update_filtered_models();
        app
    }

    pub fn is_all_selected(&self) -> bool {
        self.selected_provider == 0
    }

    /// Returns the number of items in the provider list (including "All")
    pub fn provider_list_len(&self) -> usize {
        self.providers.len() + 1
    }

    pub fn update(&mut self, msg: Message) -> bool {
        match msg {
            Message::Quit => return false,
            Message::NextProvider => {
                if self.selected_provider < self.provider_list_len().saturating_sub(1) {
                    self.select_provider_at_index(self.selected_provider + 1);
                }
            }
            Message::PrevProvider => {
                if self.selected_provider > 0 {
                    self.select_provider_at_index(self.selected_provider - 1);
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
                if self.selected_provider > 0 {
                    self.select_provider_at_index(0);
                }
            }
            Message::SelectLastProvider => {
                let last_index = self.provider_list_len().saturating_sub(1);
                if self.selected_provider < last_index {
                    self.select_provider_at_index(last_index);
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
                let page_size = 10;
                let last_index = self.provider_list_len().saturating_sub(1);
                let next = (self.selected_provider + page_size).min(last_index);
                if next != self.selected_provider {
                    self.select_provider_at_index(next);
                }
            }
            Message::PageUpProvider => {
                let page_size = 10;
                let next = self.selected_provider.saturating_sub(page_size);
                if next != self.selected_provider {
                    self.select_provider_at_index(next);
                }
            }
            Message::PageDownModel => {
                let page_size = 10;
                let last_index = self.filtered_models.len().saturating_sub(1);
                let next = (self.selected_model + page_size).min(last_index);
                if next != self.selected_model {
                    self.selected_model = next;
                    self.model_list_state.select(Some(self.selected_model + 1));
                }
            }
            Message::PageUpModel => {
                let page_size = 10;
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
                self.search_query.push(c);
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1)); // +1 for header
            }
            Message::SearchBackspace => {
                self.search_query.pop();
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1)); // +1 for header
            }
            Message::ClearSearch => {
                self.search_query.clear();
                self.selected_model = 0;
                self.update_filtered_models();
                self.model_list_state.select(Some(self.selected_model + 1)); // +1 for header
            }
            // Copy messages are handled in the main loop
            Message::CopyFull | Message::CopyModelId => {}
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
                // Help content is 27 lines, cap scroll to prevent scrolling past content
                const HELP_LINES: u16 = 27;
                const MIN_VISIBLE: u16 = 5;
                let max_scroll = HELP_LINES.saturating_sub(MIN_VISIBLE);
                if self.help_scroll < max_scroll {
                    self.help_scroll = self.help_scroll.saturating_add(1);
                }
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

        self.filtered_models = if self.is_all_selected() {
            // Show all models from all providers
            let mut entries: Vec<ModelEntry> = self
                .providers
                .iter()
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
            let provider_idx = self.selected_provider - 1;
            if let Some((provider_id, provider)) = self.providers.get(provider_idx) {
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

    /// Get the full provider/model-id string for copying
    pub fn get_copy_full(&self) -> Option<String> {
        self.current_model()
            .map(|entry| format!("{}/{}", entry.provider_id, entry.id))
    }

    /// Get just the model-id for copying
    pub fn get_copy_model_id(&self) -> Option<String> {
        self.current_model().map(|entry| entry.id.clone())
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }
}
