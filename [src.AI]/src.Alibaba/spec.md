---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Alibaba/page-agent/
language: TypeScript
version: 1.10.0
explored_at: 2026-07-04
license: MIT
repository: github.com/alibaba/page-agent
---

# spec.md — Alibaba Page-Agent Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Alibaba/page-agent/`
- **Language:** TypeScript 6.0
- **Version:** 1.10.0
- **LOC:** 62 TypeScript files across 8 packages
- **License:** MIT
- **Repository:** github.com/alibaba/page-agent

## What Page-Agent Is

Page-Agent is an AI-powered browser automation agent by Alibaba that uses LLMs (via OpenAI-compatible APIs) to control web pages through a structured "Re-act" (Reasoning + Acting) loop. The agent observes the browser DOM, thinks via an LLM call producing both reflection (evaluation, memory, next goal) and action selection, then executes the chosen action through a `PageController` that manipulates the real DOM. It ships as a browser extension with an 8-package monorepo architecture.

## Architecture

1. **Core** — `PageAgentCore` class: Re-act loop, MacroTool pattern, event system, prompt assembly
2. **LLMs** — OpenAI-compatible client with retry logic and tool calling
3. **Page Controller** — DOM flattening, element indexing, click/input/scroll actions
4. **Extension** — Background/content scripts, multi-page agent
5. **MCP** — Model Context Protocol server
6. **UI** — React panel for extension sidebar
7. **Website** — Documentation site

## Documentation Structure

```
src.Alibaba/
├── spec.md
├── build.py
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-agent-loop.md
│   ├── 03-tools.md
│   ├── 04-page-controller.md
│   ├── 05-llm-integration.md
│   └── 06-extension-mcp.md
└── html/
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-architecture.md | DONE |
| 3 | 02-agent-loop.md | DONE |
| 4 | 03-tools.md | DONE |
| 5 | 04-page-controller.md | DONE |
| 6 | 05-llm-integration.md | DONE |
| 7 | 06-extension-mcp.md | DONE |
| 8 | HTML generation | DONE |
| 9 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Alibaba
python3 build.py .
```

## Quality Requirements

All Iron Rules from documentation_directive.md apply: code snippets from source, ≥2 mermaid diagrams per doc, ≥1 Aha moment per doc, cross-references, source path refs, grandfather review.

## Expected Outcome

A reader should understand: the Re-act agent loop, the MacroTool pattern, DOM flattening for LLM consumption, tool definition and execution, LLM client retry logic, and multi-page browser extension orchestration.
