# Status Tab

## Files
- `app.rs` — `StatusApp` state, `StatusFocus` (List/Details), `OverallPanelFocus` (Incidents/Degradation/Maintenance)
- `render.rs` — `draw_status_main()`, overall dashboard with accent stripe softcards, provider detail view

## Key Patterns
- `StatusApp` is `Option<StatusApp>` on `App` — constructed when status data first arrives
- Overall view shows 3 panels (incidents, degradation, maintenance) with health-colored `"▎"` accent stripes per provider card
- Provider detail view shows incidents, components, scheduled maintenance with scroll support
- `OverallPanelFocus` cycles through visible panels — panels with no content are skipped
- Detail view accent stripes colored by incident stage (Yellow=investigating, Green=resolved, Cyan=monitoring, Blue=maintenance)
