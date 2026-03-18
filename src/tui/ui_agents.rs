use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::app::App;
use super::ui::{
    caret, centered_rect_fixed, detail_visible_height, focus_border, render_scrollbar,
    selection_style, truncate,
};
use crate::agents::{format_stars, FetchStatus};
use crate::formatting::EM_DASH;

pub(super) fn draw_agents_main(f: &mut Frame, area: Rect, app: &mut App) {
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

fn draw_agent_list(f: &mut Frame, area: Rect, app: &mut App) {
    use super::agents_app::AgentFocus;

    let agents_app = match &mut app.agents_app {
        Some(a) => a,
        None => return,
    };

    let is_focused = agents_app.focus == AgentFocus::List;
    let border_style = focus_border(is_focused);

    // Build title with count, filter, and sort indicators
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
                EM_DASH
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
                (EM_DASH, Style::default().fg(Color::DarkGray))
            };

            let (prefix, text_style) = if is_selected {
                (caret(is_focused), selection_style(true))
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
    let mut state = agents_app.agent_list_state;
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

    let border_style = focus_border(is_focused);

    let mut match_line_indices: Vec<u16> = Vec::new();

    let lines: Vec<Line> = if let Some(entry) =
        app.agents_app.as_ref().and_then(|a| a.current_entry())
    {
        let mut detail_lines = Vec::new();

        // Header: Name + Version
        let name = entry.agent.name.clone();
        let version_str = entry.github.latest_version().unwrap_or(EM_DASH).to_string();
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

        let latest_release_date = entry
            .github
            .latest_release_date()
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "\u{2014}".to_string());
        let updated_str = entry
            .latest_release_relative_time()
            .unwrap_or_else(|| "\u{2014}".to_string());
        detail_lines.push(Line::from(vec![
            Span::styled("Latest release: ", Style::default().fg(Color::DarkGray)),
            Span::raw(latest_release_date),
            Span::styled(
                format!(" ({})", updated_str),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        detail_lines.push(Line::from(vec![
            Span::styled("Release cadence: ", Style::default().fg(Color::DarkGray)),
            Span::raw(entry.release_frequency()),
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
        let line_width = line.width();
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
        format!(
            " Details [/{} {}/{}] ",
            search_query,
            current_match_display + 1,
            match_count
        )
    } else if !search_query.is_empty() {
        format!(" Details [/{}] ", search_query)
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

    // Scrollbar for detail panel
    render_scrollbar(
        f,
        area,
        visual_total as usize,
        scroll_pos as usize,
        visible_height as usize,
        true,
    );

    // Update match state and detail height (after lines are consumed)
    app.last_detail_height = visible_height;
    if let Some(ref mut agents_app) = app.agents_app {
        agents_app.update_search_matches(match_line_indices, match_visual_offsets);
    }
}

pub(super) fn draw_picker_modal(f: &mut Frame, app: &App) {
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
