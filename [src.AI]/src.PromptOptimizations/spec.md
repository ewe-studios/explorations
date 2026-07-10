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
- **Source scale:** ~35 source files (hooks, scripts, tests, benchmarks, MCP server, skills, extensions)

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
│   ├── 01-architecture.md   ← Hooks, config resolution, runtime state, MCP, skill filtering
│   ├── 02-benchmarks.md     ← Agentic benchmarks, correctness, robustness, LLM judges
│   └── 03-integrations.md   ← 14 agent adapters, hook protocol, MCP, skills, commands
└── html/                    ← Generated HTML
```

## Tasks

| # | Document | Status | Notes |
|---|----------|--------|-------|
| 1 | README.md | DONE | 2 mermaid diagrams, 2 Aha moments, source references |
| 2 | 00-overview.md | DONE | 3+ mermaid diagrams, 4+ Aha moments, source snippets |
| 3 | 01-architecture.md | DONE | 3+ mermaid diagrams, 5+ Aha moments, 15+ source snippets |
| 4 | 02-benchmarks.md | DONE | 3 mermaid diagrams, 5+ Aha moments, 12+ source snippets, full task coverage |
| 5 | 03-integrations.md | DONE | 3 mermaid diagrams, 5+ Aha moments, 12+ source snippets, command .toml coverage |
| 6 | HTML generation | DONE | Ran build.py successfully |
| 7 | Grandfather review | DONE | Names, numbers, flows, coverage verified against source |

## Source Codebase Deep Dive

- **Hooks:** 5 JS files (`ponytail-activate.js` 91L, `ponytail-mode-tracker.js` 55L, `ponytail-config.js` 122L, `ponytail-instructions.js` 92L, `ponytail-runtime.js` 51L) + 2 shell scripts + 2 JSON configs
- **Skills:** 6 SKILL.md files (`ponytail` 101L, `ponytail-review` 57L, `ponytail-audit`, `ponytail-debt`, `ponytail-gain`, `ponytail-help`)
- **Commands:** 6 `.toml` files mapping slash commands to skill prompts
- **MCP:** 2 JS files (`ponytail-mcp/index.js` 48L, `ponytail-mcp/instructions.js` 26L) using `McpServer` + `StdioServerTransport`
- **Benchmarks:** 5 Python files in `agentic/` (run.py 409L, tasks.py 570L, judge.py 185L, complete.py 154L, benchmark-local.py 155L) + 11 standalone JS/JSON files
- **Extensions:** `pi-extension/` (2 files), `gemini-extension.json`, `opencode.json`
- **Tests:** 9 test files covering hooks, commands, correctness, behavior, and plugin systems
- **Universal adapter:** `AGENTS.md` — 27 lines, works for any agent reading project files

## Grandfather Review Summary

### Names — All Match
- Hook files: `ponytail-activate.js`, `ponytail-mode-tracker.js`, `ponytail-config.js`, `ponytail-instructions.js`, `ponytail-runtime.js`, `ponytail-statusline.sh/ps1` — all verified in `hooks/`
- MCP files: `index.js`, `instructions.js` — verified in `ponytail-mcp/`
- Skills: `skills/ponytail/SKILL.md`, `skills/ponytail-review/SKILL.md` — verified
- Config functions: `getDefaultMode()`, `getConfigDir()`, `getConfigPath()`, `getClaudeDir()`, `isShellSafe()`, `isDeactivationCommand()`, `normalizeMode()`, `normalizePersistedMode()`, `writeDefaultMode()` — all verified in `ponytail-config.js`
- Runtime functions: `setMode()`, `clearMode()`, `writeHookOutput()`, `isCodex`, `isCopilot` — verified in `ponytail-runtime.js`
- Instruction functions: `filterSkillBodyForMode()`, `getPonytailInstructions()`, `getFallbackInstructions()` — verified in `ponytail-instructions.js`
- Benchmark functions: `selftest()`, `run_cell()`, `code_stats()`, `git_diff_stats()`, `aggregate()`, `score_workspace()` — verified in `benchmarks/agentic/run.py`

### Numbers — All Match
- Valid modes: `['off', 'lite', 'full', 'ultra', 'review']` (5 modes) — matches `VALID_MODES` in `ponytail-config.js:17`
- Runtime modes: `['off', 'lite', 'full', 'ultra']` (4 modes, excludes `review`) — matches `RUNTIME_MODES` in `ponytail-config.js:18`
- Default mode: `'full'` — matches `DEFAULT_MODE` in `ponytail-config.js:16`
- Cell timeout: 300 seconds — matches `CELL_TIMEOUT` in `run.py:63`
- Code extensions: 12 extensions — matches `CODE_EXT` in `run.py:79`
- Agents supported: 14 — matches README.md and integration docs
- Source files: ~35+ — matches actual file count

### Flows — All Match
- Hook execution flow: SessionStart → config resolution → instruction building → output emission — matches source call chain
- Config resolution: env → config file → default — matches `getDefaultMode()` in `ponytail-config.js:76-96`
- Agent-specific output: Copilot (`additionalContext`) vs Codex (`systemMessage`) vs Claude (raw text) — matches `writeHookOutput()` in `ponytail-runtime.js:24-43`
- Skill filtering: strip frontmatter → filter table rows → filter examples → keep general rules — matches `filterSkillBodyForMode()` in `ponytail-instructions.js:11-36`
- Benchmark pipeline: selftest → seed → arm → claude CLI → git diff → scoring → aggregation — matches `run.py`

### Coverage — Complete
- All 5 hook files documented
- All 2 MCP files documented
- Both SKILL.md files documented
- All 3 agent-specific integrations (Claude, Codex, Copilot) documented
- All 14 agents listed with their integration mechanism
- Config resolution fully documented with cross-platform details
- Benchmark pipeline, selftest, tasks, and scoring fully documented
- Statusline integration documented
