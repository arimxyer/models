# Benchmarks Tab

## Files
- `app.rs` — `BenchmarksApp` state, `BenchmarkFocus` (Creators/List/Details/Compare), `BottomView` (Detail/H2H/Scatter/Radar), `ScatterAxis`, `RadarPreset`, `detail_scroll: ScrollOffset`, sort/filter types
- `render.rs` — `draw_benchmarks_main()`, `compare_colors()` (8-color palette for multi-select)
- `compare.rs` — `draw_h2h_table_generic()`, `draw_scatter()` (comparison visualizations)
- `radar.rs` — `draw_radar()`, spoke angle math, polygon vertex calculation, preset axis definitions

## Key Patterns
- Browse mode: Creators panel (left) + model list (center) + detail (right)
- Compare mode: selected models (left) + visualization (right, switchable via `BottomView`)
- `compare_colors()` returns 8 colors indexed modulo — used by H2H columns, scatter points, radar polygons, and legend
- `RadarPreset` defines axis groups (Agentic, Academic, Indexes) — each preset maps to 3-6 benchmark axes
- Scatter axis selection cycles through benchmark metrics via `ScatterAxis::next()`
- Detail panel uses `ScrollablePanel` widget with `detail_scroll: ScrollOffset` for scrollable, focus-aware rendering
- Browse mode focus navigation uses directional `focus_left()`/`focus_right()` cycling through Creators → List → Details
- `reset_detail_scroll()` called on every benchmark selection change (navigation, filter, sort, rebuild)

## Creator Grouping
- Region grouping (`[5]`): US, China, Europe, Middle East, South Korea, Canada, Other
- Type grouping (`[6]`): Startup, Giant, Research
- Group headers are non-selectable category separators (same pattern as Models tab)

## Gotchas
- Compare mode list shows compact rows with reasoning/source indicators (R/AR/NR + O/C) — different format from browse mode
- Scatter plot auto-applies log scale when value range ratio > 2.5
