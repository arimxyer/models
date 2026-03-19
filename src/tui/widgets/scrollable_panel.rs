use ratatui::{
    buffer::Buffer,
    layout::Margin,
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::tui::ui::{focus_border, status_health_style};
use crate::tui::widgets::soft_card::AccentRegion;

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
///
/// When accent regions are provided, content is shifted 2 columns right
/// and health-colored `"▎"` stripes are painted in the left gutter.
pub struct ScrollablePanel<'a> {
    lines: Vec<Line<'a>>,
    title: String,
    scroll: u16,
    focused: bool,
    accent_regions: Vec<AccentRegion>,
}

impl<'a> ScrollablePanel<'a> {
    pub fn new(title: impl Into<String>, lines: Vec<Line<'a>>, scroll: u16, focused: bool) -> Self {
        Self {
            lines,
            title: title.into(),
            scroll,
            focused,
            accent_regions: Vec::new(),
        }
    }

    /// Add accent stripe regions for health-colored left-edge painting.
    pub fn with_accents(mut self, accents: Vec<AccentRegion>) -> Self {
        self.accent_regions = accents;
        self
    }

    /// Render the panel into the given area and return computed state.
    pub fn render(self, f: &mut Frame, area: ratatui::layout::Rect) -> ScrollablePanelState {
        let border_style = focus_border(self.focused);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", self.title));

        let has_accents = !self.accent_regions.is_empty();
        let accent_indent: u16 = if has_accents { 2 } else { 0 };

        let visible_height = area.height.saturating_sub(2);
        // Use narrower width for wrap calculation when accents shift content right
        let wrap_width = area.width.saturating_sub(2).saturating_sub(accent_indent) as usize;
        let (visual_total, visual_offsets) = wrapped_line_offsets(&self.lines, wrap_width);
        let max_scroll = visual_total.saturating_sub(visible_height);
        let clamped_scroll = self.scroll.min(max_scroll);

        if has_accents {
            // Render block manually, then Paragraph into shifted content area
            let inner = block.inner(area);
            f.render_widget(block, area);

            let content_area = ratatui::layout::Rect {
                x: inner.x + accent_indent,
                width: inner.width.saturating_sub(accent_indent),
                ..inner
            };
            let paragraph = Paragraph::new(self.lines)
                .wrap(Wrap { trim: false })
                .scroll((clamped_scroll, 0));
            f.render_widget(paragraph, content_area);

            // Paint accent stripes post-render
            paint_accent_stripes(
                f.buffer_mut(),
                inner,
                &visual_offsets,
                clamped_scroll,
                visible_height,
                &self.accent_regions,
            );
        } else {
            let paragraph = Paragraph::new(self.lines)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((clamped_scroll, 0));
            f.render_widget(paragraph, area);
        }

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

/// Paint health-colored `"▎"` accent stripes in the left gutter for visible rows.
fn paint_accent_stripes(
    buf: &mut Buffer,
    inner: ratatui::layout::Rect,
    visual_offsets: &[u16],
    scroll: u16,
    visible_height: u16,
    regions: &[AccentRegion],
) {
    for region in regions {
        let accent_style = status_health_style(region.health);
        for logical_idx in region.start_line..region.end_line {
            if logical_idx >= visual_offsets.len() {
                break;
            }
            let vis_start = visual_offsets[logical_idx];
            let vis_end = if logical_idx + 1 < visual_offsets.len() {
                visual_offsets[logical_idx + 1]
            } else {
                vis_start + 1
            };
            for vis_row in vis_start..vis_end {
                if vis_row >= scroll && vis_row < scroll + visible_height {
                    let screen_row = inner.y + (vis_row - scroll);
                    buf.set_string(inner.x, screen_row, "\u{258e}", accent_style);
                }
            }
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
