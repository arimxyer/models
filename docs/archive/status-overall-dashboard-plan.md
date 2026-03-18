# Status Overall Dashboard Plan

## Goal

Rework the Status tab's Overall view into a clearer dashboard board that separates issue classes, preserves drill-in usability, and better highlights provider-authored summaries without turning the UI into a noisy diagnostics panel.

## Current Problems

- The current `Attention Now` panel mixes formal incidents and component-only degradation into one scrollable report.
- The current Overall view has one shared detail scroll region, so visual grouping does not match interaction grouping.
- Provider summaries are available in the model but are not yet a first-class part of the Overall issue presentation.
- Maintenance is already secondary in semantics, but it still shares the same narrative container as more urgent issues.

## Target Layout

Keep the top summary strip as `Overall Status`, then render a responsive board below it.

### Wide Layout

- Left column: `Active Incidents`
- Right column, top: `Service Degradation`
- Right column, bottom: `Maintenance Outlook`

Recommended split:

- Board columns: roughly `60/40`
- Right column rows: roughly even split, with maintenance allowed to collapse away when empty

### Narrow Layout

Collapse into a vertical stack while preserving the same panel identities:

- `Active Incidents`
- `Service Degradation`
- `Maintenance Outlook` (only when non-empty)

## Panel Visibility Rules

- `Active Incidents`: always visible, even when empty
- `Service Degradation`: always visible, even when empty
- `Maintenance Outlook`: hidden entirely when empty

When `Active Incidents` or `Service Degradation` are empty, show calm empty-state copy rather than removing the panel.

## Interaction Model

Preserve the existing top-level Status tab navigation:

- `Tab` switches between provider list and detail side
- When a provider is selected, keep the existing provider-detail scroll behavior

When `Overall` is selected and detail focus is active:

- `Left` / `Right` switch between dashboard panels
- `Up` / `Down` scroll the active panel
- `PageUp` / `PageDown`, `Ctrl-u` / `Ctrl-d`, `g`, `G` scroll the active panel

Do not add panel tabs inside Overall for this pass.

## State Changes

Add explicit Overall dashboard panel state in `src/tui/status_app.rs`:

- `OverallPanelFocus` enum:
  - `Incidents`
  - `Degradation`
  - `Maintenance`

Track separate scroll positions for Overall panels instead of using the single shared Overall `detail_scroll`.

Provider detail should continue to use its own scroll position.

## Panel Rendering Style

Use panel-level `Block`s immediately.

Inside each panel, render provider entries as soft cards:

- provider header
- provider summary as the lead sentence when available
- structured metadata rows below
- subtle divider line between entries
- no nested bordered mini-cards in this pass

The visual goal is a dashboard board, not a long report and not a border-heavy tile system.

## Content Rules By Panel

### Active Incidents

- Only providers with formal incident rows
- Show provider summary prominently when available
- Show structured metadata such as status, impact, timing, affected services, and latest update

### Service Degradation

- Only degraded providers without formal incident rows
- Show provider summary prominently when available
- Show scope, health label, freshness, and affected services

### Maintenance Outlook

- Secondary and compact
- Show only when non-empty
- Never visually dominate incident or degradation content

## Freshness And Caveats

- Add one quiet freshness cue in `Overall Status`
- Keep this user-facing and calm, not source/debug heavy
- Only surface incomplete-detail caveats when they materially affect interpretation
- Do not revive a `Signal Quality` style panel

## Code Areas To Change

- `src/tui/ui.rs`
  - replace the single mixed Overall paragraph with separate panel blocks
  - add responsive board layout logic
  - split overall entry rendering into panel-specific helpers
- `src/tui/status_app.rs`
  - add Overall panel focus and per-panel scroll state
- `src/tui/app.rs`
  - route Overall dashboard navigation separately from provider detail scrolling
- `src/tui/event.rs`
  - preserve current key meanings while routing left/right and scroll actions to the active Overall panel when applicable

## Test Coverage To Add

- Overall panel focus switching
- Per-panel scrolling behavior
- Focused border styling on the active panel
- Provider summary visibility in incident and degradation soft cards
- Maintenance hidden when empty
- Calm empty-state rendering for incidents and degradation panels
- Narrow-width fallback preserving distinct panels

## Non-Goals For This Pass

- No provider-detail redesign yet
- No nested mini-card entry borders yet
- No panel tabs inside Overall
- No new trust/debug dashboard panels
