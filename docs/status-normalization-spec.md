# Status Normalization Spec

## Goal

Normalize provider status data without erasing the difference between:

- detail that exists and was parsed
- detail that was explicitly reported as empty
- detail the source does not support
- detail we tried to fetch but failed to load
- detail we could have fetched but did not preserve yet

## Non-Goals

- forcing every source family into the same raw payload shape
- treating fallback summary data as peer-quality evidence to rich official feeds
- inferring source truth from empty vectors alone

## Canonical Types

The normalized model centers on `ProviderStatus` in `src/status.rs` and the parallel detail-state contract:

- `StatusLoadState`
- `StatusDetailAvailability`
- `StatusDetailSource`
- `StatusDetailState`

Each detail-bearing collection in `ProviderStatus` has a paired state field:

- `components` + `components_state`
- `incidents` + `incidents_state`
- `scheduled_maintenances` + `scheduled_maintenances_state`

## Detail Availability Semantics

| Enum Value | Meaning | UI Implication |
| --- | --- | --- |
| `Available` | detail was loaded and contains normalized rows | render rows normally |
| `NoneReported` | source supports the detail and confirmed zero rows | render explicit empty-state copy |
| `Unsupported` | source family or adapter does not provide this detail class | render muted unsupported copy |
| `FetchFailed` | detail fetch or parse was attempted and failed | render warning/error copy |
| `NotAttempted` | detail may exist, but this pipeline path did not preserve it | render neutral incomplete-data copy |

## Detail Source Semantics

| Enum Value | Meaning |
| --- | --- |
| `Inline` | detail came from the primary source payload |
| `Enrichment` | detail came from a secondary endpoint |
| `SummaryOnly` | source only exposes overall summary information |
| `Derived` | app derived a summary from a richer upstream payload without preserving raw rows |
| `None` | no source-specific detail channel applies |

## Invariants

- `Available` implies the paired collection contains one or more rows.
- `NoneReported` implies the paired collection is empty because the source explicitly reported none.
- `Unsupported` implies the paired collection is empty and should not be treated as a successful empty result.
- `FetchFailed` implies the paired collection is empty unless partial rows were explicitly preserved on purpose.
- `NotAttempted` implies the paired collection is empty and the UI must not translate that to `0 issues`.

## Summary Fields

`ProviderStatus` separates provider-authored and app-authored prose:

- `provider_summary`: provider- or source-authored summary text
- `status_note`: app-authored caveat or adapter note

This split keeps the UI from mixing provider claims with adapter limitations.

## Load State Semantics

| Value | Meaning |
| --- | --- |
| `Placeholder` | pre-fetch placeholder state |
| `Loaded` | provider status loaded without known detail-fetch failures |
| `Partial` | provider status loaded, but at least one detail fetch failed |
| `Failed` | no usable provider status could be loaded |

## Reconciliation Rules

### Official Sources

- Preserve the source method and source URLs.
- Store source timestamps in `source_updated_at`.
- Reconcile provider health against active incidents and non-operational component rows.
- Assign each detail-state field explicitly before any UI code sees the entry.

### Enrichment-Backed Detail

- `IncidentIoShim` incidents are enrichment-backed.
- `Instatus` components are enrichment-backed.
- If enrichment succeeds:
  - use `Available` or `NoneReported`
- If enrichment fails:
  - use `FetchFailed`
- If enrichment is intentionally skipped or not yet preserved:
  - use `NotAttempted`

### Fallback Sources

- Fallback `ApiStatusCheck` remains summary-only.
- The fallback branch sets detail states to `Unsupported` with `SummaryOnly` source semantics.
- Fallback can support provider health and summary text, but not rich detail claims.

### Unavailable Sources

- If no usable official or fallback source loads, `load_state` becomes `Failed`.
- `StatusStrategy::Unverified` should map detail states to `Unsupported`.
- Failed official/fallback fetches should map detail states to `FetchFailed` rather than silently empty collections.

## Coverage Rules

`ProviderAssessment::coverage` derives from detail-state availability, not from source family alone.

- incidents + components available -> `Full`
- incidents only -> `IncidentOnly`
- components only -> `ComponentOnly`
- no structured detail but provider summary exists -> `SummaryOnly`
- no structured detail and no provider summary -> `None`

## Confidence Rules

Confidence should be downgraded when:

- freshness is stale or unknown
- contradictions exist between provider summary and normalized detail
- any detail state is `FetchFailed`

Confidence should not be downgraded merely because a source is `NoneReported`.

## Helper-Driven UI Contract

UI code should use `ProviderStatus` helpers instead of raw `Vec::is_empty()` checks.

Key helpers:

- `component_detail_available()`
- `incident_detail_available()`
- `maintenance_detail_available()`
- `confirmed_no_components()`
- `confirmed_no_incidents()`
- `confirmed_no_maintenance()`
- `has_detail_fetch_failures()`
- `has_partial_data()`
- `detail_state_message(...)`

The UI should never infer `0 active incidents` or `0 service issues` from an empty vector unless the paired detail state says `NoneReported`.

## Source-Family Mapping Notes

### `StatuspageV2`

- usually inline for incidents, components, and maintenance
- empty arrays can map to `NoneReported`

### `IncidentIoShim`

- summary endpoint is not enough for incident fidelity
- incidents are `Enrichment`

### `BetterStack`

- components come from inline `status_page_resource` entries
- scheduled maintenance is currently `Unsupported` in this adapter

### `OnlineOrNot`

- incidents, components, and maintenance are inline when present

### `StatusIo`

- incidents, components, and maintenance are inline
- `status_code = 400` maps to degraded, not unknown

### `Instatus`

- incidents and maintenance are inline
- components require enrichment and must not default to `NoneReported` before that enrichment succeeds

### `GoogleCloudJson`

- current adapter preserves provider summary and health
- raw incidents are not yet preserved in `ProviderStatus`
- model this as `NotAttempted` or `Derived`, not `Unsupported` by the source itself

### `ApiStatusCheck`

- summary only
- detail classes are unsupported by this adapter

## Test Plan

- parser fixtures per source family in `src/status_fetch.rs`
- drift tests for:
  - Better Stack `public_name`
  - Status.io `400`
  - IncidentIoShim second incidents fetch behavior
  - Instatus components enrichment
- normalization tests in `src/status.rs` for each `StatusDetailAvailability` variant
- UI tests that confirm empty vectors no longer imply trusted absence without the paired detail state

## Rollout Plan

1. document source-shape and normalization contracts
2. patch live parser drift
3. land parallel detail-state fields
4. route coverage/confidence/UI semantics through helpers
5. expand parser and normalization tests
