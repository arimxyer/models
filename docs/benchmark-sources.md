# Benchmark Sources

The benchmarks tab currently pulls model-level data from [Artificial Analysis](https://artificialanalysis.ai).

If you want to add more sources, the best fits are the ones that already publish a leaderboard or dataset with:

- a stable, machine-readable feed (JSON, CSV, API, or predictable HTML)
- model names that can be mapped to `models.dev`
- clear update cadence
- redistribution terms that allow this project to cache or republish derived data
- broad enough coverage to justify a new tab section, filter, or import job

## Good Candidates

| Source | Why it fits | Likely integration shape | Main caveat |
|--------|-------------|--------------------------|-------------|
| **LiveBench** | Strong coverage for reasoning, coding, math, instruction following, and data analysis | Import as a second benchmark feed or map its scores into new columns | Needs model-name normalization and source-specific freshness handling |
| **LMSYS Chatbot Arena / Arena Elo** | Adds human preference data instead of synthetic-only benchmark scores | New score column or alternate leaderboard view | Crowdsourced Elo is a different signal than task benchmarks, so it should stay clearly labeled |
| **Aider LLM leaderboards** | Great coding-specific signal for code editing and agent-style tasks | Coding-focused columns or a narrower supplemental feed | Coverage is smaller and more coding-centric than the current broad leaderboard |
| **Hugging Face Open LLM Leaderboard** | Standardized evals and especially good coverage for open-weight models | Open-model supplement or extra columns for shared eval suites | Mostly open-weight coverage, so it will skew away from proprietary models |
| **SWE-bench / Terminal-Bench / τ-bench leaderboards** | Good fit for the repository's coding-agent audience and complements existing coding metrics | Extra agentic/coding columns or a separate "software engineering" subsection | Model coverage is narrower and some leaderboards focus more on agents than raw base models |

## Good Raw Material, But More Work

These are useful if you want to build a custom import pipeline rather than pull a ready-made leaderboard:

- **OpenAI Evals** — broad benchmark registry, but you would need to decide which evals to aggregate and how to normalize the outputs
- **HELM / Stanford CRFM** — strong methodology and reporting, but integration is more bespoke than a single leaderboard feed
- **lm-evaluation-harness result repos** — lots of raw benchmark outputs in the ecosystem, but quality and consistency vary by publisher

## Suggested Priority Order

If the goal is "most value for the least implementation complexity", a reasonable order is:

1. **LiveBench** — broad coverage and closest in spirit to the existing tab
2. **LMSYS Chatbot Arena** — adds a very different but widely recognized signal
3. **Aider** or **SWE-bench / Terminal-Bench** — best next step if you want more coding-heavy comparisons
4. **Hugging Face Open LLM Leaderboard** — especially useful if you want stronger open-weight coverage

## Integration Notes For This Repo

Before adding a new source, decide whether it should be:

- a **new fetcher** parallel to `src/benchmark_fetch.rs`
- an **offline generated file** in `data/` refreshed by GitHub Actions, like `data/benchmarks.json`
- a **new set of columns** on `BenchmarkEntry`
- or a **separate source/view** in the benchmarks tab if the data is not directly comparable to Artificial Analysis

If you add new `BenchmarkEntry` fields, remember to mark them with `#[serde(default)]` so older or partial source payloads still deserialize cleanly.
