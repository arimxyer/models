# Status Tab Bold Acceptance Review

- reviewed_at_utc: 2026-03-17T01:17:00Z
- reviewer: worker-3
- scope: audit current/in-flight status-tab redesign against `docs/status-tab-bold-design-brief.md`

## Verdict

**Not yet acceptable.** As reviewed in `src/tui/ui.rs`, the status tab is still the pre-brief dashboard layout, and there is no in-flight redesign in the worker worktrees to validate yet.

## Evidence

### 1. Left rail is mostly navigation-only, but the sort/order requirement is still unverified
- `src/tui/ui.rs:319-358` keeps the provider rows to icon + provider name + optional active issue badge, which matches the navigation-only requirement.
- The brief asks for urgency ordering (`active incidents -> degraded/maintenance -> operational -> unavailable/unknown`), but this review pass did not find an implemented acceptance proof for that behavior.

### 2. Right panel is still a dashboard grid, not a single-column status-page flow
- `src/tui/ui.rs:536-544` still hard-splits the detail panel into `header + fixed 12-line dashboard + table`.
- `src/tui/ui.rs:556-735` still renders a horizontal two-card row (`Overview` plus `Maintenance`/`Current incidents`) instead of a single-column narrative flow.
- `src/tui/ui.rs:738-1199` still renders the lower area as a 3-column table, which the brief explicitly says should no longer be the primary reading mode.

### 3. Hero/header still duplicates trust/meta details the brief wanted removed or demoted
- `src/tui/ui.rs:454-512` still renders summary text plus `affected`, `updated`, `refreshed`, `source`, `updated`, `checked`, provenance/source-note, and caveat lines in the hero block.
- The brief explicitly calls for a compact hero with provider, dominant verdict, compact metadata line, and optional active incident badge only.

### 4. Empty/meta sections are still surfaced instead of being hidden
- `src/tui/ui.rs:631-636` still renders `No active incidents reported.` instead of removing the section when there are no active incidents.
- `src/tui/ui.rs:1160-1168` still renders `No service-level detail available.` instead of allowing the lower section to disappear when empty.

### 5. Tests still encode the old dashboard structure
- `src/tui/ui.rs:5598-5604` still expects `Overview`, `Current incidents`, `Services`, `affected: API`, `source: API Status Check`, and `Latest update` in the rendered output.
- Those assertions anchor the current dashboard/meta-heavy layout and will need to change when the redesign lands.

## Worker-tree check

At review time:
- `worktrees/worker-1`: clean working tree; no redesign diff present yet.
- `worktrees/worker-2`: clean working tree in the status-tab implementation paths; no redesign diff present yet.

This means there is currently no in-flight redesign branch available for final acceptance validation.

## Acceptance gaps to close before sign-off

1. Replace the dashboard split with a single-column detail flow: hero -> current incidents (conditional) -> services/components -> maintenance/history (conditional) -> caveat footer (conditional).
2. Strip duplicated hero metadata down to one compact source/updated line.
3. Hide empty incident and services sections instead of rendering filler copy.
4. Replace the 3-column table presentation with a status-page-style services/components list.
5. Update the status-tab rendering tests so they assert the new brief-aligned structure rather than `Overview`/dashboard-era content.
