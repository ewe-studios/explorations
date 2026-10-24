# Durable Execution -- Project Spec

## Source Codebase

**Location:** `/Users/darkvoid/GitSources/src.resonatehq/durable-execution`
**Language:** Python (`requires-python = ">=3.10"`, erasable/plain Python, Pydantic for types)
**Package:** `resonate` (`code/final/pyproject.toml`), version `0.1.0`
**Author:** Resonate (Dominik Tornow writes the design posts)
**License:** Apache-2.0
**Repository:** https://github.com/resonatehq/durable-execution
**Tagline:** "Durable Execution in 5,000 Lines"

**Scale:** `code/final/` is the real engine: ~5,500 lines of production code
(`resonate/` package, `examples/`, `live/`) plus ~5,200 lines of tests across
21 files in `test/`. The two design posts under `design/content/writing/` are
the thin write-up the user found lacking; the substance lives in
`code/final/`.

## What Durable Execution Is

Durable execution is the guarantee that a function can survive the process
running it: crash the machine mid-flight, redeploy, lose a region, and the
run picks up where it left off, with no retry, checkpoint, or recovery logic
in the user's code. A function that runs in milliseconds, is deterministic,
and has no side effects can simply be re-run; a function that runs for
minutes, is non-deterministic, and has side effects cannot, because the second
run pays again and may not do the same thing.

This codebase fixes that with exactly one primitive: the **durable promise**.
A promise is a call identity plus its arguments plus its outcome (or nothing
yet), kept in a database instead of a variable. It has two operations,
`create` and `settle`, both first-writer-wins and therefore idempotent.
Calls are memoized by *position* (`research.1`, `research.1:2`, `research.1:2:1`),
so replay derives the same id every time. A remote call is the same promise
with `create` here and `settle` over there; the running frame raises
`Blocked(id)` to unwind the stack, the task suspends on the promise, and a
later settle re-queues the task, which is re-run from the top.

The result is a durable-execution *runtime* — `resonate` — that runs as one
Cloud Run service over a Cloud Storage bucket and a Cloud Tasks queue. The
whole design rests on three claims, each visible in the code:

1. **The kernel does no I/O.** It is a pure function from a document, a
   request and an instant to a list of effects and a reply. It reads no clock,
   generates no id, calls nothing.
2. **Everything outside is two ports.** Four operations for a store, two for
   a queue, and that is the entire surface the engine may touch.
3. **Cloud Tasks pushes, so a worker is an endpoint.** There is no loop
   waiting for work; the queue delivers by POSTing, which is why the whole
   thing fits in a container that may not exist between two steps of the same
   run.

And one rule the arrows do not show: **arm the deadline → commit → disarm the
old → send**. Every crash window in between leaves the run recoverable, which
is the difference between durable and merely persistent.

## Documentation Goal

A reader who finishes these documents can:

1. Explain why durable execution needs exactly one primitive (the promise),
   and what `create`/`settle`/first-writer-wins buys.
2. Write a durable function with `@resonate`, `gather`, `sleep`, `external`,
   `Failed`, `Durable`, and `.rpc`, and state what durable execution asks of
   the code (determinism, idempotent side effects, serialisable values).
3. Describe the engine's shape: `config`/`Server`/`Worker`/`Engine`/`kernel`
   and the two ports, and the one rule `arm → commit → disarm → send`.
4. Trace the protocol: the fifteen requests, the reply, the queue's messages
   (`Execute`, `Unblock`, `Timeout`), `parse_request`, and the Pydantic helpers.
5. Explain the kernel as a pure state machine: the operations, the sweep's four
   passes, `commit`, `handle_external`/`handle_internal`.
6. Explain the engine: `Engine.process`, the document's `encode`/`decode`/
   `doc_key`, the conditional write, and every crash window.
7. Explain the worker: `Worker.run`, `_attempt`, the fence, `suspend`/
   `fulfill`/`release`, and `LeaseLost`.
8. Describe the two ports and their adapters: `StoreP`/`QueueP` contracts,
   `store_gcp` (generation preconditions), `queue_gcp` (the 30-day horizon),
   and `Conflict`/`Unavailable`.
9. Describe the evidence methodology: the simulators, the conformance
   catalogue (43 state + 50 transition entries), the exhaustive search, the
   Hypothesis state machine, and what the search found.
10. Trace the two examples end to end, describe deployment and versioning, and
    reconstruct the design of a durable-execution engine from first principles.

## Documentation Structure

```
durable-execution/
├── spec.md                          ← THIS FILE (project tracker)
├── markdown/                        ← Source documentation
│   ├── README.md                    ← Index / table of contents
│   ├── 00-overview.md               ← What durable execution is, one primitive, codebase at a glance
│   ├── 01-the-promise.md            ← create/settle, first-writer-wins, idempotence, memoize by position
│   ├── 02-programming-model.md      ← @resonate, sleep, external, Failed, Durable, what it asks of your code
│   ├── 03-rpc-and-gather.md         ← .rpc, Blocked suspension, replay, gather fan-out
│   ├── 04-architecture.md           ← The shape, two ports, the one rule, three claims
│   ├── 05-types-and-protocol.md     ← fifteen requests, reply, Execute/Unblock/Timeout, parse_request
│   ├── 06-kernel.md                 ← the pure state machine, sweep's four passes, commit
│   ├── 07-engine.md                 ← Engine.process, encode/decode/doc_key, effect order, crash windows
│   ├── 08-worker.md                 ← Worker.run, _attempt, the fence, suspend/fulfill/release
│   ├── 09-server-and-config.md      ← Server, POST /, auth, config.py env vars
│   ├── 10-spec-interfaces.md        ← StoreP/QueueP/EngineP, the P/C/M layering
│   ├── 11-ports-adapters.md         ← store_gcp, queue_gcp, errors, generations, 30-day horizon
│   ├── 12-simulators.md             ← Clock, Runtime, Fault, Crash, store_mem, queue_mem
│   ├── 13-conformance.md            ← the three contracts, 11 store + 8 queue claims, check.py
│   ├── 14-properties.md             ← the catalogue: 43 state + 50 transition entries, adapted entries
│   ├── 15-explore-and-machine.md    ← exhaustive search, Hypothesis machine, event/target, findings
│   ├── 16-examples.md               ← research-agent and travel-agent in full
│   ├── 17-deployment.md             ← serve(), env vars, versions, buildpack, the two guarantees, live/
│   ├── 18-data-flow.md              ← one run across four deliveries, the concurrency story
│   └── 19-cross-cutting.md          ← idioms, production-grade, building a DE engine from first principles
└── html/                            ← Rendered HTML (build.py output)
    ├── index.html
    ├── styles.css
    └── *.html
```

## Tasks

### Phase 1: Core Documentation (Markdown)

| # | Task | Status | File |
|---|------|--------|------|
| 1 | Overview: one primitive, three claims, codebase at a glance | DONE | `00-overview.md` |
| 2 | The promise: create/settle, first-writer-wins, memoize by position | DONE | `01-the-promise.md` |
| 3 | Programming model: `@resonate`, `sleep`, `external`, `Failed`, `Durable` | DONE | `02-programming-model.md` |
| 4 | `.rpc`, `Blocked` suspension, replay, `gather` fan-out | DONE | `03-rpc-and-gather.md` |
| 5 | Architecture: the shape, two ports, the one rule | DONE | `04-architecture.md` |
| 6 | Types and protocol: requests, reply, wire messages, `parse_request` | DONE | `05-types-and-protocol.md` |
| 7 | Kernel: pure state machine, sweep, commit | DONE | `06-kernel.md` |
| 8 | Engine: `Engine.process`, encode/decode, effect order | DONE | `07-engine.md` |
| 9 | Worker: `Worker.run`, `_attempt`, fence, suspend/fulfill/release | DONE | `08-worker.md` |
| 10 | Server and config: `POST /`, auth, env vars | DONE | `09-server-and-config.md` |
| 11 | Spec interfaces: `StoreP`/`QueueP`/`EngineP`, P/C/M | DONE | `10-spec-interfaces.md` |
| 12 | Port adapters: `store_gcp`, `queue_gcp`, `errors` | DONE | `11-ports-adapters.md` |
| 13 | Simulators: `Clock`, `Runtime`, `Fault`, `Crash`, mem impls | DONE | `12-simulators.md` |
| 14 | Conformance: contracts, 11 + 8 claims, `check.py` | DONE | `13-conformance.md` |
| 15 | Properties: the 43 + 50 catalogue, adapted entries | DONE | `14-properties.md` |
| 16 | Explore and machine: exhaustive search, Hypothesis, findings | DONE | `15-explore-and-machine.md` |
| 17 | Examples: research-agent, travel-agent | DONE | `16-examples.md` |
| 18 | Deployment: `serve()`, env vars, versions, buildpack, `live/` | DONE | `17-deployment.md` |
| 19 | Data flow: one run across four deliveries | DONE | `18-data-flow.md` |
| 20 | Cross-cutting: idioms, production-grade, from first principles | DONE | `19-cross-cutting.md` |
| 21 | README index | DONE | `README.md` |

### Phase 2: HTML Rendering

| # | Task | Status | File |
|---|------|--------|------|
| 22 | Generate all HTML pages | DONE | `html/*.html` |
| 23 | Index page with navigation | DONE | `html/index.html` |
| 24 | Shared CSS | DONE | `html/styles.css` |

### Phase 3: Grandfather Review

| # | Task | Status |
|---|------|--------|
| 25 | Names match source | DONE |
| 26 | Numbers match source | DONE |
| 27 | Flows match source | DONE |
| 28 | Nothing missing from public API | DONE |

## Build System

**Script:** `/Users/darkvoid/Dev/explorations/build.py`
**Dependencies:** None (Python 3 stdlib only)

**Usage:**
```bash
cd /Users/darkvoid/Dev/explorations
python3 build.py durable-execution
```

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

## Depth Requirement

~10,800 lines places durable-execution in the Large tier: 20 documents, 30,000+
words. Every public API surface, every important private helper, every constant
and default, and every error variant must be documented. No section may require
a reader to open the source to understand the behavior. The two
`design/content/writing/*.mdx` posts (001 From Ephemeral to Durable, 002 From
Blocking to Suspended) set the programming-model narrative; the documents here
go further, into the actual engine in `code/final/`.

## Grandfather Review

The code is the grandfather. Review checks four axes:

1. Names match source: functions, classes, fields, module names, table names.
2. Numbers match source: defaults, capacities, timeouts, claim counts.
3. Flows match source: state transitions, effect order, crash windows.
4. Nothing missing from the public API surface (`resonate/__init__.py`).

Every name, number, and flow in this documentation must be checked against the
source files in `/Users/darkvoid/GitSources/src.resonatehq/durable-execution/code/final/`.

## Resume Point

To resume: finish any `TODO` task above in document order, read the relevant
source files in `code/final/` in full before writing, write the markdown, run
`python3 build.py durable-execution`, and run the grandfather review (names,
numbers, flows, coverage) against the source before marking the task DONE.
