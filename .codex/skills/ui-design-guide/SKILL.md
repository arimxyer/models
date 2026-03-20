---
name: ui-design-guide
description: Apply the canonical models TUI audit and design guide before changing tab layouts, information architecture, or interaction disclosure.
---

# UI Design Guide Skill

Use this skill when working on the `models` TUI layout, information architecture, panel naming, section ownership, control disclosure, or cross-tab consistency.

## Canonical references
Read these first:
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`

Use these as supporting evidence when relevant:
- `docs/models-tab-lane-audit.md`
- `docs/agents-tab-audit.md`
- `docs/benchmarks-tab-audit.md`
- `docs/status-tab-focused-audit.md`
- `docs/cross-tab-controls-interaction-audit.md`
- `docs/status-tab-redesign-spec.md`

## What this skill enforces
1. Compare the target tab against the shared shell used by the other tabs.
2. Keep navigation rails lightweight and detail panels concrete.
3. Use stable field placement and explicit labels.
4. Reject abstract panel names unless they are unavoidable domain terms.
5. Make controls/modes discoverable in footer/help or remove them.
6. For Status, follow the canonical state matrix and section ownership rules before implementation.

## Workflow
1. Audit the current panel structure and interaction surface.
2. Identify where the target tab drifts from the canonical docs.
3. For Status, map the target provider state to the canonical state matrix first.
4. Propose layout/control changes using the guide's naming, ownership, placement, and disclosure rules.
5. Only then implement.
6. Add or update regression tests for stable labels, states, and controls when practical.

## Status-tab note
For the Status tab specifically:
- use the fixed section order
- use the fixed Overview slot order
- use explicit timestamp semantics
- keep Notes separate from Overview/Services
- do not reintroduce abstract labels like `Narrative`
- do not keep hidden toggles undocumented
