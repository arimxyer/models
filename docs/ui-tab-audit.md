# UI Tab Audit

## Goal
Identify the layout patterns the tabs already share, where the Status tab drifts from those patterns, and which tab-specific ideas are worth standardizing.

## Shared shell patterns already present
Across Models, Agents, and Benchmarks, the app already has a recognizable product shell:
- top tab bar for global navigation
- bottom footer for shortcuts and help hints
- bordered panels with cyan focus and dark-gray unfocused borders
- titles that usually communicate count, search, sort, or current context
- left-side navigation rails and right-side detail areas
- content-specific detail panels instead of generic dashboard chrome

These shared patterns are part of the app's identity and should be preserved.

## Per-tab audit

### Models
**Current structure**
- 3-column fixed browse layout: Providers | Models | Details
- provider rail is compact navigation
- model list owns filtering/sorting metadata in the title
- detail panel is stable and information-dense

**What it does well**
- strongest layout stability in the app
- navigation and detail responsibilities are clear
- panel titles are concrete and domain-based
- scan path is predictable for repeat use

**What to learn from it**
- preserve stable left-to-right role assignment
- keep navigation rails lightweight
- avoid renaming or repurposing panels without a strong reason

### Agents
**Current structure**
- 2-column layout: Agents | Details
- left rail is content-sized, not overly wide
- detail panel is a long-form read surface with one stable title

**What it does well**
- detail panel has a single identity: Details
- metadata is grouped near the top, then history/content flows below
- the list is obviously navigation-first

**What to learn from it**
- one strong detail surface is often enough
- avoid extra nested boxes when one detail panel can hold the story
- keep operational metadata near the top in predictable slots

### Benchmarks
**Current structure**
- browse mode: Creators | Benchmarks | Details
- compare mode: compact list + comparison panel
- explicit sub-tab bar for alternate views in compare mode

**What it does well**
- modes are explicit and legible
- panel titles are domain nouns: Creators, Models, Details, Scatter, Radar, H2H
- compare mode uses a clear alternate shell instead of cramming all views together

**What to learn from it**
- mode changes should be explicit
- alternative views should be named after the thing the user is looking at
- tabs/subtabs work better than overloaded mixed-content boxes

### Status
**Current structure**
- left provider rail + right stacked detail shell
- custom right-side blocks: Status page / Narrative / Note
- single body panel currently mixes incidents, services, maintenance, and caveats

**What it does well now**
- provider rail is much closer to navigation-first than before
- hero verdict is more obvious than earlier versions
- incidents and services are trending toward a status-page model

**Where it drifts**
- the right panel naming is inconsistent with the rest of the app
- "Narrative" is not a user concept; it is an implementation bucket
- hero metadata semantics have been moving around while being tuned
- stacked content exists, but the field locations and labels are not yet stable enough
- service-detail absence and timestamp semantics still need stronger fixed labeling

**Why it feels off**
The Status tab is still acting like a special-case experiment rather than a peer tab in the same product family. Models, Agents, and Benchmarks each have clearer panel identity and more stable field placement.

## Cross-tab strengths worth standardizing
1. **Concrete panel names**
   - Good: Providers, Models, Agents, Details, Creators, Scatter
   - Weak: Narrative, Note, generic meta wording

2. **Stable panel responsibility**
   - left side navigates
   - right side explains the selected thing
   - alternate views are explicit, not mixed into one body

3. **Predictable metadata slots**
   - counts/search/sort in titles
   - key object metadata near the top of the detail surface
   - no shifting semantics hidden behind one recurring word

4. **Minimal chrome inside the main content**
   - app footer owns global help/controls
   - panels should not repeat control hints in content bodies

## Status-specific opportunities
The Status tab should evolve toward:
- a stable hero header with fixed labeled fields
- explicit sections named after user concepts
- no abstract buckets like Narrative
- a predictable scan path for first-time and repeat users

Suggested section naming:
- Overview
- Current incidents
- Services
- Maintenance
- Notes

## Recommended next use of this audit
Use this audit with `docs/ui-design-guide.md` before making more Status-tab layout changes.
