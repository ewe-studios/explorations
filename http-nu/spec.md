---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.gedweb/http-nu
repository: https://github.com/cablehead/http-nu (fork: joeblew999 branch)
explored_at: 2026-05-13T17:30:00Z
---

# http-nu CF Port Exploration — Spec

## Source codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.gedweb/http-nu`
- **Branch:** `joeblew999` (CF code not on `main`)
- **Language:** Rust (wasm32-unknown-unknown target)
- **Author:** ged (fork of Paul Annesley / cablehead's http-nu)
- **License:** MIT
- **Nu version:** 0.112.1
- **Workers SDK:** `worker` crate + `cloudflare-shell` / `cloudflare-shell-workspace` crates

## What the project is

http-nu is an HTTP server scripted entirely in Nushell. The `joeblew999` branch adds a complete Cloudflare Workers port (`wasm32-unknown-unknown`) that runs the same Nushell closures inside DurableObjects, with a per-user Workspace backed by DO SQLite + R2 spillover. The port is entirely additive — all CF code lives under `src/cf/` with zero edits to upstream files.

## Documentation goal

A reader should understand:

1. The CF architecture: DurableObject per-user isolation, Workspace, SnapshotVfs
2. The Vfs abstraction that bridges desktop (std::fs) and CF (Workspace)
3. The shadow command strategy (Layer 1/2/3, demand map, 11 shadowed commands)
4. The per-request lifecycle: preload → eval → drain → persist
5. Handler hot-reload via Workspace onChange listener
6. How CF differs from desktop (no async, no reverse proxy, sleep is NO-OP, etc.)
7. The xs (cross-stream) and stor port plans

## Documentation structure

```
http-nu/
├── spec.md                         ← This file (project tracker)
├── exploration.md                  ← Main branch exploration (already written)
└── markdown/
    ├── README.md                   ← Index / table of contents
    ├── 00-cf-overview.md           ← What the CF port is, why it exists
    ├── 01-cf-architecture.md       ├── Module map, DurableObject routing, data flow
    ├── 02-vfs.md                   ├── Vfs trait + OsVfs desktop impl
    ├── 03-snapshot-vfs.md          ├── SnapshotVfs CF impl, Workspace preload/drain
    ├── 04-shadow-commands.md       ├── Shadow command strategy, all 11 commands
    ├── 05-cf-request-lifecycle.md  ├── Per-request pipeline: preload → eval → drain
    └── 06-desktop-vs-cf.md         ── Desktop vs CF comparison matrix
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | spec.md | DONE |
| 2 | markdown/README.md | DONE |
| 3 | markdown/00-cf-overview.md | DONE |
| 4 | markdown/01-cf-architecture.md | DONE |
| 5 | markdown/02-vfs.md | DONE |
| 6 | markdown/03-snapshot-vfs.md | DONE |
| 7 | markdown/04-shadow-commands.md | DONE |
| 8 | markdown/05-cf-request-lifecycle.md | DONE |
| 9 | markdown/06-desktop-vs-cf.md | DONE |
| 10 | Grandfather review | DONE |
| 11 | HTML generation | DONE |

## Build system

```bash
python3 /home/darkvoid/Boxxed/@dev/repo-expolorations/build.py http-nu
```

Shared build script at `build.py`, Python 3.12+ stdlib only.

## Quality requirements

All ten Iron Rules from the documentation directive apply: detailed sections with code snippets, mermaid diagrams (minimum 2 per document), Aha moments, grandfather review, cross-references, source path references, generated HTML, navigation.

## Expected outcome

A reader can understand the CF port architecture, trace any shadow command back to its upstream source, and know exactly what differs from desktop — without reading the source.

## Resume point

If interrupted: continue writing markdown documents in order (00 → 01 → ...). All source files have been read and analyzed.
