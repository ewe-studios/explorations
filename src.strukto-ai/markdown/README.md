---
title: Mirage Documentation
---

# Mirage Documentation

Unified Virtual File System for AI Agents — 30+ service mounts, Python+TS SDKs, bash-level interaction.

## Documents

### Foundation

- [**00 — Overview**](00-overview.md) — What Mirage is, supported backends, SDK architecture
- [**01 — Architecture**](01-architecture.md) — Core abstractions, dependency graph, key types

### Core Systems

- [**02 — Workspace**](02-workspace.md) — Single root, heterogeneous mounts, session management
- [**03 — Resource System**](03-resource-system.md) — Resource interface, 30+ backend implementations
- [**04 — Mount System**](04-mount-system.md) — Per-mount commands, ops, consistency policies, drift
- [**05 — Shell Parser**](05-shell-parser.md) — tree-sitter bash parsing, execution trees, Python REPL
- [**06 — Ops & Commands**](06-ops-commands.md) — Operation registry, built-in commands, overrides
- [**07 — Cross-Mount**](07-cross-mount.md) — cp, mv, diff across different backends
- [**08 — Snapshot & Replay**](08-snapshot-replay.md) — Workspace serialization, drift detection
- [**09 — Caching**](09-caching.md) — File cache, index cache, Redis integration

### Runtime & Integration

- [**10 — FUSE & CLI**](10-fuse-cli.md) — FUSE mount, CLI commands, Node.js server
- [**11 — Agent Integrations**](11-agent-integrations.md) — OpenAI, LangChain, Mastra, Vercel AI
- [**12 — Python SDK**](12-python-sdk.md) — Python API, resources, FUSE, async model
- [**13 — Cross-Cutting**](13-cross-cutting.md) — Testing, examples, security, performance
