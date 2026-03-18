use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use super::app::{App, Mode, Tab};
use crate::status::ProviderHealth;

/// Border style: Cyan when focused, DarkGray when not.
pub(super) fn focus_border(focused: bool) -> Style {
    Style::default().fg(if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    })
}

/// Caret prefix for list items: "> " when focused, "  " when not.
pub(super) fn caret(focused: bool) -> &'static str {
    if focused {
        "> "
    } else {
        "  "
    }
}

/// Selection style: Yellow + BOLD when selected, default otherwise.
pub(super) fn selection_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

/// Build a help-popup line: 16-char padded key in Yellow + description.
fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:<14}", key), Style::default().fg(Color::Yellow)),
        Span::raw(desc),
    ])
}

/// Render a vertical scrollbar if content exceeds viewport.
/// `inside_block`: true when the area has `Borders::ALL` (applies vertical margin).
pub(super) fn render_scrollbar(
    f: &mut Frame,
    area: Rect,
    content_len: usize,
    position: usize,
    viewport_len: usize,
    inside_block: bool,
) {
    if content_len <= viewport_len {
        return;
    }
    let scroll_area = if inside_block {
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        })
    } else {
        area
    };
    let mut state = ScrollbarState::new(content_len)
        .position(position)
        .viewport_content_length(viewport_len);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        scroll_area,
        &mut state,
    );
}

pub(super) fn status_health_style(health: ProviderHealth) -> Style {
    match health {
        ProviderHealth::Operational => Style::default().fg(Color::Green),
        ProviderHealth::Degraded => Style::default().fg(Color::Yellow),
        ProviderHealth::Outage => Style::default().fg(Color::Red),
        ProviderHealth::Maintenance => Style::default().fg(Color::Blue),
        ProviderHealth::Unknown => Style::default().fg(Color::DarkGray),
    }
}

pub(super) fn status_health_icon(health: ProviderHealth) -> &'static str {
    match health {
        ProviderHealth::Operational => "●",
        ProviderHealth::Degraded => "◐",
        ProviderHealth::Outage => "✗",
        ProviderHealth::Maintenance => "◆",
        ProviderHealth::Unknown => "?",
    }
}

/// Calculate visible height for detail panel (area height minus borders)
pub(super) fn detail_visible_height(area: Rect) -> u16 {
    area.height.saturating_sub(2) // 2 for top and bottom borders
}

/// Create a centered rect using fixed width and height
pub(super) fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

/// Create a centered rect using percentage of the available area
pub(super) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Footer/search
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);

    match app.current_tab {
        Tab::Models => {
            super::ui_models::draw_main(f, chunks[1], app);
        }
        Tab::Agents => {
            super::ui_agents::draw_agents_main(f, chunks[1], app);
        }
        Tab::Benchmarks => {
            super::ui_benchmarks::draw_benchmarks_main(f, chunks[1], app);
        }
        Tab::Status => {
            super::ui_status::draw_status_main(f, chunks[1], app);
        }
    }

    draw_footer(f, chunks[2], app);

    // Draw help popup on top if visible
    if app.show_help {
        draw_help_popup(f, app.help_scroll, app.current_tab);
    }

    // Draw picker modal on top if visible (agents tab only)
    if app.current_tab == Tab::Agents {
        if let Some(agents_app) = &app.agents_app {
            if agents_app.show_picker {
                super::ui_agents::draw_picker_modal(f, app);
            }
        }
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let tab_style = |tab: Tab| {
        if app.current_tab == tab {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    let header = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled("Models", tab_style(Tab::Models)),
        Span::raw(" | "),
        Span::styled("Agents", tab_style(Tab::Agents)),
        Span::raw(" | "),
        Span::styled("Benchmarks", tab_style(Tab::Benchmarks)),
        Span::raw(" | "),
        Span::styled("Status", tab_style(Tab::Status)),
        Span::styled("  [/] switch tabs", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    // If there's a status message, show it instead of normal footer
    if let Some(status) = &app.status_message {
        let content = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(status, Style::default().fg(Color::Green)),
        ]);
        let paragraph = Paragraph::new(content);
        f.render_widget(paragraph, area);
        return;
    }

    match app.mode {
        Mode::Normal => {
            // Split footer into left and right sections
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(10)])
                .split(area);

            let left_content = match app.current_tab {
                Tab::Models => Line::from(vec![
                    Span::styled(" q ", Style::default().fg(Color::Yellow)),
                    Span::raw("quit  "),
                    Span::styled(" ↑/↓ ", Style::default().fg(Color::Yellow)),
                    Span::raw("nav  "),
                    Span::styled(" Tab ", Style::default().fg(Color::Yellow)),
                    Span::raw("switch  "),
                    Span::styled(" / ", Style::default().fg(Color::Yellow)),
                    Span::raw("search  "),
                    Span::styled(" s/S ", Style::default().fg(Color::Yellow)),
                    Span::raw("sort  "),
                    Span::styled(" 1-6 ", Style::default().fg(Color::Yellow)),
                    Span::raw("filter  "),
                    Span::styled(" c ", Style::default().fg(Color::Yellow)),
                    Span::raw("copy"),
                ]),
                Tab::Agents => Line::from(vec![
                    Span::styled(" q ", Style::default().fg(Color::Yellow)),
                    Span::raw("quit  "),
                    Span::styled(" / ", Style::default().fg(Color::Yellow)),
                    Span::raw("search  "),
                    Span::styled(" s ", Style::default().fg(Color::Yellow)),
                    Span::raw("sort  "),
                    Span::styled(" a ", Style::default().fg(Color::Yellow)),
                    Span::raw("track  "),
                    Span::styled(" o ", Style::default().fg(Color::Yellow)),
                    Span::raw("docs  "),
                    Span::styled(" r ", Style::default().fg(Color::Yellow)),
                    Span::raw("repo"),
                ]),
                Tab::Benchmarks => {
                    if app.selections.len() >= 2 {
                        use super::benchmarks_app::{BenchmarkFocus, BottomView};
                        let mut spans = vec![
                            Span::styled(" q ", Style::default().fg(Color::Yellow)),
                            Span::raw("quit  "),
                            Span::styled(" h/l ", Style::default().fg(Color::Yellow)),
                            Span::raw("focus  "),
                            Span::styled(" t ", Style::default().fg(Color::Yellow)),
                            Span::raw(if app.benchmarks_app.show_creators_in_compare {
                                "models  "
                            } else {
                                "creators  "
                            }),
                            Span::styled(" Space ", Style::default().fg(Color::Yellow)),
                            Span::raw("select  "),
                            Span::styled(" v ", Style::default().fg(Color::Yellow)),
                            Span::raw("view  "),
                        ];
                        match app.benchmarks_app.bottom_view {
                            BottomView::H2H => {
                                spans.extend([
                                    Span::styled(" d ", Style::default().fg(Color::Yellow)),
                                    Span::raw("detail  "),
                                ]);
                                if app.benchmarks_app.focus == BenchmarkFocus::Compare {
                                    spans.extend([
                                        Span::styled(" j/k ", Style::default().fg(Color::Yellow)),
                                        Span::raw("scroll  "),
                                    ]);
                                }
                            }
                            BottomView::Scatter => {
                                spans.extend([
                                    Span::styled(" x ", Style::default().fg(Color::Yellow)),
                                    Span::raw("X-axis  "),
                                    Span::styled(" y ", Style::default().fg(Color::Yellow)),
                                    Span::raw("Y-axis  "),
                                ]);
                            }
                            BottomView::Radar => {
                                spans.extend([
                                    Span::styled(" a ", Style::default().fg(Color::Yellow)),
                                    Span::raw("preset  "),
                                ]);
                            }
                            BottomView::Detail => {}
                        }
                        spans.extend([
                            Span::styled(" c ", Style::default().fg(Color::Yellow)),
                            Span::raw("clear  "),
                            Span::styled(" s ", Style::default().fg(Color::Yellow)),
                            Span::raw("sort  "),
                            Span::styled(" / ", Style::default().fg(Color::Yellow)),
                            Span::raw("search"),
                        ]);
                        Line::from(spans)
                    } else {
                        Line::from(vec![
                            Span::styled(" q ", Style::default().fg(Color::Yellow)),
                            Span::raw("quit  "),
                            Span::styled(" 1 ", Style::default().fg(Color::Yellow)),
                            Span::raw("intel  "),
                            Span::styled(" 2 ", Style::default().fg(Color::Yellow)),
                            Span::raw("date  "),
                            Span::styled(" 3 ", Style::default().fg(Color::Yellow)),
                            Span::raw("speed  "),
                            Span::styled(" 4 ", Style::default().fg(Color::Yellow)),
                            Span::raw("source  "),
                            Span::styled(" 5-6 ", Style::default().fg(Color::Yellow)),
                            Span::raw("group  "),
                            Span::styled(" 7 ", Style::default().fg(Color::Yellow)),
                            Span::raw("reasoning  "),
                            Span::styled(" s ", Style::default().fg(Color::Yellow)),
                            Span::raw("sort  "),
                            Span::styled(" / ", Style::default().fg(Color::Yellow)),
                            Span::raw("search  "),
                            Span::styled(" Space ", Style::default().fg(Color::Yellow)),
                            Span::raw("select"),
                        ])
                    }
                }
                Tab::Status => Line::from(vec![
                    Span::styled(" q ", Style::default().fg(Color::Yellow)),
                    Span::raw("quit  "),
                    Span::styled(" / ", Style::default().fg(Color::Yellow)),
                    Span::raw("search  "),
                    Span::styled(" Tab ", Style::default().fg(Color::Yellow)),
                    Span::raw("focus  "),
                    Span::styled(" o ", Style::default().fg(Color::Yellow)),
                    Span::raw("open page  "),
                    Span::styled(" r ", Style::default().fg(Color::Yellow)),
                    Span::raw("refresh"),
                ]),
            };

            let right_content = Line::from(vec![
                Span::styled(" ? ", Style::default().fg(Color::Yellow)),
                Span::raw("help "),
            ]);

            f.render_widget(Paragraph::new(left_content), chunks[0]);
            f.render_widget(
                Paragraph::new(right_content).alignment(ratatui::layout::Alignment::Right),
                chunks[1],
            );
        }
        Mode::Search => {
            // Get the correct search query based on current tab
            let search_query = match app.current_tab {
                Tab::Models => &app.models_app.search_query,
                Tab::Agents => app
                    .agents_app
                    .as_ref()
                    .map(|a| &a.search_query)
                    .unwrap_or(&app.models_app.search_query),
                Tab::Benchmarks => &app.benchmarks_app.search_query,
                Tab::Status => app
                    .status_app
                    .as_ref()
                    .map(|a| &a.search_query)
                    .unwrap_or(&app.models_app.search_query),
            };
            let content = Line::from(vec![
                Span::styled(" Search: ", Style::default().fg(Color::Cyan)),
                Span::raw(search_query),
                Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
                Span::raw("  "),
                Span::styled(" Enter/Esc ", Style::default().fg(Color::Yellow)),
                Span::raw("confirm"),
            ]);
            f.render_widget(Paragraph::new(content), area);
        }
    };
}

fn draw_help_popup(f: &mut Frame, scroll: u16, current_tab: Tab) {
    let area = centered_rect(50, 70, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    let help_section = |title: &'static str| -> Line<'static> {
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
    };

    let mut help_text = vec![
        // Common: Navigation
        help_section("Navigation"),
        help_line("j/↓", "Move down"),
        help_line("k/↑", "Move up"),
        help_line("g", "First item"),
        help_line("G", "Last item"),
        help_line("Ctrl+d/PgDn", "Page down"),
        help_line("Ctrl+u/PgUp", "Page up"),
        Line::from(""),
        // Common: Panels
        help_section("Panels"),
        help_line("h/←/l/→", "Switch panels"),
        help_line("Tab", "Switch panels"),
        Line::from(""),
        // Common: Search
        help_section("Search"),
        help_line("/", "Start search"),
        help_line("Enter/Esc", "Exit search mode"),
        help_line("Esc", "Clear search (in normal mode)"),
        Line::from(""),
    ];

    // Tab-specific sections
    match current_tab {
        Tab::Models => {
            help_text.extend(vec![
                help_section("Filters & Sort"),
                help_line("s", "Cycle sort (name → date → cost → context)"),
                help_line("S", "Toggle sort direction"),
                help_line("1", "Toggle reasoning models filter"),
                help_line("2", "Toggle tools filter"),
                help_line("3", "Toggle open weights filter"),
                help_line("4", "Toggle free models filter"),
                help_line("5", "Cycle provider category filter"),
                help_line("6", "Toggle category grouping"),
                Line::from(""),
                help_section("Copy & Open"),
                help_line("c", "Copy provider/model"),
                help_line("C", "Copy model only"),
                help_line("o", "Open provider docs in browser"),
                help_line("D", "Copy provider docs URL"),
                help_line("A", "Copy provider API URL"),
                Line::from(""),
            ]);
        }
        Tab::Agents => {
            help_text.extend(vec![
                help_section("Filters & Sort"),
                help_line("s", "Cycle sort (name → updated → stars → status)"),
                help_line("1", "Toggle installed filter"),
                help_line("2", "Toggle CLI filter"),
                help_line("3", "Toggle open source filter"),
                Line::from(""),
                help_section("Actions"),
                help_line("o", "Open docs in browser"),
                help_line("r", "Open GitHub repo in browser"),
                help_line("c", "Copy agent name"),
                help_line("a", "Add/remove tracked agents"),
                Line::from(""),
                help_section("Status Indicators"),
                Line::from(vec![
                    Span::styled(
                        format!("  {:<14}", "○"),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw("Not tracked"),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<14}", "◐"), Style::default().fg(Color::Yellow)),
                    Span::raw("Loading GitHub data"),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<14}", "●"), Style::default().fg(Color::Green)),
                    Span::raw("Up to date"),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<14}", "●"), Style::default().fg(Color::Blue)),
                    Span::raw("Update available"),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<14}", "✗"), Style::default().fg(Color::Red)),
                    Span::raw("Fetch failed"),
                ]),
                Line::from(""),
            ]);
        }
        Tab::Benchmarks => {
            help_text.extend(vec![
                help_section("Quick Sort (press again to flip direction)"),
                help_line("1", "Sort by Intelligence index"),
                help_line("2", "Sort by Release date"),
                help_line("3", "Sort by Speed (tok/s)"),
                Line::from(""),
                help_section("Filters"),
                help_line("4", "Cycle source filter (Open/Closed/Mixed)"),
                help_line("5", "Cycle region filter (US/China/Europe/...)"),
                help_line("6", "Cycle type filter (Startup/Big Tech/Research)"),
                help_line("7", "Cycle reasoning filter (All/Reasoning/Non-reasoning)"),
                Line::from(""),
                help_section("Sort (full cycle)"),
                help_line("s", "Open sort picker"),
                help_line("S", "Toggle sort direction"),
                Line::from(""),
                help_section("Actions"),
                help_line("o", "Open Artificial Analysis page"),
                Line::from(""),
                help_section("Compare"),
                help_line("Space", "Toggle model for comparison (max 8)"),
                help_line("c", "Clear all selections"),
                help_line("v", "Cycle view: H2H → Scatter → Radar"),
                help_line("d", "Show detail overlay (H2H view)"),
                help_line("x", "Cycle scatter X-axis"),
                help_line("y", "Cycle scatter Y-axis"),
                help_line("a", "Cycle radar preset"),
                help_line("j/k", "Scroll H2H table (when Compare focused)"),
                help_line("h/l", "Switch focus: List ↔ Compare"),
                help_line("t", "Toggle left panel: Models ↔ Creators"),
                Line::from(""),
            ]);
        }
        Tab::Status => {
            help_text.extend(vec![
                help_section("Actions"),
                help_line("o", "Open provider status page"),
                help_line("r", "Refresh provider status"),
                Line::from(""),
                help_section("Status view"),
                help_line("Tab/h/l", "Switch list/details focus"),
                help_line("/", "Search providers"),
                Line::from(""),
            ]);
        }
    }

    // Common: Tabs and Other
    help_text.extend(vec![
        help_section("Tabs"),
        help_line("[", "Previous tab"),
        help_line("]", "Next tab"),
        Line::from(""),
        help_section("Other"),
        help_line("q", "Quit"),
        help_line("?", "Toggle this help"),
    ]);

    let title = match current_tab {
        Tab::Models => " Models Help - ? or Esc to close (j/k to scroll) ",
        Tab::Agents => " Agents Help - ? or Esc to close (j/k to scroll) ",
        Tab::Benchmarks => " Benchmarks Help - ? or Esc to close (j/k to scroll) ",
        Tab::Status => " Status Help - ? or Esc to close (j/k to scroll) ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    // Clamp scroll to content bounds
    let visible_height = area.height.saturating_sub(2); // 2 for borders
    let content_lines = help_text.len() as u16;
    let max_scroll = content_lines.saturating_sub(visible_height);
    let scroll_pos = scroll.min(max_scroll);

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .scroll((scroll_pos, 0));
    f.render_widget(paragraph, area);

    // Scrollbar for help popup
    render_scrollbar(
        f,
        area,
        content_lines as usize,
        scroll_pos as usize,
        visible_height as usize,
        true,
    );
}
