# Custom Agents

You can add custom agents to track in the Models TUI by editing your config file.

## Config Location

`~/.config/models/config.toml`

## Adding Custom Agents

Add a `[[agents.custom]]` section for each agent:

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

## Fields

| Field | Required | Description |
|-------|----------|-------------|
| name | Yes | Display name for the agent |
| repo | Yes | GitHub repo (owner/repo format) |
| agent_type | No | "cli" or "ide" |
| binary | No | CLI binary name for version detection |
| version_command | No | Command to get installed version |

## Notes

- Custom agents are tracked by default
- If a custom agent has the same name as a built-in agent, the built-in is used
- GitHub data (stars, releases) is fetched automatically
