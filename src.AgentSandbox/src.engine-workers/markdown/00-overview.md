---
title: Engine Workers — In-Process Workers Deep Dive
---

# Engine Workers — In-Process Workers Deep Dive

**The iii engine includes 7 in-process workers that provide core infrastructure — configuration store, function registry, REST API, pub/sub messaging, HTTP invocation, shell execution, and bridge client.**

## Architecture Overview

```mermaid
flowchart TB
    subgraph Config["Configuration Worker (2,693 LOC)"]
        C1["ConfigurationStore"]
        C2["Adapters: bridge, fs"]
        C3["TTL expiration"]
        C4["External change watcher"]
    end

    subgraph EngineFn["Engine Functions (2,617 LOC)"]
        E1["functions::list / info"]
        E2["triggers::list / info"]
        E3["workers::list / info"]
        E4["channels::create"]
        E5["middleware::register"]
    end

    subgraph RestAPI["REST API Worker (4,810 LOC)"]
        R1["HotRouter: dynamic routes"]
        R2["Dynamic handler generation"]
        R3["Condition functions"]
        R4["Middleware chain"]
    end

    subgraph PubSub["Pub/Sub Worker (903 LOC)"]
        P1["pubsub::publish"]
        P2["subscribe trigger"]
        P3["Adapters: local, Redis"]
    end

    subgraph HTTP["HTTP Functions (592 LOC)"]
        H1["HTTP invocation worker"]
        H2["Auth config (bearer, basic)"]
    end

    subgraph Shell["Shell Worker (973 LOC)"]
        S1["shell::exec"]
        S2["shell::exec_bg"]
        S3["Allowlist / denylist"]
        S4["Glob matching"]
    end

    subgraph Bridge["Bridge Client (541 LOC)"]
        B1["External WebSocket bridge"]
    end

    Config --> EngineFn
    EngineFn --> RestAPI
    RestAPI --> HTTP
    PubSub --> Bridge
    Shell --> EngineFn
```

## Worker Relationship Flow

```mermaid
flowchart LR
    A[REST API] -->|route matches| B[engine functions]
    C[Pub/Sub] -->|publish| D[subscribe triggers]
    E[Configuration] -->|env expansion| F[all workers]
    G[HTTP Functions] -->|external invocation| H[target service]
    I[Shell] -->|exec| J[host commands]
    K[Bridge] -->|WebSocket| L[external process]
```

**Aha:** These workers are compiled directly into the engine binary — they communicate via direct function calls, not WebSocket. This means zero serialization overhead for internal operations.

## What's Next

- [01 — Configuration](01-configuration.md) — Config store, adapters, TTL, watchers
- [02 — Engine Functions](02-engine-functions.md) — Built-in function registry
- [03 — REST API](03-rest-api.md) — Hot-reloadable routes, dynamic handlers
- [04 — Pub/Sub](04-pubsub.md) — Messaging with local and Redis adapters
- [05 — HTTP Functions](05-http-functions.md) — HTTP invocation
- [06 — Shell](06-shell.md) — Shell execution with security controls
- [07 — Bridge Client](07-bridge-client.md) — External WebSocket bridge
