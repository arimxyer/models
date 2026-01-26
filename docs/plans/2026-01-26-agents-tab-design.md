# Agents Tab Design

A second tab for browsing AI coding assistants with version tracking and changelogs.

## Overview

Add an "Agents" tab to the models TUI for browsing AI coding assistants (Claude Code, Cursor, aider, etc.). The view mirrors the existing Models tab structure with panels for categories, agent list, and details.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     DATA SOURCES                                 │
├─────────────────────────────────────────────────────────────────┤
│  1. gh-aw agent (weekly)                                        │
│     └─ Reads artificialanalysis.ai comparison page              │
│     └─ Extracts tool data using AI (not HTML parsing)           │
│     └─ Commits agents.json to repo via PR for human review      │
│                                                                  │
│  2. GitHub API (live, lazy-loaded)                              │
│     └─ Fetches releases, changelogs, stars, activity            │
│     └─ Uses `gh api` for 5000 req/hr limit                      │
│     └─ Cached with 1hr TTL                                      │
│                                                                  │
│  3. Local CLI detection (at startup)                            │
│     └─ Scans PATH + common locations for installed tools        │
│     └─ Compares installed vs latest version                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     MODELS TUI                                   │
├─────────────────────────────────────────────────────────────────┤
│  Tab 1: Models (existing)    │  Tab 2: Agents (new)             │
│  └─ Providers panel          │  └─ Categories panel             │
│  └─ Models table             │  └─ Agents table                 │
│  └─ Detail pane              │  └─ Detail pane                  │
└─────────────────────────────────────────────────────────────────┘
```

## Data Model

### Static Data (`data/agents.json`)

Curated by gh-aw agent + manual additions:

```json
{
  "schema_version": 1,
  "last_scraped": "2026-01-26T00:00:00Z",
  "scrape_source": "artificialanalysis.ai",
  "agents": {
    "claude-code": {
      "name": "Claude Code",
      "repo": "anthropics/claude-code",
      "categories": ["cli"],
      "installation_method": "cli",
      "pricing": {
        "model": "usage",
        "free_tier": false,
        "usage_notes": "Pay per token via Anthropic API"
      },
      "supported_providers": ["anthropic"],
      "platform_support": ["macos", "linux", "windows"],
      "open_source": false,
      "cli_binary": "claude",
      "version_command": ["--version"],
      "version_regex": "claude-code v([\\d.]+)",
      "config_files": ["~/.claude/"],
      "homepage": "https://claude.ai/code",
      "docs": "https://docs.anthropic.com/claude-code"
    }
  }
}
```

### Live Data (GitHub API)

Fetched via `gh api`, cached with 1hr TTL:

- `latest_version` - from releases/latest
- `release_date` - from releases/latest
- `changelog` - from releases/latest body
- `stars` - from repo metadata
- `open_issues` - from repo metadata
- `license` - from repo metadata
- `last_commit` - from repo pushed_at

### Computed Data

- `installed_version` - from CLI detection
- `update_available` - installed < latest (semver comparison)
- `tracked` - in user's tracked list

## CLI Detection

For each agent where `installation_method="cli"`:

1. Check PATH via `which {cli_binary}`
2. Check common locations:
   - `/opt/homebrew/bin/` (macOS ARM)
   - `/usr/local/bin/` (macOS Intel / Linux)
   - `~/.local/bin/` (pipx, etc.)
   - `~/.cargo/bin/` (Rust tools)
   - `~/.npm-global/bin/` (npm global)
3. If found, run: `{cli_binary} {version_command}`
4. Parse output with `{version_regex}`
5. Compare against latest using `semver` crate (fallback to string comparison)

## User Configuration

Stored at `~/.config/models/config.toml` (via `dirs` crate for cross-platform):

```toml
config_version = 1

[agents]
tracked = ["claude-code", "aider", "cursor"]
excluded = []  # auto-detected but user doesn't want to track

[cache]
github_ttl_seconds = 3600

[display]
default_tab = "models"
```

## TUI Layout

### Agents Tab

```
┌─ Models ─┬─ Agents ─────────────────────────────────────────────────────────────┐
│          │                                                                       │
│ Categories      │ Agent                   │ Installed │ Latest  │ ⭐     │ Status │
├─────────────────┼─────────────────────────┼───────────┼─────────┼────────┼────────┤
│ ► All (12)      │ Claude Code             │ 1.0.40    │ 1.0.42  │ 12.3k  │ ⬆ Update│
│   Installed (4) │ Cursor                  │ 0.45.2    │ 0.45.2  │ 45.1k  │ ✓ Latest│
│   CLI Tools (6) │ aider                   │ 0.82.1    │ 0.83.0  │ 39.9k  │ ⬆ Update│
│   IDEs (4)      │ Windsurf                │ -         │ 1.2.0   │ 8.2k   │ Not Inst│
│   Cloud (2)     │ Goose                   │ 0.9.12    │ 0.9.12  │ 5.1k   │ ✓ Latest│
│                 │                         │           │         │        │         │
├─────────────────┴─────────────────────────┴───────────┴─────────┴────────┴─────────┤
│ Claude Code                                          anthropics/claude-code        │
├────────────────────────────────────────────────────────────────────────────────────┤
│ Installed: 1.0.40  →  Latest: 1.0.42                            ⬆ UPDATE AVAILABLE │
│ Released: 2025-01-20 (6 days ago)                                                  │
│────────────────────────────────────────────────────────────────────────────────────│
│ ⭐ 12.3k  │  MIT License  │  Usage-based  │  Anthropic only  │  CLI               │
│────────────────────────────────────────────────────────────────────────────────────│
│ v1.0.42 Changelog:                                                                 │
│ • Fixed bug with MCP server connections                                            │
│ • Added support for background agents                                              │
│ • Improved memory management for long sessions                                     │
│────────────────────────────────────────────────────────────────────────────────────│
│ [o] Open Docs  [r] GitHub  [y] Copy name  [u] Copy update cmd                      │
└────────────────────────────────────────────────────────────────────────────────────┘
```

### Add/Remove Picker

Triggered by `a` key:

```
┌─ Add/Remove Tracked Agents ──────────────────────────────────────┐
│                                                                   │
│  [x] Claude Code          cli      installed                     │
│  [x] aider                cli      installed                     │
│  [ ] Cursor               ide      not installed                 │
│  [ ] Windsurf             ide      not installed                 │
│  [x] Goose                cli      installed                     │
│  [ ] Zed                  ide      not installed                 │
│  [ ] Continue             extension                               │
│  [ ] Cline                cli/extension                          │
│                                                                   │
│  ─────────────────────────────────────────────────────────────── │
│  Space: toggle  │  Enter: save  │  Esc: cancel                   │
└───────────────────────────────────────────────────────────────────┘
```

## Keybindings

### Tab Navigation (new)

| Key | Function |
|-----|----------|
| `[` | Previous tab |
| `]` | Next tab |

### Agents Tab (new keys)

| Key | Function |
|-----|----------|
| `a` | Open add/remove picker |
| `r` | Open GitHub repo in browser |
| `u` | Copy update command |

### Shared (same as Models)

| Key | Function |
|-----|----------|
| `j/k`, `↑/↓` | Navigate |
| `g/G` | First/last item |
| `h/l`, `Tab` | Switch panels |
| `/` | Search |
| `s` | Cycle sort |
| `o` | Open docs in browser |
| `c` | Copy identifier |
| `?` | Help |
| `q` | Quit |

### Filters (context-dependent)

Models tab:
- `1` - Toggle reasoning
- `2` - Toggle tools
- `3` - Toggle open weights

Agents tab:
- `1` - Toggle installed only
- `2` - Toggle CLI tools
- `3` - Toggle open source

## Data Pipeline (gh-aw)

Weekly GitHub Action workflow:

```markdown
---
name: Update Agents Data
on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday
  workflow_dispatch: {}
permissions:
  contents: read
  pull-requests: write
---

# Update Agents Data

Read the coding agents comparison page and update our data file.

1. Fetch https://artificialanalysis.ai/insights/coding-agents-comparison
2. Extract the comparison table: tool names, categories, pricing models, supported providers
3. Merge with existing data in `data/agents.json` (preserve GitHub repo mappings)
4. If changes detected, create a PR with the diff for human review

Keep existing fields that aren't on the page (GitHub repo URLs, version commands, etc.).
Don't remove tools that exist in our file but not on the page.
```

## Dependencies

New crates:
- `semver` - Version comparison
- `dirs` - Cross-platform config paths
- `toml` - Config file parsing (may already have via serde)

## Implementation Phases

### Phase 1: Core Infrastructure
- Add `data/agents.json` with initial agent catalog
- Add config file support (`~/.config/models/config.toml`)
- Add GitHub API client with caching

### Phase 2: CLI Detection
- Implement PATH + common location scanning
- Per-tool version command execution
- Semver comparison with fallback

### Phase 3: TUI - Agents Tab
- Tab switching (`[`/`]`)
- Categories panel
- Agents table with columns
- Detail pane with changelog

### Phase 4: TUI - Picker & Config
- Add/remove picker modal
- Persist selections to config
- Filter toggles

### Phase 5: gh-aw Integration
- Create workflow markdown file
- Test scraping/extraction
- Set up PR automation

## Open Questions

None - design is complete.

## References

- [artificialanalysis.ai coding agents comparison](https://artificialanalysis.ai/insights/coding-agents-comparison)
- [sourcegraph/awesome-code-ai](https://github.com/sourcegraph/awesome-code-ai)
- [gh-aw documentation](https://github.com/githubnext/gh-aw)
