# Zero to free-code Engineer: Getting Started

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code`  
**Repository:** https://github.com/paoloanzn/free-code

This guide takes you from zero knowledge to a working understanding of free-code — a modified build of Claude Code with telemetry removed, guardrails stripped, and all experimental features unlocked.

---

## Table of Contents

1. [What is free-code?](#what-is-free-code)
2. [Key Differences from Upstream](#key-differences-from-upstream)
3. [Quick Start](#quick-start)
4. [Core Concepts](#core-concepts)
5. [Architecture Overview](#architecture-overview)
6. [Build System](#build-system)
7. [Next Steps](#next-steps)

---

## What is free-code?

**free-code** is a clean, buildable fork of Anthropic's Claude Code CLI — the terminal-native AI coding agent. It applies three categories of changes to the upstream source:

### 1. Telemetry Removed

The upstream binary "phones home" through multiple channels:
- OpenTelemetry/gRPC metrics
- GrowthBook analytics
- Sentry error reporting
- Custom event logging

In free-code, all outbound telemetry endpoints are **dead-code-eliminated or stubbed**. GrowthBook feature flag evaluation still works locally (needed for runtime feature gates) but does not report back.

### 2. Guardrails Removed

Anthropic injects system-level instructions into every conversation that constrain Claude's behavior:
- Hardcoded refusal patterns
- Injected "cyber risk" instruction blocks
- Managed-settings security overlays pushed from servers

free-code strips those injections. The model's own safety training still applies — this just removes the extra layer of prompt-level restrictions.

### 3. Experimental Features Unlocked

Claude Code ships with **88 feature flags** gated behind `bun:bundle` compile-time switches. Most are disabled in the public npm release. free-code unlocks all **54 flags that compile cleanly**.

---

## Key Differences from Upstream

| Aspect | Claude Code (Upstream) | free-code |
|--------|----------------------|-----------|
| Telemetry | Enabled | Removed |
| Feature flags | 1 default (VOICE_MODE) | 54 enabled |
| System prompts | Anthropic injections | Stripped |
| Model providers | Anthropic-focused | 5 providers |
| Distribution | npm package | Source build |

### Supported Model Providers

free-code supports **five API providers** out of the box:

| Provider | Env Variable | Auth Method |
|----------|-------------|-------------|
| Anthropic (default) | — | `ANTHROPIC_API_KEY` or OAuth |
| OpenAI Codex | `CLAUDE_CODE_USE_OPENAI=1` | OAuth via OpenAI |
| AWS Bedrock | `CLAUDE_CODE_USE_BEDROCK=1` | AWS credentials |
| Google Vertex AI | `CLAUDE_CODE_USE_VERTEX=1` | `gcloud` ADC |
| Anthropic Foundry | `CLAUDE_CODE_USE_FOUNDRY=1` | `ANTHROPIC_FOUNDRY_API_KEY` |

---

## Quick Start

### Requirements

- **Runtime**: Bun >= 1.3.11
- **OS**: macOS or Linux (Windows via WSL)
- **Auth**: API key or OAuth login for your chosen provider

```bash
# Install Bun if you don't have it
curl -fsSL https://bun.sh/install | bash
```

### Installation

```bash
# One-line install
curl -fsSL https://raw.githubusercontent.com/paoloanzn/free-code/main/install.sh | bash

# Or manual install
git clone https://github.com/paoloanzn/free-code.git
cd free-code
bun install
bun run build:dev:full
```

### First Run

```bash
# Set your API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Run the CLI
./cli

# Or run from source
bun run dev
```

### Authentication

```bash
# OAuth login (for claude.ai or OpenAI Codex)
./cli /login
```

---

## Core Concepts

### The Agent Loop

At its core, free-code implements a **request-response loop** with an LLM:

```
User Input → System Prompt + Context → LLM API → Response → Tool Execution → Repeat
```

### Tools

The AI can use various tools to interact with your system:

| Tool | Purpose |
|------|---------|
| Bash | Execute shell commands |
| FileRead | Read file contents |
| FileEdit | Edit files (multi-strategy) |
| FileWrite | Create/overwrite files |
| Glob | Find files by pattern |
| Grep | Search file contents |
| WebFetch | Fetch web pages |
| WebSearch | Search the web |
| Task* | Task management (create, stop, etc.) |
| TodoWrite | Manage todo lists |
| Agent | Spawn sub-agents |

### Permission Modes

free-code supports granular permission control:

- **Read-only** — Can only read files
- **Workspace write** — Can modify files in workspace
- **Full access** — Can run any command

### Feature Flags

Feature flags control experimental functionality at **compile-time** via `bun:bundle`:

```typescript
// Example feature-gated code
if (feature('ULTRAPLAN')) {
  // UltraPlan multi-agent planning
}

if (feature('VOICE_MODE')) {
  // Voice input/dictation
}
```

---

## Architecture Overview

### High-Level Structure

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI Entrypoint                          │
│                   (src/entrypoints/cli.tsx)                  │
├─────────────────────────────────────────────────────────────┤
│  Fast-paths: --version, --daemon, --bridge, --worktree      │
├─────────────────────────────────────────────────────────────┤
│                      Main Loop                               │
│                   (src/main.tsx)                             │
├──────────────┬────────────────┬─────────────────────────────┤
│   Commands   │     Tools      │         Services            │
│  /commands/  │    /tools/     │       /services/            │
│  80+ slash   │  40+ tools     │  API, MCP, OAuth, etc.      │
├──────────────┴────────────────┴─────────────────────────────┤
│                      State Management                        │
│              (src/state/, src/bootstrap/)                    │
├─────────────────────────────────────────────────────────────┤
│                      UI Layer                                │
│              (Ink/React terminal UI)                         │
└─────────────────────────────────────────────────────────────┘
```

### Key Directories

```
free-code/
├── scripts/
│   └── build.ts              # Build script with feature flags
├── src/
│   ├── entrypoints/
│   │   └── cli.tsx           # CLI entrypoint with fast-paths
│   ├── commands.ts           # Command registry
│   ├── tools.ts              # Tool registry
│   ├── QueryEngine.ts        # LLM query engine
│   ├── screens/
│   │   └── REPL.tsx          # Main interactive UI
│   ├── commands/             # Slash commands (/help, /login, etc.)
│   ├── tools/                # Tool implementations
│   ├── components/           # Ink/React UI components
│   ├── services/
│   │   ├── api/              # API client + adapters
│   │   ├── mcp/              # Model Context Protocol
│   │   └── oauth/            # OAuth flows
│   ├── utils/
│   │   ├── model/            # Model configs & providers
│   │   └── permissions/      # Permission system
│   └── bridge/               # Remote control / IDE bridge
└── package.json
```

---

## Build System

### Build Commands

| Command | Output | Features |
|---------|--------|----------|
| `bun run build` | `./cli` | VOICE_MODE only |
| `bun run build:dev` | `./cli-dev` | VOICE_MODE + dev stamp |
| `bun run build:dev:full` | `./cli-dev` | All 54 experimental flags |
| `bun run compile` | `./dist/cli` | VOICE_MODE only |

### Feature Flag System

The build script (`scripts/build.ts`) manages feature flags:

```typescript
const fullExperimentalFeatures = [
  'AGENT_MEMORY_SNAPSHOT',
  'AGENT_TRIGGERS',
  'BRIDGE_MODE',
  'TOKEN_BUDGET',
  'ULTRAPLAN',
  'ULTRATHINK',
  'VOICE_MODE',
  // ... 47 more
] as const
```

### Custom Builds

```bash
# Enable specific flags
bun run ./scripts/build.ts --feature=ULTRAPLAN --feature=ULTRATHINK

# Add a flag to dev build
bun run ./scripts/build.ts --dev --feature=BRIDGE_MODE
```

---

## Next Steps

After reading this guide:

1. **[Read 01-free-code-exploration.md](./01-free-code-exploration.md)** — Full architecture deep-dive
2. **[Read production-grade.md](./production-grade.md)** — Production deployment guide
3. **[Read FEATURES.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/FEATURES.md)** — Complete feature flag audit

---

## Experimental Features Highlights

The `bun run build:dev:full` build enables all 54 working feature flags:

### Interaction & UI
- `ULTRAPLAN` — Remote multi-agent planning
- `ULTRATHINK` — Deep thinking mode
- `VOICE_MODE` — Push-to-talk voice input
- `TOKEN_BUDGET` — Token usage tracking
- `HISTORY_PICKER` — Interactive history picker

### Agents & Memory
- `BUILTIN_EXPLORE_PLAN_AGENTS` — Built-in agent presets
- `VERIFICATION_AGENT` — Task validation
- `AGENT_TRIGGERS` — Cron-style automation
- `EXTRACT_MEMORIES` — Auto memory extraction

### Tools & Infrastructure
- `BRIDGE_MODE` — IDE remote-control bridge
- `BASH_CLASSIFIER` — Classifier-assisted permissions
- `MCP_RICH_OUTPUT` — Rich MCP UI rendering

See [FEATURES.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/FEATURES.md) for the complete list including 34 broken flags with reconstruction notes.
