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
- **Models Tab** (`src/tui/app.rs`, `src/tui/ui.rs`) — browse models from models.dev API with 3-column layout (20% providers | 45% model list | 35% detail panel), RTFO capability indicators, adaptive provider panel
- **Benchmarks Tab** (`src/tui/benchmarks_app.rs`) — compare model benchmarks from Artificial Analysis with browse/compare modes, H2H table, scatter plot, radar chart views
- **Agents Tab** (`src/tui/agents_app.rs`) — track AI coding assistants with GitHub integration
- **Status Tab** (`src/tui/status_app.rs`) — live provider health monitoring with detail view for incidents, components, and scheduled maintenance

### Data Flow
- Model data: fetched from models.dev API at startup (`src/api.rs`)
- Benchmark data: fetched fresh from jsDelivr CDN on every launch (`src/benchmark_fetch.rs`)
- Agent/GitHub data: disk-cached with ETag conditional fetching (`src/agents/cache.rs`, `src/agents/github.rs`)
- CLI agents: uses `fetch_releases_only` (1 API call, no repo metadata) — TUI uses full `fetch_conditional` (2 calls, includes stars/issues/license)
- Status data: fetched from each provider's official status page (Statuspage, BetterStack, Instatus, etc.) with apistatuscheck.com as fallback (`src/status_fetch.rs`), provider registry and strategy mapping in `src/status.rs`

### Async Pattern
Background fetches use tokio::spawn + mpsc channels. Results arrive as `Message` variants processed in the main loop (`src/tui/mod.rs`). The app never blocks on network calls.

### Agents CLI
- `src/cli/agents.rs` — clap subcommands, dispatch, tool resolution, all agent commands
- `src/cli/agents_ui.rs` — inline release browser, source picker, agent disambiguation picker (ratatui `Viewport::Inline`)
- `src/agents/changelog_parser.rs` — comrak-backed markdown parser producing normalized IR (`ChangelogBlock` enum: `Heading`/`Bullet`/`Paragraph`)
- `src/agents/helpers.rs` — relative time formatting, release frequency, date parsing
- Binary aliases: `models agents <cmd>` or `agents <cmd>` via argv[0] symlink detection
- Commands: `status`, `latest`, `list-sources`, `<tool>` (with `--latest`, `--list`, `--pick`, `--version`, `--web`)
- Uses termimad for styled markdown output in TTY, plain text when piped

### CLI Subcommands
- `models list` / `models search` / `models show` — thin wrappers in `src/cli/list.rs`, `search.rs`, `show.rs` delegating to `src/cli/models.rs`
- `models benchmarks` — interactive benchmark picker (`src/cli/benchmarks.rs`)
- `models completions <shell>` — shell completion generation via clap_complete
- `src/cli/styles.rs` — shared color constants and styling for CLI output

### CLI Inline Pickers
- `src/cli/models.rs` — interactive model picker with `/` filter, `s`/`S` sort, preview pane
- `src/cli/benchmarks.rs` — interactive benchmark picker with filter, sort, detail preview
- `src/cli/agents_ui.rs` — release browser with changelog preview, source picker with tracking management
- All use `PickerTerminal` wrapper (ratatui `Viewport::Inline`) for raw mode lifecycle

### Key Files
- `src/tui/mod.rs` — startup, event loop, async channel handling
- `src/tui/app.rs` — App state, Message enum, update logic, Tab enum (Models/Agents/Benchmarks/Status)
- `src/tui/event.rs` — keybinding → Message mapping
- `src/tui/ui.rs` — rendering
- `src/tui/markdown.rs` — custom markdown-to-ratatui converter (headers, bullets, bold, code, URLs, search highlighting)
- `src/tui/benchmarks_app.rs` — BenchmarksApp state, compare mode, H2H/scatter/radar views
- `src/tui/status_app.rs` — StatusApp state, provider list/detail focus, search filtering
- `src/benchmarks.rs` — BenchmarkStore, BenchmarkEntry
- `src/benchmark_fetch.rs` — jsDelivr CDN fetcher (no cache, no ETag)
- `src/model_traits.rs` — runtime matching of AA entries to models.dev for open/closed status, reasoning, tool_call, and context limits
- `src/status.rs` — ProviderHealth, ProviderStatus, StatusProviderSeed, STATUS_REGISTRY
- `src/status_fetch.rs` — StatusFetcher, official status page fetchers (Statuspage/BetterStack/Instatus/etc.) with apistatuscheck.com fallback
- `src/config.rs` — user config file (agents, cache, display settings)
- `src/data.rs` — Provider/Model data structures from models.dev API
- `src/cli/styles.rs` — shared CLI color constants and styling

### GitHub Actions
- `ci.yml` — runs on PR/push: fmt check, clippy, test
- `release.yml` — triggered by `v*` tags: builds 5 targets in parallel with Rust caching, packages .deb/.rpm via cargo-binstall (pinned versions), generates SHA256SUMS, publishes to crates.io, and updates AUR package. Homebrew Core updates are handled in `Homebrew/homebrew-core` by Homebrew automation/maintainers, not from this repo. Pre-release tags (containing `-`) skip publish/AUR and mark the GitHub release as prerelease. Scoop Extras handles Windows updates via its own autoupdate mechanism.
- `update-benchmarks.yml` — runs every 30 minutes: fetches AA API, commits if data changed

## Conventions
- Use `mise run <task>` for all CLI operations — never run bare commands
- Keep clippy clean with `-D warnings`
- Enum-based message passing (no callbacks)
- No disk cache — benchmark data fetched fresh from CDN on every launch, empty store until CDN responds
- `BenchmarkEntry` must derive both `Serialize` and `Deserialize`
- New `BenchmarkEntry` fields require `#[serde(default)]`

## Gotchas
- clippy `-D warnings` treats unused enum variant fields as errors — if a Message variant's payload is only passed through (e.g., error strings logged nowhere), use a unit variant instead
- `Cargo.lock` must be committed after `Cargo.toml` version bumps
- GitHub Actions `workflow_dispatch` only works when the workflow file exists on the default branch — cannot test from feature branches
- Adding a new field to `BenchmarkEntry`: add field with `#[serde(default)]` — no cache versioning needed since data is fetched fresh every launch
- The AA API uses `0` as a sentinel for missing performance data — jq transforms must convert `0` → `null` (e.g., `if . == 0 then null else . end`)
- jq transforms use null-safe access (`?.` / `// null`) for nested objects — `mise.toml` and `update-benchmarks.yml` must stay in sync
- Never use `eprintln!` in TUI mode — stderr output corrupts ratatui's alternate screen buffer, causing rendering glitches. Use `Message` variants or status bar updates instead. (`eprintln!` is fine in CLI-only code paths like `src/cli/agents.rs`)
- `Paragraph::scroll((y, 0))` with `.wrap(Wrap { trim: false })` counts **visual (wrapped) lines**, not logical lines — scroll positions must account for line wrapping when jumping to specific content
- Use `line.width()` (unicode-aware) not `.len()` (byte count) when computing wrapped line heights — ratatui wraps on display width, not byte length. Word-wrapping needs +1 buffer per wrapped line since `div_ceil` underestimates
- TLS uses `rustls-tls-native-roots` (not `rustls-tls`) — loads certificates from the OS trust store to support corporate TLS-inspecting proxies

## Releasing
1. Bump version in `Cargo.toml`
2. `mise run fmt && mise run clippy && mise run test`
3. Commit `Cargo.toml` and `Cargo.lock` together
4. `git tag v<version> && git push && git push --tags`
5. Release workflow runs automatically: builds binaries, packages .deb/.rpm, publishes to crates.io, and updates AUR package. Homebrew Core bumps happen separately in `Homebrew/homebrew-core`.

## Secrets
- `AA_API_KEY` — Artificial Analysis API key (GitHub repo secret, local `.env`)
- `AUR_SSH_PRIVATE_KEY` — SSH key for pushing to AUR (`~/.ssh/aur`)
- `CARGO_REGISTRY_TOKEN` — crates.io publish token (GitHub repo secret)
