use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::status::ProviderHealth;
use crate::tui::ui::status_health_style;

/// A card with a health-colored left-edge accent stripe.
///
/// Pre-wraps content so every visual row gets its own accent stripe,
/// fixing the bug where ratatui's `Paragraph` wrapping loses the stripe
/// on continuation lines.
pub struct SoftCard {
    health: ProviderHealth,
    lines: Vec<Line<'static>>,
}

impl SoftCard {
    pub fn new(health: ProviderHealth, lines: Vec<Line<'static>>) -> Self {
        Self { health, lines }
    }

    /// Pre-wrap each line to fit `available_width`, prepend accent stripe
    /// to every visual row, and append a DarkGray separator line.
    ///
    /// The returned `Vec<Line>` can be fed directly into a `Paragraph`.
    pub fn to_lines(&self, available_width: u16) -> Vec<Line<'static>> {
        let accent_style = status_health_style(self.health);
        // Content width after the "▎ " accent prefix (2 columns)
        let content_width = (available_width as usize).saturating_sub(2);
        let mut result = Vec::new();

        for line in &self.lines {
            let visual_rows = pre_wrap_line(line, content_width);
            for row in visual_rows {
                result.push(prepend_accent(row, accent_style));
            }
        }

        // Append separator: accent-striped DarkGray ─ line
        let sep_width = content_width;
        let sep_line = Line::from(Span::styled(
            "\u{2500}".repeat(sep_width),
            Style::default().fg(ratatui::style::Color::DarkGray),
        ));
        result.push(prepend_accent(sep_line, accent_style));

        result
    }

    /// Total visual row count including separator, for scroll math.
    #[allow(dead_code)]
    pub fn height(&self, available_width: u16) -> u16 {
        let content_width = (available_width as usize).saturating_sub(2);
        let content_rows: u16 = self
            .lines
            .iter()
            .map(|line| visual_row_count(line, content_width))
            .sum();
        // +1 for the separator line
        content_rows + 1
    }
}

/// Prepend `"▎ "` with the given style to a line.
fn prepend_accent(line: Line<'static>, accent_style: Style) -> Line<'static> {
    let mut spans = vec![Span::styled("▎ ", accent_style)];
    spans.extend(line.spans);
    Line::from(spans)
}

/// Count how many visual rows a line occupies at the given width.
fn visual_row_count(line: &Line<'_>, wrap_width: usize) -> u16 {
    let line_width = line.width();
    if wrap_width == 0 || line_width == 0 {
        1
    } else {
        line_width.div_ceil(wrap_width).max(1) as u16
    }
}

/// Clone a `Line` into a `'static` owned version by converting all span content to owned strings.
fn owned_line(line: &Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .iter()
            .map(|s| Span::styled(s.content.to_string(), s.style))
            .collect::<Vec<_>>(),
    )
}

/// Pre-wrap a styled `Line` into multiple `Line`s that each fit within `max_width`.
///
/// For lines that fit, returns them as-is. For lines that overflow, flattens
/// spans to plain text, wraps with `textwrap`, and re-applies a simplified
/// style (inheriting the first span's style). This is acceptable because long
/// lines in cards are typically single-style (update text, component names).
fn pre_wrap_line(line: &Line<'_>, max_width: usize) -> Vec<Line<'static>> {
    if max_width == 0 {
        return vec![owned_line(line)];
    }

    let line_width = line.width();
    if line_width <= max_width {
        return vec![owned_line(line)];
    }

    // Flatten all spans to plain text for wrapping
    let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    // Inherit style from the first span (or default)
    let style = line.spans.first().map(|s| s.style).unwrap_or_default();

    let wrapped = textwrap::wrap(&full_text, max_width);
    wrapped
        .into_iter()
        .map(|cow| Line::from(Span::styled(cow.into_owned(), style)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn soft_card_basic_output() {
        let card = SoftCard::new(
            ProviderHealth::Degraded,
            vec![Line::from("Line one"), Line::from("Line two")],
        );
        let lines = card.to_lines(40);
        // 2 content lines + 1 separator = 3
        assert_eq!(lines.len(), 3);
        // Each line should start with "▎ "
        for line in &lines {
            let first_span = &line.spans[0];
            assert_eq!(first_span.content.as_ref(), "▎ ");
        }
    }

    #[test]
    fn soft_card_height_matches_to_lines() {
        let card = SoftCard::new(
            ProviderHealth::Operational,
            vec![Line::from("Short"), Line::from("Another short line")],
        );
        let width = 50;
        let lines = card.to_lines(width);
        assert_eq!(card.height(width), lines.len() as u16);
    }

    #[test]
    fn soft_card_wraps_long_lines() {
        let long_text = "a ".repeat(30); // 60 chars
        let card = SoftCard::new(ProviderHealth::Outage, vec![Line::from(long_text)]);
        // available_width=22, content_width=20, so 60 chars wraps to 3 visual rows
        let lines = card.to_lines(22);
        // 3 wrapped rows + 1 separator = 4
        assert_eq!(lines.len(), 4);
        assert_eq!(card.height(22), 4);
    }

    #[test]
    fn soft_card_separator_is_dark_gray() {
        let card = SoftCard::new(ProviderHealth::Operational, vec![Line::from("Content")]);
        let lines = card.to_lines(30);
        let sep = lines.last().unwrap();
        // Second span (after accent) should be the ─ separator
        let sep_span = &sep.spans[1];
        assert!(sep_span.content.contains('\u{2500}'));
        assert_eq!(sep_span.style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn pre_wrap_preserves_short_lines() {
        let line = Line::from("short");
        let result = pre_wrap_line(&line, 20);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn pre_wrap_splits_long_lines() {
        let line = Line::from("word ".repeat(10).trim().to_string());
        let result = pre_wrap_line(&line, 15);
        assert!(result.len() > 1);
    }

    #[test]
    fn visual_row_count_basics() {
        assert_eq!(visual_row_count(&Line::from(""), 20), 1);
        assert_eq!(visual_row_count(&Line::from("hello"), 20), 1);
        assert_eq!(visual_row_count(&Line::from("hello"), 0), 1);
        // 10 chars in width 5 = 2 rows
        assert_eq!(visual_row_count(&Line::from("0123456789"), 5), 2);
    }
}
