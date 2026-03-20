# Status Provider Detail Review Guide

_Last updated: March 16, 2026_

Reference specs:
- `docs/status-tab-bold-design-brief.md`
- `.omx/context/status-page-inspired-redesign-20260317T001900Z.md`

## Review goal
Validate that the Status tab now reads like a provider status page instead of a dashboard.

## Acceptance shape
- **Left rail** — navigation only: status icon, provider name, optional active issue count badge.
- **Right panel order** — single-column flow: hero -> current incidents -> services/components -> maintenance/history -> caveat footer.
- **Empty sections** — hidden by default; do not replace them with filler copy.
- **Trust language** — compact caveat/footer only; no repeated dashboard or meta-trust framing.

## Manual review checklist
1. Confirm the provider list stays one-line and navigation-only: no summaries, freshness copy, provenance notes, or helper text in the rail.
2. Confirm the right-hand detail reads top-to-bottom as a single column rather than an `Overview` + `Current incidents` split dashboard.
3. Confirm the hero makes the verdict obvious with provider name, verdict, compact source/update metadata, and an optional active-issue badge.
4. Confirm active incidents, when present, appear immediately after the hero and own the narrative with title, stage, relative update time, affected services, and latest meaningful update text.
5. Confirm the services/components section reads like a status-page component list rather than a three-column evidence table.
6. Confirm maintenance/history only appears when it adds signal and sits below services/components.
7. Confirm caveats are terse and footer-like (`Limited detail available`, `See official status page for full incident history`, or `Status unavailable`) instead of repeated provenance taxonomy.
8. Confirm duplicated `Affected`, `updated`, `checked`, `refreshed`, and dashboard-summary copy has been materially removed.
9. Confirm providers with no active incidents do not show filler such as `No active incidents reported.` when the section can simply disappear.
10. Toggle `c` and confirm service rows collapse/expand without changing the hero or incident narrative.

## Blocking deltas to catch
If any of the following are still present, the redesign is not done:
- An `Overview` card or any other dashboard-style summary block in the right panel.
- A horizontal split between overview and incidents/maintenance near the top of the detail view.
- Repeated `Affected` or timestamp lines in both the hero and a second summary block.
- User-facing provenance taxonomy or reconciliation/meta-trust copy outside the caveat footer.
- Empty-state filler replacing an omitted incidents or maintenance/history section.

## Code-quality review notes
- Keep the normalized assessment model internal; the UI should expose only the verdict, current incident reality, service health, and terse caveats.
- Prefer deletion/suppression over adding more explanatory layers.
- Support code in `src/status.rs` or `src/tui/status_app.rs` is acceptable only when it helps the single-column status-page presentation stay concise.

## Automated verification
- `mise run fmt`
- `mise run clippy`
- `mise run test`

## Regression anchors
- `status::tests::user_visible_caveat_prefers_simple_messages`
- `status::tests::user_visible_affected_items_prefers_surface_labels`
