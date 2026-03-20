use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::compare::{draw_h2h_table_generic, draw_scatter};
use crate::formatting::format_tokens;
use crate::formatting::truncate;
use crate::tui::app::App;
use crate::tui::ui::{caret, centered_rect, centered_rect_fixed, focus_border};
use crate::tui::widgets::scrollable_panel::ScrollablePanel;

/// Color palette for selected models in comparison mode.
pub(crate) fn compare_colors(index: usize) -> Color {
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

pub(in crate::tui) fn draw_benchmarks_main(f: &mut Frame, area: Rect, app: &mut App) {
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
            super::app::BottomView::H2H => {
                draw_h2h_table_generic(f, v_chunks[1], app);
            }
            super::app::BottomView::Scatter => {
                draw_scatter(f, v_chunks[1], app);
            }
            super::app::BottomView::Radar => {
                super::radar::draw_radar(f, v_chunks[1], app);
            }
            super::app::BottomView::Detail => {
                draw_benchmark_detail(f, v_chunks[1], app);
            }
        }
    } else {
        // Browse mode: creators (20%) | list (40%) | detail (40%)
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(40),
                Constraint::Percentage(40),
            ])
            .split(area);

        draw_benchmark_creators(f, h_chunks[0], app);
        draw_benchmark_list(f, h_chunks[1], app);
        draw_benchmark_detail(f, h_chunks[2], app);
    }

    // Detail overlay (drawn last, on top of everything)
    if app.benchmarks_app.show_detail_overlay && app.selections.len() >= 2 {
        draw_detail_overlay(f, area, app);
    }

    // Sort picker popup
    if app.benchmarks_app.show_sort_picker {
        draw_sort_picker(f, area, &app.benchmarks_app);
    }
}

fn draw_benchmark_subtab_bar(f: &mut Frame, area: Rect, bench_app: &super::app::BenchmarksApp) {
    use super::app::BottomView;
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
    use super::app::{
        BenchmarkFocus, CreatorGrouping, CreatorListItem, CreatorRegion, CreatorType,
    };

    let bench_app = &mut app.benchmarks_app;
    let store = &app.benchmark_store;

    let is_focused = bench_app.focus == BenchmarkFocus::Creators;
    let border_style = focus_border(is_focused);

    let source_indicator = match bench_app.source_filter {
        super::app::SourceFilter::All => String::new(),
        filter => format!(" [{}]", filter.label()),
    };
    let reasoning_indicator = {
        let label = bench_app.reasoning_filter.label();
        if label.is_empty() {
            String::new()
        } else {
            format!(" [{}]", label)
        }
    };
    let creators_title = format!(" Creators{}{} ", source_indicator, reasoning_indicator);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(creators_title);
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
                    Span::styled(count_str, Style::default().fg(Color::Gray)),
                ];
                if let Some((label, color)) = tag {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(label, Style::default().fg(color)));
                }
                ListItem::new(Line::from(spans))
            }
        })
        .collect();

    let caret = caret(is_focused);
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(caret);

    let mut state = bench_app.creator_list_state;
    f.render_stateful_widget(list, chunks[1], &mut state);
}

/// Compact list for compare mode: selection marker + name only, full height.
fn draw_benchmark_list_compact(f: &mut Frame, area: Rect, app: &mut App) {
    use super::app::BenchmarkFocus;

    let bench_app = &mut app.benchmarks_app;
    let store = &app.benchmark_store;

    let is_focused = bench_app.focus == BenchmarkFocus::List;
    let border_style = focus_border(is_focused);

    let sort_dir = if bench_app.sort_descending {
        "\u{2193}"
    } else {
        "\u{2191}"
    };
    let sort_indicator = format!(" {}{}", sort_dir, bench_app.sort_column.label());

    let source_indicator = match bench_app.source_filter {
        super::app::SourceFilter::All => String::new(),
        filter => format!(" [{}]", filter.label()),
    };

    let reasoning_indicator = {
        let label = bench_app.reasoning_filter.label();
        if label.is_empty() {
            String::new()
        } else {
            format!(" [{}]", label)
        }
    };

    let loading_suffix = if bench_app.loading { " loading..." } else { "" };

    let title = if bench_app.search_query.is_empty() {
        format!(
            " Models ({}){}{}{}{} ",
            bench_app.filtered_indices.len(),
            source_indicator,
            reasoning_indicator,
            sort_indicator,
            loading_suffix
        )
    } else {
        format!(
            " Models ({}) [/{}]{}{}{}{} ",
            bench_app.filtered_indices.len(),
            bench_app.search_query,
            source_indicator,
            reasoning_indicator,
            sort_indicator,
            loading_suffix
        )
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let caret = caret(is_focused);
    let entries = store.entries();

    // Extra columns: marker(2) + caret(2) + reasoning(3) + source(2) + optional region/type
    let show_region = bench_app.creator_grouping == super::app::CreatorGrouping::ByRegion;
    let show_type = bench_app.creator_grouping == super::app::CreatorGrouping::ByType;
    let extra_w: u16 = 2 + 2 + 3 + 2 + if show_region || show_type { 4 } else { 0 };
    let name_width = inner_area.width.saturating_sub(extra_w) as usize;

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

            // Reasoning status indicator
            let (rs_label, rs_color) = match entry.reasoning_status {
                crate::benchmarks::ReasoningStatus::Reasoning => ("R  ", Color::Cyan),
                crate::benchmarks::ReasoningStatus::NonReasoning => ("NR ", Color::DarkGray),
                crate::benchmarks::ReasoningStatus::Adaptive => ("AR ", Color::Yellow),
                crate::benchmarks::ReasoningStatus::None => ("   ", Color::Reset),
            };
            row_spans.push(Span::styled(rs_label, Style::default().fg(rs_color)));

            // Source indicator (Open/Closed)
            let (src_label, src_color) = match app.open_weights_map.get(&entry.slug) {
                Some(true) => ("O ", Color::Green),
                Some(false) => ("C ", Color::Red),
                None => ("  ", Color::Reset),
            };
            row_spans.push(Span::styled(src_label, Style::default().fg(src_color)));

            // Region/Type indicator when grouping is active
            if show_region {
                let region = super::app::CreatorRegion::from_creator(&entry.creator);
                row_spans.push(Span::styled(
                    format!("{:<4}", region.short_label()),
                    Style::default().fg(region.color()),
                ));
            } else if show_type {
                let ct = super::app::CreatorType::from_creator(&entry.creator);
                row_spans.push(Span::styled(
                    format!("{:<4}", ct.short_label()),
                    Style::default().fg(ct.color()),
                ));
            }

            row_spans.push(Span::styled(
                truncate(&entry.display_name, name_width),
                style,
            ));
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

    let mut state = bench_app.list_state;
    state.select(Some(bench_app.selected));
    f.render_stateful_widget(list, inner_area, &mut state);
}

fn draw_benchmark_list(f: &mut Frame, area: Rect, app: &mut App) {
    use super::app::BenchmarkFocus;

    let bench_app = &mut app.benchmarks_app;
    let store = &app.benchmark_store;

    let is_focused = bench_app.focus == BenchmarkFocus::List;
    let border_style = focus_border(is_focused);

    let sort_dir = if bench_app.sort_descending {
        "\u{2193}"
    } else {
        "\u{2191}"
    };
    let sort_indicator = format!(" {}{}", sort_dir, bench_app.sort_column.label());

    let source_indicator = match bench_app.source_filter {
        super::app::SourceFilter::All => String::new(),
        filter => format!(" [{}]", filter.label()),
    };

    let reasoning_indicator = {
        let label = bench_app.reasoning_filter.label();
        if label.is_empty() {
            String::new()
        } else {
            format!(" [{}]", label)
        }
    };

    let creator_label = bench_app.selected_creator_name().unwrap_or("Benchmarks");
    let loading_suffix = if bench_app.loading { " loading..." } else { "" };

    let title = if bench_app.search_query.is_empty() {
        format!(
            " {} ({}){}{}{}{} ",
            creator_label,
            bench_app.filtered_indices.len(),
            source_indicator,
            reasoning_indicator,
            sort_indicator,
            loading_suffix
        )
    } else {
        format!(
            " {} ({}) [/{}]{}{}{}{} ",
            creator_label,
            bench_app.filtered_indices.len(),
            bench_app.search_query,
            source_indicator,
            reasoning_indicator,
            sort_indicator,
            loading_suffix
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

    // Compute dynamic name column width from available space
    let caret_w: u16 = 2;
    let reasoning_col_w: u16 = 3;
    let source_col_w: u16 = 2;
    let show_region = bench_app.creator_grouping == super::app::CreatorGrouping::ByRegion;
    let show_type = bench_app.creator_grouping == super::app::CreatorGrouping::ByType;
    let grouping_col_w: u16 = if show_region || show_type { 4 } else { 0 };
    let fixed_width: u16 = visible_cols
        .iter()
        .map(|col| benchmark_col_width(*col))
        .sum();
    let selection_w: u16 = if !app.selections.is_empty() { 2 } else { 0 };
    let name_width = (inner_area.width.saturating_sub(
        fixed_width + caret_w + selection_w + reasoning_col_w + source_col_w + grouping_col_w,
    ) as usize)
        .max(10);

    // Caret prefix for focused panel
    let caret = caret(is_focused);

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
    header_spans.push(Span::styled("   ", header_style)); // reasoning indicator
    header_spans.push(Span::styled("  ", header_style)); // source indicator
    if show_region {
        header_spans.push(Span::styled("Rgn ", header_style));
    } else if show_type {
        header_spans.push(Span::styled("Typ ", header_style));
    }
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

        // Reasoning status indicator
        let (rs_label, rs_color) = match entry.reasoning_status {
            crate::benchmarks::ReasoningStatus::Reasoning => ("R  ", Color::Cyan),
            crate::benchmarks::ReasoningStatus::NonReasoning => ("NR ", Color::DarkGray),
            crate::benchmarks::ReasoningStatus::Adaptive => ("AR ", Color::Yellow),
            crate::benchmarks::ReasoningStatus::None => ("   ", Color::Reset),
        };
        row_spans.push(Span::styled(rs_label, Style::default().fg(rs_color)));

        // Source indicator (Open/Closed)
        let (src_label, src_color) = match app.open_weights_map.get(&entry.slug) {
            Some(true) => ("O ", Color::Green),
            Some(false) => ("C ", Color::Red),
            None => ("  ", Color::Reset),
        };
        row_spans.push(Span::styled(src_label, Style::default().fg(src_color)));

        // Region/Type indicator when grouping is active
        if show_region {
            let region = super::app::CreatorRegion::from_creator(&entry.creator);
            row_spans.push(Span::styled(
                format!("{:<4}", region.short_label()),
                Style::default().fg(region.color()),
            ));
        } else if show_type {
            let ct = super::app::CreatorType::from_creator(&entry.creator);
            row_spans.push(Span::styled(
                format!("{:<4}", ct.short_label()),
                Style::default().fg(ct.color()),
            ));
        }

        row_spans.extend(
            visible_cols
                .iter()
                .map(|col| benchmark_col_value(entry, *col, style, name_width)),
        );
        items.push(ListItem::new(Line::from(row_spans)));
    }

    let list = List::new(items);
    let mut state = bench_app.list_state;
    // Offset by 1 for the header row
    state.select(Some(bench_app.selected + 1));
    f.render_stateful_widget(list, inner_area, &mut state);
}

fn draw_benchmark_detail(f: &mut Frame, area: Rect, app: &App) {
    use super::app::BenchmarkFocus;
    let bench_app = &app.benchmarks_app;
    let store = &app.benchmark_store;
    let focused = bench_app.focus == BenchmarkFocus::Details;

    let entry = match bench_app.current_entry(store) {
        Some(e) => e,
        None => {
            let lines = vec![Line::from(Span::styled(
                "No benchmark selected",
                Style::default().fg(Color::DarkGray),
            ))];
            ScrollablePanel::new("Details", lines, &bench_app.detail_scroll, focused)
                .render(f, area);
            return;
        }
    };

    let inner_w = area.width.saturating_sub(2);
    let lines = build_benchmark_detail_lines(inner_w, entry, app);
    ScrollablePanel::new("Details", lines, &bench_app.detail_scroll, focused).render(f, area);
}

fn build_benchmark_detail_lines<'a>(
    width: u16,
    entry: &'a crate::benchmarks::BenchmarkEntry,
    app: &'a App,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    // Dynamic column widths via ratatui's constraint solver
    let cw = ColumnWidths::from_width(width);

    // Name + creator + metadata on first lines
    let creator_display = if !entry.creator_name.is_empty() {
        &entry.creator_name
    } else {
        &entry.creator
    };
    let region = super::app::CreatorRegion::from_creator(&entry.creator);
    let creator_type = super::app::CreatorType::from_creator(&entry.creator);

    // Line 1: Name
    lines.push(Line::from(Span::styled(
        &entry.display_name,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    // Metadata rows (2-wide, dynamic)
    let em = "\u{2014}";
    let (source_label, source_color) = match app.open_weights_map.get(&entry.slug) {
        Some(true) => ("Open", Color::Green),
        Some(false) => ("Closed", Color::Red),
        None => (em, Color::DarkGray),
    };
    push_meta_row(
        &mut lines,
        &cw,
        ("Creator", creator_display, Color::Reset),
        ("Source", source_label, source_color),
    );
    push_meta_row(
        &mut lines,
        &cw,
        ("Region", region.label(), Color::Reset),
        ("Type", creator_type.label(), Color::Reset),
    );
    let date_str = entry.release_date.as_deref().unwrap_or(em);
    let (reasoning_label, reasoning_color) = {
        use crate::benchmarks::ReasoningStatus;
        match entry.reasoning_status {
            ReasoningStatus::Reasoning => ("Reasoning", Color::Cyan),
            ReasoningStatus::NonReasoning => ("Non-reasoning", Color::DarkGray),
            ReasoningStatus::Adaptive => ("Adaptive", Color::Yellow),
            ReasoningStatus::None => (em, Color::DarkGray),
        }
    };
    push_meta_row(
        &mut lines,
        &cw,
        ("Released", date_str, Color::Reset),
        ("Reason", reasoning_label, reasoning_color),
    );
    // Effort + Variant (only if present)
    let has_effort = entry.effort_level.is_some();
    let has_variant = entry.variant_tag.is_some();
    if has_effort || has_variant {
        let effort_str = entry.effort_level.as_deref().unwrap_or(em);
        let variant_str = entry.variant_tag.as_deref().unwrap_or(em);
        push_meta_row(
            &mut lines,
            &cw,
            ("Effort", effort_str, Color::Reset),
            ("Variant", variant_str, Color::Reset),
        );
    }
    // Tools + Context
    let tools_str = match entry.tool_call {
        Some(true) => "Yes",
        Some(false) => "No",
        None => em,
    };
    let tools_color = match entry.tool_call {
        Some(true) => Color::Green,
        Some(false) => Color::DarkGray,
        None => Color::DarkGray,
    };
    let ctx_str = entry
        .context_window
        .map(format_tokens)
        .unwrap_or_else(|| em.to_string());
    push_meta_row(
        &mut lines,
        &cw,
        ("Tools", tools_str, tools_color),
        ("Context", &ctx_str, Color::Reset),
    );
    // Max output
    let out_str = entry
        .max_output
        .map(format_tokens)
        .unwrap_or_else(|| em.to_string());
    push_meta_row(
        &mut lines,
        &cw,
        ("Output", &out_str, Color::Reset),
        ("", "", Color::Reset),
    );

    // Composite Indexes (0-100 scale, higher is better)
    lines.push(Line::from(""));
    push_section_header(&mut lines, "Indexes (0\u{2013}100, \u{2191} better)");
    let int_idx = fmt_idx(entry.intelligence_index);
    let cod_idx = fmt_idx(entry.coding_index);
    push_detail_row(
        &mut lines,
        &cw,
        "Intelligence",
        &int_idx,
        "Coding",
        &cod_idx,
    );
    let math_idx = fmt_idx(entry.math_index);
    push_detail_row(&mut lines, &cw, "Math", &math_idx, "", "");

    // Benchmark Scores (percentage, higher is better)
    lines.push(Line::from(""));
    push_section_header(&mut lines, "Benchmarks (%, \u{2191} better)");
    let gpqa = fmt_pct(entry.gpqa);
    let mmlu = fmt_pct(entry.mmlu_pro);
    push_detail_row(&mut lines, &cw, "GPQA", &gpqa, "MMLU-Pro", &mmlu);
    let hle = fmt_pct(entry.hle);
    let livecode = fmt_pct(entry.livecodebench);
    push_detail_row(&mut lines, &cw, "HLE", &hle, "LiveCode", &livecode);
    let scicode = fmt_pct(entry.scicode);
    let ifbench = fmt_pct(entry.ifbench);
    push_detail_row(&mut lines, &cw, "SciCode", &scicode, "IFBench", &ifbench);
    let terminal = fmt_pct(entry.terminalbench_hard);
    let tau2 = fmt_pct(entry.tau2);
    push_detail_row(&mut lines, &cw, "Terminal", &terminal, "Tau2", &tau2);
    let lcr = fmt_pct(entry.lcr);
    let math500 = fmt_pct(entry.math_500);
    push_detail_row(&mut lines, &cw, "LCR", &lcr, "MATH-500", &math500);
    let aime = fmt_pct(entry.aime);
    let aime25 = fmt_pct(entry.aime_25);
    push_detail_row(&mut lines, &cw, "AIME", &aime, "AIME'25", &aime25);

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
    push_detail_row(&mut lines, &cw, "Speed", &tps_str, "TTFT", &ttft_str);
    push_detail_row(&mut lines, &cw, "TTFAT", &ttfat_str, "", "");

    // Pricing ($/M tokens, lower is better)
    lines.push(Line::from(""));
    push_section_header(&mut lines, "Pricing ($/M tokens, \u{2193} better)");
    let input_price = fmt_price(entry.price_input);
    let output_price = fmt_price(entry.price_output);
    push_detail_row(
        &mut lines,
        &cw,
        "Input",
        &input_price,
        "Output",
        &output_price,
    );
    let blended_str = entry
        .price_blended
        .map(|v| format!("${:.2}", v))
        .unwrap_or_else(|| em.to_string());
    push_detail_row(&mut lines, &cw, "Blended", &blended_str, "", "");

    // Keybinding hints
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("c ", Style::default().fg(Color::Yellow)),
        Span::styled("copy  ", Style::default().fg(Color::DarkGray)),
        Span::styled("o ", Style::default().fg(Color::Yellow)),
        Span::styled("open AA", Style::default().fg(Color::DarkGray)),
    ]));

    lines
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
    let lines = build_benchmark_detail_lines(inner.width, entry, app);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
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

struct ColumnWidths {
    indent: u16,
    label: u16,
    value: u16,
    label2: u16,
}

impl ColumnWidths {
    fn from_width(width: u16) -> Self {
        let indent: u16 = 2;
        let usable = width.saturating_sub(indent);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(28),
                Constraint::Percentage(22),
                Constraint::Percentage(28),
                Constraint::Percentage(22),
            ])
            .split(Rect::new(0, 0, usable, 1));
        Self {
            indent,
            label: chunks[0].width.max(8),
            value: chunks[1].width.max(6),
            label2: chunks[2].width.max(8),
        }
    }
}

fn push_meta_row(
    lines: &mut Vec<Line>,
    cw: &ColumnWidths,
    left: (&str, &str, Color),
    right: (&str, &str, Color),
) {
    let style_for = |c: Color| {
        if c == Color::Reset {
            Style::default()
        } else {
            Style::default().fg(c)
        }
    };

    let mut spans = vec![
        Span::styled(
            format!(
                "{:indent$}{:<w$}",
                "",
                left.0,
                indent = cw.indent as usize,
                w = cw.label as usize
            ),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(
            format!("{:<w$}", left.1, w = cw.value as usize),
            style_for(left.2),
        ),
    ];

    if !right.0.is_empty() {
        spans.push(Span::styled(
            format!("{:<w$}", right.0, w = cw.label2 as usize),
            Style::default().fg(Color::Gray),
        ));
        spans.push(Span::styled(right.1.to_string(), style_for(right.2)));
    }

    lines.push(Line::from(spans));
}

fn push_detail_row(
    lines: &mut Vec<Line>,
    cw: &ColumnWidths,
    l1: &str,
    v1: &str,
    l2: &str,
    v2: &str,
) {
    let em = "\u{2014}";
    let val_color = |s: &str| {
        if s == em {
            Color::DarkGray
        } else {
            Color::White
        }
    };

    let mut spans = vec![
        Span::styled(
            format!(
                "{:indent$}{:<w$}",
                "",
                l1,
                indent = cw.indent as usize,
                w = cw.label as usize
            ),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(
            format!("{:<w$}", v1, w = cw.value as usize),
            Style::default().fg(val_color(v1)),
        ),
    ];

    if !l2.is_empty() {
        spans.push(Span::styled(
            format!("{:<w$}", l2, w = cw.label2 as usize),
            Style::default().fg(Color::Gray),
        ));
        spans.push(Span::styled(
            v2.to_string(),
            Style::default().fg(val_color(v2)),
        ));
    }

    lines.push(Line::from(spans));
}

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
        Some(v) if v.fract() == 0.0 => format!("${:.0}", v),
        Some(v) => format!("${:.2}", v),
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
fn benchmark_col_width(col: super::app::BenchmarkSortColumn) -> u16 {
    use super::app::BenchmarkSortColumn::*;
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
    col: super::app::BenchmarkSortColumn,
    style: Style,
    name_width: usize,
) -> Span<'static> {
    use super::app::BenchmarkSortColumn::*;
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
    col: super::app::BenchmarkSortColumn,
    style: Style,
    name_width: usize,
) -> Span<'a> {
    use super::app::BenchmarkSortColumn::*;
    match col {
        Name => Span::styled(
            format!(
                "{:<width$}",
                truncate(&entry.display_name, name_width.saturating_sub(1)),
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

fn draw_sort_picker(f: &mut Frame, area: Rect, bench_app: &super::app::BenchmarksApp) {
    use super::app::BenchmarkSortColumn;

    let columns = BenchmarkSortColumn::ALL;
    let selected = bench_app.sort_picker_selected;

    // Fixed-size popup: 30 wide, enough for all items + border
    let height = (columns.len() as u16 + 2).min(area.height);
    let width = 30u16.min(area.width);
    let popup_area = centered_rect_fixed(width, height, area);

    f.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = columns
        .iter()
        .map(|col| {
            let marker = if *col == bench_app.sort_column {
                let arrow = if bench_app.sort_descending {
                    "\u{25bc}"
                } else {
                    "\u{25b2}"
                };
                format!(" {arrow}")
            } else {
                String::new()
            };
            ListItem::new(Line::from(format!(" {}{}", col.picker_label(), marker)))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Sort By "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, popup_area, &mut list_state);
}
