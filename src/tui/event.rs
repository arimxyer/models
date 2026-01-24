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
    match code {
        KeyCode::Char('q') => Some(Message::Quit),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Some(Message::Quit),

        // Copy shortcuts
        KeyCode::Char('c') => Some(Message::CopyFull), // c = copy provider/model-id
        KeyCode::Char('C') => Some(Message::CopyModelId), // C = copy model-id only

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

fn handle_search_mode(code: KeyCode) -> Option<Message> {
    match code {
        KeyCode::Esc | KeyCode::Enter => Some(Message::ExitSearch),
        KeyCode::Backspace => Some(Message::SearchBackspace),
        KeyCode::Char(c) => Some(Message::SearchInput(c)),
        _ => None,
    }
}
