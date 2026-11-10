# Resonate Documentation -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.resonatehq/resonate`
**Commit read:** `9c72d818`
**Languages:** Rust (server core, CLI, Rust SDK), Zig (S3 server), SQL (Postgres server), Lean 4 (formal specification), TLA+ (model), Go (trace checker), TypeScript (SDK and traffic generators), Python (SDK), Go (SDK), Java (SDK)
**Server version:** `0.10.1` (`impl/server/core/Cargo.toml`)
**Organization:** ResonateHQ
**License:** Apache-2.0

## What Resonate Is

Resonate is a durable execution platform built on durable promises and durable
functions. A durable promise is a named record with a state, a parameter, a
value, and a deadline. A durable task is the executable half of that same row:
it is acquired by a worker with a lease and a version, it can be suspended while
it waits for other promises, and it is resumed when those promises settle.

The repository is organized as a specification first and implementations
second. `spec/` holds an executable abstract machine written in Lean 4, a
catalogue of 92 properties, a TLA+ model, and trace checkers. Each implementation
is held to that machine: the Rust core is compared step for step against an
executable oracle, the Postgres server is one SQL file, and the Zig server keeps
its whole durable state in an object store. There is no root-level build. Each
directory has its own README, build, and tests.

## Documentation Goal

A reader must be able to:

1. State what a durable promise, a durable task, and a schedule are, and how
   their states change.
2. Trace one function invocation from the SDK through the gateway, the server,
   the storage engine, the transport, the worker, and back.
3. Explain how Resonate defers work: how `task.suspend` registers callbacks,
   how settlement resumes a task, and how timeouts fire.
4. Read the formal specification and understand what the 92 properties claim
   and what is proved versus open.
5. Name every plugin kind, every storage backend, every transport, and every
   gateway, and say where each lives in the source.
6. Explain how the TypeScript SDK implements both execution engines and how the
   other four SDKs relate to the protocol.
7. Compare Resonate with an append-only durable stream, and design a system
   that uses both.
8. Explain how each SDK creates promise and task identifiers, and how the
   server validates them.

## Documentation Structure

```
src.resonate/
├── spec.md                          ← this file
├── markdown/
│   ├── README.md                    ← index and reading order
│   ├── 00-overview.md               ← thesis, component map, what "durable" means
│   ├── 01-architecture.md           ← three ports, plugins, composition root, lifecycle
│   ├── 02-the-protocol.md           ← envelope, 21 operations, records, status codes
│   ├── 03-durable-promises-and-tasks.md ← state machine, tags, fencing, resume
│   ├── 04-the-formal-specification.md ← Lean machine, 92 properties, TLA+, checkers
│   ├── 05-the-server-core.md        ← resonate-base, registry, config, startup order
│   ├── 06-storage-engines.md        ← SQLite, Postgres, MySQL, ScyllaDB, blob, Neo4j
│   ├── 07-transports-and-workers.md ← HTTP push, poll/SSE, Pub/Sub, bash exec
│   ├── 08-gateways-console-and-metrics.md ← HTTP edge, auth, console, Prometheus
│   ├── 09-the-typescript-sdk.md     ← generator and async engines, context, traces
│   ├── 10-the-other-sdks.md         ← Python, Go, Java, Rust
│   ├── 11-schedules-timers-and-time.md ← cron, timers, lease and retry deadlines
│   ├── 12-the-postgres-server-in-one-file.md
│   ├── 13-the-s3-server-in-zig.md
│   ├── 14-patterns.md               ← saga, fan-out, human in the loop, retries
│   ├── 15-observability-and-operations.md
│   ├── 16-critical-assessment.md
│   ├── 17-glossary.md
│   ├── 18-laws-of-durable-promises.md ← the law set, Behavior-Driven Development cases, and the PostgreSQL and SQLite tables
│   ├── 19-the-rust-sdk.md           ← the Rust software development kit and its promise naming
│   └── 20-deterministic-identifiers.md ← the shared identifier grammar, the per-SDK counters, and the server checks
└── html/                            ← generated
```

## Tasks

| Phase | Document | Status |
|-------|----------|--------|
| 1 | spec.md | DONE |
| 2 | README.md | DONE |
| 2 | 00-overview.md | DONE |
| 2 | 01-architecture.md | DONE |
| 2 | 02-the-protocol.md | DONE |
| 2 | 03-durable-promises-and-tasks.md | DONE |
| 3 | 04-the-formal-specification.md | DONE |
| 3 | 05-the-server-core.md | DONE |
| 3 | 06-storage-engines.md | DONE |
| 3 | 07-transports-and-workers.md | DONE |
| 3 | 08-gateways-console-and-metrics.md | DONE |
| 4 | 09-the-typescript-sdk.md | DONE |
| 4 | 10-the-other-sdks.md | DONE |
| 4 | 11-schedules-timers-and-time.md | DONE |
| 5 | 12-the-postgres-server-in-one-file.md | DONE |
| 5 | 13-the-s3-server-in-zig.md | DONE |
| 5 | 14-patterns.md | DONE |
| 5 | 15-observability-and-operations.md | DONE |
| 6 | 16-critical-assessment.md | DONE |
| 6 | 17-glossary.md | DONE |
| 7 | 18-laws-of-durable-promises.md | DONE |
| 7 | 19-the-rust-sdk.md | DONE |
| 7 | 20-deterministic-identifiers.md | DONE |
| 8 | Grandfather review of every document | TODO |
| 9 | Fresh-reader review of every document | TODO |
| 10 | Build and verify HTML | DONE |

## Build System

The combined build lives one level up, at
`/Users/darkvoid/Dev/explorations/src.durable-streams/build.py`.

```bash
cd /Users/darkvoid/Dev/explorations/src.durable-streams
python3 -I build.py .
```

The script builds every `src.*` subtree and the root index. It uses only the
Python standard library.

## Quality Requirements

The ten Iron Rules from `markdown_engineering/documentation_directive.md` apply:

1. Detailed sections with real source snippets.
2. Thesis first in every section.
3. Clear articulation in ASD-STE100 Simplified Technical English.
4. At least two Mermaid diagrams per document.
5. Good visual assets: tables, ASCII trees, diagrams.
6. Generated HTML with the shared build.
7. Cross-references between documents.
8. Source path references with line numbers.
9. At least one Aha moment per document.
10. Index and previous/next navigation.

Every abbreviation is spelled out at first use. Every number is checked against
the source. Every flow diagram is checked against the actual call path.

## Expected Outcome

After reading this exploration, a reader can explain Resonate end to end from
the formal specification to the Zig object-store server, can operate the server,
can write durable functions in any of the five SDKs, and can decide when to use
Resonate with an append-only durable stream instead of, or together with, it.

## Resume Point

All 21 documents and the index are written. The remaining phases are the
grandfather review against the source, the fresh-reader review, and the build.
To change a page, edit Markdown under `markdown/` and run the combined build
from `../`:

```bash
cd /Users/darkvoid/Dev/explorations/src.durable-streams
python3 -I build.py .
```

Do not edit generated HTML by hand.
