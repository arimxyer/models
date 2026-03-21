# Status Module — Claude Code Instructions

## Module Structure

- **`types.rs`** — Core types: `ProviderHealth` enum, `ProviderStatus` struct, detail-state contract
- **`registry.rs`** — `STATUS_REGISTRY` (22 entries), provider slug aliases, strategy/support tier lookup
- **`assessment.rs`** — Assessment logic on `ProviderStatus`: coverage, freshness, confidence, contradictions, affected surfaces
- **`fetch.rs`** — `StatusFetcher` async fetcher with bounded concurrency (≤10 in-flight), Google pre-fetch
- **`adapters/`** — 7 adapters: statuspage, betterstack, google, instatus, onlineornot, status_io, fallback

## Key Contracts & Patterns

### Detail-State Contract (types.rs)

Three parallel state fields on `ProviderStatus`:
- `components_state`, `incidents_state`, `scheduled_maintenances_state` (`StatusDetailState`)

Each `StatusDetailState` has:
- `availability` — `Available | NoneReported | Unsupported | FetchFailed | NotAttempted`
- `source` — `Inline | Enrichment | SummaryOnly | Derived | None`
- `note`, `error` — metadata

**Use helper methods** (`component_detail_available()`, `incident_detail_available()`, etc.) instead of inferring from empty vectors. Never assume empty vector = "none reported"; check the `*_state` metadata.

### Adapter Pattern

Each adapter module:
1. Parses upstream JSON → intermediate types (`OfficialSummaryResponse`, etc.)
2. Maps to normalized IR: `ComponentStatus`, `ActiveIncident`, `ScheduledMaintenance`
3. Returns `OfficialSnapshot` (or `FallbackSnapshot` for fallback adapter) with detail states

Component status normalization via `adapters::normalize_component_status()` (Instatus UPPERCASECONCATENATED → snake_case).

### Adding a New Provider

1. Add entry to `STATUS_REGISTRY` in `registry.rs` with `OfficialStatusSource` enum variant
2. Add endpoint URL + page URL to `OfficialStatusSource::endpoint_url()` and `page_url()`
3. Add `source_method()` mapping (Statuspage V2, BetterStack, etc.)
4. Create or reuse adapter in `adapters/` (statuspage.rs covers all Statuspage V2 APIs)
5. Wire fetch logic in `fetch.rs`
6. Provider aliases (e.g., `github-copilot` → `github`) go in `STATUS_SOURCE_ALIASES`

## Key Types

- `ProviderStatus` — main output struct with slug, health, provenance, detail vectors, load state
- `ProviderHealth` — Operational | Degraded | Outage | Maintenance | Unknown (with `sort_rank()`)
- `StatusProvenance` — Official | Fallback | Unavailable (marks data source trustworthiness)
- `StatusLoadState` — Placeholder | Loaded | Partial | Failed
- `OfficialSnapshot`, `FallbackSnapshot` — intermediate results from adapters

## Assessment Helpers (assessment.rs)

Computed on-demand via `status.assessment()`:
- **Coverage** — None | SummaryOnly | IncidentOnly | ComponentOnly | Full
- **Freshness** — Fresh (≤6h) | Aging (≤24h) | Stale (>24h) | Unknown
- **Confidence** — High | Medium | Low | None (based on provenance, coverage, freshness, contradictions)
- **Contradictions** — "Operational summary with active incident", "Fallback stale snapshot is low-trust", etc.

User-facing helpers: `user_visible_caveat()`, `user_visible_affected_items()`

## Gotchas

- `OfficialStatusSource::endpoint_url()` is the canonical endpoint for each provider; must stay in sync with actual URLs
- Google Gemini uses JSON array at `/incidents.json` (not Statuspage V2); Google adapter handles specially
- Better Stack resources use `public_name` field (not `name`)
- Status.io `status_code = 400` means degraded (not error)
- incident.io (shim) and Instatus components may require secondary fetches for full detail
- Bounded concurrency in fetch: `set.len() >= 10` drains one task before spawning next (prevents thundering herd)
- Never assume empty detail vectors mean "none reported"; always check `*_state.availability`
