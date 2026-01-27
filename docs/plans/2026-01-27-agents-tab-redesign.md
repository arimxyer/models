# Agents Tab Redesign

**Date:** 2026-01-27
**Status:** Design Complete
**Branch:** `agents-tab`

## Overview

This document captures the redesign of the Agents tab based on user testing feedback from Phase 2 implementation. The redesign addresses UX issues including layout, performance, filtering, and data architecture.

## Problems Identified

1. **Layout cramped** - Three-area layout (categories sidebar + agent list + bottom detail) wastes space
2. **Slow startup** - Sequential synchronous GitHub API calls block UI rendering
3. **Picker broken** - Tracked field stored but never used for filtering; config save errors silently ignored
4. **No filter indication** - Unlike Models tab, no visual cue when filters are active
5. **Footer static** - Shows Models-specific keys even on Agents tab
6. **Table redundant** - Status column duplicates info derivable from version comparison
7. **No version history** - Can only see latest version, no changelog navigation

## Design Decisions

### 1. Two-Panel Layout

**Before:**
```
┌─────────────┬────────────────────────────────┐
│ Categories  │ Agent List                     │
│ (25%)       │ (75%)                          │
├─────────────┴────────────────────────────────┤
│ Detail Panel (bottom, fixed height)          │
└──────────────────────────────────────────────┘
```

**After:**
```
┌──────────────────┬───────────────────────────┐
│ Agent List       │ Details                   │
│ (35%)            │ (65%)                     │
│                  │                           │
│ [filter toggles] │ Agent Name    v1.0.25    │
│ [sort dropdown]  │ ★ 45.2k  Updated 2d ago  │
│                  │                           │
│ > Claude Code    │ Changelog:                │
│   Aider          │ ─────────────────────     │
│   Cursor         │ • Fixed terminal bug      │
│   Goose          │ • Added new feature       │
│   ...            │ • Improved performance    │
│                  │                           │
│                  │ [← v1.0.24] [v1.0.25 →]   │
│                  │                           │
│ (12 agents)      │ [Tab to focus, ↑↓ scroll] │
└──────────────────┴───────────────────────────┘
```

**Rationale:**
- Categories become filter toggles in the list header, not a separate panel
- Details panel gets more real estate for changelog and version history
- Simpler mental model: select agent → see details
- Tab switches focus between panels

### 2. Filter System

**Categories as Filters:**
- All / CLI / IDE / MCP (radio selection, like provider sidebar on Models)
- Additional toggles: Tracked Only, Open Source Only
- Active filters shown in block title: `Agents (12) [cli, tracked]`

**Tracked Filtering (Fixed):**
- `tracked` field in config actually filters the list
- When "Tracked Only" is on, untracked agents are hidden
- Picker modal to manage tracked status (existing, needs bug fix)

### 3. Agent List Columns

**Before:** Name (25) | Installed (10) | Latest (10) | Stars (8) | Status (8)

**After:** Name | Type | Version | Updated

| Column | Width | Content |
|--------|-------|---------|
| Name | 30% | Agent name, truncated if needed |
| Type | 10% | CLI / IDE / MCP icon or short text |
| Version | 30% | Smart display (see below) |
| Updated | 30% | Relative time since last release |

**Smart Version Column:**
- Not installed: `-`
- Installed, up to date: `v1.0.25 ✓`
- Installed, update available: `v1.0.24 → v1.0.25`
- Installed, version unknown: `installed`

### 4. Details Panel

**Layout:**
```
┌─────────────────────────────────────────────┐
│ Claude Code                        v1.0.25  │
│ anthropics/claude-code  ★ 45.2k             │
│─────────────────────────────────────────────│
│ Anthropic's official CLI for Claude.        │
│                                             │
│ Installed: v1.0.24  (update available)      │
│ Latest:    v1.0.25  (2 days ago)            │
│                                             │
│ Changelog (v1.0.25):                        │
│ ───────────────────                         │
│ • Fixed terminal rendering bug              │
│ • Added support for new model               │
│ • Improved context handling                 │
│                                             │
│         [← Older]  v1.0.25  [Newer →]       │
└─────────────────────────────────────────────┘
```

**Version History Navigation:**
- Left/Right arrows (or h/l) to page through releases
- Shows changelog for selected version
- Fetched from GitHub releases API

**Scrolling:**
- When details panel is focused, ↑↓ scrolls content
- Long changelogs can be scrolled through

### 5. Footer (Tab-Specific)

**Models Tab (unchanged):**
```
q quit  ↑/↓ nav  Tab switch  / search  s sort  c copy    ? help
```

**Agents Tab:**
```
q quit  ↑/↓ nav  Tab switch  / search  s sort  a track  u update  ? help
```

| Key | Action |
|-----|--------|
| `a` | Open picker to manage tracked agents |
| `u` | Copy update command for selected agent |
| `s` | Cycle sort order (name, updated, stars) |
| `r` | Refresh GitHub data for selected agent |

### 6. Async GitHub Fetching

**Current (slow):**
```
for agent in agents:
    data = gh_api_sync(agent.repo)  # blocks
render_ui()
```

**New (fast):**
```
render_ui()  # immediate with cached/placeholder data
spawn_async:
    for agent in agents (parallel):
        data = reqwest_async(agent.repo)
        update_ui(agent, data)  # progressive
```

**Implementation:**
- Use `reqwest` for HTTP instead of `gh` subprocess
- Use `tokio` runtime for async
- Cache responses with 1-hour TTL (keep existing cache logic)
- Show loading indicator while fetching
- Fetch in background, update UI as results arrive

### 7. Data Sourcing (Hybrid)

**Curated Core:**
- `agents.yaml` bundled with binary
- Maintained in main repo
- Contains well-known agents with accurate metadata

**User Custom Agents:**
- `~/.config/models/agents.toml` for user additions
- Format:
  ```toml
  [[agents]]
  name = "My Internal Agent"
  repo = "myorg/internal-agent"
  type = "cli"
  binary = "myagent"
  ```
- Merged with curated list at startup
- User agents can override curated ones (by name)

**Benefits:**
- Works offline with bundled data
- Users can add private/internal agents
- No external sync dependency
- Future: could add community registry sync as optional feature

### 8. Sort Options

| Sort | Description |
|------|-------------|
| Name (A-Z) | Alphabetical by agent name |
| Updated | Most recently released first |
| Stars | Most GitHub stars first |
| Status | Update available first, then installed, then not installed |

Default: Updated (show most active agents first)

## Implementation Phases

### Phase 3A: Core Fixes
1. Fix tracked filtering (wire up to `update_filtered`)
2. Fix config save error handling
3. Tab-specific footer keybindings
4. Filter indication in block title

### Phase 3B: Layout Redesign
1. Remove categories sidebar
2. Add filter toggles to agent list header
3. Expand details panel to right side
4. Implement new column layout

### Phase 3C: Async & Performance
1. Add reqwest + tokio dependencies
2. Refactor GitHubClient to async
3. Implement progressive UI updates
4. Add loading indicators

### Phase 3D: Version History
1. Fetch release history from GitHub API
2. Add version navigation (←/→) in details
3. Show changelog for selected version
4. Scroll support for long changelogs

### Phase 3E: User Custom Agents
1. Define `agents.toml` schema
2. Load and merge with curated list
3. Add "Add Custom Agent" flow (optional)
4. Documentation

## Resolved Questions

1. **GitHub rate limits** - Start unauthenticated (60 req/hour). When rate limited, prompt user to provide a GitHub token. Store token in config for future use.

2. **Changelog parsing** - Light parsing: strip excessive whitespace, render basic markdown (bold, bullets, links, code). Don't try to restructure content.

3. **MCP servers** - Defer to future phase. Focus on CLI and IDE agents for now. Type filter will be CLI / IDE only (no MCP option yet).

## Success Criteria

- [ ] Startup renders UI in <500ms (GitHub data can load progressively)
- [ ] Tracked filter actually works
- [ ] Version history shows at least 5 past releases
- [ ] User can add custom agents via config file
- [ ] Filter state is visually indicated
- [ ] Footer shows relevant keybindings for current tab
