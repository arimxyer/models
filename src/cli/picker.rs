//! Shared CLI picker infrastructure.
//!
//! Contains `PickerTerminal`, navigation helpers, table style constants,
//! and the `picker_title` formatter used by all inline CLI pickers.

use std::io;

use anyhow::Result;
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::TableState,
    Terminal, TerminalOptions, Viewport,
};

pub(crate) const VIEWPORT_HEIGHT: u16 = 14;

pub(crate) const HEADER_STYLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub(crate) const ROW_HIGHLIGHT_STYLE: Style =
    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
pub(crate) const HIGHLIGHT_SYMBOL: &str = ">> ";
pub(crate) const ACTIVE_BORDER_STYLE: Style = Style::new().fg(Color::Cyan);
pub(crate) const PREVIEW_BORDER_STYLE: Style = Style::new().fg(Color::DarkGray);

pub(crate) struct PickerTerminal {
    pub terminal: Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
}

impl PickerTerminal {
    pub fn new() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
        let terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(VIEWPORT_HEIGHT),
            },
        )?;
        Ok(Self { terminal })
    }
}

impl Drop for PickerTerminal {
    fn drop(&mut self) {
        let _ = self.terminal.clear();
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = self.terminal.show_cursor();
    }
}

pub(crate) fn nav_next(state: &mut TableState, len: usize) {
    let Some(current) = state.selected() else {
        return;
    };
    let last = len.saturating_sub(1);
    state.select(Some((current + 1).min(last)));
}

pub(crate) fn nav_previous(state: &mut TableState) {
    let Some(current) = state.selected() else {
        return;
    };
    state.select(Some(current.saturating_sub(1)));
}

pub(crate) fn nav_first(state: &mut TableState, len: usize) {
    if len > 0 {
        state.select(Some(0));
    }
}

pub(crate) fn nav_last(state: &mut TableState, len: usize) {
    if len > 0 {
        state.select(Some(len - 1));
    }
}

pub(crate) fn nav_page_down(state: &mut TableState, len: usize, page_size: usize) {
    let Some(current) = state.selected() else {
        return;
    };
    let last = len.saturating_sub(1);
    state.select(Some((current + page_size).min(last)));
}

pub(crate) fn nav_page_up(state: &mut TableState, page_size: usize) {
    let Some(current) = state.selected() else {
        return;
    };
    state.select(Some(current.saturating_sub(page_size)));
}

/// Format a picker title with count, sort indicator, and optional filter query.
///
/// Produces strings like:
/// - `" Models (42 results) | Release desc"`
/// - `" Models (3 / 42 results) | Name asc | / claude"`
pub(crate) fn picker_title(
    name: &str,
    visible: usize,
    total: usize,
    sort_label: &str,
    descending: bool,
    query: &str,
) -> String {
    let results = if visible == total {
        format!("{visible} results")
    } else {
        format!("{visible} / {total} results")
    };
    let direction = if descending { "desc" } else { "asc" };
    if query.is_empty() {
        format!("{name} ({results}) | {sort_label} {direction}")
    } else {
        format!("{name} ({results}) | {sort_label} {direction} | / {query}")
    }
}
