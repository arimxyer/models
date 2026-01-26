use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, Filters, Focus, Mode, SortOrder, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(14), // Detail panel (expanded)
            Constraint::Length(1),  // Footer/search
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_details_row(f, chunks[2], app);
    draw_footer(f, chunks[3], app);

    // Draw help popup on top if visible
    if app.show_help {
        draw_help_popup(f, app.help_scroll);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let models_style = if app.current_tab == Tab::Models {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let agents_style = if app.current_tab == Tab::Agents {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let header = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled("Models", models_style),
        Span::raw(" | "),
        Span::styled("Agents", agents_style),
        Span::styled("  [/] switch tabs", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    draw_providers(f, chunks[0], app);
    draw_models(f, chunks[1], app);
}

fn draw_providers(f: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focus == Focus::Providers;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Build items list with "All" at the top
    let mut items: Vec<ListItem> = Vec::with_capacity(app.providers.len() + 1);

    // "All" option
    let all_style = Style::default().fg(Color::Green);
    let all_text = format!("All ({})", app.total_model_count());
    items.push(ListItem::new(all_text).style(all_style));

    // Individual providers
    for (id, provider) in app.providers.iter() {
        let text = format!("{} ({})", id, provider.models.len());
        items.push(ListItem::new(text));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Providers "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.provider_list_state);
}

fn draw_models(f: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.focus == Focus::Models;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let models = app.filtered_models();
    let show_provider_col = app.is_all_selected();

    // Build items with header row
    let mut items: Vec<ListItem> = Vec::with_capacity(models.len() + 1);

    // Header row
    let header_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::UNDERLINED);
    let header_text = if show_provider_col {
        format!(
            "{:<30} {:<18} {:>10} {:>8}",
            "Model ID", "Provider", "Cost", "Context"
        )
    } else {
        format!("{:<35} {:>12} {:>8}", "Model ID", "Cost", "Context")
    };
    items.push(ListItem::new(header_text).style(header_style));

    // Model rows
    for entry in models.iter() {
        let cost = entry.model.cost_str();
        let ctx = entry.model.context_str();
        let text = if show_provider_col {
            format!(
                "{:<30} {:<18} {:>10} {:>8}",
                truncate(&entry.id, 30),
                truncate(&entry.provider_id, 18),
                cost,
                ctx
            )
        } else {
            format!("{:<35} {:>12} {:>8}", truncate(&entry.id, 35), cost, ctx)
        };
        items.push(ListItem::new(text));
    }

    let sort_indicator = match app.sort_order {
        SortOrder::Default => "",
        SortOrder::ReleaseDate => " ↓date",
        SortOrder::Cost => " ↑cost",
        SortOrder::Context => " ↓ctx",
    };

    let filter_indicator = format_filters(&app.filters);

    let title = if app.search_query.is_empty() && filter_indicator.is_empty() {
        format!(" Models ({}){} ", models.len(), sort_indicator)
    } else if app.search_query.is_empty() {
        format!(
            " Models ({}){} [{}] ",
            models.len(),
            sort_indicator,
            filter_indicator
        )
    } else if filter_indicator.is_empty() {
        format!(
            " Models ({}) [{}]{} ",
            models.len(),
            app.search_query,
            sort_indicator
        )
    } else {
        format!(
            " Models ({}) [{}] [{}]{} ",
            models.len(),
            app.search_query,
            filter_indicator,
            sort_indicator
        )
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.model_list_state);
}

fn draw_details_row(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_provider_detail(f, chunks[0], app);
    draw_model_detail(f, chunks[1], app);
}

fn draw_provider_detail(f: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = if let Some(entry) = app.current_model() {
        // Find the provider
        let provider = app
            .providers
            .iter()
            .find(|(id, _)| id == &entry.provider_id)
            .map(|(_, p)| p);

        if let Some(provider) = provider {
            vec![
                Line::from(vec![Span::styled(
                    &provider.name,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Docs: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(provider.doc.as_deref().unwrap_or("-")),
                ]),
                Line::from(vec![
                    Span::styled("API:  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(provider.api.as_deref().unwrap_or("-")),
                ]),
                Line::from(vec![
                    Span::styled("NPM:  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(provider.npm.as_deref().unwrap_or("-")),
                ]),
                Line::from(vec![
                    Span::styled("Env:  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(if provider.env.is_empty() {
                        "-".to_string()
                    } else {
                        provider.env.join(", ")
                    }),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("o ", Style::default().fg(Color::Yellow)),
                    Span::raw("open docs - web  "),
                    Span::styled("A ", Style::default().fg(Color::Yellow)),
                    Span::raw("copy api"),
                ]),
            ]
        } else {
            vec![Line::from(Span::styled(
                "Provider not found",
                Style::default().fg(Color::DarkGray),
            ))]
        }
    } else {
        vec![Line::from(Span::styled(
            "No model selected",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Provider "))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_model_detail(f: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = if let Some(entry) = app.current_model() {
        let model = &entry.model;
        let provider_id = &entry.provider_id;

        let caps = model.capabilities_str();
        let modalities = model.modalities_str();

        // Format cache costs
        let cache_read = model
            .cost
            .as_ref()
            .and_then(|c| c.cache_read)
            .map(|v| format!("${}/M", v))
            .unwrap_or("-".into());
        let cache_write = model
            .cost
            .as_ref()
            .and_then(|c| c.cache_write)
            .map(|v| format!("${}/M", v))
            .unwrap_or("-".into());

        let mut detail_lines = vec![
            // Row 1: Name and ID
            Line::from(vec![
                Span::styled(
                    &model.name,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("({})", entry.id),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            // Row 2: Provider and Family
            Line::from(vec![
                Span::styled("Provider: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:<16}", provider_id),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled("Family: ", Style::default().fg(Color::DarkGray)),
                Span::raw(model.family.as_deref().unwrap_or("-")),
            ]),
            // Row 3: Status
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                Span::raw(model.status.as_deref().unwrap_or("active")),
            ]),
            Line::from(""),
            // Row 4: Context, Input, and Output limits
            Line::from(vec![
                Span::styled("Context: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<10}", model.context_str())),
                Span::styled("Input: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<10}", model.input_limit_str())),
                Span::styled("Output: ", Style::default().fg(Color::DarkGray)),
                Span::raw(model.output_str()),
            ]),
            // Row 5: Input and Output cost
            Line::from(vec![
                Span::styled("Input: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(
                    "{:<14}",
                    model
                        .cost
                        .as_ref()
                        .and_then(|c| c.input)
                        .map(|v| format!("${}/M", v))
                        .unwrap_or("-".into())
                )),
                Span::styled("Output: ", Style::default().fg(Color::DarkGray)),
                Span::raw(
                    model
                        .cost
                        .as_ref()
                        .and_then(|c| c.output)
                        .map(|v| format!("${}/M", v))
                        .unwrap_or("-".into()),
                ),
            ]),
            // Row 6: Cache read and write costs
            Line::from(vec![
                Span::styled("Cache Read: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<10}", cache_read)),
                Span::styled("Cache Write: ", Style::default().fg(Color::DarkGray)),
                Span::raw(cache_write),
            ]),
            Line::from(""),
            // Row 7: Capabilities
            Line::from(vec![
                Span::styled("Capabilities: ", Style::default().fg(Color::DarkGray)),
                Span::raw(caps),
            ]),
            // Row 8: Modalities
            Line::from(vec![
                Span::styled("Modalities: ", Style::default().fg(Color::DarkGray)),
                Span::raw(modalities),
            ]),
            // Row 9: Dates
            Line::from(vec![
                Span::styled("Released: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(
                    "{:<14}",
                    model.release_date.as_deref().unwrap_or("-")
                )),
                Span::styled("Knowledge: ", Style::default().fg(Color::DarkGray)),
                Span::raw(model.knowledge.as_deref().unwrap_or("-")),
            ]),
        ];

        // Add last updated if available
        if let Some(updated) = &model.last_updated {
            detail_lines.push(Line::from(vec![
                Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
                Span::raw(updated),
            ]));
        }

        detail_lines
    } else {
        vec![Line::from(Span::styled(
            "No model selected",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
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

            let left_content = Line::from(vec![
                Span::styled(" q ", Style::default().fg(Color::Yellow)),
                Span::raw("quit  "),
                Span::styled(" ↑/↓ ", Style::default().fg(Color::Yellow)),
                Span::raw("nav  "),
                Span::styled(" Tab ", Style::default().fg(Color::Yellow)),
                Span::raw("switch  "),
                Span::styled(" / ", Style::default().fg(Color::Yellow)),
                Span::raw("search  "),
                Span::styled(" s ", Style::default().fg(Color::Yellow)),
                Span::raw("sort  "),
                Span::styled(" c ", Style::default().fg(Color::Yellow)),
                Span::raw("copy (prov/model)"),
            ]);

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
            let content = Line::from(vec![
                Span::styled(" Search: ", Style::default().fg(Color::Cyan)),
                Span::raw(&app.search_query),
                Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
                Span::raw("  "),
                Span::styled(" Enter/Esc ", Style::default().fg(Color::Yellow)),
                Span::raw("confirm"),
            ]);
            f.render_widget(Paragraph::new(content), area);
        }
    };
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn format_filters(filters: &Filters) -> String {
    let mut active = Vec::new();
    if filters.reasoning {
        active.push("reasoning");
    }
    if filters.tools {
        active.push("tools");
    }
    if filters.open_weights {
        active.push("open");
    }
    active.join(", ")
}

fn draw_help_popup(f: &mut Frame, scroll: u16) {
    let area = centered_rect(50, 70, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  j/↓           ", Style::default().fg(Color::Yellow)),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("  k/↑           ", Style::default().fg(Color::Yellow)),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("  g             ", Style::default().fg(Color::Yellow)),
            Span::raw("First item"),
        ]),
        Line::from(vec![
            Span::styled("  G             ", Style::default().fg(Color::Yellow)),
            Span::raw("Last item"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d/PgDn   ", Style::default().fg(Color::Yellow)),
            Span::raw("Page down"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+u/PgUp   ", Style::default().fg(Color::Yellow)),
            Span::raw("Page up"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Panels",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  h/←/l/→       ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch panels"),
        ]),
        Line::from(vec![
            Span::styled("  Tab           ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch panels"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Search",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  /             ", Style::default().fg(Color::Yellow)),
            Span::raw("Start search"),
        ]),
        Line::from(vec![
            Span::styled("  Enter/Esc     ", Style::default().fg(Color::Yellow)),
            Span::raw("Exit search mode"),
        ]),
        Line::from(vec![
            Span::styled("  Esc           ", Style::default().fg(Color::Yellow)),
            Span::raw("Clear search (in normal mode)"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Filters & Sort",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  s             ", Style::default().fg(Color::Yellow)),
            Span::raw("Cycle sort (name → date → cost → context)"),
        ]),
        Line::from(vec![
            Span::styled("  1             ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle reasoning filter"),
        ]),
        Line::from(vec![
            Span::styled("  2             ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle tools filter"),
        ]),
        Line::from(vec![
            Span::styled("  3             ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle open weights filter"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Copy & Open",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  c             ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy provider/model"),
        ]),
        Line::from(vec![
            Span::styled("  C             ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy model only"),
        ]),
        Line::from(vec![
            Span::styled("  o             ", Style::default().fg(Color::Yellow)),
            Span::raw("Open provider docs in browser"),
        ]),
        Line::from(vec![
            Span::styled("  D             ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy provider docs URL"),
        ]),
        Line::from(vec![
            Span::styled("  A             ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy provider API URL"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Other",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  q             ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help - press ? or Esc to close (j/k to scroll) ");

    let paragraph = Paragraph::new(help_text).block(block).scroll((scroll, 0));
    f.render_widget(paragraph, area);
}

/// Create a centered rect using percentage of the available area
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
