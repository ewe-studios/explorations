# Durable Execution -- Index

A reader's guide to the Resonate durable-execution engine in `code/final/` — the ~5,500-line Python
runtime behind "Durable Execution in 5,000 Lines." The documents are ordered as a course: start at 00
and follow the *Continue to* footer, or jump to any piece. Each teaches its key idea in the first
paragraph, shows the real code, and names the source file it is reading.

The whole design is three claims and one rule, and every document is a consequence of them:

1. **The kernel does no I/O** — a pure function from a document, a request, and an instant to effects
   and a reply.
2. **Everything outside is two ports** — four store operations, two queue operations.
3. **Cloud Tasks pushes, so a worker is an endpoint** — no loop waits for work.

And the one rule the arrows do not show: **arm the deadline → commit → disarm the old → send**.

## The course

| # | Document | What it teaches |
|---|---|---|
| 00 | [Overview](./00-overview.md) | Durable execution, one primitive, the codebase at a glance |
| 01 | [The Promise](./01-the-promise.md) | `create`/`settle`, first-writer-wins, idempotence, memoize by position |
| 02 | [Programming Model](./02-programming-model.md) | `@resonate`, `sleep`, `external`, `Failed`, `Durable`, and what it asks of your code |
| 03 | [RPC and Gather](./03-rpc-and-gather.md) | `.rpc`, `Blocked` suspension, replay, `gather` fan-out |
| 04 | [Architecture](./04-architecture.md) | The shape, the two ports, the one rule, the three claims |
| 05 | [Types and Protocol](./05-types-and-protocol.md) | The fifteen requests, the reply, `Execute`/`Unblock`/`Timeout`, `parse_request` |
| 06 | [Kernel](./06-kernel.md) | The pure state machine, the sweep, `commit` |
| 07 | [Engine](./07-engine.md) | `Engine.process`, `encode`/`decode`/`doc_key`, the effect order, crash windows |
| 08 | [Worker](./08-worker.md) | `Worker.run`, `_attempt`, the fence, `suspend`/`fulfill`/`release` |
| 09 | [Server and Config](./09-server-and-config.md) | The one route `POST /`, auth, `config.py` env vars |
| 10 | [Spec Interfaces](./10-spec-interfaces.md) | `StoreP`/`QueueP`/`EngineP`, the P/C/M layering |
| 11 | [Port Adapters](./11-ports-adapters.md) | `store_gcp`, `queue_gcp`, generations, the 30-day horizon |
| 12 | [Simulators](./12-simulators.md) | `Clock`, `Runtime`, `Fault`, `Crash`, the memory store and queue |
| 13 | [Conformance](./13-conformance.md) | The three contracts, 11 store + 8 queue claims, `check.py` |
| 14 | [Properties](./14-properties.md) | The 43 state + 50 transition catalogue, the adapted entries |
| 15 | [Explore and Machine](./15-explore-and-machine.md) | Exhaustive search, the Hypothesis machine, the findings |
| 16 | [Examples](./16-examples.md) | The research agent and the travel agent in full |
| 17 | [Deployment](./17-deployment.md) | `serve()`, env vars, versions, the buildpack, `live/` |
| 18 | [Data Flow](./18-data-flow.md) | One run across four deliveries, the concurrency story |
| 19 | [Cross-Cutting](./19-cross-cutting.md) | The idioms, what makes it production-grade, the recipe |

## The source

Everything is checked against the source in
`/Users/darkvoid/GitSources/src.resonatehq/durable-execution/code/final/`:

```
code/final/
├── resonate/
│   ├── __init__.py       the public surface: serve, resonate, gather, sleep, external, Failed, Durable
│   ├── config.py         serve() and build(); the only module that reads environment variables
│   ├── server.py         the one route, POST /, dispatched on kind
│   ├── engine.py         Engine.process: the only code that does I/O
│   ├── kernel.py         the pure state machine
│   ├── types.py          the fifteen requests, the reply, the wire messages
│   ├── sdk.py            @resonate, Durable, .rpc, gather, sleep, external, Failed, Blocked
│   ├── worker.py         claim a task, run until it blocks or finishes
│   ├── store_gcp.py      the Cloud Storage adapter
│   ├── queue_gcp.py      the Cloud Tasks adapter
│   ├── errors.py         Conflict / Unavailable
│   ├── spec/             StoreP / QueueP / EngineP and the P/C/M layering
│   └── testing/          simulators, conformance, the catalogue, the searches
├── examples/             research-agent, travel-agent
├── live/                 latency.py, crash_recovery.sh
└── test/                 the test suite, including the Hypothesis machine
```

Start with [00-overview.md](./00-overview.md).
