# Models Tab

## Files
- `app.rs` — `ModelsApp` state, `Focus` (Providers/Models), `SortOrder`, `Filters`, `ProviderListItem`, `ModelEntry`
- `render.rs` — `draw_main()` renders the 3-column layout (providers | model list | detail panel)

## Key Patterns
- `ModelsApp::update_filtered_models(&mut self, providers)` takes `&[(String, Provider)]` param — providers live on `App`, not `ModelsApp`
- `model_list_state` uses `select(Some(idx + 1))` offset because row 0 is the column header
- `ProviderListItem::CategoryHeader` items are non-selectable — `find_selectable_index()` skips them
- Sort/filter methods (`cycle_sort`, `toggle_reasoning`, etc.) live on `ModelsApp` and call `update_filtered_models` internally
