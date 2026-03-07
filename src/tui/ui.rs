use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::app::{App, Filters, Focus, Mode, ProviderListItem, SortOrder, Tab};
use crate::agents::{format_stars, FetchStatus};
use crate::provider_category::{provider_category, ProviderCategory};

pub fn draw(f: &mut Frame, app: &mut App) {
    let (main_constraint, detail_constraint) = match app.current_tab {
        Tab::Models => (Constraint::Min(0), Constraint::Percentage(35)),
        Tab::Agents | Tab::Benchmarks => (Constraint::Min(0), Constraint::Length(0)),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            main_constraint,       // Main content
            detail_constraint,     // Detail panel (Models only)
            Constraint::Length(1), // Footer/search
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);

    match app.current_tab {
        Tab::Models => {
            draw_main(f, chunks[1], app);
            draw_details_row(f, chunks[2], app);
        }
        Tab::Agents => {
            draw_agents_main(f, chunks[1], app);
        }
        Tab::Benchmarks => {
            draw_benchmarks_main(f, chunks[1], app);
        }
    }

    draw_footer(f, chunks[3], app);

    // Draw help popup on top if visible
    if app.show_help {
        draw_help_popup(f, app.help_scroll, app.current_tab);
    }

    // Draw picker modal on top if visible (agents tab only)
    if app.current_tab == Tab::Agents {
        if let Some(agents_app) = &app.agents_app {
            if agents_app.show_picker {
                draw_picker_modal(f, app);
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

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Providers ");
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Split inner area into filter row + list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    // Filter toggles row
    let cat_active = app.provider_category_filter != ProviderCategory::All;
    let cat_color = if cat_active {
        app.provider_category_filter.color()
    } else {
        Color::DarkGray
    };
    let grp_color = if app.group_by_category {
        Color::Green
    } else {
        Color::DarkGray
    };

    let cat_label = if cat_active {
        app.provider_category_filter.short_label()
    } else {
        "Cat"
    };

    let filter_line = Line::from(vec![
        Span::styled("[5]", Style::default().fg(cat_color)),
        Span::raw(format!(" {} ", cat_label)),
        Span::styled("[6]", Style::default().fg(grp_color)),
        Span::raw(" Grp"),
    ]);
    f.render_widget(Paragraph::new(filter_line), chunks[0]);

    // Build items list from provider_list_items
    let mut items: Vec<ListItem> = Vec::with_capacity(app.provider_list_items.len());

    for item in &app.provider_list_items {
        match item {
            ProviderListItem::All => {
                let count = app.filtered_model_count();
                let text = format!("All ({})", count);
                items.push(ListItem::new(text).style(Style::default().fg(Color::Green)));
            }
            ProviderListItem::CategoryHeader(cat) => {
                let label = cat.label();
                let color = cat.color();
                // Create a separator line like "── Origin ──────"
                let avail = inner_area.width.saturating_sub(2) as usize; // account for highlight symbol space
                let label_len = label.len() + 4; // "── " + label + " "
                let trailing = if avail > label_len {
                    "\u{2500}".repeat(avail - label_len)
                } else {
                    String::new()
                };
                let text = format!("\u{2500}\u{2500} {} {}", label, trailing);
                items.push(
                    ListItem::new(text)
                        .style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
                );
            }
            ProviderListItem::Provider(idx) => {
                if let Some((id, provider)) = app.providers.get(*idx) {
                    let cat = provider_category(id);
                    let short = cat.short_label();
                    let color = cat.color();
                    let line = Line::from(vec![
                        Span::raw(format!("{} ({}) ", id, provider.models.len())),
                        Span::styled(short, Style::default().fg(color)),
                    ]);
                    items.push(ListItem::new(line));
                }
            }
        }
    }

    let caret = if is_focused { "> " } else { "  " };
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(caret);

    f.render_stateful_widget(list, chunks[1], &mut app.provider_list_state);
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

    let sort_indicator = match app.sort_order {
        SortOrder::Default => String::new(),
        _ => {
            let arrow = if app.sort_ascending {
                "\u{2191}"
            } else {
                "\u{2193}"
            };
            let label = match app.sort_order {
                SortOrder::ReleaseDate => "date",
                SortOrder::Cost => "cost",
                SortOrder::Context => "ctx",
                SortOrder::Default => unreachable!(),
            };
            format!(" {}{}", arrow, label)
        }
    };

    let filter_indicator = format_filters(&app.filters, app.provider_category_filter);

    // Show provider name in title when a specific provider is selected
    let provider_label = app
        .selected_provider_data()
        .map(|(_, p)| p.name.as_str())
        .unwrap_or("Models");

    let title = if app.search_query.is_empty() && filter_indicator.is_empty() {
        format!(" {} ({}){} ", provider_label, models.len(), sort_indicator)
    } else if app.search_query.is_empty() {
        format!(
            " {} ({}){} [{}] ",
            provider_label,
            models.len(),
            sort_indicator,
            filter_indicator
        )
    } else if filter_indicator.is_empty() {
        format!(
            " {} ({}) [{}]{} ",
            provider_label,
            models.len(),
            app.search_query,
            sort_indicator
        )
    } else {
        format!(
            " {} ({}) [{}] [{}]{} ",
            provider_label,
            models.len(),
            app.search_query,
            filter_indicator,
            sort_indicator
        )
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Fixed column widths: caret(2) + Input(8) Output(8) Context(8) + provider(18 optional)
    let caret_w: u16 = 2;
    let input_w: u16 = 8;
    let output_w: u16 = 8;
    let ctx_w: u16 = 8;
    let provider_w: u16 = if show_provider_col { 18 } else { 0 };
    let num_gaps: u16 = if show_provider_col { 4 } else { 3 };
    let fixed_w = caret_w + provider_w + input_w + output_w + ctx_w + num_gaps;
    let name_width = (inner_area.width.saturating_sub(fixed_w) as usize).max(10);

    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let active_header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Determine which column is actively sorted
    let sort_col = match app.sort_order {
        SortOrder::Default => "name",
        SortOrder::ReleaseDate => "name",
        SortOrder::Cost => "cost",
        SortOrder::Context => "context",
    };
    let cost_style = if sort_col == "cost" {
        active_header_style
    } else {
        header_style
    };

    // Caret prefix for focused panel
    let caret = if is_focused { "> " } else { "  " };

    // Build header spans (leading spaces to align with caret)
    let mut header_spans: Vec<Span> = vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<width$}", "Model ID", width = name_width),
            if sort_col == "name" {
                active_header_style
            } else {
                header_style
            },
        ),
    ];
    if show_provider_col {
        header_spans.push(Span::styled(format!(" {:<18}", "Provider"), header_style));
    }
    header_spans.push(Span::styled(format!(" {:>8}", "Input"), cost_style));
    header_spans.push(Span::styled(format!(" {:>8}", "Output"), cost_style));
    header_spans.push(Span::styled(
        format!(" {:>8}", "Context"),
        if sort_col == "context" {
            active_header_style
        } else {
            header_style
        },
    ));

    // Build items with header row
    let mut items: Vec<ListItem> = Vec::with_capacity(models.len() + 1);
    items.push(ListItem::new(Line::from(header_spans)));

    // Model rows
    for (display_idx, entry) in models.iter().enumerate() {
        let is_selected = display_idx == app.selected_model;
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let cost = &entry.model.cost;
        let input_cost = crate::data::Model::cost_short(cost.as_ref().and_then(|c| c.input));
        let output_cost = crate::data::Model::cost_short(cost.as_ref().and_then(|c| c.output));
        let ctx = entry.model.context_str();

        let prefix = if is_selected { caret } else { "  " };
        let mut row_spans: Vec<Span> = vec![
            Span::styled(prefix, style),
            Span::styled(
                format!(
                    "{:<width$}",
                    truncate(&entry.id, name_width.saturating_sub(1)),
                    width = name_width
                ),
                style,
            ),
        ];
        if show_provider_col {
            row_spans.push(Span::styled(
                format!(" {:<18}", truncate(&entry.provider_id, 18)),
                style,
            ));
        }
        row_spans.push(Span::styled(format!(" {:>8}", input_cost), style));
        row_spans.push(Span::styled(format!(" {:>8}", output_cost), style));
        row_spans.push(Span::styled(format!(" {:>8}", ctx), style));

        items.push(ListItem::new(Line::from(row_spans)));
    }

    let list = List::new(items);
    let mut state = app.model_list_state.clone();
    f.render_stateful_widget(list, inner_area, &mut state);
}

fn draw_details_row(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_provider_detail(f, chunks[0], app);
    draw_model_detail(f, chunks[1], app);
}

fn draw_agents_main(f: &mut Frame, area: Rect, app: &mut App) {
    if app.agents_app.is_none() {
        let msg = Paragraph::new("Failed to load agents data")
            .block(Block::default().borders(Borders::ALL).title(" Agents "));
        f.render_widget(msg, area);
        return;
    }

    // Compute list panel width from content
    let max_name_len = app
        .agents_app
        .as_ref()
        .and_then(|a| {
            a.filtered_entries
                .iter()
                .filter_map(|&idx| a.entries.get(idx))
                .map(|e| e.agent.name.len())
                .max()
        })
        .unwrap_or(5)
        .max(5);
    // 2 borders + 2 highlight + 2 (dot+space) + name + 2 gap + 6 type + 4 padding
    let list_width = (max_name_len + 18) as u16;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(list_width), Constraint::Min(0)])
        .split(area);

    draw_agent_list(f, chunks[0], app);
    draw_agent_detail(f, chunks[1], &mut *app);
}

/// Calculate visible height for detail panel (area height minus borders)
fn detail_visible_height(area: Rect) -> u16 {
    area.height.saturating_sub(2) // 2 for top and bottom borders
}

fn draw_agent_list(f: &mut Frame, area: Rect, app: &mut App) {
    use super::agents_app::AgentFocus;

    let agents_app = match &mut app.agents_app {
        Some(a) => a,
        None => return,
    };

    let is_focused = agents_app.focus == AgentFocus::List;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Build title with count, search query, filter, and sort indicators
    let sort_indicator = format!(" \u{2193}{}", agents_app.sort_order.label());
    let filter_indicator = agents_app.format_active_filters();
    let search_query = &agents_app.search_query;

    let title = match (search_query.is_empty(), filter_indicator.is_empty()) {
        (true, true) => format!(
            " Agents ({}){} ",
            agents_app.filtered_entries.len(),
            sort_indicator
        ),
        (true, false) => format!(
            " Agents ({}) [{}]{} ",
            agents_app.filtered_entries.len(),
            filter_indicator,
            sort_indicator
        ),
        (false, true) => format!(
            " Agents ({}) [/{}]{} ",
            agents_app.filtered_entries.len(),
            search_query,
            sort_indicator
        ),
        (false, false) => format!(
            " Agents ({}) [/{}] [{}]{} ",
            agents_app.filtered_entries.len(),
            search_query,
            filter_indicator,
            sort_indicator
        ),
    };

    // Outer block with title at top
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Split inner area into filter row + list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    // Filter toggles row
    let filter_line = Line::from(vec![
        Span::styled(
            "[1]",
            Style::default().fg(if agents_app.filters.installed_only {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
        Span::raw(" Inst "),
        Span::styled(
            "[2]",
            Style::default().fg(if agents_app.filters.cli_only {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
        Span::raw(" CLI "),
        Span::styled(
            "[3]",
            Style::default().fg(if agents_app.filters.open_source_only {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
        Span::raw(" OSS"),
    ]);
    let filter_para = Paragraph::new(filter_line);
    f.render_widget(filter_para, chunks[0]);

    // Agent list
    let mut items: Vec<ListItem> = Vec::new();

    // Compute dynamic agent name column width
    let max_name_len = agents_app
        .filtered_entries
        .iter()
        .filter_map(|&idx| agents_app.entries.get(idx))
        .map(|e| e.agent.name.len())
        .max()
        .unwrap_or(5)
        .max(5); // minimum width of 5 for "Agent" header

    // Header row (leading spaces match the "> " / "  " prefix)
    let header = format!(
        "  {:<2} {:<width$}  {:>6}",
        "St",
        "Agent",
        "Type",
        width = max_name_len,
    );
    items.push(
        ListItem::new(header).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        ),
    );

    // Agent rows (manual highlight to preserve status dot color)
    let selected = agents_app.agent_list_state.selected();

    for (row_idx, &idx) in agents_app.filtered_entries.iter().enumerate() {
        if let Some(entry) = agents_app.entries.get(idx) {
            let is_selected = selected == Some(row_idx);

            let agent_type = if entry.agent.categories.contains(&"cli".to_string()) {
                "CLI"
            } else if entry.agent.categories.contains(&"ide".to_string()) {
                "IDE"
            } else {
                "-"
            };

            // Status indicator: colored dot for installed agents, dash for others
            let (status_indicator, status_style) = if entry.installed.version.is_some() {
                match &entry.fetch_status {
                    FetchStatus::NotStarted => ("\u{25CB}", Style::default().fg(Color::DarkGray)), // ○ gray
                    FetchStatus::Loading => ("\u{25D0}", Style::default().fg(Color::Yellow)), // ◐ yellow
                    FetchStatus::Loaded => {
                        if entry.update_available() {
                            ("\u{25CF}", Style::default().fg(Color::Blue)) // ● blue = update available
                        } else {
                            ("\u{25CF}", Style::default().fg(Color::Green)) // ● green = up to date
                        }
                    }
                    FetchStatus::Failed(_) => ("\u{2717}", Style::default().fg(Color::Red)), // ✗ red
                }
            } else {
                ("-", Style::default().fg(Color::DarkGray))
            };

            let (prefix, text_style) = if is_selected {
                (
                    if is_focused { "> " } else { "  " },
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default())
            };

            let row = Line::from(vec![
                Span::styled(prefix, text_style),
                Span::styled(status_indicator, status_style),
                Span::styled(
                    format!(
                        " {:<width$}  {:>6}",
                        truncate(&entry.agent.name, max_name_len),
                        agent_type,
                        width = max_name_len,
                    ),
                    text_style,
                ),
            ]);
            items.push(ListItem::new(row));
        }
    }

    let list = List::new(items);

    // Offset by 1 for header row
    let mut state = agents_app.agent_list_state.clone();
    if let Some(selected) = state.selected() {
        state.select(Some(selected + 1));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);
}

fn draw_agent_detail(f: &mut Frame, area: Rect, app: &mut App) {
    use super::agents_app::AgentFocus;

    // Extract what we need from agents_app before building lines
    let (is_focused, search_query) = match &app.agents_app {
        Some(a) => (a.focus == AgentFocus::Details, a.search_query.clone()),
        None => return,
    };

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut match_line_indices: Vec<u16> = Vec::new();

    let lines: Vec<Line> = if let Some(entry) =
        app.agents_app.as_ref().and_then(|a| a.current_entry())
    {
        let mut detail_lines = Vec::new();

        // Header: Name + Version
        let name = entry.agent.name.clone();
        let version_str = entry.github.latest_version().unwrap_or("-").to_string();
        detail_lines.push(Line::from(vec![
            Span::styled(
                name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("v{}", version_str),
                Style::default().fg(Color::Cyan),
            ),
        ]));

        // Repo + Stars
        let repo = entry.agent.repo.clone();
        let stars_str = entry.github.stars.map(format_stars).unwrap_or_default();
        detail_lines.push(Line::from(vec![
            Span::styled(repo, Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled(
                format!("★ {}", stars_str),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        detail_lines.push(Line::from(""));

        // Installed status
        let installed_str = entry
            .installed
            .version
            .as_deref()
            .unwrap_or("Not installed");
        let status = if entry.update_available() {
            Span::styled(" (update available)", Style::default().fg(Color::Yellow))
        } else if entry.installed.version.is_some() {
            Span::styled(" (up to date)", Style::default().fg(Color::Green))
        } else {
            Span::raw("")
        };

        detail_lines.push(Line::from(vec![
            Span::styled("Installed: ", Style::default().fg(Color::DarkGray)),
            Span::raw(installed_str),
            status,
        ]));

        // Show status indicator based on fetch_status
        match &entry.fetch_status {
            FetchStatus::Loading => {
                detail_lines.push(Line::from(Span::styled(
                    "Loading GitHub data...",
                    Style::default().fg(Color::Yellow),
                )));
            }
            FetchStatus::Failed(error) => {
                detail_lines.push(Line::from(vec![
                    Span::styled("\u{2717} ", Style::default().fg(Color::Red)), // ✗
                    Span::styled(
                        format!("Failed to fetch: {}", error),
                        Style::default().fg(Color::Red),
                    ),
                ]));
            }
            FetchStatus::NotStarted => {
                if entry.tracked {
                    detail_lines.push(Line::from(Span::styled(
                        "Waiting to fetch GitHub data...",
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
            FetchStatus::Loaded => {
                // No indicator needed when data is loaded
            }
        }

        detail_lines.push(Line::from(""));

        // Release history
        if entry.github.releases.is_empty() {
            detail_lines.push(Line::from(Span::styled(
                "No releases available",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            detail_lines.push(Line::from(Span::styled(
                "Release History:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            detail_lines.push(Line::from(Span::styled(
                "───────────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )));

            let installed_version = entry.installed.version.as_deref();
            let new_releases = entry.new_releases();

            for release in &entry.github.releases {
                let is_installed = installed_version == Some(release.version.as_str());
                let is_new = new_releases.iter().any(|r| r.version == release.version);

                // Version header with markers
                let mut version_spans = vec![Span::styled(
                    format!("v{}", release.version),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )];

                if let Some(date) = &release.date {
                    let display_date = crate::agents::helpers::parse_date(date)
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| date.clone());
                    version_spans.push(Span::styled(
                        format!("  {}", display_date),
                        Style::default().fg(Color::DarkGray),
                    ));
                }

                if is_installed {
                    version_spans.push(Span::styled(
                        "  ← INSTALLED",
                        Style::default().fg(Color::Green),
                    ));
                } else if is_new {
                    version_spans.push(Span::styled("  ← NEW", Style::default().fg(Color::Yellow)));
                }

                detail_lines.push(Line::from(version_spans));

                // Changelog for this release
                if let Some(changelog) = &release.changelog {
                    if search_query.is_empty() {
                        detail_lines.extend(super::markdown::changelog_to_lines(changelog));
                    } else {
                        let changelog_lines = super::markdown::changelog_to_lines_highlighted(
                            changelog,
                            &search_query,
                        );
                        for cl in changelog_lines {
                            if super::markdown::line_contains_match(&cl, &search_query) {
                                match_line_indices.push(detail_lines.len() as u16);
                            }
                            detail_lines.push(cl);
                        }
                    }
                }

                detail_lines.push(Line::from("")); // Space between releases
            }
        }

        // Keybinding hints at the bottom
        detail_lines.push(Line::from(""));
        let mut hints = vec![
            Span::styled(" o ", Style::default().fg(Color::Yellow)),
            Span::raw("open docs  "),
            Span::styled(" r ", Style::default().fg(Color::Yellow)),
            Span::raw("open repo  "),
            Span::styled(" c ", Style::default().fg(Color::Yellow)),
            Span::raw("copy name"),
        ];
        if !search_query.is_empty() {
            hints.push(Span::raw("  "));
            hints.push(Span::styled(" n/N ", Style::default().fg(Color::Yellow)));
            hints.push(Span::raw("next/prev match"));
        }
        detail_lines.push(Line::from(hints));

        detail_lines
    } else {
        vec![Line::from(Span::styled(
            "Select an agent to view details",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    // Compute visual (wrapped) line offsets for accurate scrolling
    let visible_height = detail_visible_height(area);
    let wrap_width = area.width.saturating_sub(2) as usize; // subtract borders

    // Build a cumulative visual line offset for each logical line
    let mut visual_offsets: Vec<u16> = Vec::with_capacity(lines.len());
    let mut visual_total: u16 = 0;
    for line in &lines {
        visual_offsets.push(visual_total);
        let line_width: usize = line.spans.iter().map(|s| s.content.len()).sum();
        let wrapped_lines = if wrap_width == 0 || line_width == 0 {
            1
        } else {
            line_width.div_ceil(wrap_width).max(1) as u16
        };
        visual_total += wrapped_lines;
    }

    // Compute visual offsets for match lines specifically
    let match_visual_offsets: Vec<u16> = match_line_indices
        .iter()
        .map(|&idx| visual_offsets.get(idx as usize).copied().unwrap_or(0))
        .collect();

    // Clamp scroll to content bounds (using visual line count)
    let max_scroll = visual_total.saturating_sub(visible_height);
    let scroll_pos = {
        let agents_app = match &app.agents_app {
            Some(a) => a,
            None => return,
        };
        agents_app.detail_scroll.min(max_scroll)
    };

    // Build detail title with match count
    let match_count = match_line_indices.len();
    let current_match_display = app
        .agents_app
        .as_ref()
        .map(|a| a.current_match)
        .unwrap_or(0);
    let detail_title = if !search_query.is_empty() && match_count > 0 {
        format!(" Details [{}/{}] ", current_match_display + 1, match_count)
    } else {
        " Details ".to_string()
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(detail_title),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_pos, 0));

    f.render_widget(paragraph, area);

    // Update match state and detail height (after lines are consumed)
    app.last_detail_height = visible_height;
    if let Some(ref mut agents_app) = app.agents_app {
        agents_app.update_search_matches(match_line_indices, match_visual_offsets);
    }
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
            let cat = provider_category(&entry.provider_id);
            vec![
                Line::from(vec![Span::styled(
                    &provider.name,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Category: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(cat.label(), Style::default().fg(cat.color())),
                ]),
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

fn draw_benchmarks_main(f: &mut Frame, area: Rect, app: &mut App) {
    let in_compare = app.selections.len() >= 2;

    if in_compare {
        // Compare mode: compact list (30%, min 35 chars) | comparison (remainder), full height
        let list_w = (area.width * 30 / 100).max(35);
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(list_w), Constraint::Min(0)])
            .split(area);

        if app.benchmarks_app.show_creators_in_compare {
            draw_benchmark_creators(f, h_chunks[0], app);
        } else {
            draw_benchmark_list_compact(f, h_chunks[0], app);
        }

        // Comparison panel: sub-tab bar + view
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(h_chunks[1]);

        draw_benchmark_subtab_bar(f, v_chunks[0], &app.benchmarks_app);

        match app.benchmarks_app.bottom_view {
            super::benchmarks_app::BottomView::H2H => {
                draw_h2h_table_generic(f, v_chunks[1], app);
            }
            super::benchmarks_app::BottomView::Scatter => {
                draw_scatter(f, v_chunks[1], app);
            }
            super::benchmarks_app::BottomView::Radar => {
                super::radar::draw_radar(f, v_chunks[1], app);
            }
            super::benchmarks_app::BottomView::Detail => {
                draw_benchmark_detail(f, v_chunks[1], app);
            }
        }
    } else {
        // Browse mode: creators (25%) | list+detail (75%)
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);

        draw_benchmark_creators(f, h_chunks[0], app);

        // Vertical split: list (65%) | detail (35%)
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(h_chunks[1]);

        draw_benchmark_list(f, v_chunks[0], app);
        draw_benchmark_detail(f, v_chunks[1], app);
    }

    // Detail overlay (drawn last, on top of everything)
    if app.benchmarks_app.show_detail_overlay && app.selections.len() >= 2 {
        draw_detail_overlay(f, area, app);
    }
}

fn draw_benchmark_subtab_bar(
    f: &mut Frame,
    area: Rect,
    bench_app: &super::benchmarks_app::BenchmarksApp,
) {
    use super::benchmarks_app::BottomView;
    let views = [
        ("H2H", BottomView::H2H),
        ("Scatter", BottomView::Scatter),
        ("Radar", BottomView::Radar),
    ];
    let mut spans = Vec::new();
    for (label, view) in &views {
        let style = if bench_app.bottom_view == *view {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" [{}] ", label), style));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_benchmark_creators(f: &mut Frame, area: Rect, app: &mut App) {
    use super::benchmarks_app::{
        BenchmarkFocus, CreatorGrouping, CreatorListItem, CreatorRegion, CreatorType,
    };

    let bench_app = &mut app.benchmarks_app;
    let store = &app.benchmark_store;

    let is_focused = bench_app.focus == BenchmarkFocus::Creators;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Creators ");
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Grouping toggle indicators
    let rgn_active = bench_app.creator_grouping == CreatorGrouping::ByRegion;
    let rgn_color = if rgn_active {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let typ_active = bench_app.creator_grouping == CreatorGrouping::ByType;
    let typ_color = if typ_active {
        Color::Magenta
    } else {
        Color::DarkGray
    };

    let filter_line = Line::from(vec![
        Span::styled("[5]", Style::default().fg(rgn_color)),
        Span::raw(if rgn_active { "Region " } else { "Rgn " }),
        Span::styled("[6]", Style::default().fg(typ_color)),
        Span::raw("Type"),
    ]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    f.render_widget(Paragraph::new(filter_line), chunks[0]);

    // Available width for creator items (inner area minus highlight symbol "  " or "> ")
    let item_width = inner_area.width.saturating_sub(2) as usize;

    let items: Vec<ListItem> = bench_app
        .creator_list_items
        .iter()
        .map(|item| match item {
            CreatorListItem::All => {
                let count = store.entries().len();
                ListItem::new(Line::from(vec![
                    Span::styled("All", Style::default().fg(Color::Green)),
                    Span::raw(format!(" ({})", count)),
                ]))
            }
            CreatorListItem::GroupHeader(label) => {
                // Match models panel: full-width colored header with trailing ───
                let header_color = match bench_app.creator_grouping {
                    CreatorGrouping::ByRegion => {
                        CreatorRegion::from_label(label).map_or(Color::DarkGray, |r| r.color())
                    }
                    CreatorGrouping::ByType => {
                        CreatorType::from_label(label).map_or(Color::DarkGray, |t| t.color())
                    }
                    _ => Color::DarkGray,
                };
                let label_len = label.len() + 4; // "── " + label + " "
                let trailing = if item_width > label_len {
                    "\u{2500}".repeat(item_width - label_len)
                } else {
                    String::new()
                };
                let text = format!("\u{2500}\u{2500} {} {}", label, trailing);
                ListItem::new(text).style(
                    Style::default()
                        .fg(header_color)
                        .add_modifier(Modifier::BOLD),
                )
            }
            CreatorListItem::Creator(slug) => {
                let (display_name, count) = bench_app.creator_display(slug);
                // When grouped, show a colored tag for the creator's classification
                let tag = match bench_app.creator_grouping {
                    CreatorGrouping::ByRegion => {
                        let r = CreatorRegion::from_creator(slug);
                        Some((r.label(), r.color()))
                    }
                    CreatorGrouping::ByType => {
                        let t = CreatorType::from_creator(slug);
                        Some((t.label(), t.color()))
                    }
                    CreatorGrouping::None => None,
                };
                let count_str = format!("({})", count);
                let tag_len = tag.as_ref().map_or(0, |(l, _)| l.len() + 1);
                let overhead = count_str.len() + 1 + tag_len;
                let max_name = item_width.saturating_sub(overhead);
                let name = truncate(display_name, max_name);
                let mut spans = vec![
                    Span::raw(format!("{} ", name)),
                    Span::styled(count_str, Style::default().fg(Color::DarkGray)),
                ];
                if let Some((label, color)) = tag {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(label, Style::default().fg(color)));
                }
                ListItem::new(Line::from(spans))
            }
        })
        .collect();

    let caret = if is_focused { "> " } else { "  " };
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(caret);

    let mut state = bench_app.creator_list_state.clone();
    f.render_stateful_widget(list, chunks[1], &mut state);
}

/// Color palette for selected models in comparison mode.
pub(super) fn compare_colors(index: usize) -> Color {
    const PALETTE: [Color; 8] = [
        Color::Red,
        Color::Green,
        Color::Blue,
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
        Color::LightRed,
        Color::LightGreen,
    ];
    PALETTE[index % PALETTE.len()]
}

/// Compact list for compare mode: selection marker + name only, full height.
fn draw_benchmark_list_compact(f: &mut Frame, area: Rect, app: &mut App) {
    use super::benchmarks_app::BenchmarkFocus;

    let bench_app = &mut app.benchmarks_app;
    let store = &app.benchmark_store;

    let is_focused = bench_app.focus == BenchmarkFocus::List;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let sort_dir = if bench_app.sort_descending {
        "\u{2193}"
    } else {
        "\u{2191}"
    };
    let sort_indicator = format!(" {}{}", sort_dir, bench_app.sort_column.label());

    let source_indicator = match bench_app.source_filter {
        super::benchmarks_app::SourceFilter::All => String::new(),
        filter => format!(" [{}]", filter.label()),
    };

    let title = if bench_app.search_query.is_empty() {
        format!(
            " Models ({}){}{} ",
            bench_app.filtered_indices.len(),
            source_indicator,
            sort_indicator
        )
    } else {
        format!(
            " Models ({}) [/{}]{}{} ",
            bench_app.filtered_indices.len(),
            bench_app.search_query,
            source_indicator,
            sort_indicator
        )
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let caret = if is_focused { "> " } else { "  " };
    let entries = store.entries();
    let name_width = inner_area.width.saturating_sub(4) as usize; // 2 caret + 2 marker

    let items: Vec<ListItem> = bench_app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &entry_idx)| {
            let entry = &entries[entry_idx];
            let is_selected = display_idx == bench_app.selected;

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { caret } else { "  " };
            let mut row_spans: Vec<Span> = Vec::new();

            // Selection marker
            if let Some(sel_pos) = app.selections.iter().position(|&i| i == entry_idx) {
                row_spans.push(Span::styled(
                    "\u{25CF} ",
                    Style::default().fg(compare_colors(sel_pos)),
                ));
            } else {
                row_spans.push(Span::raw("  "));
            }

            row_spans.push(Span::styled(prefix, style));
            row_spans.push(Span::styled(truncate(&entry.name, name_width), style));
            ListItem::new(Line::from(row_spans))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    let mut state = bench_app.list_state.clone();
    state.select(Some(bench_app.selected));
    f.render_stateful_widget(list, inner_area, &mut state);
}

fn draw_benchmark_list(f: &mut Frame, area: Rect, app: &mut App) {
    use super::benchmarks_app::BenchmarkFocus;

    let bench_app = &mut app.benchmarks_app;
    let store = &app.benchmark_store;

    let is_focused = bench_app.focus == BenchmarkFocus::List;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let sort_dir = if bench_app.sort_descending {
        "\u{2193}"
    } else {
        "\u{2191}"
    };
    let sort_indicator = format!(" {}{}", sort_dir, bench_app.sort_column.label());

    let source_indicator = match bench_app.source_filter {
        super::benchmarks_app::SourceFilter::All => String::new(),
        filter => format!(" [{}]", filter.label()),
    };

    let creator_label = bench_app.selected_creator_name().unwrap_or("Benchmarks");

    let title = if bench_app.search_query.is_empty() {
        format!(
            " {} ({}){}{} ",
            creator_label,
            bench_app.filtered_indices.len(),
            source_indicator,
            sort_indicator
        )
    } else {
        format!(
            " {} ({}) [/{}]{}{} ",
            creator_label,
            bench_app.filtered_indices.len(),
            bench_app.search_query,
            source_indicator,
            sort_indicator
        )
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Dynamic columns based on active sort
    let visible_cols = bench_app.sort_column.visible_columns();

    // Compute dynamic name column width from available space (minus 2 for caret)
    let caret_w: u16 = 2;
    let fixed_width: u16 = visible_cols
        .iter()
        .map(|col| benchmark_col_width(*col))
        .sum();
    let selection_w: u16 = if !app.selections.is_empty() { 2 } else { 0 };
    let name_width = (inner_area
        .width
        .saturating_sub(fixed_width + caret_w + selection_w) as usize)
        .max(10);

    // Caret prefix for focused panel
    let caret = if is_focused { "> " } else { "  " };

    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let active_header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let has_selections = !app.selections.is_empty();
    let mut header_spans: Vec<Span> = Vec::new();
    if has_selections {
        header_spans.push(Span::raw("  ")); // align with selection marker column
    }
    header_spans.push(Span::raw("  "));
    header_spans.extend(visible_cols.iter().map(|col| {
        let style = if *col == bench_app.sort_column {
            active_header_style
        } else {
            header_style
        };
        benchmark_col_header(*col, style, name_width)
    }));
    let header = ListItem::new(Line::from(header_spans));

    let entries = store.entries();
    let mut items: Vec<ListItem> = vec![header];

    for (display_idx, &entry_idx) in bench_app.filtered_indices.iter().enumerate() {
        let entry = &entries[entry_idx];
        let is_selected = display_idx == bench_app.selected;

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let prefix = if is_selected { caret } else { "  " };
        let mut row_spans: Vec<Span> = Vec::new();

        // Selection marker
        if let Some(sel_pos) = app.selections.iter().position(|&i| i == entry_idx) {
            row_spans.push(Span::styled(
                "\u{25CF} ",
                Style::default().fg(compare_colors(sel_pos)),
            ));
        } else if has_selections {
            row_spans.push(Span::raw("  "));
        }

        row_spans.push(Span::styled(prefix, style));
        row_spans.extend(
            visible_cols
                .iter()
                .map(|col| benchmark_col_value(entry, *col, style, name_width)),
        );
        items.push(ListItem::new(Line::from(row_spans)));
    }

    let list = List::new(items);
    let mut state = bench_app.list_state.clone();
    // Offset by 1 for the header row
    state.select(Some(bench_app.selected + 1));
    f.render_stateful_widget(list, inner_area, &mut state);
}

fn draw_benchmark_detail(f: &mut Frame, area: Rect, app: &App) {
    let bench_app = &app.benchmarks_app;
    let store = &app.benchmark_store;

    let border_style = Style::default().fg(Color::DarkGray);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Details ");

    let entry = match bench_app.current_entry(store) {
        Some(e) => e,
        None => {
            let msg = Paragraph::new("No benchmark selected").block(block);
            f.render_widget(msg, area);
            return;
        }
    };

    let inner = block.inner(area);
    f.render_widget(block, area);
    draw_benchmark_detail_content(f, inner, entry, app);
}

fn draw_benchmark_detail_content(
    f: &mut Frame,
    area: Rect,
    entry: &crate::benchmarks::BenchmarkEntry,
    app: &App,
) {
    let mut lines: Vec<Line> = Vec::new();

    // Name + creator + metadata on first lines
    let creator_display = if !entry.creator_name.is_empty() {
        &entry.creator_name
    } else {
        &entry.creator
    };
    let region = super::benchmarks_app::CreatorRegion::from_creator(&entry.creator);
    let creator_type = super::benchmarks_app::CreatorType::from_creator(&entry.creator);

    // Line 1: Name
    lines.push(Line::from(Span::styled(
        &entry.name,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    // Line 2: Creator + Source (per-model only, no fallback)
    let em = "\u{2014}";
    let (source_label, source_color) = match app.open_weights_map.get(&entry.slug) {
        Some(true) => ("Open", Color::Green),
        Some(false) => ("Closed", Color::Red),
        None => (em, Color::DarkGray),
    };
    lines.push(Line::from(vec![
        Span::styled("Creator  ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{:<16}", creator_display)),
        Span::styled("Source  ", Style::default().fg(Color::DarkGray)),
        Span::styled(source_label, Style::default().fg(source_color)),
    ]));
    // Line 3: Region + Type
    lines.push(Line::from(vec![
        Span::styled("Region   ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{:<16}", region.label())),
        Span::styled("Type    ", Style::default().fg(Color::DarkGray)),
        Span::raw(creator_type.label()),
    ]));
    // Line 4: Release date
    let date_str = entry.release_date.as_deref().unwrap_or(em);
    lines.push(Line::from(vec![
        Span::styled("Released ", Style::default().fg(Color::DarkGray)),
        Span::raw(date_str),
    ]));

    // Composite Indexes (0-100 scale, higher is better)
    lines.push(Line::from(""));
    push_section_header(&mut lines, "Indexes (0\u{2013}100, \u{2191} better)");
    push_three_col(
        &mut lines,
        "Intelligence",
        fmt_idx(entry.intelligence_index),
        "Coding",
        fmt_idx(entry.coding_index),
        "Math",
        fmt_idx(entry.math_index),
    );

    // Benchmark Scores (percentage, higher is better)
    lines.push(Line::from(""));
    push_section_header(&mut lines, "Benchmarks (%, \u{2191} better)");
    push_three_col(
        &mut lines,
        "GPQA",
        fmt_pct(entry.gpqa),
        "MMLU-Pro",
        fmt_pct(entry.mmlu_pro),
        "HLE",
        fmt_pct(entry.hle),
    );
    push_three_col(
        &mut lines,
        "LiveCode",
        fmt_pct(entry.livecodebench),
        "SciCode",
        fmt_pct(entry.scicode),
        "IFBench",
        fmt_pct(entry.ifbench),
    );
    push_three_col(
        &mut lines,
        "Terminal",
        fmt_pct(entry.terminalbench_hard),
        "Tau2",
        fmt_pct(entry.tau2),
        "LCR",
        fmt_pct(entry.lcr),
    );
    push_three_col(
        &mut lines,
        "MATH-500",
        fmt_pct(entry.math_500),
        "AIME",
        fmt_pct(entry.aime),
        "AIME'25",
        fmt_pct(entry.aime_25),
    );

    // Performance (speed: higher better, TTFT/TTFAT: lower better)
    lines.push(Line::from(""));
    push_section_header(
        &mut lines,
        "Performance (Speed \u{2191}, TTFT/TTFAT \u{2193})",
    );
    let tps_str = entry
        .output_tps
        .map(|v| format!("{:.0} tok/s", v))
        .unwrap_or_else(|| em.to_string());
    let ttft_str = entry
        .ttft
        .map(|v| format!("{:.2}s", v))
        .unwrap_or_else(|| em.to_string());
    let ttfat_str = entry
        .ttfat
        .map(|v| format!("{:.2}s", v))
        .unwrap_or_else(|| em.to_string());
    push_three_col(
        &mut lines, "Speed", tps_str, "TTFT", ttft_str, "TTFAT", ttfat_str,
    );

    // Pricing ($/M tokens, lower is better)
    lines.push(Line::from(""));
    push_section_header(&mut lines, "Pricing ($/M tokens, \u{2193} better)");
    let blended_str = entry
        .price_blended
        .map(|v| format!("${:.2}", v))
        .unwrap_or_else(|| em.to_string());
    push_three_col(
        &mut lines,
        "Input",
        fmt_price(entry.price_input),
        "Output",
        fmt_price(entry.price_output),
        "Blended",
        blended_str,
    );

    // Keybinding hints
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("c ", Style::default().fg(Color::Yellow)),
        Span::styled("copy  ", Style::default().fg(Color::DarkGray)),
        Span::styled("o ", Style::default().fg(Color::Yellow)),
        Span::styled("open AA", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_detail_overlay(f: &mut Frame, area: Rect, app: &App) {
    // Centered rect: 60% width, 75% height
    let overlay_area = centered_rect(60, 75, area);

    // Clear background
    f.render_widget(Clear, overlay_area);

    let bench_app = &app.benchmarks_app;
    let store = &app.benchmark_store;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Model Detail (Esc to close) ");

    let entry = match bench_app.current_entry(store) {
        Some(e) => e,
        None => {
            let msg = Paragraph::new("No benchmark selected").block(block);
            f.render_widget(msg, overlay_area);
            return;
        }
    };

    let inner = block.inner(overlay_area);
    f.render_widget(block, overlay_area);
    draw_benchmark_detail_content(f, inner, entry, app);
}

/// Push a section header line like "─── Title ───"
fn push_section_header(lines: &mut Vec<Line>, title: &str) {
    lines.push(Line::from(Span::styled(
        format!(
            "\u{2500}\u{2500}\u{2500} {} \u{2500}\u{2500}\u{2500}",
            title
        ),
        Style::default().fg(Color::DarkGray),
    )));
}

fn push_three_col(
    lines: &mut Vec<Line>,
    l1: &str,
    v1: String,
    l2: &str,
    v2: String,
    l3: &str,
    v3: String,
) {
    let em = "\u{2014}";
    let color = |s: &str| {
        if s == em {
            Color::DarkGray
        } else {
            Color::White
        }
    };

    let mut spans = vec![
        Span::styled(format!("  {:<13}", l1), Style::default().fg(Color::Gray)),
        Span::styled(format!("{:<10}", v1), Style::default().fg(color(&v1))),
    ];

    if !l2.is_empty() {
        spans.push(Span::styled(
            format!("{:<13}", l2),
            Style::default().fg(Color::Gray),
        ));
        spans.push(Span::styled(
            format!("{:<10}", v2),
            Style::default().fg(color(&v2)),
        ));
    }

    if !l3.is_empty() {
        spans.push(Span::styled(
            format!("{:<13}", l3),
            Style::default().fg(Color::Gray),
        ));
        spans.push(Span::styled(v3.clone(), Style::default().fg(color(&v3))));
    }

    lines.push(Line::from(spans));
}

/// Format a 0-100 index value
fn fmt_idx(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.1}", v),
        None => "\u{2014}".to_string(),
    }
}

/// Format a 0-1 decimal score as a percentage
fn fmt_pct(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.1}%", v * 100.0),
        None => "\u{2014}".to_string(),
    }
}

/// Format a price value for list columns (right-aligned, 9 chars)
fn fmt_col_price(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:>8.2}$", v),
        None => format!("{:>9}", "\u{2014}"),
    }
}

/// Format a price value
fn fmt_price(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("${:.3}", v),
        None => "\u{2014}".to_string(),
    }
}

/// Format a 0-100 index for list columns (right-aligned, 6 chars)
fn fmt_col_idx(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:>6.1}", v),
        None => format!("{:>6}", "\u{2014}"),
    }
}

/// Format a 0-1 decimal score as % for list columns (right-aligned, 6 chars)
fn fmt_col_pct(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:>5.1}%", v * 100.0),
        None => format!("{:>6}", "\u{2014}"),
    }
}

fn fmt_speed(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:>7.0}", v),
        None => format!("{:>7}", "\u{2014}"),
    }
}

fn fmt_col_ttft(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:>6.2}s", v),
        None => format!("{:>7}", "\u{2014}"),
    }
}

fn fmt_col_date(value: Option<&str>) -> String {
    match value {
        Some(d) => format!("{:>11}", d),
        None => format!("{:>11}", "\u{2014}"),
    }
}

/// Fixed width for a non-Name column.
fn benchmark_col_width(col: super::benchmarks_app::BenchmarkSortColumn) -> u16 {
    use super::benchmarks_app::BenchmarkSortColumn::*;
    match col {
        Name => 0, // dynamic
        Speed | Ttft | Ttfat => 7,
        PriceInput | PriceOutput | PriceBlended => 9,
        ReleaseDate => 11,
        _ => 6, // all index/percentage columns
    }
}

/// Render a column header span for the given sort column
fn benchmark_col_header(
    col: super::benchmarks_app::BenchmarkSortColumn,
    style: Style,
    name_width: usize,
) -> Span<'static> {
    use super::benchmarks_app::BenchmarkSortColumn::*;
    match col {
        Name => Span::styled(format!("{:<width$}", "Name", width = name_width), style),
        Intelligence => Span::styled(format!("{:>6}", "Intel"), style),
        Coding => Span::styled(format!("{:>6}", "Code"), style),
        Math => Span::styled(format!("{:>6}", "Math"), style),
        Gpqa => Span::styled(format!("{:>6}", "GPQA"), style),
        MMLUPro => Span::styled(format!("{:>6}", "MMLU"), style),
        Hle => Span::styled(format!("{:>6}", "HLE"), style),
        LiveCode => Span::styled(format!("{:>6}", "LCode"), style),
        SciCode => Span::styled(format!("{:>6}", "SciCd"), style),
        Terminal => Span::styled(format!("{:>6}", "Term"), style),
        IFBench => Span::styled(format!("{:>6}", "IFB"), style),
        Lcr => Span::styled(format!("{:>6}", "LCR"), style),
        Tau2 => Span::styled(format!("{:>6}", "Tau2"), style),
        Speed => Span::styled(format!("{:>7}", "Tok/s"), style),
        Ttft => Span::styled(format!("{:>7}", "TTFT"), style),
        Ttfat => Span::styled(format!("{:>7}", "TTFAT"), style),
        PriceInput => Span::styled(format!("{:>9}", "In $/M"), style),
        PriceOutput => Span::styled(format!("{:>9}", "Out $/M"), style),
        PriceBlended => Span::styled(format!("{:>9}", "Bld $/M"), style),
        ReleaseDate => Span::styled(format!("{:>11}", "Released"), style),
    }
}

/// Render a column value span for the given sort column
fn benchmark_col_value<'a>(
    entry: &crate::benchmarks::BenchmarkEntry,
    col: super::benchmarks_app::BenchmarkSortColumn,
    style: Style,
    name_width: usize,
) -> Span<'a> {
    use super::benchmarks_app::BenchmarkSortColumn::*;
    match col {
        Name => Span::styled(
            format!(
                "{:<width$}",
                truncate(&entry.name, name_width.saturating_sub(1)),
                width = name_width
            ),
            style,
        ),
        Intelligence => Span::styled(fmt_col_idx(entry.intelligence_index), style),
        Coding => Span::styled(fmt_col_idx(entry.coding_index), style),
        Math => Span::styled(fmt_col_idx(entry.math_index), style),
        Gpqa => Span::styled(fmt_col_pct(entry.gpqa), style),
        MMLUPro => Span::styled(fmt_col_pct(entry.mmlu_pro), style),
        Hle => Span::styled(fmt_col_pct(entry.hle), style),
        LiveCode => Span::styled(fmt_col_pct(entry.livecodebench), style),
        SciCode => Span::styled(fmt_col_pct(entry.scicode), style),
        Terminal => Span::styled(fmt_col_pct(entry.terminalbench_hard), style),
        IFBench => Span::styled(fmt_col_pct(entry.ifbench), style),
        Lcr => Span::styled(fmt_col_pct(entry.lcr), style),
        Tau2 => Span::styled(fmt_col_pct(entry.tau2), style),
        Speed => Span::styled(fmt_speed(entry.output_tps), style),
        Ttft => Span::styled(fmt_col_ttft(entry.ttft), style),
        Ttfat => Span::styled(fmt_col_ttft(entry.ttfat), style),
        PriceInput => Span::styled(fmt_col_price(entry.price_input), style),
        PriceOutput => Span::styled(fmt_col_price(entry.price_output), style),
        PriceBlended => Span::styled(fmt_col_price(entry.price_blended), style),
        ReleaseDate => Span::styled(fmt_col_date(entry.release_date.as_deref()), style),
    }
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
                            Span::styled(" s ", Style::default().fg(Color::Yellow)),
                            Span::raw("sort  "),
                            Span::styled(" / ", Style::default().fg(Color::Yellow)),
                            Span::raw("search  "),
                            Span::styled(" Space ", Style::default().fg(Color::Yellow)),
                            Span::raw("select"),
                        ])
                    }
                }
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
                Tab::Models => &app.search_query,
                Tab::Agents => app
                    .agents_app
                    .as_ref()
                    .map(|a| &a.search_query)
                    .unwrap_or(&app.search_query),
                Tab::Benchmarks => &app.benchmarks_app.search_query,
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

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn format_filters(filters: &Filters, category: ProviderCategory) -> String {
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
    if filters.free {
        active.push("free");
    }
    if category != ProviderCategory::All {
        active.push(category.label());
    }
    active.join(", ")
}

fn draw_help_popup(f: &mut Frame, scroll: u16, current_tab: Tab) {
    let area = centered_rect(50, 70, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    let mut help_text = vec![
        // Common: Navigation
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
        // Common: Panels
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
        // Common: Search
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
    ];

    // Tab-specific sections
    match current_tab {
        Tab::Models => {
            help_text.extend(vec![
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
                    Span::styled("  S             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle sort direction"),
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
                Line::from(vec![
                    Span::styled("  4             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle free models filter"),
                ]),
                Line::from(vec![
                    Span::styled("  5             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle provider category filter"),
                ]),
                Line::from(vec![
                    Span::styled("  6             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle category grouping"),
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
            ]);
        }
        Tab::Agents => {
            help_text.extend(vec![
                Line::from(Span::styled(
                    "Filters & Sort",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("  s             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle sort (name → updated → stars → status)"),
                ]),
                Line::from(vec![
                    Span::styled("  1             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle installed filter"),
                ]),
                Line::from(vec![
                    Span::styled("  2             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle CLI filter"),
                ]),
                Line::from(vec![
                    Span::styled("  3             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle open source filter"),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Actions",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("  o             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Open docs in browser"),
                ]),
                Line::from(vec![
                    Span::styled("  r             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Open GitHub repo in browser"),
                ]),
                Line::from(vec![
                    Span::styled("  c             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Copy agent name"),
                ]),
                Line::from(vec![
                    Span::styled("  a             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Add/remove tracked agents"),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Status Indicators",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("  ○             ", Style::default().fg(Color::DarkGray)),
                    Span::raw("Not tracked"),
                ]),
                Line::from(vec![
                    Span::styled("  ◐             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Loading GitHub data"),
                ]),
                Line::from(vec![
                    Span::styled("  ●             ", Style::default().fg(Color::Green)),
                    Span::raw("Up to date"),
                ]),
                Line::from(vec![
                    Span::styled("  ●             ", Style::default().fg(Color::Blue)),
                    Span::raw("Update available"),
                ]),
                Line::from(vec![
                    Span::styled("  ✗             ", Style::default().fg(Color::Red)),
                    Span::raw("Fetch failed"),
                ]),
                Line::from(""),
            ]);
        }
        Tab::Benchmarks => {
            help_text.extend(vec![
                Line::from(Span::styled(
                    "Quick Sort (press again to flip direction)",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("  1             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Sort by Intelligence index"),
                ]),
                Line::from(vec![
                    Span::styled("  2             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Sort by Release date"),
                ]),
                Line::from(vec![
                    Span::styled("  3             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Sort by Speed (tok/s)"),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Filters",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("  4             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle source filter (Open/Closed/Mixed)"),
                ]),
                Line::from(vec![
                    Span::styled("  5             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle region filter (US/China/Europe/...)"),
                ]),
                Line::from(vec![
                    Span::styled("  6             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle type filter (Startup/Big Tech/Research)"),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Sort (full cycle)",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("  s             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle through all sort columns"),
                ]),
                Line::from(vec![
                    Span::styled("  S             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle sort direction"),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Actions",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("  o             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Open Artificial Analysis page"),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Compare",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("  Space         ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle model for comparison (max 8)"),
                ]),
                Line::from(vec![
                    Span::styled("  c             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Clear all selections"),
                ]),
                Line::from(vec![
                    Span::styled("  v             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle view: H2H → Scatter → Radar"),
                ]),
                Line::from(vec![
                    Span::styled("  d             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Show detail overlay (H2H view)"),
                ]),
                Line::from(vec![
                    Span::styled("  x             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle scatter X-axis"),
                ]),
                Line::from(vec![
                    Span::styled("  y             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle scatter Y-axis"),
                ]),
                Line::from(vec![
                    Span::styled("  a             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Cycle radar preset"),
                ]),
                Line::from(vec![
                    Span::styled("  j/k           ", Style::default().fg(Color::Yellow)),
                    Span::raw("Scroll H2H table (when Compare focused)"),
                ]),
                Line::from(vec![
                    Span::styled("  h/l           ", Style::default().fg(Color::Yellow)),
                    Span::raw("Switch focus: List ↔ Compare"),
                ]),
                Line::from(vec![
                    Span::styled("  t             ", Style::default().fg(Color::Yellow)),
                    Span::raw("Toggle left panel: Models ↔ Creators"),
                ]),
                Line::from(""),
            ]);
        }
    }

    // Common: Tabs and Other
    help_text.extend(vec![
        Line::from(Span::styled(
            "Tabs",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  [             ", Style::default().fg(Color::Yellow)),
            Span::raw("Previous tab"),
        ]),
        Line::from(vec![
            Span::styled("  ]             ", Style::default().fg(Color::Yellow)),
            Span::raw("Next tab"),
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
        Line::from(vec![
            Span::styled("  ?             ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle this help"),
        ]),
    ]);

    let title = match current_tab {
        Tab::Models => " Models Help - ? or Esc to close (j/k to scroll) ",
        Tab::Agents => " Agents Help - ? or Esc to close (j/k to scroll) ",
        Tab::Benchmarks => " Benchmarks Help - ? or Esc to close (j/k to scroll) ",
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
}

fn draw_picker_modal(f: &mut Frame, app: &App) {
    let agents_app = match &app.agents_app {
        Some(a) => a,
        None => return,
    };

    let num_agents = agents_app.entries.len();

    // Calculate popup dimensions
    // Width: 60 chars or screen width - 4, whichever is smaller
    let popup_width = std::cmp::min(60, f.area().width.saturating_sub(4));
    // Height: num agents + 4 (for borders and title/footer)
    let popup_height = std::cmp::min((num_agents + 4) as u16, f.area().height.saturating_sub(4));

    // Center the popup
    let area = centered_rect_fixed(popup_width, popup_height, f.area());

    // Clear the background
    f.render_widget(Clear, area);

    // Build list items with checkboxes
    let items: Vec<ListItem> = agents_app
        .entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            // Get tracked state from picker_changes, fallback to entry.tracked
            let is_tracked = agents_app
                .picker_changes
                .get(&entry.id)
                .copied()
                .unwrap_or(entry.tracked);

            let checkbox = if is_tracked { "[x]" } else { "[ ]" };

            // Get first category or empty
            let category = entry
                .agent
                .categories
                .first()
                .map(|c| c.as_str())
                .unwrap_or("");

            // Installed status
            let installed_status = if entry.installed.version.is_some() {
                "installed"
            } else {
                ""
            };

            // Build the line with styled spans
            let line = Line::from(vec![
                Span::raw(format!("{} ", checkbox)),
                Span::styled(
                    format!("{:<20}", truncate(&entry.agent.name, 20)),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {:<10}", truncate(category, 10)),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!(" {}", installed_status),
                    Style::default().fg(Color::Green),
                ),
            ]);

            // Highlight selected row
            if idx == agents_app.picker_selected {
                ListItem::new(line).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Add/Remove Tracked Agents ")
                .title_bottom(Line::from(" Space: toggle | Enter: save | Esc: cancel ").centered()),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    // Create a ListState for proper scrolling
    let mut list_state = ListState::default();
    list_state.select(Some(agents_app.picker_selected));

    f.render_stateful_widget(list, area, &mut list_state);
}

// ── H2H comparison table ────────────────────────────────────────────────────

struct MetricDef {
    label: &'static str,
    extract: fn(&crate::benchmarks::BenchmarkEntry) -> Option<f64>,
    format: fn(f64) -> String,
    higher_is_better: bool,
}

fn fmt_h2h_index(v: f64) -> String {
    format!("{:.1}", v)
}

fn fmt_h2h_pct(v: f64) -> String {
    format!("{:.1}%", v * 100.0)
}

fn fmt_h2h_speed(v: f64) -> String {
    format!("{:.0}", v)
}

fn fmt_h2h_latency(v: f64) -> String {
    format!("{:.0}ms", v)
}

fn fmt_h2h_price(v: f64) -> String {
    format!("${:.2}", v)
}

/// A section header or a metric row in the H2H table.
enum H2HRow {
    Section(&'static str),
    Metric(MetricDef),
}

fn h2h_rows() -> Vec<H2HRow> {
    vec![
        // Indexes (0-100, higher better)
        H2HRow::Section("Indexes (0\u{2013}100)"),
        H2HRow::Metric(MetricDef {
            label: "Intelligence",
            extract: |e| e.intelligence_index,
            format: fmt_h2h_index,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "Coding",
            extract: |e| e.coding_index,
            format: fmt_h2h_index,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "Math",
            extract: |e| e.math_index,
            format: fmt_h2h_index,
            higher_is_better: true,
        }),
        // Benchmarks (%, higher better)
        H2HRow::Section("Benchmarks (%)"),
        H2HRow::Metric(MetricDef {
            label: "GPQA",
            extract: |e| e.gpqa,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "MMLU-Pro",
            extract: |e| e.mmlu_pro,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "HLE",
            extract: |e| e.hle,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "MATH-500",
            extract: |e| e.math_500,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "AIME",
            extract: |e| e.aime,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "AIME'25",
            extract: |e| e.aime_25,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "LiveCodeBench",
            extract: |e| e.livecodebench,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "SciCode",
            extract: |e| e.scicode,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "IFBench",
            extract: |e| e.ifbench,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "Terminal",
            extract: |e| e.terminalbench_hard,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "Tau2",
            extract: |e| e.tau2,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "LCR",
            extract: |e| e.lcr,
            format: fmt_h2h_pct,
            higher_is_better: true,
        }),
        // Performance (speed ↑, latency ↓)
        H2HRow::Section("Performance"),
        H2HRow::Metric(MetricDef {
            label: "Speed (tok/s)",
            extract: |e| e.output_tps,
            format: fmt_h2h_speed,
            higher_is_better: true,
        }),
        H2HRow::Metric(MetricDef {
            label: "TTFT (ms)",
            extract: |e| e.ttft,
            format: fmt_h2h_latency,
            higher_is_better: false,
        }),
        H2HRow::Metric(MetricDef {
            label: "TTFAT (ms)",
            extract: |e| e.ttfat,
            format: fmt_h2h_latency,
            higher_is_better: false,
        }),
        // Pricing ($/M tokens, lower better)
        H2HRow::Section("Pricing ($/M)"),
        H2HRow::Metric(MetricDef {
            label: "Input",
            extract: |e| e.price_input,
            format: fmt_h2h_price,
            higher_is_better: false,
        }),
        H2HRow::Metric(MetricDef {
            label: "Output",
            extract: |e| e.price_output,
            format: fmt_h2h_price,
            higher_is_better: false,
        }),
        H2HRow::Metric(MetricDef {
            label: "Blended",
            extract: |e| e.price_blended,
            format: fmt_h2h_price,
            higher_is_better: false,
        }),
    ]
}

/// Rank extracted values: 1 = best, None for missing data.
fn rank_values(values: &[Option<f64>], higher_is_better: bool) -> Vec<Option<u32>> {
    let mut indexed: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|val| (i, val)))
        .collect();

    if higher_is_better {
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    let mut ranks = vec![None; values.len()];
    for (rank, (idx, _)) in indexed.iter().enumerate() {
        ranks[*idx] = Some(rank as u32 + 1);
    }
    ranks
}

fn draw_h2h_table_generic(f: &mut Frame, area: Rect, app: &App) {
    let entries = app.benchmark_store.entries();
    let selections = &app.selections;

    if selections.len() < 2 {
        return;
    }

    let is_focused = app.benchmarks_app.focus == super::benchmarks_app::BenchmarkFocus::Compare;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Head-to-Head ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 20 || inner.height < 3 {
        return;
    }

    let rows = h2h_rows();
    let label_w = 14_u16;
    let num_models = selections.len();
    let available = inner.width.saturating_sub(label_w);
    let col_w = (available as usize / num_models).max(10);
    let total_w = inner.width as usize;

    // Header row: model names
    let mut header_spans: Vec<Span> = vec![Span::styled(
        format!("{:<width$}", "", width = label_w as usize),
        Style::default(),
    )];
    for (i, &store_idx) in selections.iter().enumerate() {
        let name = entries
            .get(store_idx)
            .map(|e| e.name.as_str())
            .unwrap_or("?");
        let color = compare_colors(i);
        let truncated = if name.len() > col_w - 1 {
            format!("{:.width$}", name, width = col_w - 2)
        } else {
            name.to_string()
        };
        header_spans.push(Span::styled(
            format!("{:>width$}", truncated, width = col_w),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }

    let mut lines: Vec<Line> = vec![Line::from(header_spans)];

    // Separator
    let sep = "\u{2500}".repeat(total_w);
    lines.push(Line::from(Span::styled(
        sep,
        Style::default().fg(Color::DarkGray),
    )));

    // ── Pre-compute win counts (need them near the top) ──
    let mut win_counts = vec![0u32; num_models];
    for row in &rows {
        if let H2HRow::Metric(metric) = row {
            let values: Vec<Option<f64>> = selections
                .iter()
                .map(|&idx| entries.get(idx).and_then(|e| (metric.extract)(e)))
                .collect();
            let ranks = rank_values(&values, metric.higher_is_better);
            for (i, rank) in ranks.iter().enumerate() {
                if *rank == Some(1) {
                    win_counts[i] += 1;
                }
            }
        }
    }

    // ── Win count (right under model names) ──
    let mut wins_spans: Vec<Span> = vec![Span::styled(
        format!("{:<width$}", "\u{2605} Wins", width = label_w as usize),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )];
    let max_wins = win_counts.iter().copied().max().unwrap_or(0);
    for (i, &count) in win_counts.iter().enumerate() {
        let color = compare_colors(i);
        let label = if count == max_wins && max_wins > 0 {
            format!("{} \u{2605}", count)
        } else {
            format!("{}", count)
        };
        let style = if count == max_wins && max_wins > 0 {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };
        wins_spans.push(Span::styled(
            format!("{:>width$}", label, width = col_w),
            style,
        ));
    }
    lines.push(Line::from(wins_spans));

    // ── Model Info section ──
    let info_header = "\u{2500}\u{2500}\u{2500} Model Info \u{2500}".to_string();
    lines.push(Line::from(Span::styled(
        format!("{:<width$}", info_header, width = total_w),
        Style::default().fg(Color::DarkGray),
    )));

    // Helper to render an info row with per-value colors
    let render_info_row = |lines: &mut Vec<Line>, label: &str, values: Vec<(String, Color)>| {
        let mut spans: Vec<Span> = vec![Span::styled(
            format!("{:<width$}", label, width = label_w as usize),
            Style::default().fg(Color::DarkGray),
        )];
        for (val, color) in values.iter() {
            let truncated = if val.len() > col_w - 1 {
                format!("{:.width$}", val, width = col_w - 2)
            } else {
                val.clone()
            };
            spans.push(Span::styled(
                format!("{:>width$}", truncated, width = col_w),
                Style::default().fg(*color),
            ));
        }
        lines.push(Line::from(spans));
    };

    // Creator
    let creators: Vec<(String, Color)> = selections
        .iter()
        .map(|&idx| {
            let name = entries
                .get(idx)
                .map(|e| {
                    if !e.creator_name.is_empty() {
                        e.creator_name.clone()
                    } else {
                        e.creator.clone()
                    }
                })
                .unwrap_or_default();
            (name, Color::White)
        })
        .collect();
    render_info_row(&mut lines, "Creator", creators);

    // Source (Open/Closed) with color
    let sources: Vec<(String, Color)> = selections
        .iter()
        .map(|&idx| {
            entries
                .get(idx)
                .and_then(|e| app.open_weights_map.get(&e.slug))
                .map(|&open| {
                    if open {
                        ("Open".to_string(), Color::Green)
                    } else {
                        ("Closed".to_string(), Color::Red)
                    }
                })
                .unwrap_or_else(|| ("\u{2014}".to_string(), Color::DarkGray))
        })
        .collect();
    render_info_row(&mut lines, "Source", sources);

    // Region with creator region colors
    let regions: Vec<(String, Color)> = selections
        .iter()
        .map(|&idx| {
            entries
                .get(idx)
                .map(|e| {
                    let region = super::benchmarks_app::CreatorRegion::from_creator(&e.creator);
                    (region.label().to_string(), region.color())
                })
                .unwrap_or_default()
        })
        .collect();
    render_info_row(&mut lines, "Region", regions);

    // Type with creator type colors
    let types: Vec<(String, Color)> = selections
        .iter()
        .map(|&idx| {
            entries
                .get(idx)
                .map(|e| {
                    let ct = super::benchmarks_app::CreatorType::from_creator(&e.creator);
                    (ct.label().to_string(), ct.color())
                })
                .unwrap_or_default()
        })
        .collect();
    render_info_row(&mut lines, "Type", types);

    // Release date
    let dates: Vec<(String, Color)> = selections
        .iter()
        .map(|&idx| {
            let d = entries
                .get(idx)
                .and_then(|e| e.release_date.clone())
                .unwrap_or_else(|| "\u{2014}".to_string());
            (d, Color::White)
        })
        .collect();
    render_info_row(&mut lines, "Released", dates);

    // ── Metric rows with section headers and ranks ──
    for row in &rows {
        match row {
            H2HRow::Section(title) => {
                let header = format!("\u{2500}\u{2500}\u{2500} {} \u{2500}", title);
                lines.push(Line::from(Span::styled(
                    format!("{:<width$}", header, width = total_w),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            H2HRow::Metric(metric) => {
                let values: Vec<Option<f64>> = selections
                    .iter()
                    .map(|&idx| entries.get(idx).and_then(|e| (metric.extract)(e)))
                    .collect();
                let ranks = rank_values(&values, metric.higher_is_better);

                let mut row_spans: Vec<Span> = vec![Span::styled(
                    format!("{:<width$}", metric.label, width = label_w as usize),
                    Style::default().fg(Color::DarkGray),
                )];

                for (i, (val, rank)) in values.iter().zip(ranks.iter()).enumerate() {
                    let color = compare_colors(i);
                    match val {
                        Some(v) => {
                            let formatted = (metric.format)(*v);
                            if *rank == Some(1) {
                                // Best: value ★
                                let value_and_star = format!("{} \u{2605}", formatted);
                                let padded = format!("{:>width$}", value_and_star, width = col_w);
                                let star_pos = padded.rfind('\u{2605}').unwrap_or(padded.len());
                                row_spans.push(Span::styled(
                                    padded[..star_pos].to_string(),
                                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                                ));
                                row_spans.push(Span::styled(
                                    "\u{2605}",
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                ));
                            } else {
                                // Non-best: value in model color, rank in medal colors
                                let rank_num = rank.unwrap_or(0);
                                let suffix = format!(" #{}", rank_num);
                                let rank_color = match rank_num {
                                    2 => Color::Indexed(250), // silver
                                    3 => Color::Indexed(172), // bronze
                                    _ => Color::DarkGray,
                                };

                                let combined = format!("{}{}", formatted, suffix);
                                let padded = format!("{:>width$}", combined, width = col_w);
                                let suffix_start = padded.len().saturating_sub(suffix.len());
                                row_spans.push(Span::styled(
                                    padded[..suffix_start].to_string(),
                                    Style::default().fg(color),
                                ));
                                row_spans.push(Span::styled(
                                    padded[suffix_start..].to_string(),
                                    Style::default().fg(rank_color),
                                ));
                            }
                        }
                        None => {
                            row_spans.push(Span::styled(
                                format!("{:>width$}", "\u{2014}", width = col_w),
                                Style::default().fg(Color::DarkGray),
                            ));
                        }
                    }
                }

                lines.push(Line::from(row_spans));
            }
        }
    }

    let max_scroll = lines.len().saturating_sub(inner.height as usize);
    let scroll_y = app.benchmarks_app.h2h_scroll.min(max_scroll);
    let paragraph = Paragraph::new(lines).scroll((scroll_y as u16, 0));
    f.render_widget(paragraph, inner);
}

fn draw_scatter(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::symbols::Marker;
    use ratatui::widgets::{Axis, Chart, Dataset, GraphType};

    let entries = app.benchmark_store.entries();
    if entries.is_empty() {
        let block = Block::default().borders(Borders::ALL).title(" Scatter ");
        f.render_widget(block, area);
        return;
    }

    let x_extract = app.benchmarks_app.scatter_x.extract();
    let y_extract = app.benchmarks_app.scatter_y.extract();

    // Collect all points with both x and y values present
    let mut all_points: Vec<(f64, f64)> = Vec::new();
    for entry in entries.iter() {
        if let (Some(x), Some(y)) = (x_extract(entry), y_extract(entry)) {
            all_points.push((x, y));
        }
    }

    if all_points.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Scatter (no data) ");
        f.render_widget(block, area);
        return;
    }

    // Split area: chart on top, legend row at bottom (if selections exist)
    let has_selections = !app.selections.is_empty();
    let (chart_area, legend_area) = if has_selections {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Auto log scale for skewed axes
    let f64_cmp = |a: &f64, b: &f64| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal);
    let mut x_vals: Vec<f64> = all_points.iter().map(|p| p.0).collect();
    let mut y_vals: Vec<f64> = all_points.iter().map(|p| p.1).collect();
    x_vals.sort_by(f64_cmp);
    y_vals.sort_by(f64_cmp);

    fn is_skewed(sorted: &[f64]) -> bool {
        if sorted.len() < 5 {
            return false;
        }
        let mid = sorted[sorted.len() / 2];
        let max = sorted[sorted.len() - 1];
        mid > 0.0 && max / mid > 5.0
    }

    let x_log = is_skewed(&x_vals);
    let y_log = is_skewed(&y_vals);

    let log_transform = |v: f64, use_log: bool| -> f64 {
        if use_log {
            (v.max(0.001)).ln()
        } else {
            v
        }
    };

    let display_points: Vec<(f64, f64)> = all_points
        .iter()
        .map(|&(x, y)| (log_transform(x, x_log), log_transform(y, y_log)))
        .collect();

    let x_min = display_points
        .iter()
        .map(|p| p.0)
        .fold(f64::INFINITY, f64::min);
    let x_max = display_points
        .iter()
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let y_min = display_points
        .iter()
        .map(|p| p.1)
        .fold(f64::INFINITY, f64::min);
    let y_max = display_points
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);

    // Snap non-log bounds to nice round numbers so ticks land on whole values.
    let nice_bounds = |lo: f64, hi: f64, num_ticks: usize| -> [f64; 2] {
        let range = hi - lo;
        let raw_step = range / (num_ticks - 1) as f64;
        let mag = 10_f64.powf(raw_step.log10().floor());
        let nice_step = if raw_step / mag < 1.5 {
            mag
        } else if raw_step / mag < 3.5 {
            mag * 2.0
        } else if raw_step / mag < 7.5 {
            mag * 5.0
        } else {
            mag * 10.0
        };
        let nice_lo = (lo / nice_step).floor() * nice_step;
        let nice_hi = (hi / nice_step).ceil() * nice_step;
        [nice_lo.max(0.0), nice_hi]
    };

    let x_pad = (x_max - x_min).max(0.1) * 0.05;
    let y_pad = (y_max - y_min).max(0.1) * 0.05;
    let num_ticks = 7_usize;
    let x_bounds = if x_log {
        [x_min - x_pad, x_max + x_pad]
    } else {
        nice_bounds(x_min - x_pad, x_max + x_pad, num_ticks)
    };
    let y_bounds = if y_log {
        [y_min - y_pad, y_max + y_pad]
    } else {
        nice_bounds(y_min - y_pad, y_max + y_pad, num_ticks)
    };

    // Compute independent averages (each axis uses all entries with data for that metric)
    let (x_sum, x_count) = entries.iter().fold((0.0_f64, 0_u32), |(s, c), e| {
        if let Some(v) = x_extract(e) {
            (s + log_transform(v, x_log), c + 1)
        } else {
            (s, c)
        }
    });
    let (y_sum, y_count) = entries.iter().fold((0.0_f64, 0_u32), |(s, c), e| {
        if let Some(v) = y_extract(e) {
            (s + log_transform(v, y_log), c + 1)
        } else {
            (s, c)
        }
    });
    let avg_x = if x_count > 0 {
        x_sum / x_count as f64
    } else {
        (x_bounds[0] + x_bounds[1]) / 2.0
    };
    let avg_y = if y_count > 0 {
        y_sum / y_count as f64
    } else {
        (y_bounds[0] + y_bounds[1]) / 2.0
    };

    let v_line = vec![(avg_x, y_bounds[0]), (avg_x, y_bounds[1])];
    let h_line = vec![(x_bounds[0], avg_y), (x_bounds[1], avg_y)];

    // Build selected model point sets + legend
    let mut legend_entries: Vec<(String, Color, u8)> = Vec::new();
    #[allow(clippy::type_complexity)]
    let mut selected_data: Vec<(String, Vec<(f64, f64)>, Color)> = Vec::new();

    for (sel_idx, &store_idx) in app.selections.iter().enumerate() {
        let color = compare_colors(sel_idx);
        if let Some(entry) = entries.get(store_idx) {
            let name = truncate(&entry.name, 20);
            if let (Some(x), Some(y)) = (x_extract(entry), y_extract(entry)) {
                let tx = log_transform(x, x_log);
                let ty = log_transform(y, y_log);
                let in_range = tx >= x_bounds[0]
                    && tx <= x_bounds[1]
                    && ty >= y_bounds[0]
                    && ty <= y_bounds[1];
                selected_data.push((entry.name.clone(), vec![(tx, ty)], color));
                legend_entries.push((name, color, if in_range { 1 } else { 2 }));
            } else {
                legend_entries.push((name, color, 0));
            }
        }
    }

    // Build datasets — crosshairs, background, then selected
    let mut datasets = vec![
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Indexed(242)))
            .data(&v_line),
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Indexed(242)))
            .data(&h_line),
        Dataset::default()
            .marker(Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Color::DarkGray))
            .data(&display_points),
    ];

    for (_, points, color) in &selected_data {
        datasets.push(
            Dataset::default()
                .marker(Marker::HalfBlock)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(*color))
                .data(points),
        );
    }

    let x_label = app.benchmarks_app.scatter_x.label();
    let y_label = app.benchmarks_app.scatter_y.label();

    // Generate evenly-spaced tick labels for an axis.
    // ratatui distributes labels uniformly across the axis, so values must be evenly spaced.
    let make_ticks = |lo: f64, hi: f64, use_log: bool, n: usize| -> Vec<String> {
        let n = n.max(2);
        let step = (hi - lo) / (n - 1) as f64;
        let raw: Vec<f64> = (0..n).map(|i| lo + step * i as f64).collect();

        if use_log {
            // Format log-scale ticks: convert back to real values, ensure no duplicates
            let reals: Vec<f64> = raw.iter().map(|v| v.exp()).collect();
            // Pick precision that avoids duplicate labels
            for decimals in 0..=3 {
                let labels: Vec<String> = reals
                    .iter()
                    .map(|v| {
                        if decimals == 0 && *v >= 1.0 {
                            format!("{}", v.round() as i64)
                        } else {
                            format!("{:.prec$}", v, prec = decimals)
                        }
                    })
                    .collect();
                let unique: std::collections::HashSet<&String> = labels.iter().collect();
                if unique.len() == labels.len() {
                    return labels;
                }
            }
            // Fallback: 3 decimal places
            reals.iter().map(|v| format!("{:.3}", v)).collect()
        } else {
            raw.iter()
                .map(|v| {
                    if v.fract().abs() < 0.01 {
                        format!("{}", v.round() as i64)
                    } else {
                        format!("{:.1}", v)
                    }
                })
                .collect()
        }
    };

    let x_ticks = make_ticks(x_bounds[0], x_bounds[1], x_log, num_ticks);
    let y_ticks = make_ticks(y_bounds[0], y_bounds[1], y_log, num_ticks);

    let x_suffix = if x_log { " [log]" } else { "" };
    let y_suffix = if y_log { " [log]" } else { "" };

    // Format average for display (use original scale for log axes)
    let fmt_avg = |avg: f64, use_log: bool| -> String {
        let v = if use_log { avg.exp() } else { avg };
        if v >= 100.0 {
            format!("{}", v.round() as i64)
        } else {
            format!("{:.1}", v)
        }
    };
    let avg_style = Style::default().fg(Color::Indexed(242));

    let x_title = Line::from(vec![
        Span::styled(
            format!("{x_label}{x_suffix}"),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(format!("  avg:{}", fmt_avg(avg_x, x_log)), avg_style),
    ]);
    let y_title = Line::from(vec![
        Span::styled(
            format!("{y_label}{y_suffix}"),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(format!("  avg:{}", fmt_avg(avg_y, y_log)), avg_style),
    ]);

    let compare_focused =
        app.benchmarks_app.focus == super::benchmarks_app::BenchmarkFocus::Compare;
    let scatter_border = if compare_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(scatter_border))
                .title(format!(" {y_label} vs {x_label} ")),
        )
        .x_axis(
            Axis::default()
                .title(x_title)
                .style(Style::default().fg(Color::Gray))
                .bounds(x_bounds)
                .labels(x_ticks),
        )
        .y_axis(
            Axis::default()
                .title(y_title)
                .style(Style::default().fg(Color::Gray))
                .bounds(y_bounds)
                .labels(y_ticks),
        )
        .legend_position(None);

    f.render_widget(chart, chart_area);

    // Custom legend row below the chart
    if let Some(leg_area) = legend_area {
        let mut spans: Vec<Span> = vec![Span::raw(" ")];
        for (i, (name, color, status)) in legend_entries.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            match status {
                1 => {
                    spans.push(Span::styled("\u{25cf} ", Style::default().fg(*color)));
                    spans.push(Span::styled(name.as_str(), Style::default().fg(*color)));
                }
                2 => {
                    spans.push(Span::styled("\u{25cf} ", Style::default().fg(*color)));
                    spans.push(Span::styled(
                        format!("{name} (off-chart)"),
                        Style::default().fg(*color),
                    ));
                }
                _ => {
                    spans.push(Span::styled(
                        "\u{25cb} ",
                        Style::default().fg(Color::DarkGray),
                    ));
                    spans.push(Span::styled(
                        format!("{name} (no data)"),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }
        }
        f.render_widget(Paragraph::new(Line::from(spans)), leg_area);
    }
}

/// Create a centered rect using fixed width and height
fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
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
