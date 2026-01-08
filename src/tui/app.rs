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
}

pub struct App {
    pub providers: Vec<(String, Provider)>,
    pub selected_provider: usize,
    pub selected_model: usize,
    pub focus: Focus,
    pub mode: Mode,
    pub search_query: String,
    filtered_models: Vec<(String, Model)>,
}

impl App {
    pub fn new(providers_map: ProvidersMap) -> Self {
        let mut providers: Vec<(String, Provider)> = providers_map.into_iter().collect();
        providers.sort_by(|a, b| a.0.cmp(&b.0));

        let mut app = Self {
            providers,
            selected_provider: 0,
            selected_model: 0,
            focus: Focus::Providers,
            mode: Mode::Normal,
            search_query: String::new(),
            filtered_models: Vec::new(),
        };

        app.update_filtered_models();
        app
    }

    pub fn update(&mut self, msg: Message) -> bool {
        match msg {
            Message::Quit => return false,
            Message::NextProvider => {
                if self.selected_provider < self.providers.len().saturating_sub(1) {
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
        }
        true
    }

    fn update_filtered_models(&mut self) {
        if let Some((_, provider)) = self.providers.get(self.selected_provider) {
            let query_lower = self.search_query.to_lowercase();

            self.filtered_models = provider
                .models
                .iter()
                .filter(|(id, model)| {
                    if query_lower.is_empty() {
                        return true;
                    }
                    id.to_lowercase().contains(&query_lower)
                        || model.name.to_lowercase().contains(&query_lower)
                })
                .map(|(id, model)| (id.clone(), model.clone()))
                .collect();

            self.filtered_models.sort_by(|a, b| a.0.cmp(&b.0));
        } else {
            self.filtered_models.clear();
        }
    }

    pub fn current_provider(&self) -> Option<&(String, Provider)> {
        self.providers.get(self.selected_provider)
    }

    pub fn current_model(&self) -> Option<&(String, Model)> {
        self.filtered_models.get(self.selected_model)
    }

    pub fn filtered_models(&self) -> &[(String, Model)] {
        &self.filtered_models
    }
}
