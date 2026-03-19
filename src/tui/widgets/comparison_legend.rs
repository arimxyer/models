use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

/// A single metric column in a legend entry.
pub struct LegendMetric {
    /// Column label (e.g., "Int", "Quality").
    pub label: String,
    /// Formatted value text.
    pub value: String,
    /// Style for the value text (default: White foreground).
    pub value_style: Style,
}

impl LegendMetric {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            value_style: Style::default().fg(Color::White),
        }
    }

    pub fn value_style(mut self, style: Style) -> Self {
        self.value_style = style;
        self
    }
}

/// A single entry in a comparison legend.
pub struct LegendEntry {
    /// Display name of the model/item.
    pub name: String,
    /// Style for the name text.
    pub name_style: Style,
    /// Marker character (e.g., `●`, `○`, `┅`).
    pub marker: &'static str,
    /// Style for the marker.
    pub marker_style: Style,
    /// Metric columns with styled values.
    pub metrics: Vec<LegendMetric>,
}

impl LegendEntry {
    /// Create a new entry with a colored `●` marker and matching name color.
    pub fn new(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            name_style: Style::default().fg(color),
            marker: "\u{25cf} ",
            marker_style: Style::default().fg(color),
            metrics: Vec::new(),
        }
    }

    pub fn marker(mut self, marker: &'static str) -> Self {
        self.marker = marker;
        self
    }

    pub fn metrics(mut self, metrics: Vec<LegendMetric>) -> Self {
        self.metrics = metrics;
        self
    }
}

/// A legend table showing colored markers and metric values for compared items.
///
/// Used by scatter and radar charts to display a consistent legend box.
pub struct ComparisonLegend {
    entries: Vec<LegendEntry>,
    title: String,
    value_width: u16,
}

impl ComparisonLegend {
    pub fn new(entries: Vec<LegendEntry>) -> Self {
        Self {
            entries,
            title: " Legend ".to_string(),
            value_width: 8,
        }
    }

    pub fn value_width(mut self, width: u16) -> Self {
        self.value_width = width;
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
                let mut cells = vec![
                    Cell::from(Span::styled(entry.marker, entry.marker_style)),
                    Cell::from(Span::styled(entry.name.clone(), entry.name_style)),
                ];
                for metric in &entry.metrics {
                    cells.push(Cell::from(Span::styled(
                        format!("{}: ", metric.label),
                        Style::default().fg(Color::DarkGray),
                    )));
                    cells.push(Cell::from(Span::styled(
                        metric.value.clone(),
                        metric.value_style,
                    )));
                }
                Row::new(cells)
            })
            .collect();

        // Widths: marker(2) + name(fill) + (label + value) per metric
        let mut widths: Vec<Constraint> = vec![Constraint::Length(2), Constraint::Fill(1)];
        if let Some(first) = self.entries.first() {
            for metric in &first.metrics {
                widths.push(Constraint::Length((metric.label.len() + 2) as u16));
                widths.push(Constraint::Length(self.value_width));
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
        let entry = LegendEntry::new("GPT-4", Color::Red).metrics(vec![
            LegendMetric::new("Quality", "92.1"),
            LegendMetric::new("Speed", "45.2"),
        ]);
        assert_eq!(entry.name, "GPT-4");
        assert_eq!(entry.metrics.len(), 2);
    }

    #[test]
    fn legend_entry_custom_marker() {
        let entry = LegendEntry::new("Avg", Color::Gray).marker("\u{2505} ");
        assert_eq!(entry.marker, "\u{2505} ");
    }

    #[test]
    fn legend_metric_custom_style() {
        let metric =
            LegendMetric::new("Int", "42.0").value_style(Style::default().fg(Color::Indexed(250)));
        assert_eq!(metric.value, "42.0");
    }
}
