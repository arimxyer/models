use ratatui::{
    buffer::Buffer,
    layout::Margin,
    text::Line,
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Widget, Wrap,
    },
    Frame,
};

use crate::tui::ui::focus_border;
use crate::tui::widgets::scroll_offset::ScrollOffset;
use crate::tui::widgets::soft_card::SoftCard;

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
/// Accepts either raw lines or SoftCards. When SoftCards are provided,
/// each card renders itself (accent stripes, content, separator).
pub struct ScrollablePanel<'a> {
    lines: Option<Vec<Line<'a>>>,
    cards: Option<Vec<SoftCard>>,
    title: String,
    scroll: &'a ScrollOffset,
    focused: bool,
}

impl<'a> ScrollablePanel<'a> {
    pub fn new(
        title: impl Into<String>,
        lines: Vec<Line<'a>>,
        scroll: &'a ScrollOffset,
        focused: bool,
    ) -> Self {
        Self {
            lines: Some(lines),
            cards: None,
            title: title.into(),
            scroll,
            focused,
        }
    }

    /// Create a panel that renders SoftCards instead of raw lines.
    pub fn with_cards(
        title: impl Into<String>,
        cards: Vec<SoftCard>,
        scroll: &'a ScrollOffset,
        focused: bool,
    ) -> Self {
        Self {
            lines: None,
            cards: Some(cards),
            title: title.into(),
            scroll,
            focused,
        }
    }

    /// Render the panel into the given area and return computed state.
    pub fn render(self, f: &mut Frame, area: ratatui::layout::Rect) -> ScrollablePanelState {
        let title = self.title;
        let scroll = self.scroll;
        let focused = self.focused;
        if let Some(cards) = self.cards {
            Self::render_cards_inner(f, area, cards, &title, scroll, focused)
        } else {
            Self::render_lines_inner(
                f,
                area,
                self.lines.unwrap_or_default(),
                &title,
                scroll,
                focused,
            )
        }
    }

    fn render_cards_inner(
        f: &mut Frame,
        area: ratatui::layout::Rect,
        cards: Vec<SoftCard>,
        title: &str,
        scroll: &ScrollOffset,
        focused: bool,
    ) -> ScrollablePanelState {
        let border_style = focus_border(focused);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", title));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let visible_height = inner.height;
        let inner_width = inner.width;

        // Compute card heights and total
        let card_heights: Vec<u16> = cards.iter().map(|c| c.height(inner_width)).collect();
        let visual_total: u16 = card_heights.iter().copied().sum();

        let max_scroll = visual_total.saturating_sub(visible_height);
        let clamped_scroll = scroll.get().min(max_scroll);
        scroll.set(clamped_scroll);

        // Build visual offsets (cumulative heights)
        let mut visual_offsets = Vec::with_capacity(cards.len());
        let mut cumulative: u16 = 0;
        for &h in &card_heights {
            visual_offsets.push(cumulative);
            cumulative += h;
        }

        // Render visible cards
        let mut y_offset: u16 = 0;
        for (i, card) in cards.into_iter().enumerate() {
            let card_h = card_heights[i];
            let card_top = y_offset;
            let card_bottom = y_offset + card_h;

            // Skip cards entirely above viewport
            if card_bottom <= clamped_scroll {
                y_offset += card_h;
                continue;
            }
            // Stop if card starts below viewport
            if card_top >= clamped_scroll + visible_height {
                break;
            }

            // Calculate screen position and clipping
            let screen_y = if card_top >= clamped_scroll {
                inner.y + (card_top - clamped_scroll)
            } else {
                inner.y
            };

            let clip_top = clamped_scroll.saturating_sub(card_top);
            let available_below = (inner.y + visible_height).saturating_sub(screen_y);
            let render_h = card_h.saturating_sub(clip_top).min(available_below);

            if render_h > 0 {
                // Create a temporary buffer for the full card, then copy visible portion
                let card_area = ratatui::layout::Rect::new(0, 0, inner_width, card_h);
                let mut card_buf = Buffer::empty(card_area);
                card.render(card_area, &mut card_buf);

                // Copy visible rows from card buffer to frame buffer
                let buf = f.buffer_mut();
                for row in 0..render_h {
                    let src_row = clip_top + row;
                    let dst_y = screen_y + row;
                    for col in 0..inner_width {
                        let src_cell = &card_buf[(col, src_row)];
                        let dst_cell = &mut buf[(inner.x + col, dst_y)];
                        *dst_cell = src_cell.clone();
                    }
                }
            }

            y_offset += card_h;
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

    fn render_lines_inner(
        f: &mut Frame,
        area: ratatui::layout::Rect,
        lines: Vec<Line<'a>>,
        title: &str,
        scroll: &ScrollOffset,
        focused: bool,
    ) -> ScrollablePanelState {
        let border_style = focus_border(focused);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", title));

        let visible_height = area.height.saturating_sub(2);
        let wrap_width = area.width.saturating_sub(2) as usize;
        let (visual_total, visual_offsets) = wrapped_line_offsets(&lines, wrap_width);
        let max_scroll = visual_total.saturating_sub(visible_height);
        let clamped_scroll = scroll.get().min(max_scroll);
        scroll.set(clamped_scroll);

        let paragraph = Paragraph::new(lines)
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
    let width = wrap_width as u16;
    for line in lines {
        offsets.push(cumulative);
        let wrapped_lines = if wrap_width == 0 {
            1
        } else {
            let p = Paragraph::new(vec![line.clone()]).wrap(Wrap { trim: false });
            (p.line_count(width) as u16).max(1)
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

    #[test]
    fn wrapped_line_offsets_word_boundary() {
        // "aaaa bbbb cccc" at width 7: word wrapper produces 3 lines
        // (div_ceil would predict 2, but words can't share a line)
        let lines = vec![Line::from("aaaa bbbb cccc")];
        let (total, offsets) = wrapped_line_offsets(&lines, 7);
        assert_eq!(total, 3);
        assert_eq!(offsets, vec![0]);

        // Two lines with word wrapping
        let lines = vec![Line::from("aaaa bbbb cccc"), Line::from("short")];
        let (total, offsets) = wrapped_line_offsets(&lines, 7);
        assert_eq!(total, 4); // 3 + 1
        assert_eq!(offsets, vec![0, 3]);
    }
}
