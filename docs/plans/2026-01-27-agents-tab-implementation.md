# Agents Tab Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Redesign the Agents tab with a two-panel layout, async GitHub fetching, version history navigation, and proper filter/tracking support.

**Architecture:** Two-panel layout (agent list 35% + details 65%). Categories become filter toggles. GitHub data fetched asynchronously via reqwest/tokio with progressive UI updates. User custom agents supported via config file.

**Tech Stack:** Rust, ratatui, tokio (async runtime), reqwest (async HTTP), serde, toml

---

## Phase 3A: Core Fixes

### Task 3A.1: Fix Tracked Filtering in update_filtered

**Files:**
- Modify: `src/tui/agents_app.rs:114-152`
- Test: Manual testing (TUI app)

**Step 1: Add tracked_only filter field to AgentFilters**

In `src/tui/agents_app.rs`, modify the `AgentFilters` struct:

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct AgentFilters {
    pub installed_only: bool,
    pub cli_only: bool,
    pub open_source_only: bool,
    pub tracked_only: bool,  // NEW
}
```

**Step 2: Wire tracked filter into update_filtered**

In `src/tui/agents_app.rs`, modify `update_filtered` method to check tracked:

```rust
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
                && (!self.filters.open_source_only || entry.agent.open_source)
                && (!self.filters.tracked_only || entry.tracked);  // NEW

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
```

**Step 3: Add toggle method for tracked filter**

Add new method in `AgentsApp`:

```rust
pub fn toggle_tracked_filter(&mut self) {
    self.filters.tracked_only = !self.filters.tracked_only;
    self.selected_agent = 0;
    self.update_filtered();
}
```

**Step 4: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add src/tui/agents_app.rs
git commit -m "fix: wire tracked filter into update_filtered"
```

---

### Task 3A.2: Fix Config Save Error Handling

**Files:**
- Modify: `src/tui/agents_app.rs:300-309`

**Step 1: Propagate config save errors to status message**

Modify `picker_save` method to return Result and handle errors:

```rust
pub fn picker_save(&mut self, config: &mut Config) -> Result<(), String> {
    for (agent_id, tracked) in &self.picker_changes {
        config.set_tracked(agent_id, *tracked);
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == *agent_id) {
            entry.tracked = *tracked;
        }
    }

    if let Err(e) = config.save() {
        self.close_picker();
        return Err(format!("Failed to save config: {}", e));
    }

    self.close_picker();
    self.update_filtered();  // Re-filter in case tracked_only is active
    Ok(())
}
```

**Step 2: Update Message handler in app.rs**

In `src/tui/app.rs`, modify the `PickerSave` handler:

```rust
Message::PickerSave => {
    if let Some(ref mut agents_app) = self.agents_app {
        if let Err(e) = agents_app.picker_save(&mut self.config) {
            self.set_status(e);
        } else {
            self.set_status("Tracked agents saved".to_string());
        }
    }
}
```

**Step 3: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/tui/agents_app.rs src/tui/app.rs
git commit -m "fix: propagate config save errors to status message"
```

---

### Task 3A.3: Tab-Specific Footer Keybindings

**Files:**
- Modify: `src/tui/ui.rs:754-812`

**Step 1: Create separate footer content for each tab**

Replace the `draw_footer` function's `Mode::Normal` branch:

```rust
Mode::Normal => {
    // Split footer into left and right sections
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(10)])
        .split(area);

    let left_content = match app.current_tab {
        Tab::Models => Line::from(vec![
            Span::styled(" q ", Style::default().fg(Color::Yellow)),
            Span::raw("quit  "),
            Span::styled(" ↑/↓ ", Style::default().fg(Color::Yellow)),
            Span::raw("nav  "),
            Span::styled(" Tab ", Style::default().fg(Color::Yellow)),
            Span::raw("switch  "),
            Span::styled(" / ", Style::default().fg(Color::Yellow)),
            Span::raw("search  "),
            Span::styled(" s ", Style::default().fg(Color::Yellow)),
            Span::raw("sort  "),
            Span::styled(" c ", Style::default().fg(Color::Yellow)),
            Span::raw("copy"),
        ]),
        Tab::Agents => Line::from(vec![
            Span::styled(" q ", Style::default().fg(Color::Yellow)),
            Span::raw("quit  "),
            Span::styled(" ↑/↓ ", Style::default().fg(Color::Yellow)),
            Span::raw("nav  "),
            Span::styled(" Tab ", Style::default().fg(Color::Yellow)),
            Span::raw("switch  "),
            Span::styled(" s ", Style::default().fg(Color::Yellow)),
            Span::raw("sort  "),
            Span::styled(" a ", Style::default().fg(Color::Yellow)),
            Span::raw("track  "),
            Span::styled(" u ", Style::default().fg(Color::Yellow)),
            Span::raw("update"),
        ]),
    };

    let right_content = Line::from(vec![
        Span::styled(" ? ", Style::default().fg(Color::Yellow)),
        Span::raw("help "),
    ]);

    f.render_widget(Paragraph::new(left_content), chunks[0]);
    f.render_widget(
        Paragraph::new(right_content).alignment(ratatui::layout::Alignment::Right),
        chunks[1],
    );
}
```

**Step 2: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: show tab-specific keybindings in footer"
```

---

### Task 3A.4: Filter Indication in Block Title

**Files:**
- Modify: `src/tui/ui.rs:293-372` (draw_agent_list function)
- Modify: `src/tui/agents_app.rs` (add format_filters method)

**Step 1: Add format_filters method to AgentsApp**

In `src/tui/agents_app.rs`, add:

```rust
pub fn format_active_filters(&self) -> String {
    let mut active = Vec::new();

    // Category (if not "All")
    let category = AgentCategory::variants()[self.selected_category];
    if category != AgentCategory::All {
        active.push(category.label().to_lowercase());
    }

    // Additional filters
    if self.filters.installed_only {
        active.push("installed");
    }
    if self.filters.cli_only {
        active.push("cli");
    }
    if self.filters.open_source_only {
        active.push("open");
    }
    if self.filters.tracked_only {
        active.push("tracked");
    }

    if !self.search_query.is_empty() {
        active.push("search");
    }

    active.join(", ")
}
```

**Step 2: Update draw_agent_list to show filters in title**

In `src/tui/ui.rs`, modify `draw_agent_list` title generation:

```rust
let filter_indicator = app.format_active_filters();
let title = if filter_indicator.is_empty() {
    format!(" Agents ({}) ", app.filtered_entries.len())
} else {
    format!(" Agents ({}) [{}] ", app.filtered_entries.len(), filter_indicator)
};
```

**Step 3: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/tui/agents_app.rs src/tui/ui.rs
git commit -m "feat: show active filters in Agents block title"
```

---

### Task 3A.5: Add Tracked Filter Keybinding

**Files:**
- Modify: `src/tui/event.rs:127-166`
- Modify: `src/tui/app.rs:67-122` (Message enum)
- Modify: `src/tui/app.rs:200-441` (update method)

**Step 1: Add Message variant**

In `src/tui/app.rs`, add to Message enum after `ToggleOpenSourceFilter`:

```rust
ToggleTrackedFilter,
```

**Step 2: Add keybinding in handle_agents_keys**

In `src/tui/event.rs`, add in `handle_agents_keys` match:

```rust
KeyCode::Char('4') => Some(Message::ToggleTrackedFilter),
```

**Step 3: Handle message in App::update**

In `src/tui/app.rs`, add in update match after `ToggleOpenSourceFilter`:

```rust
Message::ToggleTrackedFilter => {
    if let Some(ref mut agents_app) = self.agents_app {
        agents_app.toggle_tracked_filter();
    }
}
```

**Step 4: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add src/tui/event.rs src/tui/app.rs
git commit -m "feat: add '4' keybinding to toggle tracked filter"
```

---

## Phase 3B: Layout Redesign

### Task 3B.1: Simplify AgentFocus to Two Panels

**Files:**
- Modify: `src/tui/agents_app.rs:40-45`

**Step 1: Update AgentFocus enum**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentFocus {
    #[default]
    List,
    Details,
}
```

**Step 2: Update switch_focus method**

```rust
pub fn switch_focus(&mut self) {
    self.focus = match self.focus {
        AgentFocus::List => AgentFocus::Details,
        AgentFocus::Details => AgentFocus::List,
    };
}
```

**Step 3: Run cargo check**

Run: `mise run check`
Expected: Compile errors (we'll fix in next tasks)

**Step 4: Commit**

```bash
git add src/tui/agents_app.rs
git commit -m "refactor: simplify AgentFocus to List/Details"
```

---

### Task 3B.2: Remove Categories Sidebar, Add Filter Toggles

**Files:**
- Modify: `src/tui/ui.rs:237-291`
- Delete function: `draw_agent_categories`

**Step 1: Rewrite draw_agents_main for two-panel layout**

Replace `draw_agents_main`:

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

    // Two-panel layout: List (35%) | Details (65%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_agent_list_panel(f, chunks[0], app);
    draw_agent_detail_panel(f, chunks[1], app);
}
```

**Step 2: Remove draw_agent_categories function entirely**

Delete the `draw_agent_categories` function (lines ~257-291).

**Step 3: Run cargo check**

Run: `mise run check`
Expected: Compile errors for missing functions (we'll add them)

**Step 4: Commit**

```bash
git add src/tui/ui.rs
git commit -m "refactor: remove categories sidebar, use two-panel layout"
```

---

### Task 3B.3: Create New Agent List Panel with Filter Header

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Create draw_agent_list_panel function**

Add new function:

```rust
fn draw_agent_list_panel(f: &mut Frame, area: Rect, app: &mut App) {
    use super::agents_app::AgentFocus;

    let agents_app = match &mut app.agents_app {
        Some(a) => a,
        None => return,
    };

    let is_focused = agents_app.focus == AgentFocus::List;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Split into filter row + list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    // Filter toggles row
    let filter_line = Line::from(vec![
        Span::styled(" [1]", Style::default().fg(if agents_app.filters.installed_only { Color::Green } else { Color::DarkGray })),
        Span::raw(" Inst "),
        Span::styled("[2]", Style::default().fg(if agents_app.filters.cli_only { Color::Green } else { Color::DarkGray })),
        Span::raw(" CLI "),
        Span::styled("[3]", Style::default().fg(if agents_app.filters.open_source_only { Color::Green } else { Color::DarkGray })),
        Span::raw(" OSS "),
        Span::styled("[4]", Style::default().fg(if agents_app.filters.tracked_only { Color::Green } else { Color::DarkGray })),
        Span::raw(" Track"),
    ]);
    let filter_para = Paragraph::new(filter_line)
        .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT).border_style(border_style));
    f.render_widget(filter_para, chunks[0]);

    // Agent list
    let mut items: Vec<ListItem> = Vec::new();

    // Header row
    let header = format!("{:<20} {:>6} {:>12}", "Agent", "Type", "Version");
    items.push(
        ListItem::new(header).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED),
        ),
    );

    // Agent rows
    for &idx in &agents_app.filtered_entries {
        if let Some(entry) = agents_app.entries.get(idx) {
            let agent_type = if entry.agent.categories.contains(&"cli".to_string()) {
                "CLI"
            } else if entry.agent.categories.contains(&"ide".to_string()) {
                "IDE"
            } else {
                "-"
            };

            let version = format_smart_version(entry);

            let row = format!(
                "{:<20} {:>6} {:>12}",
                truncate(&entry.agent.name, 20),
                agent_type,
                truncate(&version, 12),
            );
            items.push(ListItem::new(row));
        }
    }

    let filter_indicator = agents_app.format_active_filters();
    let title = if filter_indicator.is_empty() {
        format!(" Agents ({}) ", agents_app.filtered_entries.len())
    } else {
        format!(" Agents ({}) [{}] ", agents_app.filtered_entries.len(), filter_indicator)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
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
    let mut state = agents_app.agent_list_state.clone();
    if let Some(selected) = state.selected() {
        state.select(Some(selected + 1));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);
}

fn format_smart_version(entry: &crate::agents::AgentEntry) -> String {
    match (&entry.installed.version, &entry.github.latest_version) {
        (None, _) => "-".to_string(),
        (Some(installed), None) => format!("v{}", installed),
        (Some(installed), Some(latest)) => {
            if entry.update_available() {
                format!("{} -> {}", installed, latest)
            } else {
                format!("v{} ✓", installed)
            }
        }
    }
}
```

**Step 2: Run cargo check**

Run: `mise run check`
Expected: Compiles (may have warnings)

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: new agent list panel with filter header and smart version"
```

---

### Task 3B.4: Create New Details Panel

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Create draw_agent_detail_panel function**

Add new function (replaces old draw_agent_detail):

```rust
fn draw_agent_detail_panel(f: &mut Frame, area: Rect, app: &App) {
    use super::agents_app::AgentFocus;

    let agents_app = match &app.agents_app {
        Some(a) => a,
        None => return,
    };

    let is_focused = agents_app.focus == AgentFocus::Details;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let lines: Vec<Line> = if let Some(entry) = agents_app.current_entry() {
        let mut detail_lines = Vec::new();

        // Header: Name + Version
        let version_str = entry.github.latest_version.as_deref().unwrap_or("-");
        detail_lines.push(Line::from(vec![
            Span::styled(
                &entry.agent.name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("v{}", version_str),
                Style::default().fg(Color::Cyan),
            ),
        ]));

        // Repo + Stars
        let stars_str = entry.github.stars.map(format_stars).unwrap_or_default();
        detail_lines.push(Line::from(vec![
            Span::styled(&entry.agent.repo, Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled(format!("★ {}", stars_str), Style::default().fg(Color::Yellow)),
        ]));

        detail_lines.push(Line::from(""));

        // Installed vs Latest
        let installed_str = entry.installed.version.as_deref().unwrap_or("Not installed");
        let status = if entry.update_available() {
            Span::styled(" (update available)", Style::default().fg(Color::Yellow))
        } else if entry.installed.version.is_some() {
            Span::styled(" (up to date)", Style::default().fg(Color::Green))
        } else {
            Span::raw("")
        };

        detail_lines.push(Line::from(vec![
            Span::styled("Installed: ", Style::default().fg(Color::DarkGray)),
            Span::raw(installed_str),
            status,
        ]));

        if let Some(latest) = &entry.github.latest_version {
            let release_date = entry.github.release_date.as_deref().unwrap_or("unknown");
            detail_lines.push(Line::from(vec![
                Span::styled("Latest:    ", Style::default().fg(Color::DarkGray)),
                Span::raw(format!("v{}", latest)),
                Span::styled(format!(" ({})", release_date), Style::default().fg(Color::DarkGray)),
            ]));
        }

        detail_lines.push(Line::from(""));

        // Changelog
        if let Some(changelog) = &entry.github.changelog {
            detail_lines.push(Line::from(Span::styled(
                "Changelog:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            detail_lines.push(Line::from(Span::styled(
                "───────────────────────────────────",
                Style::default().fg(Color::DarkGray),
            )));

            // Light parsing: split by newlines, render each line
            for line in changelog.lines().take(15) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // Basic bullet point detection
                let formatted = if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    format!("  • {}", &trimmed[2..])
                } else if trimmed.starts_with("## ") {
                    trimmed[3..].to_string()
                } else {
                    format!("  {}", trimmed)
                };
                detail_lines.push(Line::from(truncate(&formatted, 60)));
            }
        } else {
            detail_lines.push(Line::from(Span::styled(
                "No changelog available",
                Style::default().fg(Color::DarkGray),
            )));
        }

        detail_lines
    } else {
        vec![Line::from(Span::styled(
            "Select an agent to view details",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Details "),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
```

**Step 2: Remove old draw_agent_detail function**

Delete the old `draw_agent_detail` function.

**Step 3: Update draw function to not call old detail pane**

In `draw()`, change the Agents tab rendering:

```rust
Tab::Agents => {
    draw_agents_main(f, chunks[1], app);
    // Detail panel is now integrated into draw_agents_main
}
```

Also update the layout constraints to remove the bottom detail panel for Agents:

```rust
pub fn draw(f: &mut Frame, app: &mut App) {
    let (main_constraint, detail_constraint) = match app.current_tab {
        Tab::Models => (Constraint::Min(0), Constraint::Length(14)),
        Tab::Agents => (Constraint::Min(0), Constraint::Length(0)), // No bottom detail for Agents
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            main_constraint,        // Main content
            detail_constraint,      // Detail panel (Models only)
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
            draw_agents_main(f, chunks[1], app);
        }
    }

    draw_footer(f, chunks[3], app);
    // ... rest unchanged
}
```

**Step 4: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat: new details panel with changelog display"
```

---

### Task 3B.5: Fix Event Handling for New Focus Model

**Files:**
- Modify: `src/tui/event.rs:127-166`

**Step 1: Update handle_agents_keys for new focus model**

```rust
fn handle_agents_keys(app: &App, code: KeyCode, _modifiers: KeyModifiers) -> Option<Message> {
    use super::agents_app::AgentFocus;

    let focus = app
        .agents_app
        .as_ref()
        .map(|a| a.focus)
        .unwrap_or(AgentFocus::List);

    match code {
        // Navigation - works in List focus
        KeyCode::Char('j') | KeyCode::Down => {
            if focus == AgentFocus::List {
                Some(Message::NextAgent)
            } else {
                Some(Message::ScrollDetailDown)
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if focus == AgentFocus::List {
                Some(Message::PrevAgent)
            } else {
                Some(Message::ScrollDetailUp)
            }
        }
        KeyCode::Char('h') | KeyCode::Left => Some(Message::SwitchAgentFocus),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::SwitchAgentFocus),
        KeyCode::Tab | KeyCode::BackTab => Some(Message::SwitchAgentFocus),

        // Actions
        KeyCode::Char('o') => Some(Message::OpenAgentDocs),
        KeyCode::Char('r') => Some(Message::OpenAgentRepo),
        KeyCode::Char('c') => Some(Message::CopyAgentName),
        KeyCode::Char('u') => Some(Message::CopyUpdateCommand),

        // Filters
        KeyCode::Char('1') => Some(Message::ToggleInstalledFilter),
        KeyCode::Char('2') => Some(Message::ToggleCliFilter),
        KeyCode::Char('3') => Some(Message::ToggleOpenSourceFilter),
        KeyCode::Char('4') => Some(Message::ToggleTrackedFilter),

        // Picker
        KeyCode::Char('a') => Some(Message::OpenPicker),

        // Search
        KeyCode::Char('/') => Some(Message::EnterSearch),

        _ => None,
    }
}
```

**Step 2: Add new Message variants for detail scrolling**

In `src/tui/app.rs`, add to Message enum:

```rust
ScrollDetailUp,
ScrollDetailDown,
```

**Step 3: Add detail_scroll field to AgentsApp**

In `src/tui/agents_app.rs`, add field:

```rust
pub struct AgentsApp {
    // ... existing fields
    pub detail_scroll: u16,
}
```

Initialize in `new()`:

```rust
detail_scroll: 0,
```

Reset in selection changes (in `next_agent`, `prev_agent`):

```rust
self.detail_scroll = 0;
```

**Step 4: Handle scroll messages in App::update**

In `src/tui/app.rs`:

```rust
Message::ScrollDetailUp => {
    if let Some(ref mut agents_app) = self.agents_app {
        agents_app.detail_scroll = agents_app.detail_scroll.saturating_sub(1);
    }
}
Message::ScrollDetailDown => {
    if let Some(ref mut agents_app) = self.agents_app {
        agents_app.detail_scroll = agents_app.detail_scroll.saturating_add(1);
    }
}
```

**Step 5: Apply scroll in draw_agent_detail_panel**

Add `.scroll((agents_app.detail_scroll, 0))` to the Paragraph in `draw_agent_detail_panel`:

```rust
let paragraph = Paragraph::new(lines)
    .block(/* ... */)
    .wrap(Wrap { trim: false })
    .scroll((agents_app.detail_scroll, 0));
```

**Step 6: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add src/tui/event.rs src/tui/app.rs src/tui/agents_app.rs src/tui/ui.rs
git commit -m "feat: detail panel scrolling with focus-based navigation"
```

---

### Task 3B.6: Add Sort Functionality for Agents

**Files:**
- Modify: `src/tui/agents_app.rs`
- Modify: `src/tui/event.rs`
- Modify: `src/tui/app.rs`

**Step 1: Add AgentSortOrder enum**

In `src/tui/agents_app.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentSortOrder {
    #[default]
    Name,
    Updated,
    Stars,
    Status,
}

impl AgentSortOrder {
    pub fn next(self) -> Self {
        match self {
            AgentSortOrder::Name => AgentSortOrder::Updated,
            AgentSortOrder::Updated => AgentSortOrder::Stars,
            AgentSortOrder::Stars => AgentSortOrder::Status,
            AgentSortOrder::Status => AgentSortOrder::Name,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AgentSortOrder::Name => "name",
            AgentSortOrder::Updated => "updated",
            AgentSortOrder::Stars => "stars",
            AgentSortOrder::Status => "status",
        }
    }
}
```

**Step 2: Add sort_order field to AgentsApp**

```rust
pub struct AgentsApp {
    // ... existing fields
    pub sort_order: AgentSortOrder,
}
```

Initialize in `new()`:

```rust
sort_order: AgentSortOrder::default(),
```

**Step 3: Add cycle_sort and apply_sort methods**

```rust
pub fn cycle_sort(&mut self) {
    self.sort_order = self.sort_order.next();
    self.apply_sort();
}

fn apply_sort(&mut self) {
    // Sort filtered_entries based on sort_order
    let entries = &self.entries;
    self.filtered_entries.sort_by(|&a, &b| {
        let ea = &entries[a];
        let eb = &entries[b];
        match self.sort_order {
            AgentSortOrder::Name => ea.agent.name.cmp(&eb.agent.name),
            AgentSortOrder::Updated => {
                // Sort by release date descending (newest first)
                let da = ea.github.release_date.as_deref().unwrap_or("");
                let db = eb.github.release_date.as_deref().unwrap_or("");
                db.cmp(da)
            }
            AgentSortOrder::Stars => {
                // Sort by stars descending
                let sa = ea.github.stars.unwrap_or(0);
                let sb = eb.github.stars.unwrap_or(0);
                sb.cmp(&sa)
            }
            AgentSortOrder::Status => {
                // Update available first, then installed, then not installed
                let status_a = if ea.update_available() { 0 } else if ea.installed.version.is_some() { 1 } else { 2 };
                let status_b = if eb.update_available() { 0 } else if eb.installed.version.is_some() { 1 } else { 2 };
                status_a.cmp(&status_b)
            }
        }
    });
}
```

**Step 4: Call apply_sort at end of update_filtered**

```rust
pub fn update_filtered(&mut self) {
    // ... existing filter logic
    self.apply_sort();

    // Reset selection if out of bounds
    // ... rest unchanged
}
```

**Step 5: Add keybinding and message**

In `src/tui/event.rs`, add in `handle_agents_keys`:

```rust
KeyCode::Char('s') => Some(Message::CycleAgentSort),
```

In `src/tui/app.rs`, add Message variant:

```rust
CycleAgentSort,
```

Handle it:

```rust
Message::CycleAgentSort => {
    if let Some(ref mut agents_app) = self.agents_app {
        agents_app.cycle_sort();
    }
}
```

**Step 6: Show sort indicator in title**

Update `draw_agent_list_panel` title:

```rust
let sort_indicator = format!(" ↓{}", agents_app.sort_order.label());
let filter_indicator = agents_app.format_active_filters();
let title = if filter_indicator.is_empty() {
    format!(" Agents ({}){} ", agents_app.filtered_entries.len(), sort_indicator)
} else {
    format!(" Agents ({}) [{}]{} ", agents_app.filtered_entries.len(), filter_indicator, sort_indicator)
};
```

**Step 7: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 8: Commit**

```bash
git add src/tui/agents_app.rs src/tui/event.rs src/tui/app.rs src/tui/ui.rs
git commit -m "feat: add sort functionality for agents (name/updated/stars/status)"
```

---

## Phase 3C: Async & Performance

### Task 3C.1: Add Async Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Update Cargo.toml**

Add tokio runtime and update reqwest for async:

```toml
[dependencies]
# ... existing deps

# Async runtime
tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros"] }

# HTTP (update to async)
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

Note: Remove "blocking" feature from reqwest.

**Step 2: Run cargo check**

Run: `mise run check`
Expected: Compiles (may have warnings about unused async)

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "deps: add tokio for async runtime"
```

---

### Task 3C.2: Create Async GitHub Client

**Files:**
- Rewrite: `src/agents/github.rs`

**Step 1: Rewrite GitHubClient for async**

```rust
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::GitHubData;

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);
const GITHUB_API_BASE: &str = "https://api.github.com";

struct CacheEntry {
    data: GitHubData,
    fetched_at: Instant,
}

#[derive(Debug, Deserialize)]
pub struct RepoResponse {
    pub stargazers_count: u64,
    pub open_issues_count: u64,
    pub license: Option<LicenseResponse>,
    pub pushed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LicenseResponse {
    pub spdx_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseResponse {
    pub tag_name: String,
    pub published_at: Option<String>,
    pub body: Option<String>,
}

#[derive(Clone)]
pub struct AsyncGitHubClient {
    client: reqwest::Client,
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    token: Option<String>,
}

impl Default for AsyncGitHubClient {
    fn default() -> Self {
        Self::new(None)
    }
}

impl AsyncGitHubClient {
    pub fn new(token: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("models-tui")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            cache: Arc::new(Mutex::new(HashMap::new())),
            token,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub async fn fetch(&self, repo: &str) -> Result<GitHubData> {
        // Check cache
        {
            let cache = self.cache.lock().await;
            if let Some(entry) = cache.get(repo) {
                if entry.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(entry.data.clone());
                }
            }
        }

        let data = self.fetch_fresh(repo).await?;

        // Update cache
        {
            let mut cache = self.cache.lock().await;
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

    pub async fn fetch_fresh(&self, repo: &str) -> Result<GitHubData> {
        let mut data = GitHubData::default();

        // Fetch repo and release in parallel
        let repo_url = format!("{}/repos/{}", GITHUB_API_BASE, repo);
        let release_url = format!("{}/repos/{}/releases/latest", GITHUB_API_BASE, repo);

        let (repo_result, release_result) = tokio::join!(
            self.get_json::<RepoResponse>(&repo_url),
            self.get_json::<ReleaseResponse>(&release_url),
        );

        if let Ok(repo_info) = repo_result {
            data.stars = Some(repo_info.stargazers_count);
            data.open_issues = Some(repo_info.open_issues_count);
            data.license = repo_info
                .license
                .and_then(|l| l.spdx_id)
                .filter(|s| s != "NOASSERTION");
            data.last_commit = repo_info.pushed_at.map(|s| format_relative_time(&s));
        }

        if let Ok(release) = release_result {
            let version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
            data.latest_version = Some(version.to_string());
            data.release_date = release.published_at.map(|s| format_relative_time(&s));
            data.changelog = release.body;
        }

        Ok(data)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut request = self.client.get(url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if response.status() == 403 {
            // Rate limited
            return Err(anyhow!("GitHub API rate limit exceeded"));
        }

        if !response.status().is_success() {
            return Err(anyhow!("GitHub API error: {}", response.status()));
        }

        Ok(response.json().await?)
    }
}

pub fn format_stars(stars: u64) -> String {
    if stars >= 1_000_000 {
        format!("{:.1}m", stars as f64 / 1_000_000.0)
    } else if stars >= 1_000 {
        format!("{:.1}k", stars as f64 / 1_000.0)
    } else {
        stars.to_string()
    }
}

pub fn format_relative_time(iso_date: &str) -> String {
    if let Some(date) = iso_date.split('T').next() {
        date.to_string()
    } else {
        iso_date.to_string()
    }
}

// Keep sync client for backwards compatibility during migration
pub struct GitHubClient {
    async_client: AsyncGitHubClient,
    runtime: tokio::runtime::Handle,
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubClient {
    pub fn new() -> Self {
        Self {
            async_client: AsyncGitHubClient::new(None),
            runtime: tokio::runtime::Handle::current(),
        }
    }

    pub fn fetch(&self, repo: &str) -> Result<GitHubData> {
        self.runtime.block_on(self.async_client.fetch(repo))
    }

    pub fn fetch_fresh(&self, repo: &str) -> Result<GitHubData> {
        self.runtime.block_on(self.async_client.fetch_fresh(repo))
    }
}
```

**Step 2: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/agents/github.rs
git commit -m "feat: async GitHub client with reqwest"
```

---

### Task 3C.3: Add Background Fetch Channel

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Add channel for GitHub updates**

In `src/tui/app.rs`, add:

```rust
use tokio::sync::mpsc;

pub struct App {
    // ... existing fields
    pub github_rx: Option<mpsc::Receiver<(String, GitHubData)>>,
}
```

**Step 2: Add Message variant for GitHub updates**

```rust
pub enum Message {
    // ... existing
    GitHubDataReceived(String, GitHubData),
}
```

**Step 3: Handle GitHub data in update**

```rust
Message::GitHubDataReceived(agent_id, data) => {
    if let Some(ref mut agents_app) = self.agents_app {
        if let Some(entry) = agents_app.entries.iter_mut().find(|e| e.id == agent_id) {
            entry.github = data;
        }
    }
}
```

**Step 4: Create spawn_github_fetch function**

In `src/tui/mod.rs` or a new `src/tui/fetch.rs`:

```rust
use tokio::sync::mpsc;
use crate::agents::{AsyncGitHubClient, GitHubData, AgentEntry};

pub fn spawn_github_fetches(
    entries: &[AgentEntry],
    client: AsyncGitHubClient,
) -> mpsc::Receiver<(String, GitHubData)> {
    let (tx, rx) = mpsc::channel(100);

    for entry in entries {
        let tx = tx.clone();
        let client = client.clone();
        let id = entry.id.clone();
        let repo = entry.agent.repo.clone();

        tokio::spawn(async move {
            if let Ok(data) = client.fetch(&repo).await {
                let _ = tx.send((id, data)).await;
            }
        });
    }

    rx
}
```

**Step 5: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add src/tui/app.rs src/tui/mod.rs
git commit -m "feat: background GitHub fetch with channel"
```

---

### Task 3C.4: Integrate Async Fetching into TUI Loop

**Files:**
- Modify: `src/tui/mod.rs`

**Step 1: Update run function for async**

The main TUI loop needs to poll both keyboard events and the GitHub channel:

```rust
pub async fn run(providers_map: ProvidersMap, agents_file: Option<AgentsFile>, config: Option<Config>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = config.unwrap_or_default();
    let mut app = App::new(providers_map, agents_file.as_ref(), Some(config));

    // Spawn background GitHub fetches
    let github_client = AsyncGitHubClient::new(None);
    let mut github_rx = if let Some(ref agents_app) = app.agents_app {
        Some(spawn_github_fetches(&agents_app.entries, github_client))
    } else {
        None
    };

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Poll for events with timeout
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(msg) = event::handle_key(&app, key) {
                        if !app.update(msg.clone()) {
                            break;
                        }
                        // Handle copy/open actions
                        handle_action(&mut app, &msg);
                    }
                }
            }
        }

        // Check for GitHub updates (non-blocking)
        if let Some(ref mut rx) = github_rx {
            while let Ok((id, data)) = rx.try_recv() {
                app.update(Message::GitHubDataReceived(id, data));
            }
        }

        // Clear status message after delay
        // ... existing logic
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
```

**Step 2: Update main.rs to use tokio runtime**

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ... existing CLI parsing

    match cli.command {
        Some(Commands::Tui) | None => {
            tui::run(providers, agents, config).await?;
        }
        // ... other commands
    }

    Ok(())
}
```

**Step 3: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/tui/mod.rs src/main.rs
git commit -m "feat: integrate async GitHub fetching into TUI loop"
```

---

### Task 3C.5: Add Loading Indicator

**Files:**
- Modify: `src/tui/agents_app.rs`
- Modify: `src/tui/ui.rs`

**Step 1: Add loading state field**

In `AgentsApp`:

```rust
pub loading_github: bool,
```

Initialize as `true` in `new()`, set to `false` when all fetches complete.

**Step 2: Show loading in detail panel**

In `draw_agent_detail_panel`, when `loading_github` is true and no data:

```rust
if agents_app.loading_github && entry.github.latest_version.is_none() {
    detail_lines.push(Line::from(Span::styled(
        "Loading GitHub data...",
        Style::default().fg(Color::DarkGray),
    )));
}
```

**Step 3: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/tui/agents_app.rs src/tui/ui.rs
git commit -m "feat: loading indicator for GitHub data"
```

---

## Phase 3D: Version History (Deferred)

> Note: This phase requires fetching multiple releases from GitHub API and implementing version navigation. Deferring to keep initial implementation scope manageable.

### Task 3D.1: Fetch Release History

**Status:** Deferred to future iteration

### Task 3D.2: Version Navigation UI

**Status:** Deferred to future iteration

---

## Phase 3E: User Custom Agents

### Task 3E.1: Define Custom Agents Schema

**Files:**
- Modify: `src/config.rs`

**Step 1: Add CustomAgent struct**

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomAgent {
    pub name: String,
    pub repo: String,
    #[serde(default)]
    pub agent_type: Option<String>,  // "cli" or "ide"
    #[serde(default)]
    pub binary: Option<String>,
    #[serde(default)]
    pub version_command: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AgentsConfig {
    #[serde(default)]
    pub tracked: HashSet<String>,
    #[serde(default)]
    pub excluded: HashSet<String>,
    #[serde(default)]
    pub custom: Vec<CustomAgent>,
}
```

**Step 2: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat: custom agents schema in config"
```

---

### Task 3E.2: Load and Merge Custom Agents

**Files:**
- Modify: `src/agents/loader.rs` (or create if needed)
- Modify: `src/tui/agents_app.rs`

**Step 1: Add function to convert CustomAgent to Agent**

```rust
impl CustomAgent {
    pub fn to_agent(&self) -> crate::agents::Agent {
        crate::agents::Agent {
            name: self.name.clone(),
            repo: self.repo.clone(),
            categories: self.agent_type.as_ref()
                .map(|t| vec![t.clone()])
                .unwrap_or_default(),
            cli_binary: self.binary.clone(),
            version_command: self.version_command.clone().unwrap_or_default(),
            // ... other fields default
            installation_method: self.agent_type.clone(),
            pricing: None,
            supported_providers: vec![],
            platform_support: vec![],
            open_source: true,
            version_regex: None,
            config_files: vec![],
            homepage: None,
            docs: None,
        }
    }
}
```

**Step 2: Merge custom agents in AgentsApp::new**

```rust
pub fn new(agents_file: &AgentsFile, config: &Config) -> Self {
    let mut entries: Vec<AgentEntry> = agents_file
        .agents
        .iter()
        .map(|(id, agent)| {
            // ... existing logic
        })
        .collect();

    // Add custom agents from config
    for custom in &config.agents.custom {
        let id = custom.name.to_lowercase().replace(' ', "-");
        // Skip if already exists (user override)
        if entries.iter().any(|e| e.id == id) {
            continue;
        }
        let agent = custom.to_agent();
        let installed = detect_installed(&agent);
        entries.push(AgentEntry {
            id,
            agent,
            github: GitHubData::default(),
            installed,
            tracked: true,  // Custom agents are tracked by default
        });
    }

    // Sort by name
    entries.sort_by(|a, b| a.agent.name.cmp(&b.agent.name));

    // ... rest unchanged
}
```

**Step 3: Run cargo check**

Run: `mise run check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/config.rs src/tui/agents_app.rs
git commit -m "feat: load and merge custom agents from config"
```

---

### Task 3E.3: Document Custom Agents

**Files:**
- Create: `docs/custom-agents.md`

**Step 1: Write documentation**

```markdown
# Custom Agents

You can add custom agents to track in the Models TUI by editing your config file.

## Config Location

`~/.config/models/config.toml`

## Adding Custom Agents

Add a `[[agents.custom]]` section for each agent:

```toml
[[agents.custom]]
name = "My Internal Agent"
repo = "myorg/internal-agent"
agent_type = "cli"
binary = "myagent"
version_command = ["myagent", "--version"]

[[agents.custom]]
name = "Custom IDE Plugin"
repo = "myorg/ide-plugin"
agent_type = "ide"
```

## Fields

| Field | Required | Description |
|-------|----------|-------------|
| name | Yes | Display name for the agent |
| repo | Yes | GitHub repo (owner/repo format) |
| agent_type | No | "cli" or "ide" |
| binary | No | CLI binary name for version detection |
| version_command | No | Command to get installed version |

## Notes

- Custom agents are tracked by default
- If a custom agent has the same name as a built-in agent, the built-in is used
- GitHub data (stars, releases) is fetched automatically
```

**Step 2: Commit**

```bash
git add docs/custom-agents.md
git commit -m "docs: add custom agents documentation"
```

---

## Final Task: Integration Test

### Task 3F.1: Manual Testing Checklist

**Run the application:**

```bash
mise run dev
```

**Test checklist:**

1. [ ] Startup renders quickly (UI visible in <500ms)
2. [ ] GitHub data loads progressively (stars, versions appear)
3. [ ] Two-panel layout displays correctly
4. [ ] Filter toggles work (1-4 keys)
5. [ ] Filter indication shows in title
6. [ ] Footer shows Agents-specific keys
7. [ ] Tab switches focus between list and details
8. [ ] Details panel scrolls with j/k when focused
9. [ ] Sort cycles through options (s key)
10. [ ] Picker opens (a key) and saves changes
11. [ ] Tracked filter hides untracked agents
12. [ ] Copy update command works (u key)
13. [ ] Custom agents appear if configured

**Commit final state:**

```bash
git add -A
git commit -m "feat: agents tab redesign complete"
```

---

## Summary

| Phase | Tasks | Focus |
|-------|-------|-------|
| 3A | 5 | Core bug fixes (tracked filter, config save, footer, filter indication) |
| 3B | 6 | Layout redesign (two-panel, remove categories, new list/detail panels) |
| 3C | 5 | Async performance (tokio, reqwest, background fetch, loading indicator) |
| 3D | - | Deferred (version history navigation) |
| 3E | 3 | Custom agents (schema, loading, docs) |
| 3F | 1 | Integration testing |

**Total: 20 tasks**
