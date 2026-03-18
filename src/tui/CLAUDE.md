# TUI Module Architecture

## Module Structure

The TUI is split across two layers:

- **`app.rs`**: Core state (`App` struct), `Tab` enum (Models/Agents/Benchmarks/Status), `Message` enum, and `update()` logic
- **Sub-apps** (`models_app.rs`, `agents_app.rs`, `benchmarks_app.rs`, `status_app.rs`): Focused tab state (filters, selection, scroll positions)
- **`event.rs`**: Keybinding → `Message` mapping using the `NavAction` dedup pattern
- **`ui.rs`** + tab-specific `ui_*.rs`: Rendering functions and shared helpers (`focus_border()`, `caret()`, `selection_style()`, `render_scrollbar()`)
- **`mod.rs`**: Startup, event loop, async channel handling (GitHub, benchmark, status fetches)
- **`markdown.rs`**: Custom markdown converter (no comrak — regex-based for inline style preservation in detail panels)

## NavAction Dedup Pattern

`event.rs` defines a shared `NavAction` enum (Down/Up/First/Last/PageDown/PageUp/FocusLeft/FocusRight/Search/ClearEsc) to avoid duplicating keybinding logic across tabs. `parse_nav_key()` maps crossterm `KeyCode` to `NavAction`, then each tab-specific handler converts `NavAction` to tab-specific `Message` variants. This keeps vim keys and arrow key aliases in one place.

## Adding a New Tab

1. Create `src/tui/{tab}_app.rs` with state struct (filters, selection, scroll)
2. Add `{Tab}` variant to `Tab` enum in `app.rs`
3. Add tab-specific `Message` variants to the `Message` enum
4. Implement `update()` handlers in `app.rs`
5. Create `src/tui/ui_{tab}.rs` with rendering logic
6. Add render call to `draw_tab()` in `ui.rs`
7. Add keybinding handlers to `event.rs` using `NavAction` pattern
8. Add footer hints and help text to `ui.rs`
9. Follow TUI Style Guide color/border/focus conventions

## Shared UI Helpers

- `focus_border(focused)` → Cyan or DarkGray `Style`
- `caret(focused)` → `"> "` or `"  "` prefix for list items
- `selection_style(selected)` → Yellow+BOLD or default `Style`
- `render_scrollbar()` → Draws vertical scrollbar inside block borders when content overflows
- `help_line(key, desc)` → 16-char padded key (Yellow) + description for help popup

## Key Gotchas

- Never use `eprintln!` in TUI mode — corrupts ratatui's alternate screen buffer. Use `Message` variants or status bar updates.
- `Paragraph::scroll((y, 0))` counts **visual wrapped lines**, not logical lines — compute cumulative wrapped heights for scroll-to accuracy.
- Use `line.width()` (unicode-aware), not `.len()` (byte count), for width calculations.
- Borrow checker in render: extract values before `Paragraph::new()` consumes them; defer mutable updates after.
- `LazyLock` for compiled regex singletons in `markdown.rs`.
- Async fetches use tokio::spawn + mpsc channels. Results arrive as `Message` variants in the main loop — app never blocks.
