---
title: Herdr — Documentation Spec
---

# Herdr Documentation Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Hedr/herdr/`
- **Language:** Rust (edition 2021)
- **Version:** 0.6.8
- **License:** AGPL-3.0-or-later
- **Repository:** https://github.com/ogulcancelik/herdr
- **Rust files:** ~130 files
- **Key dependencies:** ratatui 0.30, crossterm 0.29, portable-pty 0.9, tokio 1.x, bincode 2, toml 0.8, serde 1, regex 1

## What Herdr Is

Herdr is a terminal workspace manager purpose-built for AI coding agents. It provides a ratatui TUI with workspace/tab/pane management, auto-detection of 17 AI agents (Claude Code, Codex, Cursor, Gemini, etc.) by analyzing terminal screen output and process trees, PTY isolation via portable-pty, and server/client architecture with Unix socket IPC for detach/reattach, multi-client viewing, and live terminal handoff during updates.

## Documentation Goal

A reader should be able to:
1. Understand Herdr's architecture and layer model
2. Explain how 17 AI agents are auto-detected from terminal output
3. Trace PTY lifecycle from spawn to handoff
4. Navigate the workspace/tab/pane model
5. Understand the TUI rendering pipeline
6. Configure Herdr via TOML config
7. Use all CLI commands for scripting
8. Trace data flows through the system
9. Understand error handling and recovery strategies

## Documentation Structure

```
markdown/
├── README.md                       ← Index / table of contents
├── 00-overview.md                  ← What Herdr is, architecture at a glance
├── 01-architecture.md              ← Layer map, module dependency, event loop
├── 02-agent-detection.md           ← 17 agent detectors, process ID, state model
├── 03-pty-management.md            ← PTY actor, handoff, terminal parser
├── 04-workspace-tabs-panes.md      ← Workspace model, layout, git, navigator
├── 05-tui-rendering.md             ← ratatui rendering, mobile layout, sync output
├── 06-server-client.md             ← Headless server, wire protocol, handoff, remote
├── 07-configuration.md             ← TOML config, live reload, keybindings, themes
├── 08-cli-commands.md              ← All CLI subcommands, API communication
├── 09-data-flow.md                 ← End-to-end flows with sequence diagrams
└── 10-error-handling.md            ← Panic hooks, PTY errors, config fallback, recovery
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all ~130 Rust source files | DONE |
| 2 | Read existing 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-agent-detection.md | DONE |
| 5 | Write 03-pty-management.md | DONE |
| 6 | Write 04-workspace-tabs-panes.md | DONE |
| 7 | Write 05-tui-rendering.md | DONE |
| 8 | Write 06-server-client.md | DONE |
| 9 | Write 07-configuration.md | DONE |
| 10 | Write 08-cli-commands.md | DONE |
| 11 | Write 09-data-flow.md | DONE |
| 12 | Write 10-error-handling.md | DONE |
| 13 | Update README.md | DONE |
| 14 | Create spec.md | DONE |
| 15 | Grandfather review | DONE |
| 16 | Build HTML | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]
python3 build.py src.Hedr
```

The build script (`build.py`) converts markdown to HTML, generates index pages, embeds Mermaid CDN, and handles dark/light theming. No external dependencies beyond Python 3.12+ stdlib.

## Quality Requirements

Every document must:
- Have at least 2 mermaid diagrams
- Have at least 1 Aha moment (prefixed with "Aha:" or "Key insight:")
- Include actual source code snippets with file paths and line numbers
- Cross-reference to at least 2 other documents
- End with a section pointing to the next document
- Pass grandfather review (names, numbers, flows, coverage all match source)

## Expected Outcome

A reader can understand Herdr deeply — its architecture, agent detection system, PTY management, workspace model, server/client handoff, configuration, CLI, data flows, and error handling — without reading the source code. Every claim in the documentation is verifiable against the source.

## Resume Point

All documents are written. To continue: run `python3 build.py src.Hedr` from `[src.AI]/`, verify HTML renders correctly, then commit and push.
