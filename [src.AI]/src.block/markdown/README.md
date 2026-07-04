# Goose — On-Machine AI Agent by Block

Goose is an on-machine AI agent built by Block (Square) that operates as a developer's programmable assistant. It connects to any LLM through a pluggable provider system, orchestrates tools through MCP extensions, enforces a multi-layered security/permission system, manages persistent sessions via SQLite, and runs as both a CLI TUI and HTTP server.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.block/goose/` — 337 Rust files, 96,846 LOC, 6 crates, Apache-2.0

## Documents

### Foundation
| # | Document | What You'll Learn |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | 6 crates, key design decisions, system architecture at a glance |
| 01 | [Architecture](01-architecture.md) | Agent struct, reply loop, tool dispatch, event system, compaction |

### Deep Dives
| # | Document | What You'll Learn |
|---|----------|-------------------|
| 02 | [Extensions](02-extensions.md) | ExtensionManager, MCP client, platform extensions, 5 transport types |
| 03 | [Providers](03-providers.md) | Provider trait, 8+ format backends, LeadWorker router, model metadata |
| 04 | [Session & Conversation](04-session-conversation.md) | SQLite session persistence, conversation compaction, context management |
| 05 | [Security & Permissions](05-security-permissions.md) | ToolInspectionManager, 3-layer pipeline, SecurityInspector, PermissionJudge |
| 06 | [MCP Servers](06-mcp-servers.md) | 5 built-in MCP servers: developer, computercontroller, memory, tutorial, autovisualiser |

### Interfaces
| # | Document | What You'll Learn |
|---|----------|-------------------|
| 07 | [CLI & Server](07-cli-server.md) | CLI TUI, HTTP server (goosed), GooseMode, recipes, scheduling, OpenAPI |

## Quick Start

```bash
# Start a session
goose session

# Start with a specific provider
goose session --provider openai

# Start HTTP server
goosed --port 3000
```

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.block
python3 build.py .
```
