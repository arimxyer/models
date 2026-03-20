# Models Tab Lane Audit

## Role in the doc stack
This is a **supporting evidence** document.

Use it as input to the canonical docs:
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`
- `docs/status-tab-redesign-spec.md`


## Scope
Focused lane audit for the Models tab before more Status-tab redesign work.

Inputs reviewed:
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`
- `src/tui/ui.rs`
- `Cargo.toml`

Framework note:
- this repo is a Rust `ratatui`/`crossterm` TUI, not a web frontend; there is no `package.json` in the workspace.

## Method
### Pass 1: shell and ownership audit
Questions:
- what makes the Models tab feel like the clearest tab in the app?
- which panel responsibilities stay stable across selection changes?
- which parts are reusable for other tabs?

### Pass 2: scan-path and labeling audit
Questions:
- what can a first-time user understand in one left-to-right pass?
- which labels are domain nouns vs implementation shorthand?
- where does compactness help or hurt clarity?

### Pass 3: portability audit for future tab work
Questions:
- which Models patterns should be treated as product-shell defaults?
- which Models-specific choices should not be copied blindly into Status?

## Code anchors
- Three-column shell: `src/tui/ui.rs:87-100`
- Provider summary block: `src/tui/ui.rs:704-773`
- Provider rail: `src/tui/ui.rs:811-910`
- Model list title and column header: `src/tui/ui.rs:913-1045`
- Model detail structure: `src/tui/ui.rs:1669-2005`

## Findings

### Lane A: shell architecture
Key findings:
- The Models tab has the clearest structural contract in the app: narrow navigation rail, dense selection list, explanatory detail surface (`src/tui/ui.rs:87-100`).
- Left-to-right responsibility never changes: provider rail narrows the universe, model list selects the object, right column explains the selection.
- The right column is split into a small provider summary plus the main model detail panel (`src/tui/ui.rs:776-808`). This works because the upper block is short, stable, and subordinate to the main detail panel.

Why it works:
1. stable spatial memory
2. concrete panel identities
3. no overloaded mixed-purpose body
4. metadata is close to the object it describes

### Lane B: scan path and labeling
Key findings:
- The provider rail is navigation-first and stays lightweight even when grouped by category (`src/tui/ui.rs:819-910`).
- The model list title does useful work: provider context, count, search state, filters, and sort state are all visible in the title instead of leaking into the body (`src/tui/ui.rs:943-981`).
- The detail view uses concrete section names: `Capabilities`, `Pricing`, `Limits`, `Modalities`, `Dates` (`src/tui/ui.rs:1789-1956`). This is the strongest example of domain naming in the repo.
- The detail surface reads top-to-bottom as one information story: identity, capabilities, economics, limits, modalities, dates.

Minor clarity debt:
- `Provider` is slightly generic as the title of the upper-right summary box; the content is clear, but the title is weaker than the section names below (`src/tui/ui.rs:1636-1638`).
- `RTFO` is compact and familiar to repeat users, but it is insider shorthand rather than a plain-language label (`src/tui/ui.rs:985-1045`).
- The `[5] Cat` / `[6] Grp` row is efficient but terse; it favors expert speed over first-use readability (`src/tui/ui.rs:832-857`).

### Lane C: portability to Status and other tabs
High-confidence defaults worth standardizing:
1. keep the navigation rail narrow and obviously navigational
2. put counts, search, and sort state in titles before inventing extra body chrome
3. use domain nouns for sections, not implementation buckets
4. keep one primary reading order in the detail surface
5. let small secondary summary blocks exist only when their responsibility is fixed and narrow

What should not be copied blindly into Status:
- The exact stacked-right-column split. Models can support it because provider metadata is tiny and static; Status has more volatile operational content and is more likely to drift if split into too many small boxes.
- The `RTFO` shorthand. Status should avoid insider abbreviations because it serves broader operational scanning.
- The terse toggle row language. Status should bias toward clearer field semantics over control density.

## Consolidated diagnosis
The Models tab feels strongest because it behaves like a disciplined product surface, not a custom experiment. Its main strengths are not visual flair; they are structural:
- fixed left-to-right ownership
- concrete titles
- explicit metadata placement
- one durable scan path

This is why Models is the right reference tab for Status shell discipline, even though Status should not copy every micro-pattern.

## Recommendations for the shared guide
Add these as product-shell defaults when future tab work is reviewed:
1. Use the Models tab as the baseline example for navigation rail width and responsibility.
2. Prefer title-level state exposure before adding nested helper text or dashboard chrome.
3. Allow stacked right-column surfaces only when the upper panel has one fixed, narrow job.
4. Treat concrete section headers as mandatory for long-form details.
5. Flag insider shorthand during review unless it is already established product vocabulary.

## Acceptance checks for future redesign work
When a tab claims to be aligned with Models-quality shell discipline, verify:
- the left rail is still navigation-first
- the middle/list panel title carries count and state cleanly
- the right side has one dominant reading order
- section titles are domain nouns
- repeated use does not require relearning where metadata moved