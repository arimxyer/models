---
description: Benchmarks tab design conventions — browse/compare modes, creator sidebar, H2H table, scatter plot, radar chart, sort picker
globs:
  - src/tui/benchmarks/**
---

# Benchmarks Tab Design Conventions

Tab-specific patterns only. For shared colors, borders, focus, search, footer, and scrollbars see `tui-style-guide.md`.

---

## 1. Browse Mode Layout

```
Percentage(20)  -- Creators sidebar
Percentage(40)  -- Benchmark list
Percentage(40)  -- Detail panel (ScrollablePanel)
```

---

## 2. Compare Mode Layout

```
Length(max(area_width * 30 / 100, 35))  -- Compact list (or creators if toggled)
Min(0)                                   -- Comparison panel
```

Comparison panel internal split:

```rust
Constraint::Length(1)  -- Subtab bar ([H2H] [Scatter] [Radar])
Constraint::Min(0)     -- Active view
```

---

## 3. Compare Palette

8 colors, indexed modulo 8. Used for selection markers, H2H value columns, scatter points, radar polygons, and legend entries.

```rust
[Red, Green, Blue, Yellow, Magenta, Cyan, LightRed, LightGreen]
```

Selection marker in list: `●` (U+25CF) in the model's compare color.

---

## 4. Subtab Bar

Format: ` [H2H]  [Scatter]  [Radar] ` (each label space-padded inside brackets):

- Active view: `Color::Cyan` + `Modifier::BOLD`
- Inactive views: `Color::DarkGray`
- Rendered as a `Paragraph` into the 1-line `Constraint::Length(1)` area
- `v` key cycles through views

---

## 5. Creator Sidebar

**"All" item**: `"All"` in `Color::Green` + `" ({count})"` in default

**Group header items** (when grouping active): same `── Label ──────` format as Models tab, colored by group classification + `Modifier::BOLD`

**Creator items** (ungrouped): `"{name} ({count})"` — name truncated to available width, count in `Color::Gray`. When grouping active, a short tag (label + color) is appended after the count.

**Region grouping colors**:

| Region | Color |
|--------|-------|
| US | `Color::Blue` |
| China | `Color::Red` |
| Europe | `Color::Magenta` |
| Middle East | `Color::Yellow` |
| South Korea | `Color::Cyan` |
| Canada | `Color::Green` |
| Other | `Color::DarkGray` |

Region grouping key `[5]` color: `Color::Yellow` when active, `Color::DarkGray` when not.

**Type grouping colors**:

| Type | Color |
|------|-------|
| Startup | `Color::Green` |
| Giant | `Color::Blue` |
| Research | `Color::Magenta` |

Type grouping key `[6]` color: `Color::Magenta` when active, `Color::DarkGray` when not.

**Filter row**:

```
[5] Rgn  [6] Type       (ungrouped)
[5] Region  [6] Type    (region grouping active — label expands)
```

**Reasoning/Source indicators** in compact list rows:

| Indicator | Chars | Color |
|-----------|-------|-------|
| Reasoning | `"R "` | `Color::Cyan` |
| Adaptive Reasoning | `"AR"` | `Color::Yellow` |
| Non-reasoning | `"  "` or `"NR"` | `Color::DarkGray` |
| Open source | `"O"` | `Color::Green` |
| Closed source | `"C"` | `Color::Red` |

Width: 3 chars for reasoning indicator, 2 for source indicator.

---

## 6. Sort Picker Popup

- **Size**: `centered_rect_fixed(30, sort_options_count + 2)` — 30 chars wide, fixed height
- **Border**: `Color::Cyan`
- **Title**: `" Sort By "`
- Current sort highlighted with `▼` (descending) or `▲` (ascending) prefix in `Color::Cyan` + `Modifier::BOLD`
- Other options: default style with `Color::DarkGray` prefix space
- `s` opens picker, `Enter` confirms, `Esc` cancels

---

## 7. H2H Table

Rendered inside `ScrollablePanel` with `.with_wrap(false)` (pre-formatted, no wrapping).

**Label column width**: 14 chars, left-aligned.

**Section header rows**:

```
─── Section ────────────────
```

Style: `Color::DarkGray`. Fills to panel width with `\u{2500}`.

**Value formats by metric type**:

| Type | Format |
|------|--------|
| Index (0–100) | `{:.1}` |
| Percentage benchmark | `{:.1}%` (value × 100) |
| Speed (tok/s) | `{:.0}` |
| Latency (ms) | `{:.0}ms` |
| Pricing ($/M) | `${:.2}` |

**Winner highlighting**: best value per row shown in compare color + `Modifier::BOLD`. Non-best values shown in compare color without bold.

**Wins row**: prefix `"★ "` (Yellow + BOLD), count per model in compare color + BOLD.

**Missing values**: em-dash `\u{2014}` in `Color::DarkGray`.

---

## 8. Scatter Plot

- Background points: `Marker::Dot` style, `Color::DarkGray`
- Selected model points: `Marker::HalfBlock` style, compare palette colors
- Average crosshair lines (horizontal + vertical): `Color::DarkGray`
- Auto log-scale applied to an axis when value range ratio > 2.5

---

## 9. Radar Chart

- Spoke lines from center: `Color::DarkGray`
- Average polygon: `Color::Indexed(242)` (medium gray)
- Model polygons: compare palette colors
- Axis labels: offset ~56 units from center, wrapped at 16 chars
- Legend uses `ComparisonLegend` widget (see below)

**RadarPreset axis groups**:

| Preset | Axes |
|--------|------|
| Agentic | 6 axes (coding/agent benchmarks) |
| Academic | 6 axes (reasoning/knowledge benchmarks) |
| Indexes | 3 axes (Intelligence/Coding/Math indexes) |

Preset cycles with `r` key.

---

## 10. ComparisonLegend Widget

Used in scatter and radar views. Reusable widget from `src/tui/widgets/comparison_legend.rs`.

- **Average row**: name `"Avg"`, color `Color::Indexed(250)` (light gray), marker `┅` (U+2505)
- **Model rows**: name truncated to fit, compare palette color, marker `●` (U+25CF)
- Value width: 6 chars per column

---

## 11. Detail Overlay (Compare Mode)

Full model detail shown as an overlay when `d` is pressed in compare mode:

- **Size**: `centered_rect(60, 75)` — 60% width, 75% height
- **Border**: `Color::Cyan`
- **Title**: `" Model Detail (Esc to close) "`
- Background: `Clear` widget rendered first
- Must intercept global keys (`q`, `?`, etc.) to prevent pass-through

---

## 12. Detail Panel (Browse Mode)

4-column layout for label-value pairs. Column percentages: `[28%, 22%, 28%, 22%]`.

- Labels: `Color::Gray`
- Values: colored by type (index scores = White, percentages = White, speeds = White, prices = varies)
- Section headers: `"─── Title ───"` in `Color::DarkGray` (same `\u{2500}` fill pattern as H2H sections)
- Panel uses `ScrollablePanel` with `detail_scroll: ScrollOffset`

Focus cycles through Creators → List → Details via `h`/`l`. `reset_detail_scroll()` called on every selection, filter, and sort change.

---

## 13. Loading State

- List title appends `" loading..."` (literal string, no icon) when `bench_app.loading` is true
- Detail panel shows `"Loading..."` in `Color::Yellow` when benchmark store is empty
