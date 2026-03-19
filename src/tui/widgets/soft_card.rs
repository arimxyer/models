use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::status::ProviderHealth;

/// A card with a health-colored left-edge accent stripe.
///
/// Produces raw content lines that are fed to `Paragraph` for native wrapping.
/// The accent stripe is painted post-render by `ScrollablePanel` using `AccentRegion`.
pub struct SoftCard {
    health: ProviderHealth,
    lines: Vec<Line<'static>>,
}

impl SoftCard {
    pub fn new(health: ProviderHealth, lines: Vec<Line<'static>>) -> Self {
        Self { health, lines }
    }

    /// Consume the card, returning content lines (with separator) and health.
    ///
    /// The returned lines have no accent prefix — wrapping is handled by
    /// `Paragraph` when rendered. A DarkGray separator `─` is appended as the
    /// last line.
    pub fn into_parts(self) -> (Vec<Line<'static>>, ProviderHealth) {
        let mut lines = self.lines;
        // Single-char separator — Paragraph clips to width, no over-wrap
        lines.push(Line::from(Span::styled(
            "\u{2500}".repeat(200),
            Style::default().fg(ratatui::style::Color::DarkGray),
        )));
        (lines, self.health)
    }
}

/// A range of logical lines that should have an accent stripe painted.
pub struct AccentRegion {
    /// First logical line index (inclusive).
    pub start_line: usize,
    /// Last logical line index (exclusive).
    pub end_line: usize,
    /// Health determines accent stripe color.
    pub health: ProviderHealth,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn into_parts_returns_content_plus_separator() {
        let card = SoftCard::new(
            ProviderHealth::Degraded,
            vec![Line::from("Line one"), Line::from("Line two")],
        );
        let (lines, health) = card.into_parts();
        // 2 content lines + 1 separator = 3
        assert_eq!(lines.len(), 3);
        assert!(matches!(health, ProviderHealth::Degraded));
        // Last line is the separator
        let sep = &lines[2];
        assert!(sep.spans[0].content.contains('\u{2500}'));
        assert_eq!(sep.spans[0].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn into_parts_preserves_health() {
        let card = SoftCard::new(ProviderHealth::Outage, vec![Line::from("content")]);
        let (lines, health) = card.into_parts();
        assert_eq!(lines.len(), 2); // content + separator
        assert!(matches!(health, ProviderHealth::Outage));
    }

    #[test]
    fn into_parts_empty_card() {
        let card = SoftCard::new(ProviderHealth::Operational, vec![]);
        let (lines, _) = card.into_parts();
        // Just the separator
        assert_eq!(lines.len(), 1);
    }
}
