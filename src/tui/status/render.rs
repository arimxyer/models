use crate::formatting::{format_relative_time_from_str, truncate};
use crate::status::{ProviderHealth, StatusProvenance, StatusSourceMethod};
use crate::tui::app::App;
use crate::tui::ui::{caret, selection_style, status_health_icon, status_health_style};
use crate::tui::widgets::scrollable_panel::ScrollablePanel;
use crate::tui::widgets::soft_card::SoftCard;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, LineGauge, List, ListItem, Paragraph},
    Frame,
};

fn component_status_icon(status: &str) -> &'static str {
    let s = status.to_lowercase();
    if s.contains("operational") {
        "●"
    } else if s.contains("degraded") || s.contains("partial") {
        "◐"
    } else if s.contains("outage") || s.contains("major") || s.contains("down") {
        "✗"
    } else if s.contains("maintenance") {
        "◆"
    } else {
        "?"
    }
}

fn component_status_style(status: &str) -> Style {
    let s = status.to_lowercase();
    if s.contains("operational") {
        Style::default().fg(Color::Green)
    } else if s.contains("degraded") || s.contains("partial") {
        Style::default().fg(Color::Yellow)
    } else if s.contains("outage") || s.contains("major") || s.contains("down") {
        Style::default().fg(Color::Red)
    } else if s.contains("maintenance") {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// 6-char left-aligned gutter tag (padded with spaces, DarkGray) + content spans at column 7.
#[allow(dead_code)]
fn gutter_line<'a>(tag: &str, spans: Vec<Span<'a>>) -> Line<'a> {
    let padded = format!("{:<6}", tag);
    let mut all = vec![Span::styled(padded, Style::default().fg(Color::DarkGray))];
    all.extend(spans);
    Line::from(all)
}

/// Chinese component name map for DeepSeek and others.
const CHINESE_NAME_MAP: &[(&str, &str)] =
    &[("API 服务", "API Service"), ("网页对话服务", "Web Chat")];

fn translate_component_name(name: &str) -> String {
    for &(chinese, english) in CHINESE_NAME_MAP {
        if name == chinese {
            return format!("{} ({})", english, chinese);
        }
    }
    name.to_string()
}

fn provider_last_meaningful_update(
    entry: &crate::status::ProviderStatus,
) -> Option<(&'static str, String)> {
    let latest = entry
        .incidents
        .iter()
        .filter_map(|incident| {
            incident
                .updated_at
                .as_deref()
                .or(incident.created_at.as_deref())
        })
        .chain(entry.scheduled_maintenances.iter().filter_map(|maint| {
            maint
                .scheduled_for
                .as_deref()
                .or(maint.scheduled_until.as_deref())
        }))
        .filter_map(|raw| {
            crate::agents::helpers::parse_date(raw).map(|parsed| (parsed.timestamp(), raw))
        })
        .max_by_key(|(timestamp, _)| *timestamp)
        .map(|(_, raw)| raw.to_string());

    if let Some(raw) = latest {
        return Some(("latest event", format_relative_time_from_str(&raw)));
    }

    entry.source_updated_at.as_deref().map(|raw| {
        let label = match entry.source_method {
            Some(StatusSourceMethod::ApiStatusCheck) => "last checked",
            _ => "source updated",
        };
        (label, format_relative_time_from_str(raw))
    })
}

fn title_case_status_time_label(label: &str) -> &'static str {
    match label {
        "latest event" => "Latest event",
        "source updated" => "Source updated",
        "last checked" => "Last checked",
        _ => "Source updated",
    }
}

fn overall_non_operational_components(
    entry: &crate::status::ProviderStatus,
) -> Vec<&crate::status::ComponentStatus> {
    entry
        .components
        .iter()
        .filter(|component| {
            let status = component.status.to_lowercase();
            !status.contains("operational") && status != "unknown" && !status.is_empty()
        })
        .collect()
}

fn overall_attention_components(
    entry: &crate::status::ProviderStatus,
) -> Vec<&crate::status::ComponentStatus> {
    overall_non_operational_components(entry)
        .into_iter()
        .filter(|component| !component.status.to_lowercase().contains("maint"))
        .collect()
}

fn overall_attention_entries(
    status_app: &super::app::StatusApp,
) -> Vec<&crate::status::ProviderStatus> {
    let mut entries: Vec<_> = status_app
        .entries
        .iter()
        .filter(|entry| {
            !entry.active_incidents().is_empty()
                || !overall_attention_components(entry).is_empty()
                || matches!(
                    entry.health,
                    ProviderHealth::Outage | ProviderHealth::Degraded | ProviderHealth::Unknown
                )
        })
        .collect();
    entries.sort_by(|a, b| a.health.sort_rank().cmp(&b.health.sort_rank()));
    entries
}

fn component_scope_name(component: &crate::status::ComponentStatus) -> String {
    component
        .group_name
        .as_deref()
        .filter(|group| !group.is_empty())
        .unwrap_or(&component.name)
        .to_string()
}

fn component_display_name(component: &crate::status::ComponentStatus) -> String {
    let name = translate_component_name(&component.name);
    match component.group_name.as_deref() {
        Some(group) if !group.is_empty() && group != component.name => {
            format!("{group}: {name}")
        }
        _ => name,
    }
}

fn component_only_scope_title(components: &[&crate::status::ComponentStatus]) -> String {
    let mut scopes: Vec<String> = Vec::new();
    for component in components {
        let scope = component_scope_name(component);
        if !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }

    match scopes.len() {
        0 => "Component-reported service degradation".to_string(),
        1 => scopes[0].clone(),
        _ => "Multiple affected services".to_string(),
    }
}

fn provider_health_label(health: ProviderHealth) -> &'static str {
    match health {
        ProviderHealth::Operational => "operational",
        ProviderHealth::Degraded => "degraded",
        ProviderHealth::Outage => "outage",
        ProviderHealth::Maintenance => "maintenance",
        ProviderHealth::Unknown => "unknown",
    }
}

fn sparse_incident_metadata(incident: &crate::status::ActiveIncident) -> bool {
    incident.created_at.is_none()
        && incident.updated_at.is_none()
        && incident.latest_update.is_none()
        && incident.impact.trim().eq_ignore_ascii_case("none")
        && incident.affected_components.is_empty()
}

fn incident_status_value(incident: &crate::status::ActiveIncident) -> String {
    if sparse_incident_metadata(incident) && incident.status.eq_ignore_ascii_case("investigating") {
        "reported".to_string()
    } else {
        incident.status.clone()
    }
}

fn incident_time_value(
    entry: &crate::status::ProviderStatus,
    incident: &crate::status::ActiveIncident,
) -> Option<(&'static str, String)> {
    if let Some(updated_at) = incident.updated_at.as_deref() {
        return Some(("Updated", format_relative_time_from_str(updated_at)));
    }

    if let Some(update) = incident.latest_update.as_ref() {
        if !update.created_at.trim().is_empty() {
            return Some(("Updated", format_relative_time_from_str(&update.created_at)));
        }
    }

    if let Some(created_at) = incident.created_at.as_deref() {
        return Some(("Reported", format_relative_time_from_str(created_at)));
    }

    provider_last_meaningful_update(entry).map(|(label, value)| {
        let display_label = match label {
            "source updated" => "Source updated",
            "last checked" => "Last checked",
            _ => "Updated",
        };
        (display_label, value)
    })
}

fn incident_impact_style(impact: &str) -> Style {
    let normalized = impact.to_lowercase();
    if normalized.contains("critical") || normalized.contains("major") {
        Style::default().fg(Color::Red)
    } else if normalized.contains("minor") || normalized.contains("partial") {
        Style::default().fg(Color::Yellow)
    } else if normalized.contains("maint") {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn status_field_label_style() -> Style {
    Style::default().fg(Color::Blue)
}

fn status_section_label_style() -> Style {
    Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

fn push_component_scope_lines(
    lines: &mut Vec<Line<'static>>,
    components: &[&crate::status::ComponentStatus],
    max_items: usize,
) {
    if components.is_empty() {
        return;
    }

    lines.push(Line::from(Span::styled(
        "  Services",
        status_section_label_style(),
    )));

    for component in components.iter().take(max_items) {
        lines.push(Line::from(vec![
            Span::styled("    - ", Style::default().fg(Color::DarkGray)),
            Span::raw(component_display_name(component)),
            Span::styled(" (", Style::default().fg(Color::DarkGray)),
            Span::styled(
                component.status.replace('_', " "),
                component_status_style(&component.status),
            ),
            Span::styled(")", Style::default().fg(Color::DarkGray)),
        ]));
    }

    let remaining = components.len().saturating_sub(max_items);
    if remaining > 0 {
        lines.push(Line::from(Span::styled(
            format!("    +{remaining} more affected service(s)"),
            Style::default().fg(Color::DarkGray),
        )));
    }
}

fn push_plain_scope_lines(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    items: &[String],
    max_items: usize,
) {
    if items.is_empty() {
        return;
    }

    lines.push(Line::from(Span::styled(
        format!("  {label}"),
        status_section_label_style(),
    )));

    for item in items.iter().take(max_items) {
        lines.push(Line::from(vec![
            Span::styled("    - ", Style::default().fg(Color::DarkGray)),
            Span::raw(item.clone()),
        ]));
    }

    let remaining = items.len().saturating_sub(max_items);
    if remaining > 0 {
        lines.push(Line::from(Span::styled(
            format!("    +{remaining} more"),
            Style::default().fg(Color::DarkGray),
        )));
    }
}

fn push_wrapped_bullet_lines(
    lines: &mut Vec<Line<'static>>,
    text: &str,
    body_width: usize,
    bullet_indent: &str,
    continuation_indent: &str,
) {
    let available_width = body_width.saturating_sub(continuation_indent.len()).max(12);
    let wrapped = textwrap::wrap(text.trim(), available_width);

    if let Some(first_line) = wrapped.first() {
        lines.push(Line::from(vec![
            Span::styled(
                bullet_indent.to_string(),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(first_line.to_string()),
        ]));
    }

    for line in wrapped.iter().skip(1) {
        lines.push(Line::from(vec![
            Span::raw(continuation_indent.to_string()),
            Span::raw(line.to_string()),
        ]));
    }
}

fn status_verdict_copy(health: ProviderHealth) -> &'static str {
    match health {
        ProviderHealth::Operational => "All systems operational",
        ProviderHealth::Degraded => "Some services degraded",
        ProviderHealth::Outage => "Major service disruption",
        ProviderHealth::Maintenance => "Scheduled maintenance in progress",
        ProviderHealth::Unknown => "Status unavailable",
    }
}

/// Map incident stage to a `ProviderHealth` for accent stripe coloring.
fn incident_stage_health(stage: &str) -> ProviderHealth {
    let normalized = stage.to_lowercase();
    if normalized.contains("resolved") {
        ProviderHealth::Operational
    } else if normalized.contains("monitoring") {
        ProviderHealth::Degraded
    } else if normalized.contains("maint") {
        ProviderHealth::Maintenance
    } else {
        ProviderHealth::Degraded
    }
}

fn incident_stage_style(stage: &str) -> Style {
    let normalized = stage.to_lowercase();
    if normalized.contains("resolved") {
        Style::default().fg(Color::Green)
    } else if normalized.contains("monitoring") {
        Style::default().fg(Color::Cyan)
    } else if normalized.contains("maint") {
        Style::default().fg(Color::Blue)
    } else {
        Style::default().fg(Color::Yellow)
    }
}

pub(in crate::tui) fn draw_status_main(f: &mut Frame, area: Rect, app: &mut App) {
    use super::app::StatusFocus;

    let Some(status_app) = app.status_app.as_mut() else {
        let msg = Paragraph::new("Failed to load status data")
            .block(Block::default().borders(Borders::ALL).title(" Status "));
        f.render_widget(msg, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(0)])
        .split(area);

    let list_border = if status_app.focus == StatusFocus::List {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if status_app.loading {
        format!(
            " Providers ({}) refreshing... ",
            status_app.filtered_entries.len()
        )
    } else if status_app.search_query.is_empty() {
        format!(" Providers ({}) ", status_app.filtered_entries.len())
    } else {
        format!(
            " Providers ({}) [/{query}] ",
            status_app.filtered_entries.len(),
            query = status_app.search_query
        )
    };

    let is_list_focused = status_app.focus == StatusFocus::List;

    // Build list items: Overall at index 0, then providers
    let mut items = Vec::new();

    // Overall entry (always first, display index 0)
    let overall_selected = status_app.list_state.selected() == Some(0);
    let (overall_prefix, overall_style) = if overall_selected {
        (
            if is_list_focused { "> " } else { "  " },
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("  ", Style::default())
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(overall_prefix, overall_style),
        Span::styled("  Overall", overall_style),
    ])));

    // Provider entries (display index 1+)
    for (row_idx, &idx) in status_app.filtered_entries.iter().enumerate() {
        if let Some(entry) = status_app.entries.get(idx) {
            let display_idx = row_idx + 1; // offset for Overall
            let is_selected = status_app.list_state.selected() == Some(display_idx);
            let (prefix, text_style) = if is_selected {
                (caret(is_list_focused), selection_style(true))
            } else {
                ("  ", Style::default())
            };
            let mut spans = vec![
                Span::styled(prefix, text_style),
                Span::styled(
                    status_health_icon(entry.health),
                    status_health_style(entry.health),
                ),
                Span::raw(" "),
                Span::styled(truncate(&entry.display_name, 20), text_style),
            ];
            let issue_count = entry.issue_count();
            if issue_count > 0 {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    issue_count.to_string(),
                    status_health_style(entry.health),
                ));
            }
            items.push(ListItem::new(Line::from(spans)));
        }
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(list_border)
            .title(title),
    );
    f.render_stateful_widget(list, chunks[0], &mut status_app.list_state);

    // Detail area: dispatch based on selection
    let detail_area = chunks[1];

    if status_app.is_overall_selected() {
        draw_overall_dashboard(
            f,
            detail_area,
            status_app,
            status_app.focus == StatusFocus::Details,
        );
    } else if let Some(entry) = status_app.current_entry() {
        let display_name = entry.display_name.clone();
        let health = entry.health;
        let provenance = entry.provenance;
        let error_msg = entry.error_summary();
        let source_name = entry
            .source_label
            .clone()
            .unwrap_or_else(|| "Unavailable".to_string());
        let source_display = if entry.official_url.is_some() {
            format!("{source_name} • official page")
        } else {
            source_name
        };
        let (time_label, time_value) = provider_last_meaningful_update(entry)
            .map(|(label, value)| (title_case_status_time_label(label), value))
            .unwrap_or(("Source updated", "Unknown".to_string()));
        let service_note = entry.detail_state_message(&entry.components_state, "Service details");
        let incident_note = entry.detail_state_message(&entry.incidents_state, "Incident details");
        let maintenance_note =
            entry.detail_state_message(&entry.scheduled_maintenances_state, "Maintenance details");
        let maintenance_problem = entry.scheduled_maintenances_state.is_fetch_failed();
        let caveat = service_note
            .clone()
            .or_else(|| incident_note.clone())
            .or_else(|| entry.user_visible_caveat().map(str::to_string));
        let confirmed_no_components = entry.confirmed_no_components();
        let confirmed_no_incidents = entry.confirmed_no_incidents();
        let active_incidents = sorted_active_incidents(entry);
        let components = sorted_components(entry, &active_incidents);
        let detail_scroll = status_app.detail_scroll;
        let is_detail_focused = status_app.focus == StatusFocus::Details;

        draw_provider_status_detail(
            f,
            detail_area,
            &display_name,
            health,
            provenance,
            &error_msg,
            &source_display,
            time_label,
            &time_value,
            &caveat,
            &service_note,
            &incident_note,
            &maintenance_note,
            confirmed_no_components,
            confirmed_no_incidents,
            maintenance_problem,
            &active_incidents,
            &components,
            &entry.scheduled_maintenances,
            detail_scroll,
            is_detail_focused,
        );
    } else {
        let detail_border = if status_app.focus == StatusFocus::Details {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let paragraph = Paragraph::new(vec![Line::from(Span::styled(
            "Select a provider to view details",
            Style::default().fg(Color::DarkGray),
        ))])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(detail_border)
                .title(" Status "),
        );
        f.render_widget(paragraph, detail_area);
    }
}

/// Sort active incidents by impact severity, then recency.
fn sorted_active_incidents(
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
fn sorted_components<'a>(
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

// ── Overall Dashboard ──────────────────────────────────────────────────

fn format_relative_time_from_instant(instant: std::time::Instant) -> String {
    let elapsed = instant.elapsed();
    let secs = elapsed.as_secs();

    match secs {
        0..=4 => "just now".to_string(),
        5..=59 => format!("{secs}s ago"),
        60..=3599 => format!("{}m ago", secs / 60),
        3600..=86_399 => format!("{}h ago", secs / 3600),
        _ => format!("{}d ago", secs / 86_400),
    }
}

fn overall_freshness_line(status_app: &super::app::StatusApp) -> Line<'static> {
    if status_app.loading {
        return Line::from(vec![
            Span::styled("Refreshing status", Style::default().fg(Color::Yellow)),
            Span::styled("...", Style::default().fg(Color::DarkGray)),
        ]);
    }

    let freshness = status_app
        .last_refreshed
        .map(format_relative_time_from_instant)
        .map(|value| format!("Updated {value}"))
        .unwrap_or_else(|| "Waiting for status refresh".to_string());

    if status_app.last_error.is_some() {
        Line::from(vec![
            Span::styled(freshness, Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("Last refresh failed", Style::default().fg(Color::Red)),
        ])
    } else {
        Line::from(Span::styled(
            freshness,
            Style::default().fg(Color::DarkGray),
        ))
    }
}

fn push_soft_card_summary(card_lines: &mut Vec<Line<'static>>, summary: &str) {
    card_lines.push(Line::from(Span::raw(format!("  {summary}"))));
}

fn normalized_status_copy(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn summary_duplicates_issue(summary: &str, issue: &str) -> bool {
    let normalized_summary = normalized_status_copy(summary);
    let normalized_issue = normalized_status_copy(issue);

    !normalized_summary.is_empty() && normalized_summary == normalized_issue
}

fn update_duplicates_summary_or_issue(update: &str, summary: Option<&str>, issue: &str) -> bool {
    let normalized_update = normalized_status_copy(update);
    if normalized_update.is_empty() {
        return true;
    }

    if summary.is_some_and(|summary| summary_duplicates_issue(summary, update)) {
        return true;
    }

    summary_duplicates_issue(update, issue)
}

fn push_overall_caveat(lines: &mut Vec<Line<'static>>, note: &str, body_width: usize) {
    let _ = body_width;
    lines.push(Line::from(vec![
        Span::styled("  Note: ", status_field_label_style()),
        Span::styled(note.to_string(), Style::default().fg(Color::DarkGray)),
    ]));
}

fn push_panel_empty_state(lines: &mut Vec<Line<'static>>, title: &str, description: &str) {
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default().fg(Color::Green),
    )));
    lines.push(Line::from(Span::styled(
        description.to_string(),
        Style::default().fg(Color::DarkGray),
    )));
}

fn build_incidents_panel_cards(
    entries: &[&crate::status::ProviderStatus],
    body_width: usize,
) -> Vec<SoftCard> {
    let mut cards = Vec::new();

    for entry in entries.iter() {
        let incidents = entry.active_incidents();
        let non_op_components = overall_attention_components(entry);
        let incident = incidents[0];
        let summary = entry.provider_summary_text();

        let mut card_lines = Vec::new();

        card_lines.push(Line::from(vec![
            Span::styled(
                status_health_icon(entry.health),
                status_health_style(entry.health),
            ),
            Span::raw(" "),
            Span::styled(
                entry.display_name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));

        card_lines.push(Line::from(vec![
            Span::styled("  Issue: ", status_field_label_style()),
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

        if let Some((label, value)) = incident_time_value(entry, incident) {
            metadata_spans.push(Span::raw("  "));
            metadata_spans.push(Span::styled(
                format!("{label}: "),
                status_field_label_style(),
            ));
            metadata_spans.push(Span::styled(value, Style::default().fg(Color::Cyan)));
        }
        card_lines.push(Line::from(metadata_spans));

        if incidents.len() > 1 {
            card_lines.push(Line::from(vec![
                Span::styled("  Additional incidents: ", status_field_label_style()),
                Span::styled(
                    format!("{} more", incidents.len() - 1),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        if !non_op_components.is_empty() {
            push_component_scope_lines(&mut card_lines, &non_op_components, 4);
        } else if !incident.affected_components.is_empty() {
            push_plain_scope_lines(
                &mut card_lines,
                "Affected",
                &incident.affected_components,
                4,
            );
        }

        if let Some(update) = &incident.latest_update {
            if !update_duplicates_summary_or_issue(&update.body, summary, &incident.name) {
                card_lines.push(Line::from(Span::styled(
                    "  Latest Update",
                    status_section_label_style(),
                )));
                push_wrapped_bullet_lines(
                    &mut card_lines,
                    &update.body,
                    body_width,
                    "    - ",
                    "      ",
                );
            }
        }

        if let Some(note) = entry.user_visible_caveat() {
            push_overall_caveat(&mut card_lines, note, body_width);
        }

        cards.push(SoftCard::new(entry.health, card_lines));
    }

    cards
}

fn build_degradation_panel_cards(
    entries: &[&crate::status::ProviderStatus],
    body_width: usize,
) -> Vec<SoftCard> {
    let mut cards = Vec::new();

    for entry in entries.iter() {
        let non_op_components = overall_attention_components(entry);

        let mut card_lines = Vec::new();

        card_lines.push(Line::from(vec![
            Span::styled(
                status_health_icon(entry.health),
                status_health_style(entry.health),
            ),
            Span::raw(" "),
            Span::styled(
                entry.display_name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));

        if let Some(summary) = entry.provider_summary_text() {
            push_soft_card_summary(&mut card_lines, summary);
        }

        card_lines.push(Line::from(vec![
            Span::styled("  Scope: ", status_field_label_style()),
            Span::styled(
                component_only_scope_title(&non_op_components),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        card_lines.push(Line::from(vec![
            Span::styled("  Status: ", status_field_label_style()),
            Span::styled(
                provider_health_label(entry.health),
                status_health_style(entry.health),
            ),
            Span::raw("  "),
            Span::styled("Updated: ", status_field_label_style()),
            Span::styled(
                provider_last_meaningful_update(entry)
                    .map(|(_, value)| value)
                    .unwrap_or_else(|| "recently updated".to_string()),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        push_component_scope_lines(&mut card_lines, &non_op_components, 4);

        if let Some(note) = entry.user_visible_caveat() {
            push_overall_caveat(&mut card_lines, note, body_width);
        }

        cards.push(SoftCard::new(entry.health, card_lines));
    }

    cards
}

fn build_maintenance_panel_cards(
    items: &[(&str, &crate::status::ScheduledMaintenance)],
) -> Vec<SoftCard> {
    let mut cards = Vec::new();

    for (provider_name, maint) in items.iter() {
        let mut card_lines = Vec::new();

        card_lines.push(Line::from(vec![
            Span::styled("◆", Style::default().fg(Color::Blue)),
            Span::raw(" "),
            Span::styled(
                provider_name.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        card_lines.push(Line::from(vec![
            Span::styled("  Window: ", status_field_label_style()),
            Span::styled(
                maint.name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));

        let mut bits = vec![Span::styled("  Status: ", status_field_label_style())];
        bits.push(Span::styled(
            maint.status.replace('_', " "),
            component_status_style(&maint.status),
        ));
        if let Some(start) = maint.scheduled_for.as_deref() {
            bits.push(Span::raw("  "));
            bits.push(Span::styled("Scheduled: ", status_field_label_style()));
            bits.push(Span::styled(
                format_relative_time_from_str(start),
                Style::default().fg(Color::Cyan),
            ));
        }
        card_lines.push(Line::from(bits));

        if !maint.affected_components.is_empty() {
            push_plain_scope_lines(&mut card_lines, "Affected", &maint.affected_components, 3);
        }

        cards.push(SoftCard::new(ProviderHealth::Maintenance, card_lines));
    }

    cards
}

fn incidents_empty_lines() -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    push_panel_empty_state(
        &mut lines,
        "No active incidents reported right now",
        "Tracked providers are not currently publishing formal incident rows.",
    );
    lines
}

fn degradation_empty_lines() -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    push_panel_empty_state(
        &mut lines,
        "No component-reported degradation right now",
        "Tracked providers are not currently reporting degraded services without incident rows.",
    );
    lines
}

fn render_overall_panel(
    f: &mut Frame,
    area: Rect,
    title: &str,
    cards: Vec<SoftCard>,
    empty_lines: Vec<Line<'static>>,
    scroll: u16,
    focused: bool,
) {
    if cards.is_empty() {
        ScrollablePanel::new(title, empty_lines, scroll, focused).render(f, area);
    } else {
        ScrollablePanel::with_cards(title, cards, scroll, focused).render(f, area);
    }
}

fn draw_overall_dashboard(
    f: &mut Frame,
    area: Rect,
    status_app: &super::app::StatusApp,
    is_focused: bool,
) {
    let (op, deg, out, other) = status_app.health_counts();
    let total = status_app.entries.len();
    let attention_entries = overall_attention_entries(status_app);
    let incident_entries: Vec<_> = attention_entries
        .iter()
        .copied()
        .filter(|entry| !entry.active_incidents().is_empty())
        .collect();
    let component_entries: Vec<_> = attention_entries
        .iter()
        .copied()
        .filter(|entry| entry.active_incidents().is_empty())
        .collect();
    let all_maint = status_app.all_maintenances();
    let maintenance_visible = !all_maint.is_empty();
    let dark_border = Style::default().fg(Color::DarkGray);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    {
        let ratio = if total > 0 {
            op as f64 / total as f64
        } else {
            0.0
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(dark_border)
            .title(" Overall Status ");
        let inner = block.inner(rows[0]);
        f.render_widget(block, rows[0]);

        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let gauge = LineGauge::default()
            .filled_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
            .ratio(ratio)
            .label(format!(" {op}/{total}  {:.0}% ", ratio * 100.0));
        f.render_widget(gauge, inner_chunks[0]);

        let mut summary_spans = vec![
            Span::styled("● ", Style::default().fg(Color::Green)),
            Span::raw(format!("{op} operational  ")),
        ];
        if deg > 0 {
            summary_spans.push(Span::styled("◐ ", Style::default().fg(Color::Yellow)));
            summary_spans.push(Span::raw(format!("{deg} degraded  ")));
        }
        if out > 0 {
            summary_spans.push(Span::styled("✗ ", Style::default().fg(Color::Red)));
            summary_spans.push(Span::raw(format!("{out} outage  ")));
        }
        if other > 0 {
            summary_spans.push(Span::styled("? ", Style::default().fg(Color::DarkGray)));
            summary_spans.push(Span::raw(format!("{other} other  ")));
        }
        f.render_widget(Paragraph::new(Line::from(summary_spans)), inner_chunks[1]);
        f.render_widget(
            Paragraph::new(overall_freshness_line(status_app)),
            inner_chunks[2],
        );
    }

    let board_area = rows[1];
    let stacked_layout = board_area.width < 100;

    if stacked_layout {
        let mut constraints = vec![Constraint::Percentage(55), Constraint::Percentage(45)];
        if maintenance_visible {
            constraints = vec![
                Constraint::Percentage(42),
                Constraint::Percentage(34),
                Constraint::Percentage(24),
            ];
        }
        let panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(board_area);

        let incident_cards = build_incidents_panel_cards(
            &incident_entries,
            usize::from(panels[0].width.saturating_sub(4)).max(24),
        );
        render_overall_panel(
            f,
            panels[0],
            "Active Incidents",
            incident_cards,
            incidents_empty_lines(),
            status_app.overall_incidents_scroll,
            is_focused
                && status_app.overall_panel_focus == super::app::OverallPanelFocus::Incidents,
        );

        let degradation_cards = build_degradation_panel_cards(
            &component_entries,
            usize::from(panels[1].width.saturating_sub(4)).max(24),
        );
        render_overall_panel(
            f,
            panels[1],
            "Service Degradation",
            degradation_cards,
            degradation_empty_lines(),
            status_app.overall_degradation_scroll,
            is_focused
                && status_app.overall_panel_focus == super::app::OverallPanelFocus::Degradation,
        );

        if maintenance_visible {
            let maintenance_cards = build_maintenance_panel_cards(&all_maint);
            render_overall_panel(
                f,
                panels[2],
                "Maintenance Outlook",
                maintenance_cards,
                Vec::new(),
                status_app.overall_maintenance_scroll,
                is_focused
                    && status_app.overall_panel_focus == super::app::OverallPanelFocus::Maintenance,
            );
        }
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(board_area);
        let right_panels = if maintenance_visible {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(columns[1])
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0)])
                .split(columns[1])
        };

        let incident_cards = build_incidents_panel_cards(
            &incident_entries,
            usize::from(columns[0].width.saturating_sub(4)).max(24),
        );
        render_overall_panel(
            f,
            columns[0],
            "Active Incidents",
            incident_cards,
            incidents_empty_lines(),
            status_app.overall_incidents_scroll,
            is_focused
                && status_app.overall_panel_focus == super::app::OverallPanelFocus::Incidents,
        );

        let degradation_cards = build_degradation_panel_cards(
            &component_entries,
            usize::from(right_panels[0].width.saturating_sub(4)).max(24),
        );
        render_overall_panel(
            f,
            right_panels[0],
            "Service Degradation",
            degradation_cards,
            degradation_empty_lines(),
            status_app.overall_degradation_scroll,
            is_focused
                && status_app.overall_panel_focus == super::app::OverallPanelFocus::Degradation,
        );

        if maintenance_visible {
            let maintenance_cards = build_maintenance_panel_cards(&all_maint);
            render_overall_panel(
                f,
                right_panels[1],
                "Maintenance Outlook",
                maintenance_cards,
                Vec::new(),
                status_app.overall_maintenance_scroll,
                is_focused
                    && status_app.overall_panel_focus == super::app::OverallPanelFocus::Maintenance,
            );
        }
    }
}

// ── Individual Provider Detail (4 subpanels) ───────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_provider_status_detail(
    f: &mut Frame,
    area: Rect,
    display_name: &str,
    health: ProviderHealth,
    provenance: StatusProvenance,
    error_msg: &Option<String>,
    source_display: &str,
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
    detail_scroll: u16,
    is_focused: bool,
) {
    let dark_border = Style::default().fg(Color::DarkGray);

    // Compute dynamic subpanel heights
    // Base: 4 content lines (name, verdict, issue_summary, source) + 2 borders = 6
    let mut status_h: u16 = 6;
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
    let maint_lines = if has_maintenance {
        if scheduled_maintenances.is_empty() {
            1
        } else {
            (scheduled_maintenances.len() * 2) as u16
        }
    } else {
        0
    };

    let mut constraints: Vec<Constraint> = vec![Constraint::Length(status_h)];
    if has_components {
        constraints.push(Constraint::Length(service_rows + 2)); // rows + borders
    }
    constraints.push(Constraint::Min(0)); // Incidents (scrollable)
    if has_maintenance {
        constraints.push(Constraint::Length(maint_lines + 2)); // lines + borders
    }

    let panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // ── Status header ──────────────────────────────────────────
    {
        let status_area = panel_chunks[chunk_idx];
        chunk_idx += 1;

        let issue_summary = match (
            incident_note.as_deref(),
            active_incidents.len(),
            scheduled_maintenances.len(),
        ) {
            (Some(note), 0, 0) => note.to_string(),
            (Some(note), 0, maintenance) => format!(
                "{note} • {maintenance} maintenance item{}",
                if maintenance == 1 { "" } else { "s" }
            ),
            (None, 0, 0) => "0 active incidents".to_string(),
            (None, incidents, 0) => format!(
                "{incidents} active incident{}",
                if incidents == 1 { "" } else { "s" }
            ),
            (None, 0, maintenance) => format!(
                "0 active incidents • {maintenance} maintenance item{}",
                if maintenance == 1 { "" } else { "s" }
            ),
            (None, incidents, maintenance) => format!(
                "{incidents} active incident{} • {maintenance} maintenance item{}",
                if incidents == 1 { "" } else { "s" },
                if maintenance == 1 { "" } else { "s" },
            ),
            (Some(note), incidents, maintenance) => format!(
                "{incidents} active incident{} • {maintenance} maintenance item{} • {note}",
                if incidents == 1 { "" } else { "s" },
                if maintenance == 1 { "" } else { "s" },
            ),
        };
        let support_line = format!("Source: {source_display} • {time_label}: {time_value}");

        let mut lines: Vec<Line<'static>> = vec![
            Line::from(vec![
                Span::styled(status_health_icon(health), status_health_style(health)),
                Span::raw(" "),
                Span::styled(
                    display_name.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                status_verdict_copy(health),
                status_health_style(health).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                issue_summary,
                if active_incidents.is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                },
            )),
            Line::from(Span::styled(
                support_line,
                Style::default().fg(Color::DarkGray),
            )),
        ];

        // Optional caveat/notes line
        if let Some(caveat_text) = caveat {
            lines.push(Line::from(Span::styled(
                caveat_text.clone(),
                Style::default().fg(Color::Yellow),
            )));
        } else if provenance == StatusProvenance::Unavailable {
            lines.push(Line::from(Span::styled(
                "Status unavailable",
                Style::default().fg(Color::Yellow),
            )));
        }
        if let Some(err) = error_msg {
            lines.push(Line::from(Span::styled(
                err.clone(),
                Style::default().fg(Color::Red),
            )));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(dark_border)
            .title(" Status ");
        f.render_widget(Paragraph::new(lines).block(block), status_area);
    }

    // ── Services (non-operational highlighted, healthy summarized) ─
    if has_components {
        let services_area = panel_chunks[chunk_idx];
        chunk_idx += 1;

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
        } else {
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

            if healthy_comp_count > 0 {
                lines.push(Line::from(vec![
                    Span::styled("●", Style::default().fg(Color::Green)),
                    Span::raw(format!(
                        " {} service{} operational",
                        healthy_comp_count,
                        if healthy_comp_count == 1 { "" } else { "s" }
                    )),
                ]));
            }
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(dark_border)
            .title(format!(" Services ({}) ", components.len()));
        f.render_widget(Paragraph::new(lines).block(block), services_area);
    }

    // ── Current Incidents (scrollable, focusable) ──────────────
    {
        let incidents_area = panel_chunks[chunk_idx];
        chunk_idx += 1;

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
                let mut detail_bits = vec![incident.status.clone()];
                if let Some(updated_at) = incident
                    .updated_at
                    .as_deref()
                    .or(incident.created_at.as_deref())
                {
                    detail_bits.push(format_relative_time_from_str(updated_at));
                }
                if !incident.affected_components.is_empty() {
                    detail_bits.push(incident.affected_components.join(", "));
                }
                card_lines.push(Line::from(Span::styled(
                    format!("  {}", detail_bits.join(" • ")),
                    Style::default().fg(Color::DarkGray),
                )));
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
    }

    // ── Maintenance ────────────────────────────────────────────
    if has_maintenance {
        let maint_area = panel_chunks[chunk_idx];
        let mut lines: Vec<Line<'static>> = Vec::new();
        if maintenance_problem {
            lines.push(Line::from(Span::styled(
                maintenance_note
                    .clone()
                    .unwrap_or_else(|| "Maintenance details failed to load".to_string()),
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for maint in scheduled_maintenances {
                lines.push(Line::from(vec![
                    Span::styled("◆ ", Style::default().fg(Color::Blue)),
                    Span::styled(
                        maint.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]));
                let mut maint_bits = vec![maint.status.clone()];
                if let Some(start) = maint.scheduled_for.as_deref() {
                    maint_bits.push(format_relative_time_from_str(start));
                }
                if !maint.affected_components.is_empty() {
                    maint_bits.push(maint.affected_components.join(", "));
                }
                lines.push(Line::from(Span::styled(
                    format!("  {}", maint_bits.join(" • ")),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(dark_border)
            .title(format!(" Maintenance ({}) ", scheduled_maintenances.len()));
        f.render_widget(Paragraph::new(lines).block(block), maint_area);
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Instant};

    use ratatui::{backend::TestBackend, Terminal};

    use super::*;
    use crate::{
        agents::AgentsFile,
        benchmarks::BenchmarkStore,
        status::{
            ActiveIncident, ComponentStatus, IncidentUpdate, ProviderStatus, ScheduledMaintenance,
            StatusProvenance, StatusSourceMethod, StatusSupportTier,
        },
        tui::app::{App, Tab},
    };

    fn make_status_app(entry: ProviderStatus) -> App {
        let mut app = App::new(
            HashMap::new(),
            Some(&AgentsFile {
                schema_version: 1,
                last_scraped: None,
                scrape_source: None,
                agents: HashMap::new(),
            }),
            None,
            BenchmarkStore::empty(),
        );
        app.current_tab = Tab::Status;
        let status_app = app.status_app.as_mut().expect("status app");
        status_app.entries = vec![entry];
        status_app.loading = false;
        status_app.last_refreshed = Some(Instant::now());
        status_app.update_filtered();
        // Select the first provider (display index 1; index 0 = Overall)
        status_app.selected = 1;
        status_app.list_state.select(Some(1));
        app
    }

    fn sample_provider_status() -> ProviderStatus {
        ProviderStatus {
            slug: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            source_slug: "openai".to_string(),
            support_tier: StatusSupportTier::Required,
            health: ProviderHealth::Degraded,
            provenance: StatusProvenance::Fallback,
            load_state: crate::status::StatusLoadState::Loaded,
            source_label: Some("API Status Check".to_string()),
            source_method: Some(StatusSourceMethod::ApiStatusCheck),
            official_url: Some("https://status.openai.com".to_string()),
            fallback_url: Some("https://apistatuscheck.com/openai".to_string()),
            source_updated_at: Some("2026-03-16T23:55:00Z".to_string()),
            provider_summary: Some("Elevated API errors affecting chat completions.".to_string()),
            status_note: Some(
                "Fallback adapter exposes only provider-level summary status.".to_string(),
            ),
            components: vec![
                ComponentStatus {
                    name: "API".to_string(),
                    status: "partial_outage".to_string(),
                    group_name: None,
                },
                ComponentStatus {
                    name: "Auth".to_string(),
                    status: "operational".to_string(),
                    group_name: None,
                },
            ],
            components_state: crate::status::StatusDetailState {
                availability: crate::status::StatusDetailAvailability::Available,
                source: crate::status::StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            incidents: vec![ActiveIncident {
                name: "Elevated API errors".to_string(),
                status: "investigating".to_string(),
                impact: "minor".to_string(),
                shortlink: None,
                created_at: Some("2026-03-16T23:40:00Z".to_string()),
                updated_at: Some("2026-03-16T23:58:00Z".to_string()),
                latest_update: Some(IncidentUpdate {
                    status: "investigating".to_string(),
                    body: "We are investigating elevated error rates for API requests.".to_string(),
                    created_at: "2026-03-16T23:58:00Z".to_string(),
                }),
                affected_components: vec!["API".to_string()],
            }],
            incidents_state: crate::status::StatusDetailState {
                availability: crate::status::StatusDetailAvailability::Available,
                source: crate::status::StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            scheduled_maintenances: vec![ScheduledMaintenance {
                name: "Database maintenance".to_string(),
                status: "scheduled".to_string(),
                impact: "none".to_string(),
                scheduled_for: Some("2026-03-17T03:00:00Z".to_string()),
                scheduled_until: Some("2026-03-17T04:00:00Z".to_string()),
                affected_components: vec!["Auth".to_string()],
            }],
            scheduled_maintenances_state: crate::status::StatusDetailState {
                availability: crate::status::StatusDetailAvailability::Available,
                source: crate::status::StatusDetailSource::Inline,
                note: None,
                error: None,
            },
            official_error: None,
            fallback_error: None,
        }
    }

    fn render_status_buffer_with_size(
        app: &mut App,
        width: u16,
        height: u16,
    ) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| draw_status_main(frame, frame.area(), app))
            .expect("draw succeeds");
        terminal.backend().buffer().clone()
    }

    fn render_status_text_with_size(app: &mut App, width: u16, height: u16) -> String {
        let buffer = render_status_buffer_with_size(app, width, height);
        let mut lines = Vec::new();
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                line.push_str(buffer[(x, y)].symbol());
            }
            lines.push(line);
        }
        lines.join("\n")
    }

    fn render_status_text(app: &mut App) -> String {
        render_status_text_with_size(app, 140, 40)
    }

    #[test]
    fn status_detail_reads_like_a_status_page() {
        let mut app = make_status_app(sample_provider_status());

        let rendered = render_status_text(&mut app);

        assert!(rendered.contains("Status"));
        assert!(rendered.contains("Some services degraded"));
        assert!(rendered.contains("API Status Check"));
        assert!(rendered.contains("official page"));
        assert!(rendered.contains("1 active incident"));
        assert!(!rendered.contains("Narrative"));
        assert!(!rendered.contains("Status page"));
        assert!(rendered.contains("Current Incidents"));
        assert!(rendered.contains("Services"));
        assert!(rendered.contains("Database maintenance"));
        assert!(!rendered.contains("Tracking:"));
        assert!(!rendered.contains("Agents:"));
        assert!(!rendered.contains("confidence"));
        assert!(!rendered.contains("coverage"));
        assert!(!rendered.contains("freshness"));
        assert!(!rendered.contains("contradiction"));
        assert!(!rendered.contains("R/FB"));
    }

    #[test]
    fn operational_status_hides_affected_right_now_summary() {
        let mut entry = sample_provider_status();
        entry.health = ProviderHealth::Operational;
        entry.provenance = StatusProvenance::Official;
        entry.provider_summary = Some("All systems operational".to_string());
        entry.incidents.clear();
        entry.scheduled_maintenances.clear();
        entry.incidents_state.availability = crate::status::StatusDetailAvailability::NoneReported;
        entry.scheduled_maintenances_state.availability =
            crate::status::StatusDetailAvailability::NoneReported;
        for component in &mut entry.components {
            component.status = "operational".to_string();
        }

        let mut app = make_status_app(entry);
        let rendered = render_status_text(&mut app);

        assert!(rendered.contains("All systems operational"));
        assert!(!rendered.contains("Affected right now:"));
    }

    #[test]
    fn summary_only_status_hides_services_section_and_shows_service_note() {
        let mut entry = sample_provider_status();
        entry.health = ProviderHealth::Operational;
        entry.provenance = StatusProvenance::Official;
        entry.source_method = Some(StatusSourceMethod::ApiStatusCheck);
        entry.provider_summary = Some("All systems operational".to_string());
        entry.components.clear();
        entry.incidents.clear();
        entry.scheduled_maintenances.clear();
        entry.components_state = crate::status::StatusDetailState {
            availability: crate::status::StatusDetailAvailability::Unsupported,
            source: crate::status::StatusDetailSource::SummaryOnly,
            note: Some("Service details unavailable".to_string()),
            error: None,
        };
        entry.incidents_state = crate::status::StatusDetailState {
            availability: crate::status::StatusDetailAvailability::Unsupported,
            source: crate::status::StatusDetailSource::SummaryOnly,
            note: Some("Incident details unavailable".to_string()),
            error: None,
        };
        entry.scheduled_maintenances_state = crate::status::StatusDetailState {
            availability: crate::status::StatusDetailAvailability::Unsupported,
            source: crate::status::StatusDetailSource::SummaryOnly,
            note: Some("Maintenance details unavailable".to_string()),
            error: None,
        };

        let mut app = make_status_app(entry);
        let rendered = render_status_text(&mut app);

        assert!(rendered.contains("Service details unavailable"));
        assert!(rendered.contains("Last checked"));
        assert!(!rendered.contains("Affected right now:"));
    }

    #[test]
    fn incident_driven_status_uses_latest_event_label() {
        let mut app = make_status_app(sample_provider_status());

        let rendered = render_status_text(&mut app);

        assert!(rendered.contains("Latest event"));
        assert!(!rendered.contains("updated 23"));
    }

    #[test]
    fn provider_list_stays_navigation_focused() {
        let mut app = make_status_app(sample_provider_status());

        let rendered = render_status_text(&mut app);

        assert!(rendered.contains("Providers (1)"));
        assert!(rendered.contains("OpenAI 1"));
        assert!(!rendered.contains("R/"));
        assert!(!rendered.contains("/FB"));
        assert!(!rendered.contains("/OFF"));
        assert!(!rendered.contains("/MISS"));
    }

    #[test]
    fn overall_dashboard_prioritizes_attention_details_over_signal_quality() {
        let mut app = make_status_app(sample_provider_status());
        let status_app = app.status_app.as_mut().expect("status app");
        status_app.selected = 0;
        status_app.list_state.select(Some(0));

        let rendered = render_status_text(&mut app);

        assert!(rendered.contains("Overall Status"));
        assert!(rendered.contains("Active Incidents"));
        assert!(rendered.contains("Service Degradation"));
        assert!(rendered.contains("Maintenance Outlook"));
        assert!(rendered.contains("Updated just now"));
        assert!(rendered.contains("Elevated API errors"));
        assert!(rendered.contains("investigating"));
        assert!(rendered.contains("Services"));
        assert!(rendered.contains("API (partial outage)"));
        assert!(rendered.contains("Update"));
        assert!(!rendered.contains("Signal Quality"));
        assert!(!rendered.contains("Active Issues"));
        assert!(!rendered.contains("need attention •"));
    }

    #[test]
    fn overall_dashboard_uses_stacked_panels_on_narrow_widths() {
        let mut app = make_status_app(sample_provider_status());
        let status_app = app.status_app.as_mut().expect("status app");
        status_app.selected = 0;
        status_app.list_state.select(Some(0));

        let rendered = render_status_text_with_size(&mut app, 90, 40);

        assert!(rendered.contains("Active Incidents"));
        assert!(rendered.contains("Service Degradation"));
        assert!(rendered.contains("Maintenance Outlook"));
    }

    #[test]
    fn overall_incident_card_avoids_repeating_summary_as_issue_and_update() {
        let mut entry = sample_provider_status();
        entry.provider_summary = Some("Elevated API errors".to_string());
        entry.incidents[0].name = "Elevated API errors".to_string();
        entry.incidents[0].latest_update = Some(IncidentUpdate {
            status: "investigating".to_string(),
            body: "Elevated API errors".to_string(),
            created_at: "2026-03-16T23:58:00Z".to_string(),
        });

        let mut app = make_status_app(entry);
        let status_app = app.status_app.as_mut().expect("status app");
        status_app.selected = 0;
        status_app.list_state.select(Some(0));

        let rendered = render_status_text(&mut app);

        assert!(rendered.contains("Elevated API errors"));
        assert!(rendered.contains("Issue: Elevated API errors"));
        assert!(!rendered.contains("Latest Update"));
        assert!(!rendered.contains("  Elevated API errors\n"));
    }

    #[test]
    fn overall_update_renders_as_labeled_block() {
        let mut entry = sample_provider_status();
        entry.provider_summary = Some("Distinct summary".to_string());
        entry.incidents[0].name = "Distinct issue".to_string();
        entry.incidents[0].latest_update = Some(IncidentUpdate {
            status: "investigating".to_string(),
            body: "This is a long update message that should wrap onto another rendered line in the incidents panel for styling verification.".to_string(),
            created_at: "2026-03-16T23:58:00Z".to_string(),
        });

        let mut app = make_status_app(entry);
        let status_app = app.status_app.as_mut().expect("status app");
        status_app.selected = 0;
        status_app.list_state.select(Some(0));

        let rendered = render_status_text_with_size(&mut app, 100, 40);

        assert!(rendered.contains("Latest Update"));
        assert!(rendered.contains("- This is a long update message"));
    }
}
