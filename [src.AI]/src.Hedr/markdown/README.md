# Herdr — Terminal Workspace Manager for AI Coding Agents

Rust TUI that auto-detects 17 AI agents, manages workspaces/tabs/panes, and provides remote terminal handoff. Built with ratatui + portable-pty.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/` — ~130 Rust files, AGPL-3.0, v0.6.8

## Documents

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | What Herdr is, architecture at a glance, key design decisions |
| 01 | [Architecture](01-architecture.md) | Layer map, module dependency graph, event loop, config architecture |
| 02 | [Agent Detection](02-agent-detection.md) | 17 AI agent detectors, process identification, state stabilization, visibility signals |
| 03 | [PTY Management](03-pty-management.md) | PTY actor model, handoff mechanics, terminal parser, OSC tracking, dirty patches |
| 04 | [Workspace/Tabs/Panes](04-workspace-tabs-panes.md) | Workspace model, layout tree, git integration, navigator, selection, session restore |
| 05 | [TUI Rendering](05-tui-rendering.md) | ratatui rendering pipeline, mobile layout, sync output, Kitty graphics |
| 06 | [Server/Client](06-server-client.md) | Headless server, wire protocol (v12), live handoff, remote SSH bridging |
| 07 | [Configuration](07-configuration.md) | TOML config sections, live reload, keybinding system, theme system |
| 08 | [CLI Commands](08-cli-commands.md) | All CLI subcommands, API communication, server/workspace/tab/pane/agent commands |
| 09 | [Data Flow](09-data-flow.md) | End-to-end flows: session creation, input routing, detection, save/restore, handoff, render |
| 10 | [Error Handling](10-error-handling.md) | Panic hooks, PTY errors, config fallback, handoff recovery, nested prevention |

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]
python3 build.py src.Hedr
```
