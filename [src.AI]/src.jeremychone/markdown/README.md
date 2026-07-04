# src.jeremychone — The AIP Stack

Jeremy Chone's ecosystem for building AI agents that execute **programs** rather than calling **tools**. The LLM writes Lua scripts; the runtime executes them deterministically. This collapses N tool-calling round trips into 1.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.jeremychone/` — 21 projects, 250K+ LOC

## Core Stack

| # | Document | What You'll Master |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | The AIP philosophy: AI programs vs AI tools. Full system architecture |
| 01 | [aiprog](01-aiprog.md) | Lua scripting engine, proc macros, module system, type bridging |
| 02 | [aipack](02-aipack.md) | Agent orchestrator: Runtime, Hub, 28 Lua modules, agent pipeline |
| 03 | [rust-udiffx](03-udiffx.md) | Security-hardened unified diff patcher with 3-tier matching |
| 04 | [rust-genai](04-genai.md) | 19+ provider LLM client with adapter-based dispatch |

## Quick Start

```bash
# Create a new aipack project
aipack new my-agent

# Run an agent
aipack run my-agent

# Use genai directly
cargo add genai
```

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.jeremychone
python3 build.py .
```
