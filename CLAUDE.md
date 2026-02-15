# Models — Claude Code Instructions

## Project Overview
A Rust CLI/TUI for browsing AI models, benchmarks, and coding agents. Built with ratatui, crossterm, and tokio.

## Build & Test
```bash
mise run fmt        # Format code (required before commit)
mise run clippy     # Lint with -D warnings
mise run test       # Run tests
mise run build      # Build debug
mise run run        # Run the TUI
```

Always run the full check sequence before committing:
```bash
mise run fmt && mise run clippy && mise run test
```

## Architecture

### Tabs
- **Models Tab** (`src/tui/app.rs`, `src/tui/ui.rs`) — browse models from models.dev API
- **Benchmarks Tab** (`src/tui/benchmarks_app.rs`) — compare model benchmarks from Artificial Analysis
- **Agents Tab** (`src/tui/agents_app.rs`) — track AI coding assistants with GitHub integration

### Data Flow
- Model data: fetched from models.dev API at startup (`src/api.rs`)
- Benchmark data: embedded in binary + auto-refreshed from jsDelivr CDN every 6h (`src/benchmark_cache.rs`, `src/benchmark_fetch.rs`)
- Agent/GitHub data: disk-cached with ETag conditional fetching (`src/agents/cache.rs`, `src/agents/github.rs`)

### Async Pattern
Background fetches use tokio::spawn + mpsc channels. Results arrive as `Message` variants processed in the main loop (`src/tui/mod.rs`). The app never blocks on network calls.

### Key Files
- `src/tui/mod.rs` — startup, event loop, async channel handling
- `src/tui/app.rs` — App state, Message enum, update logic
- `src/tui/event.rs` — keybinding → Message mapping
- `src/tui/ui.rs` — rendering
- `src/benchmarks.rs` — BenchmarkStore (embedded + runtime constructors)
- `src/benchmark_cache.rs` — disk cache with 6h TTL
- `src/benchmark_fetch.rs` — jsDelivr CDN fetcher with ETag

### GitHub Actions
- `ci.yml` — runs on PR/push: fmt check, clippy, test
- `release.yml` — triggered by `v*` tags: builds 5 targets, publishes to crates.io, updates Homebrew/Scoop
- `update-benchmarks.yml` — runs every 6h: fetches AA API, commits if data changed

## Conventions
- Use `mise run <task>` for all CLI operations — never run bare commands
- Keep clippy clean with `-D warnings`
- Enum-based message passing (no callbacks)
- Embedded data as offline fallback, disk cache for freshness, async fetch for updates
- `BenchmarkEntry` must derive both `Serialize` and `Deserialize` (needed for cache)

## Secrets
- `AA_API_KEY` — Artificial Analysis API key (GitHub repo secret, local `.env`)
- `CARGO_REGISTRY_TOKEN` — crates.io publish token (GitHub repo secret)
- `TAP_GITHUB_TOKEN` — for updating Homebrew tap and Scoop bucket repos
