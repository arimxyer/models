# UI Design Guide

## Role in the doc stack
This is the **canonical design rules document**.

Use it to decide:
- how tab shells should be structured
- how names and titles should work
- where metadata should live
- how controls should be disclosed
- how Status should be rebuilt without drifting again

Evidence lives in the audit docs. This file is the rule layer, not the evidence layer.

## Core principles
1. **Stable locations beat clever rearrangement**
   - values may change; field locations should not change casually
2. **Use domain nouns**
   - avoid internal or abstract labels
3. **Navigation is not explanation**
   - rails select; detail panes explain
4. **One panel, one job**
   - mixed-purpose boxes are a smell
5. **Label semantics explicitly**
   - if a field can mean different things, label the exact meaning shown
6. **Controls must be discoverable**
   - active modes/toggles must be surfaced in footer/help or removed

## Shared shell rules
- global tab bar at top
- global shortcut/help footer at bottom
- focused panel border: cyan
- unfocused panel border: dark gray
- titles should communicate object, count, search, sort, filter, or mode
- avoid body-level control-hint clutter when footer/help can own it

## Panel naming rules
Prefer:
- Providers
- Models
- Agents
- Details
- Overview
- Current incidents
- Services
- Maintenance
- History
- Notes
- Head-to-Head
- Scatter
- Radar

Avoid:
- Narrative
- Insight
- Context
- Story
- Meta
- generic mixed-content labels

## Title truthfulness rules
- if a title implies a domain concept, the panel must actually own that concept
- titles should not hide changing semantics behind a stable but vague name
- if sort/filter/mode materially changes what the user is seeing, expose that in the title or an adjacent explicit control surface

## Field placement rules
If a detail surface has metadata, keep it in fixed slots:
- slot 1: identity + verdict/status
- slot 2: source/ownership metadata
- slot 3: time metadata with explicit label
- slot 4: issue badge or high-priority state

Do not let a recurring row silently change meaning between states.

## Controls and interaction rules
### Shared movement language
Prefer reuse of:
- `j/k/g/G`
- `Ctrl-d/Ctrl-u`
- `PageUp/PageDown`
- `/` for search
- `Tab` / `h` / `l` for focus shifts where applicable

### Disclosure rules
- every active mode toggle must appear in footer/help unless intentionally removed
- footer/help should describe the actual control surface, not a reduced subset
- hidden defaults (sort order, searchable hidden fields, mode toggles) must be justified or surfaced

### Alternate views
- alternate views should be explicit and named
- do not hide major view differences under subtle container-title mutations

## Section ownership rules
Each visible section should own one concept only.

Examples:
- `Overview` owns identity/verdict/source/time/issue count
- `Current incidents` owns active incident content
- `Services` owns service rows
- `Maintenance` owns maintenance rows
- `Notes` owns caveats/errors

A section must not absorb another section's semantics just because one section is hidden.

## Status-tab-specific guidance
### Left rail contract
One-line provider rows only:
- status icon
- provider name
- optional active issue count

Do not add:
- summary text
- provenance badges
- timestamp text
- support-tier meta

### Right panel contract
Use a stable section stack in this order:
1. Overview
2. Current incidents
3. Services
4. Maintenance
5. Notes

### Overview contract
Fixed slot order:
1. provider identity + verdict
2. `Source: ...`
3. one explicit time label:
   - `Latest event`
   - `Source updated`
   - `Last checked`
4. issue badge line

Forbidden:
- unlabeled timestamps
- generic `Updated` when semantics differ by source/state
- caveats mixed into Overview

### Visibility rules
- Overview: always visible
- Current incidents: visible only when incidents exist
- Services: visible only when service detail exists
- Maintenance: visible only when maintenance exists
- Notes: visible only when caveat/error exists

### Canonical state matrix
| State | Overview | Current incidents | Services | Maintenance | Notes |
|------|----------|-------------------|----------|-------------|-------|
| Operational + full detail | visible | hidden | visible | conditional | hidden unless caveat |
| Operational + no service detail | visible | hidden | hidden | conditional | `Service details unavailable` |
| Degraded/Outage + active incident | visible | visible | visible if detail exists | conditional | conditional |
| Maintenance | visible | hidden unless separate incident exists | visible if detail exists | visible | conditional |
| Unavailable | visible | hidden | hidden | hidden | `Status unavailable` plus error if relevant |

### Status controls rule
For the redesign pass, hidden service-density mode should not survive unless it is clearly surfaced and justified. Simpler is better.

## Audit workflow rule
Before major UI redesign work:
1. run or refresh per-tab audits
2. synthesize cross-tab patterns and drift
3. update the guide if the rule layer changed
4. then write the implementation spec
5. then implement

## Review checklist
Before accepting UI work, ask:
- Are labels concrete?
- Are field locations stable?
- Is the rail still navigation-first?
- Does each section own one concept?
- Are controls discoverable?
- Are hidden defaults justified?
- Are render tests covering canonical states/modes?
- Would a repeat user know where to look without relearning the page?
