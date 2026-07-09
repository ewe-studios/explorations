---
title: Herdr — CLI Command Reference
---

# Herdr — CLI Command Reference

**Herdr provides a comprehensive CLI for managing sessions, workspaces, tabs, panes, agents, worktrees, and integrations.** Commands communicate with the running server via the JSON-RPC API socket.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli.rs` and submodules

See [07-configuration.md](07-configuration.md) for configuration. This document covers all CLI commands.

## Command Routing

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli.rs:25-53`

The `maybe_run()` function dispatches CLI subcommands:

```rust
// cli.rs:25-53
pub fn maybe_run(args: &[String]) -> io::Result<CommandOutcome> {
    match args.get(1).map(|s| s.as_str()) {
        Some("server") => run_server_command(args),
        Some("status") => run_status_command(args),
        Some("config") => run_config_command(args),
        Some("channel") => run_channel_command(args),
        Some("workspace") => run_workspace_command(args),
        Some("worktree") => run_worktree_command(args),
        Some("tab") => run_tab_command(args),
        Some("pane") => run_pane_command(args),
        Some("agent") => run_agent_command(args),
        Some("terminal") => run_terminal_command(args),
        Some("wait") => run_wait_command(args),
        Some("session") => run_session_command(args),
        Some("integration") => run_integration_command(args),
        _ => Ok(CommandOutcome::NotCli),
    }
}
```

Commands that return `CommandOutcome::Handled(code)` exit immediately with the given code. `NotCli` means the args should be handled by the normal launch flow.

## Launch Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/main.rs:332-513`

| Command | Description |
|---------|-------------|
| `herdr` | Launch or attach to persistent session (auto-detect) |
| `herdr --no-session` | Run monolithically (no server/client) |
| `herdr --session <name>` | Use or create a named persistent session |
| `herdr --remote <target>` | Attach through SSH to a remote Herdr server |
| `herdr --version` / `-V` | Print version and exit |
| `herdr --help` / `-h` | Show help and exit |
| `herdr --default-config` | Print default config and exit |

## Server Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/server.rs`

| Command | Description |
|---------|-------------|
| `herdr server` | Run as headless server (daemon mode) |
| `herdr server stop` | Stop the running server via API socket |
| `herdr server reload-config` | Reload config.toml in running server |
| `herdr server live-handoff` | Perform live handoff (for updates) |

### Server Live Handoff

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/server.rs:45-74`

```bash
herdr server live-handoff --timeout 30
```

This triggers a live handoff to a new server instance. Used during self-updates to transfer running sessions without disconnecting PTY children.

## Status Command

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/status.rs`

| Command | Description |
|---------|-------------|
| `herdr status` | Show full status (server + client) |
| `herdr status server` | Show server status only |
| `herdr status client` | Show client status only |

The status output includes:
- Server running state (version, protocol, live handoff capability)
- Client connection state (socket path, connection status)
- Update status (available version, channel)

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/status.rs:82-109`

## Config Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli.rs:182-199`

| Command | Description |
|---------|-------------|
| `herdr config reset-keys` | Back up config.toml and remove custom keybindings |

## Channel Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli.rs:55-72`

| Command | Description |
|---------|-------------|
| `herdr channel set <stable|preview>` | Choose the update channel |
| `herdr channel` | Show current channel |

Setting the preview channel from a stable binary requires `--allow-preview` confirmation.

## Workspace Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/workspace.rs`

| Command | Description |
|---------|-------------|
| `herdr workspace list` | List all workspaces |
| `herdr workspace create [--cwd <path>]` | Create a new workspace |
| `herdr workspace get <id>` | Get workspace details |
| `herdr workspace focus <id>` | Focus a workspace |
| `herdr workspace rename <id> <name>` | Rename a workspace |
| `herdr workspace close <id>` | Close a workspace |

## Worktree Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/worktree.rs`

| Command | Description |
|---------|-------------|
| `herdr worktree list` | List all worktrees |
| `herdr worktree create [--path <path>] [--branch <branch>]` | Create a worktree |
| `herdr worktree open <path>` | Open an existing worktree |
| `herdr worktree remove <path>` | Remove a worktree |

## Tab Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/tab.rs`

| Command | Description |
|---------|-------------|
| `herdr tab list [--workspace <id>]` | List tabs in workspace |
| `herdr tab create [--workspace <id>] [--cwd <path>]` | Create a tab |
| `herdr tab get <id>` | Get tab details |
| `herdr tab focus <id>` | Focus a tab |
| `herdr tab rename <id> <name>` | Rename a tab |
| `herdr tab close <id>` | Close a tab |

## Pane Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/pane.rs`

| Command | Description |
|---------|-------------|
| `herdr pane list [--workspace <id>] [--tab <id>]` | List panes |
| `herdr pane get <id>` | Get pane details |
| `herdr pane rename <id> <name>` | Rename a pane |
| `herdr pane read --source <source> --format <format>` | Read pane content |
| `herdr pane split <id> --direction <h|v>` | Split a pane |
| `herdr pane close <id>` | Close a pane |
| `herdr pane send-text <id> <text>` | Send text to pane |
| `herdr pane send-keys <id> <keys>` | Send keys to pane |
| `herdr pane run <id> <command>` | Run a command in pane |
| `herdr pane report-agent <id>` | Report agent state for pane |
| `herdr pane report-metadata <id>` | Report metadata for pane |

### Pane Read

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/pane.rs:105-180`

```bash
herdr pane read --source recent --format text
herdr pane read --source visible --format ansi
```

Sources: `visible`, `recent`, `detection`
Formats: `text`, `ansi`

This is used by agent integrations to capture terminal output for agent detection development.

## Agent Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/agent.rs`

| Command | Description |
|---------|-------------|
| `herdr agent start [--cwd <path>] [--agent <name>]` | Start an agent session |
| `herdr agent list [--workspace <id>]` | List agent sessions |
| `herdr agent get <id>` | Get agent details |
| `herdr agent focus <id>` | Focus an agent pane |
| `herdr agent attach <id>` | Attach to an agent pane |
| `herdr agent wait <id> --status <status>` | Wait for agent state change |
| `herdr agent rename <id> <name>` | Rename an agent pane |
| `herdr agent send <id> <text>` | Send text to agent |
| `herdr agent read <id>` | Read agent terminal output |

### Agent Wait

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/agent.rs:186-276`

```bash
herdr agent wait claude --status idle
herdr agent wait claude --status working --timeout 60
```

This command blocks until the agent reaches the specified state (idle/working/blocked). Useful for scripting agent workflows.

## Terminal Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli.rs:290-307`

| Command | Description |
|---------|-------------|
| `herdr terminal attach <terminal-id>` | Attach to a specific terminal pane |

## Wait Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli.rs:309-327`

| Command | Description |
|---------|-------------|
| `herdr wait output <pane-id>` | Wait for pane output |
| `herdr wait agent-status <pane-id> <status>` | Wait for agent state change |

## Session Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli.rs:329-437`

| Command | Description |
|---------|-------------|
| `herdr session attach <name>` | Attach to a named session |
| `herdr session list` | List all sessions |
| `herdr session stop <name>` | Stop a named session |
| `herdr session delete <name>` | Delete a named session |

## Integration Commands

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli/integration.rs`

| Command | Description |
|---------|-------------|
| `herdr integration status` | Show integration install status |
| `herdr integration install <target>` | Install an integration |
| `herdr integration uninstall <target>` | Uninstall an integration |

Integration targets include Claude Code, Codex, and other supported agents.

## API Communication

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/cli.rs:683-701`

CLI commands communicate with the server via the JSON-RPC API socket:

```rust
// cli.rs:683-687
fn send_request(request: &Request) -> io::Result<String> {
    // Connect to herdr.sock
    // Send JSON-RPC request
    // Read response
}
```

The API socket path is resolved using the same precedence as the client socket.

## Help Output

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/src/main.rs:411-512`

Running `herdr --help` shows:

```
herdr — terminal workspace manager for AI coding agents

Usage: herdr [options]
       herdr --session <name> [options]
       herdr --remote <ssh-target> [--session <name>]
       herdr session attach <name>
       herdr update [--handoff]
       herdr channel set <stable|preview>
       herdr server stop
       ...

Common commands:
  herdr                            Launch or attach to the persistent session
  herdr status [server|client]     Show local client and running server status
  herdr update                     Download and install the latest version
  herdr server stop                Stop the running server via the API socket
  ...
```

## Cross-References

- [01-architecture.md](01-architecture.md) — Overall system architecture
- [06-server-client.md](06-server-client.md) — Server/client communication
- [07-configuration.md](07-configuration.md) — Configuration options
- [09-data-flow.md](09-data-flow.md) — How CLI commands flow through the system

→ Next: [09-data-flow.md](09-data-flow.md) — End-to-end data flows
