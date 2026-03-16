# Status Provider Detail Review Guide

_Last updated: March 16, 2026_

## What changed
- `src/tui/ui.rs` now leads with provider verdict, active issue count, affected surfaces, recent update timing, and compact source/caveat context.
- `src/status.rs` now exposes `active_incidents`, `user_visible_affected_items`, and `user_visible_caveat` so suppression rules live in the status model instead of the UI.
- The right-hand detail no longer foregrounds confidence, coverage, and freshness metadata.

## Manual review checklist
1. Open the Status tab and confirm the selected provider header reads as: verdict first, then active issue count if any, then affected/update metadata.
2. Verify fallback or unavailable providers show a short caveat (`Limited detail available` or `Status unavailable`) instead of internal trust taxonomy.
3. Verify providers with active incidents show incident title, phase, and recent update text in the `Active detail` block.
4. Verify providers with no service-level detail show `No service-level detail available.` instead of an empty component table.
5. Toggle `c` in the Status detail panel and confirm service rows collapse or expand without changing the top-level verdict messaging.

## Automated verification
- `mise run fmt`
- `mise run clippy`
- `mise run test`

## Regression anchors
- `status::tests::user_visible_caveat_prefers_simple_messages`
- `status::tests::user_visible_affected_items_prefers_surface_labels`
