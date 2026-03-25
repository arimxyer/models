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
  layouts/Layout.astro       -- base HTML shell, fonts, meta tags
  components/
    Header.astro             -- fixed top nav
    Hero.astro               -- hero title + tagline + install command
    Stats.astro              -- 3 stat cards (models/benchmarks/providers)
    Screenshot.astro         -- TUI screenshot with terminal chrome
    Features.astro           -- feature tabs + detail panel
    Commands.astro           -- CLI command cards
    Install.astro            -- install method grid
    Footer.astro             -- footer
  pages/index.astro          -- composes all components
  styles/global.css          -- Tailwind + custom CSS vars + utilities
```

### Design Direction

R2 "Data Dashboard" — sci-fi neon-on-dark aesthetic. Design explorations in `.stitch/designs/`.

- Background: deep slate (#0f172a) with subtle cyan grid
- Triple neon accent: cyan (#22d3ee), magenta (#f472b6), green (#4ade80)
- Fonts: Outfit (display), Inter (body), JetBrains Mono (code)
- Scanline effects, data-border styling, glow utilities

### Data Flow

Static site — no runtime data fetching. Screenshots from `public/` directory. All content is hardcoded in components.

## Gotchas

- Static asset paths must use `import.meta.env.BASE_URL` prefix due to `/models` base path on GitHub Pages
- No Astro LSP server available — use `astro check` / `mise run typecheck` for `.astro` file diagnostics
- `astro check` is wired into the build script — runs automatically before every `bun run build`
- Stitch-generated HTML hallucinated CLI commands and descriptions — always verify factual content against the real tool
