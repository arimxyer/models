# Source Layout

## Module Map

| Directory | Purpose | CLAUDE.md |
|-----------|---------|-----------|
| `agents/` | Agent data, GitHub integration, caching, changelog parsing, agent-to-status-provider mapping (`health.rs`) | Yes |
| `benchmarks/` | Benchmark store, CDN fetching, models.dev trait matching | Yes |
| `cli/` | Subcommands, inline pickers, shared picker infrastructure | Yes |
| `status/` | Provider health types, registry, assessment, fetch adapters | Yes |
| `tui/` | App state, sub-apps, event handling, per-tab rendering | Yes |
| `bin/` | `agents` binary alias (argv[0] symlink detection) | — |

## Top-Level Files

| File | Purpose |
|------|---------|
| `main.rs` | Clap CLI definition, command dispatch, TUI launch |
| `api.rs` | Synchronous models.dev API fetch (blocking reqwest — intentionally not async, runs before tokio runtime) |
| `data.rs` | `Provider`, `Model`, `ProvidersMap` — core data structures from models.dev. Used by nearly every module |
| `config.rs` | User config (`~/.config/models/config.toml`) — tracked agents, cache settings, display preferences, symlink aliases (`[aliases]` section) |
| `formatting.rs` | Shared utilities: `truncate`, `parse_date`, `format_tokens`, `format_stars`, `cmp_opt_f64`, `EM_DASH` |
| `provider_category.rs` | `ProviderCategory` enum (Origin/Cloud/Inference/Gateway/Tool), categorization logic, display labels |

## Cross-Module Dependencies

- `data.rs` is imported by everything — treat as foundational, avoid adding module-specific logic
- `formatting.rs` is imported by CLI, TUI, agents, and status — keep functions generic and stateless
- `api.rs` uses `reqwest::blocking` (not async) — this is intentional, called before the tokio runtime starts
- `config.rs` is consumed by agents (tracked agents) and TUI (display settings)
- `provider_category.rs` is consumed by TUI models tab and CLI models picker
