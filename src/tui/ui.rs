use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, Focus, Mode, SortOrder};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(10), // Detail panel (expanded)
            Constraint::Length(1),  // Footer/search
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_detail(f, chunks[2], app);
    draw_footer(f, chunks[3], app);
}

fn draw_header(f: &mut Frame, area: Rect, _app: &App) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " models ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("- AI Model Browser"),
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

    let title = if app.search_query.is_empty() {
        format!(" Models ({}){} ", models.len(), sort_indicator)
    } else {
        format!(
            " Models ({}) [filter: {}]{} ",
            models.len(),
            app.search_query,
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

fn draw_detail(f: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = if let Some(entry) = app.current_model() {
        let model = &entry.model;
        let provider_id = &entry.provider_id;

        let caps = model.capabilities_str();
        let modalities = model.modalities_str();

        let mut detail_lines = vec![
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
            Line::from(vec![
                Span::styled("Provider: ", Style::default().fg(Color::DarkGray)),
                Span::styled(provider_id, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Context: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<12}", model.context_str())),
                Span::styled("Output: ", Style::default().fg(Color::DarkGray)),
                Span::raw(model.output_str()),
            ]),
            Line::from(vec![
                Span::styled("Input Cost: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(
                    "{:<10}",
                    model
                        .cost
                        .as_ref()
                        .and_then(|c| c.input)
                        .map(|v| format!("${}/M", v))
                        .unwrap_or("-".into())
                )),
                Span::styled("Output Cost: ", Style::default().fg(Color::DarkGray)),
                Span::raw(
                    model
                        .cost
                        .as_ref()
                        .and_then(|c| c.output)
                        .map(|v| format!("${}/M", v))
                        .unwrap_or("-".into()),
                ),
            ]),
            Line::from(vec![
                Span::styled("Capabilities: ", Style::default().fg(Color::DarkGray)),
                Span::raw(caps),
            ]),
            Line::from(vec![
                Span::styled("Modalities: ", Style::default().fg(Color::DarkGray)),
                Span::raw(modalities),
            ]),
        ];

        // Add release date if available
        if let Some(date) = &model.release_date {
            detail_lines.push(Line::from(vec![
                Span::styled("Released: ", Style::default().fg(Color::DarkGray)),
                Span::raw(date),
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

    let content = match app.mode {
        Mode::Normal => Line::from(vec![
            Span::styled(" j/k ", Style::default().fg(Color::Yellow)),
            Span::raw("nav  "),
            Span::styled(" h/l ", Style::default().fg(Color::Yellow)),
            Span::raw("panel  "),
            Span::styled(" / ", Style::default().fg(Color::Yellow)),
            Span::raw("search  "),
            Span::styled(" s ", Style::default().fg(Color::Yellow)),
            Span::raw("sort  "),
            Span::styled(" c ", Style::default().fg(Color::Yellow)),
            Span::raw("copy  "),
            Span::styled(" q ", Style::default().fg(Color::Yellow)),
            Span::raw("quit"),
        ]),
        Mode::Search => Line::from(vec![
            Span::styled(" Search: ", Style::default().fg(Color::Cyan)),
            Span::raw(&app.search_query),
            Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            Span::raw("  "),
            Span::styled(" Enter/Esc ", Style::default().fg(Color::Yellow)),
            Span::raw("confirm"),
        ]),
    };

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
