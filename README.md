# models

<p align="center">
  <a href="https://crates.io/crates/modelsdev"><img src="https://img.shields.io/crates/v/modelsdev.svg?label=version" alt="Version"></a>
  <a href="https://github.com/arimxyer/models/actions/workflows/ci.yml"><img src="https://github.com/arimxyer/models/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
</p>

A fast CLI and TUI for browsing AI models, benchmarks, and coding agents.

- **Models Tab**: Browse 2000+ models across 85+ providers from [models.dev](https://models.dev), categorized by type (Origin, Cloud, Inference, Gateway, Dev Tool)
- **Agents Tab**: Track AI coding assistants (Claude Code, Aider, Cursor, etc.) with version detection and GitHub integration
- **Benchmarks Tab**: Compare model performance across 15+ benchmarks from [Artificial Analysis](https://artificialanalysis.ai), with creator filtering by source, region, and type

<video src="https://github.com/user-attachments/assets/07c750f4-ca47-4f89-8a32-99e0be5004d8" controls width="100%"></video>

## What's New (v0.8.6)

### Cost Sorting
- **Price sort columns** — sort benchmarks by input, output, or blended price per million tokens via `[s]` cycle

### Open Weights Detection
- **Per-model source detection** — runtime matching of AA benchmark entries against models.dev data to determine open/closed status per model (not just per creator)
- **Source filter** — `[4]` cycles through All / Open / Closed to filter the benchmark list by open-weights status

### Creator Grouping
- **Region and type grouping** — `[5]` and `[6]` now toggle grouped layout with colored section headers instead of filtering creators out
- **Colored sidebar** — group headers and creator tags use per-group colors (e.g., US=Blue, China=Red, Startup=Green, Big Tech=Blue)

### v0.8.5: Release Profile
- **Optimized release binary** — strip, LTO, single codegen unit, panic=abort (~6MB, down from ~11MB)

### v0.8.4: Fixes & Cleanup
- **Fixed TUI rendering glitch** — resolved an issue where the interface could render incorrectly on launch without a terminal resize
- **Removed embedded benchmark data** — two-tier loading (disk cache + CDN fetch) replaces the previous three-tier system

### v0.8.3: New Benchmark Fields
- **TTFAT column** — Time to First Answer Token, distinguishing thinking time from TTFT on reasoning models (e.g., 6.7s TTFAT vs 0.46s TTFT)
- **AIME benchmark** — original AIME evaluation score displayed alongside MATH-500 and AIME'25 in the detail panel
- **Stable IDs** — `id` and `creator_id` from Artificial Analysis for reliable cross-session entity tracking
- **Schema-aware cache validation** — stale CDN payloads with missing fields are automatically rejected

### v0.8.2: UI Polish & Consistency
- **Split cost columns** — Models list now shows separate Input and Output cost columns with smart rounding
- **Dynamic column widths** — Model ID and Benchmark Name columns expand to fill available terminal width
- **Focus-aware caret** — `> ` indicator acts as a pseudo cursor, appearing only on the active panel
- **Dynamic panel titles** — Models list shows the selected provider name; Benchmarks list shows the selected creator name
- **Styled column headers** — yellow bold headers with cyan highlight on the active sort column
- **`g`/`G` keybindings** — jump to first/last item on Benchmarks and Agents tabs (already on Models)

### v0.8.1: Runtime Benchmark Data
- **Auto-updating benchmarks** — benchmark data refreshes from CDN every 6 hours in the background, no user configuration required
- **Disk cache** — previously fetched data persists across sessions at `~/.config/models/benchmarks-cache.json`
- **GitHub Action** — daily automated refresh of benchmark data from the Artificial Analysis API

### v0.8.0: Benchmarks Tab (New)
- **Dedicated Benchmarks tab** — browse ~400 model entries from Artificial Analysis with quality, speed, and pricing data
- **Creator sidebar** with 40+ creators, classified by region and type with grouping toggles
- **Source filter** — `[4]` filters by open/closed weights; `[5]`/`[6]` toggle region/type grouping with colored headers
- **Quick-sort keys** — `[1]` Intelligence, `[2]` Date, `[3]` Speed — press again to flip direction
- **Dynamic column visibility** — list columns adapt based on the active sort group (knowledge, code, reasoning, math, performance)
- **Detail panel** — non-scrollable flat layout with indexes, benchmark scores, performance metrics, and pricing
- **Null-filtering** — entries missing data for the active sort column are hidden automatically

### Other
- **Provider categories** — filter and group providers by type (Origin, Cloud, Inference, Gateway, Dev Tool)
- **OpenClaw agent** added to the agents catalog
- **Responsive layouts** — models tab detail panel scales with terminal height

## Features

### Models Tab
- **CLI commands** for scripting and quick lookups
- **Interactive TUI** for browsing and comparing models
- **Provider categories** — filter and group providers by type (Origin, Cloud, Inference, Gateway, Dev Tool)
- **Cross-provider search** to compare the same model across different providers
- **Copy to clipboard** with a single keypress
- **JSON output** for scripting and automation

### Agents Tab
- **Curated catalog** of 12+ AI coding assistants
- **Version detection** — automatically detects installed agents
- **GitHub integration** — stars, releases, changelogs, update availability
- **Persistent cache** — instant startup with ETag-based conditional fetching
- **Customizable tracking** — choose which agents to monitor

### Benchmarks Tab
- **~400 benchmark entries** from Artificial Analysis with quality, speed, and pricing scores
- **Auto-updating** — data refreshes from CDN every 6 hours in the background, persisted to disk cache
- **Creator sidebar** with 40+ creators — group by region or type with colored section headers
- **Per-model open weights detection** — runtime matching against models.dev, with source filter toggle
- **Quick-sort keys** — instantly sort by Intelligence, Date, or Speed
- **Dynamic columns** — list columns adapt to show the most relevant benchmarks for the active sort
- **Detail panel** — full benchmark breakdown with indexes, scores, performance, and pricing

## Installation

### Cargo (from crates.io)

```bash
cargo install modelsdev
```

### Homebrew (macOS/Linux)

```bash
brew install arimxyer/tap/models
```

### Scoop (Windows)

```powershell
scoop bucket add arimxyer https://github.com/arimxyer/scoop-bucket
scoop install models
```

### Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/arimxyer/models/releases).

### Build from source

```bash
git clone https://github.com/arimxyer/models
cd models
cargo build --release
./target/release/models
```

## Usage

### TUI (Interactive Browser)

Just run `models` with no arguments to launch the interactive browser:

```bash
models
```

![Models tab screenshot](public/assets/models-screenshot.png)

### TUI Keybindings

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
| `1` | Toggle reasoning filter |
| `2` | Toggle tools filter |
| `3` | Toggle open weights filter |
| `4` | Cycle provider category filter (All → Origin → Cloud → Inference → Gateway → Tool) |
| `5` | Toggle category grouping |

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

### Benchmarks Tab

![Benchmarks tab screenshot](public/assets/benchmark-screenshot.png)

**Quick Sort** (press again to toggle direction)
| Key | Action |
|-----|--------|
| `1` | Sort by Intelligence index |
| `2` | Sort by Release date |
| `3` | Sort by Speed (tok/s) |

**Filters & Grouping**
| Key | Action |
|-----|--------|
| `4` | Cycle source filter (All / Open / Closed) |
| `5` | Toggle region grouping |
| `6` | Toggle type grouping |

**Sort (full cycle)**
| Key | Action |
|-----|--------|
| `s` | Cycle through all 20 sort columns |
| `S` | Toggle sort direction (asc/desc) |

**Actions**
| Key | Action |
|-----|--------|
| `c` | Copy benchmark name |
| `o` | Open Artificial Analysis page |

### CLI Commands

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

All commands support `--json` for scripting:

```bash
models list providers --json
models show claude-opus-4-5 --json
models search "llama" --json
```

## Data Sources

Lots of gratitude and couldn't have made this application without these workhorses doing the legwork. Shout out to the sources!:
- **Model data**: Fetched from [models.dev](https://models.dev), an open-source database of AI models maintained by [SST](https://github.com/sst/models.dev)
- **Benchmark data**: Fetched from [Artificial Analysis](https://artificialanalysis.ai) — quality indexes, benchmark scores, speed, and pricing for ~400 model entries
- **Agent data**: Curated catalog in [`data/agents.json`](data/agents.json) — contributions welcome!
- **GitHub data**: Fetched from GitHub API (stars, releases, changelogs)

## License

MIT
