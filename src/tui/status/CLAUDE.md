# Status Tab

## Files
- `app.rs` — `StatusApp` state, `StatusFocus` (List/Details), `OverallPanelFocus` (Incidents/Degradation/Maintenance), `DetailPanelFocus` (Services/Incidents/Maintenance)
- `render.rs` — `draw_status_main()` (dispatch), list rendering, shared `pub(super)` helpers (icon/style/label functions)
- `overall.rs` — `draw_overall_dashboard`, card builders (incidents, degradation, maintenance)
- `detail.rs` — `draw_provider_status_detail`, `sorted_active_incidents`, `sorted_components`, `build_services_title`

## Key Patterns
- `StatusApp` is `Option<StatusApp>` on `App` — constructed when status data first arrives
- `StatusApp::new(&Config)` takes a `&Config` reference (not `&AgentsFile`) — status tab is independent of agents data loading
- `tracked: HashSet<String>` field on `StatusApp` holds the set of provider slugs to display — providers not in this set are hidden by `update_filtered()`
- `show_picker: bool` and associated picker state on `StatusApp` control the provider tracking modal (press `a` to open)
- `apply_fetch()` merges incoming provider status by slug (not full replacement) — existing entries are updated in place, new slugs are inserted
- `update_filtered()` rebuilds the visible provider list, hiding untracked providers (those whose slug is not in `tracked`)
- Overall view: gauge + icon+count legend + 3 SoftCard panels (incidents, degradation, maintenance)
- Provider detail view: gauge header with icon legend, grouped services panel, incidents + maintenance (horizontal when wide)
- `OverallPanelFocus` cycles through overall panels, `DetailPanelFocus` cycles through detail panels — both via h/l
- Services panel: always expanded, grouped by `group_name` with aggregate health headers, scrollable
- Maintenance icons: ◇ = scheduled, ◆ = active/in-progress (both Blue)
- Component status colors: ◐ Yellow = degraded_performance, ◐ Red = partial_outage, ✗ Red = major_outage
- Shared helpers in `render.rs` are `pub(super)` — accessible to `overall.rs` and `detail.rs` but not outside `status/`
- `issue_count()` excludes maintenance components — planned work is not an issue
