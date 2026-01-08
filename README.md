# models

A fast CLI and TUI for browsing AI model information from [models.dev](https://models.dev).

Quickly look up context windows, pricing, capabilities, and more for 2000+ models across 75+ providers.

<video src="demo.webm" controls width="100%"></video>

## Features

- **CLI commands** for scripting and quick lookups
- **Interactive TUI** for browsing and comparing models
- **Cross-provider search** to compare the same model across different providers
- **Copy to clipboard** with a single keypress
- **JSON output** for scripting and automation

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

![models TUI screenshot](screenshot.png)

#### TUI Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Switch to providers panel |
| `l` / `→` | Switch to models panel |
| `Tab` | Switch panels |
| `/` | Enter search mode |
| `Esc` | Clear search / exit search mode |
| `c` | Copy `provider/model-id` to clipboard |
| `C` | Copy `model-id` to clipboard |
| `q` | Quit |

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

## Data Source

Model data is fetched from [models.dev](https://models.dev), an open-source database of AI models maintained by [SST](https://github.com/sst/models.dev).

## License

MIT
