# Status Tab Redesign Spec

## Goal
Rewrite the Status tab so it behaves like a stable product surface rather than a one-off custom layout. The redesign must optimize for:
- first-use clarity
- repeat-use scanning
- fixed field locations
- concrete section names
- truthful handling of missing service detail

This spec is implementation-grade and should be used with:
- `docs/ui-tab-audit.md`
- `docs/status-tab-multi-pass-audit.md`
- `docs/ui-design-guide.md`
- `.codex/skills/ui-design-guide/SKILL.md`

## Non-goals
- no new fetch pipeline
- no new status providers
- no attempt to expose all raw source evidence
- no decorative charts
- no new generalized design system abstractions

## Shell contract
### Left rail
One-line provider rows only.

Each row may contain only:
- status icon
- provider name
- optional active issue count

Do not add:
- provenance badges
- timestamp metadata
- summary text
- support-tier meta

### Right panel
Use a single stable detail surface with internal section headers.

Avoid multiple abstract top-level boxes like:
- `Narrative`
- `Note`
- any other non-domain container labels

Preferred top-level right-panel title:
- `Status`
- or `Details`

The important point is that the outer panel title stays stable while the internal section headers carry the domain meaning.

## Fixed reading order
Internal section order is always:
1. Overview
2. Current incidents
3. Services
4. Maintenance
5. Notes

Sections may hide when empty according to the visibility rules below, but their semantic ownership must never drift.

## Section specs
### 1. Overview
Purpose: answer "what is the provider state right now?"

Fixed slot order inside Overview:
1. provider identity line
2. verdict line
3. source line
4. time line
5. issue badge line

#### Slot details
**Identity line**
- provider name
- optional health icon

**Verdict line**
Allowed verdict copy:
- `All systems operational`
- `Some services degraded`
- `Major service disruption`
- `Scheduled maintenance in progress`
- `Status unavailable`

**Source line**
Must always be explicitly labeled:
- `Source: <source label>`

If official page exists, append as a separate token on the same line or the next line:
- `Official page`

**Time line**
Must always be explicitly labeled with one of:
- `Latest event: ...`
- `Source updated: ...`
- `Last checked: ...`

Forbidden:
- bare `Updated`
- unlabeled timestamp text
- changing the same label to mean different things in different states

**Issue badge line**
Examples:
- `1 active incident`
- `3 active incidents`

If zero, this line may be omitted.

#### Overview ownership
Overview may include only:
- provider identity
- verdict
- source
- time
- issue count

Overview must not include:
- caveat prose
- service lists
- incident update bodies
- maintenance rows

### 2. Current incidents
Purpose: answer "what is broken right now?"

Include only active incident content:
- incident title
- stage/status
- most recent incident time
- affected services/components
- latest meaningful update text

Do not include:
- maintenance items
- source caveats
- component inventory rows

### 3. Services
Purpose: answer "which services are healthy or affected?"

Include only service/component rows.

Each row may include:
- status icon
- service/component name
- concise status text
- optional linked incident/maintenance name

Ordering:
1. outage/degraded services
2. maintenance-tagged services
3. operational services

In compact mode:
- operational services may collapse into a summary row

If no service detail exists:
- hide the section entirely
- surface `Service details unavailable` in Notes

### 4. Maintenance
Purpose: answer "is planned maintenance happening?"

Include only:
- maintenance title
- status
- scheduled time/window
- affected services

Do not mix maintenance into incidents or notes.

### 5. Notes
Purpose: communicate compact caveats only.

Allowed note types:
- `Service details unavailable`
- `Limited detail available`
- `Status unavailable`
- source/fetch error text when necessary

Rules:
- keep notes compact
- do not use Notes as a dump for extra metadata
- Notes must not contain service rows or incident narrative

## Visibility rules
### Overview
- always visible

### Current incidents
- visible only when active incidents exist

### Services
- visible only when component/service detail exists

### Maintenance
- visible only when maintenance exists

### Notes
- visible only when caveat/error exists

## Canonical state matrix
| State | Overview | Current incidents | Services | Maintenance | Notes |
|------|----------|-------------------|----------|-------------|-------|
| Operational + full detail | visible | hidden | visible | conditional | hidden unless caveat |
| Operational + no service detail | visible | hidden | hidden | conditional | `Service details unavailable` |
| Degraded/Outage + active incident | visible | visible | visible if detail exists | conditional | conditional |
| Maintenance | visible | hidden unless separate incident exists | visible if detail exists | visible | conditional |
| Unavailable | visible | hidden | hidden | hidden | `Status unavailable` plus error if relevant |

## Naming rules
Allowed internal section headers:
- `Overview`
- `Current incidents`
- `Services`
- `Maintenance`
- `Notes`

Forbidden:
- `Narrative`
- `Context`
- `Insight`
- `Story`
- generic unlabeled boxes for mixed content

## Spacing and rhythm rules
- each visible section gets a concrete header
- one blank separator between sections maximum
- do not create extra bordered micro-panels for each content type unless the entire right-panel architecture changes intentionally
- avoid long uninterrupted mixed-content wells
- the user should be able to visually distinguish where incidents end and services begin without rereading labels multiple times

## Scrolling rules
- Overview must remain above the fold
- if incidents are long, services should still remain reachable without the page feeling like a prose document
- compact mode should optimize for scanning, not completeness

## Render-test expectations
Add or update tests for at least these states:
1. operational + full detail
2. operational + no service detail
3. degraded + active incident
4. maintenance
5. unavailable

Tests should assert:
- expected section titles are present/absent by state
- forbidden titles are absent (`Narrative`, `Updated` as generic timestamp label)
- service-less states do not render the Services section
- explicit timestamp labels are used
- Notes only contain caveat/error content

## Implementation lane suggestions
### Lane 1: shell/section rewrite
Primary files:
- `src/tui/ui.rs`

Responsibilities:
- remove abstract right-panel containers
- implement stable section order
- fix section naming and field placement

### Lane 2: state/test coverage
Primary files:
- `src/tui/ui.rs`
- `src/status.rs` only if helper behavior needs tightening

Responsibilities:
- add/adjust render tests for canonical states
- tighten helper behavior to support section visibility rules

### Lane 3: review/docs
Primary files:
- `docs/status-tab-redesign-spec.md`
- `docs/ui-design-guide.md` if final wording must be synced after implementation

Responsibilities:
- ensure implementation matches the spec
- record any deliberate deviations

## Acceptance criteria
The redesign is acceptable only if:
- the right panel no longer uses `Narrative`
- each visible section has a concrete domain title
- timestamps use explicit semantic labels
- service-less providers do not display pseudo-service content
- the provider rail remains one line tall and navigation-first
- the right panel feels stable across provider states
- render tests cover the canonical state matrix
