use ratatui::text::Line;

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

    /// Consume the card, returning content lines and health.
    ///
    /// The returned lines have no accent prefix — wrapping is handled by
    /// `Paragraph` when rendered. No separator is included; the caller
    /// is responsible for adding separators between cards.
    pub fn into_parts(self) -> (Vec<Line<'static>>, ProviderHealth) {
        (self.lines, self.health)
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

    #[test]
    fn into_parts_returns_content_lines() {
        let card = SoftCard::new(
            ProviderHealth::Degraded,
            vec![Line::from("Line one"), Line::from("Line two")],
        );
        let (lines, health) = card.into_parts();
        assert_eq!(lines.len(), 2);
        assert!(matches!(health, ProviderHealth::Degraded));
    }

    #[test]
    fn into_parts_preserves_health() {
        let card = SoftCard::new(ProviderHealth::Outage, vec![Line::from("content")]);
        let (lines, health) = card.into_parts();
        assert_eq!(lines.len(), 1);
        assert!(matches!(health, ProviderHealth::Outage));
    }

    #[test]
    fn into_parts_empty_card() {
        let card = SoftCard::new(ProviderHealth::Operational, vec![]);
        let (lines, _) = card.into_parts();
        assert_eq!(lines.len(), 0);
    }
}
