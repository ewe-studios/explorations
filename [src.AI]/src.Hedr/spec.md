---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/
language: Rust
version: 0.6.8
explored_at: 2026-07-04
license: AGPL-3.0-or-later
repository: github.com/ogulcancelik/herdr
---

# spec.md — Herdr Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/`
- **Language:** Rust (edition 2021)
- **Version:** 0.6.8
- **LOC:** ~130 Rust source files
- **License:** AGPL-3.0-or-later

## What Herdr Is

Herdr is a terminal workspace manager purpose-built for AI coding agents. It's a Rust TUI (ratatui + crossterm) that auto-detects 20+ AI agents (Claude Code, Codex, Cursor, Gemini, Copilot, etc.) by analyzing terminal output, manages workspaces/tabs/panes with PTY isolation, and provides a server/client architecture for remote terminal handoff. The design philosophy separates pure state (`AppState`) from runtime (`PaneRuntime`) so workspace logic is testable without real PTYs.

## Architecture

1. **TUI Layer** (ratatui + crossterm) — mouse-first UI with tab bar, sidebar, status bar, dialogs
2. **Agent Detection** — 20+ detector modules that identify AI agents from terminal screen snapshots
3. **PTY Management** — portable-pty based pseudo-terminal lifecycle with async I/O
4. **Server/Client** — Unix socket IPC for remote terminal attach/detach and headless mode
5. **Workspace State** — Pure data model (tabs, panes, git status) separated from runtime
6. **Persistence** — Session snapshot and restore via bincode serialization

## Documentation Structure

```
src.Hedr/
├── spec.md
├── build.py
├── markdown/
│   ├── README.md
│   └── 00-overview.md
└── html/
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | HTML generation | DONE |
