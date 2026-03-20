# UI Design Artifact Rewrite Plan

## Role in the doc stack
This is a **supporting planning artifact** document.

Use it as input to the canonical docs:
- `docs/ui-tab-audit.md`
- `docs/ui-design-guide.md`
- `docs/status-tab-redesign-spec.md`


## Goal
Turn the current audit/spec set into a tighter design-document stack that can guide implementation without interpretation drift.

This synthesis uses:
- the shared shell audit in `docs/ui-tab-audit.md:6-124`
- the reusable rules in `docs/ui-design-guide.md:18-167`
- the implementation-grade Status spec in `docs/status-tab-redesign-spec.md:24-292`
- the fresh Status evidence pass in `docs/status-tab-focused-audit.md`
- current shell/title behavior in `src/tui/ui.rs:267-353`, `src/tui/ui.rs:930-981`, `src/tui/ui.rs:1150-1203`, `src/tui/ui.rs:1588-1655`, and `src/tui/ui.rs:2267-2326`
- current Status interactions in `src/tui/event.rs:365-413` and `src/tui/status_app.rs:105-152`

## Cross-tab synthesis
### What is already working across the product family
1. **Panel titles usually expose count, search, filter, or sort state.**
   - Models surfaces provider-specific title + count + search/filter/sort (`src/tui/ui.rs:943-981`).
   - Agents surfaces count + search/filter/sort in the list title (`src/tui/ui.rs:1165-1203`).
   - Benchmarks surfaces count + search + source/reasoning filters + sort (`src/tui/ui.rs:2281-2326`).
2. **The best tabs keep left-side navigation lightweight and right-side detail ownership stable.**
   - Models and Agents already behave this way (`docs/ui-tab-audit.md:19-51`, `src/tui/ui.rs:1588-1655`).
3. **Mode changes are strongest when they are explicit.**
   - Benchmarks names alternate views directly instead of hiding them inside a mixed panel (`docs/ui-tab-audit.md:53-67`).

### Where the doc stack is still too soft
1. **The shared audit is shell-strong but audit-process-light.**
   - `docs/ui-tab-audit.md` captures naming and shell lessons (`docs/ui-tab-audit.md:90-124`) but does not define a required audit template, minimum evidence set, or file-anchor contract for future tab audits.
2. **The design guide is strong on naming and section ownership, but weaker on control explainability.**
   - It says titles should communicate thing/count/search/mode (`docs/ui-design-guide.md:18-24`) but does not explicitly say what to do when a tab has an important hidden default sort or a search target that is not visibly rendered.
3. **The Status spec is implementation-grade for layout, but not yet explicit about list-order/search explainability.**
   - The spec nails shell, section ownership, naming, and testing (`docs/status-tab-redesign-spec.md:24-292`), but it does not yet state how the provider list should expose or intentionally hide severity-first ordering, nor how search-on-summary should behave when `summary` is not displayed.
4. **The latest Status audit exposed a reusable class of drift that the shared docs should own.**
   - `docs/status-tab-focused-audit.md` shows the recurring failure mode: a UI can be structurally close to correct while still hiding semantics in container titles, compressed metadata rows, or invisible search targets.

## Required audit contract for future tab audits
Every tab audit should include these sections in this order:
1. Scope + inputs
2. Evidence anchors
3. What the tab does especially well
4. Keybinds and interaction model
5. Sorts / filters / toggles / modes
6. Panel layout + naming
7. Spacing + rhythm
8. Data structure and presentation
9. Scan path
10. Repeat-use stability
11. Highest-confidence keeps
12. Highest-confidence rewrite targets

### Required evidence rules
- each claim must anchor to code or docs
- keybinds must cite the event handler
- title/sort/filter claims must cite the renderer/state owner
- data-shape claims must cite the underlying app/status/agent/benchmark structures
- repeat-use stability claims must identify which labels/positions stay fixed versus drift

## Recommended artifact rewrites
### 1. Rewrite `docs/ui-tab-audit.md` into a true cross-tab reference artifact
**Current role:** directional shell audit (`docs/ui-tab-audit.md:6-124`)

**Recommended rewrite:**
- keep the current cross-tab shell observations
- add a formal "required audit dimensions" section using the contract above
- add a short cross-tab title-surfacing matrix:
  - Models: count + search/filter/sort visible (`src/tui/ui.rs:949-981`)
  - Agents: count + search/filter/sort visible (`src/tui/ui.rs:1170-1203`)
  - Benchmarks: count + search/filter/sort visible (`src/tui/ui.rs:2302-2326`)
  - Status: count + search visible, severity-first sort hidden (`src/tui/ui.rs:293-306`, `src/tui/status_app.rs:105-113`)
- add a "hidden defaults must be intentionally justified" rule

**Why:**
This turns the document from a one-time narrative into the canonical audit rubric for all tabs.

### 2. Extend `docs/ui-design-guide.md` with control-surface explainability rules
**Current role:** naming, shell, and field-placement guidance (`docs/ui-design-guide.md:18-167`)

**Recommended additions:**
- a rule that important default orderings must either be surfaced in the title/footer or explicitly justified as invisible-by-design
- a rule that search should prefer visible or easily relocatable content; if invisible fields are searchable, docs/specs must say why
- a rule that mode toggles should mutate the section they affect, not an abstract parent container
- a reusable detail-surface rule: if sections are already domain-distinct, do not place them under a generic parent title unless the parent title is itself stable and domain-meaningful (`Status`, `Details`)

**Why:**
The current guide prevents the most obvious naming mistakes, but it does not yet prevent hidden-sort confusion or search-on-invisible-content drift.

### 3. Tighten `docs/status-tab-redesign-spec.md` around list behavior and metadata surfacing
**Current role:** implementation contract for Status (`docs/status-tab-redesign-spec.md:24-292`)

**Recommended additions:**
- explicitly document the intended provider-list ordering policy, whether visible or intentionally implicit
- state whether provider-list titles should surface a sort marker or whether severity-first ordering is part of the product contract
- add a search rule: either stop indexing `summary` or make summary-derived wording visibly present in Overview/Notes so search hits are explainable
- add a micro-rule that compact/expanded state may suffix `Services` but must not rename the outer detail surface
- add an explicit ban on compressing source + time + official-page hint into one unlabeled metadata row

**Why:**
The current spec solves section naming and ownership, but the latest audit shows the remaining ambiguity is now in metadata surfacing and invisible interaction rules.

## Suggested rewrite sequence
1. Update `docs/ui-tab-audit.md` first so every new audit lands on the same rubric.
2. Update `docs/ui-design-guide.md` second so the cross-tab rules own control explainability, title truthfulness, and section ownership.
3. Update `docs/status-tab-redesign-spec.md` third so Status inherits the revised shared rules instead of restating them differently.
4. Only after those rewrites, continue Status UI implementation work.

## Recommended acceptance checks for the rewritten docs
The doc stack is ready when:
- future tab audits can follow one stable section order without inventing their own structure
- the guide explains what to do with hidden sort defaults and invisible search targets
- the Status spec no longer leaves container-title ownership or metadata-row labeling open to interpretation
- cross-tab examples are anchored to current code, not just prose summaries

## Immediate deliverables from this task
- keep `docs/status-tab-focused-audit.md` as the freshest evidence artifact for Status-specific drift
- use this rewrite plan as the handoff artifact for the leader's design-doc/spec refresh pass