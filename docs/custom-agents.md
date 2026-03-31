# Custom Agents

You can add custom agents to track alongside built-in agents. Custom agents appear in both the TUI and CLI, with automatic GitHub data fetching (stars, releases, changelogs).

## Config Location

`~/.config/models/config.toml`

## Adding Custom Agents

Add a `[[agents.custom]]` block for each agent you want to track:

```toml
[[agents.custom]]
name = "My Internal Agent"
repo = "myorg/internal-agent"
agent_type = "cli"
binary = "myagent"
version_command = ["myagent", "--version"]

[[agents.custom]]
name = "Custom IDE Plugin"
repo = "myorg/ide-plugin"
agent_type = "ide"
```

Each `[[agents.custom]]` block adds one agent. To track multiple custom agents, add multiple blocks.

## Fields

| Field | Required | Description |
|-------|----------|-------------|
| name | Yes | Display name for the agent |
| repo | Yes | GitHub repo (owner/repo format) |
| agent_type | No | "cli" or "ide" |
| binary | No | CLI binary name for version detection |
| version_command | No | Command to get installed version (e.g., `["myagent", "--version"]`) |

## Notes

- Custom agents are tracked by default — no need to add them to `[agents] tracked`
- If a custom agent has the same name as a built-in agent, the built-in is used
- GitHub data (stars, releases, changelogs) is fetched automatically
- Version detection uses `binary` + `version_command` to find the installed version; falls back to semver pattern matching (`x.y.z`) if no custom `version_regex` is provided
