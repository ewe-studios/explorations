# pi_latest -- Documentation Index

Three TypeScript packages form one durable agent runtime. `chord` is the foundation: a typed host runtime. `durable` builds a durable agent harness on chord. `codemode` runs user JavaScript in a sandbox.

## Documents

### Foundation

| Document | What It Covers |
|----------|---------------|
| [00-overview.md](./00-overview.md) | The three packages, the philosophy, and what each does at a glance |
| [01-architecture.md](./01-architecture.md) | Dependency graph, layering, and how the packages compose |

### Chord Deep Dives

| Document | What It Covers |
|----------|---------------|
| [02-chord-concepts.md](./02-chord-concepts.md) | Objects, contracts, references, principals, observable state |
| [03-chord-runtime.md](./03-chord-runtime.md) | Host, links, connection flow control, hot reload, delta tracker, context |

### Durable Deep Dives

| Document | What It Covers |
|----------|---------------|
| [04-durable-concepts.md](./04-durable-concepts.md) | Session, conversation, entry, document, task, submission |
| [05-durable-storage.md](./05-durable-storage.md) | Transaction, and the memory / JSONL / SQLite backends |
| [06-durable-harness.md](./06-durable-harness.md) | Session kernel, mutation line, failure guard, document cache |
| [07-durable-tasks.md](./07-durable-tasks.md) | Scheduler, ownership tree, abort cascades, wait-for graph |
| [08-durable-generation.md](./08-durable-generation.md) | Generation and compaction tasks, the model call loop |
| [09-durable-tools-env.md](./09-durable-tools-env.md) | Tool tasks, submissions, inbox, filesystem and shell |

### Codemode

| Document | What It Covers |
|----------|---------------|
| [10-codemode.md](./10-codemode.md) | Sandboxed JS execution in QuickJS-WASM |

### Cross-Cutting

| Document | What It Covers |
|----------|---------------|
| [11-data-flow.md](./11-data-flow.md) | End-to-end sequence diagrams |
| [12-cross-cutting.md](./12-cross-cutting.md) | Rust, production-grade, and a resilient-storage guide |

## Quick Orientation

```
chord            ← typed host runtime (foundation)
   ↓
durable          ← durable agent harness (session, storage, scheduler)
   ↓
codemode         ← sandboxed JS execution (used by durable tool tasks)
```

`chord` gives objects an address and a contract. `durable` makes every change a committed, recoverable record. `codemode` runs untrusted code safely inside that record-keeping system.

## Source

`/Users/darkvoid/GitSources/src.pi-ai/pi/packages/{codemode,durable,chord}/`
