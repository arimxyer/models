# CLI Module — Architecture & Patterns

## Module Purpose
Subcommands for models, benchmarks, agents: thin clap wrappers (`list.rs`, `search.rs`, `show.rs`, `benchmarks.rs`) that delegate to interactive pickers or direct output. Binary aliases support `models <cmd>`, `agents <cmd>`, and `benchmarks <cmd>` (detected via argv[0]).

## Shared Picker Infrastructure (`picker.rs`)

- **`PickerTerminal`** — wraps ratatui `Viewport::Inline` with raw mode lifecycle (Drop trait disables raw mode + clears cursor)
- **`VIEWPORT_HEIGHT = 14`** — fixed inline height; preview panes must fit within this constraint
- **Navigation functions** — `nav_next()`, `nav_previous()`, `nav_first()`, `nav_last()`, `nav_page_down()`
- **Style constants** — HEADER_STYLE (Cyan bold), ROW_HIGHLIGHT_STYLE (Yellow bold), ACTIVE_BORDER_STYLE (Cyan)
- **Layout pattern** — outer vertical split (content + 1-line status bar), inner horizontal split (table + preview)

## Inline Picker Pattern

All 3 pickers (models, benchmarks, agents) follow the same lifecycle:

1. Create `PickerTerminal::new()` (enables raw mode)
2. Init `TableState::new()` and populate data
3. Event loop: poll stdin, update state, render frame
4. Drop `PickerTerminal` auto-disables raw mode
5. Return selected result or None

**Gotchas:**
- `TableState::select(Some(idx))` required before render — starts unselected otherwise
- Preview must scroll with content (use `Paragraph::scroll` for long changelogs)
- `table.bottom_margin(1)` removes blank header separator for tight picker layouts

## Command Structure

- `models list` — filters + sort, delegates to picker or table output
- `models search <query>` — keyword match, interactive picker for selection
- `models show <name>` — single-model detail view with benchmarks/capabilities
- `models providers` — list all providers, supports --json
- `models completions <shell>` — generate shell completions (bash/fish/zsh/elvish/powershell)
- `models benchmarks` — interactive picker, can output JSON via --json
- `agents status|latest|list-sources` — table output; `agents status` sorts by most recently updated and includes a "Status" column with service health icons
- `agents <tool>` — release browser with changelog search (agents_ui.rs)
- `status list` — interactive picker (TTY) or table (non-TTY), shows tracked providers
- `status show <provider>` — detailed provider status with components, incidents, maintenance
- `status status` — quick summary table (always table, even on TTY)
- `status sources` — interactive picker to manage tracked providers (persists to config)

Tool-specific flags parsed manually in `ToolArgs::parse_from()` (not clap, since tools are `external_subcommand`).

## Key Files

- `mod.rs` — module index
- `picker.rs` — PickerTerminal, nav helpers, shared styles, VIEWPORT_HEIGHT constant
- `models.rs` — ModelRow/ProviderInfo data, filter/sort logic, interactive picker render
- `benchmarks.rs` — BenchmarkRow, scorer, H2H picker render
- `agents.rs` — clap schema, ToolArgs parsing, command dispatch
- `agents_ui.rs` — release browser, changelog search (n/N), source picker (ratatui inline)
- `status.rs` — status CLI: list picker, show detail, status table, sources picker. Same `StatusFetcher` pipeline as TUI
- `list.rs`, `search.rs`, `show.rs` — subcommand wrappers, delegate to models.rs logic
- `styles.rs` — shared CLI colors (not duplicated from tui/ palette)
- `link.rs` — symlink creation/removal for binary aliases, reads names from config

## Gotchas

- **No `eprintln!` in picker code** — stdout is in raw mode; stderr corrupts output. Use silent failures or status bar messages instead.
- **Arboard clipboard persistence** — on Linux/Wayland, `Clipboard` object must outlive the copy operation. Spawn a background thread with `sleep(2s)` to keep it alive (models picker `'c'` key).
- **Comrak AST lifetime invariance** — `parse_changelog()` requires named lifetimes: `fn f<'a>(node: &'a AstNode<'a>)`.
- **Preview pane preview panic prevention** — agents changelog preview must handle very long changelogs; use `truncate()` if needed to fit VIEWPORT_HEIGHT constraint.
