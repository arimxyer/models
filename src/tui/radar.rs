use std::collections::HashMap;
use std::f64::consts::PI;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Block, Borders, Cell, Row, Table,
    },
    Frame,
};

use super::benchmarks_app::RadarPreset;
use crate::benchmarks::BenchmarkEntry;

/// Compute N spoke angles starting at top (-PI/2), going clockwise.
pub fn spoke_angles(n: usize) -> Vec<f64> {
    let step = 2.0 * PI / n as f64;
    (0..n).map(|i| -PI / 2.0 + step * i as f64).collect()
}

/// Given center, radius, spoke angles, and normalized values (0-1), compute vertex positions.
pub fn polygon_vertices(
    cx: f64,
    cy: f64,
    radius: f64,
    angles: &[f64],
    values: &[f64],
) -> Vec<(f64, f64)> {
    angles
        .iter()
        .zip(values.iter())
        .map(|(&angle, &val)| {
            let r = radius * val;
            (cx + r * angle.cos(), cy + r * angle.sin())
        })
        .collect()
}

pub struct RadarAxis {
    pub label: &'static str,
    pub short: &'static str,
    pub key: &'static str,
    pub extract: fn(&BenchmarkEntry) -> Option<f64>,
}

pub fn axes_for_preset(preset: RadarPreset) -> Vec<RadarAxis> {
    match preset {
        RadarPreset::Agentic => vec![
            RadarAxis {
                label: "Coding",
                short: "Cod",
                key: "coding_index",
                extract: |e| e.coding_index,
            },
            RadarAxis {
                label: "LiveCodeBench",
                short: "LC",
                key: "livecodebench",
                extract: |e| e.livecodebench,
            },
            RadarAxis {
                label: "SciCode",
                short: "SC",
                key: "scicode",
                extract: |e| e.scicode,
            },
            RadarAxis {
                label: "TerminalBench",
                short: "TB",
                key: "terminalbench_hard",
                extract: |e| e.terminalbench_hard,
            },
            RadarAxis {
                label: "IFBench",
                short: "IF",
                key: "ifbench",
                extract: |e| e.ifbench,
            },
            RadarAxis {
                label: "Long Context Reasoning",
                short: "LCR",
                key: "lcr",
                extract: |e| e.lcr,
            },
        ],
        RadarPreset::Academic => vec![
            RadarAxis {
                label: "GPQA Diamond",
                short: "GQ",
                key: "gpqa",
                extract: |e| e.gpqa,
            },
            RadarAxis {
                label: "MMLU-Pro",
                short: "MM",
                key: "mmlu_pro",
                extract: |e| e.mmlu_pro,
            },
            RadarAxis {
                label: "Humanity's Last Exam",
                short: "HLE",
                key: "hle",
                extract: |e| e.hle,
            },
            RadarAxis {
                label: "MATH-500",
                short: "M5",
                key: "math_500",
                extract: |e| e.math_500,
            },
            RadarAxis {
                label: "AIME '24",
                short: "AI",
                key: "aime",
                extract: |e| e.aime,
            },
            RadarAxis {
                label: "AIME '25",
                short: "A25",
                key: "aime_25",
                extract: |e| e.aime_25,
            },
        ],
        RadarPreset::Indexes => vec![
            RadarAxis {
                label: "Intel",
                short: "Int",
                key: "intelligence_index",
                extract: |e| e.intelligence_index,
            },
            RadarAxis {
                label: "Coding",
                short: "Cod",
                key: "coding_index",
                extract: |e| e.coding_index,
            },
            RadarAxis {
                label: "Math",
                short: "Mth",
                key: "math_index",
                extract: |e| e.math_index,
            },
        ],
    }
}

/// Draw the radar chart in the given area.
pub fn draw_radar(f: &mut Frame, area: Rect, app: &super::app::App) {
    let axes = axes_for_preset(app.benchmarks_app.radar_preset);
    let preset_label = app.benchmarks_app.radar_preset.label();

    // Empty guard: need at least 3 axes and at least one selection
    if axes.len() < 3 || app.selections.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Radar [{preset_label}] "));
        f.render_widget(block, area);
        return;
    }

    let angles = spoke_angles(axes.len());
    let radius: f64 = 45.0;

    // Pre-compute max values for normalization (MUST be outside paint() closure)
    let entries = app.benchmark_store.entries();
    let mut max_values: HashMap<&str, f64> = HashMap::new();
    for entry in entries.iter() {
        for ax in &axes {
            if let Some(v) = (ax.extract)(entry) {
                let current = max_values.entry(ax.key).or_insert(0.0);
                if v > *current {
                    *current = v;
                }
            }
        }
    }

    // Pre-compute axis line endpoints and labels
    let axis_lines: Vec<(f64, f64)> = angles
        .iter()
        .map(|&a| (radius * a.cos(), radius * a.sin()))
        .collect();

    let label_offset = 56.0;
    // Each axis label can be multiple lines (for wrapping long names)
    let axis_labels: Vec<Vec<(f64, f64, String)>> = angles
        .iter()
        .zip(axes.iter())
        .map(|(&a, ax)| {
            let lx = label_offset * a.cos();
            let ly = label_offset * a.sin();
            let full = if ax.short == ax.label {
                ax.label.to_string()
            } else {
                format!("{} ({})", ax.label, ax.short)
            };
            // Wrap labels longer than 16 chars at the last space before the limit
            if full.len() <= 16 {
                vec![(lx, ly, full)]
            } else if let Some(split) = full[..16].rfind(' ') {
                let line1 = full[..split].to_string();
                let line2 = full[split + 1..].to_string();
                // Offset second line down by ~4 canvas units
                vec![(lx, ly, line1), (lx, ly - 4.0, line2)]
            } else {
                vec![(lx, ly, full)]
            }
        })
        .collect();

    // Pre-compute polygon data and legend entries for each selected model
    let mut polygons: Vec<(Vec<(f64, f64)>, Color)> = Vec::new();
    let mut legend_entries: Vec<(String, Color, Vec<Option<f64>>)> = Vec::new();

    for (sel_idx, &store_idx) in app.selections.iter().enumerate() {
        if let Some(entry) = entries.get(store_idx) {
            let color = super::ui::compare_colors(sel_idx);

            let raw_values: Vec<Option<f64>> = axes.iter().map(|ax| (ax.extract)(entry)).collect();

            // Normalize values for this model
            let values: Vec<f64> = axes
                .iter()
                .map(|ax| {
                    let raw = (ax.extract)(entry).unwrap_or(0.0);
                    let max = max_values.get(ax.key).copied().unwrap_or(1.0);
                    if max > 0.0 {
                        raw / max
                    } else {
                        0.0
                    }
                })
                .collect();

            let vertices = polygon_vertices(0.0, 0.0, radius, &angles, &values);
            polygons.push((vertices, color));
            legend_entries.push((entry.display_name.clone(), color, raw_values));
        }
    }

    // Compute average polygon (baseline reference) — always uses all entries
    let avg_values: Vec<f64> = axes
        .iter()
        .map(|ax| {
            let (sum, count) = entries.iter().fold((0.0, 0usize), |(s, c), entry| {
                if let Some(v) = (ax.extract)(entry) {
                    (s + v, c + 1)
                } else {
                    (s, c)
                }
            });
            if count > 0 {
                let raw_avg = sum / count as f64;
                let max = max_values.get(ax.key).copied().unwrap_or(1.0);
                if max > 0.0 {
                    raw_avg / max
                } else {
                    0.0
                }
            } else {
                0.0
            }
        })
        .collect();

    // Raw average values for labels
    let avg_raw_values: Vec<f64> = axes
        .iter()
        .map(|ax| {
            let (sum, count) = entries.iter().fold((0.0, 0usize), |(s, c), entry| {
                if let Some(v) = (ax.extract)(entry) {
                    (s + v, c + 1)
                } else {
                    (s, c)
                }
            });
            if count > 0 {
                sum / count as f64
            } else {
                0.0
            }
        })
        .collect();

    let avg_vertices = polygon_vertices(0.0, 0.0, radius, &angles, &avg_values);

    // avg_raw_values used in legend table below

    let compare_focused =
        app.benchmarks_app.focus == super::benchmarks_app::BenchmarkFocus::Compare;
    let radar_border = if compare_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    // Split area: canvas on top, legend box at bottom (+1 for avg row)
    let legend_height = (legend_entries.len() as u16 + 3).min(area.height / 3); // +2 borders +1 avg
    let (canvas_area, legend_area) = if !legend_entries.is_empty() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(legend_height)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(radar_border))
                .title(format!(" Radar [{preset_label}] ")),
        )
        .x_bounds([-65.0, 65.0])
        .y_bounds([-62.0, 62.0])
        .marker(ratatui::symbols::Marker::Braille)
        .paint(move |ctx| {
            // Draw axis spokes
            for &(ex, ey) in &axis_lines {
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: ex,
                    y2: ey,
                    color: Color::DarkGray,
                });
            }

            // Draw average baseline polygon
            let n_avg = avg_vertices.len();
            for i in 0..n_avg {
                let (x1, y1) = avg_vertices[i];
                let (x2, y2) = avg_vertices[(i + 1) % n_avg];
                ctx.draw(&CanvasLine {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: Color::Indexed(242),
                });
            }

            // Draw axis labels (may be multi-line for long names)
            for lines in &axis_labels {
                for (lx, ly, label) in lines {
                    ctx.print(
                        *lx,
                        *ly,
                        Span::styled(label.clone(), Style::default().fg(Color::Gray)),
                    );
                }
            }

            // Draw model polygons
            for (vertices, color) in &polygons {
                let n = vertices.len();
                for i in 0..n {
                    let (x1, y1) = vertices[i];
                    let (x2, y2) = vertices[(i + 1) % n];
                    ctx.draw(&CanvasLine {
                        x1,
                        y1,
                        x2,
                        y2,
                        color: *color,
                    });
                }
            }
        });

    f.render_widget(canvas, canvas_area);

    // Legend table below the radar chart
    if let Some(leg_area) = legend_area {
        let fmt_axis_val = |v: Option<f64>, key: &str| -> String {
            match v {
                Some(val) if key.ends_with("_index") => format!("{:.1}", val),
                Some(val) => format!("{:.1}%", val * 100.0),
                None => "\u{2014}".into(),
            }
        };

        let mut rows: Vec<Row> = legend_entries
            .iter()
            .map(|(name, color, raw_vals)| {
                let mut cells = vec![
                    Cell::from(Span::styled("\u{25cf} ", Style::default().fg(*color))),
                    Cell::from(Span::styled(name.clone(), Style::default().fg(*color))),
                ];
                for (i, ax) in axes.iter().enumerate() {
                    cells.push(Cell::from(Span::styled(
                        format!("{}: ", ax.short),
                        Style::default().fg(Color::DarkGray),
                    )));
                    cells.push(Cell::from(Span::styled(
                        fmt_axis_val(raw_vals.get(i).copied().flatten(), ax.key),
                        Style::default().fg(Color::White),
                    )));
                }
                Row::new(cells)
            })
            .collect();

        // Average row
        let avg_color = Color::Indexed(250); // light gray, distinct from model colors
        let mut avg_cells = vec![
            Cell::from(Span::styled("\u{2505} ", Style::default().fg(avg_color))),
            Cell::from(Span::styled("Avg", Style::default().fg(avg_color))),
        ];
        for (i, ax) in axes.iter().enumerate() {
            avg_cells.push(Cell::from(Span::styled(
                format!("{}: ", ax.short),
                Style::default().fg(Color::DarkGray),
            )));
            avg_cells.push(Cell::from(Span::styled(
                fmt_axis_val(Some(avg_raw_values[i]), ax.key),
                Style::default().fg(avg_color),
            )));
        }
        rows.push(Row::new(avg_cells));

        // Widths: marker(2) + name(fill) + (short_label + value) per axis
        let mut widths: Vec<Constraint> = vec![Constraint::Length(2), Constraint::Fill(1)];
        for ax in &axes {
            widths.push(Constraint::Length((ax.short.len() + 2) as u16));
            widths.push(Constraint::Length(6));
        }

        let legend_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Legend ");
        let table = Table::new(rows, widths).block(legend_block);
        f.render_widget(table, leg_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spoke_angles_start_at_top() {
        let angles = spoke_angles(6);
        assert!((angles[0] - (-PI / 2.0)).abs() < 1e-10);
    }

    #[test]
    fn spoke_angles_evenly_spaced() {
        let angles = spoke_angles(4);
        let expected_gap = 2.0 * PI / 4.0;
        for i in 0..3 {
            let gap = angles[i + 1] - angles[i];
            assert!((gap - expected_gap).abs() < 1e-10);
        }
    }

    #[test]
    fn polygon_vertex_at_max_reaches_radius() {
        let angles = spoke_angles(4);
        let values = vec![1.0, 0.5, 1.0, 0.5];
        let vertices = polygon_vertices(50.0, 50.0, 40.0, &angles, &values);
        assert!((vertices[0].0 - 50.0).abs() < 1e-10);
        assert!((vertices[0].1 - 10.0).abs() < 1e-10);
    }

    #[test]
    fn polygon_vertex_at_zero_stays_at_center() {
        let angles = spoke_angles(4);
        let values = vec![0.0, 0.0, 0.0, 0.0];
        let vertices = polygon_vertices(50.0, 50.0, 40.0, &angles, &values);
        for &(x, y) in &vertices {
            assert!((x - 50.0).abs() < 1e-10);
            assert!((y - 50.0).abs() < 1e-10);
        }
    }
}
