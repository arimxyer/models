# Website — Claude Code Instructions

## Overview

Astro 6 landing/marketing page for the models CLI/TUI. Lives in `website/`, completely separate from the Rust binary — not packaged into the application.

## Stack

- **Framework:** Astro 6 with TypeScript strict
- **Styling:** Tailwind CSS 4
- **UI Components:** Bearnie (Astro-native, zero-JS-runtime component library)
- **Package manager:** bun (not npm/pnpm)
- **Formatting:** Prettier with astro + tailwindcss plugins
- **Deployment:** GitHub Pages at `https://arimxyer.github.io/models`

## Build & Test

First-time setup:

```bash
cd website && bun install
```

```bash
mise run dev          # Start dev server
mise run check        # Astro diagnostics
mise run typecheck    # Astro check + tsc --noEmit
mise run fmt          # Format with Prettier
mise run fmt-check    # Check formatting
mise run build        # Astro check + production build
mise run preview      # Preview production build
```

Always run before committing:

```bash
mise run fmt && mise run typecheck && mise run build
```

## Architecture

### Components

```
src/
  data/site.ts               -- build-time data sourcing (Cargo.toml, API, data files)
  layouts/Layout.astro       -- base HTML shell, fonts, meta tags, Toaster
  components/
    Header.astro             -- sticky top nav
    Hero.astro               -- hero title + tagline + copy-to-clipboard install command
    Stats.astro              -- 3 stat cards (models/benchmarks/providers)
    Screenshot.astro         -- TUI screenshot with terminal chrome + Astro Image
    Features.astro           -- vertical tabs with autoplay videos, auto-cycle, progress bars
    Commands.astro           -- CLI command cards
    Install.astro            -- install method grid with copy-to-clipboard + global copy script
    Footer.astro             -- footer with dynamic version + copyright year
    bearnie/                 -- Bearnie UI components (tabs, toast, tooltip, button)
  pages/index.astro          -- composes all components
  styles/global.css          -- Tailwind + CSS custom properties + utilities
```

### Design Direction

R2 "Data Dashboard" — sci-fi neon-on-dark aesthetic. See `DESIGN.md` for the full design system (atmosphere, color palette, typography, component stylings, layout principles). Design explorations archived in `.stitch/designs/`.

Key constraints: no rounded corners on containers, no box-shadows, no emoji, asymmetric layouts, `text-slate-400` minimum for contrast. See `.claude/rules/website-design.md` for implementation rules.

### Data Flow

Static site — all data sourced at build time via `src/data/site.ts`:

- Version, repo URL, crate name: parsed from `../Cargo.toml`
- Benchmark/agent counts: parsed from `../data/*.json`
- Status provider count: regex-counted from `../src/status/registry.rs`
- Model/provider counts: fetched from `models.dev/api.json`

Components import from `@/data/site` — never hardcode stats, versions, or URLs. Videos from `public/assets/wiki/`, hero screenshot from `src/assets/`.

## Gotchas

- Static asset paths must use `import.meta.env.BASE_URL` prefix due to `/models` base path on GitHub Pages
- No Astro LSP server available — use `astro check` / `mise run typecheck` for `.astro` file diagnostics
- `astro check` is wired into the build script — runs automatically before every `bun run build`
- Stitch-generated HTML hallucinated CLI commands and descriptions — always verify factual content against the real tool
- `src/data/site.ts` uses `process.cwd()` (not `import.meta.url`) to resolve repo root — Astro's build runs from a different directory than the source, so `import.meta.url`-based paths break during SSG
- The copy-to-clipboard `<script>` in `Install.astro` is a global handler — it selects ALL `[data-copy-btn]` elements on the page, including the Hero button. Don't duplicate the script in other components
- `models.dev/api.json` returns models as an object (keyed by model ID), not an array — use `Object.keys().length` not `.length` for counting
- `data/agents.json` `agents` field is also an object, not an array
- Bearnie's base `TabsTrigger.astro` carries `rounded-md` and `shadow-sm` defaults — override with `rounded-none` and avoid shadow classes in the `tabTriggerClass` const in Features.astro
- The `.claude/` directory is in the root `.gitignore` — rules files must be force-added with `git add -f`
