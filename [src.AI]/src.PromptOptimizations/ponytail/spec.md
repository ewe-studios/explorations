---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PromptOptimizations/ponytail/
language: JavaScript/Python
version: 0.1.0
runtime: Node.js
explored_at: 2026-07-04
license: MIT
---

# spec.md — Ponytail Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PromptOptimizations/ponytail/`
- **Language:** JavaScript (Node.js hooks), Python (benchmarks)
- **Version:** 0.1.0
- **License:** MIT
- **Source scale:** ~35 source files (hooks, scripts, tests, benchmarks)

## What Ponytail Is

A "lazy senior dev" mode for AI coding agents. When activated, it instructs agents to write minimal code — replacing 50-line implementations with one-liners where possible. Measured to reduce code by ~54% (up to 94%), lower costs by ~20%, and speed up sessions by ~27% while keeping all safety guards.

Supports 14 AI agents: Claude Code, Gemini CLI, Codex CLI, Copilot CLI, OpenCode, Aider, and more.

## Documentation Structure

```
ponytail/
├── spec.md
├── markdown/
│   ├── README.md            ← Index / table of contents
│   ├── 00-overview.md       ← What ponytail is, philosophy, numbers, agents
│   ├── 01-architecture.md   ← Hooks, config resolution, runtime state, MCP
│   ├── 02-benchmarks.md     ← Agentic benchmarks, correctness, robustness
│   └── 03-integrations.md   ← Agent plugins, hooks, skills
└── html/                    ← Generated HTML
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | README.md | DONE |
| 2 | 00-overview.md | DONE |
| 3 | 01-architecture.md | DONE |
| 4 | 02-benchmarks.md | DONE |
| 5 | 03-integrations.md | DONE |
| 6 | HTML generation | DONE |
