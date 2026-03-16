# Status reconciliation lane 3 review

Reference spec: `.omx/context/status-reconciliation-details-20260316T221540Z.md`

## Current UI gaps

- The right-hand panel is still source-first: it leads with provenance, source label/method, and provider summary before any app-owned trust assessment.
- The displayed "fresh" value is currently the app fetch age (`StatusApp::last_refreshed`), not reconciled source freshness derived from provider evidence.
- The dashboard area still prioritizes component counts and active incidents, so there is no dedicated place for `confidence`, `coverage`, `warnings`, `contradictions`, or `reconciliation_notes`.
- `StatusApp::apply_fetch` still sorts by raw `health`, `support_tier`, and `provenance`, which means lane 3 has no assessment-first ordering yet.

## Dependency boundary

Lane 3 should not invent reconciliation rules inside `src/tui/ui.rs`.
To implement the target right panel cleanly, the UI needs normalized fields from the reconciliation layer, including:

- `overall_health`
- `confidence`
- `coverage`
- `freshness`
- `affected_surfaces`
- `assessment_summary`
- `evidence_summary`
- `warnings`
- `contradictions`

## Recommended lane 3 implementation once upstream fields land

1. Replace the current header with an assessment row: health, confidence, coverage, freshness.
2. Render a short "Why" section from `assessment_summary` and `evidence_summary`.
3. Render a trust/caveats section before raw evidence for warnings, contradictions, fallback caveats, and missing coverage.
4. Demote source metadata and provider-native summary into the raw evidence section.
5. Reuse the current component/incident table as the raw evidence section to keep layout churn low.

## Test coverage to add with the UI change

- Fully populated reconciled entry renders the five target sections in order.
- Contradiction case surfaces a contradiction explicitly.
- Fallback / low-coverage case shows caveats instead of looking complete.
- Unavailable / no-coverage case still renders explicit trust signals.
