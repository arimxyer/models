use ratatui::{
    layout::Margin,
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::tui::ui::focus_border;

/// Computed metadata returned after rendering a `ScrollablePanel`.
#[allow(dead_code)]
pub struct ScrollablePanelState {
    /// Scroll position after clamping to content bounds.
    pub clamped_scroll: u16,
    /// Total visual line count (accounting for wrapping).
    pub visual_line_count: u16,
    /// Number of visible lines in the viewport.
    pub visible_height: u16,
}

/// A bordered panel with wrapped text, scroll, and scrollbar.
///
/// Encapsulates the repeated pattern of:
/// Block + Paragraph with Wrap + scroll clamping + Scrollbar.
pub struct ScrollablePanel<'a> {
    lines: Vec<Line<'a>>,
    title: String,
    scroll: u16,
    focused: bool,
}

impl<'a> ScrollablePanel<'a> {
    pub fn new(title: impl Into<String>, lines: Vec<Line<'a>>, scroll: u16, focused: bool) -> Self {
        Self {
            lines,
            title: title.into(),
            scroll,
            focused,
        }
    }

    /// Render the panel into the given area and return computed state.
    pub fn render(self, f: &mut Frame, area: ratatui::layout::Rect) -> ScrollablePanelState {
        let border_style = focus_border(self.focused);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", self.title));

        let visible_height = area.height.saturating_sub(2);
        let wrap_width = area.width.saturating_sub(2) as usize;
        let visual_total = wrapped_visual_line_count(&self.lines, wrap_width);
        let max_scroll = visual_total.saturating_sub(visible_height);
        let clamped_scroll = self.scroll.min(max_scroll);

        let paragraph = Paragraph::new(self.lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((clamped_scroll, 0));
        f.render_widget(paragraph, area);

        // Scrollbar
        if (visual_total as usize) > (visible_height as usize) {
            let scroll_area = area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            });
            let mut state = ScrollbarState::new(visual_total as usize)
                .position(clamped_scroll as usize)
                .viewport_content_length(visible_height as usize);
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                scroll_area,
                &mut state,
            );
        }

        ScrollablePanelState {
            clamped_scroll,
            visual_line_count: visual_total,
            visible_height,
        }
    }
}

/// Count total visual (wrapped) lines for a set of lines at a given wrap width.
fn wrapped_visual_line_count(lines: &[Line<'_>], wrap_width: usize) -> u16 {
    lines
        .iter()
        .map(|line| {
            let line_width = line.width();
            if wrap_width == 0 || line_width == 0 {
                1
            } else {
                line_width.div_ceil(wrap_width).max(1) as u16
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_visual_line_count_basics() {
        let lines = vec![
            Line::from("short"),
            Line::from(""),
            Line::from("0123456789"), // 10 chars
        ];
        // width 20: all fit in 1 row each = 3
        assert_eq!(wrapped_visual_line_count(&lines, 20), 3);
        // width 5: "0123456789" wraps to 2 rows = 4 total
        assert_eq!(wrapped_visual_line_count(&lines, 5), 4);
        // width 0: each line = 1 row = 3
        assert_eq!(wrapped_visual_line_count(&lines, 0), 3);
    }
}
