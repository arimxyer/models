# Status Tab Multi-Pass Audit

## Scope
A deeper audit of the Status tab using multiple lanes and passes:
- cross-tab shell/architecture audit
- UX/layout audit against the current screenshot and guide
- critical review of the guide and acceptance criteria

## Method
### Pass 1: Structural shell audit
Inputs:
- `src/tui/ui.rs`
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`

Questions:
- what shell patterns are shared across tabs?
- where does Status drift from those patterns?
- which parts of Status are special-case architecture vs product-family architecture?

### Pass 2: UX/layout audit
Inputs:
- current Status screenshot from the thread
- current `draw_status_main` rendering in `src/tui/ui.rs`
- the shared guide

Questions:
- what is easy for first-time users?
- what breaks repeat-use scanning?
- which labels or sections are not domain concepts?
- where does spacing/rhythm hurt comprehension?

### Pass 3: Critical rubric audit
Inputs:
- the audit and guide docs themselves
- the current Status renderer

Questions:
- are the rules concrete enough to implement?
- what still allows interpretation drift?
- what acceptance criteria are still too soft?

## Multi-lane findings

### Lane A: Cross-tab shell audit
Key findings:
- Models, Agents, and Benchmarks already share a strong shell: navigation rail(s), concrete panel titles, stable left-to-right responsibilities, and a right-side explanation surface.
- Status still drifts because its right side is a special stacked shell using `Status page`, `Narrative`, and `Note` rather than the concrete panel identities the other tabs use.
- The provider rail is now close to the shared pattern and should stay lightweight.

High-confidence recommendations:
1. remove `Narrative` as a container concept
2. stop using `Note` as a vague separate box
3. prefer one stable detail surface or two stable named surfaces rather than three abstract boxes
4. keep hero metadata in fixed slots
5. keep the provider rail navigation-first

### Lane B: UX/layout audit
Key findings:
- First-use clarity is improved, but the page still requires interpretation because the middle body is one mixed content well.
- `Narrative` is weak because it is not a domain noun, mixes multiple semantic jobs, and hides layout drift.
- Repeat-use stability is still weak because the middle section can change meaning without changing its title.
- The current scan path is still: provider -> verdict -> figure out what `Narrative` means this time.

High-confidence recommendations:
1. use fixed domain section names: `Overview`, `Current incidents`, `Services`, `Maintenance`, `Notes`
2. keep metadata in fixed labeled rows near the top
3. keep caveats in one explicit notes area
4. improve visual rhythm so section boundaries are easier to scan

### Lane C: Critical rubric audit
Key findings:
- the first guide pass was directionally correct but too soft for implementation
- missing pieces included:
  - fixed-slot contract
  - section ownership rules
  - visibility rules
  - canonical state matrix
  - stronger render/live verification rubric

High-confidence recommendations:
1. define exact Overview slot order
2. define what content is forbidden in each section
3. define when sections are visible vs hidden
4. require tests for canonical provider states

## Consolidated diagnosis
The core problem is not just wording. The Status tab still behaves like a custom microsite layered on top of the product shell rather than a stable sibling of Models, Agents, and Benchmarks.

That causes three recurring failure modes:
1. abstract container naming (`Narrative`)
2. unstable semantic locations (same field area, different meaning)
3. conditional layout drift (sections appearing/disappearing without a fixed structural contract)

## Stronger redesign target
### Preferred right-panel order
1. Overview
2. Current incidents
3. Services
4. Maintenance
5. Notes

### Overview slot order
1. provider identity + verdict
2. source field
3. time field with explicit semantic label
4. issue badge line

### Section ownership
- Overview: identity and top-level metadata only
- Current incidents: active incident content only
- Services: service/component rows only
- Maintenance: maintenance rows only
- Notes: caveats/errors only

## Recommended implementation sequence
1. update the guide and skill with the stricter contract
2. redesign Status against the stricter contract
3. add render tests for canonical states
4. perform a live visual review after implementation

## Acceptance criteria for the next redesign pass
- no abstract panel titles remain
- no unlabeled timestamps remain
- section order is stable across provider states
- service-less providers do not fabricate service detail
- repeat users can find verdict, incidents, services, maintenance, and caveats in fixed places
- render tests cover operational/full, operational/no-service-detail, degraded/incident, maintenance, and unavailable states
