# Cross-Tab Controls and Interaction Audit

## Role in the doc stack
This is a **supporting evidence** document.

Use it as input to the canonical docs:
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`
- `docs/status-tab-redesign-spec.md`


## Scope
Focused audit of control surfaces and interaction consistency across:
- Models
- Agents
- Benchmarks
- Status

This audit covers:
- keybindings
- sorts
- filters
- toggles
- focus changes
- mode changes
- footer/help discoverability
- cross-tab interaction consistency

## Inputs reviewed
- `src/tui/event.rs`
- `src/tui/app.rs`
- `src/tui/ui.rs`
- `src/tui/status_app.rs`
- `src/tui/agents_app.rs`
- `src/tui/benchmarks_app.rs`
- `docs/ui-design-guide.md`

## High-level findings
The app has a strong base interaction language:
- Vim-style movement (`j/k/g/G`, `Ctrl-d/u`, `PageUp/PageDown`)
- `/` for search
- `Tab` / `h` / `l` for focus changes in multi-panel tabs
- footer/help as the main discoverability surfaces

The main cross-tab problem is not lack of functionality; it is **inconsistent disclosure**. Some tabs expose their controls and modes clearly in titles/footers/help, while others have active capabilities that are under-advertised or named inconsistently.

## Interaction matrix

| Tab | Primary navigation | Search | Focus switch | Sort | Filters/toggles | Modes/views |
|-----|--------------------|--------|--------------|------|-----------------|------------|
| Models | `j/k/g/G`, paging in focused rail/list | `/` | panel focus in app shell | `s` / `S` via sort controls | numeric filter toggles + category/group toggles | stable browse layout |
| Agents | `j/k/g/G`, paging in list | `/` | list/details | cycle sort | filter toggles `1/2/3`, picker workflows | stable list/details |
| Benchmarks | `j/k/g/G`, paging by focused panel | `/` | creator/list/compare focus | rich quick-sort + picker | source/reasoning/grouping toggles | browse mode vs compare mode, H2H/Scatter/Radar |
| Status | `j/k/g/G`, paging in list | `/` | `Tab` / `h` / `l` | none | `c` comp-view toggle | list/details focus, summary/expanded comp view |

## Shared strengths
1. **Core movement is fairly consistent**
   - `j/k/g/G`, paging, search, and focus movement are reused across tabs (`src/tui/event.rs:79-107`, `138-229`, `255-307`, `376-412`).
2. **Footer and help are the intended control-discovery surfaces**
   - all tabs rely on footer/help rather than inline body hints (`src/tui/ui.rs:3200-3302`, `3375-3799`).
3. **Benchmarks is the strongest model for explicit mode surfacing**
   - compare mode and H2H/Scatter/Radar are clearly named and visible (`src/tui/ui.rs:2013-2103`).

## Cross-tab drift and issues

### 1. Status has an undocumented live view toggle
- `event.rs` binds `c` to `CycleCompView` for Status (`src/tui/event.rs:412`).
- `StatusApp` still has `CompView::{Summary, Expanded}` state (`src/tui/status_app.rs:21-42`).
- but Status footer/help do **not** mention `c` (`src/tui/ui.rs:3292-3302`, `3720-3752`).

**Why this matters**
A repeat user cannot reliably discover or remember a mode that exists in code but is absent from footer/help.

### 2. Status under-advertises navigation compared with other tabs
- Status supports full list navigation, paging, and focus movement in `event.rs` (`src/tui/event.rs:376-412`)
- but the footer only advertises: search, focus, status page, refresh (`src/tui/ui.rs:3292-3302`)
- and the help block is minimal compared with richer guidance elsewhere (`src/tui/ui.rs:3720-3752`)

**Why this matters**
The control surface is less learnable than the actual feature set.

### 3. Agents has dormant/hidden IA in filters/categories
- `agents_app.rs` supports category-ish structures and filters not fully surfaced as primary UX (`src/tui/agents_app.rs:37-66`, `209-250`, `500-520`)
- worker-4 also flagged dormant category IA and tracked-only discovery friction

**Why this matters**
The tab’s control model suggests richer segmentation than the user can cleanly access.

### 4. Benchmarks is the most explicit, but also the most complex
- quick-sort, sort picker, creator grouping, reasoning/source filters, compare-mode subviews all exist and are surfaced (`src/tui/event.rs:245-367`, `src/tui/ui.rs:2013-2103`, `4896-4934`)

**What it does well**
It proves that complex controls can still feel coherent when modes and views are explicitly named.

### 5. Models is efficient, but some controls are terse rather than obvious
- category/group toggles and compact label rows are fast for repeat use (`src/tui/ui.rs:832-857`)
- but they favor expert familiarity over first-use clarity

## Discoverability audit

### Strongest discoverability
- Benchmarks: best explicit mode/view naming
- Agents: stable list/details shell keeps control context understandable

### Weakest discoverability
- Status: hidden `c` mode toggle and under-advertised navigation
- Agents: discovery/onboarding around tracked vs untracked agents is weaker than it should be

## Consistency recommendations
1. **Every active mode toggle must appear in footer/help**
   - Status must surface `c` if comp-view remains.
2. **If a tab supports the common movement model, footer/help should advertise it consistently enough for repeat users**
   - especially search, focus, paging, and mode changes.
3. **Use explicit names for alternate views**
   - Benchmarks is the template; Status should not hide expanded/summary state behind invisible or weakly labeled controls.
4. **Do not keep dormant interaction models in the code/UI contract**
   - either surface them clearly or remove/simplify them.
5. **Footer/help should reflect the actual interaction surface, not a subset chosen opportunistically**.

## Recommendations for Status specifically
1. Decide whether `CompView` survives the redesign.
2. If it survives, expose it clearly in footer/help and section naming.
3. If it does not survive, remove the hidden toggle and simplify the interaction model.
4. Keep Status controls aligned with the app-wide movement/search/focus language.

## Acceptance checks for future redesign work
- no hidden active toggle remains undocumented in footer/help
- repeated users can discover movement, focus, and view changes without rereading source
- alternate views/modes are explicitly named
- controls shown in footer/help match actual code behavior