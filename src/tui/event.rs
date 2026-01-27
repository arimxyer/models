use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use super::app::{App, Focus, Message, Mode};

pub fn handle_events(app: &App) -> Result<Option<Message>> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(None);
            }

            // When help is showing, handle scroll and dismiss keys
            if app.show_help {
                let msg = match key.code {
                    KeyCode::Char('?') | KeyCode::Esc => Some(Message::ToggleHelp),
                    KeyCode::Char('j') | KeyCode::Down => Some(Message::ScrollHelpDown),
                    KeyCode::Char('k') | KeyCode::Up => Some(Message::ScrollHelpUp),
                    _ => None,
                };
                return Ok(msg);
            }

            let msg = match app.mode {
                Mode::Normal => handle_normal_mode(app, key.code, key.modifiers),
                Mode::Search => handle_search_mode(key.code),
            };

            return Ok(msg);
        }
    }

    Ok(None)
}

fn handle_normal_mode(app: &App, code: KeyCode, modifiers: KeyModifiers) -> Option<Message> {
    // Check for picker mode (intercepts before normal handling)
    if app.current_tab == super::app::Tab::Agents {
        if let Some(ref agents_app) = app.agents_app {
            if agents_app.show_picker {
                return handle_picker_keys(code);
            }
        }
    }

    // Global keys (work on any tab)
    match code {
        KeyCode::Char('q') => return Some(Message::Quit),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Message::Quit)
        }
        KeyCode::Char('[') => return Some(Message::PrevTab),
        KeyCode::Char(']') => return Some(Message::NextTab),
        KeyCode::Char('?') => return Some(Message::ToggleHelp),
        _ => {}
    }

    // Tab-specific keys
    match app.current_tab {
        super::app::Tab::Models => handle_models_keys(app, code, modifiers),
        super::app::Tab::Agents => handle_agents_keys(app, code, modifiers),
    }
}

fn handle_models_keys(app: &App, code: KeyCode, modifiers: KeyModifiers) -> Option<Message> {
    match code {
        // Copy shortcuts
        KeyCode::Char('c') => Some(Message::CopyFull),
        KeyCode::Char('C') => Some(Message::CopyModelId),
        KeyCode::Char('D') => Some(Message::CopyProviderDoc),
        KeyCode::Char('A') => Some(Message::CopyProviderApi),
        KeyCode::Char('o') => Some(Message::OpenProviderDoc),

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::Providers => Some(Message::NextProvider),
            Focus::Models => Some(Message::NextModel),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::Providers => Some(Message::PrevProvider),
            Focus::Models => Some(Message::PrevModel),
        },
        KeyCode::Char('g') => match app.focus {
            Focus::Providers => Some(Message::SelectFirstProvider),
            Focus::Models => Some(Message::SelectFirstModel),
        },
        KeyCode::Char('G') => match app.focus {
            Focus::Providers => Some(Message::SelectLastProvider),
            Focus::Models => Some(Message::SelectLastModel),
        },
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => match app.focus {
            Focus::Providers => Some(Message::PageDownProvider),
            Focus::Models => Some(Message::PageDownModel),
        },
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => match app.focus {
            Focus::Providers => Some(Message::PageUpProvider),
            Focus::Models => Some(Message::PageUpModel),
        },
        KeyCode::PageDown => match app.focus {
            Focus::Providers => Some(Message::PageDownProvider),
            Focus::Models => Some(Message::PageDownModel),
        },
        KeyCode::PageUp => match app.focus {
            Focus::Providers => Some(Message::PageUpProvider),
            Focus::Models => Some(Message::PageUpModel),
        },
        KeyCode::Char('h') | KeyCode::Left => Some(Message::SwitchFocus),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::SwitchFocus),
        KeyCode::Tab | KeyCode::BackTab => Some(Message::SwitchFocus),

        // Search
        KeyCode::Char('/') => Some(Message::EnterSearch),
        KeyCode::Esc => Some(Message::ClearSearch),

        // Sort
        KeyCode::Char('s') => Some(Message::CycleSort),

        // Filters
        KeyCode::Char('1') => Some(Message::ToggleReasoning),
        KeyCode::Char('2') => Some(Message::ToggleTools),
        KeyCode::Char('3') => Some(Message::ToggleOpenWeights),

        _ => None,
    }
}

fn handle_agents_keys(app: &App, code: KeyCode, _modifiers: KeyModifiers) -> Option<Message> {
    use super::agents_app::AgentFocus;

    let focus = app
        .agents_app
        .as_ref()
        .map(|a| a.focus)
        .unwrap_or(AgentFocus::List);

    match code {
        // Navigation - works in List focus
        KeyCode::Char('j') | KeyCode::Down => {
            if focus == AgentFocus::List {
                Some(Message::NextAgent)
            } else {
                Some(Message::ScrollDetailDown)
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if focus == AgentFocus::List {
                Some(Message::PrevAgent)
            } else {
                Some(Message::ScrollDetailUp)
            }
        }
        KeyCode::Char('h') | KeyCode::Left => Some(Message::SwitchAgentFocus),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::SwitchAgentFocus),
        KeyCode::Tab | KeyCode::BackTab => Some(Message::SwitchAgentFocus),

        // Actions
        KeyCode::Char('o') => Some(Message::OpenAgentDocs),
        KeyCode::Char('r') => Some(Message::OpenAgentRepo),
        KeyCode::Char('c') => Some(Message::CopyAgentName),
        KeyCode::Char('u') => Some(Message::CopyUpdateCommand),

        // Filters
        KeyCode::Char('1') => Some(Message::ToggleInstalledFilter),
        KeyCode::Char('2') => Some(Message::ToggleCliFilter),
        KeyCode::Char('3') => Some(Message::ToggleOpenSourceFilter),

        // Picker
        KeyCode::Char('a') => Some(Message::OpenPicker),

        // Search
        KeyCode::Char('/') => Some(Message::EnterSearch),

        // Sort
        KeyCode::Char('s') => Some(Message::CycleAgentSort),

        _ => None,
    }
}

fn handle_picker_keys(code: KeyCode) -> Option<Message> {
    match code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::PickerNext),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::PickerPrev),
        KeyCode::Char(' ') => Some(Message::PickerToggle),
        KeyCode::Enter => Some(Message::PickerSave),
        KeyCode::Esc => Some(Message::ClosePicker),
        _ => None,
    }
}

fn handle_search_mode(code: KeyCode) -> Option<Message> {
    match code {
        KeyCode::Esc | KeyCode::Enter => Some(Message::ExitSearch),
        KeyCode::Backspace => Some(Message::SearchBackspace),
        KeyCode::Char(c) => Some(Message::SearchInput(c)),
        _ => None,
    }
}
