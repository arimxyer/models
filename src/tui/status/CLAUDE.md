# Status Tab

## Files
- `app.rs` — `StatusApp` state, `StatusFocus` (List/Details), `OverallPanelFocus` (Incidents/Degradation/Maintenance)
- `render.rs` — `draw_status_main()` (dispatch), list rendering, shared helpers (icon/style/label functions)
- `overall.rs` — `draw_overall_dashboard`, card builders (incidents, degradation, maintenance)
- `detail.rs` — `draw_provider_status_detail`, `sorted_active_incidents`, `sorted_components`

## Key Patterns
- `StatusApp` is `Option<StatusApp>` on `App` — constructed when status data first arrives
- Overall view shows 3 panels (incidents, degradation, maintenance) with health-colored `"▎"` accent stripes per provider card
- Provider detail view shows incidents, components, scheduled maintenance with scroll support
- `OverallPanelFocus` cycles through visible panels — panels with no content are skipped
- Detail view accent stripes colored by incident stage (Yellow=investigating, Green=resolved, Cyan=monitoring, Blue=maintenance)
- Shared helpers in `render.rs` are `pub(super)` — accessible to `overall.rs` and `detail.rs` but not outside `status/`
