---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.block/goose/
language: Rust
version: 1.13.0
explored_at: 2026-07-04
license: Apache-2.0
repository: github.com/block/goose
---

# spec.md — Goose AI Agent Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.block/goose/`
- **Language:** Rust (edition 2021)
- **Version:** 1.13.0
- **LOC:** 96,846 Rust lines (337 files across 6 crates)
- **License:** Apache-2.0
- **Author:** Block (Square)
- **Repository:** github.com/block/goose

## What Goose Is

Goose is an on-machine AI agent built by Block that operates as a developer's programmable assistant. It connects to any LLM through a pluggable provider system (OpenAI, Anthropic, Bedrock, Vertex AI, Databricks, GitHub Copilot, Venice, Ollama, and a declarative config format), orchestrates tools through MCP extensions — both built-in (developer, computer controller, memory, tutorial, autovisualiser) and user-installed — enforces a multi-layered security/permission system, manages persistent sessions via SQLite, supports cron-based scheduled execution, recipe-based automation, sub-agent delegation, and conversation compaction. It runs as both a CLI TUI and an HTTP server (goosed) with OpenAPI docs.

## Architecture

1. **Agent Core** (`goose`) — Agent loop, extension manager, tool execution, providers, conversation, session, security
2. **CLI** (`goose-cli`) — TUI with clap/cliclack, session management, recipe loading
3. **MCP Servers** (`goose-mcp`) — 5 built-in MCP servers
4. **HTTP Server** (`goose-server`) — Axum REST API + WebSocket, OpenAPI
5. **Benchmarks** (`goose-bench`) — Eval suites
6. **Test** (`goose-test`) — Capture/replay utilities

## Documentation Structure

```
src.block/
├── spec.md
├── build.py
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-extensions.md
│   ├── 03-providers.md
│   ├── 04-session-conversation.md
│   ├── 05-security-permissions.md
│   ├── 06-mcp-servers.md
│   └── 07-cli-server.md
└── html/
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE | 00-overview.md | DONE |
| 1 | 01-architecture.md | DONE |
| 2 | 02-extensions.md | DONE |
| 3 | 03-providers.md | DONE |
| 4 | 04-security-permissions.md | DONE |
| 5 | 05-session-conversation.md | DONE |
| 6 | 06-compaction-context.md | DONE |
| 7 | 07-hints-context.md | DONE |
| 8 | 08-recipes.md | DONE |
| 9 | 09-autopilot-retry.md | DONE |
| 10 | 10-mcp-servers.md | DONE |
| 11 | 11-tracing.md | DONE |
| 12 | 12-cli.md | DONE |
| 13 | 13-server.md | DONE |
| 14 | HTML generation | DONE |
| 15 | Grandfather review | DONE |

**Total:** 14 deep docs, 15,944 words, every major subsystem covered.

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.block
python3 build.py .
```

## Quality Requirements

All Iron Rules from documentation_directive.md apply.

## Expected Outcome

A reader should understand: the Agent architecture and reply loop, ExtensionManager and MCP client system, provider abstraction with 8+ format backends, SQLite session persistence, 3-layer tool inspection pipeline, all 5 built-in MCP servers, and both CLI TUI and HTTP server interfaces.
