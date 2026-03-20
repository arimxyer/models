# Status Tab Focused Audit

## Role in the doc stack
This is a **supporting evidence** document.

Use it as input to the canonical docs:
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`
- `docs/status-tab-redesign-spec.md`


## Scope
Read-only audit of the current Status tab. This pass covers keybinds, implicit sort/filter behavior, panel layout, naming, spacing, data structure and presentation, scan path, repeat-use stability, and what the tab already does well.

## Evidence anchors
- `src/tui/event.rs:365-413` — Status-tab keybindings
- `src/tui/status_app.rs:45-118` — state shape, fetch application, implicit list sort
- `src/tui/status_app.rs:126-152` — search/filter behavior
- `src/tui/ui.rs:267-353` — left rail layout and title treatment
- `src/tui/ui.rs:357-688` — right-panel structure, section rendering, footer behavior
- `src/status.rs:705-800` — provider data model, active-incident selection, caveat derivation
- `docs/ui-design-guide.md:26-53` — naming + field-placement rules
- `docs/ui-design-guide.md:55-150` — Status-tab shell contract and canonical section rules

## What the Status tab already does especially well
1. **The provider rail is finally navigation-first.**
   - Rows are one line tall and limited to health icon, provider name, and optional incident count (`src/tui/ui.rs:308-343`).
   - That matches the guide's lightweight-rail contract (`docs/ui-design-guide.md:55-60`).
2. **The tab has a credible operational severity sort.**
   - Fetched entries sort by health, then support tier, then provenance, then display name (`src/tui/status_app.rs:105-113`).
   - Operationally broken providers rise to the top without extra user work.
3. **The right side already speaks in mostly domain terms inside the body.**
   - `Current incidents`, `Services`, and `Maintenance` are concrete, user-facing sections (`src/tui/ui.rs:452-615`).
4. **The renderer avoids fake service detail.**
   - When components are absent, the UI hides the Services section and emits `Service details unavailable` instead (`src/tui/ui.rs:373-379`, `621-639`).
   - That is aligned with the guide's truthfulness requirement (`docs/ui-design-guide.md:140-150`).

## Keybind audit
### Current bindings
- navigation in list: `j`/`k`, arrows, `g`, `G`, `Ctrl-d`, `Ctrl-u`, `PageDown`, `PageUp` (`src/tui/event.rs:374-404`)
- focus switch: `h`/`l`, arrows, `Tab`, `BackTab` (`src/tui/event.rs:405-407`)
- search: `/`, `Esc` (`src/tui/event.rs:408-409`, `417-422`)
- open source page: `o` (`src/tui/event.rs:410`)
- refresh: `r` (`src/tui/event.rs:411`)
- service density toggle: `c` (`src/tui/event.rs:412`)

### Findings
- The core navigation set is strong and consistent with the rest of the app.
- `c` is the weakest affordance in the tab. It changes service density, but the mode signal lives in the body panel title as `Narrative [c expanded]` instead of a domain-owned label (`src/tui/ui.rs:665-675`).
- There is no visible sorting control because sort is fixed in code. That is okay if the order is stable, but it should be described as an assessment-first default rather than left implicit.

## Sort + filter audit
### Sort behavior
The tab applies a hidden multi-key sort when fetches land:
1. health severity
2. support tier
3. provenance
4. display name
(`src/tui/status_app.rs:105-113`)

### Filter behavior
Search matches:
- display name
- slug
- source label
- summary
(`src/tui/status_app.rs:126-146`)

### Findings
- The severity-first sort is useful for first-use triage.
- The sort is **not explainable from the UI**. The Providers title exposes count and search query only (`src/tui/ui.rs:293-306`), so repeat users have to remember a hidden ranking policy.
- Search over `summary` is semantically odd because the current detail renderer does not visibly present `summary`; it builds the page from incidents, services, maintenance, source metadata, and caveats instead (`src/tui/ui.rs:357-688`). This means search can match text the user cannot later relocate on the screen.
- Support tier and provenance affect ordering but are not visible in the list. That keeps the rail clean, but it weakens user explainability when two providers with similar health appear in a surprising order.

## Panel layout audit
### Current shell
- fixed two-column layout: 32-char provider rail + flexible detail pane (`src/tui/ui.rs:277-280`)
- detail pane splits into up to three bordered boxes:
  - `Status page`
  - `Narrative`
  - optional `Note`
  (`src/tui/ui.rs:641-687`)

### Findings
- The left side is stable.
- The right side is still a special-case shell, not a peer of Models/Agents/Benchmarks. The guide explicitly prefers a stable detail surface with domain sections in a fixed order (`docs/ui-design-guide.md:61-98`, `116-150`), but the current layout still relies on outer-box role names plus an internal mixed-content scroll region.
- The hero box is functioning as **Overview**, but it is labeled `Status page` instead of a reusable product noun (`src/tui/ui.rs:657-663`).
- The main scroll body is functioning as a composite detail surface, but the title `Narrative` hides what content lives inside (`src/tui/ui.rs:665-675`).
- The footer note box is useful only when caveats exist, but splitting it into a separate border makes the page feel like three independent products rather than one structured details surface.

## Naming audit
### Strong names already present
- `Providers`
- `Current incidents`
- `Services`
- `Maintenance`

### Drift / weak names
- `Status page` (`src/tui/ui.rs:661`)
- `Narrative` / `Narrative [c expanded]` (`src/tui/ui.rs:670-674`)
- `Note` (`src/tui/ui.rs:685`)

### Findings
- The guide forbids abstract/internal labels like `Narrative`, `Insight`, `Context`, and wants concrete nouns such as `Overview`, `Services`, `Maintenance`, `Notes` (`docs/ui-design-guide.md:26-45`, `109-123`).
- `Narrative` is the biggest remaining naming failure because it conceals changing meaning under one recurring title.
- `Note` is less harmful than `Narrative`, but it is still vaguer than `Notes` and still framed as a separate mini-panel instead of a fixed section within the detail surface.

## Spacing + rhythm audit
### Current rhythm
- hero section height depends on how many hero lines are present (`src/tui/ui.rs:419-448`)
- incident/service/maintenance blocks are separated with blank lines inside one scrolling body (`src/tui/ui.rs:450-619`)
- footer notes break out into their own bordered box when present (`src/tui/ui.rs:641-687`)

### Findings
- Internal section spacing is readable enough once the user is inside the body.
- The main rhythm problem is **between bordered boxes, not inside them**. Three stacked borders create unnecessary chrome for one detail experience.
- The hero metadata line compresses `source`, `time`, and optional `official page` into one bullet row (`src/tui/ui.rs:433-440`). That is dense, but it undercuts the guide's fixed-slot principle because the eye must parse separators instead of scanning labeled rows.
- The `Affected right now:` line is useful when incidents exist (`src/tui/ui.rs:540-544`), but it competes with the service list for the first slot in the Services section. It reads like summary metadata rather than a service row.

## Data structure and presentation audit
### Data structures the renderer is built from
- `ProviderStatus` owns health, provenance, source metadata, components, incidents, maintenance, and error (`src/status.rs:705-722`)
- `active_incidents()` filters incident state (`src/status.rs:752-757`)
- `user_visible_affected_items()` derives affected surfaces/components (`src/status.rs:759-784`)
- `user_visible_caveat()` derives compact notes (`src/status.rs:786-800`)

### Findings
- The data model is richer than the current presentation shell.
- Presentation is strongest when it uses explicit structures directly:
  - incidents -> incident section
  - components -> service rows
  - maintenance -> maintenance rows
- Presentation is weakest when it synthesizes mixed metadata bundles:
  - hero meta line combines source + time + official-page hint (`src/tui/ui.rs:433-440`)
  - `Narrative` owns all interior sections even though those sections are already domain-distinct (`src/tui/ui.rs:665-675`)
- The app has enough structure to support the guide's canonical `Overview / Current incidents / Services / Maintenance / Notes` model already; the current drift is mostly layout ownership and naming, not missing data.

## Scan-path audit
### First-use scan path today
1. pick provider from left rail
2. read verdict in `Status page`
3. decode the compressed source/time/official-page metadata row
4. enter `Narrative` and infer whether the next visible block is incidents, services, or maintenance
5. optionally notice `Note` at the bottom

### Repeat-use scan-path quality
- Repeat users can probably find verdicts and incidents quickly.
- Repeat users still pay a small but recurring interpretation tax because the outer-box names do not tell them where Overview ends and section ownership begins.
- `Narrative` especially hurts scanning because the same box title can front different combinations of incidents, services, and maintenance.

## Repeat-use stability audit
### Stable now
- provider rail structure
- severity-first default ordering
- domain section headers inside the body
- caveat fallback when service detail is unavailable

### Still unstable / still drifting
1. **Overview is not explicitly named as Overview.**
   - The guide expects a fixed semantic slot; the current hero is just `Status page` (`src/tui/ui.rs:657-663`, `docs/ui-design-guide.md:61-98`).
2. **Metadata is still bundled, not slotted.**
   - Source and time occupy one bullet row (`src/tui/ui.rs:433-440`) instead of clearly labeled stable rows.
3. **The body title changes meaning less than before, but still too much.**
   - `Narrative` remains the top-level owner of incidents, services, and maintenance (`src/tui/ui.rs:665-675`).
4. **The compact/expanded toggle mutates the wrong label.**
   - The guide allows suffixing the section being compacted, but the current UI mutates the body container title instead of the Services section alone (`docs/ui-design-guide.md:109-114`, `src/tui/ui.rs:529-539`, `665-675`).

## Design conclusions
### Highest-confidence keep
- one-line provider rail
- severity-first provider ordering
- domain sections for incidents/services/maintenance
- truthful caveat handling for missing service detail

### Highest-confidence rewrite targets
1. Replace `Status page` with an explicit `Overview` section or make the outer panel `Status` and move `Overview` into the body.
2. Delete `Narrative` as a concept.
3. Convert hero metadata from one bullet row into fixed labeled rows: `Source`, time label, then incident count.
4. Keep `Services (expanded)` as a section-level variant if needed, but stop making the outer detail container carry mode meaning.
5. Fold `Note` into a proper `Notes` section within the stable reading order unless there is a strong reason to preserve a separate footer surface.

## Recommended artifact follow-up
Use this audit to tighten:
- `docs/ui-design-guide.md`
- `docs/status-tab-redesign-spec.md`

Specifically, the next doc pass should make hidden sort semantics, search-on-invisible-summary behavior, and container-title ownership explicit so the Status tab cannot regress back into mixed-purpose shell language.