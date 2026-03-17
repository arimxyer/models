---
name: ui-design-guide
description: Apply the models TUI design audit and guide before changing tab layouts or information architecture.
---

# UI Design Guide Skill

Use this skill when working on the `models` TUI layout, information architecture, panel naming, or cross-tab consistency.

## Required references
Read these first:
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`

## What this skill enforces
1. Compare the target tab against the shared shell used by the other tabs.
2. Keep navigation rails lightweight and detail panels concrete.
3. Use stable field placement and explicit labels.
4. Reject abstract panel names unless they are unavoidable domain terms.
5. Prefer domain sections such as Overview, Details, Services, Incidents, Maintenance, Notes.

## Workflow
1. Audit the current panel structure.
2. Identify where the target tab drifts from the guide.
3. For Status, map the target provider state to the canonical state matrix before designing.
4. Propose layout changes using the guide's naming, ownership, and placement rules.
5. Only then implement.
6. Add or update regression tests for stable labels/structure when practical.

## Status-tab note
For the Status tab specifically, use the guide's fixed section order, fixed Overview slot order, explicit timestamp semantics, and section ownership rules. Do not reintroduce ambiguous labels like `Updated` or abstract container names like `Narrative`.
