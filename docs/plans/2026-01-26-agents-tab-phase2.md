# Agents Tab Phase 2: Live Data & Picker

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add GitHub API integration for live data (stars, versions, changelogs), enhance the detail pane, add the agent picker modal, and implement copy update command.

**Architecture:** Use `gh api` CLI for GitHub data (avoids auth complexity), cache responses with 1hr TTL in memory. Picker modal overlays the main UI. Update commands are generated from agent metadata.

**Tech Stack:** Rust, ratatui, std::process::Command (for gh), serde_json

**Context:** This plan addresses features from the original design that were not included in Phase 1's implementation plan:
- Design Phase 1 specified "GitHub API client with caching" - was omitted
- Design Phase 4 specified "Add/remove picker modal" - was omitted entirely
- Design keybindings specified `u` for "Copy update command" - was omitted

---

## Phase 1: GitHub API Integration

### Task 1.1: Add GitHub Client Module

**Files:**
- Create: `src/agents/github.rs`
- Modify: `src/agents/mod.rs`

**Step 1: Create `src/agents/github.rs`**

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::data::GitHubData;

const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

#[derive(Debug, Deserialize)]
struct RepoResponse {
    stargazers_count: u64,
    open_issues_count: u64,
    license: Option<LicenseResponse>,
    pushed_at: String,
}

#[derive(Debug, Deserialize)]
struct LicenseResponse {
    spdx_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    published_at: String,
    body: Option<String>,
}

struct CacheEntry {
    data: GitHubData,
    fetched_at: Instant,
}

pub struct GitHubClient {
    cache: Mutex<HashMap<String, CacheEntry>>,
}

impl GitHubClient {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Fetch GitHub data for a repo, using cache if available
    pub fn fetch(&self, repo: &str) -> Result<GitHubData> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(repo) {
                if entry.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(entry.data.clone());
                }
            }
        }

        // Fetch fresh data
        let data = self.fetch_fresh(repo)?;

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(
                repo.to_string(),
                CacheEntry {
                    data: data.clone(),
                    fetched_at: Instant::now(),
                },
            );
        }

        Ok(data)
    }

    fn fetch_fresh(&self, repo: &str) -> Result<GitHubData> {
        let mut data = GitHubData::default();

        // Fetch repo metadata (stars, issues, license, last commit)
        if let Ok(repo_data) = self.fetch_repo(repo) {
            data.stars = Some(repo_data.stargazers_count);
            data.open_issues = Some(repo_data.open_issues_count);
            data.license = repo_data.license.and_then(|l| l.spdx_id);
            data.last_commit = Some(repo_data.pushed_at);
        }

        // Fetch latest release (version, date, changelog)
        if let Ok(release) = self.fetch_latest_release(repo) {
            // Strip 'v' prefix if present
            let version = release.tag_name.strip_prefix('v')
                .unwrap_or(&release.tag_name)
                .to_string();
            data.latest_version = Some(version);
            data.release_date = Some(release.published_at);
            data.changelog = release.body;
        }

        Ok(data)
    }

    fn fetch_repo(&self, repo: &str) -> Result<RepoResponse> {
        let output = Command::new("gh")
            .args(["api", &format!("repos/{}", repo)])
            .output()
            .context("Failed to execute gh api")?;

        if !output.status.success() {
            anyhow::bail!("gh api failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        serde_json::from_slice(&output.stdout).context("Failed to parse repo response")
    }

    fn fetch_latest_release(&self, repo: &str) -> Result<ReleaseResponse> {
        let output = Command::new("gh")
            .args(["api", &format!("repos/{}/releases/latest", repo)])
            .output()
            .context("Failed to execute gh api")?;

        if !output.status.success() {
            anyhow::bail!("gh api failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        serde_json::from_slice(&output.stdout).context("Failed to parse release response")
    }
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Format star count for display (e.g., 12345 -> "12.3k")
pub fn format_stars(stars: u64) -> String {
    if stars >= 1000 {
        format!("{:.1}k", stars as f64 / 1000.0)
    } else {
        stars.to_string()
    }
}

/// Format relative time (e.g., "2025-01-20T..." -> "6 days ago")
pub fn format_relative_time(iso_date: &str) -> String {
    // Parse ISO 8601 date and compute relative time
    // For simplicity, just show the date portion for now
    // A full implementation would use chrono crate
    iso_date.split('T').next().unwrap_or(iso_date).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_stars() {
        assert_eq!(format_stars(500), "500");
        assert_eq!(format_stars(1000), "1.0k");
        assert_eq!(format_stars(12345), "12.3k");
        assert_eq!(format_stars(123456), "123.5k");
    }
}
```

**Step 2: Update `src/agents/mod.rs`**

Add the new module:

```rust
pub mod data;
pub mod detect;
pub mod github;
pub mod loader;

pub use data::*;
pub use detect::*;
pub use github::*;
pub use loader::*;
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Run tests**

Run: `cargo test test_format_stars`
Expected: Test passes

**Step 5: Commit**

```bash
git add src/agents/github.rs src/agents/mod.rs
git commit -m "feat: add GitHub API client with caching"
```

---

### Task 1.2: Integrate GitHub Data into AgentsApp

**Files:**
- Modify: `src/tui/agents_app.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Update AgentsApp to fetch GitHub data**

In `src/tui/agents_app.rs`, add a method to refresh GitHub data:

```rust
use crate::agents::{GitHubClient, GitHubData};

impl AgentsApp {
    /// Refresh GitHub data for all agents (called on startup and periodically)
    pub fn refresh_github_data(&mut self, client: &GitHubClient) {
        for entry in &mut self.entries {
            if let Ok(data) = client.fetch(&entry.agent.repo) {
                entry.github = data;
            }
        }
    }

    /// Refresh GitHub data for a single agent
    pub fn refresh_agent_github(&mut self, client: &GitHubClient, agent_id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == agent_id) {
            if let Ok(data) = client.fetch(&entry.agent.repo) {
                entry.github = data;
            }
        }
    }
}
```

**Step 2: Store GitHubClient in App**

In `src/tui/app.rs`, add the client:

```rust
use crate::agents::GitHubClient;

pub struct App {
    // ... existing fields ...
    pub github_client: GitHubClient,
}
```

Update `App::new()` to initialize the client:

```rust
impl App {
    pub fn new(/* existing params */) -> Self {
        Self {
            // ... existing fields ...
            github_client: GitHubClient::new(),
        }
    }
}
```

**Step 3: Call refresh on startup**

In `src/tui/mod.rs`, after creating AgentsApp, refresh data:

```rust
if let Some(ref mut agents_app) = app.agents_app {
    agents_app.refresh_github_data(&app.github_client);
}
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/tui/agents_app.rs src/tui/app.rs src/tui/mod.rs
git commit -m "feat: integrate GitHub data fetching into agents app"
```

---

### Task 1.3: Update Agent List to Show Live Data

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Update draw_agent_list to show stars and latest version**

Replace the current agent list rendering to show:
- Latest version from GitHub (or "-" if not available)
- Stars formatted (e.g., "12.3k")
- Status based on version comparison

```rust
use crate::agents::format_stars;

// In draw_agent_list function, update the row rendering:
let latest = entry.github.latest_version.as_deref().unwrap_or("-");
let stars = entry.github.stars.map(format_stars).unwrap_or_else(|| "-".to_string());
let status = entry.status_str();

let row = Row::new(vec![
    Cell::from(entry.agent.name.clone()),
    Cell::from(installed),
    Cell::from(latest),
    Cell::from(stars),
    Cell::from(status),
]);
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Test manually**

Run: `cargo run -- tui`
Expected: Agents tab shows stars and latest versions (requires `gh` CLI authenticated)

**Step 4: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: show GitHub stars and latest version in agents list"
```

---

### Task 1.4: Enhance Detail Pane with GitHub Data

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Update draw_agent_detail to show rich information**

The detail pane should show:
- Version comparison: "Installed: 1.0.40 → Latest: 1.0.42"
- Release date with relative time
- Stars, license, pricing as a badge row
- Changelog section

```rust
use crate::agents::{format_stars, format_relative_time};

fn draw_agent_detail(f: &mut Frame, area: Rect, entry: &AgentEntry) {
    let mut lines = vec![];

    // Header: Name and repo
    lines.push(Line::from(vec![
        Span::styled(&entry.agent.name, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(&entry.agent.repo, Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));

    // Version comparison
    let installed = entry.installed.version.as_deref().unwrap_or("-");
    let latest = entry.github.latest_version.as_deref().unwrap_or("-");
    let version_line = if entry.update_available() {
        Line::from(vec![
            Span::raw(format!("Installed: {}  →  Latest: {}", installed, latest)),
            Span::raw("  "),
            Span::styled("⬆ UPDATE AVAILABLE", Style::default().fg(Color::Yellow)),
        ])
    } else if entry.installed.version.is_some() {
        Line::from(vec![
            Span::raw(format!("Installed: {}  →  Latest: {}", installed, latest)),
            Span::raw("  "),
            Span::styled("✓ Up to date", Style::default().fg(Color::Green)),
        ])
    } else {
        Line::from(format!("Latest: {}", latest))
    };
    lines.push(version_line);

    // Release date
    if let Some(ref date) = entry.github.release_date {
        lines.push(Line::from(format!("Released: {}", format_relative_time(date))));
    }
    lines.push(Line::from(""));

    // Badge row: stars, license, pricing, category
    let mut badges = vec![];
    if let Some(stars) = entry.github.stars {
        badges.push(Span::styled(
            format!("⭐ {}", format_stars(stars)),
            Style::default().fg(Color::Yellow),
        ));
        badges.push(Span::raw("  │  "));
    }
    if let Some(ref license) = entry.github.license {
        badges.push(Span::raw(license.clone()));
        badges.push(Span::raw("  │  "));
    }
    if let Some(ref pricing) = entry.agent.pricing {
        badges.push(Span::raw(pricing.model.clone()));
        badges.push(Span::raw("  │  "));
    }
    let category = entry.agent.categories.first().cloned().unwrap_or_default();
    badges.push(Span::raw(category.to_uppercase()));
    lines.push(Line::from(badges));
    lines.push(Line::from(""));

    // Changelog
    if let Some(ref changelog) = entry.github.changelog {
        lines.push(Line::from(Span::styled(
            format!("v{} Changelog:", latest),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        // Show first few lines of changelog
        for line in changelog.lines().take(5) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                lines.push(Line::from(format!("  {}", trimmed)));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Details "));
    f.render_widget(paragraph, area);
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Test manually**

Run: `cargo run -- tui`
Navigate to Agents tab, select an agent, verify detail pane shows rich data.

**Step 4: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: enhance agent detail pane with GitHub data and changelog"
```

---

## Phase 2: Add/Remove Picker Modal

### Task 2.1: Add Picker State

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/agents_app.rs`

**Step 1: Add picker state to AgentsApp**

```rust
pub struct AgentsApp {
    // ... existing fields ...
    pub show_picker: bool,
    pub picker_selected: usize,
    pub picker_changes: HashMap<String, bool>, // agent_id -> new tracked state
}

impl AgentsApp {
    pub fn open_picker(&mut self) {
        self.show_picker = true;
        self.picker_selected = 0;
        self.picker_changes.clear();
        // Initialize with current tracked states
        for entry in &self.entries {
            self.picker_changes.insert(entry.id.clone(), entry.tracked);
        }
    }

    pub fn close_picker(&mut self) {
        self.show_picker = false;
        self.picker_changes.clear();
    }

    pub fn picker_toggle_current(&mut self) {
        if let Some(entry) = self.entries.get(self.picker_selected) {
            let current = self.picker_changes.get(&entry.id).copied().unwrap_or(entry.tracked);
            self.picker_changes.insert(entry.id.clone(), !current);
        }
    }

    pub fn picker_next(&mut self) {
        if self.picker_selected < self.entries.len().saturating_sub(1) {
            self.picker_selected += 1;
        }
    }

    pub fn picker_prev(&mut self) {
        if self.picker_selected > 0 {
            self.picker_selected -= 1;
        }
    }

    pub fn picker_save(&mut self, config: &mut Config) {
        for (agent_id, tracked) in &self.picker_changes {
            config.set_tracked(agent_id, *tracked);
            // Update local entry
            if let Some(entry) = self.entries.iter_mut().find(|e| e.id == *agent_id) {
                entry.tracked = *tracked;
            }
        }
        let _ = config.save(); // Ignore save errors for now
        self.close_picker();
    }
}
```

**Step 2: Add Message variants for picker**

In `src/tui/app.rs`:

```rust
pub enum Message {
    // ... existing variants ...
    OpenPicker,
    ClosePicker,
    PickerNext,
    PickerPrev,
    PickerToggle,
    PickerSave,
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/tui/app.rs src/tui/agents_app.rs
git commit -m "feat: add picker state and navigation methods"
```

---

### Task 2.2: Add Picker Keybindings

**Files:**
- Modify: `src/tui/event.rs`

**Step 1: Handle `a` key to open picker**

In `handle_agents_keys`:

```rust
KeyCode::Char('a') => Some(Message::OpenPicker),
```

**Step 2: Handle picker mode keys**

When picker is open, intercept keys:

```rust
fn handle_picker_keys(code: KeyCode) -> Option<Message> {
    match code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::PickerNext),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::PickerPrev),
        KeyCode::Char(' ') => Some(Message::PickerToggle),
        KeyCode::Enter => Some(Message::PickerSave),
        KeyCode::Esc => Some(Message::ClosePicker),
        _ => None,
    }
}
```

**Step 3: Check for picker mode in event handler**

```rust
// In handle_key_event or handle_normal_mode
if app.current_tab == Tab::Agents {
    if let Some(ref agents_app) = app.agents_app {
        if agents_app.show_picker {
            return handle_picker_keys(code);
        }
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/tui/event.rs
git commit -m "feat: add picker keybindings"
```

---

### Task 2.3: Draw Picker Modal

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Add draw_picker_modal function**

```rust
fn draw_picker_modal(f: &mut Frame, agents_app: &AgentsApp) {
    // Calculate centered popup area
    let area = f.area();
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = (agents_app.entries.len() as u16 + 4).min(area.height.saturating_sub(4));
    let popup_area = Rect::new(
        (area.width - popup_width) / 2,
        (area.height - popup_height) / 2,
        popup_width,
        popup_height,
    );

    // Clear background
    f.render_widget(Clear, popup_area);

    // Build list items
    let items: Vec<ListItem> = agents_app
        .entries
        .iter()
        .map(|entry| {
            let tracked = agents_app.picker_changes.get(&entry.id).copied().unwrap_or(entry.tracked);
            let checkbox = if tracked { "[x]" } else { "[ ]" };
            let installed = if entry.installed.version.is_some() { "installed" } else { "" };
            let category = entry.agent.categories.first().cloned().unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::raw(format!("  {} ", checkbox)),
                Span::styled(
                    format!("{:<20}", entry.agent.name),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{:<12}", category), Style::default().fg(Color::DarkGray)),
                Span::styled(installed, Style::default().fg(Color::Green)),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(agents_app.picker_selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Add/Remove Tracked Agents ")
                .title_bottom(" Space: toggle │ Enter: save │ Esc: cancel "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(list, popup_area, &mut list_state);
}
```

**Step 2: Call draw_picker_modal from draw function**

In the `draw` function, after drawing the main content:

```rust
// Draw picker modal if visible
if app.current_tab == Tab::Agents {
    if let Some(ref agents_app) = app.agents_app {
        if agents_app.show_picker {
            draw_picker_modal(f, agents_app);
        }
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Test manually**

Run: `cargo run -- tui`
Press `]` to go to Agents tab, press `a` to open picker.

**Step 5: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: add picker modal UI"
```

---

### Task 2.4: Handle Picker Messages

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add message handlers**

```rust
impl App {
    pub fn update(&mut self, message: Message) {
        match message {
            // ... existing handlers ...

            Message::OpenPicker => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.open_picker();
                }
            }
            Message::ClosePicker => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.close_picker();
                }
            }
            Message::PickerNext => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.picker_next();
                }
            }
            Message::PickerPrev => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.picker_prev();
                }
            }
            Message::PickerToggle => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.picker_toggle_current();
                }
            }
            Message::PickerSave => {
                if let Some(ref mut agents_app) = self.agents_app {
                    agents_app.picker_save(&mut self.config);
                }
            }
        }
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat: handle picker messages"
```

---

## Phase 3: Copy Update Command

### Task 3.1: Add Update Command Generation

**Files:**
- Modify: `src/agents/data.rs`

**Step 1: Add update_command method to Agent**

```rust
impl Agent {
    /// Generate the update/install command for this agent
    pub fn update_command(&self) -> Option<String> {
        match self.installation_method.as_deref() {
            Some("cli") => {
                // Determine package manager based on cli_binary
                match self.cli_binary.as_deref() {
                    Some("claude") => Some("npm update -g @anthropic-ai/claude-code".to_string()),
                    Some("aider") => Some("pip install --upgrade aider-chat".to_string()),
                    Some("goose") => Some("pip install --upgrade goose-ai".to_string()),
                    _ => None,
                }
            }
            Some("ide") => {
                // IDEs typically auto-update or have their own update mechanism
                self.homepage.as_ref().map(|h| format!("Visit {} for download", h))
            }
            _ => None,
        }
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/agents/data.rs
git commit -m "feat: add update command generation"
```

---

### Task 3.2: Add Copy Update Command Keybinding

**Files:**
- Modify: `src/tui/event.rs`
- Modify: `src/tui/app.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Add Message variant**

In `src/tui/app.rs`:

```rust
pub enum Message {
    // ... existing variants ...
    CopyUpdateCommand,
}
```

**Step 2: Add keybinding**

In `src/tui/event.rs`, in `handle_agents_keys`:

```rust
KeyCode::Char('u') => Some(Message::CopyUpdateCommand),
```

**Step 3: Handle the message**

In `src/tui/mod.rs`, add handler:

```rust
Message::CopyUpdateCommand => {
    if let Some(ref agents_app) = app.agents_app {
        if let Some(entry) = agents_app.current_entry() {
            if let Some(cmd) = entry.agent.update_command() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&cmd);
                }
            }
        }
    }
}
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/tui/event.rs src/tui/app.rs src/tui/mod.rs
git commit -m "feat: add copy update command keybinding"
```

---

### Task 3.3: Update Help Popup

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Add `u` and `a` keybindings to help**

In the "Agents Tab" section of `draw_help_popup`:

```rust
Line::from(vec![
    Span::styled("  a             ", Style::default().fg(Color::Yellow)),
    Span::raw("Add/remove tracked agents"),
]),
Line::from(vec![
    Span::styled("  u             ", Style::default().fg(Color::Yellow)),
    Span::raw("Copy update command"),
]),
```

**Step 2: Update HELP_LINES constant**

Increase by 2 to account for new lines.

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: add picker and update command to help"
```

---

## Phase 4: Final Integration & Testing

### Task 4.1: Final Testing & Cleanup

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Test full workflow manually**

Run: `cargo run -- tui`
- Switch to Agents tab with `]`
- Verify stars and latest versions are populated (requires `gh` auth)
- Select an agent, verify detail pane shows changelog
- Press `a` to open picker, toggle some agents, press Enter to save
- Press `u` to copy update command, verify clipboard
- Press `?` to verify help shows new keybindings

**Step 4: Commit any final fixes**

```bash
git add -A
git commit -m "chore: final cleanup and integration"
```

---

## Summary

This plan adds the missing features from the original design:

1. **GitHub API** (Phase 1) - Live stars, versions, changelogs via `gh api`
2. **Add/Remove Picker** (Phase 2) - Modal to manage tracked agents
3. **Copy Update Command** (Phase 3) - `u` key to copy install/update command
4. **Enhanced Detail Pane** (Phase 1) - Rich display with version comparison, badges, changelog

Each task is a small, testable unit with exact file paths, code, and commands.
