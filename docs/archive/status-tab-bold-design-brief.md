# Status Tab Bold Design Brief

- created_at_utc: 2026-03-17T01:06:00Z
- repo: /home/arimayer/dev/personal/models
- source_context:
  - .omx/context/status-tab-bold-redesign-20260317T010200Z.md
  - .omx/context/status-page-inspired-redesign-20260317T001900Z.md
  - src/tui/ui.rs
  - src/tui/status_app.rs
- intent: make the Status tab read like a provider status page first and an internal data model second

## Aesthetic direction
**Tone:** calm, high-trust, operations-room minimalism.

The memorable thing is not decoration. It is the feeling that every provider has a real status page with a single dominant verdict, a clean incident stack, and quiet secondary context. The tab should feel closer to OpenAI/Anthropic/OpenRouter status pages than to a metrics dashboard.

## What is wrong with the current layout
Current `src/tui/ui.rs` still spreads the same truth across too many surfaces:
- the left rail already shows provider state and incident count
- the header repeats status, summary, affected info, updated info, source info, provenance note, caveat, and controls
- the overview card repeats status, latest update, affected scope, incidents/maintenance, services count, and caveat again
- the incident/maintenance card competes with the overview instead of owning the narrative
- the component table becomes the third place where incident context appears

Result: the user has to reconcile the interface instead of reading a status page.

### Current code anchors
- `src/tui/ui.rs:431-554` builds an evidence-heavy hero/header with duplicated summary, affected, update, source, provenance, caveat, and control copy.
- `src/tui/ui.rs:556-735` splits the upper body into `Overview` plus `Current incidents`/`Maintenance`, creating a dashboard feel and repeating the same narrative.
- `src/tui/ui.rs:738+` turns the lower panel into a three-column component/evidence table, which is useful data but not the right primary reading order for a status page.

## Design principle
One screen, one takeaway:
1. overall provider state
2. active incident reality now
3. impacted services/components
4. recent maintenance/history if useful
5. compact source caveat only when needed

## Information architecture

### 1. Left rail: provider navigation only
Each row should contain only:
- status dot/icon
- provider name
- optional active issue count badge

Do **not** show summaries, provenance language, freshness language, or extra helper text in the rail.

**Ordering:**
- providers with active incidents first
- then degraded/maintenance
- then operational
- then unavailable/unknown

### 2. Main panel shell
Top-to-bottom order:
1. Hero status
2. Current incidents (conditional)
3. Services / components
4. Maintenance or recent history (conditional)
5. Caveat footer (conditional)

This should be a single-column reading flow. No split dashboard row.

## Section spec

### Hero status
Purpose: give the user the answer in one glance.

Include:
- provider name
- one overall status verdict in large, high-contrast styling
- compact line with source + last updated + official-page hint
- small active incident badge only if non-zero

Do **not** include:
- prose summary block if it only restates the verdict
- separate refreshed/checked timestamps unless needed for trust
- provenance taxonomy as a featured concept
- keyboard hints in the hero body

**Hero copy model:**
- Operational: `All systems operational`
- Degraded: `Some services degraded`
- Outage: `Major service disruption`
- Maintenance: `Scheduled maintenance in progress`
- Unknown: `Status unavailable`

### Current incidents
Show only when active incidents exist.

Each incident card/row should contain:
- incident title
- current stage (`Investigating`, `Identified`, `Monitoring`, etc.)
- relative latest-update time
- affected services/components
- latest meaningful update text

Rules:
- newest/highest-severity incident first
- cap verbose body length before the components section gets pushed off-screen
- if there are no active incidents, remove this entire section rather than replacing it with filler copy

### Services / components
This is the durable operational view after the hero.

Show:
- degraded/outage services first
- maintenance-tagged services next
- operational services last or collapsed behind a summary row in compact mode

Preferred presentation:
- service name
- status chip/icon
- optional linked incident/maintenance label
- short note only when there is an active issue

This section should feel like a status-page component list, not a 3-column internal evidence table.

### Maintenance / recent history
Conditional section.

Show only when:
- scheduled maintenance exists, or
- there is recent resolved history that adds context

Prefer a compact timeline list:
- maintenance title
- scheduled window / recent event time
- affected services

If there is no meaningful maintenance/history, hide the section entirely.

### Caveat footer
One narrow footer line only when needed.

Allowed caveats:
- `Limited detail available`
- `See official status page for full incident history`
- `Status unavailable`

This is the only place where fallback/coverage limitations should usually surface.

## What to delete or aggressively suppress
From the current Status tab presentation, remove or demote:
- the two-column overview + incidents dashboard block
- duplicated `Affected` lines
- duplicated `updated` / `checked` / `refreshed` timestamps in the hero area
- source note paragraphs as a default section
- caveat/note lines when status is otherwise clear
- provider-list semantics that duplicate the main panel
- empty-state filler like `No active incidents reported` when a section can simply disappear
- internal assessment vocabulary as user-facing structure (`confidence`, `coverage`, `contradictions`, `freshness`, reconciliation language)
- keyboard hint copy from the hero content area; move controls to footer/help chrome only
- any chart/pie treatment unless a future version proves it improves decisions

## Proposed screen composition in TUI terms

### Left column
- keep current provider list width or narrow it slightly
- make rows visually quieter and more status-page-like
- selected row gets border emphasis; unselected rows stay plain

### Right panel
Replace current vertical structure in `src/tui/ui.rs`:
- **remove:** header + fixed 12-line dashboard split + evidence-heavy component table framing
- **replace with:**
  1. hero block
  2. incidents list block if active
  3. services/components list block
  4. maintenance/history block if present
  5. compact footer note if needed

The right panel should read as stacked narrative blocks, not a dashboard grid.

## Implementation-ready executor brief

### File ownership targets
- `src/tui/ui.rs`: primary redesign work
- `src/tui/status_app.rs`: only small support changes if ordering/view state needs adjustment
- `src/status.rs`: presentation helper additions only if absolutely necessary

### Execution rules
1. Keep the normalized status model internal.
2. Use existing health/provenance/component helpers where possible.
3. Prefer deleting UI branches over adding more explanatory layers.
4. Default-hide empty sections.
5. Preserve keyboard behavior, but relocate hint copy out of the hero body.

### Concrete layout changes
1. **Provider list**
   - keep one-line rows
   - show status icon, provider name, optional active issue count
   - sort by operational urgency if low-risk to implement; otherwise keep existing order for this pass

2. **Hero block**
   - create a single verdict block with provider name, verdict label, compact metadata row, optional incident badge
   - derive a short hero sentence from health, not from the full summary paragraph unless the summary adds unique meaning

3. **Incident section**
   - render only active incidents
   - show latest update text directly under each incident heading
   - keep stage/time/components visible without forcing users into the lower table

4. **Services/components section**
   - simplify from current three-column evidence table toward a cleaner service-status list
   - preserve severity ordering
   - keep incident/maintenance linkage terse
   - collapse all-clear operational items when space is tight

5. **Maintenance/history section**
   - show only when populated
   - make it visually subordinate to active incidents and service health

6. **Caveat footer**
   - only show when non-official provenance or sparse data materially changes interpretation
   - use short human phrasing, not provenance taxonomy copy

### Non-goals for this pass
- no new data fetch pipeline
- no new provenance taxonomy surfaced to users
- no decorative charts
- no attempt to show every raw evidence field

## Acceptance checklist
- At a glance, the provider verdict is obvious.
- The tab feels recognizable as a status page.
- Active incidents own the narrative when present.
- Services/components read as operational components, not internal evidence rows.
- Empty sections disappear.
- Caveats are compact and conditional.
- Duplication is materially reduced versus current `src/tui/ui.rs`.
