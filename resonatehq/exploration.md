# ResonateHQ Exploration

## Overview

ResonateHQ is a distributed task orchestration platform implementing the **Distributed Async Await** specification. It provides reliability and scalability for distributed applications through durable functions and durable promises.

## Source Directory Structure

The ResonateHQ source at `/home/darkvoid/Boxxed/@formulas/src.rust/src.resonatehq/` contains:

### Core Projects

| Project | Description |
|---------|-------------|
| `resonate/` | The main Resonate server (Go) - orchestrator for application nodes |
| `resonate-sdk-ts/` | TypeScript SDK for building distributed applications |
| `resonate-sdk-py/` | Python SDK |
| `distributed-async-await.io/` | Specification documentation |
| `durable-promise-specification/` | Formal specification for durable promises |

### Supporting Projects

- `docs/` - Documentation site (Docusaurus)
- `resonatehq.io/` - Main website
- `examples-ts/`, `examples-py/` - Example applications
- `gocoro/` - Go coroutine library used internally
- `p-resonate-workers/` - Worker infrastructure

## Core Concepts

### Durable Promises

A **durable promise** is a promise with identity and state that persists beyond a single runtime. Unlike traditional promises bound to ephemeral memory, durable promises survive crashes and restarts.

**Promise States:**
- `PENDING` - Awaiting completion
- `RESOLVED` - Successfully completed with a value
- `REJECTED` - Failed with an error
- `REJECTED_CANCELED` - Explicitly canceled
- `REJECTED_TIMEDOUT` - Timed out

**API Operations:**
```
Create(promise-id, idempotency-key, param, header, timeout, strict)
Resolve(promise-id, idempotency-key, value, header, strict)
Reject(promise-id, idempotency-key, value, header, strict)
Cancel(promise-id, idempotency-key, value, header, strict)
Callback(id, promise-id, root-promise-id, timeout, recv)
```

### Durable Functions

Durable functions are functions with strong execution guarantees. If the runtime crashes, a durable function execution is rescheduled on a different runtime and continues from where it left off.

### Distributed Async Await Model

The programming model extends async/await with durability:

```javascript
async function foo(m) {
  let r = 0;
  for(let i = 0; i < m; i++) {
    // remotely invoke bar (creates durable promise)
    const p = async_r bar(i)
    // await durable promise
    const v = await p
    r = r + v
  }
  return r;
}
```

## Architecture

### Server Architecture (Go)

The Resonate server is built with a layered architecture:

```
resonate/
├── cmd/           # CLI commands (serve, schedules, tasks, promises)
├── internal/
│   ├── aio/       # Async I/O abstractions
│   ├── api/       # API layer
│   ├── app/
│   │   └── coroutines/  # Core business logic coroutines
│   │       ├── createPromise.go
│   │       ├── completePromise.go
│   │       ├── claimTask.go
│   │       └── ...
│   ├── kernel/
│   │   ├── t_aio/  # AIO types (Store, Router, Network submissions)
│   │   └── t_api/  # API types (Request/Response)
│   ├── util/      # Utilities
│   └── metrics/   # Observability
└── pkg/           # Public packages
    ├── promise/   # Promise data model
    ├── task/      # Task data model
    ├── callback/  # Callback handling
    ├── schedule/  # Schedule management
    ├── lock/      # Distributed locking
    └── receiver/  # Message receivers
```

### SDK Architecture (TypeScript)

```
resonate-sdk-ts/lib/
├── resonate.ts         # Main Resonate and Context classes
├── core/
│   ├── promises/       # DurablePromise implementation
│   ├── schedules/      # Schedule management
│   ├── stores/         # Local and Remote store implementations
│   ├── storages/       # Storage backends (Memory, WithTimeout)
│   ├── encoders/       # Data encoders (JSON)
│   ├── loggers/        # Logging abstractions
│   ├── retry.ts        # Retry policies (exponential, linear, never)
│   ├── options.ts      # Configuration options
│   └── utils.ts        # Utilities
└── index.ts
```

## Task Orchestration Model

### Task Lifecycle

1. **Creation**: When a durable function is invoked, a durable promise is created with `resonate:invocation=true` tag
2. **Claiming**: Workers claim tasks from the server
3. **Execution**: Function executes with retry support
4. **Completion**: Promise is resolved/rejected, triggering callbacks

### Task States

```go
// From task package
const (
    Init    State = iota // Task initialized
    Claimed              // Task claimed by worker
    Completed            // Task completed
)
```

### Coroutines for Task Management

Key coroutines in `internal/app/coroutines/`:

| Coroutine | Purpose |
|-----------|---------|
| `createPromise.go` | Creates durable promises, optionally with tasks |
| `completePromise.go` | Resolves/rejects promises, completes associated tasks |
| `claimTask.go` | Claims tasks for execution |
| `completeTask.go` | Marks tasks as completed |
| `createCallback.go` | Registers callbacks on promises |
| `claimTasks.go` | Batch task claiming |
| `enqueueTasks.go` | Enqueues tasks for execution |
| `timeoutTasks.go` | Handles task timeouts |

## Promise System & Idempotency

### Idempotency Design

Resonate uses idempotency keys to handle duplicate requests:

- **Create Idempotency Key** (`ikc`): Identifies create operations
- **Complete Idempotency Key** (`iku`): Identifies resolve/reject/cancel operations

**State Machine** (simplified):

| Current State | Action | Next State | Notes |
|--------------|--------|------------|-------|
| Init | Create(id, ikc) | Pending | First create |
| Init | Create(id, ikc) | Pending | Deduplicated (same ikc) |
| Init | Create(id, ikc*) | Error | Different ikc rejected |
| Pending | Resolve(id, iku) | Resolved | First complete |
| Pending | Resolve(id, iku) | Resolved | Deduplicated (same iku) |
| Resolved | Resolve(id, iku) | Resolved | Deduplicated |
| Resolved | Resolve(id, iku*) | Error | Different iku rejected |

### Strict Mode

- `strict=true`: Only deduplicates if in the target state
- `strict=false`: More permissive, allows state transitions

## Durability & Consistency Guarantees

### Resume Semantics

Resonate provides **resume semantics** through durable executions:

> A Durable Execution is a function execution that, if the execution is invoked, interrupted, restarted, and completes, is equivalent to some function execution that is invoked and completes.

### How Durability Works

1. **State Persistence**: After each step, execution state is persisted
2. **Recovery Path**: On restart, execution recreates state from persisted durable promises
3. **Deduplication**: Recreated executions deduplicate on existing promises

### Consistency Model

- **Single Assignment**: Durable promises can only be resolved/rejected once
- **Idempotent Operations**: All operations support deduplication via idempotency keys
- **Eventual Consistency**: Recovery paths may see slightly stale state

## Retry System

### Retry Policies

```typescript
// Exponential backoff
exponential(initialDelayMs=100, backoffFactor=2, maxAttempts=Infinity, maxDelayMs=60000)

// Linear backoff
linear(delayMs=1000, maxAttempts=Infinity)

// No retry
never()
```

### Retry Implementation

```typescript
async function runWithRetry<T>(
  func: () => Promise<T>,
  onRetry: () => Promise<void>,
  retryPolicy: RetryPolicy,
  timeout: number,
) {
  for (const delay of retryIterator(ctx)) {
    await new Promise(resolve => setTimeout(resolve, delay));
    if (ctx.attempt > 0) await onRetry();
    try {
      return await func();
    } catch (e) {
      error = e;
      ctx.attempt++;
    }
  }
  throw error;
}
```

## Storage Abstractions

### Store Interface

```typescript
interface IStore {
  readonly promises: IPromiseStore;
  readonly schedules: IScheduleStore;
  readonly locks: ILockStore;
}
```

### Storage Backends

1. **MemoryStorage**: In-memory storage for local development
2. **WithTimeout**: Wraps storage with timeout handling
3. **RemoteStore**: HTTP/gRPC backend for distributed deployments

### Read-Modify-Write Pattern

```typescript
async create(id, ikey, strict, ...) {
  return this.storage.rmw(id, (promise) => {
    if (!promise) {
      // Create new promise
      return { state: "PENDING", ... };
    }
    // Handle existing promise with idempotency
    if (strict && !isPendingPromise(promise)) {
      throw STORE_FORBIDDEN;
    }
    return promise;
  });
}
```

## Scheduling System

### Schedule Creation

```typescript
await schedules.create(
  name,           // Schedule ID
  cron,           // CRON expression
  promiseId,      // Template for promise ID (supports {{.id}}, {{.timestamp}})
  promiseTimeout, // Promise timeout
  opts            // Options (idempotencyKey, promiseParam, promiseTags)
);
```

### Schedule Execution

1. Parse CRON to compute next run time
2. At run time, create promise with templated ID
3. Update schedule with next run time
4. Repeat

## Locking System

### Distributed Locks

```typescript
interface ILockStore {
  tryAcquire(id: string, eid: string, expiry?: number): Promise<boolean>;
  release(id: string, eid: string): Promise<boolean>;
}
```

### Lock Usage

```typescript
// In _runFunc
if (opts.shouldLock) {
  while (!(await acquireLock(id, eid, locksStore))) {
    await sleep(opts.pollFrequency);
  }
}
// ... execute function ...
// Finally: release lock
```

## Key Design Patterns

### 1. Coroutine-Based Architecture

The Go server uses the `gocoro` library for async operations:

```go
func CreatePromise(c gocoro.Coroutine[...], r *t_api.Request) (*t_api.Response, error) {
    // Yield to read promise from store
    completion, err := gocoro.YieldAndAwait(c, &t_aio.Submission{...})

    // Spawn sub-coroutines
    ok, err := gocoro.SpawnAndAwait(c, completePromise(...))
}
```

### 2. Command Pattern for Store Operations

```go
Transaction{
    Commands: []t_aio.Command{
        &t_aio.ReadPromiseCommand{Id: req.Id},
        &t_aio.UpdatePromiseCommand{...},
        &t_aio.CreateTasksCommand{...},
        &t_aio.DeleteCallbacksCommand{...},
    }
}
```

### 3. Tag-Based Routing

```go
// Router matches promises to receivers based on tags
completion, err := gocoro.YieldAndAwait(c, &t_aio.Submission{
    Kind: t_aio.Router,
    Router: &t_aio.RouterSubmission{
        Promise: &promise.Promise{...},
    },
})
```

## Performance Characteristics

### Design Choices

1. **Polling-Based**: Clients poll for task completion (configurable frequency, default 5s)
2. **Heartbeat System**: Workers heartbeat to maintain task ownership
3. **Batch Operations**: Search returns paginated results
4. **Lazy Loading**: Promises loaded on-demand, not eagerly

### Scaling Considerations

- **Horizontal Scaling**: Multiple workers can claim tasks
- **Lock Contention**: Optional locking for exclusive execution
- **Callback System**: HTTP/poll callbacks for event-driven patterns
