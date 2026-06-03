---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/examples
repository: git@github.com:iii-hq/examples
explored_at: 2026-06-03T00:00:00Z
language: TypeScript, Python
---

# Project Exploration: iii Examples — SDK Example Projects

## Overview

The examples repository contains **standalone iii SDK example projects** demonstrating real-world usage patterns across different stacks (TypeScript with Bun/pnpm, Python with uv). Each example is a complete, runnable project with its own `iii-config.yaml`, source code, and README.

```
examples/
├── ai-chat-agent/          # TypeScript (pnpm)
├── human-in-the-loop/      # TypeScript (Bun)
├── property-search-agent/  # Python (uv)
└── todo-app/               # TypeScript (Bun)
```

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/examples`
- **Remote:** `git@github.com:iii-hq/examples`
- **Languages:** TypeScript (Bun, pnpm), Python (uv)
- **License:** Apache-2.0 (inferred)

## Example Projects

### 1. AI Chat Agent

**Stack:** TypeScript (pnpm lockfile, Bun runtime)

AI chat application demonstrating:
- Message streams
- Durable topics
- LLM integration

```
ai-chat-agent/
├── package.json
├── iii-config.yaml
├── src/
│   └── index.ts
├── esbuild.config.ts
├── tsconfig.json
├── pnpm-lock.yaml
└── README.md
```

### 2. Human-in-the-Loop

**Stack:** TypeScript (pnpm lockfile, Bun runtime)

Order workflow demonstrating:
- Human approval gates
- Multi-step workflows
- Pause/resume patterns

```
human-in-the-loop/
├── package.json
├── iii-config.yaml
├── src/
│   └── index.ts
├── esbuild.config.ts
├── tsconfig.json
├── pnpm-lock.yaml
├── test-htl-flow.sh
└── README.md
```

### 3. Property Search Agent

**Stack:** Python (uv), Python >= 3.11

Multi-step property search agent demonstrating:
- Python SDK usage
- Search workflows
- Multi-step agent patterns

```
property-search-agent/
├── pyproject.toml          # iii-sdk==0.11.0, agno>=0.1.0, openai>=1.0.0, firecrawl-py>=0.1.0
├── iii-config.yaml
├── src/
└── README.md
```

### 4. Todo App

**Stack:** TypeScript (pnpm lockfile, Bun runtime)

Todo REST API demonstrating:
- Streams
- Durable topics
- Cron jobs

```
todo-app/
├── package.json
├── iii-config.yaml
├── src/
│   └── index.ts
├── esbuild.config.ts
├── tsconfig.json
├── pnpm-lock.yaml
├── test-todo-flow.sh
└── README.md
```

## Patterns Demonstrated

| Pattern | Examples |
|---------|----------|
| **HTTP triggers** | todo-app, ai-chat-agent |
| **Streams** | todo-app, ai-chat-agent |
| **Durable topics** | todo-app, ai-chat-agent |
| **Cron jobs** | todo-app |
| **Human approval** | human-in-the-loop |
| **Multi-step workflows** | human-in-the-loop, property-search-agent |
| **LLM integration** | ai-chat-agent, property-search-agent |

> **Note:** Pattern assignments inferred from iii-config.yaml trigger definitions in each example.

## Key Insights

1. **Each example is fully runnable.** No boilerplate — each example has its own dependencies, configuration, and source code. You can `cd` into any directory and run it.

2. **Cross-language coverage.** Examples span TypeScript (Bun, pnpm) and Python (uv), showing that the iii SDK works consistently across languages.

3. **Progressive complexity.** The examples range from simple (todo-app) to complex (human-in-the-loop with approval gates), allowing developers to learn incrementally.

## Open Questions

1. **Example completeness.** Are these examples tested against the latest iii SDK version? Do they work with iii-engine v0.18.0-next.1?

2. **Deployment instructions.** How are these examples deployed? Are they meant for local development only, or can they be deployed to production?

## Related Explorations

- [iii Engine](../iii/exploration.md) — The iii engine
- [Cookbook](../iii-cookbook/exploration.md) — Runnable recipes (larger collection)
- [Workers](../workers/exploration.md) — iii worker modules

## Next Steps

1. Deep-dive into each example's source code
2. Verify examples work against latest iii-engine version
3. Create `rust-revision.md` for Rust SDK examples
