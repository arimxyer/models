use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Paragraph, Widget, Wrap},
};

use crate::status::ProviderHealth;
use crate::tui::ui::status_health_style;

/// A card with a health-colored left-edge accent stripe that renders itself.
///
/// Renders content as a wrapped `Paragraph` indented 2 columns from the left,
/// paints a health-colored `"▎"` accent stripe on every visual row (including
/// wrapped continuation lines), and draws a DarkGray `─` separator on the last row.
pub struct SoftCard {
    health: ProviderHealth,
    lines: Vec<Line<'static>>,
}

impl SoftCard {
    pub fn new(health: ProviderHealth, lines: Vec<Line<'static>>) -> Self {
        Self { health, lines }
    }

    /// Compute the total height this card will occupy when rendered at `available_width`.
    ///
    /// Returns content visual rows (accounting for word-wrapping) + 1 separator row.
    /// Content width is `available_width - 2` (accent indent).
    pub fn height(&self, available_width: u16) -> u16 {
        let content_width = available_width.saturating_sub(2) as usize;
        let content_rows = self.visual_line_count(content_width);
        content_rows + 1 // +1 for separator
    }

    /// Count visual lines accounting for wrapping at the given content width.
    fn visual_line_count(&self, content_width: usize) -> u16 {
        if content_width == 0 || self.lines.is_empty() {
            return self.lines.len() as u16;
        }
        let p = Paragraph::new(self.lines.clone()).wrap(Wrap { trim: false });
        (p.line_count(content_width as u16) as u16).max(self.lines.len() as u16)
    }
}

impl Widget for SoftCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let accent_style = status_health_style(self.health);
        let separator_row = area.y + area.height.saturating_sub(1);
        let content_height = area.height.saturating_sub(1); // rows available for content

        // Render content paragraph into indented sub-rect (columns 2..)
        if content_height > 0 && area.width > 2 {
            let content_area = Rect {
                x: area.x + 2,
                y: area.y,
                width: area.width - 2,
                height: content_height,
            };
            let paragraph = Paragraph::new(self.lines).wrap(Wrap { trim: false });
            paragraph.render(content_area, buf);
        }

        // Paint accent stripe on every content row
        for row in 0..content_height {
            let y = area.y + row;
            buf.set_string(area.x, y, "\u{258e}", accent_style);
        }

        // Bottom separator: DarkGray ─ across full width
        let sep_style = Style::default().fg(Color::DarkGray);
        for x in 0..area.width {
            buf.set_string(area.x + x, separator_row, "\u{2500}", sep_style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;
    use ratatui::text::Span;

    /// Helper: render a SoftCard into a fresh buffer and return it.
    fn render_card(card: SoftCard, width: u16, height: u16) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        card.render(area, &mut buf);
        buf
    }

    #[test]
    fn basic_rendering() {
        let card = SoftCard::new(
            ProviderHealth::Degraded,
            vec![Line::from("Hello"), Line::from("World")],
        );
        assert_eq!(card.height(20), 3); // 2 content + 1 separator
        let card = SoftCard::new(
            ProviderHealth::Degraded,
            vec![Line::from("Hello"), Line::from("World")],
        );
        let buf = render_card(card, 20, 3);
        // Accent stripe on rows 0 and 1
        assert_eq!(buf[(0u16, 0u16)].symbol(), "\u{258e}");
        assert_eq!(buf[(0u16, 1u16)].symbol(), "\u{258e}");
        // Content starts at column 2
        assert_eq!(buf[(2u16, 0u16)].symbol(), "H");
        assert_eq!(buf[(2u16, 1u16)].symbol(), "W");
        // Separator on last row
        assert_eq!(buf[(0u16, 2u16)].symbol(), "\u{2500}");
        assert_eq!(buf[(5u16, 2u16)].symbol(), "\u{2500}");
    }

    #[test]
    fn wrapped_lines_get_accent_stripes() {
        // "abcdefghij" is 10 chars wide; at width=7, content_width=5 → wraps to 2 visual lines
        let card = SoftCard::new(ProviderHealth::Outage, vec![Line::from("abcdefghij")]);
        assert_eq!(card.height(7), 3); // 2 wrapped + 1 separator

        let card = SoftCard::new(ProviderHealth::Outage, vec![Line::from("abcdefghij")]);
        let buf = render_card(card, 7, 3);
        // Accent stripe on both content rows
        assert_eq!(buf[(0u16, 0u16)].symbol(), "\u{258e}");
        assert_eq!(buf[(0u16, 1u16)].symbol(), "\u{258e}");
        // Both should have Red color (Outage)
        assert_eq!(buf[(0u16, 0u16)].fg, Color::Red);
        assert_eq!(buf[(0u16, 1u16)].fg, Color::Red);
    }

    #[test]
    fn separator_appears_on_last_row() {
        let card = SoftCard::new(ProviderHealth::Operational, vec![Line::from("Test")]);
        let buf = render_card(card, 15, 2);
        // Last row (row 1) is all separators
        for x in 0..15 {
            assert_eq!(buf[(x, 1u16)].symbol(), "\u{2500}");
            assert_eq!(buf[(x, 1u16)].fg, Color::DarkGray);
        }
    }

    #[test]
    fn height_calculation_matches_actual_render() {
        let lines = vec![
            Line::from("short"),
            Line::from("a]bcdefghijklmnopqrst"), // 21 chars
            Line::from(""),
        ];
        let width: u16 = 12; // content_width = 10
        let card = SoftCard::new(ProviderHealth::Degraded, lines.clone());
        let expected_height = card.height(width);
        // "short" = 1 row, 21-char line = ceil(21/10)=3 rows, "" = 1 row => 5 content + 1 sep = 6
        assert_eq!(expected_height, 6);
    }

    #[test]
    fn empty_card() {
        let card = SoftCard::new(ProviderHealth::Operational, vec![]);
        assert_eq!(card.height(20), 1); // 0 content + 1 separator

        let card = SoftCard::new(ProviderHealth::Operational, vec![]);
        let buf = render_card(card, 20, 1);
        // Only separator row
        assert_eq!(buf[(0u16, 0u16)].symbol(), "\u{2500}");
    }

    #[test]
    fn multi_span_styling_preserved() {
        let line = Line::from(vec![
            Span::styled("Bold", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" normal"),
        ]);
        let card = SoftCard::new(ProviderHealth::Degraded, vec![line]);
        let buf = render_card(card, 20, 2);
        // "B" at col 2 should be bold
        assert!(buf[(2u16, 0u16)].modifier.contains(Modifier::BOLD));
        // "n" at col 7 should not be bold
        assert!(!buf[(7u16, 0u16)].modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn height_accounts_for_word_boundary_wrapping() {
        // "aaaa bbbb cccc" at content_width=5 (available_width=7, minus 2 for accent):
        // word wrapper produces 3 lines, not div_ceil(14,5)=3 — same here, but
        // at content_width=7 (available_width=9): div_ceil(14,7)=2 but word wrapper gives 3
        let card = SoftCard::new(ProviderHealth::Degraded, vec![Line::from("aaaa bbbb cccc")]);
        // available_width=9 → content_width=7
        // Word wrapping: "aaaa" (4) + " bbbb" would be 9 > 7, wrap → "bbbb" (4) + " cccc" = 9 > 7, wrap
        // = 3 visual lines + 1 separator = 4
        assert_eq!(card.height(9), 4);
    }

    #[test]
    fn accent_stripe_uses_health_color() {
        let card = SoftCard::new(ProviderHealth::Maintenance, vec![Line::from("test")]);
        let buf = render_card(card, 20, 2);
        assert_eq!(buf[(0u16, 0u16)].fg, Color::Blue); // Maintenance = Blue
    }
}
