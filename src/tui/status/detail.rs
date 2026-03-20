use crate::formatting::format_relative_time_from_str;
use crate::status::{ProviderHealth, StatusProvenance};
use crate::tui::ui::{status_health_icon, status_health_style};
use crate::tui::widgets::scroll_offset::ScrollOffset;
use crate::tui::widgets::scrollable_panel::ScrollablePanel;
use crate::tui::widgets::soft_card::SoftCard;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use super::render::{
    component_status_icon, component_status_style, incident_impact_style, incident_stage_health,
    incident_stage_style, incident_status_value, push_plain_scope_lines, status_field_label_style,
    status_verdict_copy, translate_component_name,
};

/// Sort active incidents by impact severity, then recency.
pub(super) fn sorted_active_incidents(
    entry: &crate::status::ProviderStatus,
) -> Vec<crate::status::ActiveIncident> {
    let mut items: Vec<_> = entry.active_incidents().into_iter().cloned().collect();
    items.sort_by(|a, b| {
        let impact_rank = |impact: &str| -> u8 {
            let impact = impact.to_lowercase();
            if impact.contains("critical") || impact.contains("major") {
                0
            } else if impact.contains("minor") || impact.contains("partial") {
                1
            } else {
                2
            }
        };
        let ts = |raw: Option<&str>| {
            raw.and_then(crate::agents::helpers::parse_date)
                .map(|dt| dt.timestamp())
                .unwrap_or(0)
        };
        impact_rank(&a.impact)
            .cmp(&impact_rank(&b.impact))
            .then_with(|| {
                ts(b.updated_at.as_deref().or(b.created_at.as_deref()))
                    .cmp(&ts(a.updated_at.as_deref().or(a.created_at.as_deref())))
            })
    });
    items
}

/// Sort components by severity, then alphabetically.
pub(super) fn sorted_components<'a>(
    entry: &'a crate::status::ProviderStatus,
    active_incidents: &[crate::status::ActiveIncident],
) -> Vec<&'a crate::status::ComponentStatus> {
    if !entry.component_detail_available() {
        return Vec::new();
    }
    let mut component_incident_map: std::collections::HashSet<&str> =
        std::collections::HashSet::new();
    for incident in active_incidents {
        for component in &incident.affected_components {
            component_incident_map.insert(component.as_str());
        }
    }
    for maint in &entry.scheduled_maintenances {
        for component in &maint.affected_components {
            component_incident_map.insert(component.as_str());
        }
    }
    let mut components: Vec<_> = entry.components.iter().collect();
    components.sort_by(|a, b| {
        let severity = |status: &str| -> u8 {
            match component_status_icon(status) {
                "✗" => 0,
                "◐" => 1,
                "◆" => 2,
                "●" => 3,
                _ => 4,
            }
        };
        severity(&a.status)
            .cmp(&severity(&b.status))
            .then_with(|| translate_component_name(&a.name).cmp(&translate_component_name(&b.name)))
    });
    components
}

/// Build a services panel title with health summary icons.
/// Example: ` Services (23)  ● 18  ◐ 3  ✗ 1  ◆ 1 `
fn build_services_title(components: &[&crate::status::ComponentStatus]) -> Line<'static> {
    let mut op = 0u16;
    let mut degraded = 0u16;
    let mut outage = 0u16;
    let mut maintenance = 0u16;
    for comp in components {
        match component_status_icon(&comp.status) {
            "●" => op += 1,
            "◐" => degraded += 1,
            "✗" => outage += 1,
            "◆" => maintenance += 1,
            _ => {}
        }
    }

    let mut spans = vec![Span::raw(format!(" Services ({}) ", components.len()))];
    if op > 0 {
        spans.push(Span::styled(" ● ", Style::default().fg(Color::Green)));
        spans.push(Span::raw(format!("{op} ")));
    }
    if degraded > 0 {
        spans.push(Span::styled(" ◐ ", Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(format!("{degraded} ")));
    }
    if outage > 0 {
        spans.push(Span::styled(" ✗ ", Style::default().fg(Color::Red)));
        spans.push(Span::raw(format!("{outage} ")));
    }
    if maintenance > 0 {
        spans.push(Span::styled(" ◆ ", Style::default().fg(Color::Blue)));
        spans.push(Span::raw(format!("{maintenance} ")));
    }
    Line::from(spans)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_provider_status_detail(
    f: &mut Frame,
    area: Rect,
    display_name: &str,
    health: ProviderHealth,
    provenance: StatusProvenance,
    error_msg: &Option<String>,
    time_label: &str,
    time_value: &str,
    caveat: &Option<String>,
    service_note: &Option<String>,
    incident_note: &Option<String>,
    maintenance_note: &Option<String>,
    confirmed_no_components: bool,
    confirmed_no_incidents: bool,
    maintenance_problem: bool,
    active_incidents: &[crate::status::ActiveIncident],
    components: &[&crate::status::ComponentStatus],
    scheduled_maintenances: &[crate::status::ScheduledMaintenance],
    detail_scroll: &ScrollOffset,
    is_focused: bool,
    services_expanded: bool,
    services_scroll: &ScrollOffset,
) {
    let dark_border = Style::default().fg(Color::DarkGray);

    // Compute dynamic subpanel heights
    // Base: gauge + legend + 2 borders = 4
    let mut status_h: u16 = 4;
    if caveat.is_some() || provenance == StatusProvenance::Unavailable {
        status_h += 1;
    }
    if error_msg.is_some() {
        status_h += 1;
    }

    let has_components =
        !components.is_empty() || service_note.is_some() || confirmed_no_components;
    // Count non-operational components for the services panel
    let non_op_comp_count = components
        .iter()
        .filter(|c| {
            let s = c.status.to_lowercase();
            !s.contains("operational") && s != "unknown" && !s.is_empty()
        })
        .count();
    let healthy_comp_count = components.len() - non_op_comp_count;
    // Service rows: one per non-operational component + optional summary line
    let service_rows = if has_components {
        let base = non_op_comp_count as u16;
        let summary = if healthy_comp_count > 0 { 1u16 } else { 0 };
        (base + summary).max(1) // at least 1 row
    } else {
        0
    };

    let has_maintenance = !scheduled_maintenances.is_empty() || maintenance_problem;

    let mut constraints: Vec<Constraint> = vec![Constraint::Length(status_h)];
    if has_components {
        if services_expanded {
            let expanded_h = (components.len() as u16 + 2).min(12);
            constraints.push(Constraint::Length(expanded_h));
        } else {
            constraints.push(Constraint::Length(service_rows + 2)); // rows + borders
        }
    }
    // Bottom area: incidents (+ maintenance if present) share remaining space
    constraints.push(Constraint::Min(0));

    let panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // ── Status header ──────────────────────────────────────────
    {
        let status_area = panel_chunks[chunk_idx];
        chunk_idx += 1;

        let title = format!(" {display_name} · {time_label}: {time_value} ");

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .title(title);
        let inner = block.inner(status_area);
        f.render_widget(block, status_area);

        let mut inner_constraints = vec![
            Constraint::Length(1), // gauge
            Constraint::Length(1), // legend (icon+count summary)
        ];
        if caveat.is_some() || provenance == StatusProvenance::Unavailable {
            inner_constraints.push(Constraint::Length(1));
        }
        if error_msg.is_some() {
            inner_constraints.push(Constraint::Length(1));
        }
        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(inner_constraints)
            .split(inner);

        // Gauge: operational components / total
        let total = components.len();
        let ratio = if total > 0 {
            healthy_comp_count as f64 / total as f64
        } else {
            match health {
                ProviderHealth::Operational => 1.0,
                ProviderHealth::Unknown => 1.0,
                _ => 0.5,
            }
        };
        let gauge_label = if total > 0 {
            format!("{}/{total}  {:.0}%", healthy_comp_count, ratio * 100.0)
        } else {
            format!(
                "{} {}",
                status_health_icon(health),
                status_verdict_copy(health)
            )
        };
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(status_health_style(health).fg.unwrap_or(Color::Green))
                    .bg(Color::DarkGray),
            )
            .ratio(ratio)
            .label(gauge_label);
        f.render_widget(gauge, inner_chunks[0]);

        // Legend line: icon+count for each category (matching Overall panel style)
        // Separate degraded components from maintenance components
        let degraded_comp_count = components
            .iter()
            .filter(|c| {
                let s = c.status.to_lowercase();
                !s.contains("operational")
                    && !s.contains("maint")
                    && s != "unknown"
                    && !s.is_empty()
            })
            .count();
        let maint_comp_count = components
            .iter()
            .filter(|c| c.status.to_lowercase().contains("maint"))
            .count();

        let mut legend_spans: Vec<Span<'static>> = Vec::new();
        if healthy_comp_count > 0 {
            legend_spans.push(Span::styled("● ", Style::default().fg(Color::Green)));
            legend_spans.push(Span::raw(format!("{healthy_comp_count} operational  ")));
        }
        if !active_incidents.is_empty() {
            legend_spans.push(Span::styled("◐ ", Style::default().fg(Color::Yellow)));
            legend_spans.push(Span::raw(format!(
                "{} active incident{}  ",
                active_incidents.len(),
                if active_incidents.len() == 1 { "" } else { "s" }
            )));
        }
        if degraded_comp_count > 0 {
            legend_spans.push(Span::styled("◐ ", Style::default().fg(Color::Yellow)));
            legend_spans.push(Span::raw(format!(
                "{degraded_comp_count} service degradation{}  ",
                if degraded_comp_count == 1 { "" } else { "s" }
            )));
        }
        if maint_comp_count > 0 {
            legend_spans.push(Span::styled("◆ ", Style::default().fg(Color::Blue)));
            legend_spans.push(Span::raw(format!("{maint_comp_count} under maintenance  ")));
        }
        if !scheduled_maintenances.is_empty() {
            legend_spans.push(Span::styled("◆ ", Style::default().fg(Color::Blue)));
            legend_spans.push(Span::raw(format!(
                "{} scheduled maintenance  ",
                scheduled_maintenances.len()
            )));
        }
        if legend_spans.is_empty() {
            if let Some(note) = incident_note.as_deref() {
                legend_spans.push(Span::styled(
                    note.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                legend_spans.push(Span::styled(
                    "No active issues",
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
        f.render_widget(Paragraph::new(Line::from(legend_spans)), inner_chunks[1]);

        // Optional caveat/notes
        let mut extra_idx = 2;
        if let Some(caveat_text) = caveat {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    caveat_text.clone(),
                    Style::default().fg(Color::Yellow),
                ))),
                inner_chunks[extra_idx],
            );
            extra_idx += 1;
        } else if provenance == StatusProvenance::Unavailable {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "Status unavailable",
                    Style::default().fg(Color::Yellow),
                ))),
                inner_chunks[extra_idx],
            );
            extra_idx += 1;
        }
        if let Some(err) = error_msg {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    err.clone(),
                    Style::default().fg(Color::Red),
                ))),
                inner_chunks[extra_idx],
            );
        }
    }

    // ── Services (non-operational highlighted, healthy summarized) ─
    if has_components {
        let services_area = panel_chunks[chunk_idx];
        chunk_idx += 1;

        // Build title with health summary icons
        let services_title = build_services_title(components);

        let mut lines: Vec<Line<'static>> = Vec::new();

        if let Some(note) = service_note {
            lines.push(Line::from(Span::styled(
                note.clone(),
                Style::default().fg(Color::DarkGray),
            )));
        } else if confirmed_no_components {
            lines.push(Line::from(Span::styled(
                "No service-level issues reported",
                Style::default().fg(Color::DarkGray),
            )));
        } else if services_expanded {
            // Expanded: show ALL services with status icon + name
            for comp in components {
                let name = translate_component_name(&comp.name);
                lines.push(Line::from(vec![
                    Span::styled(
                        component_status_icon(&comp.status),
                        component_status_style(&comp.status),
                    ),
                    Span::raw(" "),
                    Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("  {}", comp.status.replace('_', " ")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        } else {
            // Collapsed: only non-operational + summary line
            for comp in components {
                let s = comp.status.to_lowercase();
                if s.contains("operational") || s == "unknown" || s.is_empty() {
                    continue;
                }
                let name = translate_component_name(&comp.name);
                lines.push(Line::from(vec![
                    Span::styled(
                        component_status_icon(&comp.status),
                        component_status_style(&comp.status),
                    ),
                    Span::raw(" "),
                    Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("  {}", comp.status.replace('_', " ")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }

            // Title icons already communicate healthy count — no summary line needed
        }

        if services_expanded {
            ScrollablePanel::new(services_title, lines, services_scroll, false)
                .render(f, services_area);
        } else {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(dark_border)
                .title(services_title);
            f.render_widget(Paragraph::new(lines).block(block), services_area);
        }
    }

    // ── Bottom area: Incidents + Maintenance ──────────────────
    {
        let bottom_area = panel_chunks[chunk_idx];

        // Split horizontally when wide enough and maintenance exists, else stack
        let (incidents_area, maint_area) = if has_maintenance && bottom_area.width >= 60 {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(bottom_area);
            (cols[0], Some(cols[1]))
        } else if has_maintenance {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(bottom_area);
            (rows[0], Some(rows[1]))
        } else {
            (bottom_area, None)
        };

        // ── Incidents ──
        let body_width = usize::from(incidents_area.width.saturating_sub(4)).max(24);
        let title = format!("Current Incidents ({})", active_incidents.len());

        if active_incidents.is_empty() {
            let incident_empty_text = incident_note.clone().unwrap_or_else(|| {
                if confirmed_no_incidents {
                    "No active incidents".to_string()
                } else {
                    "Incident details unavailable".to_string()
                }
            });
            let lines = vec![Line::from(Span::styled(
                incident_empty_text,
                Style::default().fg(Color::DarkGray),
            ))];
            ScrollablePanel::new(title, lines, detail_scroll, is_focused).render(f, incidents_area);
        } else {
            let mut cards = Vec::new();

            for incident in active_incidents.iter() {
                let accent_health = incident_stage_health(&incident.status);
                let mut card_lines = Vec::new();

                card_lines.push(Line::from(vec![
                    Span::styled("◉ ", incident_stage_style(&incident.status)),
                    Span::styled(
                        incident.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]));
                let mut metadata_spans = vec![Span::raw("  ")];
                metadata_spans.push(Span::styled("Status: ", status_field_label_style()));
                metadata_spans.push(Span::styled(
                    incident_status_value(incident),
                    incident_stage_style(&incident.status),
                ));

                let impact_lower = incident.impact.to_lowercase();
                if !impact_lower.is_empty() && impact_lower != "none" {
                    metadata_spans.push(Span::raw("  "));
                    metadata_spans.push(Span::styled("Impact: ", status_field_label_style()));
                    metadata_spans.push(Span::styled(
                        incident.impact.clone(),
                        incident_impact_style(&incident.impact),
                    ));
                }

                if let Some(updated_at) = incident
                    .updated_at
                    .as_deref()
                    .or(incident.created_at.as_deref())
                {
                    metadata_spans.push(Span::raw("  "));
                    metadata_spans.push(Span::styled("Updated: ", status_field_label_style()));
                    metadata_spans.push(Span::styled(
                        format_relative_time_from_str(updated_at),
                        Style::default().fg(Color::Cyan),
                    ));
                }
                card_lines.push(Line::from(metadata_spans));

                if let Some(shortlink) = &incident.shortlink {
                    card_lines.push(Line::from(vec![
                        Span::styled("  Link: ", status_field_label_style()),
                        Span::styled(
                            shortlink.clone(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                    ]));
                }

                if !incident.affected_components.is_empty() {
                    card_lines.push(Line::from(Span::styled(
                        format!("  Affected: {}", incident.affected_components.join(", ")),
                        Style::default().fg(Color::DarkGray),
                    )));
                }

                if let Some(update) = &incident.latest_update {
                    for line in textwrap::wrap(&update.body, body_width.saturating_sub(2))
                        .iter()
                        .take(3)
                    {
                        card_lines.push(Line::from(Span::raw(format!("  {line}"))));
                    }
                }

                cards.push(SoftCard::new(accent_health, card_lines));
            }

            ScrollablePanel::with_cards(title, cards, detail_scroll, is_focused)
                .render(f, incidents_area);
        }

        // ── Maintenance ──
        if let Some(maint_area) = maint_area {
            let title = format!("Maintenance ({})", scheduled_maintenances.len());

            if maintenance_problem || scheduled_maintenances.is_empty() {
                let lines = vec![Line::from(Span::styled(
                    maintenance_note
                        .clone()
                        .unwrap_or_else(|| "Maintenance details failed to load".to_string()),
                    Style::default().fg(Color::DarkGray),
                ))];
                ScrollablePanel::new(title, lines, detail_scroll, false).render(f, maint_area);
            } else {
                let mut cards = Vec::new();

                for maint in scheduled_maintenances {
                    let mut card_lines = Vec::new();

                    card_lines.push(Line::from(vec![
                        Span::styled("◆ ", Style::default().fg(Color::Blue)),
                        Span::styled(
                            maint.name.clone(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    let mut status_spans = vec![
                        Span::styled("  Status: ", status_field_label_style()),
                        Span::styled(
                            maint.status.replace('_', " "),
                            component_status_style(&maint.status),
                        ),
                    ];
                    if let Some(start) = maint.scheduled_for.as_deref() {
                        status_spans.push(Span::raw("  "));
                        status_spans.push(Span::styled("Scheduled: ", status_field_label_style()));
                        status_spans.push(Span::styled(
                            format_relative_time_from_str(start),
                            Style::default().fg(Color::Cyan),
                        ));
                    }
                    card_lines.push(Line::from(status_spans));

                    if let Some(until) = maint.scheduled_until.as_deref() {
                        card_lines.push(Line::from(vec![
                            Span::styled("  Until: ", status_field_label_style()),
                            Span::styled(
                                format_relative_time_from_str(until),
                                Style::default().fg(Color::Cyan),
                            ),
                        ]));
                    }

                    if !maint.affected_components.is_empty() {
                        push_plain_scope_lines(
                            &mut card_lines,
                            "Affected",
                            &maint.affected_components,
                            3,
                        );
                    }

                    cards.push(SoftCard::new(ProviderHealth::Maintenance, card_lines));
                }

                ScrollablePanel::with_cards(title, cards, detail_scroll, false)
                    .render(f, maint_area);
            }
        }
    }
}
