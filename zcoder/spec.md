# zcoder Documentation -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.zcoder/zcoder`
**Language:** Rust (edition 2024, `unsafe_code = "forbid"`)
**Version:** `0.0.4-WIP`
**Author:** Jeremy Chone (BriteSnow, Inc)
**License:** MIT OR Apache-2.0 (dual)
**Repository:** https://github.com/zcoder-run/zcoder
**Binary name:** `zc`

**Crates (workspace member order):**

| Crate | Role |
|-------|------|
| `zc-asset` | Embed and extract bundled config + prompt assets |
| `zc-core` | Domain types and contracts (no persistence, no execution) |
| `zc-common` | Event channels, time, dirs, clipboard, cache, JSON/YAML |
| `zc-tui` | Terminal UI client |
| `zc-router` | Transport + message routing over a Unix socket |
| `zc-base` | Daemon server: SQLite persistence + AI execution |

## What This Documentation Is

zcoder is an early-stage, parallel-first AI coding harness. A terminal UI client (`zc`)
talks to a background daemon (`zc base`) over a Unix domain socket. The daemon owns a
SQLite database of workspaces, runs, and AI requests, and drives model calls through the
`genai` crate. This documentation teaches the whole system end to end: the domain model,
the layered config, the persistence layer, the executor, the wire protocol, and the TUI.

The goal is a plain-English, deep explanation. It uses Simplified Technical English where
possible. Sentences are short. Jargon is defined before it is used.

## Documentation Goal

A reader who finishes these documents can:

1. Explain the client/server split and why the daemon owns the state.
2. Trace how a typed prompt becomes a `Run`, then `Air` requests, then file changes and Lua scripts.
3. Describe the three-table SQLite schema and the BMC (Base Model Controller) CRUD pattern.
4. Explain config layering, JSON merge, model alias resolution, and hot reload.
5. Describe the postcard wire framing and the attach handshake over the socket.
6. Explain the single-writer executor and the udiffx/aiprog side-effect pipeline.
7. Read the TUI event loop, debouncer, prompter state machine, and view rendering.
8. See the Rust idioms used, what a production-grade version adds, and how to build a
   resilient storage system like this from first principles.

## Documentation Structure

```
zcoder/
├── spec.md                          ← THIS FILE (project tracker)
├── markdown/                        ← Source documentation
│   ├── README.md                    ← Index / table of contents
│   ├── 00-overview.md               ← What zcoder is, philosophy, six crates at a glance
│   ├── 01-architecture.md           ← Crate dependency graph, layering, socket, event streams
│   ├── 02-domain-model.md           ← Air/Run/Wks entities, Id/EpochUs, EntityType, events
│   ├── 03-config-system.md          ← Layered TOML, merge, aliases, resolution, hot reload
│   ├── 04-persistence.md            ← SQLite schema, BMC CRUD, ModelManager, bus
│   ├── 05-executor.md               ← RunPrompt flow, air exec, udiffx, aiprog, pricing
│   ├── 06-router-transport.md       ← RouterMsg, wire framing, RouterServer/RouterClient
│   ├── 07-event-infrastructure.md   ← Channels, time, dirs, clipboard, cache, JSON
│   ├── 08-tui-core.md               ← Event loop, debouncer, state processor, prompter state
│   ├── 09-tui-rendering.md          ← View layer, tblock builder, style system
│   ├── 10-bootstrap-lifecycle.md    ← main, base command, spawner, socket lifecycle, assets
│   ├── 11-data-flow.md              ← End-to-end sequence diagrams
│   └── 12-cross-cutting.md          ← Rust idioms, production-grade, resilient storage
└── html/                            ← Rendered HTML (build.py output)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

### Phase 1: Core Documentation (Markdown)

| # | Task | Status | File |
|---|------|--------|------|
| 1 | Overview: six crates, philosophy, at a glance | DONE | `00-overview.md` |
| 2 | Architecture: dependency graph, layering, socket | DONE | `01-architecture.md` |
| 3 | Domain model: entities, scalar types, events | DONE | `02-domain-model.md` |
| 4 | Config: layering, merge, aliases, hot reload | DONE | `03-config-system.md` |
| 5 | Persistence: schema, BMC CRUD, ModelManager, bus | DONE | `04-persistence.md` |
| 6 | Executor: RunPrompt flow, air exec, udiffx, aiprog | DONE | `05-executor.md` |
| 7 | Router/transport: msg, wire, server, client | DONE | `06-router-transport.md` |
| 8 | Event infrastructure: channels, time, dirs, cache | DONE | `07-event-infrastructure.md` |
| 9 | TUI core: event loop, debouncer, prompter state | DONE | `08-tui-core.md` |
| 10 | TUI rendering: views, tblock, style | DONE | `09-tui-rendering.md` |
| 11 | Bootstrap: main, base command, spawner, assets | DONE | `10-bootstrap-lifecycle.md` |
| 12 | Data flow: end-to-end sequence diagrams | DONE | `11-data-flow.md` |
| 13 | Cross-cutting: Rust, production-grade, resilient storage | DONE | `12-cross-cutting.md` |
| 14 | README index | DONE | `README.md` |

### Phase 2: HTML Rendering

| # | Task | Status | File |
|---|------|--------|------|
| 15 | Generate all HTML pages | DONE | `html/*.html` |
| 16 | Index page with navigation | DONE | `html/index.html` |
| 17 | Shared CSS | DONE | `html/styles.css` |

### Phase 3: Grandfather Review

| # | Task | Status |
|---|------|--------|
| 18 | Names match source | DONE |
| 19 | Numbers match source | DONE |
| 20 | Flows match source | DONE |
| 21 | Nothing missing from public API | DONE |

## Build System

**Script:** `/Users/darkvoid/Dev/explorations/build.py`
**Dependencies:** None (Python 3 stdlib only)

**Usage:**
```bash
cd /Users/darkvoid/Dev/explorations
python3 build.py zcoder
```

The script discovers sorted `markdown/*.md`, builds `html/*.html` plus `index.html`, and
copies `styles.css` from the parent directory on first run. Mermaid blocks become
`<div class="mermaid">` rendered client-side via CDN. Code blocks become `<pre><code>`.
The index page uses the `descriptions` entry for the project.

## File Counts

| Type | Count |
|------|-------|
| Rust source files | 137 |
| Markdown source files | 14 |
| Generated HTML files | 15 (14 docs + 1 index) |
| CSS files | 1 (shared) |

## Expected Outcome

A reader unfamiliar with zcoder can:

1. Read the overview and understand the client/server shape in five minutes.
2. Read the architecture doc and understand the crate dependency graph.
3. Deep-dive into config, persistence, executor, and transport.
4. Follow the sequence diagrams to trace a full prompt-to-answer round trip.
5. Read the cross-cutting doc to understand Rust idioms, production hardening, and how to
   build a resilient storage system.

## Quality Requirements (Iron Rules)

1. Detailed sections with real code snippets and source paths.
2. Teach the key idea in the first paragraph, then go deeper.
3. Short, clear sentences. Mermaid diagrams for every process.
4. At least two Mermaid diagrams per document.
5. At least three code snippets per document.
6. At least one Aha moment per document.
7. At least two cross-references per document.
8. Generated HTML via the shared build.py.
9. No orphan pages.
10. Navigation: index button, prev/next buttons, theme toggle.

## Grandfather Review

The code is the grandfather. Review checks four axes:

1. Names match source: types, functions, constants, table names.
2. Numbers match source: defaults, capacities, timeouts, frame limits.
3. Flows match source: state transitions, call graphs.
4. Nothing missing from the public API surface.

Every name, number, and flow in this documentation was checked against the source files in
`/Users/darkvoid/GitSources/src.zcoder/zcoder`.

## Resume Point

To resume: finish any `TODO` task above in document order, write the markdown, run
`python3 build.py zcoder`, and run the grandfather review (names, numbers, flows, coverage)
against the source before marking the task DONE.
