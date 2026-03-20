# Compare Tab Design

## Overview

A new 4th tab ("Compare") for side-by-side model comparison with visual charting. Two modes: a browse/select default mode with scatter plot, and a head-to-head mode with radar chart and ranked comparison table.

## Motivation

The Benchmarks tab is effective for browsing and filtering the full model list, but doesn't support focused comparison of specific models. The Compare tab fills this gap with visual tools (scatter plots, radar charts) and a ranked comparison table that highlights winners per metric.

## Tab Structure

```
Tab order: Models | Agents | Benchmarks | Compare
```

New `Tab::Compare` variant. Navigated with `[`/`]` like existing tabs.

## Modes

### Default Mode (Browse + Select)

Layout: ranked table (top ~45%) + scatter plot (bottom ~55%).

**Ranked Table:**
- All models from BenchmarkStore, sorted/filterable
- Key columns: Name, Intelligence, Coding, Math, Speed, Price
- `Space` toggles a model into/out of the compare set (max 8)
- Selected models show a visual marker (colored dot or `*` prefix)
- Sortable with `s`/`S` (same pattern as Benchmarks tab)
- Searchable with `/`

**Scatter Plot:**
- All models as dim Braille dots on a 2D plane
- Default axes: Intelligence Index (Y) vs Blended Price (X)
- Selected models rendered as bright colored dots with name labels
- `x`/`y` keys cycle the X/Y axis metric
- Built-in ratatui `Chart` widget with `GraphType::Scatter` + `Marker::Braille`
- Axis bounds: min/max of active metric across all models, 5% padding

### H2H Mode (Head-to-Head)

Activated with `Enter` (requires 2+ models selected). `Esc`/`Enter` returns to default mode.

Layout: ranked comparison table (top ~40%) + radar chart (bottom ~60%).

**Ranked Comparison Table:**
- Columns = selected models (2-8), Rows = metrics
- Metric row groups:
  - Indexes: Intelligence, Coding, Math
  - Benchmarks: GPQA, LiveCodeBench, SciCode, TerminalBench, IFBench, LCR, MATH-500, AIME, AIME'25, MMLU-Pro, HLE, Tau2
  - Performance: Speed, TTFT, TTFAT
  - Pricing: Input, Output, Blended
- Each cell: value + rank (e.g., `89.2 #1`)
- Winner per row: highlighted bold + Cyan
- Missing values: `—` with no rank
- Scrollable vertically

**Radar Chart:**
- One polygon per selected model, each a distinct color
- Default axes (6 spokes, agentic focus): Coding Index, LiveCodeBench, SciCode, TerminalBench, IFBench, LCR
- Values normalized 0-1 relative to max in full BenchmarkStore (absolute standing)
- `a` key cycles axis presets: Agentic -> Academic -> Indexes -> Custom
- Canvas widget with Braille markers for high-resolution polygon rendering
- Legend: model name -> color mapping

**Chart toggle:** `v` swaps bottom pane between radar and scatter in H2H mode.

## Keybindings

### Default Mode (Browse + Select)

| Key | Action |
|-----|--------|
| `j`/`k`, arrows | Navigate table rows |
| `Space` | Toggle model for comparison |
| `Enter` | Enter H2H mode (2+ selected) |
| `s`/`S` | Cycle sort column / toggle direction |
| `x` | Cycle scatter X axis metric |
| `y` | Cycle scatter Y axis metric |
| `/` | Search models by name |
| `h`/`l`, `Tab` | Switch focus between table and scatter |
| `c` | Clear all selections |

### H2H Mode

| Key | Action |
|-----|--------|
| `j`/`k`, arrows | Scroll metric rows in table |
| `v` | Toggle bottom pane: radar <-> scatter |
| `a` | Cycle radar axis presets |
| `Esc`/`Enter` | Return to default mode |
| `o` | Open selected model on Artificial Analysis |

### Cross-Tab (Benchmarks tab)

| Key | Action |
|-----|--------|
| `m` | Toggle current model for comparison |

## Selection Mechanics

- `compare_selections: Vec<usize>` on `App` (indices into BenchmarkStore) — shared between tabs
- Max 8 models; attempting to add a 9th shows a status message
- Selections persist across tab switches
- `m` on Benchmarks tab and `Space` on Compare tab both modify the same shared set
- `c` on Compare tab clears all selections

## Technical Implementation

### New Files

- `src/tui/compare_app.rs` — CompareApp state, CompareView enum, CompareFocus enum, selection logic, radar axis configuration
- `src/tui/radar.rs` — radar chart renderer: Canvas + Braille, polygon vertex math, normalization

### Modified Files

- `src/tui/app.rs` — Tab::Compare variant, CompareApp field, Message variants (ToggleCompareModel, EnterH2H, ExitH2H, CycleScatterX, CycleScatterY, CycleRadarPreset, ToggleH2HChart, ClearSelections)
- `src/tui/event.rs` — keybinding mappings for Compare tab + `m` on Benchmarks tab
- `src/tui/ui.rs` — draw_compare_default, draw_compare_h2h, draw_scatter, draw_radar render functions
- `src/tui/mod.rs` — initialize CompareApp alongside BenchmarksApp

### Data Flow

- No new data fetching — reads from existing BenchmarkStore
- Radar normalization: compute max values per metric from full store at rebuild time, cache on CompareApp
- CompareApp gets a `rebuild()` method called when BenchmarkDataReceived arrives (same as BenchmarksApp)

### Radar Chart Math

- N spokes at equal angles: `angle_i = 2*pi * i / N - pi/2` (start at top)
- Vertex position: `(cx + r * cos(angle), cy + r * sin(angle))` where `r = value / max_value * radius`
- Draw: axis lines from center to edge, model polygons as connected Canvas line segments
- Labels: placed at axis endpoints beyond the outer ring

### Scatter Plot

- ratatui `Chart` widget with `GraphType::Scatter` + `Marker::Braille`
- Background dataset: all models in DarkGray
- Per-selected-model datasets: distinct color + name in legend
- LegendPosition::TopRight

### Color Palette (for model differentiation)

Cycle: Cyan, Yellow, Green, Magenta, Red, Blue, LightCyan, LightYellow (8 colors for 8 max models).

## Radar Axis Presets

| Preset | Axes |
|--------|------|
| Agentic (default) | Coding Index, LiveCodeBench, SciCode, TerminalBench, IFBench, LCR |
| Academic | GPQA, MMLU-Pro, HLE, MATH-500, AIME, AIME'25 |
| Indexes | Intelligence, Coding, Math |
| Custom | User-configured (future enhancement) |
