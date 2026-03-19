use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::tui::benchmarks::render::compare_colors;

/// A single entry in a comparison legend.
pub struct LegendEntry {
    /// Display name of the model/item.
    pub name: String,
    /// Index into the compare color palette.
    pub color_index: usize,
    /// Metric columns: (label, formatted value) pairs.
    pub metrics: Vec<(String, String)>,
}

/// A legend table showing colored markers and metric values for compared items.
///
/// Used by scatter and radar charts to display a consistent legend box.
pub struct ComparisonLegend {
    entries: Vec<LegendEntry>,
    title: String,
}

impl ComparisonLegend {
    pub fn new(entries: Vec<LegendEntry>) -> Self {
        Self {
            entries,
            title: " Legend ".to_string(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Render the legend table into the given area.
    pub fn render(self, f: &mut Frame, area: Rect) {
        if self.entries.is_empty() {
            return;
        }

        let rows: Vec<Row> = self
            .entries
            .iter()
            .map(|entry| {
                let color = compare_colors(entry.color_index);
                let mut cells = vec![
                    Cell::from(Span::styled("\u{25cf} ", Style::default().fg(color))),
                    Cell::from(Span::styled(entry.name.clone(), Style::default().fg(color))),
                ];
                for (label, value) in &entry.metrics {
                    cells.push(Cell::from(Span::styled(
                        format!("{}: ", label),
                        Style::default().fg(Color::DarkGray),
                    )));
                    cells.push(Cell::from(Span::styled(
                        value.clone(),
                        Style::default().fg(Color::White),
                    )));
                }
                Row::new(cells)
            })
            .collect();

        // Widths: marker(2) + name(fill) + (label + value) per metric
        let mut widths: Vec<Constraint> = vec![Constraint::Length(2), Constraint::Fill(1)];
        if let Some(first) = self.entries.first() {
            for (label, _) in &first.metrics {
                widths.push(Constraint::Length((label.len() + 2) as u16));
                widths.push(Constraint::Length(8));
            }
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(self.title);
        let table = Table::new(rows, widths).block(block);
        f.render_widget(table, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legend_entry_creation() {
        let entry = LegendEntry {
            name: "GPT-4".into(),
            color_index: 0,
            metrics: vec![
                ("Quality".into(), "92.1".into()),
                ("Speed".into(), "45.2".into()),
            ],
        };
        assert_eq!(entry.name, "GPT-4");
        assert_eq!(entry.metrics.len(), 2);
    }

    #[test]
    fn legend_with_title() {
        let legend = ComparisonLegend::new(vec![]).title(" Custom ");
        assert_eq!(legend.title, " Custom ");
    }
}
