# UI Design Guide

## Purpose
Provide a reusable layout and UX standard for this TUI so new work stays aligned across tabs instead of drifting tab-by-tab.

## Core principles
1. **Stable locations beat clever layout changes**
   - users expect values to change, not field locations
2. **Use domain nouns for panel names**
   - avoid abstract or internal labels
3. **Navigation is not explanation**
   - rails select things; detail panes explain them
4. **One panel, one job**
   - avoid mixed-purpose boxes
5. **Label semantics explicitly**
   - if a field can mean different things, label the exact meaning shown

## Shared shell rules
- global tab bar at top
- global shortcut/help footer at bottom
- focused panel border: cyan
- unfocused panel border: dark gray
- panel titles should communicate the thing, count, search, or mode
- no control-hint prose inside the main content unless there is no better home

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

Avoid:
- Narrative
- Insight
- Context
- Story
- Meta
unless the domain truly requires it.

## Field placement rules
If a detail surface has metadata, keep it in fixed slots:
- line 1: object identity + status/verdict
- line 2: source and time metadata with explicit labels
- line 3: issue count or high-priority badge

Do not swap the meaning of a recurring field label between states.

## Status-tab-specific guidance
### Right panel structure
The Status tab should use named sections in this order:
1. Overview
2. Current incidents
3. Services
4. Maintenance
5. Notes

### Overview rules
Always keep the same field layout:
- provider name
- status verdict
- source
- time field with explicit semantic label
- official page hint/link text if available
- incident count badge if non-zero

Preferred time labels:
- `Latest event`
- `Source updated`
- `Last checked`

Never collapse all of these under just `Updated`.

### Services rules
- if service detail exists, show Services in its normal slot
- if service detail does not exist, do not replace the slot with misleading pseudo-services
- use a clear note such as `Service details unavailable`

### Notes rules
Notes should be conditional and compact:
- service details unavailable
- limited detail available
- status unavailable
- fetch/source error summary when truly relevant

## Consistency over optimization
When improving one tab, compare it to the others first:
- does the panel naming still match the rest of the app?
- are field locations becoming more stable or less stable?
- is the right panel easier to learn after one use?

## Review checklist
Before shipping UI changes, ask:
- Are labels concrete?
- Are field locations stable?
- Is the navigation rail still lightweight?
- Does the detail panel have one clear reading order?
- Did we remove internal/implementation vocabulary from the UI?
- Would a repeat user know where to look without relearning the page?
