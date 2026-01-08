use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{App, Focus, Mode};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Detail panel
            Constraint::Length(1), // Footer/search
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_detail(f, chunks[2], app);
    draw_footer(f, chunks[3], app);
}

fn draw_header(f: &mut Frame, area: Rect, _app: &App) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" models ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("- AI Model Browser"),
    ]));
    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    draw_providers(f, chunks[0], app);
    draw_models(f, chunks[1], app);
}

fn draw_providers(f: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Providers;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .providers
        .iter()
        .enumerate()
        .map(|(i, (id, provider))| {
            let style = if i == app.selected_provider {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let text = format!("{} ({})", id, provider.models.len());
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Providers "),
        )
        .highlight_symbol("> ");

    f.render_widget(list, area);
}

fn draw_models(f: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Models;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let models = app.filtered_models();

    let items: Vec<ListItem> = models
        .iter()
        .enumerate()
        .map(|(i, (id, model))| {
            let style = if i == app.selected_model {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let cost = model.cost_str();
            let ctx = model.context_str();
            let text = format!("{:<35} {:>12}  {:>8} ctx", id, cost, ctx);
            ListItem::new(text).style(style)
        })
        .collect();

    let title = if app.search_query.is_empty() {
        " Models ".to_string()
    } else {
        format!(" Models (filter: {}) ", app.search_query)
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    f.render_widget(list, area);
}

fn draw_detail(f: &mut Frame, area: Rect, app: &App) {
    let detail = if let Some((_id, model)) = app.current_model() {
        let provider_name = app
            .current_provider()
            .map(|(pid, _)| pid.as_str())
            .unwrap_or("-");

        let caps = model.capabilities_str();
        let modalities = model.modalities_str();

        Line::from(vec![
            Span::styled(&model.name, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(provider_name, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::raw(format!("Context: {} | Output: {} | ", model.context_str(), model.output_str())),
            Span::raw(format!("Caps: {} | ", caps)),
            Span::raw(format!("IO: {}", modalities)),
        ])
    } else {
        Line::from("No model selected")
    };

    let paragraph = Paragraph::new(detail)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let content = match app.mode {
        Mode::Normal => Line::from(vec![
            Span::styled(" j/k ", Style::default().fg(Color::Yellow)),
            Span::raw("navigate  "),
            Span::styled(" h/l ", Style::default().fg(Color::Yellow)),
            Span::raw("switch panel  "),
            Span::styled(" / ", Style::default().fg(Color::Yellow)),
            Span::raw("search  "),
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
