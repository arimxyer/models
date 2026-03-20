# Status Tab Redesign Spec

## Role in the doc stack
This is the **canonical implementation spec** for the next Status-tab rewrite.

It is derived from:
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`
- `docs/status-tab-focused-audit.md`
- `docs/status-tab-multi-pass-audit.md`
- `docs/cross-tab-controls-interaction-audit.md`

## Goal
Rewrite the Status tab so it feels like a stable member of the product family, not a custom microsite.

Success means:
- first-use clarity
- repeat-use scanning
- fixed field locations
- explicit labels
- concrete section names
- truthful handling of missing detail
- no hidden or weakly explained interaction state

## Non-goals
- no new fetch pipeline
- no new provider integrations
- no raw-evidence dump UI
- no decorative charts
- no speculative design-system abstraction work

## Current problems this rewrite must fix
1. abstract container naming (`Narrative`, `Note`, similar drift)
2. unstable semantics in hero metadata
3. hidden/under-advertised interaction state
4. too much special-case shell behavior relative to other tabs
5. service/detail absence not always expressed through stable structure

## Shell contract
### Left rail
One-line rows only:
- health icon
- provider name
- optional active issue count

### Right side
Use **one stable outer detail surface**.

Preferred outer title:
- `Status`
- or `Details`

Do not create multiple abstract top-level boxes for the same read flow.
The semantic structure belongs in internal section headers.

## Internal section order
Always render sections in this order:
1. Overview
2. Current incidents
3. Services
4. Maintenance
5. Notes

Sections may be hidden by visibility rules, but ownership must not drift.

## Section contracts
### 1. Overview
Purpose: answer “what is the provider state right now?”

Fixed slot order:
1. provider identity + health icon
2. verdict line
3. `Source: ...`
4. explicit time line:
   - `Latest event: ...`
   - `Source updated: ...`
   - `Last checked: ...`
5. issue badge line

Allowed verdict copy:
- `All systems operational`
- `Some services degraded`
- `Major service disruption`
- `Scheduled maintenance in progress`
- `Status unavailable`

Forbidden in Overview:
- caveat prose
- service rows
- incident body text
- maintenance rows

### 2. Current incidents
Purpose: answer “what is broken right now?”

Show only active incidents.
Each incident row/card may include:
- title
- stage/status
- latest incident time
- affected services/components
- latest meaningful update text

Do not include maintenance or source caveats here.

### 3. Services
Purpose: answer “which services are healthy or affected?”

Show only service/component rows.
Each row may include:
- status icon
- service name
- concise service status
- optional linked incident/maintenance name

Ordering:
1. outage/degraded
2. maintenance-tagged
3. operational

If no service detail exists:
- hide Services entirely
- surface `Service details unavailable` in Notes

### 4. Maintenance
Purpose: answer “is planned maintenance happening?”

Show only maintenance items:
- title
- status
- scheduled window/time
- affected services

### 5. Notes
Purpose: answer “what caveat or limitation matters here?”

Allowed content:
- `Service details unavailable`
- `Limited detail available`
- `Status unavailable`
- relevant source/fetch error text

Notes must stay compact and must not become a junk drawer.

## Visibility rules
- Overview: always visible
- Current incidents: only if active incidents exist
- Services: only if service detail exists
- Maintenance: only if maintenance exists
- Notes: only if caveat/error exists

## Canonical state matrix
| State | Overview | Current incidents | Services | Maintenance | Notes |
|------|----------|-------------------|----------|-------------|-------|
| Operational + full detail | visible | hidden | visible | conditional | hidden unless caveat |
| Operational + no service detail | visible | hidden | hidden | conditional | `Service details unavailable` |
| Degraded/Outage + active incident | visible | visible | visible if detail exists | conditional | conditional |
| Maintenance | visible | hidden unless separate incident exists | visible if detail exists | visible | conditional |
| Unavailable | visible | hidden | hidden | hidden | `Status unavailable` plus error if relevant |

## Interaction contract
### Must keep
- standard movement/search/focus language shared by the app
- `o` open status page
- `r` refresh

### Must change
- hidden service-density mode (`c`) should be **removed** for this redesign pass unless a clearly named, explicitly surfaced reason to keep it emerges
- footer/help must match the actual interaction surface

Rationale:
The cross-tab controls audit found that the current hidden toggle is under-disclosed and makes the tab less learnable.

## Title and metadata rules
- outer panel title stays stable
- section titles use concrete domain nouns only
- no generic `Updated`
- no unlabeled metadata bullet sentence that forces semantic parsing
- source and time must be readable as fixed fields, not one compressed sentence fragment

## Search and list-order rules
- if the provider list uses severity-first ordering, treat that as intentional product behavior
- searchable fields should preferably map to visible content; avoid search matching invisible semantics unless clearly justified

## Spacing and rhythm rules
- one blank separator between visible sections max
- section headers should make section boundaries visually obvious
- avoid multiple stacked bordered micro-panels for one conceptual detail flow
- Overview should remain above the fold

## Test requirements
Render tests should cover at least:
1. operational + full detail
2. operational + no service detail
3. degraded + active incident
4. maintenance
5. unavailable

Tests should assert:
- required section titles appear/vanish correctly
- forbidden labels are absent (`Narrative`, generic `Updated`)
- service-less states do not render Services
- explicit time labels are present
- Notes only contain caveat/error content
- footer/help do not advertise removed controls

## Implementation lanes
### Lane 1 — shell rewrite
Primary file:
- `src/tui/ui.rs`

Responsibilities:
- collapse special-case outer boxes
- implement one stable detail surface
- implement section order and naming

### Lane 2 — interaction/test cleanup
Primary files:
- `src/tui/ui.rs`
- `src/tui/event.rs`
- `src/tui/status_app.rs`
- `src/tui/app.rs` only if needed

Responsibilities:
- remove or explicitly surface hidden mode behavior
- align footer/help with actual controls
- add render/regression tests

### Lane 3 — verification and doc alignment
Primary files:
- `docs/status-tab-redesign-spec.md`
- `docs/ui-design-guide.md` if wording must be synced after implementation

Responsibilities:
- confirm implementation matches the spec
- record any justified deviations

## Acceptance criteria
The redesign is acceptable only if:
- no abstract right-panel container names remain
- section order is stable across provider states
- source/time semantics are explicitly labeled
- service-less providers do not fabricate pseudo-services
- controls shown in footer/help match actual behavior
- the tab feels structurally aligned with Models / Agents / Benchmarks
- tests cover the canonical state matrix
