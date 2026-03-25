# Design System: models — Landing Page
**Project ID:** 194825300356652245

## 1. Visual Theme & Atmosphere

**Creative North Star: "The Mission Control Dashboard"**

The aesthetic is dense, utilitarian, and precise — a command center for browsing AI infrastructure. It rejects the softness of modern SaaS marketing. Instead of friendly illustrations and gradient blobs, the page presents raw data density: neon readouts on obsidian panels, CRT-style scan lines, and monospace classification labels.

The atmosphere is **cold, technical, and deliberately mechanical** — like a systems operator's terminal rendered as a website. The "sci-fi" is understated: no starfields, no hologram effects, just the language of status monitors, data grids, and terminal emulators.

**Density over elegance.** Every element earns its pixel. Decorative elements must represent data or reinforce the terminal metaphor — never exist solely for visual fill. Whitespace is intentional and asymmetric, creating rhythmic tension rather than centered comfort.

**Anti-Patterns (hard rules):**
- No gradient blobs, mesh backgrounds, or glassmorphism
- No rounded corners — all `border-radius: 0`. Sharp, squared-off edges exclusively
- No emoji — use monospace labels and terminal notation
- No symmetrical card grids — asymmetric column splits only
- No generic stock illustrations — show the actual product
- No gradient-filled text — text is solid white or neon accent
- No box-shadow depth — depth is achieved through tonal layering and borders
- No decorative elements without data meaning

## 2. Color Palette & Roles

### Primary Surfaces

| Descriptive Name | Hex | Functional Role |
|-----------------|-----|-----------------|
| **Deep Naval Slate** | `#0f172a` (`--bg-slate`) | Canvas background — the base "void" everything sits on. Applied to `<body>` with a subtle 40px cyan grid overlay at 5% opacity creating the graph-paper effect. |
| **Smoky Panel** | `bg-slate-900/50` | Container surfaces — stat cards, feature panels, command blocks. Semi-transparent to let the grid bleed through subtly. |
| **Terminal Black** | `bg-black` or `bg-black/40` | Deep-recessed containers — video panels, code blocks, terminal chrome. Creates the "sunken monitor" effect. |

### Neon Accent Triad

Three neon accents, each with a strict functional assignment. They are never interchangeable.

| Descriptive Name | Hex | Functional Role |
|-----------------|-----|-----------------|
| **Electric Cyan** | `#22d3ee` (`--neon-cyan`) | Primary system accent — focus states, active tab indicators, data highlights, links, section headers, glow effects. The "power-on" color. Used for `.data-border` (20% opacity), `.data-header` backgrounds (10% opacity), and the background grid pattern (5% opacity). |
| **Hot Magenta** | `#f472b6` (`--neon-magenta`) | Command/CLI accent — CLI command labels (`COMMAND_FILTER`), top-border accents on command cards, text selection highlight, the "Browse the AI ecosystem" tagline. Signals "input" and "action." |
| **Terminal Green** | `#4ade80` (`--neon-green`) | Install/CTA accent — install buttons, the hero install prompt (`>`), the "System Access" block, positive/go states. Signals "execute" and "ready." |

### Text Hierarchy

| Descriptive Name | Tailwind Class | Functional Role |
|-----------------|---------------|-----------------|
| **Full Bright** | `text-white` | Primary headings, model names, emphasized data values |
| **Cool Silver** | `text-slate-300` | Body descriptions, tagline text |
| **Muted Steel** | `text-slate-400` | Secondary labels, metadata, inactive states. Minimum contrast level for readable text on dark backgrounds (WCAG AA compliant). |
| Accent colors | `text-[var(--neon-*)]` | Functional highlights per accent role above |

**Contrast rule:** Never use `text-slate-500` or darker for text that conveys meaning on the `--bg-slate` background. `text-slate-400` is the floor.

### Neon Glow

Three glow utilities (`.glow-cyan`, `.glow-magenta`, `.glow-green`) apply `text-shadow: 0 0 10px` at 50% accent opacity. Used exclusively on large display numbers in stat cards — nowhere else.

## 3. Typography Rules

### Font Families

| Font | Character | Role |
|------|-----------|------|
| **Outfit** (300, 600, 900) | Geometric, technical sans-serif with extreme weight range | Display headings (900 black), body text (300 light), section headers (600 semibold). The "voice" of the site. |
| **Inter** (400, 700, 900) | Neutral UI workhorse | Loaded as fallback; Outfit is primary for all visible text. |
| **JetBrains Mono** (400, 700) | Developer-grade monospace | CLI commands, install strings, data classification labels (`Model_Density`, `COMMAND_FILTER`), terminal chrome labels, footer tech specs. The "data" voice. |

### Type Scale

| Name | Size / Style | Usage |
|------|-------------|-------|
| **Display** | `clamp(60px, 15vw, 180px)` / Black / tracking-tighter / `text-balance` | Hero title ("models.") only. Fluid scaling, never fixed. |
| **Section Heading** | `text-4xl` / Black / tracking-tighter / uppercase | Section labels ("Operations", "System Access") |
| **Feature Body** | `text-2xl` or `text-xl` / Light / leading-tight | Feature descriptions in tab content and hero tagline |
| **UI Text** | `text-sm` / `text-xs` | Navigation links, button labels, list items |
| **Data Label** | `font-mono text-[10px] tracking-widest uppercase` | Terminal-style metadata labels. Used 20+ times across components. Always paired with `text-slate-400`. |
| **Code** | `font-mono text-sm` / `font-mono text-xs` | CLI commands, install strings |

### Typographic Details

- `font-variant-numeric: tabular-nums` on stat display numbers for consistent digit width
- `text-balance` on the hero heading to prevent orphaned words on reflow
- Stat numbers use the glow utility matching their accent color

## 4. Component Stylings

### Terminal Chrome
The signature "monitor frame" wrapping video panels and the hero screenshot:
- **Header bar** (`data-header`): 3 circles (`h-2 w-2 rounded-full bg-slate-700`) left-aligned + monospace uppercase label right-aligned (e.g., `MODE: TUI // TAB: MODELS`). Background `rgba(34, 211, 238, 0.1)` with bottom border at 30% opacity.
- **Container**: `data-border` class — `1px solid rgba(34, 211, 238, 0.2)`. Black background. Content below the header bar.
- All terminal chrome elements are `aria-hidden="true"`.

### Stat Cards
Three cards in a vertical stack, each with:
- `data-border` + `bg-slate-900/50` background
- Data label in JetBrains Mono at top
- Large display number with accent glow at bottom
- Accent-colored data visualization alongside the number (currently placeholder divs — to be replaced with meaningful visualizations)
- Third card has a `border-l-4` in Terminal Green for differentiation

### Feature Tabs (Bearnie Vertical Tabs)
- Tab triggers: `data-border`, left border accent (4px cyan when active, transparent when inactive), `bg-[var(--neon-cyan)]/10` active background
- Tab content: Terminal chrome header + autoplay video with gradient overlay (`from-black/90 via-black/30 to-transparent`) + text overlaid at bottom-left
- 2px cyan progress bar at bottom of each trigger, animated via `requestAnimationFrame` synced to video playback
- Auto-cycles through tabs on video end; user click stops cycling

### Copy-to-Clipboard Buttons
Used on install cards and hero command:
- `<button>` with `data-copy-btn` + `data-copy-text` attributes
- Clipboard SVG icon transitions to checkmark on copy (1.8s CSS opacity transition)
- Bearnie toast notification ("Copied!") on success
- CSS-only "Click to copy" tooltip via `group-hover:opacity-100`

### Command Cards
Three cards in a horizontal grid:
- `border-t-4` in Hot Magenta accent
- Monospace label (e.g., `COMMAND_FILTER`) in magenta
- CLI command in white monospace
- Description in data-label style below

### Navigation
- **Header**: Sticky, backdrop-blur. Cyan pulse dot (status indicator) + "System: Models_OS" label + bracketed nav links (`[ documentation ]`) in monospace uppercase
- **Footer**: Four-column grid with tech specs, nav links, environment info, copyright

## 5. Layout Principles

### Asymmetric Grid Philosophy
Layouts use intentionally unequal column splits to create visual tension:

| Section | Grid | Split Ratio |
|---------|------|-------------|
| Hero + Stats | `lg:grid-cols-12` | 8:4 (hero dominates) |
| Feature Tabs | `md:grid-cols-4` | 1:3 (sidebar:content) |
| Install Grid | `lg:grid-cols-4` | 1:3 (label:cards) |
| Commands | `md:grid-cols-3` | Equal (exception: dense data grid) |
| Footer | `md:grid-cols-4` | Equal (exception: dense data) |

### Spacing Rhythm
- Between sections: `space-y-8` (32px)
- Between cards within a section: `gap-4` (16px)
- Card internal padding: `p-6` (small cards) or `p-8` (large panels)

### Depth Without Shadows
Depth is achieved exclusively through tonal layering:
1. Canvas (`--bg-slate`) — the deepest layer
2. Panels (`bg-slate-900/50`) — mid-layer containers
3. Recessed (`bg-black`) — terminal screens, video panels
4. Accent highlights — `data-header` background at 10% cyan

No `box-shadow` anywhere. The 1px `data-border` is the only boundary mechanism.

## 6. Motion & Animation

All animation is gated behind `prefers-reduced-motion: no-preference`:
- **Scanline overlay** (`.active-scanline::after`): Repeating 4px horizontal gradient creating CRT scan line effect. Purely cosmetic, `pointer-events: none`.
- **Pulse dot** (`motion-safe:animate-pulse`): Header status indicator. Signals "system active."
- **Tab progress bar**: `requestAnimationFrame`-driven, synced to video `currentTime/duration`.
- **Icon transitions**: Clipboard-to-checkmark uses CSS opacity transitions (200ms).

No spring physics, no parallax, no scroll-triggered animations. Motion is functional (indicating state) or atmospheric (scanlines, pulse), never decorative.
