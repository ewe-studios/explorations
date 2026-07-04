---
title: src.Google — Google's AI Agent Ecosystem
---

# src.Google — Google's AI Agent Ecosystem

**12 Google projects spanning agent frameworks, UI design SDKs, cloud infrastructure, and knowledge management.** Each major project has its own deep-dive document covering architecture, key types, and design decisions with actual source code.

Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Google/` — 12 projects

## Project Map

| # | Project | Language | Purpose | Deep Dive |
|---|---------|----------|---------|-----------|
| 1 | **adk-go** | Go | Agent Development Kit — official Go agent framework (Apache 2.0) | [→](01-adk-go.md) |
| 2 | **adk-go-example** | Go | Multi-agent research assistant: 7+ agents, parallel execution, entity persistence | — |
| 3 | **stitch-sdk** | TypeScript | `@google/stitch-sdk` — AI UI generation SDK (MCP transport, AI SDK/ADK adapters) | [→](02-stitch-sdk.md) |
| 4 | **stitch-mcp** | TypeScript | CLI + MCP proxy for Stitch dev workflows | — |
| 5 | **stitchkit** | Markdown | Claude Code plugin: Stitch + Figma + NanoBanana MCP servers | — |
| 6 | **Centravity** | TypeScript | Open-source agentic IDE: ModelGateway, Liquid Working Memory, 14-table SQLite | [→](03-centravity.md) |
| 7 | **gcsfuse** | Go | GCS FUSE adapter: streaming writes, parallel downloads, ML auto-optimization | [→](05-gcsfuse.md) |
| 8 | **design.md** | TypeScript | DESIGN.md format spec + CLI: lint, diff, export to Tailwind/W3C DTCG | [→](04-design-md.md) |
| 9 | **generative-ai** | Python | 30+ Gemini notebooks: vision, audio, embeddings, RAG, agents, tuning | [→](06-generative-ai.md) |
| 10 | **agents-cli** | Python | CLI + 7 skills for Gemini Enterprise Agent Platform | — |
| 11 | **knowledge-catalog** | Python/TS | AI data catalog with Open Knowledge Format (OKF) | — |
| 12 | **llm-wiki.md** | Markdown | Design pattern: LLM incrementally builds structured markdown wikis | — |

## Quick Summary

- **adk-go** — Google's answer to LangChain/LlamaIndex. Code-first Go agent framework with tree routing, callback lifecycle, and A2A protocol support.
- **Stitch ecosystem** — AI-powered UI generation. SDK wraps MCP server behind lazy-loading domain model with schema repair resilience.
- **Centravity** — Open-source agentic IDE. Most interesting feature: Liquid Working Memory using ODEs and Hebbian learning, not vector DBs.
- **gcsfuse** — Production-grade cloud storage for ML. Auto-detects A100/H100 machines and applies performance profiles.
- **design.md** — Format for describing brand identity to coding agents. Prose matters more than tokens.

## Cross-References

- [src.Wayfinder](../src.Wayfinder/markdown/00-overview.md) — Deterministic knowledge management (RAC)
- [src.opendesign](../src.opendesign/markdown/00-overview.md) — Open Design ecosystem
- [Goose](../src.block/markdown/00-overview.md) — Block's on-machine AI agent
