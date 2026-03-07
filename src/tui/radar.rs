use std::collections::HashMap;
use std::f64::consts::PI;

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Block, Borders,
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
    pub key: &'static str,
    pub extract: fn(&BenchmarkEntry) -> Option<f64>,
}

pub fn axes_for_preset(preset: RadarPreset) -> Vec<RadarAxis> {
    match preset {
        RadarPreset::Agentic => vec![
            RadarAxis {
                label: "Coding",
                key: "coding_index",
                extract: |e| e.coding_index,
            },
            RadarAxis {
                label: "LiveCode",
                key: "livecodebench",
                extract: |e| e.livecodebench,
            },
            RadarAxis {
                label: "SciCode",
                key: "scicode",
                extract: |e| e.scicode,
            },
            RadarAxis {
                label: "Terminal",
                key: "terminalbench_hard",
                extract: |e| e.terminalbench_hard,
            },
            RadarAxis {
                label: "IFBench",
                key: "ifbench",
                extract: |e| e.ifbench,
            },
            RadarAxis {
                label: "LCR",
                key: "lcr",
                extract: |e| e.lcr,
            },
        ],
        RadarPreset::Academic => vec![
            RadarAxis {
                label: "GPQA",
                key: "gpqa",
                extract: |e| e.gpqa,
            },
            RadarAxis {
                label: "MMLU-Pro",
                key: "mmlu_pro",
                extract: |e| e.mmlu_pro,
            },
            RadarAxis {
                label: "HLE",
                key: "hle",
                extract: |e| e.hle,
            },
            RadarAxis {
                label: "MATH-500",
                key: "math_500",
                extract: |e| e.math_500,
            },
            RadarAxis {
                label: "AIME",
                key: "aime",
                extract: |e| e.aime,
            },
            RadarAxis {
                label: "AIME'25",
                key: "aime_25",
                extract: |e| e.aime_25,
            },
        ],
        RadarPreset::Indexes => vec![
            RadarAxis {
                label: "Intel",
                key: "intelligence_index",
                extract: |e| e.intelligence_index,
            },
            RadarAxis {
                label: "Coding",
                key: "coding_index",
                extract: |e| e.coding_index,
            },
            RadarAxis {
                label: "Math",
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

    let label_offset = 52.0;
    let axis_labels: Vec<(f64, f64, &str)> = angles
        .iter()
        .zip(axes.iter())
        .map(|(&a, ax)| {
            let lx = label_offset * a.cos();
            let ly = label_offset * a.sin();
            (lx, ly, ax.label)
        })
        .collect();

    // Pre-compute polygon data for each selected model
    let mut polygons: Vec<(Vec<(f64, f64)>, Color)> = Vec::new();

    for (sel_idx, &store_idx) in app.selections.iter().enumerate() {
        if let Some(entry) = entries.get(store_idx) {
            let color = super::ui::compare_colors(sel_idx);

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

    // Pre-compute average value labels positioned near each spoke
    let avg_label_offset = 42.0;
    let avg_labels: Vec<(f64, f64, String)> = angles
        .iter()
        .zip(avg_raw_values.iter())
        .zip(axes.iter())
        .map(|((&a, &raw_val), ax)| {
            let lx = avg_label_offset * a.cos();
            let ly = avg_label_offset * a.sin() - 4.0;
            let label = if ax.key.ends_with("_index") {
                format!("{:.1}", raw_val)
            } else {
                format!("{:.1}%", raw_val * 100.0)
            };
            (lx, ly, label)
        })
        .collect();

    let compare_focused =
        app.benchmarks_app.focus == super::benchmarks_app::BenchmarkFocus::Compare;
    let radar_border = if compare_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(radar_border))
                .title(format!(" Radar [{preset_label}] ")),
        )
        .x_bounds([-60.0, 60.0])
        .y_bounds([-60.0, 60.0])
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

            // Draw axis labels
            for &(lx, ly, label) in &axis_labels {
                ctx.print(
                    lx,
                    ly,
                    Span::styled(label, Style::default().fg(Color::Gray)),
                );
            }

            // Draw average value labels
            for (lx, ly, label) in &avg_labels {
                ctx.print(
                    *lx,
                    *ly,
                    Span::styled(label.clone(), Style::default().fg(Color::Indexed(242))),
                );
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

    f.render_widget(canvas, area);
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
