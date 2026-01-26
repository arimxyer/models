---
name: Update Agents Data
on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday
  workflow_dispatch: {}
permissions:
  contents: read
  pull-requests: write
---

# Update Agents Data

Read the coding agents comparison page from artificialanalysis.ai and update our data file.

## Instructions

1. Fetch the page at https://artificialanalysis.ai/insights/coding-agents-comparison
2. Extract the comparison table data for each coding agent/assistant:
   - Name
   - Category (CLI, IDE, Extension, Cloud)
   - Pricing model (free, subscription, usage-based, hybrid)
   - Supported model providers
   - Open source status
3. Read the existing `data/agents.json` file
4. For each agent found on the page:
   - If it exists in our file, update the scraped fields (pricing, category, providers)
   - If it's new, add a skeleton entry (we'll fill in repo/version details manually)
   - Preserve fields that aren't on the page (repo, cli_binary, version_command, etc.)
5. Do NOT remove agents that exist in our file but aren't on the page (they may be user additions)
6. If any changes were made, create a PR with:
   - Title: "chore: update agents data from artificialanalysis.ai"
   - Body: Summary of changes (agents added, agents updated)

## Important

- Keep the schema_version unchanged
- Update last_scraped to current timestamp
- Set scrape_source to "artificialanalysis.ai"
- Preserve all existing repo URLs and version detection settings
