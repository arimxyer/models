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

#[derive(Debug)]
pub enum Message {
    Quit,
    NextProvider,
    PrevProvider,
    NextModel,
    PrevModel,
    SwitchFocus,
    EnterSearch,
    ExitSearch,
    SearchInput(char),
    SearchBackspace,
    ClearSearch,
    CopyFull,    // Copy provider/model-id
    CopyModelId, // Copy just model-id
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
    pub focus: Focus,
    pub mode: Mode,
    pub search_query: String,
    pub status_message: Option<String>,
    filtered_models: Vec<ModelEntry>,
}

impl App {
    pub fn new(providers_map: ProvidersMap) -> Self {
        let mut providers: Vec<(String, Provider)> = providers_map.into_iter().collect();
        providers.sort_by(|a, b| a.0.cmp(&b.0));

        let mut app = Self {
            providers,
            selected_provider: 0, // Start with "All"
            selected_model: 0,
            focus: Focus::Providers,
            mode: Mode::Normal,
            search_query: String::new(),
            status_message: None,
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
                    self.selected_provider += 1;
                    self.selected_model = 0;
                    self.update_filtered_models();
                }
            }
            Message::PrevProvider => {
                if self.selected_provider > 0 {
                    self.selected_provider -= 1;
                    self.selected_model = 0;
                    self.update_filtered_models();
                }
            }
            Message::NextModel => {
                if self.selected_model < self.filtered_models.len().saturating_sub(1) {
                    self.selected_model += 1;
                }
            }
            Message::PrevModel => {
                if self.selected_model > 0 {
                    self.selected_model -= 1;
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
            }
            Message::SearchBackspace => {
                self.search_query.pop();
                self.selected_model = 0;
                self.update_filtered_models();
            }
            Message::ClearSearch => {
                self.search_query.clear();
                self.selected_model = 0;
                self.update_filtered_models();
            }
            // Copy messages are handled in the main loop
            Message::CopyFull | Message::CopyModelId => {}
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
                        let matches = query_lower.is_empty()
                            || model_id.to_lowercase().contains(&query_lower)
                            || model.name.to_lowercase().contains(&query_lower)
                            || provider_id.to_lowercase().contains(&query_lower);

                        if matches {
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

            entries.sort_by(|a, b| a.provider_id.cmp(&b.provider_id).then(a.id.cmp(&b.id)));
            entries
        } else {
            // Show models for selected provider only
            let provider_idx = self.selected_provider - 1;
            if let Some((provider_id, provider)) = self.providers.get(provider_idx) {
                let mut entries: Vec<ModelEntry> = provider
                    .models
                    .iter()
                    .filter_map(|(model_id, model)| {
                        let matches = query_lower.is_empty()
                            || model_id.to_lowercase().contains(&query_lower)
                            || model.name.to_lowercase().contains(&query_lower);

                        if matches {
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

                entries.sort_by(|a, b| a.id.cmp(&b.id));
                entries
            } else {
                Vec::new()
            }
        };
    }

    pub fn current_provider(&self) -> Option<&(String, Provider)> {
        if self.is_all_selected() {
            None
        } else {
            self.providers.get(self.selected_provider - 1)
        }
    }

    pub fn current_model(&self) -> Option<&ModelEntry> {
        self.filtered_models.get(self.selected_model)
    }

    pub fn filtered_models(&self) -> &[ModelEntry] {
        &self.filtered_models
    }

    pub fn total_model_count(&self) -> usize {
        self.providers
            .iter()
            .map(|(_, p)| p.models.len())
            .sum()
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
