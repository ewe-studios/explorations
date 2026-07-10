# Ponytail Documentation

Lazy senior dev mode for AI coding agents. ~54% less code, ~20% cheaper, ~27% faster.

| Attribute | Value |
|-----------|-------|
| Source files | ~35+ |
| Language | JavaScript (Node.js), Python |
| Agents supported | 14 |
| License | MIT |
| Modes | lite, full (default), ultra, off, review |

```mermaid
flowchart TD
    Docs[ponytail docs] --> Overview[00-overview.md<br/>What it is, numbers, philosophy]
    Docs --> Arch[01-architecture.md<br/>Hooks, config, runtime, MCP, skill filtering]
    Docs --> Bench[02-benchmarks.md<br/>Agentic bench, correctness, robustness, LLM judges]
    Docs --> Integ[03-integrations.md<br/>14 agent adapters, plugins, skills, commands]

    Overview -.~54% less code.-> Bench
    Arch -.how hooks work.-> Integ
    Arch -.how instructions built.-> Bench
    Integ -.which agents supported.-> Overview

    Source[(ponytail/<br/>source codebase)] -.grandfather.-> Overview
    Source -.grandfather.-> Arch
    Source -.grandfather.-> Bench
    Source -.grandfather.-> Integ
```

## Ponytail Skills Ecosystem

Ponytail ships with 6 specialized skills, each a self-contained SKILL.md file:

```mermaid
flowchart LR
    User[User] --> Commands{slash command}
    Commands -->|/ponytail| Core[ponytail<br/>lazy senior dev mode]
    Commands -->|/ponytail-review| Review[ponytail-review<br/>diff over-engineering review]
    Commands -->|/ponytail-audit| Audit[ponytail-audit<br/>whole-repo audit]
    Commands -->|/ponytail-debt| Debt[ponytail-debt<br/>shortcut debt ledger]
    Commands -->|/ponytail-gain| Gain[ponytail-gain<br/>impact scoreboard]
    Commands -->|/ponytail-help| Help[ponytail-help<br/>quick reference]

    Core --> Ladder[5-rung ladder<br/>YAGNI to minimum code]
    Review --> Tags[delete/stdlib/native/yagni/shrink]
    Audit --> Ranked[ranked by biggest cut first]
    Debt --> Grep[grep for ponytail: comments]
    Gain --> Bench[benchmark medians]
    Help --> Card[levels + skills + commands]
```

**Aha:** Ponytail's 27-line AGENTS.md is the most powerful integration point in the entire system. While the hooks, MCP server, and plugin extensions provide richer integration for agents that support them, the AGENTS.md file works universally — any agent that reads project-level instructions gets ponytail just by this file existing at the repo root. The hooks, skills, and MCP server are amplifiers, not requirements. This is why ponytail supports 14 agents with ~35 source files: the core is 27 lines, and everything else is adapter plumbing.

## Navigation

| # | Document | Purpose |
|---|----------|---------|
| 00 | [Overview](00-overview.md) | What ponytail is, philosophy, benchmark numbers |
| 01 | [Architecture](01-architecture.md) | Hooks, config resolution, runtime state, MCP, skill filtering |
| 02 | [Benchmarks](02-benchmarks.md) | Agentic benchmarks, correctness, robustness, LLM judges |
| 03 | [Integrations](03-integrations.md) | Agent plugins, hooks, skills, commands |

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.PromptOptimizations/ponytail/`
- **Repository:** [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail)
- **Key files:** `hooks/` (5 JS files, 2 JSON configs, 2 shell scripts), `skills/` (2 SKILL.md files), `benchmarks/agentic/` (4 Python files), `ponytail-mcp/` (2 JS files), `pi-extension/` (1 JS file)

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]
python3 build.py src.PromptOptimizations
```
