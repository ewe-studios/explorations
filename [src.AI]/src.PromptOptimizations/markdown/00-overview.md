# Ponytail: Overview

"Lazy senior dev" mode for AI coding agents. When activated, ponytail instructs agents to write minimal code — replacing 50-line implementations with one-liners where possible.

**Source:** `https://github.com/DietrichGebert/ponytail`
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PromptOptimizations/ponytail/`
**Version:** 0.1.0, MIT license

## Philosophy

> He says nothing. He writes one line. It works.

Ponytail implements the "lazy senior developer" pattern: the developer who replaces your 50-line implementation with a single `<input type="date">` because the browser already has one.

## Numbers

Measured on real Claude Code sessions editing a real open-source repo (FastAPI + React):

| Metric | Improvement |
|--------|------------|
| Code reduction | ~54% less code (up to 94%) |
| Cost | ~20% cheaper |
| Speed | ~27% faster |
| Safety | 100% — keeps all safety guards |

The 54% figure is the mean across 12 feature tasks (Haiku 4.5, n=4). It reaches 94% where an agent over-builds (a date picker) and is near zero where the code is already minimal.

## Supported Agents (14)

| Agent | Integration |
|-------|------------|
| Claude Code | Claude hooks (SessionStart, UserPromptSubmit) |
| Gemini CLI | Gemini extension (gemini-extension.json) |
| Codex CLI | Plugin data via env vars |
| Copilot CLI | Copilot plugin data via env vars |
| OpenCode | opencode.json config |
| Aider | Skills directory |
| Pi | Pi extension (pi-extension/) |
| And 7 more | MCP server, skills directory |

## The Lazy Ladder

Before writing any code, ponytail instructs the agent to stop at the first rung that holds:

1. **Does this need to be built at all?** (YAGNI)
2. **Does the standard library do this?** Use it.
3. **Does a native platform feature cover it?** Use it.
4. **Does an already-installed dependency solve it?** Use it.
5. **Can this be one line?** Make it one line.
6. **Only then:** write the minimum code that works.

## Modes

| Mode | Intensity | Description |
|------|-----------|-------------|
| `off` | Disabled | Normal agent behavior |
| `lite` | Gentle | Suggest simplifications, keep structure |
| `full` | Default | Apply lazy ladder, simplify aggressively |
| `ultra` | Maximum | One-liners only, delete over-built code |
| `review` | Independent | Review existing code for simplifications |

## Related Documents

- [Architecture](01-architecture.md) — Hooks, config resolution, runtime state
- [Benchmarks](02-benchmarks.md) — Agentic benchmarks, correctness, robustness
- [Integrations](03-integrations.md) — Agent plugins, hooks, skills

[Next → Architecture](01-architecture.md)
