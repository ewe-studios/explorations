# pi_latest Documentation -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.pi-ai/pi/packages/{codemode,durable,chord}`
**Language:** TypeScript (npm workspaces, Node strip-only types)
**Packages covered:** `codemode`, `durable`, `chord`
**Package dependencies:**
- `chord` (foundation) depends on `@earendil-works/pi-ai` for TypeBox schemas.
- `durable` depends on `chord`, `pi-ai`, `pi-codemode`, `@silvia-odwyer/photon`, `diff`, and TypeBox.
- `codemode` stands alone; its only runtime dependency is `quickjs-wasi`.

## What This Documentation Is

These three packages form one durable, live-reloadable agent runtime. `chord` is the foundation: a typed host runtime where objects hold state and code at an address. `durable` builds a durable agent harness on top of chord: one session, immutable conversations, atomic commits, a task scheduler, and three storage backends. `codemode` runs user JavaScript in a sandbox inside durable tool tasks.

The goal is a plain-English, deep explanation. It uses Simplified Technical English where possible. Sentences are short. Jargon is defined before it is used.

## Documentation Goal

A reader who finishes these documents can:
1. Explain what chord, durable, and codemode each do, and why they are separate packages.
2. Trace how a user message becomes durable state, then a model call, then a written answer.
3. Describe the storage contract and the three backends (memory, JSONL, SQLite).
4. Explain the task scheduler's ownership tree, abort cascades, and wait-for graph.
5. Describe hot reload in chord and live task migration in durable.
6. See what each piece would look like in Rust, and what a production-grade version would add.
7. Build a resilient storage system like durable's, from first principles.

## Documentation Structure

```
pi_latest/
├── spec.md                          ← THIS FILE (project tracker)
├── markdown/                        ← Source documentation (viewable on GitHub)
│   ├── README.md                    ← Index / table of contents
│   ├── 00-overview.md               ← Three packages at a glance, philosophy
│   ├── 01-architecture.md           ← Dependency graph, layering, data flow
│   ├── 02-chord-concepts.md         ← Objects, contracts, references, state
│   ├── 03-chord-runtime.md          ← Host, links, connection, reload, delta
│   ├── 04-durable-concepts.md       ← Session, conversation, entry, document, task
│   ├── 05-durable-storage.md        ← Transaction + three backends, resilient storage
│   ├── 06-durable-harness.md        ← Session kernel, mutation line, failure guard
│   ├── 07-durable-tasks.md          ← Scheduler, ownership tree, abort cascades
│   ├── 08-durable-generation.md     ← Generation + compaction tasks, model loop
│   ├── 09-durable-tools-env.md      ← Tool tasks, submissions, inbox, filesystem/shell
│   ├── 10-codemode.md               ← Sandboxed JS execution in QuickJS-WASM
│   ├── 11-data-flow.md              ← End-to-end sequence diagrams
│   └── 12-cross-cutting.md          ← Rust, production-grade, resilient storage guide
└── html/                            ← Rendered HTML (build.py output)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

### Phase 1: Core Documentation (Markdown) -- COMPLETE

| # | Task | Status | File |
|---|------|--------|------|
| 1 | Overview: three packages, philosophy, at a glance | DONE | `00-overview.md` |
| 2 | Architecture: dependency graph, layering | DONE | `01-architecture.md` |
| 3 | Chord concepts: objects, contracts, references, state | DONE | `02-chord-concepts.md` |
| 4 | Chord runtime: host, links, connection, reload, delta | DONE | `03-chord-runtime.md` |
| 5 | Durable concepts: session, conversation, entry, document, task | DONE | `04-durable-concepts.md` |
| 6 | Durable storage: transaction + memory/JSONL/SQLite | DONE | `05-durable-storage.md` |
| 7 | Durable harness: Session kernel, mutation line, guard | DONE | `06-durable-harness.md` |
| 8 | Durable tasks: scheduler, ownership, abort cascades | DONE | `07-durable-tasks.md` |
| 9 | Durable generation: generation + compaction tasks | DONE | `08-durable-generation.md` |
| 10 | Durable tools/env: tool tasks, submissions, fs/shell | DONE | `09-durable-tools-env.md` |
| 11 | Codemode: sandboxed JS execution | DONE | `10-codemode.md` |
| 12 | Data flow: end-to-end sequence diagrams | DONE | `11-data-flow.md` |
| 13 | Cross-cutting: Rust, production-grade, resilient storage | DONE | `12-cross-cutting.md` |
| 14 | README index | DONE | `README.md` |

### Phase 2: HTML Rendering -- COMPLETE

| # | Task | Status | File |
|---|------|--------|------|
| 15 | Generate all HTML pages | DONE | `html/*.html` |
| 16 | Index page with navigation | DONE | `html/index.html` |
| 17 | Shared CSS | DONE | `html/styles.css` |

### Phase 3: Grandfather Review -- COMPLETE

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
python3 build.py pi_latest
```

The script discovers sorted `markdown/*.md`, builds `html/*.html` plus `index.html`, and copies `styles.css` from the parent directory on first run. Mermaid blocks become `<div class="mermaid">` rendered client-side via CDN. Code blocks become `<pre><code>`. The index page uses the `descriptions` entry for `pi_latest`.

## File Counts

| Type | Count |
|------|-------|
| Source files read (chord + durable + codemode) | 31 + 82 + 17 = 130 |
| Markdown source files | 14 |
| Generated HTML files | 15 (14 docs + 1 index) |
| CSS files | 1 (shared) |

## Expected Outcome

A reader unfamiliar with the three packages can:
1. Read the overview and understand the three packages in five minutes.
2. Read the architecture doc and understand the layering.
3. Deep-dive into durable and understand storage, scheduling, and generation.
4. Follow the sequence diagrams to understand a full request.
5. Read the cross-cutting doc to understand Rust and production-grade equivalents.

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

## Grandfather Review

The code is the grandfather. Review checks four axes:

1. Names match source: types, classes, functions, constants.
2. Numbers match source: defaults, counts, timeouts.
3. Flows match source: state transitions, call graphs.
4. Nothing missing from the public API surface.

Every name, number, and flow in this documentation was checked against the source files in `/Users/darkvoid/GitSources/src.pi-ai/pi/packages/{chord,durable,codemode}/src`.

## Resume Point

All phases complete. To add content, edit `markdown/*.md`, then run `python3 build.py pi_latest`.
