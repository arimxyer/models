# models

<p align="center">
  <a href="https://crates.io/crates/modelsdev"><img src="https://img.shields.io/crates/v/modelsdev.svg?label=version" alt="Version"></a>
  <a href="https://github.com/arimxyer/models/actions/workflows/ci.yml"><img src="https://github.com/arimxyer/models/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
</p>

TUI and CLI for browsing AI models, benchmarks, and coding agents.

- **Models Tab**: Browse 3,000+ models across 85+ providers from [models.dev](https://models.dev) with capability indicators, adaptive layouts, and provider categorization
- **Benchmarks Tab**: Compare model performance across 15+ benchmarks from [Artificial Analysis](https://artificialanalysis.ai), with head-to-head tables, scatter plots, radar charts, and creator filtering
- **Agents Tab**: Track AI coding assistants (Claude Code, Aider, Cursor, etc.) with version detection, changelogs, and GitHub integration

<video src="https://github.com/user-attachments/assets/07c750f4-ca47-4f89-8a32-99e0be5004d8" controls width="100%"></video>
> **Video (and screenshots below) are out-of-sync with the current state of the app, I've been moving fast on making changes and so I'll have to record a new one!**

## What's New

- **Models tab redesign** — capability indicators, adaptive provider panel, and detailed model info at a glance
- **Benchmark compare mode** — head-to-head tables, scatter plots, and radar charts for selected models
- **Linux packages** — native .deb and .rpm packages for x86_64 and aarch64
- **Agents CLI** — track agent releases, view changelogs, and compare versions from the terminal

## Features

### Models Tab
- **Capability indicators** — see Reasoning, Tools, Files, and Open/Closed status at a glance in the model list
- **Provider categories** — filter and group providers by type (Origin, Cloud, Inference, Gateway, Dev Tool)
- **Detail panel** — capabilities, pricing, modalities, and metadata for the selected model
- **Cross-provider search** to compare the same model across different providers
- **Copy to clipboard** with a single keypress
- **CLI commands** and **JSON output** for scripting and automation

### Agents Tab
- **Curated catalog** of 12+ AI coding assistants
- **Version detection** — automatically detects installed agents
- **GitHub integration** — stars, releases, changelogs, update availability
- **Styled changelogs** — markdown rendering with syntax highlighting in the detail pane
- **Changelog search** — search across changelogs with highlighted matches and `n`/`N` jump-to-match
- **Persistent cache** — instant startup with ETag-based conditional fetching
- **Customizable tracking** — choose which agents to monitor

### Benchmarks Tab
- **~400 benchmark entries** from Artificial Analysis with quality, speed, and pricing scores
- **Compare mode** — select models for head-to-head tables, scatter plots, and radar charts
- **Auto-updating** — benchmark data refreshed automatically every 30 minutes
- **Creator sidebar** with 40+ creators — filter by region, type, or open/closed source
- **Sort & filter** — sort by any metric, filter by reasoning capability, source type, and more
- **Detail panel** — full benchmark breakdown with indexes, scores, performance, and pricing

### Agents CLI
- **Status table** — see installed vs latest version, 24h release indicator, and release frequency at a glance
- **Changelogs** — view release notes for any agent by name or version
- **Interactive picker** — fuzzy-select any version with `--pick`, view its changelog
- **Dual entry point** — use as `models agents` or create an `agents` symlink for standalone usage
- **Fast** — concurrent GitHub fetching and version detection

## Installation

### Cargo (from crates.io)

```bash
cargo install modelsdev
```

### Homebrew (macOS/Linux)

```bash
brew install models
```

> **Migrating from the tap?** Run `brew untap arimxyer/tap` — updates now land through `homebrew-core` bump PRs and may take a bit to merge.

### Scoop (Windows)

```powershell
scoop install extras/models
```

> **Migrating from the custom bucket?** Run `scoop bucket rm arimxyer` — Scoop Extras handles updates automatically.

### Arch Linux (AUR)

```bash
paru -S models-bin   # or: yay -S models-bin
```

> Maintained by [@Dominiquini](https://aur.archlinux.org/packages/models-bin)

### Debian / Ubuntu

Download the `.deb` from [GitHub Releases](https://github.com/arimxyer/models/releases) and install:

```bash
# Download the latest .deb for your architecture (amd64 or arm64)
sudo dpkg -i modelsdev_*_amd64.deb
```

### Fedora / RHEL

Download the `.rpm` from [GitHub Releases](https://github.com/arimxyer/models/releases) and install:

```bash
# Download the latest .rpm for your architecture (x86_64 or aarch64)
sudo rpm -i modelsdev-*.x86_64.rpm
```

> **Verifying downloads**: Each GitHub Release includes a `SHA256SUMS` file. After downloading, verify with: `sha256sum -c SHA256SUMS --ignore-missing`

### Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/arimxyer/models/releases).

### Build from source

```bash
git clone https://github.com/arimxyer/models
cd models
cargo build --release
./target/release/models
```

## TUI Usage

### Interactive Browser

Run `models` with no arguments to launch the interactive TUI:

```bash
models
```

![Models tab screenshot](public/assets/models-screenshot.png)

### Keybindings

**Global**
| Key | Action |
|-----|--------|
| `]` / `[` | Switch tabs (Models / Agents / Benchmarks) |
| `?` | Show context-aware help |
| `q` | Quit |

**Navigation**
| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Jump to first item |
| `G` | Jump to last item |
| `Ctrl+d` / `PageDown` | Page down |
| `Ctrl+u` / `PageUp` | Page up |
| `Tab` / `Shift+Tab` | Switch panels |
| `←` / `→` | Switch panels |

**Search**
| Key | Action |
|-----|--------|
| `/` | Enter search mode |
| `Enter` / `Esc` | Exit search mode |
| `Esc` | Clear search (in normal mode) |

### Models Tab

**Filters & Sort**
| Key | Action |
|-----|--------|
| `s` | Cycle sort (name → date → cost → context) |
| `S` | Toggle sort direction (asc/desc) |
| `1` | Toggle reasoning filter |
| `2` | Toggle tools filter |
| `3` | Toggle open weights filter |
| `4` | Toggle free models filter |
| `5` | Cycle provider category filter (All → Origin → Cloud → Inference → Gateway → Tool) |
| `6` | Toggle category grouping |

**Copy & Open**
| Key | Action |
|-----|--------|
| `c` | Copy `provider/model-id` |
| `C` | Copy `model-id` only |
| `o` | Open provider docs in browser |
| `D` | Copy provider docs URL |
| `A` | Copy provider API URL |

### Agents Tab

![Agents tab screenshot](public/assets/agents-screenshot.png)

**Filters & Sort**
| Key | Action |
|-----|--------|
| `s` | Cycle sort (name → updated → stars → status) |
| `1` | Toggle installed filter |
| `2` | Toggle CLI tools filter |
| `3` | Toggle open source filter |

**Search**
| Key | Action |
|-----|--------|
| `/` | Search agents and changelogs |
| `n` | Jump to next match |
| `N` | Jump to previous match |

**Actions**
| Key | Action |
|-----|--------|
| `a` | Open tracked agents picker |
| `o` | Open docs in browser |
| `r` | Open GitHub repo |
| `c` | Copy agent name |

### Customizing Tracked Agents

By default, models tracks 4 popular agents: Claude Code, Codex, Gemini CLI, and OpenCode.

Press `a` in the Agents tab to open the picker and customize which agents you track. Your preferences are saved to `~/.config/models/config.toml`.

You can also add custom agents not in the catalog:

```toml
# ~/.config/models/config.toml
[[agents.custom]]
name = "My Agent"
repo = "owner/repo"
binary = "my-agent"
version_command = ["--version"]
```

See [Custom Agents](docs/custom-agents.md) for the full reference.

### Benchmarks Tab

![Benchmarks tab screenshot](public/assets/benchmark-screenshot.png)

**Quick Sort** (press again to toggle direction)
| Key | Action |
|-----|--------|
| `1` | Sort by Intelligence index |
| `2` | Sort by Release date |
| `3` | Sort by Speed (tok/s) |

**Filters**
| Key | Action |
|-----|--------|
| `4` | Cycle source filter (All / Open / Closed) |
| `5` | Cycle region filter (US / China / Europe / ...) |
| `6` | Cycle type filter (Startup / Big Tech / Research) |
| `7` | Cycle reasoning filter (All / Reasoning / Non-reasoning) |

**Sort**
| Key | Action |
|-----|--------|
| `s` | Open sort picker popup |
| `S` | Toggle sort direction (asc/desc) |

**Compare Mode**
| Key | Action |
|-----|--------|
| `Space` | Toggle model selection (max 8) |
| `v` | Cycle view (H2H table → Scatter → Radar) |
| `t` | Toggle left panel (Models / Creators) |
| `d` | Show detail overlay (H2H view) |
| `c` | Clear all selections |
| `h` / `l` | Switch focus (List / Compare) |
| `j` / `k` | Scroll H2H table (when Compare focused) |
| `x` / `y` | Cycle scatter plot axes |
| `a` | Cycle radar chart preset |

**Actions**
| Key | Action |
|-----|--------|
| `o` | Open Artificial Analysis page |

---

## CLI Usage

### Agents CLI

Track AI coding agent releases from the command line. Install the `agents` alias during setup, or use `models agents` as a fallback.

```bash
# Create the agents alias (one-time setup)
mkdir -p ~/.local/bin
ln -s $(which models) ~/.local/bin/agents
```

> **Note:** Make sure `~/.local/bin` is in your `PATH`. For example, in **bash/zsh** add `export PATH="$HOME/.local/bin:$PATH"` to your shell config, or in **fish** run `fish_add_path ~/.local/bin`.

#### Status table

```bash
agents status
```

```
┌──────────────┬─────┬───────────┬──────────┬─────────┬───────────────┐
│ Tool         │ 24h │ Installed │ Latest   │ Updated │ Freq.         │
├──────────────┼─────┼───────────┼──────────┼─────────┼───────────────┤
│ Claude Code  │ ✓   │ 2.1.42    │ 2.1.42   │ 1d ago  │ ~1d           │
│ OpenAI Codex │ ✓   │ 0.92.0    │ 0.92.0   │ 6h ago  │ ~3h           │
│ Goose        │     │ —         │ 1.0.20   │ 3d ago  │ ~2d           │
└──────────────┴─────┴───────────┴──────────┴─────────┴───────────────┘
```

#### View changelogs

```bash
agents claude              # Latest changelog (by CLI binary name)
agents claude-code         # By agent ID
agents claude --version 1.0.170  # Specific version
```

#### Browse versions

```bash
agents claude --list       # List all versions
agents claude --pick       # Interactive fuzzy picker
```

#### Other commands

```bash
agents latest              # All releases from last 24 hours
agents list-sources        # List all available agents
agents claude --web        # Open GitHub releases in browser
```

### Models CLI

#### List providers

```bash
models list providers
```

#### List models

```bash
# All models
models list models

# Models from a specific provider
models list models anthropic
```

#### Show model details

```bash
models show claude-opus-4-5-20251101
```

```
Claude Opus 4.5
===============

ID:          claude-opus-4-5-20251101
Provider:    Anthropic (anthropic)
Family:      claude-opus

Limits
------
Context:     200k tokens
Max Output:  64k tokens

Pricing (per million tokens)
----------------------------
Input:       $5.00
Output:      $25.00
Cache Read:  $0.50
Cache Write: $6.25

Capabilities
------------
Reasoning:   Yes
Tool Use:    Yes
Attachments: Yes
Modalities:  text, image, pdf -> text

Metadata
--------
Released:    2025-11-01
Updated:     2025-11-01
Knowledge:   2025-03-31
Open Weights: No
```

#### Search models

```bash
models search "gpt-4"
models search "claude opus"
```

#### JSON output

All model commands support `--json` for scripting:

```bash
models list providers --json
models show claude-opus-4-5 --json
models search "llama" --json
```

## Data Sources

Lots of gratitude to the companies who do all the hard work! Shout out to the sources:
- **Model data**: Fetched from [models.dev](https://models.dev), an open-source database of AI models maintained by [SST](https://github.com/sst/models.dev)
- **Benchmark data**: Fetched from [Artificial Analysis](https://artificialanalysis.ai) — quality indexes, benchmark scores, speed, and pricing for ~400 model entries
- **Agent data**: Curated catalog in [`data/agents.json`](data/agents.json) — contributions welcome!
- **GitHub data**: Fetched from GitHub API (stars, releases, changelogs)

## Roadmap

- **Nix flake** — Nix packaging with a proper `flake.lock` for reproducible builds (PRs welcome!)

## Contributing

Contributions are welcome! Please read the [Contributing Guide](CONTRIBUTING.md) before submitting a PR.

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

## License

MIT
