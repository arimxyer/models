# Cross-Tab UI Audit

## Role in the doc stack
This is a **canonical synthesis document**.

Use this file to understand:
- the shared product shell across tabs
- where each tab is strongest
- where each tab drifts
- what should be standardized in the design guide

Supporting evidence lives in:
- `docs/models-tab-lane-audit.md`
- `docs/agents-tab-audit.md`
- `docs/benchmarks-tab-audit.md`
- `docs/status-tab-focused-audit.md`
- `docs/cross-tab-controls-interaction-audit.md`
- `docs/status-tab-multi-pass-audit.md`

## Goal
Define the shared interaction and layout language of the app so future tab work can improve consistency without flattening the strengths of individual tabs.

## Audit method
Every tab should be reviewed through the same lens:
1. keybinds and interaction model
2. sorts / filters / toggles / modes
3. panel layout and panel ownership
4. naming and title semantics
5. spacing / rhythm
6. data structure and presentation
7. first-use scan path
8. repeat-use stability
9. strongest keepers
10. highest-confidence rewrite targets

## Shared shell invariants
Across Models, Agents, Benchmarks, and Status, the app already has a recognizable product shell:
- global tab bar at the top
- global shortcut/help footer at the bottom
- cyan focus border and dark-gray unfocused border
- navigation-first rails on the left
- explanation/detail surfaces on the right
- counts/search/sort/filter state usually surfaced in titles rather than scattered into the body

These are product-level strengths and should be preserved.

## Per-tab synthesis

### Models
**Current shell**
- `Providers | Models | Details`
- the clearest left-to-right ownership model in the app

**What it does especially well**
- strongest scan path and repeat-use stability
- concrete section naming inside details (`Capabilities`, `Pricing`, `Limits`, etc.)
- title-level state exposure instead of extra body chrome
- lightweight navigation rails

**What should be standardized from it**
- navigation rail discipline
- title truthfulness
- one dominant reading order in the detail panel

**What should not be copied blindly**
- stacked provider-summary micro-panel patterns only work when the upper panel has a narrow, stable job
- terse shorthand like `RTFO` is efficient, but not a good general default for broader UX surfaces

### Agents
**Current shell**
- `Agents | Details`
- content-sized rail + long-form detail surface

**What it does especially well**
- stable detail identity (`Details`)
- good metadata-at-the-top structure
- clear separation between navigation and explanation

**Where it drifts**
- dormant/hidden IA around categories and tracked-vs-untracked discovery
- some inline detail-body action hints duplicate footer/help responsibilities

**What should be standardized from it**
- one strong detail surface can be enough
- long-form detail panels still need stable top metadata slots

### Benchmarks
**Current shell**
- browse mode: `Creators | Benchmarks | Details`
- compare mode: compact left rail + explicit compare view (`H2H`, `Scatter`, `Radar`)

**What it does especially well**
- strongest explicit mode/view naming in the app
- complex controls remain coherent because the views are named and bounded
- powerful compare workflow without collapsing everything into one mixed panel

**Where it drifts**
- some control labeling is inaccurate (`filter` vs grouping)
- compare left rail changes identity depending on mode, which reduces spatial certainty
- abbreviations are efficient but somewhat expert-coded

**What should be standardized from it**
- alternate views should be explicit and named after the thing the user is seeing
- complex control surfaces are okay if the mode boundaries are obvious

### Status
**Current shell**
- provider rail + custom stacked right-side shell
- right side currently behaves more like a special-case microsite than a peer tab

**What it does especially well now**
- provider rail is finally navigation-first
- operational severity sort is useful
- incidents / services / maintenance are trending toward domain sections
- missing service detail is handled more honestly than before

**Where it drifts most**
- abstract container naming (`Narrative`, `Note` in prior/current variants)
- hidden or weakly surfaced semantics in hero metadata and controls
- section ownership is not yet as stable as in Models / Agents / Benchmarks
- mode/control disclosure is weaker than the actual feature set

**What this means**
Status should not be tuned as an isolated status-page clone. It should be rebuilt as a stable member of the app family.

## Cross-tab strengths worth standardizing
1. **Concrete panel names**
   - prefer domain nouns: `Providers`, `Agents`, `Details`, `Current incidents`, `Services`
2. **Stable panel responsibility**
   - rails navigate, detail panes explain
3. **Title-level state exposure**
   - count, search, sort, filter, or mode belongs in titles when possible
4. **Explicit mode/view naming**
   - when alternate views exist, name them directly
5. **Stable top metadata slots**
   - the most important status/identity information should be in fixed places

## Cross-tab drift patterns to watch for
1. abstract or implementation-bucket names
2. hidden interaction state that is not disclosed in footer/help
3. search matching invisible content the user cannot relocate
4. conditional layout changes that alter semantic location too much
5. expert shorthand becoming default user-facing language

## Design implications
The app should aim for:
- stable field placement
- stable section ownership
- explicit labels for changing semantics
- minimal body chrome
- explicit controls when modes or views change meaningfully

## Canonical next use
Use this file together with:
- `docs/ui-design-guide.md` for rules
- `docs/status-tab-redesign-spec.md` for Status implementation
