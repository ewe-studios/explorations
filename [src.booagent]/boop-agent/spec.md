---
name: booagent-boop-agent
description: Documentation project for boop-agent — iMessage-based personal agent
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.booagent/boop-agent
language: TypeScript
version: 0.2.0
author: Chris Roarque (raroque)
license: MIT
---

# Boop-Agent Documentation Spec

## What the Project Is

Boop is an iMessage-based personal agent powered by Claude Agent SDK or OpenAI Codex. Users text it like a person; it dispatches work to sub-agents, remembers facts across conversations, schedules recurring automations, and connects to 1000+ services through Composio. It's a fork-and-own template, not a SaaS product.

## Documentation Goal

A reader should be able to:
1. Understand the dispatcher/executor architecture and why it's split
2. Trace a message from iMessage webhook through to reply delivery
3. Configure, extend, and deploy the system
4. Understand every memory tier, decay model, and consolidation pipeline
5. Connect new Composio toolkits or write custom MCP integrations
6. Tune the runtime (Claude vs Codex), models, and browser/Apple features
7. Read the Convex data model and understand every table and index
8. Monitor via the debug dashboard and troubleshoot production issues

## Documentation Structure

```
boop-agent/
├── spec.md
├── markdown/
│   ├── README.md                         ← Index / table of contents
│   ├── 00-overview.md                    ← What it is, philosophy, quick architecture
│   ├── 01-architecture.md                ← Full dependency graph, layer diagram, module map
│   ├── 02-interaction-agent.md           ← Dispatcher deep dive
│   ├── 03-execution-agent.md             ← Sub-agent worker deep dive
│   ├── 04-memory-system.md               ← Memory tools, extraction, decay, cleaning
│   ├── 05-runtimes.md                    ← Claude + Codex runtime adapters
│   ├── 06-integrations-composio.md       ← Composio integration system
│   ├── 07-automations.md                 ← Cron scheduling + execution
│   ├── 08-consolidation.md               ← Adversarial memory consolidation pipeline
│   ├── 09-sendblue-imessage.md           ← iMessage bridge via Sendblue
│   ├── 10-convex-data-model.md           ← Convex schema, tables, indexes
│   ├── 11-optional-features.md           ← Browser, Apple data, local Mac features
│   ├── 12-cost-tracking.md               ← Usage logging and cost estimation
│   ├── 13-cross-cutting.md               ← Privacy, security, heartbeat, broadcast
│   └── 14-setup-deploy.md                ← Setup, dev, desktop, production
├── html/
│   ├── index.html
│   └── *.html
└── build.py
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 00 | spec.md | DONE |
| 01 | README.md | DONE |
| 02 | 00-overview.md | DONE |
| 03 | 01-architecture.md | DONE |
| 04 | 02-interaction-agent.md | DONE |
| 05 | 03-execution-agent.md | DONE |
| 06 | 04-memory-system.md | DONE |
| 07 | 05-runtimes.md | DONE |
| 08 | 06-integrations-composio.md | DONE |
| 09 | 07-automations.md | DONE |
| 10 | 08-consolidation.md | DONE |
| 11 | 09-sendblue-imessage.md | DONE |
| 12 | 10-convex-data-model.md | DONE |
| 13 | 11-optional-features.md | DONE |
| 14 | 12-cost-tracking.md | DONE |
| 15 | 13-cross-cutting.md | DONE |
| 16 | 14-setup-deploy.md | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.booagent]/boop-agent
python3 build.py boop-agent    # build all docs
```

Shared `build.py` at repo root. No external dependencies (Python 3.12+ stdlib).

## Quality Requirements

All 10 Iron Rules from documentation_directive.md apply:
1. Detailed sections with code snippets
2. Teach key facts, principles, and ideas quickly
3. Clear articulation
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, ASCII art, code blocks)
6. Generated HTML with navigation
7. Cross-references between documents
8. Source path references
9. Aha moments (at least 1 per document)
10. Navigation: index + prev/next buttons

## Expected Outcome

A reader can understand Boop's architecture, trace any message flow, configure integrations, extend the system, and troubleshoot — without reading the source code.

## Resume Point

If interrupted, check the Tasks table above for the next TODO document.
