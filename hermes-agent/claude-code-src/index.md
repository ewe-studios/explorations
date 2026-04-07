# Claude Code Source — Comprehensive Exploration Index

**Repository:** claude-code-main  
**Source Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src`  
**Generated:** 2026-04-07

---

## Repository Overview

The official Claude Code TypeScript codebase containing the full implementation of the Claude Code CLI and desktop application.

### Statistics

| Metric | Count |
|--------|-------|
| Top-level directories | 35 |
| Total TypeScript files | ~1,800+ |
| Largest module | `utils/` (542 files) |
| Second largest | `components/` (331 files) |
| Third largest | `commands/` (189 files) |

---

## Module Inventory

### Core Modules

| Directory | Files | Description | Exploration |
|-----------|-------|-------------|-------------|
| `bridge/` | 31 | Bridge communication protocol, REPL transport, JWT, remote sessions | [exploration.md](./bridge/exploration.md) |
| `cli/` | 19 | CLI entrypoints, transports (SSE, WebSocket), NDJSON, remote I/O | [exploration.md](./cli/exploration.md) |
| `commands/` | 189 | All slash commands implementation | [exploration.md](./commands/exploration.md) |
| `components/` | 331 | React UI components | [exploration.md](./components/exploration.md) |
| `hooks/` | 101 | React hooks for state management | [exploration.md](./hooks/exploration.md) |
| `ink/` | 96 | Terminal UI rendering (Ink framework) | [exploration.md](./ink/exploration.md) |
| `services/` | 130 | Background services and daemons | [exploration.md](./services/exploration.md) |
| `tools/` | 178 | Tool implementations (bash, file ops, web) | [exploration.md](./tools/exploration.md) |
| `utils/` | 542 | Utility functions and helpers | [exploration.md](./utils/exploration.md) |

### State and Configuration

| Directory | Files | Description | Exploration |
|-----------|-------|-------------|-------------|
| `state/` | 6 | Application state management | [exploration.md](./state/exploration.md) |
| `bootstrap/` | 1 | Bootstrap phase detection and state | [exploration.md](./bootstrap/exploration.md) |
| `constants/` | 21 | Application constants | [exploration.md](./constants/exploration.md) |
| `config/` | - | Configuration handling | [exploration.md](./config/exploration.md) |
| `context/` | 9 | React context providers | [exploration.md](./context/exploration.md) |
| `types/` | 7 | TypeScript type definitions | [exploration.md](./types/exploration.md) |

### Session and Conversation

| Directory | Files | Description | Exploration |
|-----------|-------|-------------|-------------|
| `assistant/` | 1 | Assistant session history | [exploration.md](./assistant/exploration.md) |
| `history/` | - | Conversation history management | [exploration.md](./history/exploration.md) |
| `tasks/` | 12 | Task tracking and management | [exploration.md](./tasks/exploration.md) |

### Specialized Modules

| Directory | Files | Description | Exploration |
|-----------|-------|-------------|-------------|
| `buddy/` | 6 | Companion/buddy features | [exploration.md](./buddy/exploration.md) |
| `coordinator/` | 1 | Task coordination | [exploration.md](./coordinator/exploration.md) |
| `keybindings/` | 14 | Keyboard shortcut definitions | [exploration.md](./keybindings/exploration.md) |
| `memdir/` | 8 | Memory directory management | [exploration.md](./memdir/exploration.md) |
| `migrations/` | 11 | Data migrations | [exploration.md](./migrations/exploration.md) |
| `plugins/` | 2 | Plugin system | [exploration.md](./plugins/exploration.md) |
| `mcp/` | 1 | MCP implementation (deep dive) | [claude-code-mcp-exploration.md](./mcp/claude-code-mcp-exploration.md) |
| `query/` | 4 | Query engine interface | [exploration.md](./query/exploration.md) |
| `remote/` | 4 | Remote execution | [exploration.md](./remote/exploration.md) |
| `remote-execution/` | 1 | Remote execution deep dive | [claude-code-remote-execution.md](./remote-execution/claude-code-remote-execution.md) |
| `schemas/` | 1 | JSON schemas | [exploration.md](./schemas/exploration.md) |
| `screens/` | 3 | Screen definitions | [exploration.md](./screens/exploration.md) |
| `server/` | 3 | Local server | [exploration.md](./server/exploration.md) |
| `skills/` | 20 | Skill definitions | [exploration.md](./skills/exploration.md) |
| `upstreamproxy/` | 2 | Upstream proxy | [exploration.md](./upstreamproxy/exploration.md) |
| `vim/` | 5 | Vim mode support | [exploration.md](./vim/exploration.md) |
| `voice/` | 1 | Voice input support | [exploration.md](./voice/exploration.md) |

### Additional Modules

| Directory | Files | Description | Exploration |
|-----------|-------|-------------|-------------|
| `entrypoints/` | 8 | Application entry points | [exploration.md](./entrypoints/exploration.md) |
| `moreright/` | 1 | Additional features | [exploration.md](./moreright/exploration.md) |
| `native-ts/` | 4 | Native TypeScript bindings | [exploration.md](./native-ts/exploration.md) |
| `outputStyles/` | 1 | Output styling | [exploration.md](./outputStyles/exploration.md) |

### Root-Level Files

| File | Size | Description |
|------|------|-------------|
| `main.tsx` | 803KB | Main application entry |
| `commands.ts` | 25KB | Command registry |
| `tools.ts` | 17KB | Tool registry |
| `Tool.ts` | 29KB | Tool base class |
| `Task.ts` | 3KB | Task definition |
| `QueryEngine.ts` | 46KB | Query engine |
| `query.ts` | 68KB | Query implementation |
| `cost-tracker.ts` | 10KB | Token cost tracking |
| `dialogLaunchers.tsx` | 22KB | Dialog components |
| `history.ts` | 14KB | History management |
| `interactiveHelpers.tsx` | 57KB | Interactive helpers |
| `setup.ts` | 20KB | Application setup |

---

## Task Progress

See [task.md](../task.md) for the complete task list and progress tracking.

---

## Exploration Status

| Status | Count | Modules |
|--------|-------|---------|
| Completed | 38 | All modules + server + remote-execution + mcp |
| In Progress | 0 | - |
| Pending | 0 | - |

---

**Last Updated:** 2026-04-07
