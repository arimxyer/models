# v0.8.0 Demo Script

Total runtime target: ~90 seconds

---

## Setup

- Terminal size: at least 120x40 (wider is better for benchmarks columns)
- Start with `models` command
- Consider using a clean terminal with a nice theme

---

## Scene 1: Models Tab (15s)

**On screen:** App launches on Models tab, providers list on left, models on right.

**Say:** "models is a fast CLI and TUI for browsing AI models, benchmarks, and coding agents."

**Actions:**
1. Press `j`/`k` a few times to scroll through providers
2. Press `l` to switch to models panel, scroll a bit
3. Press `/`, type `claude`, show search filtering
4. Press `Enter` to lock search, scroll to show results
5. Press `4` a couple times to cycle provider categories (Origin, Cloud, Inference)

**Say:** "Browse 2000+ models across 85+ providers. Filter by category, search across providers, and copy model IDs with a single keypress."

---

## Scene 2: Benchmarks Tab (45s) — the star of the show

**Actions:**
1. Press `]` to switch to Benchmarks tab

**Say:** "New in v0.8.0 — the Benchmarks tab. 400+ model entries from Artificial Analysis with quality, speed, and pricing data."

2. Pause to show the layout: creators sidebar, benchmark list, detail panel

**Say:** "Every model creator is classified by source — open, closed, or mixed — along with their region and type."

3. Press `j`/`k` to browse creators, show the openness/region tags
4. Select a specific creator (e.g., scroll to "Anthropic" or "DeepSeek")

**Say:** "Select a creator to filter the list, or use All to see everything."

5. Press `l` to switch to the list, scroll through entries
6. Point out the detail panel updating as you navigate

### Quick Sort Demo

7. Press `1` — sort by Intelligence

**Say:** "Quick-sort keys let you instantly rank models. Press 1 for Intelligence..."

8. Press `2` — sort by Date

**Say:** "...2 for release date to see what's newest..."

9. Press `3` — sort by Speed

**Say:** "...and 3 for speed."

10. Press `3` again to flip direction (ascending)

**Say:** "Press the same key again to flip the sort direction."

### Dynamic Columns Demo

11. Press `s` a few times to cycle to a different sort group (e.g., GPQA/MMLU/HLE)

**Say:** "The list columns adapt automatically — when you sort by a knowledge benchmark, the knowledge group appears. Sort by code benchmarks, and you see LiveCode, SciCode, and Terminal."

12. Press `s` a couple more times to show columns changing

### Filters Demo

13. Press `h` to go back to creators panel
14. Press `5` to cycle region to "China"

**Say:** "Filter creators by region..."

15. Press `5` again to "Europe" (shows just Mistral)
16. Press `5` back to "All"
17. Press `6` to cycle to "Research"

**Say:** "...or by type — startups, big tech, or research labs."

18. Press `6` back to "All"

---

## Scene 3: Agents Tab (15s)

1. Press `]` to switch to Agents tab

**Say:** "The Agents tab tracks AI coding assistants with automatic version detection and GitHub integration."

2. Scroll through agents, show the status indicators (installed, update available, etc.)
3. Press `l` to show detail panel with stars, latest release, changelog

**Say:** "See which agents are installed, check for updates, and browse changelogs — all from your terminal."

---

## Scene 4: Wrap Up (10s)

**Say:** "Install with cargo install, brew, or scoop. Check it out at github.com/arimxyer/models."

Press `q` to quit.

---

## Recording Tips

- **Tool:** [vhs](https://github.com/charmbracelet/vhs) (generates GIFs from a tape file), [asciinema](https://asciinema.org) (terminal recording), or OBS for video with voiceover
- **Pacing:** Pause ~1 second between actions so viewers can follow
- **Font size:** Bump terminal font to 14-16pt for readability
- **If silent:** Add text overlays for each section header
