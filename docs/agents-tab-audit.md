# Agents Tab Audit

## Scope
Read-only audit of the current Agents tab against the shared shell described in `docs/ui-tab-audit.md` and `docs/ui-design-guide.md`.

## Shared-shell alignment

### What already matches the guide
1. **Concrete two-panel shell**
   - The tab keeps the same product-family pattern described in `docs/ui-tab-audit.md:37-51`: a compact navigation rail on the left and a single `Details` reading surface on the right.
   - The current implementation preserves that shell with a content-sized `Agents` rail and one `Details` panel (`src/tui/ui.rs:1120-1142`, `src/tui/ui.rs:1611-1645`).
2. **Clear navigation vs explanation split**
   - List focus and detail focus are explicit (`src/tui/agents_app.rs:69-74`, `src/tui/agents_app.rs:371-376`, `src/tui/event.rs:203-205`).
   - Navigation keys stay in the rail; vertical scroll transfers to the detail surface when focus moves right (`src/tui/event.rs:145-201`).
3. **Operational metadata is front-loaded**
   - Name/version, repo/stars, install state, latest release date, and cadence appear before changelog history (`src/tui/ui.rs:1367-1460`).
   - This matches the guide's preference for stable metadata near the top of the detail surface (`docs/ui-tab-audit.md:43-51`, `docs/ui-design-guide.md:47-53`).

## Strengths worth preserving

### 1. The rail is lightweight and scan-friendly
- The list title carries count/search/filter/sort context without adding extra panels (`src/tui/ui.rs:1165-1203`).
- The rail width is content-sized rather than fixed-wasteful, which keeps the detail surface dominant (`src/tui/ui.rs:1120-1139`).
- The status dot + type column gives quick triage value in one row (`src/tui/ui.rs:1257-1330`).

### 2. The detail surface has one stable identity
- The right panel stays named `Details`, which aligns with the preferred noun-based naming in `docs/ui-design-guide.md:26-45`.
- It avoids the abstract labels called out as weak elsewhere in the shared audit (`docs/ui-tab-audit.md:90-107`).

### 3. Search-aware scroll behavior is strong for repeat use
- Search matches are tracked against wrapped-line visual offsets, so jumping between matches lands accurately in long release notes (`src/tui/ui.rs:1569-1633`, `src/tui/agents_app.rs:461-498`).
- This is one of the more polished high-volume reading flows in the app.

## UX drifts / weaknesses

### 1. Discovery is hidden behind a tracked-only primary filter
- The main list filters out every untracked agent before any visible filter toggles are applied (`src/tui/agents_app.rs:227-230`).
- The only way to discover or add untracked agents is the separate picker modal (`src/tui/agents_app.rs:397-458`, `src/tui/ui.rs:3819-3909`).
- Result: the rail looks like the full catalog, but it is actually a private shortlist. That weakens onboarding and makes the `Agents` title less literal than the rest of the app.

### 2. The information architecture exposes dormant categories in code, not in UX
- `AgentCategory` includes `Installed`, `CLI Tools`, `IDEs`, and `Open Source` (`src/tui/agents_app.rs:37-66`).
- Filtering logic and title formatting account for category state (`src/tui/agents_app.rs:209-250`, `src/tui/agents_app.rs:500-520`).
- But the event map exposes only the boolean filters and sort actions; there is no user path to switch category (`src/tui/event.rs:212-226`).
- Result: the tab has an unrealized IA layer that increases code complexity without improving the visible scan path.

### 3. The detail surface mixes stable summary with archival history too early
- The current detail panel starts well, then immediately expands into full release history and changelog text inside the same panel (`src/tui/ui.rs:1463-1561`).
- This keeps one panel, but it weakens the guide's `one panel, one job` rule (`docs/ui-design-guide.md:11-16`) because the surface is doing both object summary and deep archival reading.
- Repeat users can adapt, but first-pass users have to visually separate "what this agent is" from "all historical release notes" on their own.

### 4. Control hints leak into content that already has a global footer/help model
- The detail panel appends `o / r / c / n/N` key hints inside the content body (`src/tui/ui.rs:1546-1561`).
- The app already owns a global footer/help contract (`docs/ui-tab-audit.md:6-15`, `docs/ui-design-guide.md:18-24`) and an Agents-specific footer/help section (`src/tui/ui.rs:3200-3213`, `src/tui/ui.rs:3524-3569`).
- Result: a small but real duplication of chrome inside the main reading surface.

### 5. Rail vocabulary is solid, but status semantics are compressed
- The header uses `St / Agent / Type` and the rows compress install/fetch/update state into a single glyph (`src/tui/ui.rs:1257-1304`).
- This is compact, but more cryptic than the rest of the app's explicit labels. It works best only after the help legend is learned (`src/tui/ui.rs:3569-3585`).

## Scan-path assessment

### First use
- Good: selection, sort/search state, and primary metadata are easy to find.
- Weak: users may not realize the list is tracked-only, may not understand why certain agents are absent, and may not immediately parse the status glyphs.

### Repeat use
- Strong for users who primarily monitor tracked tools and skim release notes.
- Less strong for users trying to compare the broader agent landscape or manage tracking as a first-class workflow.

## Recommendations

### Keep
1. Keep the two-column `Agents | Details` shell.
2. Keep the rail lightweight and content-sized.
3. Keep `Details` as the right-panel noun.

### Change next
1. **Make catalog scope explicit**
   - Either expose the full catalog in the rail and treat tracking as a visible filter/state, or rename/title the rail so it clearly represents tracked agents only.
2. **Resolve the dormant category layer**
   - Either surface category switching in the UI, or remove the unused category abstraction and keep only the visible boolean filters.
3. **Split summary from history within the same detail surface**
   - Preserve one right-hand panel, but create explicit internal sections such as `Overview` and `Release history` so the scan path is clearer.
4. **Move action hints back to footer/help ownership**
   - Keep the detail body focused on agent information, not controls.
5. **Consider slightly more explicit rail semantics for status**
   - If the glyph legend remains, reinforce it through title copy or a clearer header label than `St`.

## Bottom line
The Agents tab is already one of the app's more stable shells. Its main design debt is not layout churn; it is hidden scope. The rail behaves like a tracked shortlist while presenting itself like a complete catalog, and the detail surface behaves like both a profile and an archive. Fixing those two issues would improve first-use clarity without giving up the tab's current strengths.
