# TUI Module Architecture

## Module Structure

The TUI uses per-tab subdirectories, each containing app state and rendering:

```
tui/
├── models/
│   ├── mod.rs     (pub use app::*)
│   ├── app.rs     (ModelsApp state, Focus, Filters, SortOrder)
│   └── render.rs  (draw_main)
├── agents/
│   ├── mod.rs     (pub use app::*)
│   ├── app.rs     (AgentsApp state, AgentFocus, AgentSortOrder)
│   └── render.rs  (draw_agents_main, draw_picker_modal)
├── benchmarks/
│   ├── mod.rs     (pub use app::*)
│   ├── app.rs     (BenchmarksApp state, BenchmarkFocus, BottomView, ScatterAxis, RadarPreset)
│   ├── render.rs  (draw_benchmarks_main, compare_colors)
│   ├── compare.rs (draw_h2h_table_generic, draw_scatter)
│   └── radar.rs   (draw_radar, spoke_angles, polygon_vertices, axes_for_preset)
├── status/
│   ├── mod.rs     (pub use app::*)
│   ├── app.rs     (StatusApp state, StatusFocus, OverallPanelFocus, DetailPanelFocus)
│   ├── render.rs  (draw_status_main, shared pub(super) helpers)
│   ├── overall.rs (draw_overall_dashboard, card builders)
│   └── detail.rs  (draw_provider_status_detail, sorted_*, build_services_title)
├── widgets/
│   ├── mod.rs              (re-exports)
│   ├── scrollable_panel.rs (ScrollablePanel — bordered scroll with Line title support)
│   ├── scroll_offset.rs    (ScrollOffset — Cell<u16> newtype for render-time writeback)
│   ├── soft_card.rs        (SoftCard — health-colored accent stripe cards)
│   └── comparison_legend.rs(ComparisonLegend — benchmarks compare views)
├── mod.rs          (startup, event loop, async channel handling)
├── app.rs          (App struct, Tab, Message enum, update() logic)
├── event.rs        (keybinding → Message mapping, NavAction dedup)
├── ui.rs           (draw(), shared helpers: focus_border, caret, selection_style)
└── markdown.rs     (custom markdown converter, regex-based)
```

### Import Conventions

- **Cross-layer** (render → tui/app.rs, tui/ui.rs): use `crate::tui::app::App`, `crate::tui::ui::{...}`
- **Intra-subdirectory** (render → tab's app.rs): use `super::app::{...}`
- **Tab types from app.rs/event.rs/ui.rs**: use `super::models::`, `super::benchmarks::`, etc.
- Each tab's `mod.rs` uses `pub use app::*;` so types are accessible via e.g. `super::benchmarks::BenchmarkFocus`

### Visibility

- Tab render entry functions use `pub(in crate::tui)` — callable from `ui.rs` but not outside `tui/`
- Tab `render` and `compare` modules use `pub(in crate::tui)` visibility in their parent mod.rs
- Tab `app` modules are `pub` (types used by app.rs, event.rs, and external code)

## NavAction Dedup Pattern

`event.rs` defines a shared `NavAction` enum (Down/Up/First/Last/PageDown/PageUp/FocusLeft/FocusRight/Search/ClearEsc) to avoid duplicating keybinding logic across tabs. `parse_nav_key()` maps crossterm `KeyCode` to `NavAction`, then each tab-specific handler converts `NavAction` to tab-specific `Message` variants. This keeps vim keys and arrow key aliases in one place.

## Adding a New Tab

1. Create `src/tui/{tab}/` directory with `mod.rs`, `app.rs`, and `render.rs`
2. In `mod.rs`: `pub mod app; pub(in crate::tui) mod render; pub use app::*;`
3. Add `pub mod {tab};` to `tui/mod.rs`
4. Add `{Tab}` variant to `Tab` enum in `tui/app.rs`
5. Add tab-specific `Message` variants to the `Message` enum
6. Implement `update()` handlers in `tui/app.rs`
7. Add render call in `ui.rs` via `super::{tab}::render::draw_{tab}_main()`
8. Add keybinding handlers to `event.rs` using `NavAction` pattern
9. Add footer hints and help text to `ui.rs`
10. Follow TUI Style Guide color/border/focus conventions

## Shared UI Helpers

- `focus_border(focused)` → Cyan or DarkGray `Style`
- `caret(focused)` → `"> "` or `"  "` prefix for list items
- `selection_style(selected)` → Yellow+BOLD or default `Style`
- `ScrollablePanel` widget → Bordered panel with scroll, scrollbar, and optional wrap; use instead of manual Block+Paragraph+Scrollbar
- `help_line(key, desc)` → 16-char padded key (Yellow) + description for help popup

## Key Gotchas

- Tab render functions use `pub(in crate::tui)` — callable from `ui.rs` but not from outside `tui/`.
- Sub-app methods needing provider data take `&[(String, Provider)]` as parameter (e.g., `ModelsApp::update_filtered_models`). This is the established pattern for cross-tab data access — don't store shared data on sub-apps.
- Never use `eprintln!` in TUI mode — corrupts ratatui's alternate screen buffer. Use `Message` variants or status bar updates.
- `Paragraph::scroll((y, 0))` counts **visual wrapped lines**, not logical lines — compute cumulative wrapped heights for scroll-to accuracy.
- Use `line.width()` (unicode-aware), not `.len()` (byte count), for width calculations.
- Borrow checker in render: extract values before `Paragraph::new()` consumes them; defer mutable updates after.
- `LazyLock` for compiled regex singletons in `markdown.rs`.
- Async fetches use tokio::spawn + mpsc channels. Results arrive as `Message` variants in the main loop — app never blocks.
