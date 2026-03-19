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
    /// Cumulative visual line offset for each logical line.
    /// `visual_offsets[i]` is the visual row where logical line `i` starts.
    pub visual_offsets: Vec<u16>,
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
        let (visual_total, visual_offsets) = wrapped_line_offsets(&self.lines, wrap_width);
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
            visual_offsets,
        }
    }
}

/// Compute cumulative visual line offsets and total visual line count.
///
/// Returns `(total_visual_lines, offsets)` where `offsets[i]` is the visual row
/// at which logical line `i` starts (accounting for word-wrapping).
fn wrapped_line_offsets(lines: &[Line<'_>], wrap_width: usize) -> (u16, Vec<u16>) {
    let mut offsets = Vec::with_capacity(lines.len());
    let mut cumulative: u16 = 0;
    for line in lines {
        offsets.push(cumulative);
        let line_width = line.width();
        let wrapped_lines = if wrap_width == 0 || line_width == 0 {
            1
        } else {
            line_width.div_ceil(wrap_width).max(1) as u16
        };
        cumulative += wrapped_lines;
    }
    (cumulative, offsets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_line_offsets_basics() {
        let lines = vec![
            Line::from("short"),
            Line::from(""),
            Line::from("0123456789"), // 10 chars
        ];
        // width 20: all fit in 1 row each = 3 total, offsets [0, 1, 2]
        let (total, offsets) = wrapped_line_offsets(&lines, 20);
        assert_eq!(total, 3);
        assert_eq!(offsets, vec![0, 1, 2]);

        // width 5: "short"=1, ""=1, "0123456789"=2 → total 4, offsets [0, 1, 2]
        let (total, offsets) = wrapped_line_offsets(&lines, 5);
        assert_eq!(total, 4);
        assert_eq!(offsets, vec![0, 1, 2]);

        // width 0: each line = 1 row = 3 total
        let (total, offsets) = wrapped_line_offsets(&lines, 0);
        assert_eq!(total, 3);
        assert_eq!(offsets, vec![0, 1, 2]);
    }
}
