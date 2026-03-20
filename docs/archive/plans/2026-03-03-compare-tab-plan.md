# Compare Tab Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a 4th "Compare" tab with scatter plots, radar charts, and ranked comparison tables for side-by-side model evaluation.

**Architecture:** New `CompareApp` state struct following the existing Elm-architecture pattern (Message enum → update → draw). Radar chart rendered via ratatui Canvas + Braille markers. Scatter plot uses built-in Chart widget. Shared `compare_selections` on App for cross-tab model tagging.

**Tech Stack:** Rust, ratatui (Canvas, Chart, BarChart widgets), crossterm, tokio

---

### Task 1: Add Tab::Compare variant

**Files:**
- Modify: `src/tui/app.rs:48-54` (Tab enum)
- Modify: `src/tui/app.rs` (Tab::next, Tab::prev)

**Step 1: Write the failing test**

Add to the existing `#[cfg(test)]` module in `src/tui/app.rs`:

```rust
#[test]
fn tab_cycle_includes_compare() {
    assert_eq!(Tab::Benchmarks.next(), Tab::Compare);
    assert_eq!(Tab::Compare.next(), Tab::Models);
    assert_eq!(Tab::Models.prev(), Tab::Compare);
    assert_eq!(Tab::Compare.prev(), Tab::Benchmarks);
}
```

**Step 2: Run test to verify it fails**

Run: `mise run test`
Expected: FAIL — `Tab::Compare` does not exist

**Step 3: Write minimal implementation**

Update the `Tab` enum to add `Compare` after `Benchmarks`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Models,
    Agents,
    Benchmarks,
    Compare,
}
```

Update `Tab::next()`: `Benchmarks -> Compare`, `Compare -> Models`.
Update `Tab::prev()`: `Models -> Compare`, `Compare -> Benchmarks`.

**Step 4: Fix compilation errors**

The new variant will cause exhaustive match errors throughout the codebase. Add placeholder arms:

- `src/tui/ui.rs:draw()` — match `Tab::Compare` alongside `Tab::Benchmarks` for now (same constraint tuple, call a stub `draw_compare_main`)
- `src/tui/event.rs:handle_normal_mode()` — match `Tab::Compare` returning `None` for now
- `src/tui/app.rs:update()` — any `match self.current_tab` blocks that exist need `Tab::Compare` arms (check `SearchInput`, `SearchBackspace`, `ClearSearch`, `ExitSearch` dispatches)
- `src/tui/ui.rs:draw_footer()` — add Compare to the tab bar rendering
- `src/tui/ui.rs:draw_help_popup()` — add a placeholder `Tab::Compare` arm

For the stub `draw_compare_main`, just render a `Paragraph::new("Compare tab coming soon")` in the area.

**Step 5: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS — all tests green, no clippy warnings

**Step 6: Commit**

```bash
git add -A && git commit -m "feat: add Tab::Compare variant with cycle navigation"
```

---

### Task 2: Create CompareApp struct with basic state

**Files:**
- Create: `src/tui/compare_app.rs`
- Modify: `src/tui/mod.rs` (add module declaration)

**Step 1: Write the failing test**

In `src/tui/compare_app.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_compare_app_has_empty_selections() {
        let app = CompareApp::new();
        assert!(app.selections.is_empty());
        assert_eq!(app.mode, CompareMode::Default);
        assert_eq!(app.chart_view, ChartView::Scatter);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `mise run test`
Expected: FAIL — module and types don't exist

**Step 3: Write minimal implementation**

Create `src/tui/compare_app.rs`:

```rust
use crate::benchmarks::BenchmarkStore;
use std::collections::HashMap;

pub const MAX_SELECTIONS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompareMode {
    #[default]
    Default,
    HeadToHead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartView {
    #[default]
    Scatter,
    Radar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompareFocus {
    #[default]
    Table,
    Chart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadarPreset {
    Agentic,
    Academic,
    Indexes,
}

impl Default for RadarPreset {
    fn default() -> Self {
        Self::Agentic
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompareSort {
    #[default]
    Intelligence,
    Coding,
    Math,
    Speed,
    Price,
    Name,
}

pub struct CompareApp {
    pub selections: Vec<usize>,          // indices into BenchmarkStore, max 8
    pub mode: CompareMode,
    pub chart_view: ChartView,
    pub focus: CompareFocus,
    pub table_scroll: usize,             // selected row in default mode table
    pub h2h_scroll: usize,              // selected metric row in H2H table
    pub scatter_x: ScatterAxis,
    pub scatter_y: ScatterAxis,
    pub radar_preset: RadarPreset,
    pub sort_column: CompareSort,
    pub sort_descending: bool,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,    // filtered/sorted indices into BenchmarkStore
    pub max_values: HashMap<String, f64>, // metric name -> max value for normalization
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScatterAxis {
    Intelligence,
    Coding,
    Math,
    Speed,
    Price,
}

impl Default for ScatterAxis {
    fn default() -> Self {
        Self::Intelligence
    }
}

impl CompareApp {
    pub fn new() -> Self {
        Self {
            selections: Vec::new(),
            mode: CompareMode::default(),
            chart_view: ChartView::default(),
            focus: CompareFocus::default(),
            table_scroll: 0,
            h2h_scroll: 0,
            scatter_x: ScatterAxis::Price,
            scatter_y: ScatterAxis::default(), // Intelligence
            radar_preset: RadarPreset::default(),
            sort_column: CompareSort::default(),
            sort_descending: true,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            max_values: HashMap::new(),
        }
    }
}
```

Add to `src/tui/mod.rs`:
```rust
pub mod compare_app;
```

**Step 4: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 5: Commit**

```bash
git add src/tui/compare_app.rs src/tui/mod.rs && git commit -m "feat: add CompareApp struct with enums and basic state"
```

---

### Task 3: Wire CompareApp into App and add selection logic

**Files:**
- Modify: `src/tui/app.rs` (App struct, App::new, Message enum)
- Modify: `src/tui/compare_app.rs` (toggle_selection, rebuild)

**Step 1: Write the failing test**

In `src/tui/compare_app.rs` tests:

```rust
#[test]
fn toggle_selection_adds_and_removes() {
    let mut app = CompareApp::new();
    assert!(app.toggle_selection(5));
    assert_eq!(app.selections, vec![5]);
    assert!(app.toggle_selection(3));
    assert_eq!(app.selections, vec![5, 3]);
    // toggle off
    assert!(app.toggle_selection(5));
    assert_eq!(app.selections, vec![3]);
}

#[test]
fn toggle_selection_respects_max() {
    let mut app = CompareApp::new();
    for i in 0..MAX_SELECTIONS {
        assert!(app.toggle_selection(i));
    }
    assert!(!app.toggle_selection(99)); // 9th model rejected
    assert_eq!(app.selections.len(), MAX_SELECTIONS);
}

#[test]
fn clear_selections_empties() {
    let mut app = CompareApp::new();
    app.toggle_selection(1);
    app.toggle_selection(2);
    app.clear_selections();
    assert!(app.selections.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `mise run test`
Expected: FAIL — methods don't exist

**Step 3: Write minimal implementation**

Add to `CompareApp` in `src/tui/compare_app.rs`:

```rust
/// Toggle a model index in/out of selections. Returns false if at max and adding.
pub fn toggle_selection(&mut self, index: usize) -> bool {
    if let Some(pos) = self.selections.iter().position(|&i| i == index) {
        self.selections.remove(pos);
        true
    } else if self.selections.len() < MAX_SELECTIONS {
        self.selections.push(index);
        true
    } else {
        false
    }
}

pub fn clear_selections(&mut self) {
    self.selections.clear();
}
```

Add `compare_app: CompareApp` field to `App` struct in `src/tui/app.rs`.
Initialize in `App::new()`: `compare_app: CompareApp::new()`.

Add Message variants to `src/tui/app.rs`:

```rust
// Compare tab
ToggleCompareSelection,
ClearCompareSelections,
EnterH2H,
ExitH2H,
ToggleH2HChart,
CycleScatterX,
CycleScatterY,
CycleRadarPreset,
CycleCompareSort,
ToggleCompareSortDir,
SwitchCompareFocus,
// Compare tab navigation
NextCompare,
PrevCompare,
SelectFirstCompare,
SelectLastCompare,
PageDownCompare,
PageUpCompare,
ScrollH2HUp,
ScrollH2HDown,
// Cross-tab
MarkForCompare,
```

Wire the simple handlers in `App::update()`:

```rust
Message::ToggleCompareSelection => {
    // handled in mod.rs for status messages
}
Message::ClearCompareSelections => {
    self.compare_app.clear_selections();
}
Message::EnterH2H => {
    if self.compare_app.selections.len() >= 2 {
        self.compare_app.mode = CompareMode::HeadToHead;
    }
}
Message::ExitH2H => {
    self.compare_app.mode = CompareMode::Default;
}
Message::ToggleH2HChart => {
    self.compare_app.chart_view = match self.compare_app.chart_view {
        ChartView::Scatter => ChartView::Radar,
        ChartView::Radar => ChartView::Scatter,
    };
}
Message::SwitchCompareFocus => {
    self.compare_app.focus = match self.compare_app.focus {
        CompareFocus::Table => CompareFocus::Chart,
        CompareFocus::Chart => CompareFocus::Table,
    };
}
```

**Step 4: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: wire CompareApp into App with selection logic and Message variants"
```

---

### Task 4: Add compare table filtering, sorting, and rebuild

**Files:**
- Modify: `src/tui/compare_app.rs`

**Step 1: Write the failing tests**

```rust
use crate::benchmarks::{BenchmarkEntry, BenchmarkStore};

fn make_entry(name: &str, intelligence: Option<f64>, price: Option<f64>) -> BenchmarkEntry {
    let mut entry = BenchmarkEntry {
        id: String::new(),
        name: name.to_string(),
        slug: name.to_lowercase().replace(' ', "-"),
        creator: "test".to_string(),
        creator_id: String::new(),
        creator_name: "Test".to_string(),
        release_date: None,
        intelligence_index: intelligence,
        coding_index: None,
        math_index: None,
        mmlu_pro: None,
        gpqa: None,
        hle: None,
        livecodebench: None,
        scicode: None,
        ifbench: None,
        lcr: None,
        terminalbench_hard: None,
        tau2: None,
        math_500: None,
        aime: None,
        aime_25: None,
        output_tps: None,
        ttft: None,
        ttfat: None,
        price_input: None,
        price_output: None,
        price_blended: price,
    };
    entry
}

#[test]
fn rebuild_populates_filtered_indices() {
    let store = BenchmarkStore::from_entries(vec![
        make_entry("Alpha", Some(90.0), Some(5.0)),
        make_entry("Beta", Some(80.0), Some(3.0)),
        make_entry("Gamma", Some(85.0), Some(4.0)),
    ]);
    let mut app = CompareApp::new();
    app.rebuild(&store);
    assert_eq!(app.filtered_indices.len(), 3);
}

#[test]
fn rebuild_sorts_by_intelligence_descending() {
    let store = BenchmarkStore::from_entries(vec![
        make_entry("Low", Some(70.0), None),
        make_entry("High", Some(95.0), None),
        make_entry("Mid", Some(85.0), None),
    ]);
    let mut app = CompareApp::new();
    app.sort_column = CompareSort::Intelligence;
    app.sort_descending = true;
    app.rebuild(&store);
    // First filtered index should point to "High"
    assert_eq!(store.entries()[app.filtered_indices[0]].name, "High");
    assert_eq!(store.entries()[app.filtered_indices[1]].name, "Mid");
    assert_eq!(store.entries()[app.filtered_indices[2]].name, "Low");
}

#[test]
fn rebuild_computes_max_values() {
    let store = BenchmarkStore::from_entries(vec![
        make_entry("A", Some(90.0), Some(5.0)),
        make_entry("B", Some(80.0), Some(3.0)),
    ]);
    let mut app = CompareApp::new();
    app.rebuild(&store);
    assert_eq!(app.max_values.get("intelligence_index"), Some(&90.0));
    assert_eq!(app.max_values.get("price_blended"), Some(&5.0));
}
```

**Step 2: Run test to verify it fails**

Run: `mise run test`
Expected: FAIL — `rebuild` method doesn't exist

**Step 3: Write minimal implementation**

Add to `CompareApp`:

```rust
pub fn rebuild(&mut self, store: &BenchmarkStore) {
    // Build filtered indices (all entries for now, search filter later)
    self.filtered_indices = (0..store.entries().len())
        .filter(|&i| {
            if self.search_query.is_empty() {
                return true;
            }
            let entry = &store.entries()[i];
            let q = self.search_query.to_lowercase();
            entry.name.to_lowercase().contains(&q)
                || entry.creator_name.to_lowercase().contains(&q)
        })
        .collect();

    // Sort
    self.apply_sort(store);

    // Compute max values for radar normalization
    self.compute_max_values(store);

    // Clamp scroll
    if !self.filtered_indices.is_empty() {
        self.table_scroll = self.table_scroll.min(self.filtered_indices.len() - 1);
    } else {
        self.table_scroll = 0;
    }
}

fn apply_sort(&mut self, store: &BenchmarkStore) {
    let entries = store.entries();
    self.filtered_indices.sort_by(|&a, &b| {
        let ea = &entries[a];
        let eb = &entries[b];
        let ord = match self.sort_column {
            CompareSort::Intelligence => ea.intelligence_index.partial_cmp(&eb.intelligence_index),
            CompareSort::Coding => ea.coding_index.partial_cmp(&eb.coding_index),
            CompareSort::Math => ea.math_index.partial_cmp(&eb.math_index),
            CompareSort::Speed => ea.output_tps.partial_cmp(&eb.output_tps),
            CompareSort::Price => ea.price_blended.partial_cmp(&eb.price_blended),
            CompareSort::Name => Some(ea.name.to_lowercase().cmp(&eb.name.to_lowercase())),
        };
        let ord = ord.unwrap_or(std::cmp::Ordering::Equal);
        if self.sort_descending { ord.reverse() } else { ord }
    });
}

fn compute_max_values(&mut self, store: &BenchmarkStore) {
    self.max_values.clear();
    for entry in store.entries() {
        let metrics: &[(&str, Option<f64>)] = &[
            ("intelligence_index", entry.intelligence_index),
            ("coding_index", entry.coding_index),
            ("math_index", entry.math_index),
            ("livecodebench", entry.livecodebench),
            ("scicode", entry.scicode),
            ("terminalbench_hard", entry.terminalbench_hard),
            ("ifbench", entry.ifbench),
            ("lcr", entry.lcr),
            ("gpqa", entry.gpqa),
            ("mmlu_pro", entry.mmlu_pro),
            ("hle", entry.hle),
            ("math_500", entry.math_500),
            ("aime", entry.aime),
            ("aime_25", entry.aime_25),
            ("tau2", entry.tau2),
            ("output_tps", entry.output_tps),
            ("price_blended", entry.price_blended),
        ];
        for &(name, val) in metrics {
            if let Some(v) = val {
                let max = self.max_values.entry(name.to_string()).or_insert(0.0);
                if v > *max {
                    *max = v;
                }
            }
        }
    }
}
```

**Step 4: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add CompareApp rebuild with filtering, sorting, and max value computation"
```

---

### Task 5: Add navigation methods to CompareApp

**Files:**
- Modify: `src/tui/compare_app.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn navigation_clamps_to_bounds() {
    let store = BenchmarkStore::from_entries(vec![
        make_entry("A", Some(90.0), None),
        make_entry("B", Some(80.0), None),
        make_entry("C", Some(70.0), None),
    ]);
    let mut app = CompareApp::new();
    app.rebuild(&store);

    app.next();
    assert_eq!(app.table_scroll, 1);
    app.next();
    assert_eq!(app.table_scroll, 2);
    app.next(); // should clamp
    assert_eq!(app.table_scroll, 2);
    app.prev();
    assert_eq!(app.table_scroll, 1);
    app.select_first();
    assert_eq!(app.table_scroll, 0);
    app.select_last();
    assert_eq!(app.table_scroll, 2);
}
```

**Step 2: Run test to verify it fails**

Run: `mise run test`
Expected: FAIL — methods don't exist

**Step 3: Write minimal implementation**

```rust
pub fn next(&mut self) {
    if !self.filtered_indices.is_empty() {
        self.table_scroll = (self.table_scroll + 1).min(self.filtered_indices.len() - 1);
    }
}

pub fn prev(&mut self) {
    self.table_scroll = self.table_scroll.saturating_sub(1);
}

pub fn select_first(&mut self) {
    self.table_scroll = 0;
}

pub fn select_last(&mut self) {
    if !self.filtered_indices.is_empty() {
        self.table_scroll = self.filtered_indices.len() - 1;
    }
}

pub fn page_down(&mut self) {
    if !self.filtered_indices.is_empty() {
        self.table_scroll = (self.table_scroll + 10).min(self.filtered_indices.len() - 1);
    }
}

pub fn page_up(&mut self) {
    self.table_scroll = self.table_scroll.saturating_sub(10);
}

pub fn current_entry<'a>(&self, store: &'a BenchmarkStore) -> Option<&'a BenchmarkEntry> {
    self.filtered_indices
        .get(self.table_scroll)
        .and_then(|&i| store.entries().get(i))
}

pub fn cycle_sort(&mut self, store: &BenchmarkStore) {
    self.sort_column = match self.sort_column {
        CompareSort::Intelligence => CompareSort::Coding,
        CompareSort::Coding => CompareSort::Math,
        CompareSort::Math => CompareSort::Speed,
        CompareSort::Speed => CompareSort::Price,
        CompareSort::Price => CompareSort::Name,
        CompareSort::Name => CompareSort::Intelligence,
    };
    self.rebuild(store);
}

pub fn toggle_sort_dir(&mut self, store: &BenchmarkStore) {
    self.sort_descending = !self.sort_descending;
    self.rebuild(store);
}

pub fn cycle_scatter_x(&mut self) {
    self.scatter_x = Self::next_scatter_axis(self.scatter_x);
}

pub fn cycle_scatter_y(&mut self) {
    self.scatter_y = Self::next_scatter_axis(self.scatter_y);
}

fn next_scatter_axis(axis: ScatterAxis) -> ScatterAxis {
    match axis {
        ScatterAxis::Intelligence => ScatterAxis::Coding,
        ScatterAxis::Coding => ScatterAxis::Math,
        ScatterAxis::Math => ScatterAxis::Speed,
        ScatterAxis::Speed => ScatterAxis::Price,
        ScatterAxis::Price => ScatterAxis::Intelligence,
    }
}

pub fn cycle_radar_preset(&mut self) {
    self.radar_preset = match self.radar_preset {
        RadarPreset::Agentic => RadarPreset::Academic,
        RadarPreset::Academic => RadarPreset::Indexes,
        RadarPreset::Indexes => RadarPreset::Agentic,
    };
}
```

**Step 4: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add CompareApp navigation, sort cycling, and axis cycling methods"
```

---

### Task 6: Wire keybindings for Compare tab

**Files:**
- Modify: `src/tui/event.rs` (add `handle_compare_keys`)
- Modify: `src/tui/app.rs` (wire Message handlers in `update()`)

**Step 1: Write the implementation**

Add `handle_compare_keys` in `src/tui/event.rs`, following the pattern of `handle_benchmarks_keys`:

```rust
fn handle_compare_keys(app: &App, code: KeyCode, modifiers: KeyModifiers) -> Option<Message> {
    use super::compare_app::{CompareMode, CompareFocus};
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let mode = app.compare_app.mode;
    let focus = app.compare_app.focus;

    match mode {
        CompareMode::Default => match code {
            // Navigation
            KeyCode::Char('j') | KeyCode::Down => Some(Message::NextCompare),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::PrevCompare),
            KeyCode::Char('g') => Some(Message::SelectFirstCompare),
            KeyCode::Char('G') => Some(Message::SelectLastCompare),
            KeyCode::Char('d') if ctrl => Some(Message::PageDownCompare),
            KeyCode::Char('u') if ctrl => Some(Message::PageUpCompare),
            KeyCode::PageDown => Some(Message::PageDownCompare),
            KeyCode::PageUp => Some(Message::PageUpCompare),
            // Selection
            KeyCode::Char(' ') => Some(Message::ToggleCompareSelection),
            KeyCode::Char('c') => Some(Message::ClearCompareSelections),
            KeyCode::Enter => Some(Message::EnterH2H),
            // Sort
            KeyCode::Char('s') => Some(Message::CycleCompareSort),
            KeyCode::Char('S') => Some(Message::ToggleCompareSortDir),
            // Scatter axes
            KeyCode::Char('x') => Some(Message::CycleScatterX),
            KeyCode::Char('y') => Some(Message::CycleScatterY),
            // Focus
            KeyCode::Tab | KeyCode::BackTab => Some(Message::SwitchCompareFocus),
            KeyCode::Char('h') | KeyCode::Left => Some(Message::SwitchCompareFocus),
            KeyCode::Char('l') | KeyCode::Right => Some(Message::SwitchCompareFocus),
            // Search
            KeyCode::Char('/') => Some(Message::EnterSearch),
            KeyCode::Esc => Some(Message::ClearSearch),
            _ => None,
        },
        CompareMode::HeadToHead => match code {
            // Scroll metric rows
            KeyCode::Char('j') | KeyCode::Down => Some(Message::ScrollH2HDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Message::ScrollH2HUp),
            // Toggle chart view
            KeyCode::Char('v') => Some(Message::ToggleH2HChart),
            // Radar presets
            KeyCode::Char('a') => Some(Message::CycleRadarPreset),
            // Open AA
            KeyCode::Char('o') => Some(Message::OpenBenchmarkUrl),
            // Exit H2H
            KeyCode::Esc | KeyCode::Enter => Some(Message::ExitH2H),
            _ => None,
        },
    }
}
```

Update `handle_normal_mode` tab dispatch to include:
```rust
Tab::Compare => handle_compare_keys(app, code, modifiers),
```

Wire remaining Message handlers in `App::update()`:

```rust
Message::NextCompare => { self.compare_app.next(); }
Message::PrevCompare => { self.compare_app.prev(); }
Message::SelectFirstCompare => { self.compare_app.select_first(); }
Message::SelectLastCompare => { self.compare_app.select_last(); }
Message::PageDownCompare => { self.compare_app.page_down(); }
Message::PageUpCompare => { self.compare_app.page_up(); }
Message::CycleCompareSort => {
    self.compare_app.cycle_sort(&self.benchmark_store);
}
Message::ToggleCompareSortDir => {
    self.compare_app.toggle_sort_dir(&self.benchmark_store);
}
Message::CycleScatterX => { self.compare_app.cycle_scatter_x(); }
Message::CycleScatterY => { self.compare_app.cycle_scatter_y(); }
Message::CycleRadarPreset => { self.compare_app.cycle_radar_preset(); }
Message::ScrollH2HUp => { self.compare_app.h2h_scroll = self.compare_app.h2h_scroll.saturating_sub(1); }
Message::ScrollH2HDown => { self.compare_app.h2h_scroll += 1; }
```

Also add `Compare` arm to search dispatch blocks (`SearchInput`, `SearchBackspace`, `ClearSearch`):

```rust
Tab::Compare => {
    self.compare_app.search_query.push(c);
    self.compare_app.rebuild(&self.benchmark_store);
}
```

Wire `MarkForCompare` for use from the Benchmarks tab — add `KeyCode::Char('m')` to `handle_benchmarks_keys`:
```rust
KeyCode::Char('m') => Some(Message::MarkForCompare),
```

Handle `MarkForCompare` in `App::update()` — get current benchmark entry index and toggle it.

Also wire `CompareApp::rebuild()` into `BenchmarkDataReceived`:
```rust
self.compare_app.rebuild(&self.benchmark_store);
```

**Step 2: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: wire Compare tab keybindings and Message handlers"
```

---

### Task 7: Implement default mode UI — ranked table

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Write the implementation**

Replace the stub `draw_compare_main` with a proper implementation. This task focuses on the top pane (ranked table). The bottom pane (scatter) will be a placeholder for now.

```rust
fn draw_compare_main(f: &mut Frame, area: Rect, app: &mut App) {
    match app.compare_app.mode {
        CompareMode::Default => draw_compare_default(f, area, app),
        CompareMode::HeadToHead => draw_compare_h2h(f, area, app),
    }
}

fn draw_compare_default(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_compare_table(f, chunks[0], app);
    draw_scatter_placeholder(f, chunks[1], app);
}
```

`draw_compare_table` renders a `List` widget similar to `draw_benchmark_list`:
- Title shows selection count: `" Compare (3/8 selected) ↓Intelligence "`
- Columns: Name, Intelligence, Coding, Math, Speed, Price
- Selected models get a colored marker (e.g., colored `●` prefix matching their compare color)
- Current row highlighted with `>` caret
- Uses `app.compare_app.filtered_indices` for row data
- Column widths follow existing patterns from `draw_benchmark_list`

For the scatter placeholder, just render a bordered `Paragraph` with `"Scatter plot (coming next)"`.

Reference `draw_benchmark_list` (lines 1251-1366 of `ui.rs`) closely for formatting patterns (`fmt_col_idx`, `fmt_col_price`, `fmt_speed` functions).

**Step 2: Run the TUI to verify visually**

Run: `mise run run`
Navigate to Compare tab with `]`, verify the table renders with model data.

**Step 3: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: implement Compare tab default mode ranked table"
```

---

### Task 8: Implement scatter plot rendering

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Write the implementation**

Replace `draw_scatter_placeholder` with `draw_scatter`:

```rust
fn draw_scatter(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::{Axis, Chart, Dataset, GraphType};
    use ratatui::symbols::Marker;

    let entries = app.benchmark_store.entries();
    if entries.is_empty() {
        let block = Block::bordered().title(" Scatter ");
        f.render_widget(block, area);
        return;
    }

    let x_extract = scatter_extract_fn(app.compare_app.scatter_x);
    let y_extract = scatter_extract_fn(app.compare_app.scatter_y);

    // Collect all points and compute bounds
    let mut all_points: Vec<(f64, f64)> = Vec::new();
    let mut selected_points: Vec<Vec<(f64, f64)>> = vec![Vec::new(); app.compare_app.selections.len()];

    for (i, entry) in entries.iter().enumerate() {
        if let (Some(x), Some(y)) = (x_extract(entry), y_extract(entry)) {
            all_points.push((x, y));
            if let Some(sel_idx) = app.compare_app.selections.iter().position(|&s| s == i) {
                selected_points[sel_idx].push((x, y));
            }
        }
    }

    // Compute bounds with 5% padding
    // ... (standard min/max with padding)

    // Build datasets
    let mut datasets = vec![
        Dataset::default()
            .name("all")
            .marker(Marker::Braille)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Color::DarkGray))
            .data(&all_points),
    ];

    // Add selected model datasets with distinct colors
    let colors = compare_colors();
    for (i, points) in selected_points.iter().enumerate() {
        if !points.is_empty() {
            let name = app.compare_app.selections.get(i)
                .and_then(|&idx| entries.get(idx))
                .map(|e| e.name.as_str())
                .unwrap_or("?");
            datasets.push(
                Dataset::default()
                    .name(name)
                    .marker(Marker::Dot)
                    .graph_type(GraphType::Scatter)
                    .style(Style::default().fg(colors[i % colors.len()]))
                    .data(points),
            );
        }
    }

    let x_label = scatter_axis_label(app.compare_app.scatter_x);
    let y_label = scatter_axis_label(app.compare_app.scatter_y);

    let chart = Chart::new(datasets)
        .block(Block::bordered().title(format!(" {y_label} vs {x_label} ")))
        .x_axis(Axis::default().title(x_label).bounds([x_min, x_max]))
        .y_axis(Axis::default().title(y_label).bounds([y_min, y_max]));

    f.render_widget(chart, area);
}

fn compare_colors() -> &'static [Color] {
    &[Color::Cyan, Color::Yellow, Color::Green, Color::Magenta, Color::Red, Color::Blue, Color::LightCyan, Color::LightYellow]
}

fn scatter_extract_fn(axis: ScatterAxis) -> fn(&BenchmarkEntry) -> Option<f64> {
    match axis {
        ScatterAxis::Intelligence => |e| e.intelligence_index,
        ScatterAxis::Coding => |e| e.coding_index,
        ScatterAxis::Math => |e| e.math_index,
        ScatterAxis::Speed => |e| e.output_tps,
        ScatterAxis::Price => |e| e.price_blended,
    }
}

fn scatter_axis_label(axis: ScatterAxis) -> &'static str {
    match axis {
        ScatterAxis::Intelligence => "Intelligence",
        ScatterAxis::Coding => "Coding",
        ScatterAxis::Math => "Math",
        ScatterAxis::Speed => "Speed (tok/s)",
        ScatterAxis::Price => "Price ($/M)",
    }
}
```

**Step 2: Run the TUI to verify visually**

Run: `mise run run`
Navigate to Compare tab. Verify scatter plot renders with dots. Select models with `Space` and verify they appear in distinct colors.

**Step 3: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: implement scatter plot with Braille markers and model highlighting"
```

---

### Task 9: Implement H2H ranked comparison table

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Write the implementation**

Implement `draw_compare_h2h`:

```rust
fn draw_compare_h2h(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_h2h_table(f, chunks[0], app);
    match app.compare_app.chart_view {
        ChartView::Scatter => draw_scatter(f, chunks[1], app),
        ChartView::Radar => draw_radar_placeholder(f, chunks[1], app),
    }
}
```

`draw_h2h_table` renders the side-by-side comparison:
- Columns: one "Metric" label column + one column per selected model
- Column headers: model names, each styled with their compare color
- Metric rows grouped with section headers (use `push_section_header` pattern):
  - `─── Indexes ───`
  - Intelligence, Coding, Math
  - `─── Benchmarks ───`
  - GPQA, MMLU-Pro, HLE, LiveCodeBench, SciCode, IFBench, TerminalBench, Tau2, LCR, MATH-500, AIME, AIME'25
  - `─── Performance ───`
  - Speed, TTFT, TTFAT
  - `─── Pricing ───`
  - Input, Output, Blended
- Each cell: formatted value + rank (e.g., `89.2 #1`)
- Winner per row: Bold + Cyan
- Missing values: `—` in DarkGray

For each metric row, compute the rank among selected models:
```rust
fn rank_values(values: &[Option<f64>], higher_is_better: bool) -> Vec<Option<usize>> {
    // Sort indices by value, assign ranks 1..N, handle None as unranked
}
```

Render as a `List` or `Paragraph` widget with `scroll((h2h_scroll, 0))`.

**Step 2: Run the TUI to verify**

Run: `mise run run`
Select 2-3 models, press `Enter` to enter H2H. Verify the comparison table renders with correct rankings and winner highlighting.

**Step 3: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: implement H2H ranked comparison table with winner highlighting"
```

---

### Task 10: Implement radar chart renderer

**Files:**
- Create: `src/tui/radar.rs`
- Modify: `src/tui/mod.rs` (add module)
- Modify: `src/tui/ui.rs` (replace `draw_radar_placeholder`)

**Step 1: Write the failing tests**

In `src/tui/radar.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn spoke_angles_start_at_top() {
        let angles = spoke_angles(6);
        // First spoke should point up (-PI/2)
        assert!((angles[0] - (-PI / 2.0)).abs() < 1e-10);
    }

    #[test]
    fn spoke_angles_evenly_spaced() {
        let angles = spoke_angles(4);
        let expected_gap = 2.0 * PI / 4.0;
        for i in 0..3 {
            let gap = angles[i + 1] - angles[i];
            assert!((gap - expected_gap).abs() < 1e-10);
        }
    }

    #[test]
    fn polygon_vertex_at_max_reaches_radius() {
        let angles = spoke_angles(4);
        let values = vec![1.0, 0.5, 1.0, 0.5]; // normalized 0-1
        let vertices = polygon_vertices(50.0, 50.0, 40.0, &angles, &values);
        // First vertex (value=1.0, angle=-PI/2) should be at (50, 50-40) = (50, 10)
        assert!((vertices[0].0 - 50.0).abs() < 1e-10);
        assert!((vertices[0].1 - 10.0).abs() < 1e-10);
    }

    #[test]
    fn polygon_vertex_at_zero_stays_at_center() {
        let angles = spoke_angles(4);
        let values = vec![0.0, 0.0, 0.0, 0.0];
        let vertices = polygon_vertices(50.0, 50.0, 40.0, &angles, &values);
        for &(x, y) in &vertices {
            assert!((x - 50.0).abs() < 1e-10);
            assert!((y - 50.0).abs() < 1e-10);
        }
    }
}
```

**Step 2: Run test to verify it fails**

Run: `mise run test`
Expected: FAIL — module doesn't exist

**Step 3: Write minimal implementation**

Create `src/tui/radar.rs`:

```rust
use std::f64::consts::PI;

/// Compute spoke angles for N axes, starting at top (−π/2), going clockwise.
pub fn spoke_angles(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| -PI / 2.0 + 2.0 * PI * i as f64 / n as f64)
        .collect()
}

/// Compute polygon vertices given center, radius, angles, and normalized values (0-1).
pub fn polygon_vertices(
    cx: f64,
    cy: f64,
    radius: f64,
    angles: &[f64],
    values: &[f64],
) -> Vec<(f64, f64)> {
    angles
        .iter()
        .zip(values.iter())
        .map(|(&angle, &value)| {
            let r = radius * value;
            (cx + r * angle.cos(), cy + r * angle.sin())
        })
        .collect()
}
```

Add `pub mod radar;` to `src/tui/mod.rs`.

**Step 4: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 5: Commit**

```bash
git add src/tui/radar.rs src/tui/mod.rs && git commit -m "feat: add radar chart math (spoke angles, polygon vertices)"
```

---

### Task 11: Implement radar chart Canvas rendering

**Files:**
- Modify: `src/tui/radar.rs` (add preset axis definitions)
- Modify: `src/tui/ui.rs` (replace `draw_radar_placeholder`)

**Step 1: Write the implementation**

Add to `src/tui/radar.rs` — axis label/extract definitions for each preset:

```rust
use crate::benchmarks::BenchmarkEntry;

pub struct RadarAxis {
    pub label: &'static str,
    pub key: &'static str, // matches max_values key
    pub extract: fn(&BenchmarkEntry) -> Option<f64>,
}

pub fn axes_for_preset(preset: RadarPreset) -> Vec<RadarAxis> {
    match preset {
        RadarPreset::Agentic => vec![
            RadarAxis { label: "Coding", key: "coding_index", extract: |e| e.coding_index },
            RadarAxis { label: "LiveCode", key: "livecodebench", extract: |e| e.livecodebench },
            RadarAxis { label: "SciCode", key: "scicode", extract: |e| e.scicode },
            RadarAxis { label: "Terminal", key: "terminalbench_hard", extract: |e| e.terminalbench_hard },
            RadarAxis { label: "IFBench", key: "ifbench", extract: |e| e.ifbench },
            RadarAxis { label: "LCR", key: "lcr", extract: |e| e.lcr },
        ],
        RadarPreset::Academic => vec![
            RadarAxis { label: "GPQA", key: "gpqa", extract: |e| e.gpqa },
            RadarAxis { label: "MMLU-Pro", key: "mmlu_pro", extract: |e| e.mmlu_pro },
            RadarAxis { label: "HLE", key: "hle", extract: |e| e.hle },
            RadarAxis { label: "MATH-500", key: "math_500", extract: |e| e.math_500 },
            RadarAxis { label: "AIME", key: "aime", extract: |e| e.aime },
            RadarAxis { label: "AIME'25", key: "aime_25", extract: |e| e.aime_25 },
        ],
        RadarPreset::Indexes => vec![
            RadarAxis { label: "Intel", key: "intelligence_index", extract: |e| e.intelligence_index },
            RadarAxis { label: "Coding", key: "coding_index", extract: |e| e.coding_index },
            RadarAxis { label: "Math", key: "math_index", extract: |e| e.math_index },
        ],
    }
}
```

Implement `draw_radar` in `src/tui/ui.rs`:

```rust
fn draw_radar(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::canvas::{Canvas, Line, Points};
    use crate::tui::radar::{spoke_angles, polygon_vertices, axes_for_preset};

    let axes = axes_for_preset(app.compare_app.radar_preset);
    let n = axes.len();
    if n < 3 || app.compare_app.selections.is_empty() {
        let block = Block::bordered().title(" Radar ");
        f.render_widget(block, area);
        return;
    }

    let entries = app.benchmark_store.entries();
    let angles = spoke_angles(n);
    let colors = compare_colors();
    let preset_label = match app.compare_app.radar_preset {
        RadarPreset::Agentic => "Agentic",
        RadarPreset::Academic => "Academic",
        RadarPreset::Indexes => "Indexes",
    };

    // Canvas coordinate system: -60..60 both axes, center at (0,0)
    let radius = 45.0;
    let canvas = Canvas::default()
        .block(Block::bordered().title(format!(" Radar [{preset_label}] ")))
        .x_bounds([-60.0, 60.0])
        .y_bounds([-60.0, 60.0])
        .marker(ratatui::symbols::Marker::Braille)
        .paint(move |ctx| {
            // Draw axis lines from center to edge
            for &angle in &angles {
                let ex = radius * angle.cos();
                let ey = radius * angle.sin();
                ctx.draw(&Line { x1: 0.0, y1: 0.0, x2: ex, y2: ey, color: Color::DarkGray });
            }

            // Draw axis labels
            for (i, axis) in axes.iter().enumerate() {
                let lx = (radius + 8.0) * angles[i].cos();
                let ly = (radius + 4.0) * angles[i].sin();
                ctx.print(lx, ly, Span::styled(axis.label, Style::default().fg(Color::White)));
            }

            // Draw model polygons
            for (sel_idx, &store_idx) in app.compare_app.selections.iter().enumerate() {
                if let Some(entry) = entries.get(store_idx) {
                    let values: Vec<f64> = axes.iter().map(|ax| {
                        let val = (ax.extract)(entry).unwrap_or(0.0);
                        let max = app.compare_app.max_values.get(ax.key).copied().unwrap_or(1.0);
                        if max > 0.0 { val / max } else { 0.0 }
                    }).collect();

                    let verts = polygon_vertices(0.0, 0.0, radius, &angles, &values);
                    let color = colors[sel_idx % colors.len()];

                    // Draw polygon edges
                    for i in 0..verts.len() {
                        let j = (i + 1) % verts.len();
                        ctx.draw(&Line {
                            x1: verts[i].0, y1: verts[i].1,
                            x2: verts[j].0, y2: verts[j].1,
                            color,
                        });
                    }
                }
            }
        });

    f.render_widget(canvas, area);
}
```

Note: The exact Canvas API may need adjustments based on ratatui version (0.29). The `paint` closure captures references — may need to clone data before the closure to satisfy the borrow checker. Extract `selections`, `entries` data, and `max_values` into local vecs before the closure.

**Step 2: Run the TUI to verify**

Run: `mise run run`
Select 2-3 models, enter H2H, verify radar chart renders with overlaid colored polygons. Press `a` to cycle presets.

**Step 3: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: implement radar chart Canvas rendering with Braille markers"
```

---

### Task 12: Add footer and help popup for Compare tab

**Files:**
- Modify: `src/tui/ui.rs` (draw_footer, draw_help_popup)

**Step 1: Write the implementation**

Update `draw_footer` to show Compare-specific keybinding hints:

Default mode: `" Space select  Enter H2H  s sort  x/y axes  c clear  / search "`
H2H mode: `" v chart  a preset  j/k scroll  Esc back "`

Update `draw_help_popup` to include a `Tab::Compare` section listing all keybindings.

**Step 2: Run the TUI to verify**

Run: `mise run run`
Verify footer updates when on Compare tab. Press `?` to open help popup and verify Compare keybindings are listed.

**Step 3: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: add Compare tab footer hints and help popup section"
```

---

### Task 13: Add selection indicator to Benchmarks tab

**Files:**
- Modify: `src/tui/ui.rs` (`draw_benchmark_list`)

**Step 1: Write the implementation**

In `draw_benchmark_list`, check if each entry's store index is in `app.compare_app.selections`. If so, prepend a colored `●` (or `*`) to the model name, using the model's assigned compare color.

This gives visual feedback on the Benchmarks tab about which models are marked for comparison.

**Step 2: Run the TUI to verify**

Run: `mise run run`
On Benchmarks tab, press `m` to mark a model. Verify a colored dot appears. Switch to Compare tab, verify model is selected. Switch back, verify dot persists.

**Step 3: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: show compare selection indicator on Benchmarks tab"
```

---

### Task 14: Handle ToggleCompareSelection and MarkForCompare in mod.rs

**Files:**
- Modify: `src/tui/mod.rs` (event loop, status messages for selection)

**Step 1: Write the implementation**

In the event dispatch section of `run_app` (similar to how `CopyBenchmarkName` is handled in mod.rs for status messages):

```rust
app::Message::ToggleCompareSelection => {
    if let Some(&idx) = app.compare_app.filtered_indices.get(app.compare_app.table_scroll) {
        if app.compare_app.toggle_selection(idx) {
            let name = app.benchmark_store.entries().get(idx)
                .map(|e| e.name.as_str()).unwrap_or("?");
            let count = app.compare_app.selections.len();
            if app.compare_app.selections.contains(&idx) {
                app.set_status(format!("Added {name} ({count}/{MAX_SELECTIONS})"));
            } else {
                app.set_status(format!("Removed {name}"));
            }
        } else {
            app.set_status(format!("Max {} models selected", MAX_SELECTIONS));
        }
        last_status_time = Some(std::time::Instant::now());
    }
}
app::Message::MarkForCompare => {
    if let Some(entry) = app.benchmarks_app.current_entry(&app.benchmark_store) {
        // Find the store index for this entry
        if let Some(idx) = app.benchmarks_app.filtered_indices.get(app.benchmarks_app.selected) {
            if app.compare_app.toggle_selection(*idx) {
                let count = app.compare_app.selections.len();
                if app.compare_app.selections.contains(idx) {
                    app.set_status(format!("Marked {} for compare ({count}/{MAX_SELECTIONS})", entry.name));
                } else {
                    app.set_status(format!("Unmarked {} from compare", entry.name));
                }
            } else {
                app.set_status(format!("Max {} models selected", MAX_SELECTIONS));
            }
            last_status_time = Some(std::time::Instant::now());
        }
    }
}
```

Note: `ToggleCompareSelection` and `MarkForCompare` need to be intercepted in `mod.rs` (before `app.update()`) because they need access to status message timing. Follow the pattern used by `CopyBenchmarkName` / `OpenBenchmarkUrl`.

**Step 2: Run the TUI to verify**

Run: `mise run run`
On Benchmarks tab, press `m` — verify status message shows. On Compare tab, press `Space` — verify status message shows. Try adding 9th model — verify "Max 8" message.

**Step 3: Run tests and clippy**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A && git commit -m "feat: handle compare selection with status messages"
```

---

### Task 15: Final integration testing and polish

**Files:**
- All modified files

**Step 1: Full manual testing**

Run: `mise run run`

Test the following flows:
1. Navigate to Compare tab — verify ranked table + scatter plot render
2. Sort with `s`/`S` — verify table reorders
3. Search with `/` — verify filtering works
4. Cycle scatter axes with `x`/`y` — verify chart updates
5. Select 3+ models with `Space` — verify colored dots on scatter
6. Press `Enter` — verify H2H mode with comparison table + radar
7. Scroll H2H table with `j`/`k`
8. Toggle chart with `v` — verify scatter ↔ radar
9. Cycle radar presets with `a` — verify axes change
10. Press `Esc` — verify return to default mode
11. Go to Benchmarks tab, `m` to mark — verify indicator shows
12. Return to Compare tab — verify selection persisted
13. Press `c` — verify all selections cleared
14. Press `?` — verify help popup has Compare section

**Step 2: Run full check sequence**

Run: `mise run fmt && mise run clippy && mise run test`
Expected: PASS — all clean

**Step 3: Commit any polish fixes**

```bash
git add -A && git commit -m "chore: polish Compare tab integration"
```
