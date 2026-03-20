# Agents Tab

## Files
- `app.rs` — `AgentsApp` state, `AgentFocus` (List/Details), `AgentSortOrder`, `AgentCategory`, `AgentFilters`
- `render.rs` — `draw_agents_main()` (list + detail), `draw_picker_modal()` (source tracking popup)

## Key Patterns
- `AgentsApp` is `Option<AgentsApp>` on `App` — constructed after agents file loads, not at startup
- Changelog search with `n`/`N` match navigation uses `search_matches: Vec<usize>` (line indices into rendered markdown)
- Detail scroll uses `detail_scroll: u16` — counts visual wrapped lines, not logical lines
- Source picker modal intercepts global keys (especially `q`) to prevent accidental quit
- Service health display: agents with status provider mappings show health icon + label in detail panel via `resolve_agent_service_health()`
