# Benchmarks Tab Audit

## Scope
Focused audit of the Benchmarks tab only, using the shared shell guidance in `docs/ui-tab-audit.md` and `docs/ui-design-guide.md`. This is read-only analysis; no UI code changes are proposed here.

## Benchmarks tab in the shared shell
The Benchmarks tab is the strongest example of the app using **explicit mode changes** instead of overloading one detail surface. In browse mode it keeps a classic three-column shell — `Creators | <selected creator or Benchmarks> | Details` — and in compare mode it swaps to a dedicated compare shell with a left selection rail plus an explicit sub-tab strip for `H2H | Scatter | Radar` (`src/tui/ui.rs:2013-2076`, `src/tui/ui.rs:2079-2101`). That matches the design guide's preference for concrete nouns and one-panel-one-job better than any other non-Models tab.

## What the tab does especially well
1. **Mode changes are explicit, not hidden.** Selection count automatically promotes `Detail -> H2H` and demotes back when comparison collapses, so the user gets a clean mode boundary instead of mixed compare chrome inside the detail pane (`src/tui/benchmarks_app.rs:969-979`, `src/tui/app.rs:1070-1095`).
2. **Panel naming is concrete and domain-based.** `Creators`, `Details`, `Head-to-Head`, `Scatter`, `Radar`, and `Sort By` all read like user concepts rather than implementation buckets (`src/tui/ui.rs:2131-2138`, `src/tui/ui.rs:2623-2639`, `src/tui/ui.rs:4118-4123`, `src/tui/ui.rs:4512-4513`, `src/tui/ui.rs:4931-4935`).
3. **The comparison views respect different reading jobs.** H2H supports ranked row-by-row reading, Scatter supports relationship scanning with crosshairs and a legend, and Radar stays visually separate instead of competing for space in one overloaded panel (`src/tui/ui.rs:2030-2050`, `src/tui/ui.rs:4109-4446`, `src/tui/ui.rs:4512-4894`).
4. **List titles carry useful state.** Browse titles expose count, search, source filter, reasoning filter, and active sort in the title bar, which is consistent with the design guide's preference for keeping counts/search/sort in titles (`src/tui/ui.rs:2456-2477`).
5. **The detail panel is information-dense but structured.** Identity and metadata stay near the top, then indexes, benchmark scores, performance, and pricing follow in stable section order (`src/tui/ui.rs:2642-2824`).

## Audit by category

### 1) Keybinds and interaction model
**Strengths**
- Benchmarks has a coherent split between navigation, filtering, sorting, selection, and compare-only actions in the event handler (`src/tui/event.rs:242-348`).
- `h/l` and `Tab` consistently switch focus, while `j/k/g/G/Ctrl+d/Ctrl+u` retarget based on focus, which preserves a familiar shell-level navigation grammar (`src/tui/event.rs:254-305`).
- Compare-specific actions are correctly gated: `v`, `d`, and `t` only activate with 2+ selections; `x/y` only activate in Scatter and `a` only in Radar (`src/tui/event.rs:322-343`).

**Weaknesses / drift**
- The help popup says `5` and `6` "Cycle region filter" and "Cycle type filter," but the code does **grouping toggles**, not filters (`src/tui/event.rs:312-316`, `src/tui/benchmarks_app.rs:863-883`, `src/tui/ui.rs:3632-3639`). This is the clearest terminology drift in the tab.
- Browse-mode footer compresses `5-6` into `group` and omits `o open AA`, while the help popup documents `o`. The footer under-signals an important action and over-compresses two distinct grouping controls (`src/tui/ui.rs:3270-3289`, `src/tui/ui.rs:3660-3669`).
- Compare-mode footer says `t creators/models`, but the left panel actually toggles between **creator rail** and **compact selected-model list**, which is a stronger distinction than the terse footer implies (`src/tui/app.rs:1120-1130`, `src/tui/ui.rs:2024-2028`, `src/tui/ui.rs:3218-3226`).

### 2) Sorts and filters
**Strengths**
- Quick-sort keys (`1/2/3`) cover the most useful pivots: intelligence, release date, and speed (`src/tui/event.rs:307-310`, `src/tui/app.rs:1049-1068`).
- Full sorting remains available via a dedicated centered picker, which is cleaner than cycling through a long linear sort list in the footer (`src/tui/app.rs:1021-1047`, `src/tui/ui.rs:4896-4943`).
- Filtering logic is layered and readable: source, reasoning, creator, search, then a null-filter that hides rows missing the active sort metric (`src/tui/benchmarks_app.rs:663-717`).

**Weaknesses / drift**
- The null-filter is operationally useful but easy to miss: sorting by a sparse metric silently removes entries, and the title only exposes the active sort label, not the fact that the dataset has been narrowed for data availability (`src/tui/benchmarks_app.rs:703-709`, `src/tui/ui.rs:2458-2477`).
- `5` and `6` live beside filters, but they alter creator presentation rather than result set membership. That placement is efficient for experts but semantically muddy for new users (`src/tui/event.rs:312-316`, `src/tui/ui.rs:2155-2167`).
- Compare-mode list title drops the selected creator context and reverts to generic `Models (...)`, which is correct for the left rail's content but removes the stronger browse-mode anchor when the user is comparing within a creator slice (`src/tui/ui.rs:2302-2319` vs. `src/tui/ui.rs:2456-2477`).

### 3) Browse/compare panel layout
**Strengths**
- Browse mode uses a stable 20/40/40 split that keeps creators lightweight and gives equal weight to list and detail (`src/tui/ui.rs:2053-2065`).
- Compare mode deliberately switches shells instead of trying to squeeze H2H/Scatter/Radar under the browse layout (`src/tui/ui.rs:2016-2051`).
- The compare left rail is capped at 30% width with a minimum 35 columns, preventing the comparison surface from being starved (`src/tui/ui.rs:2017-2022`).

**Weaknesses / drift**
- Compare mode's left rail can flip between `Creators` and compact `Models`, which changes both content and interaction target within the same spatial slot. It works, but it reduces spatial certainty compared with the browse shell's stable creator rail (`src/tui/ui.rs:2024-2028`, `src/tui/app.rs:1120-1130`).
- The sub-tab strip is visually minimal — just bracketed labels on a single line — so view state depends on learned convention more than on strong panel ownership cues (`src/tui/ui.rs:2079-2101`).
- Detail overlay is only available from H2H and not from Scatter/Radar, which is a sensible shortcut choice but creates a small interaction asymmetry inside compare mode (`src/tui/event.rs:342-347`, `src/tui/ui.rs:2068-2070`, `src/tui/ui.rs:2840-2866`).

### 4) Naming and labels
**Strengths**
- Major surfaces use concrete nouns (`Creators`, `Details`, `Head-to-Head`, `Scatter`, `Radar`, `Sort By`) and align with the design guide's naming rules.
- Detail sections also stay domain-specific: `Indexes`, `Benchmarks`, `Performance`, `Pricing` (`src/tui/ui.rs:2750-2811`).

**Weaknesses / drift**
- `Rgn`, `Typ`, `R`, `NR`, `AR`, `O`, and `C` are efficient but require prior legend knowledge; most are only fully explained in the help popup or by context (`src/tui/ui.rs:2155-2160`, `src/tui/ui.rs:2515-2521`, `src/tui/ui.rs:2570-2599`).
- The help popup's "filter" wording for region/type is inaccurate, so one of the tab's main labels teaches the wrong mental model (`src/tui/ui.rs:3622-3643`).

### 5) Spacing and density
**Strengths**
- Browse list density is high without feeling chaotic because the dynamic name column expands around the active metric group (`src/tui/ui.rs:2486-2505`).
- Detail rows use fixed two-column metadata and metric sections, which keeps comparison inside a single model detail fast on repeat reads (`src/tui/ui.rs:2648-2815`).
- Creator group headers visually separate region/type clusters without extra boxes (`src/tui/ui.rs:2183-2205`).

**Weaknesses / drift**
- The creator rail spends its first content row on inline grouping hints, reducing visible list real estate for a panel that is already only 20% wide in browse mode (`src/tui/ui.rs:2155-2167`, `src/tui/ui.rs:2053-2060`).
- The detail panel ends with inline action hints (`c copy`, `o open AA`) inside the content body, which slightly violates the guide's preference to keep control hints in the global footer/help layer (`docs/ui-design-guide.md:19-25`, `src/tui/ui.rs:2825-2833`).

### 6) Data structure and presentation
**Strengths**
- H2H precomputes win counts and ranks each metric, so the view answers "who wins where?" immediately instead of forcing manual row-by-row comparison (`src/tui/ui.rs:4155-4207`, `src/tui/ui.rs:4424-4479`).
- Scatter includes average crosshairs, auto log scaling for skewed axes, and a legend that marks off-chart selections, which makes the visualization more useful than a bare point cloud (`src/tui/ui.rs:4542-4894`).
- Detail surfaces keep source/open-weights status, reasoning status, tools, context, and pricing close to core benchmark metrics, which helps bridge performance with operational practicality (`src/tui/ui.rs:2670-2748`).

**Weaknesses / drift**
- Compare mode uses different nouns for the same underlying objects (`Models` in compact list, model names in H2H header, creator metadata inside H2H), which is correct but dense; the view asks the user to continuously translate between creator slice, selected models, and metric scoreboard.
- Browse list headers expose abbreviations like `LCode`, `SciCd`, `IFB`, and `Bld $/M`; they are compact, but the cognitive load is higher than in Models or Agents where headers are more self-evident (`src/tui/ui.rs:2486-2505`, `src/tui/ui.rs:2939-2978`).

### 7) Scan path
**Browse mode scan path**
1. Start in `Creators` to choose a slice (`src/tui/ui.rs:2104-2249`).
2. Move to the benchmark list, where title state + column headers explain the current slice and sorting (`src/tui/ui.rs:2458-2539`).
3. Read `Details` for one model, top-down from identity to metadata to metrics (`src/tui/ui.rs:2623-2833`).

This is a strong repeat-use path because navigation and explanation stay in separate panels.

**Compare mode scan path**
1. Keep or adjust selected models in the left rail (`src/tui/ui.rs:2267-2420`).
2. Use the single-line sub-tab strip to choose `H2H`, `Scatter`, or `Radar` (`src/tui/ui.rs:2079-2101`).
3. Read the compare view based on task: ranked H2H table, relationship plot, or radar profile (`src/tui/ui.rs:4109-4894`).

This is powerful, but slightly less teachable than browse mode because the left rail's role can change and the sub-tab strip is visually subtle.

### 8) Repeat-use stability
**Stable**
- Browse mode shell is stable.
- Compare mode auto-entry/exit is stable.
- Detail section order is stable.
- Footer/help both treat Benchmarks as a high-control tab with explicit compare affordances.

**Less stable / relearning risk**
- Region/type controls are labeled as filters in help but behave as grouping toggles in code.
- The compare left rail changes identity (`Creators` vs `Models`) under one shortcut.
- Some important semantics are encoded in abbreviations rather than explicit labels.

## Priority findings

### Keep
- The two-shell browse/compare structure.
- Concrete compare view naming (`H2H`, `Scatter`, `Radar`).
- The dense but well-sectioned detail panel.
- H2H ranking plus scatter legend/crosshair behavior.

### Fix first in future design work
1. **Correct the interaction vocabulary around keys `5` and `6`.** They are grouping toggles, not filters.
2. **Tighten footer/help consistency.** Footer should expose `o open AA`; help/footer should describe `t` and grouping the same way.
3. **Reduce semantic drift in the compare left rail.** Either strengthen the label when it flips or make the panel role more explicit.
4. **Move inline detail actions out of the detail body if possible.** The footer/help layer already exists.
5. **Consider surfacing "missing-data sort narrowing" more explicitly** when the active sort hides incomplete rows.

## Bottom line
The Benchmarks tab is already a strong model for the rest of the app: explicit modes, concrete nouns, clear section ownership, and a high-value compare workflow. Its main problems are not structural; they are **terminology drift and control discoverability drift**. That makes it a good candidate to preserve as a reference shell while tightening labels, footer/help consistency, and compare-left-rail semantics.
