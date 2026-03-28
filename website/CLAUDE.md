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
    Stats.astro              -- 4 stat cards composed from modular graphics, scroll-triggered
    stats/                   -- stat card graphic components
      StatCard.astro         -- Bearnie Card wrapper with accent theming and slot
      GalaxyGraphic.astro    -- PixiJS procedural galaxy with black hole sphere
      ScatterGraphic.astro   -- SVG scatter plot with anime.js animations
      GlobeGraphic.astro     -- cobe WebGL globe with outage flash easter egg
      RobotGraphic.astro     -- animated robot SVG with click-to-swap variants
    Screenshot.astro         -- TUI screenshot with terminal chrome + Astro Image
    Features.astro           -- vertical tabs with autoplay videos, auto-cycle, progress bars
    Commands.astro           -- CLI command cards
    Install.astro            -- install method grid with copy-to-clipboard + global copy script
    Footer.astro             -- footer with dynamic version + copyright year
    bearnie/                 -- Bearnie UI components (card, tabs, toast, tooltip, button)
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
- Bearnie's base `TabsTrigger.astro` carries `rounded-md` default — override with `rounded-none` in the `tabTriggerClass` const in Features.astro
- The `.claude/` directory is in the root `.gitignore` — rules files must be force-added with `git add -f`
- Website uses shadcn/ui theme tokens via `@theme inline` in `global.css`. Bearnie components resolve classes like `bg-popover`, `text-foreground`, `bg-muted` from these tokens. When adding new Bearnie components, ensure any new tokens they reference are defined in `global.css` `:root` and `@theme inline` blocks
- Animation deps: `animejs` v4 (animation engine), `cobe` (WebGL globe), `pixi.js` v8 (procedural galaxy). `lottie-to-svg` is a devDependency for SVG export only
- Stat card graphics use CustomEvent activation: `Stats.astro` dispatches `{name}:activate` events, each graphic component listens with `document.addEventListener("{name}:activate", ...)` and initializes lazily
- WebGL/canvas/looping animation components must use IntersectionObserver to pause when off-screen and resume when visible — see `GalaxyGraphic.astro`, `GlobeGraphic.astro`, `RobotGraphic.astro` for the pattern
- PixiJS v8: use `preference: "webgl"` in `Application.init()` to skip the 38KB WebGPU chunk. Use `cacheAsTexture(true)` on static containers with BlurFilters to avoid per-frame recomposition. Subpath imports do not save bundle size in v8
- cobe globe: `markerColor` is global (single RGB array for all markers, no per-marker coloring). Call `destroy()` before recreating. `update({phi})` for rotation
- anime.js v4 overwrites the entire CSS `transform` property — do not use CSS `transform: translateX()` for positioning if anime.js will animate `scale` or `rotate` on the same element. Use CSS `left`/`right`/`top`/`bottom` positioning instead
- Lottie-to-SVG pipeline (historical, used for `stat-robot.svg`): export frame 0 via `lottie-to-svg`, wrap each `<g>` layer in an outer `<g id="part-wrap">` for safe anime.js transforms
- anime.js v4 `alternate` syntax: use a single target value (`translateY: 8`) with `alternate: true` — NOT array syntax (`[-5, 5]`) which creates discrete keyframes and choppy motion
- CSS `translateX`/`translateY` on SVG `<g>` elements uses SVG coordinate units (viewBox), not CSS pixels. In a 1080-wide viewBox rendered at 150px, ~1800 units needed to exit the frame
