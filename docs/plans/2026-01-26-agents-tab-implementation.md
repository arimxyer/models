# Agents Tab Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a second TUI tab for browsing AI coding assistants with version tracking, changelogs, and CLI detection.

**Architecture:** Static agent data lives in `data/agents.json`. At runtime, enrich with GitHub API data (cached) and local CLI detection. User preferences persist to `~/.config/models/config.toml`.

**Tech Stack:** Rust, ratatui, serde, semver, dirs, toml, reqwest (existing)

---

## Phase 1: Core Infrastructure

### Task 1.1: Add New Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add semver, dirs, and toml crates**

Add these dependencies to `Cargo.toml` after the `open = "5"` line:

```toml
# Version comparison
semver = "1"

# Cross-platform paths
dirs = "6"

# Config file parsing
toml = "0.8"
```

**Step 2: Verify dependencies resolve**

Run: `cargo check`
Expected: Compiles successfully with new dependencies

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add semver, dirs, and toml dependencies"
```

---

### Task 1.2: Create Agent Data Structures

**Files:**
- Create: `src/agents/data.rs`
- Create: `src/agents/mod.rs`
- Modify: `src/main.rs` (add module)

**Step 1: Create agents module directory**

```bash
mkdir -p src/agents
```

**Step 2: Create data structures in `src/agents/data.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentsFile {
    pub schema_version: u32,
    #[serde(default)]
    pub last_scraped: Option<String>,
    #[serde(default)]
    pub scrape_source: Option<String>,
    pub agents: HashMap<String, Agent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Agent {
    pub name: String,
    pub repo: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub installation_method: Option<String>,
    #[serde(default)]
    pub pricing: Option<Pricing>,
    #[serde(default)]
    pub supported_providers: Vec<String>,
    #[serde(default)]
    pub platform_support: Vec<String>,
    #[serde(default)]
    pub open_source: bool,
    #[serde(default)]
    pub cli_binary: Option<String>,
    #[serde(default)]
    pub version_command: Vec<String>,
    #[serde(default)]
    pub version_regex: Option<String>,
    #[serde(default)]
    pub config_files: Vec<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub docs: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pricing {
    pub model: String,
    #[serde(default)]
    pub subscription_price: Option<f64>,
    #[serde(default)]
    pub subscription_period: Option<String>,
    #[serde(default)]
    pub free_tier: bool,
    #[serde(default)]
    pub usage_notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GitHubData {
    pub latest_version: Option<String>,
    pub release_date: Option<String>,
    pub changelog: Option<String>,
    pub stars: Option<u64>,
    pub open_issues: Option<u64>,
    pub license: Option<String>,
    pub last_commit: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct InstalledInfo {
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentEntry {
    pub id: String,
    pub agent: Agent,
    pub github: GitHubData,
    pub installed: InstalledInfo,
    pub tracked: bool,
}

impl AgentEntry {
    pub fn update_available(&self) -> bool {
        match (&self.installed.version, &self.github.latest_version) {
            (Some(installed), Some(latest)) => {
                // Try semver comparison, fallback to string
                match (semver::Version::parse(installed), semver::Version::parse(latest)) {
                    (Ok(i), Ok(l)) => l > i,
                    _ => latest != installed,
                }
            }
            _ => false,
        }
    }

    pub fn status_str(&self) -> &'static str {
        if self.installed.version.is_none() {
            "Not Inst"
        } else if self.update_available() {
            "⬆ Update"
        } else {
            "✓ Latest"
        }
    }
}
```

**Step 3: Create module file `src/agents/mod.rs`**

```rust
pub mod data;

pub use data::*;
```

**Step 4: Add module to `src/main.rs`**

Add after existing `mod` declarations:

```rust
mod agents;
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src/agents/
git add src/main.rs
git commit -m "feat: add agent data structures"
```

---

### Task 1.3: Create Initial agents.json

**Files:**
- Create: `data/agents.json`

**Step 1: Create data directory and agents.json**

```bash
mkdir -p data
```

**Step 2: Write `data/agents.json`**

```json
{
  "schema_version": 1,
  "last_scraped": null,
  "scrape_source": null,
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
      "version_regex": "([0-9]+\\.[0-9]+\\.[0-9]+)",
      "config_files": ["~/.claude/"],
      "homepage": "https://claude.ai/code",
      "docs": "https://docs.anthropic.com/en/docs/claude-code"
    },
    "aider": {
      "name": "Aider",
      "repo": "paul-gauthier/aider",
      "categories": ["cli"],
      "installation_method": "cli",
      "pricing": {
        "model": "free",
        "free_tier": true,
        "usage_notes": "Free tool, pay for underlying model API"
      },
      "supported_providers": ["openai", "anthropic", "openrouter", "ollama"],
      "platform_support": ["macos", "linux", "windows"],
      "open_source": true,
      "cli_binary": "aider",
      "version_command": ["--version"],
      "version_regex": "aider v([0-9]+\\.[0-9]+\\.[0-9]+)",
      "config_files": ["~/.aider.conf.yml", ".aider.conf.yml"],
      "homepage": "https://aider.chat",
      "docs": "https://aider.chat/docs"
    },
    "cursor": {
      "name": "Cursor",
      "repo": "getcursor/cursor",
      "categories": ["ide"],
      "installation_method": "ide",
      "pricing": {
        "model": "hybrid",
        "subscription_price": 20.0,
        "subscription_period": "monthly",
        "free_tier": true,
        "usage_notes": "Free tier with limits, Pro $20/mo"
      },
      "supported_providers": ["openai", "anthropic", "google"],
      "platform_support": ["macos", "linux", "windows"],
      "open_source": false,
      "cli_binary": "cursor",
      "version_command": ["--version"],
      "version_regex": "([0-9]+\\.[0-9]+\\.[0-9]+)",
      "config_files": [],
      "homepage": "https://cursor.com",
      "docs": "https://docs.cursor.com"
    },
    "windsurf": {
      "name": "Windsurf",
      "repo": "codeium/windsurf",
      "categories": ["ide"],
      "installation_method": "ide",
      "pricing": {
        "model": "hybrid",
        "free_tier": true,
        "usage_notes": "Free tier available, Pro subscription for more features"
      },
      "supported_providers": ["openai", "anthropic", "google"],
      "platform_support": ["macos", "linux", "windows"],
      "open_source": false,
      "cli_binary": "windsurf",
      "version_command": ["--version"],
      "version_regex": "([0-9]+\\.[0-9]+\\.[0-9]+)",
      "config_files": [],
      "homepage": "https://windsurf.com",
      "docs": "https://docs.windsurf.com"
    },
    "goose": {
      "name": "Goose",
      "repo": "block/goose",
      "categories": ["cli"],
      "installation_method": "cli",
      "pricing": {
        "model": "free",
        "free_tier": true,
        "usage_notes": "Free tool, pay for underlying model API"
      },
      "supported_providers": ["openai", "anthropic", "openrouter"],
      "platform_support": ["macos", "linux", "windows"],
      "open_source": true,
      "cli_binary": "goose",
      "version_command": ["--version"],
      "version_regex": "([0-9]+\\.[0-9]+\\.[0-9]+)",
      "config_files": ["~/.config/goose/"],
      "homepage": "https://github.com/block/goose",
      "docs": "https://block.github.io/goose/"
    },
    "zed": {
      "name": "Zed",
      "repo": "zed-industries/zed",
      "categories": ["ide"],
      "installation_method": "ide",
      "pricing": {
        "model": "hybrid",
        "free_tier": true,
        "usage_notes": "Free editor, pay for AI features"
      },
      "supported_providers": ["openai", "anthropic", "ollama"],
      "platform_support": ["macos", "linux"],
      "open_source": true,
      "cli_binary": "zed",
      "version_command": ["--version"],
      "version_regex": "([0-9]+\\.[0-9]+\\.[0-9]+)",
      "config_files": ["~/.config/zed/"],
      "homepage": "https://zed.dev",
      "docs": "https://zed.dev/docs"
    }
  }
}
```

**Step 3: Commit**

```bash
git add data/agents.json
git commit -m "feat: add initial agents.json catalog"
```

---

### Task 1.4: Add Agent Data Loading

**Files:**
- Create: `src/agents/loader.rs`
- Modify: `src/agents/mod.rs`

**Step 1: Create `src/agents/loader.rs`**

```rust
use anyhow::{Context, Result};
use std::path::Path;

use super::data::AgentsFile;

const EMBEDDED_AGENTS: &str = include_str!("../../data/agents.json");

pub fn load_agents() -> Result<AgentsFile> {
    serde_json::from_str(EMBEDDED_AGENTS).context("Failed to parse embedded agents.json")
}

pub fn load_agents_from_file(path: &Path) -> Result<AgentsFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read agents file: {}", path.display()))?;
    serde_json::from_str(&content).context("Failed to parse agents.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_embedded_agents() {
        let agents = load_agents().expect("Should load embedded agents");
        assert!(agents.schema_version >= 1);
        assert!(!agents.agents.is_empty());
        assert!(agents.agents.contains_key("claude-code"));
        assert!(agents.agents.contains_key("aider"));
    }
}
```

**Step 2: Update `src/agents/mod.rs`**

```rust
pub mod data;
pub mod loader;

pub use data::*;
pub use loader::*;
```

**Step 3: Run tests**

Run: `cargo test test_load_embedded_agents`
Expected: Test passes

**Step 4: Commit**

```bash
git add src/agents/loader.rs src/agents/mod.rs
git commit -m "feat: add agent data loader with embedded JSON"
```

---

### Task 1.5: Add User Config Support

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

**Step 1: Create `src/config.rs`**

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub config_version: u32,
    #[serde(default)]
    pub agents: AgentsConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AgentsConfig {
    #[serde(default)]
    pub tracked: HashSet<String>,
    #[serde(default)]
    pub excluded: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    #[serde(default = "default_github_ttl")]
    pub github_ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            github_ttl_seconds: default_github_ttl(),
        }
    }
}

fn default_github_ttl() -> u64 {
    3600
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DisplayConfig {
    #[serde(default)]
    pub default_tab: Option<String>,
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("models").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Ok(Self::default()),
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;

        toml::from_str(&content).context("Failed to parse config.toml")
    }

    pub fn save(&self) -> Result<()> {
        let path = match Self::config_path() {
            Some(p) => p,
            None => anyhow::bail!("Could not determine config directory"),
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;

        Ok(())
    }

    pub fn is_tracked(&self, agent_id: &str) -> bool {
        if self.agents.excluded.contains(agent_id) {
            return false;
        }
        self.agents.tracked.is_empty() || self.agents.tracked.contains(agent_id)
    }

    pub fn set_tracked(&mut self, agent_id: &str, tracked: bool) {
        if tracked {
            self.agents.tracked.insert(agent_id.to_string());
            self.agents.excluded.remove(agent_id);
        } else {
            self.agents.tracked.remove(agent_id);
            self.agents.excluded.insert(agent_id.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.cache.github_ttl_seconds, 3600);
        assert!(config.agents.tracked.is_empty());
    }

    #[test]
    fn test_is_tracked_empty() {
        let config = Config::default();
        // Empty tracked list means all are tracked by default
        assert!(config.is_tracked("claude-code"));
    }

    #[test]
    fn test_is_tracked_excluded() {
        let mut config = Config::default();
        config.agents.excluded.insert("aider".to_string());
        assert!(!config.is_tracked("aider"));
        assert!(config.is_tracked("claude-code"));
    }
}
```

**Step 2: Add module to `src/main.rs`**

Add after existing `mod` declarations:

```rust
mod config;
```

**Step 3: Run tests**

Run: `cargo test config::tests`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add user config support with persistence"
```

---

## Phase 2: CLI Detection

### Task 2.1: Add CLI Detection Module

**Files:**
- Create: `src/agents/detect.rs`
- Modify: `src/agents/mod.rs`

**Step 1: Create `src/agents/detect.rs`**

```rust
use std::env;
use std::path::PathBuf;
use std::process::Command;

use super::data::{Agent, InstalledInfo};

pub fn detect_installed(agent: &Agent) -> InstalledInfo {
    let binary = match &agent.cli_binary {
        Some(b) => b,
        None => return InstalledInfo::default(),
    };

    // Only detect CLI tools
    if agent.installation_method.as_deref() != Some("cli") {
        return InstalledInfo::default();
    }

    // Try to find the binary
    let path = find_binary(binary);
    if path.is_none() {
        return InstalledInfo::default();
    }

    // Try to get version
    let version = get_version(binary, &agent.version_command, agent.version_regex.as_deref());

    InstalledInfo {
        version,
        path: path.map(|p| p.to_string_lossy().to_string()),
    }
}

fn find_binary(name: &str) -> Option<PathBuf> {
    // First try which/where
    if let Some(path) = which_binary(name) {
        return Some(path);
    }

    // Check common locations
    let home = env::var("HOME").ok()?;
    let common_paths = [
        format!("/opt/homebrew/bin/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("{}/.local/bin/{}", home, name),
        format!("{}/.cargo/bin/{}", home, name),
        format!("{}/.npm-global/bin/{}", home, name),
        format!("/usr/bin/{}", name),
    ];

    for path_str in common_paths {
        let path = PathBuf::from(&path_str);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn which_binary(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            return Some(PathBuf::from(path_str));
        }
    }

    None
}

fn get_version(binary: &str, version_cmd: &[String], version_regex: Option<&str>) -> Option<String> {
    if version_cmd.is_empty() {
        return None;
    }

    let output = Command::new(binary)
        .args(version_cmd)
        .output()
        .ok()?;

    let output_str = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        // Some tools output version to stderr
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    extract_version(&output_str, version_regex)
}

fn extract_version(output: &str, regex_pattern: Option<&str>) -> Option<String> {
    let pattern = regex_pattern.unwrap_or(r"([0-9]+\.[0-9]+\.[0-9]+)");

    // Simple regex-like extraction (avoid regex crate dependency)
    // Look for version pattern in output
    for line in output.lines() {
        if let Some(version) = extract_semver_from_line(line, pattern) {
            return Some(version);
        }
    }
    None
}

fn extract_semver_from_line(line: &str, _pattern: &str) -> Option<String> {
    // Simple extraction: find X.Y.Z pattern
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0;

            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                if chars[i] == '.' {
                    dots += 1;
                }
                i += 1;
            }

            if dots >= 2 {
                let version: String = chars[start..i].iter().collect();
                // Validate it looks like semver
                let parts: Vec<&str> = version.split('.').collect();
                if parts.len() >= 3 && parts.iter().all(|p| !p.is_empty()) {
                    return Some(version.trim_end_matches('.').to_string());
                }
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_semver() {
        assert_eq!(
            extract_version("claude-code v1.0.30", None),
            Some("1.0.30".to_string())
        );
        assert_eq!(
            extract_version("aider v0.82.1", None),
            Some("0.82.1".to_string())
        );
        assert_eq!(
            extract_version("Version: 2.3.4-beta", None),
            Some("2.3.4".to_string())
        );
    }

    #[test]
    fn test_no_version() {
        assert_eq!(extract_version("no version here", None), None);
        assert_eq!(extract_version("1.2", None), None); // Not enough parts
    }
}
```

**Step 2: Update `src/agents/mod.rs`**

```rust
pub mod data;
pub mod detect;
pub mod loader;

pub use data::*;
pub use detect::*;
pub use loader::*;
```

**Step 3: Run tests**

Run: `cargo test agents::detect::tests`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/agents/detect.rs src/agents/mod.rs
git commit -m "feat: add CLI detection for installed agents"
```

---

## Phase 3: TUI - Tab System

### Task 3.1: Add Tab Enum and State

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add Tab enum after existing enums**

Add after `pub enum SortOrder`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Models,
    Agents,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Models => Tab::Agents,
            Tab::Agents => Tab::Models,
        }
    }

    pub fn prev(self) -> Self {
        self.next() // Only two tabs, so prev == next
    }
}
```

**Step 2: Add Tab field to App struct**

In the `App` struct, add after `pub help_scroll: u16,`:

```rust
    pub current_tab: Tab,
```

**Step 3: Initialize tab in App::new**

In `App::new`, add to the struct initialization (after `help_scroll: 0,`):

```rust
            current_tab: Tab::default(),
```

**Step 4: Add tab messages to Message enum**

Add to the `Message` enum:

```rust
    NextTab,
    PrevTab,
```

**Step 5: Handle tab messages in App::update**

Add these cases in the `match msg` block in `update()`:

```rust
            Message::NextTab => {
                self.current_tab = self.current_tab.next();
            }
            Message::PrevTab => {
                self.current_tab = self.current_tab.prev();
            }
```

**Step 6: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 7: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat: add tab state to TUI app"
```

---

### Task 3.2: Add Tab Keybindings

**Files:**
- Modify: `src/tui/event.rs`

**Step 1: Add tab navigation keybindings**

In `handle_normal_mode`, add these cases (before the `_ => None` catch-all):

```rust
        // Tab navigation
        KeyCode::Char('[') => Some(Message::PrevTab),
        KeyCode::Char(']') => Some(Message::NextTab),
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/tui/event.rs
git commit -m "feat: add tab navigation keybindings ([ and ])"
```

---

### Task 3.3: Update Header to Show Tabs

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Update draw_header function**

Replace the `draw_header` function with:

```rust
fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let models_style = if app.current_tab == super::app::Tab::Models {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let agents_style = if app.current_tab == super::app::Tab::Agents {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let header = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled("Models", models_style),
        Span::raw(" | "),
        Span::styled("Agents", agents_style),
        Span::styled("  [/] switch tabs", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(header, area);
}
```

**Step 2: Add import for Tab**

At the top of the file, update the import from app:

```rust
use super::app::{App, Filters, Focus, Mode, SortOrder, Tab};
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Test manually**

Run: `cargo run -- tui`
Expected: Header shows "Models | Agents" with Models highlighted. Press `]` to switch to Agents tab.

**Step 5: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: add tab indicator to TUI header"
```

---

### Task 3.4: Conditional Rendering Based on Tab

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Update draw function to check tab**

Replace the `draw` function with:

```rust
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(14), // Detail panel (expanded)
            Constraint::Length(1),  // Footer/search
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);

    match app.current_tab {
        Tab::Models => {
            draw_main(f, chunks[1], app);
            draw_details_row(f, chunks[2], app);
        }
        Tab::Agents => {
            draw_agents_placeholder(f, chunks[1]);
            draw_agents_detail_placeholder(f, chunks[2]);
        }
    }

    draw_footer(f, chunks[3], app);

    // Draw help popup on top if visible
    if app.show_help {
        draw_help_popup(f, app.help_scroll);
    }
}
```

**Step 2: Add placeholder functions for Agents tab**

Add these functions (we'll implement them fully later):

```rust
fn draw_agents_placeholder(f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    let categories = Paragraph::new("Agents tab coming soon...")
        .block(Block::default().borders(Borders::ALL).title(" Categories "));
    f.render_widget(categories, chunks[0]);

    let agents = Paragraph::new("Select an agent to view details")
        .block(Block::default().borders(Borders::ALL).title(" Agents "));
    f.render_widget(agents, chunks[1]);
}

fn draw_agents_detail_placeholder(f: &mut Frame, area: Rect) {
    let detail = Paragraph::new("Agent details will appear here")
        .block(Block::default().borders(Borders::ALL).title(" Details "));
    f.render_widget(detail, area);
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Test manually**

Run: `cargo run -- tui`
Expected:
- Models tab shows normal content
- Press `]` to see Agents placeholder
- Press `[` to go back to Models

**Step 5: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: add conditional rendering for Models/Agents tabs"
```

---

## Phase 4: Agents Tab Implementation

### Task 4.1: Create Agents App State

**Files:**
- Create: `src/tui/agents_app.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Create `src/tui/agents_app.rs`**

```rust
use ratatui::widgets::ListState;

use crate::agents::{Agent, AgentEntry, AgentsFile, GitHubData, InstalledInfo, detect_installed};
use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentCategory {
    #[default]
    All,
    Installed,
    Cli,
    Ide,
    OpenSource,
}

impl AgentCategory {
    pub fn label(&self) -> &'static str {
        match self {
            AgentCategory::All => "All",
            AgentCategory::Installed => "Installed",
            AgentCategory::Cli => "CLI Tools",
            AgentCategory::Ide => "IDEs",
            AgentCategory::OpenSource => "Open Source",
        }
    }

    pub fn variants() -> &'static [AgentCategory] {
        &[
            AgentCategory::All,
            AgentCategory::Installed,
            AgentCategory::Cli,
            AgentCategory::Ide,
            AgentCategory::OpenSource,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentFocus {
    #[default]
    Categories,
    Agents,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AgentFilters {
    pub installed_only: bool,
    pub cli_only: bool,
    pub open_source_only: bool,
}

pub struct AgentsApp {
    pub entries: Vec<AgentEntry>,
    pub filtered_entries: Vec<usize>, // indices into entries
    pub selected_category: usize,
    pub selected_agent: usize,
    pub category_list_state: ListState,
    pub agent_list_state: ListState,
    pub focus: AgentFocus,
    pub filters: AgentFilters,
    pub search_query: String,
}

impl AgentsApp {
    pub fn new(agents_file: &AgentsFile, config: &Config) -> Self {
        let mut entries: Vec<AgentEntry> = agents_file
            .agents
            .iter()
            .map(|(id, agent)| {
                let installed = detect_installed(agent);
                AgentEntry {
                    id: id.clone(),
                    agent: agent.clone(),
                    github: GitHubData::default(),
                    installed,
                    tracked: config.is_tracked(id),
                }
            })
            .collect();

        // Sort by name
        entries.sort_by(|a, b| a.agent.name.cmp(&b.agent.name));

        let mut category_list_state = ListState::default();
        category_list_state.select(Some(0));
        let mut agent_list_state = ListState::default();
        agent_list_state.select(Some(0));

        let mut app = Self {
            entries,
            filtered_entries: Vec::new(),
            selected_category: 0,
            selected_agent: 0,
            category_list_state,
            agent_list_state,
            focus: AgentFocus::default(),
            filters: AgentFilters::default(),
            search_query: String::new(),
        };

        app.update_filtered();
        app
    }

    pub fn update_filtered(&mut self) {
        let category = AgentCategory::variants()[self.selected_category];
        let query_lower = self.search_query.to_lowercase();

        self.filtered_entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                // Category filter
                let category_match = match category {
                    AgentCategory::All => true,
                    AgentCategory::Installed => entry.installed.version.is_some(),
                    AgentCategory::Cli => entry.agent.categories.contains(&"cli".to_string()),
                    AgentCategory::Ide => entry.agent.categories.contains(&"ide".to_string()),
                    AgentCategory::OpenSource => entry.agent.open_source,
                };

                // Additional filters
                let filter_match = (!self.filters.installed_only || entry.installed.version.is_some())
                    && (!self.filters.cli_only || entry.agent.categories.contains(&"cli".to_string()))
                    && (!self.filters.open_source_only || entry.agent.open_source);

                // Search filter
                let search_match = query_lower.is_empty()
                    || entry.agent.name.to_lowercase().contains(&query_lower)
                    || entry.id.to_lowercase().contains(&query_lower);

                category_match && filter_match && search_match
            })
            .map(|(i, _)| i)
            .collect();

        // Reset selection if out of bounds
        if self.selected_agent >= self.filtered_entries.len() {
            self.selected_agent = 0;
        }
        self.agent_list_state.select(Some(self.selected_agent));
    }

    pub fn current_entry(&self) -> Option<&AgentEntry> {
        self.filtered_entries
            .get(self.selected_agent)
            .and_then(|&i| self.entries.get(i))
    }

    pub fn category_count(&self, category: AgentCategory) -> usize {
        self.entries
            .iter()
            .filter(|e| match category {
                AgentCategory::All => true,
                AgentCategory::Installed => e.installed.version.is_some(),
                AgentCategory::Cli => e.agent.categories.contains(&"cli".to_string()),
                AgentCategory::Ide => e.agent.categories.contains(&"ide".to_string()),
                AgentCategory::OpenSource => e.agent.open_source,
            })
            .count()
    }

    pub fn next_category(&mut self) {
        let max = AgentCategory::variants().len() - 1;
        if self.selected_category < max {
            self.selected_category += 1;
            self.category_list_state.select(Some(self.selected_category));
            self.selected_agent = 0;
            self.update_filtered();
        }
    }

    pub fn prev_category(&mut self) {
        if self.selected_category > 0 {
            self.selected_category -= 1;
            self.category_list_state.select(Some(self.selected_category));
            self.selected_agent = 0;
            self.update_filtered();
        }
    }

    pub fn next_agent(&mut self) {
        if self.selected_agent < self.filtered_entries.len().saturating_sub(1) {
            self.selected_agent += 1;
            self.agent_list_state.select(Some(self.selected_agent));
        }
    }

    pub fn prev_agent(&mut self) {
        if self.selected_agent > 0 {
            self.selected_agent -= 1;
            self.agent_list_state.select(Some(self.selected_agent));
        }
    }

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            AgentFocus::Categories => AgentFocus::Agents,
            AgentFocus::Agents => AgentFocus::Categories,
        };
    }

    pub fn toggle_installed_filter(&mut self) {
        self.filters.installed_only = !self.filters.installed_only;
        self.selected_agent = 0;
        self.update_filtered();
    }

    pub fn toggle_cli_filter(&mut self) {
        self.filters.cli_only = !self.filters.cli_only;
        self.selected_agent = 0;
        self.update_filtered();
    }

    pub fn toggle_open_source_filter(&mut self) {
        self.filters.open_source_only = !self.filters.open_source_only;
        self.selected_agent = 0;
        self.update_filtered();
    }
}
```

**Step 2: Update `src/tui/mod.rs`**

```rust
pub mod agents_app;
pub mod app;
pub mod event;
pub mod ui;
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/tui/agents_app.rs src/tui/mod.rs
git commit -m "feat: add agents app state management"
```

---

### Task 4.2: Integrate AgentsApp into Main App

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Add AgentsApp to main App struct**

In `src/tui/app.rs`, add import at top:

```rust
use super::agents_app::AgentsApp;
use crate::agents::{load_agents, AgentsFile};
use crate::config::Config;
```

Add field to App struct (after `current_tab: Tab,`):

```rust
    pub agents_app: Option<AgentsApp>,
```

**Step 2: Update App::new to accept agents and config**

Change the `App::new` signature and implementation:

```rust
    pub fn new(providers_map: ProvidersMap, agents_file: Option<&AgentsFile>, config: Option<&Config>) -> Self {
        let mut providers: Vec<(String, Provider)> = providers_map.into_iter().collect();
        providers.sort_by(|a, b| a.0.cmp(&b.0));

        let mut provider_list_state = ListState::default();
        provider_list_state.select(Some(0));
        let mut model_list_state = ListState::default();
        model_list_state.select(Some(1)); // +1 for header row

        let agents_app = match (agents_file, config) {
            (Some(af), Some(cfg)) => Some(AgentsApp::new(af, cfg)),
            (Some(af), None) => Some(AgentsApp::new(af, &Config::default())),
            _ => None,
        };

        let mut app = Self {
            providers,
            selected_provider: 0, // Start with "All"
            selected_model: 0,
            provider_list_state,
            model_list_state,
            focus: Focus::Providers,
            mode: Mode::Normal,
            sort_order: SortOrder::Default,
            filters: Filters::default(),
            search_query: String::new(),
            status_message: None,
            show_help: false,
            help_scroll: 0,
            current_tab: Tab::default(),
            agents_app,
            filtered_models: Vec::new(),
        };

        app.update_filtered_models();
        app
    }
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compilation errors about App::new call sites (we'll fix in next step)

**Step 4: Commit partial progress**

```bash
git add src/tui/app.rs
git commit -m "wip: integrate AgentsApp into main App struct"
```

---

### Task 4.3: Update TUI Entry Point

**Files:**
- Modify: `src/tui/mod.rs`

**Step 1: Update run function**

Replace the `run` function in `src/tui/mod.rs`:

```rust
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub mod agents_app;
pub mod app;
pub mod event;
pub mod ui;

use crate::agents::load_agents;
use crate::api::fetch_providers;
use crate::config::Config;

pub fn run() -> Result<()> {
    // Load data
    let providers = fetch_providers()?;
    let agents_file = load_agents().ok();
    let config = Config::load().ok();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = app::App::new(providers, agents_file.as_ref(), config.as_ref());

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Some(msg) = event::handle_events(app)? {
            // Handle clipboard operations
            match &msg {
                app::Message::CopyFull => {
                    if let Some(text) = app.get_copy_full() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                        }
                    }
                }
                app::Message::CopyModelId => {
                    if let Some(text) = app.get_copy_model_id() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                        }
                    }
                }
                app::Message::CopyProviderDoc => {
                    if let Some(text) = app.get_provider_doc() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                        }
                    }
                }
                app::Message::CopyProviderApi => {
                    if let Some(text) = app.get_provider_api() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                        }
                    }
                }
                app::Message::OpenProviderDoc => {
                    if let Some(url) = app.get_provider_doc() {
                        let _ = open::that(&url);
                        app.set_status(format!("Opened: {}", url));
                    }
                }
                _ => {}
            }

            if !app.update(msg) {
                return Ok(());
            }
        }

        // Clear status after a short display
        app.clear_status();
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Run the app**

Run: `cargo run -- tui`
Expected: App starts, can switch between Models and Agents tabs

**Step 4: Commit**

```bash
git add src/tui/mod.rs
git commit -m "feat: update TUI entry point to load agents and config"
```

---

### Task 4.4: Implement Agents Tab UI

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Replace placeholder functions with real implementation**

Replace `draw_agents_placeholder` and `draw_agents_detail_placeholder` with:

```rust
fn draw_agents_main(f: &mut Frame, area: Rect, app: &mut App) {
    let agents_app = match &mut app.agents_app {
        Some(a) => a,
        None => {
            let msg = Paragraph::new("Failed to load agents data")
                .block(Block::default().borders(Borders::ALL).title(" Agents "));
            f.render_widget(msg, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    draw_agent_categories(f, chunks[0], agents_app);
    draw_agent_list(f, chunks[1], agents_app);
}

fn draw_agent_categories(f: &mut Frame, area: Rect, app: &mut super::agents_app::AgentsApp) {
    use super::agents_app::{AgentCategory, AgentFocus};

    let is_focused = app.focus == AgentFocus::Categories;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = AgentCategory::variants()
        .iter()
        .map(|cat| {
            let count = app.category_count(*cat);
            let text = format!("{} ({})", cat.label(), count);
            ListItem::new(text)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Categories "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.category_list_state);
}

fn draw_agent_list(f: &mut Frame, area: Rect, app: &mut super::agents_app::AgentsApp) {
    use super::agents_app::AgentFocus;

    let is_focused = app.focus == AgentFocus::Agents;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut items: Vec<ListItem> = Vec::new();

    // Header row
    let header = format!(
        "{:<25} {:>10} {:>10} {:>8}",
        "Agent", "Installed", "Latest", "Status"
    );
    items.push(
        ListItem::new(header).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        ),
    );

    // Agent rows
    for &idx in &app.filtered_entries {
        if let Some(entry) = app.entries.get(idx) {
            let installed = entry
                .installed
                .version
                .as_deref()
                .unwrap_or("-");
            let latest = entry
                .github
                .latest_version
                .as_deref()
                .unwrap_or("-");
            let status = entry.status_str();

            let row = format!(
                "{:<25} {:>10} {:>10} {:>8}",
                truncate(&entry.agent.name, 25),
                truncate(installed, 10),
                truncate(latest, 10),
                status
            );
            items.push(ListItem::new(row));
        }
    }

    let title = format!(" Agents ({}) ", app.filtered_entries.len());

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    // Offset by 1 for header row
    let mut state = app.agent_list_state.clone();
    if let Some(selected) = state.selected() {
        state.select(Some(selected + 1));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_agent_detail(f: &mut Frame, area: Rect, app: &App) {
    let agents_app = match &app.agents_app {
        Some(a) => a,
        None => {
            let msg = Paragraph::new("No agent data")
                .block(Block::default().borders(Borders::ALL).title(" Details "));
            f.render_widget(msg, area);
            return;
        }
    };

    let lines: Vec<Line> = if let Some(entry) = agents_app.current_entry() {
        let installed = entry
            .installed
            .version
            .as_deref()
            .unwrap_or("Not installed");
        let latest = entry
            .github
            .latest_version
            .as_deref()
            .unwrap_or("Unknown");

        let status = if entry.installed.version.is_none() {
            "Not Installed"
        } else if entry.update_available() {
            "UPDATE AVAILABLE"
        } else {
            "Up to date"
        };

        let pricing = entry
            .agent
            .pricing
            .as_ref()
            .map(|p| {
                if p.free_tier {
                    format!("{} (free tier)", p.model)
                } else {
                    p.model.clone()
                }
            })
            .unwrap_or_else(|| "-".to_string());

        let providers = if entry.agent.supported_providers.is_empty() {
            "-".to_string()
        } else {
            entry.agent.supported_providers.join(", ")
        };

        let categories = if entry.agent.categories.is_empty() {
            "-".to_string()
        } else {
            entry.agent.categories.join(", ")
        };

        vec![
            Line::from(vec![
                Span::styled(
                    &entry.agent.name,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    &entry.agent.repo,
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Installed: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<12}", installed)),
                Span::styled("Latest: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<12}", latest)),
                Span::styled(
                    status,
                    if entry.update_available() {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Pricing: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<20}", pricing)),
                Span::styled("Providers: ", Style::default().fg(Color::DarkGray)),
                Span::raw(providers),
            ]),
            Line::from(vec![
                Span::styled("Categories: ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{:<18}", categories)),
                Span::styled("Open Source: ", Style::default().fg(Color::DarkGray)),
                Span::raw(if entry.agent.open_source { "Yes" } else { "No" }),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("o ", Style::default().fg(Color::Yellow)),
                Span::raw("open docs  "),
                Span::styled("r ", Style::default().fg(Color::Yellow)),
                Span::raw("open repo  "),
                Span::styled("c ", Style::default().fg(Color::Yellow)),
                Span::raw("copy name"),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled(
            "No agent selected",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}
```

**Step 2: Update draw function to use new functions**

Update the `Tab::Agents` match arm in `draw`:

```rust
        Tab::Agents => {
            draw_agents_main(f, chunks[1], app);
            draw_agent_detail(f, chunks[2], app);
        }
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Test manually**

Run: `cargo run -- tui`
Expected: Agents tab shows categories, agent list, and details

**Step 5: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: implement agents tab UI with categories and details"
```

---

### Task 4.5: Add Agents Tab Event Handling

**Files:**
- Modify: `src/tui/event.rs`
- Modify: `src/tui/app.rs`

**Step 1: Add agents-specific messages to Message enum**

In `src/tui/app.rs`, add to the `Message` enum:

```rust
    // Agents tab messages
    NextCategory,
    PrevCategory,
    NextAgent,
    PrevAgent,
    SwitchAgentFocus,
    ToggleInstalledFilter,
    ToggleCliFilter,
    ToggleOpenSourceFilter,
    OpenAgentRepo,
    OpenAgentDocs,
    CopyAgentName,
```

**Step 2: Handle agents messages in App::update**

Add these cases in the `match msg` block:

```rust
            Message::NextCategory => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.next_category();
                }
            }
            Message::PrevCategory => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.prev_category();
                }
            }
            Message::NextAgent => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.next_agent();
                }
            }
            Message::PrevAgent => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.prev_agent();
                }
            }
            Message::SwitchAgentFocus => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.switch_focus();
                }
            }
            Message::ToggleInstalledFilter => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.toggle_installed_filter();
                }
            }
            Message::ToggleCliFilter => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.toggle_cli_filter();
                }
            }
            Message::ToggleOpenSourceFilter => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.toggle_open_source_filter();
                }
            }
            Message::OpenAgentRepo | Message::OpenAgentDocs | Message::CopyAgentName => {
                // Handled in main loop
            }
```

**Step 3: Update event handling to be tab-aware**

In `src/tui/event.rs`, update `handle_normal_mode` to check the current tab:

```rust
fn handle_normal_mode(app: &App, code: KeyCode, modifiers: KeyModifiers) -> Option<Message> {
    // Global keys (work on any tab)
    match code {
        KeyCode::Char('q') => return Some(Message::Quit),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return Some(Message::Quit),
        KeyCode::Char('[') => return Some(Message::PrevTab),
        KeyCode::Char(']') => return Some(Message::NextTab),
        KeyCode::Char('?') => return Some(Message::ToggleHelp),
        _ => {}
    }

    // Tab-specific keys
    match app.current_tab {
        super::app::Tab::Models => handle_models_keys(app, code, modifiers),
        super::app::Tab::Agents => handle_agents_keys(app, code, modifiers),
    }
}

fn handle_models_keys(app: &App, code: KeyCode, modifiers: KeyModifiers) -> Option<Message> {
    match code {
        // Copy shortcuts
        KeyCode::Char('c') => Some(Message::CopyFull),
        KeyCode::Char('C') => Some(Message::CopyModelId),
        KeyCode::Char('D') => Some(Message::CopyProviderDoc),
        KeyCode::Char('A') => Some(Message::CopyProviderApi),
        KeyCode::Char('o') => Some(Message::OpenProviderDoc),

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::Providers => Some(Message::NextProvider),
            Focus::Models => Some(Message::NextModel),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::Providers => Some(Message::PrevProvider),
            Focus::Models => Some(Message::PrevModel),
        },
        KeyCode::Char('g') => match app.focus {
            Focus::Providers => Some(Message::SelectFirstProvider),
            Focus::Models => Some(Message::SelectFirstModel),
        },
        KeyCode::Char('G') => match app.focus {
            Focus::Providers => Some(Message::SelectLastProvider),
            Focus::Models => Some(Message::SelectLastModel),
        },
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => match app.focus {
            Focus::Providers => Some(Message::PageDownProvider),
            Focus::Models => Some(Message::PageDownModel),
        },
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => match app.focus {
            Focus::Providers => Some(Message::PageUpProvider),
            Focus::Models => Some(Message::PageUpModel),
        },
        KeyCode::PageDown => match app.focus {
            Focus::Providers => Some(Message::PageDownProvider),
            Focus::Models => Some(Message::PageDownModel),
        },
        KeyCode::PageUp => match app.focus {
            Focus::Providers => Some(Message::PageUpProvider),
            Focus::Models => Some(Message::PageUpModel),
        },
        KeyCode::Char('h') | KeyCode::Left => Some(Message::SwitchFocus),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::SwitchFocus),
        KeyCode::Tab | KeyCode::BackTab => Some(Message::SwitchFocus),

        // Search
        KeyCode::Char('/') => Some(Message::EnterSearch),
        KeyCode::Esc => Some(Message::ClearSearch),

        // Sort
        KeyCode::Char('s') => Some(Message::CycleSort),

        // Filters
        KeyCode::Char('1') => Some(Message::ToggleReasoning),
        KeyCode::Char('2') => Some(Message::ToggleTools),
        KeyCode::Char('3') => Some(Message::ToggleOpenWeights),

        _ => None,
    }
}

fn handle_agents_keys(app: &App, code: KeyCode, _modifiers: KeyModifiers) -> Option<Message> {
    use super::agents_app::AgentFocus;

    let focus = app
        .agents_app
        .as_ref()
        .map(|a| a.focus)
        .unwrap_or(AgentFocus::Categories);

    match code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => match focus {
            AgentFocus::Categories => Some(Message::NextCategory),
            AgentFocus::Agents => Some(Message::NextAgent),
        },
        KeyCode::Char('k') | KeyCode::Up => match focus {
            AgentFocus::Categories => Some(Message::PrevCategory),
            AgentFocus::Agents => Some(Message::PrevAgent),
        },
        KeyCode::Char('h') | KeyCode::Left => Some(Message::SwitchAgentFocus),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::SwitchAgentFocus),
        KeyCode::Tab | KeyCode::BackTab => Some(Message::SwitchAgentFocus),

        // Actions
        KeyCode::Char('o') => Some(Message::OpenAgentDocs),
        KeyCode::Char('r') => Some(Message::OpenAgentRepo),
        KeyCode::Char('c') => Some(Message::CopyAgentName),

        // Filters
        KeyCode::Char('1') => Some(Message::ToggleInstalledFilter),
        KeyCode::Char('2') => Some(Message::ToggleCliFilter),
        KeyCode::Char('3') => Some(Message::ToggleOpenSourceFilter),

        _ => None,
    }
}
```

**Step 4: Handle agents clipboard/open in main loop**

In `src/tui/mod.rs`, add handling for agent actions in `run_app`:

```rust
                app::Message::OpenAgentDocs => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            if let Some(ref url) = entry.agent.docs {
                                let _ = open::that(url);
                                app.set_status(format!("Opened: {}", url));
                            } else if let Some(ref url) = entry.agent.homepage {
                                let _ = open::that(url);
                                app.set_status(format!("Opened: {}", url));
                            }
                        }
                    }
                }
                app::Message::OpenAgentRepo => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            let url = format!("https://github.com/{}", entry.agent.repo);
                            let _ = open::that(&url);
                            app.set_status(format!("Opened: {}", url));
                        }
                    }
                }
                app::Message::CopyAgentName => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                let _ = clipboard.set_text(&entry.agent.name);
                                app.set_status(format!("Copied: {}", entry.agent.name));
                            }
                        }
                    }
                }
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Test manually**

Run: `cargo run -- tui`
Expected:
- Navigate agents with j/k
- Switch focus with Tab/h/l
- Open repo with r
- Filters with 1/2/3

**Step 7: Commit**

```bash
git add src/tui/event.rs src/tui/app.rs src/tui/mod.rs
git commit -m "feat: add agents tab event handling and navigation"
```

---

## Phase 5: Polish & gh-aw

### Task 5.1: Update Help Popup

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Update help text to include Agents tab info**

In `draw_help_popup`, add after the existing help text (before the "Other" section):

```rust
        Line::from(""),
        Line::from(Span::styled(
            "Tabs",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  [             ", Style::default().fg(Color::Yellow)),
            Span::raw("Previous tab"),
        ]),
        Line::from(vec![
            Span::styled("  ]             ", Style::default().fg(Color::Yellow)),
            Span::raw("Next tab"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Agents Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  r             ", Style::default().fg(Color::Yellow)),
            Span::raw("Open GitHub repo"),
        ]),
        Line::from(vec![
            Span::styled("  1             ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle installed filter"),
        ]),
        Line::from(vec![
            Span::styled("  2             ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle CLI filter"),
        ]),
        Line::from(vec![
            Span::styled("  3             ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle open source filter"),
        ]),
```

Also update the `HELP_LINES` constant to account for new lines.

**Step 2: Verify compilation and test**

Run: `cargo run -- tui`
Press `?` to verify help shows new content.

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: update help popup with Agents tab keybindings"
```

---

### Task 5.2: Create gh-aw Workflow

**Files:**
- Create: `.github/workflows/update-agents.md`

**Step 1: Create workflow file**

```bash
mkdir -p .github/workflows
```

**Step 2: Write `.github/workflows/update-agents.md`**

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

Read the coding agents comparison page from artificialanalysis.ai and update our data file.

## Instructions

1. Fetch the page at https://artificialanalysis.ai/insights/coding-agents-comparison
2. Extract the comparison table data for each coding agent/assistant:
   - Name
   - Category (CLI, IDE, Extension, Cloud)
   - Pricing model (free, subscription, usage-based, hybrid)
   - Supported model providers
   - Open source status
3. Read the existing `data/agents.json` file
4. For each agent found on the page:
   - If it exists in our file, update the scraped fields (pricing, category, providers)
   - If it's new, add a skeleton entry (we'll fill in repo/version details manually)
   - Preserve fields that aren't on the page (repo, cli_binary, version_command, etc.)
5. Do NOT remove agents that exist in our file but aren't on the page (they may be user additions)
6. If any changes were made, create a PR with:
   - Title: "chore: update agents data from artificialanalysis.ai"
   - Body: Summary of changes (agents added, agents updated)

## Important

- Keep the schema_version unchanged
- Update last_scraped to current timestamp
- Set scrape_source to "artificialanalysis.ai"
- Preserve all existing repo URLs and version detection settings
```

**Step 3: Commit**

```bash
git add .github/workflows/update-agents.md
git commit -m "feat: add gh-aw workflow for weekly agents data updates"
```

---

### Task 5.3: Final Testing & Cleanup

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Test full workflow manually**

Run: `cargo run -- tui`
- Verify Models tab works
- Switch to Agents tab with `]`
- Navigate categories and agents
- Test filters 1/2/3
- Open repo with `r`
- Switch back to Models with `[`
- Verify help popup (`?`) shows all keybindings

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete agents tab implementation"
```

---

## Summary

This plan implements the Agents tab in 5 phases:

1. **Core Infrastructure** - Dependencies, data structures, config
2. **CLI Detection** - Find and version-check installed tools
3. **TUI Tab System** - Tab state, switching, conditional rendering
4. **Agents Tab UI** - Categories, list, details, event handling
5. **Polish** - Help updates, gh-aw workflow

Each task is a small, testable unit with exact file paths, code, and commands.
