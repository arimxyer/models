use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::app::{App, Filters, Focus, Mode, SortOrder, Tab};
use crate::agents::{format_stars, FetchStatus};

pub fn draw(f: &mut Frame, app: &mut App) {
    let (main_constraint, detail_constraint) = match app.current_tab {
        Tab::Models => (Constraint::Min(0), Constraint::Length(14)),
        Tab::Agents => (Constraint::Min(0), Constraint::Length(0)), // No bottom detail for Agents
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

fn draw_agents_main(f: &mut Frame, area: Rect, app: &mut App) {
    if app.agents_app.is_none() {
        let msg = Paragraph::new("Failed to load agents data")
            .block(Block::default().borders(Borders::ALL).title(" Agents "));
        f.render_widget(msg, area);
        return;
    }

    // Two-panel layout: List (35%) | Details (65%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_agent_list(f, chunks[0], app);
    draw_agent_detail(f, chunks[1], app);
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

    // Build title with count and sort indicator
    let sort_indicator = format!(" \u{2193}{}", agents_app.sort_order.label());
    let filter_indicator = agents_app.format_active_filters();
    let title = if filter_indicator.is_empty() {
        format!(
            " Agents ({}){} ",
            agents_app.filtered_entries.len(),
            sort_indicator
        )
    } else {
        format!(
            " Agents ({}) [{}]{} ",
            agents_app.filtered_entries.len(),
            filter_indicator,
            sort_indicator
        )
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

    // Header row (with status column)
    let header = format!("{:<22} {:>6} {:>8} {:>3}", "Agent", "Type", "Installed", "");
    items.push(
        ListItem::new(header).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        ),
    );

    // Agent rows
    for &idx in &agents_app.filtered_entries {
        if let Some(entry) = agents_app.entries.get(idx) {
            let agent_type = if entry.agent.categories.contains(&"cli".to_string()) {
                "CLI"
            } else if entry.agent.categories.contains(&"ide".to_string()) {
                "IDE"
            } else {
                "-"
            };

            let installed = if entry.installed.version.is_some() {
                "\u{2713}" // checkmark
            } else {
                "-"
            };

            // Status indicator based on fetch_status and update availability
            let (status_indicator, status_style) = match &entry.fetch_status {
                FetchStatus::NotStarted => ("\u{25CB}", Style::default().fg(Color::DarkGray)), // ○ gray
                FetchStatus::Loading => ("\u{25D0}", Style::default().fg(Color::Yellow)),      // ◐ yellow
                FetchStatus::Loaded => {
                    if entry.update_available() {
                        ("\u{25CF}", Style::default().fg(Color::Blue)) // ● blue = update available
                    } else {
                        ("\u{25CF}", Style::default().fg(Color::Green)) // ● green = up to date
                    }
                }
                FetchStatus::Failed(_) => ("\u{2717}", Style::default().fg(Color::Red)), // ✗ red
            };

            let row = Line::from(vec![
                Span::raw(format!(
                    "{:<22} {:>6} {:>8} ",
                    truncate(&entry.agent.name, 22),
                    agent_type,
                    installed,
                )),
                Span::styled(status_indicator, status_style),
            ]);
            items.push(ListItem::new(row));
        }
    }

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    // Offset by 1 for header row
    let mut state = agents_app.agent_list_state.clone();
    if let Some(selected) = state.selected() {
        state.select(Some(selected + 1));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);
}

fn draw_agent_detail(f: &mut Frame, area: Rect, app: &App) {
    use super::agents_app::AgentFocus;

    let agents_app = match &app.agents_app {
        Some(a) => a,
        None => return,
    };

    let is_focused = agents_app.focus == AgentFocus::Details;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let lines: Vec<Line> = if let Some(entry) = agents_app.current_entry() {
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
                    version_spans.push(Span::styled(
                        format!("  {}", date),
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
                    for line in changelog.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        // Basic bullet point detection
                        let formatted = if let Some(rest) = trimmed
                            .strip_prefix("- ")
                            .or_else(|| trimmed.strip_prefix("* "))
                        {
                            format!("  \u{2022} {}", rest)
                        } else if let Some(rest) = trimmed.strip_prefix("## ") {
                            rest.to_string()
                        } else if trimmed.starts_with('#') {
                            // Skip other headers like "# What's Changed"
                            continue;
                        } else {
                            format!("  {}", trimmed)
                        };
                        detail_lines.push(Line::from(formatted));
                    }
                }

                detail_lines.push(Line::from("")); // Space between releases
            }
        }

        // Keybinding hints at the bottom
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![
            Span::styled(" o ", Style::default().fg(Color::Yellow)),
            Span::raw("open docs  "),
            Span::styled(" r ", Style::default().fg(Color::Yellow)),
            Span::raw("open repo  "),
            Span::styled(" c ", Style::default().fg(Color::Yellow)),
            Span::raw("copy name"),
        ]));

        detail_lines
    } else {
        vec![Line::from(Span::styled(
            "Select an agent to view details",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    // Clamp scroll to content bounds (calculate clamped value without mutating)
    let visible_height = detail_visible_height(area);
    let content_lines = lines.len() as u16;
    let max_scroll = content_lines.saturating_sub(visible_height);
    let scroll_pos = agents_app.detail_scroll.min(max_scroll);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Details "),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_pos, 0));

    f.render_widget(paragraph, area);
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
                    Span::styled(" s ", Style::default().fg(Color::Yellow)),
                    Span::raw("sort  "),
                    Span::styled(" c ", Style::default().fg(Color::Yellow)),
                    Span::raw("copy"),
                ]),
                Tab::Agents => Line::from(vec![
                    Span::styled(" q ", Style::default().fg(Color::Yellow)),
                    Span::raw("quit  "),
                    Span::styled(" ↑/↓ ", Style::default().fg(Color::Yellow)),
                    Span::raw("nav  "),
                    Span::styled(" Tab ", Style::default().fg(Color::Yellow)),
                    Span::raw("switch  "),
                    Span::styled(" s ", Style::default().fg(Color::Yellow)),
                    Span::raw("sort  "),
                    Span::styled(" a ", Style::default().fg(Color::Yellow)),
                    Span::raw("track  "),
                    Span::styled(" o ", Style::default().fg(Color::Yellow)),
                    Span::raw("docs  "),
                    Span::styled(" r ", Style::default().fg(Color::Yellow)),
                    Span::raw("repo"),
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
                    Span::raw("Cycle sort (name → stars → updated)"),
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
