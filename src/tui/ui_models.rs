use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

use super::app::App;
use super::models_app::{Filters, Focus, ProviderListItem, SortOrder};
use super::ui::{caret, focus_border};
use crate::formatting::truncate;
use crate::formatting::EM_DASH;
use crate::provider_category::{provider_category, ProviderCategory};

fn provider_detail_lines(app: &App) -> Vec<Line<'static>> {
    let Some(entry) = app.models_app.current_model() else {
        return vec![Line::from(Span::styled(
            "No model selected",
            Style::default().fg(Color::DarkGray),
        ))];
    };
    let provider = app
        .providers
        .iter()
        .find(|(id, _)| id == &entry.provider_id)
        .map(|(_, p)| p);
    let Some(provider) = provider else {
        return vec![Line::from(Span::styled(
            "Provider not found",
            Style::default().fg(Color::DarkGray),
        ))];
    };

    let cat = provider_category(&entry.provider_id);
    let has_doc = provider.doc.is_some();
    let has_api = provider.api.is_some();

    let mut lines = vec![
        Line::from(vec![Span::styled(
            provider.name.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Category: ", Style::default().fg(Color::DarkGray)),
            Span::styled(cat.label(), Style::default().fg(cat.color())),
        ]),
        Line::from(vec![
            Span::styled("Docs: ", Style::default().fg(Color::DarkGray)),
            Span::raw(provider.doc.clone().unwrap_or_else(|| EM_DASH.into())),
        ]),
        Line::from(vec![
            Span::styled("API:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(provider.api.clone().unwrap_or_else(|| EM_DASH.into())),
        ]),
        Line::from(vec![
            Span::styled("Env:  ", Style::default().fg(Color::DarkGray)),
            Span::raw(if provider.env.is_empty() {
                EM_DASH.to_string()
            } else {
                provider.env.join(", ")
            }),
        ]),
    ];

    // Only show keybinding hints for available URLs
    let mut hints: Vec<Span<'static>> = Vec::new();
    if has_doc {
        hints.push(Span::styled("o ", Style::default().fg(Color::Yellow)));
        hints.push(Span::raw("docs"));
    }
    if has_doc && has_api {
        hints.push(Span::raw("  "));
    }
    if has_api {
        hints.push(Span::styled("A ", Style::default().fg(Color::Yellow)));
        hints.push(Span::raw("api"));
    }
    if !hints.is_empty() {
        lines.push(Line::from(hints));
    }

    lines
}

fn draw_right_panel(f: &mut Frame, area: Rect, app: &App) {
    let lines = provider_detail_lines(app);

    // Compute visual height: sum of wrapped line heights + 2 for borders.
    // Word-wrapping can use more lines than char-level div_ceil predicts,
    // so we add 1 extra line for each line that wraps as a buffer.
    let border_block = Block::default().borders(Borders::ALL);
    let inner_w = border_block.inner(area).width as usize;
    let visual_lines: u16 = if inner_w == 0 {
        lines.len() as u16
    } else {
        lines
            .iter()
            .map(|line| {
                let w = line.width();
                if w <= inner_w {
                    1u16
                } else {
                    // div_ceil for char-level + 1 for word-wrap slack
                    w.div_ceil(inner_w) as u16 + 1
                }
            })
            .sum()
    };
    let provider_h = visual_lines + 2; // +2 for borders

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(provider_h), Constraint::Min(0)])
        .split(area);

    draw_provider_detail(f, chunks[0], lines);
    draw_model_detail(f, chunks[1], app);
}

pub(super) fn draw_main(f: &mut Frame, area: Rect, app: &mut App) {
    // 3-column layout: providers 20% | models 45% | right panel 35%
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(45),
            Constraint::Percentage(35),
        ])
        .split(area);

    draw_providers(f, chunks[0], app);
    draw_models(f, chunks[1], app);
    draw_right_panel(f, chunks[2], app);
}

fn draw_providers(f: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.models_app.focus == Focus::Providers;
    let border_style = focus_border(is_focused);

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
    let cat_active = app.models_app.provider_category_filter != ProviderCategory::All;
    let cat_color = if cat_active {
        app.models_app.provider_category_filter.color()
    } else {
        Color::DarkGray
    };
    let grp_color = if app.models_app.group_by_category {
        Color::Green
    } else {
        Color::DarkGray
    };

    let cat_label = if cat_active {
        app.models_app.provider_category_filter.short_label()
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
    let mut items: Vec<ListItem> = Vec::with_capacity(app.models_app.provider_list_items.len());

    for item in &app.models_app.provider_list_items {
        match item {
            ProviderListItem::All => {
                let count = app.models_app.filtered_model_count(&app.providers);
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

    let caret = caret(is_focused);
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(caret);

    f.render_stateful_widget(list, chunks[1], &mut app.models_app.provider_list_state);
}

fn draw_models(f: &mut Frame, area: Rect, app: &mut App) {
    let is_focused = app.models_app.focus == Focus::Models;
    let border_style = focus_border(is_focused);

    let models = app.models_app.filtered_models();

    let sort_indicator = match app.models_app.sort_order {
        SortOrder::Default => String::new(),
        _ => {
            let arrow = if app.models_app.sort_ascending {
                "\u{2191}"
            } else {
                "\u{2193}"
            };
            let label = match app.models_app.sort_order {
                SortOrder::ReleaseDate => "date",
                SortOrder::Cost => "cost",
                SortOrder::Context => "ctx",
                SortOrder::Default => unreachable!(),
            };
            format!(" {}{}", arrow, label)
        }
    };

    let filter_indicator = format_filters(
        &app.models_app.filters,
        app.models_app.provider_category_filter,
    );

    // Show provider name in title when a specific provider is selected
    let provider_label = app
        .models_app
        .selected_provider_data(&app.providers)
        .map(|(_, p)| p.name.as_str())
        .unwrap_or("Models");

    let title = if app.models_app.search_query.is_empty() && filter_indicator.is_empty() {
        format!(" {} ({}){} ", provider_label, models.len(), sort_indicator)
    } else if app.models_app.search_query.is_empty() {
        format!(
            " {} ({}){} [{}] ",
            provider_label,
            models.len(),
            sort_indicator,
            filter_indicator
        )
    } else if filter_indicator.is_empty() {
        format!(
            " {} ({}) [/{}]{} ",
            provider_label,
            models.len(),
            app.models_app.search_query,
            sort_indicator
        )
    } else {
        format!(
            " {} ({}) [/{}] [{}]{} ",
            provider_label,
            models.len(),
            app.models_app.search_query,
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

    // Fixed column widths: caret(2) + caps(5) + Input(8) Output(8) Context(8) + gaps(3)
    let caret_w: u16 = 2;
    let caps_w: u16 = 5; // "RTFO " — 4 indicator chars + 1 space
    let input_w: u16 = 8;
    let output_w: u16 = 8;
    let ctx_w: u16 = 8;
    let num_gaps: u16 = 3;
    let fixed_w = caret_w + caps_w + input_w + output_w + ctx_w + num_gaps;
    let name_width = (inner_area.width.saturating_sub(fixed_w) as usize).max(10);

    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let active_header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Determine which column is actively sorted
    let sort_col = match app.models_app.sort_order {
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
    let caret = caret(is_focused);

    // Build header spans (leading spaces to align with caret)
    let mut header_spans: Vec<Span> = vec![
        Span::raw("  "),
        Span::styled("RTFO ", header_style),
        Span::styled(
            format!("{:<width$}", "Model ID", width = name_width),
            if sort_col == "name" {
                active_header_style
            } else {
                header_style
            },
        ),
    ];
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
        let is_selected = display_idx == app.models_app.selected_model;
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
        let m = &entry.model;
        let (r_ch, r_color) = if m.reasoning {
            ("R", Color::Cyan)
        } else {
            ("·", Color::DarkGray)
        };
        let (t_ch, t_color) = if m.tool_call {
            ("T", Color::Yellow)
        } else {
            ("·", Color::DarkGray)
        };
        let (f_ch, f_color) = if m.attachment {
            ("F", Color::Magenta)
        } else {
            ("·", Color::DarkGray)
        };
        let (o_ch, o_color) = if m.open_weights {
            ("O", Color::Green)
        } else {
            ("C", Color::Red)
        };
        let mut row_spans: Vec<Span> = vec![
            Span::styled(prefix, style),
            Span::styled(r_ch, Style::default().fg(r_color)),
            Span::styled(t_ch, Style::default().fg(t_color)),
            Span::styled(f_ch, Style::default().fg(f_color)),
            Span::styled(o_ch, Style::default().fg(o_color)),
            Span::raw(" "),
            Span::styled(
                format!(
                    "{:<width$}",
                    truncate(&entry.id, name_width.saturating_sub(1)),
                    width = name_width
                ),
                style,
            ),
        ];
        row_spans.push(Span::styled(format!(" {:>8}", input_cost), style));
        row_spans.push(Span::styled(format!(" {:>8}", output_cost), style));
        row_spans.push(Span::styled(format!(" {:>8}", ctx), style));

        items.push(ListItem::new(Line::from(row_spans)));
    }

    let list = List::new(items);
    let mut state = app.models_app.model_list_state;
    f.render_stateful_widget(list, inner_area, &mut state);
}

fn draw_provider_detail(f: &mut Frame, area: Rect, lines: Vec<Line<'static>>) {
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Provider "))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_model_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(" Details ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(entry) = app.models_app.current_model() else {
        let para = Paragraph::new(Line::from(Span::styled(
            "No model selected",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(para, inner);
        return;
    };

    let model = &entry.model;
    let provider_id = &entry.provider_id;
    let is_deprecated = model.status.as_deref() == Some("deprecated");
    let text_color = if is_deprecated {
        Color::DarkGray
    } else {
        Color::White
    };
    let label_color = Color::DarkGray;
    let em = "\u{2014}";

    // Helper: render a dash-padded section header into a 1-line rect
    let render_section_header = |f: &mut Frame, rect: Rect, title: &str| {
        let w = rect.width as usize;
        let prefix = format!("\u{2500}\u{2500} {} ", title);
        let fill_len = w.saturating_sub(prefix.chars().count());
        let header = format!("{}{}", prefix, "\u{2500}".repeat(fill_len));
        let para = Paragraph::new(Line::from(Span::styled(
            header,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )));
        f.render_widget(para, rect);
    };

    // Helper: label width for Table columns (longest label + 1 space)
    // Pricing labels: "Cache Read: " = 12, "Cache Write: " = 13 → use 13
    // Limits labels:  "Context: " = 9, "Input: " = 7, "Output: " = 8 → use 9
    // Dates labels:   "Released: " = 10, "Knowledge: " = 11, "Updated: " = 9 → use 11
    let pricing_lw: u16 = 13;
    let limits_lw: u16 = 9;
    let dates_lw: u16 = 11;

    // ── Determine dates table height (1 or 2 rows) ────────────────────────
    let has_updated = model.last_updated.is_some();
    let dates_rows: u16 = if has_updated { 2 } else { 1 };

    // ── Pre-build modalities paragraph for dynamic height ────────────────
    let (mod_in, mod_out) = match &model.modalities {
        Some(m) => (
            if m.input.is_empty() {
                "text".to_string()
            } else {
                m.input.join(", ")
            },
            if m.output.is_empty() {
                "text".to_string()
            } else {
                m.output.join(", ")
            },
        ),
        None => ("text".to_string(), "text".to_string()),
    };
    let mod_para = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Input:  ", Style::default().fg(label_color)),
            Span::styled(mod_in, Style::default().fg(text_color)),
        ]),
        Line::from(vec![
            Span::styled("Output: ", Style::default().fg(label_color)),
            Span::styled(mod_out, Style::default().fg(text_color)),
        ]),
    ])
    .wrap(Wrap { trim: false });
    let mod_rows = mod_para.line_count(inner.width) as u16;

    // ── Vertical layout ───────────────────────────────────────────────────
    // identity(3) + gap(1) + cap_hdr(1) + cap(3) + gap(1)
    // + price_hdr(1) + price_tbl(2) + gap(1)
    // + limits_hdr(1) + limits_tbl(1) + gap(1)
    // + mod_hdr(1) + mod(dynamic) + gap(1)
    // + dates_hdr(1) + dates_tbl(1 or 2) + remainder
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),          // 0: identity
            Constraint::Length(1),          // 1: gap
            Constraint::Length(1),          // 2: capabilities header
            Constraint::Length(3),          // 3: capabilities table
            Constraint::Length(1),          // 4: gap
            Constraint::Length(1),          // 5: pricing header
            Constraint::Length(2),          // 6: pricing table
            Constraint::Length(1),          // 7: gap
            Constraint::Length(1),          // 8: limits header
            Constraint::Length(1),          // 9: limits table
            Constraint::Length(1),          // 10: gap
            Constraint::Length(1),          // 11: modalities header
            Constraint::Length(mod_rows),   // 12: modalities content (dynamic)
            Constraint::Length(1),          // 13: gap
            Constraint::Length(1),          // 14: dates header
            Constraint::Length(dates_rows), // 15: dates table
            Constraint::Min(0),             // 16: remainder
        ])
        .split(inner);

    // ── Identity ──────────────────────────────────────────────────────────
    let name_spans: Vec<Span> = vec![Span::styled(
        model.name.clone(),
        Style::default().fg(text_color).add_modifier(Modifier::BOLD),
    )];
    let row_id = Line::from(Span::styled(
        entry.id.clone(),
        Style::default().fg(Color::DarkGray),
    ));
    let mut provider_spans = vec![
        Span::styled("Provider: ", Style::default().fg(label_color)),
        Span::styled(provider_id.clone(), Style::default().fg(Color::Cyan)),
        Span::raw("     "),
        Span::styled("Family: ", Style::default().fg(label_color)),
        Span::raw(model.family.clone().unwrap_or_else(|| em.to_string())),
    ];
    if let Some(status) = model.status.as_deref() {
        if status != "active" {
            let status_color = if status == "deprecated" {
                Color::Red
            } else {
                Color::DarkGray
            };
            provider_spans.push(Span::raw("     "));
            provider_spans.push(Span::styled("Status: ", Style::default().fg(label_color)));
            provider_spans.push(Span::styled(
                status.to_string(),
                Style::default().fg(status_color),
            ));
        }
    }
    let row_provider = Line::from(provider_spans);
    let identity_para = Paragraph::new(vec![Line::from(name_spans), row_id, row_provider]);
    f.render_widget(identity_para, chunks[0]);

    // ── Capabilities ──────────────────────────────────────────────────────
    render_section_header(f, chunks[2], "Capabilities");

    let cap_val = |active: bool, color: Color| -> (String, Color) {
        if active {
            ("Yes".to_string(), color)
        } else {
            ("No".to_string(), Color::DarkGray)
        }
    };
    let (r_val, r_col) = cap_val(model.reasoning, Color::Cyan);
    let (t_val, t_col) = cap_val(model.tool_call, Color::Yellow);
    let (f_val, f_col) = cap_val(model.attachment, Color::Magenta);
    let (ow_val, ow_col) = if model.open_weights {
        ("Open".to_string(), Color::Green)
    } else {
        ("Closed".to_string(), Color::Red)
    };
    let (tmp_val, tmp_col) = cap_val(model.temperature, Color::White);
    let cap_lw: u16 = 10;
    let caps_table = Table::new(
        vec![
            Row::new(vec![
                Cell::from(Span::styled("Reasoning:", Style::default().fg(label_color))),
                Cell::from(Span::styled(r_val, Style::default().fg(r_col))),
                Cell::from(Span::styled("Tools:", Style::default().fg(label_color))),
                Cell::from(Span::styled(t_val, Style::default().fg(t_col))),
            ]),
            Row::new(vec![
                Cell::from(Span::styled("Source:", Style::default().fg(label_color))),
                Cell::from(Span::styled(ow_val, Style::default().fg(ow_col))),
                Cell::from(Span::styled("Files:", Style::default().fg(label_color))),
                Cell::from(Span::styled(f_val, Style::default().fg(f_col))),
            ]),
            Row::new(vec![
                Cell::from(Span::styled("Temp:", Style::default().fg(label_color))),
                Cell::from(Span::styled(tmp_val, Style::default().fg(tmp_col))),
                Cell::from(Span::raw("")),
                Cell::from(Span::raw("")),
            ]),
        ],
        [
            Constraint::Length(cap_lw),
            Constraint::Fill(1),
            Constraint::Length(cap_lw),
            Constraint::Fill(1),
        ],
    );
    f.render_widget(caps_table, chunks[3]);

    // ── Pricing ───────────────────────────────────────────────────────────
    render_section_header(f, chunks[5], "Pricing");

    let free = model.is_free();
    let cost_color = if free { Color::Green } else { text_color };
    let fmt_cost = |val: Option<f64>| -> (String, Color) {
        match val {
            None => {
                if free {
                    ("Free".to_string(), Color::Green)
                } else {
                    (em.to_string(), Color::DarkGray)
                }
            }
            Some(0.0) => ("$0/M".to_string(), Color::Green),
            Some(v) => {
                let formatted = if v.fract() == 0.0 {
                    format!("${}/M", v as u64)
                } else {
                    format!("${:.2}/M", v)
                };
                (formatted, cost_color)
            }
        }
    };
    let (input_str, input_color) = fmt_cost(model.cost.as_ref().and_then(|c| c.input));
    let (output_str, output_color) = fmt_cost(model.cost.as_ref().and_then(|c| c.output));
    let (cache_read_str, cache_read_color) =
        fmt_cost(model.cost.as_ref().and_then(|c| c.cache_read));
    let (cache_write_str, cache_write_color) =
        fmt_cost(model.cost.as_ref().and_then(|c| c.cache_write));

    let pricing_table = Table::new(
        vec![
            Row::new(vec![
                Cell::from(Span::styled("Input:", Style::default().fg(label_color))),
                Cell::from(Span::styled(input_str, Style::default().fg(input_color))),
                Cell::from(Span::styled("Output:", Style::default().fg(label_color))),
                Cell::from(Span::styled(output_str, Style::default().fg(output_color))),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(
                    "Cache Read:",
                    Style::default().fg(label_color),
                )),
                Cell::from(Span::styled(
                    cache_read_str,
                    Style::default().fg(cache_read_color),
                )),
                Cell::from(Span::styled(
                    "Cache Write:",
                    Style::default().fg(label_color),
                )),
                Cell::from(Span::styled(
                    cache_write_str,
                    Style::default().fg(cache_write_color),
                )),
            ]),
        ],
        [
            Constraint::Length(pricing_lw),
            Constraint::Fill(1),
            Constraint::Length(pricing_lw),
            Constraint::Fill(1),
        ],
    );
    f.render_widget(pricing_table, chunks[6]);

    // ── Limits ────────────────────────────────────────────────────────────
    render_section_header(f, chunks[8], "Limits");

    let ctx_str = model.context_str();
    let inp_lim_str = model.input_limit_str();
    let out_str = model.output_str();
    let (ctx_val, ctx_color) = if ctx_str == "-" {
        (em.to_string(), Color::DarkGray)
    } else {
        (ctx_str, text_color)
    };
    let (inp_lim_val, inp_lim_color) = if inp_lim_str == "-" {
        (em.to_string(), Color::DarkGray)
    } else {
        (inp_lim_str, text_color)
    };
    let (out_val, out_color) = if out_str == "-" {
        (em.to_string(), Color::DarkGray)
    } else {
        (out_str, text_color)
    };
    let limits_table = Table::new(
        vec![Row::new(vec![
            Cell::from(Span::styled("Context:", Style::default().fg(label_color))),
            Cell::from(Span::styled(ctx_val, Style::default().fg(ctx_color))),
            Cell::from(Span::styled("Input:", Style::default().fg(label_color))),
            Cell::from(Span::styled(
                inp_lim_val,
                Style::default().fg(inp_lim_color),
            )),
            Cell::from(Span::styled("Output:", Style::default().fg(label_color))),
            Cell::from(Span::styled(out_val, Style::default().fg(out_color))),
        ])],
        [
            Constraint::Length(limits_lw),
            Constraint::Min(6),
            Constraint::Length(limits_lw),
            Constraint::Min(6),
            Constraint::Length(limits_lw),
            Constraint::Min(6),
        ],
    );
    f.render_widget(limits_table, chunks[9]);

    // ── Modalities ────────────────────────────────────────────────────────
    render_section_header(f, chunks[11], "Modalities");
    f.render_widget(mod_para, chunks[12]);

    // ── Dates ─────────────────────────────────────────────────────────────
    render_section_header(f, chunks[14], "Dates");

    let released = model.release_date.as_deref().unwrap_or(em);
    let knowledge = model.knowledge.as_deref().unwrap_or(em);
    let rel_color = if released == em {
        Color::DarkGray
    } else {
        text_color
    };
    let know_color = if knowledge == em {
        Color::DarkGray
    } else {
        text_color
    };

    let mut dates_rows_data: Vec<Row> = vec![Row::new(vec![
        Cell::from(Span::styled("Released:", Style::default().fg(label_color))),
        Cell::from(Span::styled(
            released.to_string(),
            Style::default().fg(rel_color),
        )),
        Cell::from(Span::styled("Knowledge:", Style::default().fg(label_color))),
        Cell::from(Span::styled(
            knowledge.to_string(),
            Style::default().fg(know_color),
        )),
    ])];

    if let Some(updated) = &model.last_updated {
        let upd_color = if is_deprecated {
            Color::DarkGray
        } else {
            text_color
        };
        dates_rows_data.push(Row::new(vec![
            Cell::from(Span::styled("Updated:", Style::default().fg(label_color))),
            Cell::from(Span::styled(
                updated.clone(),
                Style::default().fg(upd_color),
            )),
            Cell::from(""),
            Cell::from(""),
        ]));
    }

    let dates_table = Table::new(
        dates_rows_data,
        [
            Constraint::Length(dates_lw),
            Constraint::Fill(1),
            Constraint::Length(dates_lw),
            Constraint::Fill(1),
        ],
    );
    f.render_widget(dates_table, chunks[15]);
}

/// Unicode-safe truncation with ellipsis for table cells.
pub(super) fn format_filters(filters: &Filters, category: ProviderCategory) -> String {
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
