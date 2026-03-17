# Status Provider Detail Review Guide

_Last updated: March 16, 2026_

Reference spec: `.omx/context/status-page-inspired-redesign-20260317T001900Z.md`

## What changed
- `src/tui/ui.rs` now leads with provider verdict, active issue count, affected surfaces, recent update timing, and compact source/caveat context.
- `src/status.rs` now exposes `active_incidents`, `user_visible_affected_items`, and `user_visible_caveat` so suppression rules live in the status model instead of the UI.
- The right-hand detail no longer foregrounds confidence, coverage, and freshness metadata.

## Spec coverage map
- **Top / primary takeaway** — provider name, overall verdict, active issue badge, affected surfaces, and last meaningful update are grouped into the opening header.
- **Middle / current events** — the `Active detail` panel shows current incidents first and falls back to scheduled maintenance when no active incident is present.
- **Bottom / supporting evidence** — the service table remains available for component-level detail, with a collapsed summary mode behind `c`.
- **Trust language** — reconciliation stays internal; users see short caveats such as `Limited detail available`, `Verify details on the official status page`, or `Status unavailable` instead of internal confidence taxonomy.

## Manual review checklist
1. Open the Status tab and confirm the selected provider header reads as: verdict first, then active issue count if any, then affected/update metadata.
2. Verify fallback or unavailable providers show a short caveat (`Limited detail available` or `Status unavailable`) instead of internal trust taxonomy.
3. Verify providers with active incidents show incident title, phase, and recent update text in the `Active detail` block.
4. Verify providers with scheduled maintenance but no active incident show the maintenance item in `Active detail` and a maintenance count in `At a glance`.
5. Verify providers with no service-level detail show `No service-level detail available.` instead of an empty component table.
6. Toggle `c` in the Status detail panel and confirm service rows collapse or expand without changing the top-level verdict messaging.

## Code-quality review notes
- The design intent from the spec is present: the primary takeaway is now in the header, while raw evidence sits below it.
- The data-quality helpers in `src/status.rs` reduce duplication inside the UI and keep suppression rules testable.
- Remaining UX debt is limited and non-blocking for this lane:
  - `At a glance` still repeats the top-level verdict and affected summary already shown in the header.
  - Scheduled maintenance is contextual rather than a dedicated section when incidents are present.
  - Source metadata is compact, but fallback providers still get two header rows (`source` plus `source note`).

## Follow-up ideas (non-blocking)
1. Trim `At a glance` to only fields that do not already appear in the header.
2. Promote scheduled maintenance to its own small section when both maintenance and incidents are present.
3. Collapse fallback source context into a single compact row if header density becomes an issue.

## Automated verification
- `mise run fmt`
- `mise run clippy`
- `mise run test`

## Regression anchors
- `status::tests::user_visible_caveat_prefers_simple_messages`
- `status::tests::user_visible_affected_items_prefers_surface_labels`
