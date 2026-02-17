# Plan: Remove benchmark disk cache

## Rationale
- Models tab: fetches from API every launch, no cache
- Agents tab: uses disk cache only as warm-start, always fetches fresh in background
- Benchmarks tab: has a full 6h TTL disk cache with schema versioning — unnecessary complexity
- jsDelivr CDN is fast (~100ms), and we now purge it on data updates
- The disk cache introduced `BenchmarkSchemaCoverage`, `benchmark_entries_compatible()`, `DATA_SCHEMA_VERSION`, `CACHE_VERSION` — all removable

## Changes

### 1. Simplify `src/tui/mod.rs` startup
- Remove `BenchmarkCache::load()` and fresh-check logic
- Always spawn the CDN fetch (unconditionally, no ETag)
- Start with `BenchmarkStore::empty()` (already the fallback today)
- Keep the `BenchmarkFetchResult::Fresh` handler but remove cache save

### 2. Simplify `src/benchmark_fetch.rs`
- Remove ETag conditional request support (no cache = no ETag to send)
- Remove `NotModified` variant from `BenchmarkFetchResult`
- Just fetch and return `Fresh(entries)` or `Error`

### 3. Delete `src/benchmark_cache.rs`
- Remove the entire module — no more `BenchmarkCache`, `CACHE_VERSION`, `DATA_SCHEMA_VERSION`

### 4. Remove `BenchmarkSchemaCoverage` from `src/benchmarks.rs`
- Remove `BenchmarkSchemaCoverage` struct and `from_entries()`
- Remove `benchmark_entries_compatible()` function
- These only existed to validate cache staleness

### 5. Clean up imports and `mod` declarations
- Remove `mod benchmark_cache` from `src/main.rs` or `src/lib.rs`
- Remove unused imports in `src/tui/mod.rs`

### 6. Delete cache file reference from CLAUDE.md / memory
- Update docs to reflect the simpler architecture
