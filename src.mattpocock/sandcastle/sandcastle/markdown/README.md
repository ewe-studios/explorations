# Sandcastle — Documentation Index

**@ai-hero/sandcastle** v0.12.0 — CLI for orchestrating AI agents in isolated sandbox environments

## Foundation

| Document | Covers |
|----------|--------|
| [00-overview.md](./00-overview.md) | What Sandcastle is, why it exists, key concepts at a glance |
| [01-architecture.md](./01-architecture.md) | Full dependency graph, module map, layer architecture, Effect integration |

## Deep Dives

| Document | Covers |
|----------|--------|
| [02-sandbox-providers.md](./02-sandbox-providers.md) | Provider contract, Docker/Podman/Vercel implementations, branch strategies, custom providers |
| [03-agent-integration.md](./03-agent-integration.md) | Agent providers (Claude/Codex/Pi/Cursor/OpenCode/Copilot), stream parsing, session capture/resume/fork, structured output |
| [04-orchestration-workflow.md](./04-orchestration-workflow.md) | Orchestrator, SandboxFactory, worktree management, hooks, sync-in/sync-out |
| [05-prompt-system.md](./05-prompt-system.md) | Prompt resolution, `{{KEY}}` substitution, `` !`command` `` expansion, built-in arguments |

## Cross-Cutting

| Document | Covers |
|----------|--------|
| [06-cli-interface.md](./06-cli-interface.md) | CLI commands, `sandcastle init`, Docker/Podman image management, Display system |
| [07-utilities.md](./07-utilities.md) | BoundedTail, TextDeltaBuffer, shutdownRegistry, errors, session transfer, templates |
| [08-cross-cutting.md](./08-cross-cutting.md) | Error handling patterns, AbortSignal, shutdown lifecycle, Windows handling, testing, CI/CD |

## Quick Reference

### Entry Points
- `run()` — one-shot agent invocation
- `createSandbox()` — reusable sandbox for multiple runs
- `createWorktree()` — worktree as first-class concept
- `interactive()` — interactive TUI agent session

### Agent Providers
- `claudeCode(model, opts)` — Claude Code with resume/fork
- `codex(model, opts)` — OpenAI Codex with resume/fork
- `pi(model, opts)` — Pi agent with resume
- `cursor(model)` — Cursor (no resume)
- `opencode(model)` — OpenCode (no resume)
- `copilot(model)` — GitHub Copilot CLI (no resume)

### Sandbox Providers
- `docker(opts)` — Docker container with bind-mount
- `podman(opts)` — Podman container with bind-mount
- `vercel(opts)` — Vercel Firecracker microVM (isolated)
- `noSandbox()` — Run directly on host

### Branch Strategies
- `head` — Direct write to host working directory (bind-mount default)
- `merge-to-head` — Temp branch, merged back to HEAD (isolated default)
- `branch` — Explicit named branch, worktree reused

---

Source: [github.com/mattpocock/sandcastle](https://github.com/mattpocock/sandcastle)
