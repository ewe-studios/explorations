# CLI Commands and Configuration

The OpenMono CLI provides commands for running the agent, managing sessions, and configuring the environment.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.OpenMonoAgent/OpenMonoAgent.ai/src/OpenMono.Cli/Commands/`

## Commands

| Command | Purpose |
|---------|---------|
| `openmono run` | Start new conversation |
| `openmono continue` | Continue last session |
| `openmono compact` | Compact session history |
| `openmono config` | Show/edit configuration |
| `openmono tools` | List available tools |
| `openmono hooks` | List/manage hooks |
| `openmono playbooks` | List/run playbooks |

## Configuration

**Source:** `Config/`

Configuration is loaded from `~/.openmono/config.json` with environment variable overrides.

### Config Structure

```json
{
  "llm": {
    "endpoint": "http://localhost:7474",
    "api_key": "...",
    "model": "qwen3.6"
  },
  "workspace": "/workspace",
  "permissions": {
    "auto_allow": ["read", "glob"]
  }
}
```

## Workspace

The workspace is bind-mounted at `/workspace` in the Docker container. The agent cannot escape this directory.
