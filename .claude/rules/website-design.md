---
description: Website design conventions — colors, layout, data sourcing, accessibility, and component patterns for the Astro landing page
globs:
  - website/**
---

# Website Design Conventions

See `website/DESIGN.md` for the full design system (atmosphere, color palette, typography, component stylings, layout principles). This file covers implementation rules only.

---

## 1. Build & Verify

Always run from the `website/` directory before committing:
```bash
cd website && mise run fmt && mise run typecheck && mise run build
```

Package manager is **bun** (not npm/pnpm). UI components from **Bearnie** (Astro-native, zero-JS-runtime).

## 2. Hard Design Constraints

- Zero `border-radius` on structural containers — exception: terminal chrome dots (`rounded-full`) and tooltips (`rounded`)
- No `box-shadow` — depth via tonal layering and `data-border`. Note: Bearnie base TabsTrigger carries latent `shadow-sm` — always override
- No gradient fills on text — solid white or neon accent
- No emoji — monospace labels and terminal notation. Exception: hero prompt `❯` (U+276F)
- Asymmetric column splits for content sections — exception: Commands and Footer use equal grids for dense data
- `prefers-reduced-motion` — CSS animations wrapped in `@media (prefers-reduced-motion: no-preference)` or `motion-safe:` prefix. Known gap: video auto-cycle is not yet gated

## 3. Colors

Use CSS custom properties — never raw hex literals in components:

| Variable | Role |
|----------|------|
| `var(--neon-cyan)` | Primary accent — focus, active, links, data |
| `var(--neon-magenta)` | Command/CLI accent |
| `var(--neon-green)` | Install/CTA accent |
| `var(--bg-slate)` | Canvas background |

Minimum readable text: `text-slate-400`. Never `text-slate-500` or darker on `--bg-slate`.

## 4. Data Sourcing

All dynamic data comes from `src/data/site.ts` (build-time). Never hardcode:

| Data | Import |
|------|--------|
| Version | `VERSION` |
| Stats | `DISPLAY`, `MODEL_COUNT`, `PROVIDER_COUNT`, `BENCHMARK_COUNT`, `AGENT_COUNT`, `STATUS_PROVIDER_COUNT` |
| URLs | `REPO_URL`, `WIKI_URL`, `RELEASES_URL`, `LICENSE_URL`, `CRATES_URL` |
| Meta | `SITE.title`, `SITE.description` |

## 5. Accessibility

- Interactive elements: `<button>` or `<a>`, never `<div>` with click handlers
- Focus states: every interactive element needs `focus-visible:` styles
- Decorative elements: `aria-hidden="true"` on terminal chrome, badges, visualizations
- Nav landmarks: `aria-label` on `<nav>` elements
- Skip link: `<a href="#main-content">` in Layout.astro

## 6. Asset Paths

- `public/` assets: prefix with `import.meta.env.BASE_URL` (GitHub Pages `/models` base path)
- `src/assets/` images: use ESM imports with Astro `<Image>` component
- Videos: `public/assets/wiki/` with BASE_URL prefix in `<source>` tags
