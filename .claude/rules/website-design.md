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
| `bg-background` | Canvas background (shadcn token, resolves to `--background: #0f172a`) |

Bearnie components use shadcn/ui semantic tokens (`bg-popover`, `text-foreground`, `bg-muted`, etc.) defined via `@theme inline` in `global.css`. Site components use the `--neon-*` variables directly. Both systems coexist — shadcn tokens for Bearnie defaults, neon vars for explicit accent styling.

`--bg-slate` is deprecated — use `bg-background` instead.

Minimum readable text: `text-slate-400`. Never `text-slate-500` or darker on the canvas background.

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

## 7. Animation

Four animation approaches coexist, each for its correct domain:
- **PixiJS v8** — procedural canvas/WebGL (galaxy graphic). Use `ParticleContainer` for particle systems, `preference: "webgl"` in `app.init()`, `cacheAsTexture(true)` on static containers with BlurFilters
- **cobe** — WebGL globe. Driven by rAF loop with `update({phi})`. Single `markerColor` for all markers (no per-marker coloring)
- **anime.js v4** — DOM/SVG animations (scatter dots, robot idle, stat counters, hero tagline rotator, scroll-triggered entry)
- **CSS `@keyframes`** — stepped/decorative animations (cursor blink, typewriter). Use `step-end` for discrete state changes

### Rules
- **Visibility gating**: WebGL/canvas/looping animation components must use `IntersectionObserver` to pause when off-screen and resume when visible. See `GalaxyGraphic.astro`, `GlobeGraphic.astro`, `RobotGraphic.astro` for the pattern
- **Activation**: Stat graphics use CustomEvent deferred activation — `Stats.astro` dispatches `{name}:activate`, each graphic listens with `document.addEventListener` and initializes lazily
- **Transform conflict**: anime.js v4 overwrites the entire CSS `transform` property. Do not use CSS `transform` for positioning if anime.js will animate `scale`/`rotate` on the same element. Use `left`/`right`/`top`/`bottom` instead
- **`prefers-reduced-motion`**: CSS animations wrapped in `@media (prefers-reduced-motion: no-preference)` or `motion-safe:` prefix. JS loops gated by `window.matchMedia`. Known gap: video auto-cycle is not yet gated

### SVG-specific
- When animating Lottie-exported SVGs with anime.js, wrap each `<g>` layer in an outer `<g id="part-wrap">` with no transform. Animate the wrapper — the inner `<g>` retains its positional `transform="matrix(...)"` untouched
- anime.js v4 alternate loops: use single target value + `alternate: true`, not keyframe arrays
- CSS `translateX`/`translateY` on SVG `<g>` uses SVG coordinate units (viewBox scale), not CSS pixels

### Bearnie Card tokens
- `bg-card` resolves to `rgba(15, 23, 42, 0.8)` and `text-card-foreground` to `#e2e8f0` via `global.css` `:root` and `@theme inline` blocks. Ensure these are defined when using Bearnie Card
