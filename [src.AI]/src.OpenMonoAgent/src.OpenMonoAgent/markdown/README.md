# src.OpenMonoAgent Documentation

A .NET 10 CLI agentic coding tool running locally against llama.cpp or any OpenAI-compatible endpoint.

| Attribute | Value |
|-----------|-------|
| Language | C# (.NET 10) |
| Source Files | 190 (145 CLI + 45 tests) |
| License | MIT |
| Runtime | Docker container |

## Navigation

Start here:

- **[00-overview](00-overview.md)** — What OpenMono is, architecture, topology

Core systems:

- **[01-cli](01-cli.md)** — CLI commands, configuration, setup
- **[02-conversation-loop](02-conversation-loop.md)** — Agent loop, LLM client, streaming
- **[03-tools](03-tools.md)** — 20+ built-in tools, MCP, LSP, Roslyn
- **[04-permissions](04-permissions.md)** — Permission engine, auto-allow rules
- **[05-session](05-session.md)** — JSONL session management, compaction

UI and operations:

- **[06-tui](06-tui.md)** — Terminal UI, rendering
- **[07-playbooks](07-playbooks.md)** — Playbook system, examples
- **[08-deployment](08-deployment.md)** — Docker setup, Caddy gateway, services
