# Inngest Exploration

## Overview

Inngest is a developer platform for building **durable functions** - serverless event-driven workflows that replace queues, state management, and scheduling with a unified programming model. The core value proposition is enabling any developer to write reliable step functions without managing infrastructure.

**License**: Server Side Public License (SSPL) with delayed open source publication (DOSP) under Apache 2.0 for the server/CLI. SDKs are Apache 2.0 licensed.

### Key Concepts

1. **Durable Functions** - Functions that survive server restarts, crashes, and deployments by persisting state between steps
2. **Event-Driven Execution** - Functions triggered by events, cron schedules, or webhook events
3. **Step Functions** - Functions composed of atomic steps that each retry independently on failure
4. **Flow Control** - Built-in concurrency limits, throttling, debouncing, rate limiting, and batching

---

## Source Directory Structure

The source directory `/home/darkvoid/Boxxed/@formulas/src.rust/src.inngest` contains:

| Sub-project | Description |
|-------------|-------------|
| `inngest/` | Main Go server implementation - the core Inngest engine |
| `inngestgo/` | Go SDK for writing durable functions |
| `inngest-py/` | Python SDK for Django, Flask, FastAPI, etc. |
| `inngest-rs/` | Rust SDK (experimental) |
| `inngest-kt/` | Kotlin/Java SDK |
| `workflow-kit/` | React components and engine for building workflow editors |
| `event-schemas/` | Event schema definitions |
| `envelop-plugin-inngest/` | GraphQL envelop plugin |
| `dbcap/` | Database capabilities |
| `launchweek.dev/` | Launch week demo projects |
| `website/` | Documentation website |
| `inngest-squiggle-conf-workshop/` | Conference workshop materials |

---

## Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Applications                      │
│                    (TypeScript, Python, Go, Rust)                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                          Event API                               │
│              - Receives events via HTTP                          │
│              - Authenticates via Event Keys                      │
│              - Publishes to internal event stream                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Event Stream                              │
│              - NATS-based streaming buffer                       │
│              - Decouples event ingestion from processing         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                          Runner                                  │
│              - Consumes events from stream                       │
│              - Schedules function runs                           │
│              - Handles waitForEvent resumes                      │
│              - Processes cancelOn expressions                    │
│              - Writes events to history database                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Multi-Tier Queue System                         │
│              - Fair scheduling across tenants                    │
│              - Concurrency control                               │
│              - Throttling/Rate limiting                          │
│              - Debouncing                                        │
│              - Batching                                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Executor                                 │
│              - Invokes function steps                            │
│              - Manages retries with backoff                      │
│              - Writes step output to state store                 │
│              - Handles cancellations                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        State Store                               │
│              - Redis-backed state management                     │
│              - Function run state persistence                    │
│              - Step output caching                               │
│              - Pause/Resume coordination                         │
└─────────────────────────────────────────────────────────────────┘
```

### Key Packages (Go Server)

- `pkg/execution/` - Core execution engine with drivers, runners, and state management
- `pkg/execution/state/` - State store interfaces and Redis implementation
- `pkg/execution/queue/` - Multi-tier queue with GCRA rate limiting
- `pkg/execution/debounce/` - Debounce manager with Lua scripts
- `pkg/execution/concurrency/` - Concurrency limit tracking
- `pkg/eventstream/` - Event parsing and streaming
- `pkg/inngest/` - Function definitions, triggers, and configuration
- `pkg/devserver/` - Local development server
- `pkg/sdk/` - SDK protocol definitions

---

## Step Execution Model

### How Steps Work

The step execution model is the core innovation of Inngest. Here's how it works:

1. **Generator Pattern**: Functions act as generators that yield opcodes instead of executing directly
2. **Opcode Communication**: SDKs return structured opcodes (StepRun, Sleep, WaitForEvent, InvokeFunction) to the executor
3. **State Machine**: The executor maintains a state machine that tracks which steps have been completed
4. **Idempotent Execution**: Each step has a deterministic hash based on its position, allowing safe re-execution

### Step Opcodes (from `inngest-rs/inngest/src/step_tool.rs`)

```rust
enum Opcode {
    StepRun,       // Execute a user-defined function
    Sleep,         // Sleep for a duration
    WaitForEvent,  // Wait for a matching event
    InvokeFunction,// Invoke another function
}
```

### Execution Flow

1. **First Invocation**:
   - SDK receives invoke request with empty step state
   - Function executes until first `step.run()` call
   - SDK returns `Opcode::StepRun` with step ID and data
   - Executor stores the opcode, schedules step execution

2. **Step Execution**:
   - Executor calls SDK with step ID and target step
   - SDK skips to target step using context
   - Step function executes, returns result
   - Executor stores result in state

3. **Resume After Step**:
   - SDK receives invoke request with completed step state
   - Function replays, `step.run()` returns cached result
   - Function continues to next step
   - Process repeats

### Control Flow Mechanism

The Go SDK uses a clever control flow mechanism:

```go
// From inngestgo/handler.go
func invoke(...) {
    fCtx, cancel := context.WithCancel(context.Background())
    if stepID != nil {
        fCtx = step.SetTargetStepID(fCtx, *stepID)
    }

    // Execute function
    res = fVal.Call([]reflect.Value{fCtx, inputVal})

    // Check for ControlHijack panic from step tools
    defer func() {
        if r := recover(); r != nil {
            if _, ok := r.(step.ControlHijack); ok {
                return // Expected control flow
            }
        }
    }()
}
```

Step tools panic to hijack control flow after yielding an opcode, preventing further execution.

---

## Event Processing and Function Queues

### Event Ingestion Flow

1. Client sends event to Event API (`/e/<key>`)
2. Event validated against Event Key
3. Event parsed via `pkg/eventstream/ParseStream()`
4. Events published to NATS stream
5. Runner consumes events, evaluates triggers

### Queue Architecture (from `pkg/execution/queue/item.go`)

Queue items have multiple kinds:

```go
const (
    KindStart         = "start"          // New function run
    KindEdge          = "edge"           // Step execution
    KindSleep         = "sleep"          // Scheduled wake-up
    KindPause         = "pause"          // Waiting for event
    KindDebounce      = "debounce"       // Debounce delay
    KindScheduleBatch = "schedule-batch" // Batch accumulation
    KindEdgeError     = "edge-error"     // Error handling
)
```

### Queue Item Structure

```go
type Item struct {
    JobID       *string           // Internal deduplication
    GroupID     string            // Correlate step history
    WorkspaceID uuid.UUID         // Multi-tenant isolation
    Kind        string            // Job type
    Identifier  state.Identifier  // Workflow/run IDs
    Attempt     int               // Zero-indexed attempt
    MaxAttempts *int              // Retry limit
    Payload     any               // Step/edge data
    Throttle    *Throttle         // GCRA rate limiting
    CustomConcurrencyKeys []state.CustomConcurrency
    PriorityFactor *int64         // Run-level priority
}
```

### Multi-Tier Queue

The queue implements multiple tiers:
1. **Backlog Queue**: Unlimited storage for pending work
2. **Partition Queue**: Fair distribution across workspaces
3. **In-Progress Queue**: Currently executing items
4. **Rate Limited Queue**: GCRA-based throttling

---

## Durability and Retry Logic

### State Persistence

State is persisted in Redis with the following structure:

- **Run Metadata**: Account, workspace, app, function IDs
- **Step Outputs**: Hash-mapped step results by step ID
- **Event Data**: Triggering events stored inline
- **Pauses**: waitForEvent and invoke pauses

### Retry Configuration (from `pkg/backoff/backoff.go`)

Default backoff table:

```go
var BackoffTable = []time.Duration{
    15 * time.Second,   // Attempt 0
    30 * time.Second,   // Attempt 1
    time.Minute,        // Attempt 2
    2 * time.Minute,    // Attempt 3
    5 * time.Minute,    // Attempt 4
    10 * time.Minute,   // Attempt 5
    20 * time.Minute,   // Attempt 6
    40 * time.Minute,   // Attempt 7
    time.Hour,          // Attempt 8
    2 * time.Hour,      // Attempt 9+
}
```

Alternative backoff strategies:
- `ExponentialJitterBackoff`: Exponential with 15% jitter
- `GetLinearBackoffFunc`: Fixed interval between attempts

### Error Types (from Go SDK)

```go
// From inngestgo/errors/errors.go
type NoRetryError struct{ /* Permanent failure */ }
type RetryAtError struct{ RetryAt time.Time }
type StepError struct{ /* Wrapped step error */ }
```

### Step Error Handling

1. Step returns error
2. Executor stores error in state
3. Retry scheduled based on backoff
4. After max retries, function fails permanently
5. Lifecycle hooks notified

---

## Scheduling and Cron System

### Trigger Types

```rust
// From inngest-rs/inngest/src/function.rs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Trigger {
    EventTrigger {
        event: String,
        expression: Option<String>,  // CEL expression filter
    },
    CronTrigger {
        cron: String,  // Standard cron syntax
    },
}
```

### Cron Implementation

Cron triggers are evaluated by the Runner:
1. Cron expression parsed and validated
2. Scheduled jobs created in queue
3. Jobs execute at scheduled time
4. Missed executions handled based on policy

### Sleep vs Scheduling

Two mechanisms for delayed execution:

1. **`step.Sleep`**: Function-state sleep, survives restarts
   - Stored as `KindSleep` queue item
   - Uses same step position hashing

2. **Scheduled Triggers**: External cron-based
   - Creates new function runs
   - Independent of function state

---

## Flow Control Mechanisms

### Concurrency (from `pkg/inngest/concurrency.go`)

```typescript
{
  concurrency: {
    key: "event.data.userId",  // Dynamic key expression
    limit: 10,                  // Max concurrent runs per key
    scope: "fn"                 // fn | env | account
  }
}
```

Implementation:
- Keys evaluated per run
- Redis counters track active runs
- Queue items held until capacity available

### Debouncing (from `pkg/execution/debounce/debounce.go`)

```typescript
{
  debounce: {
    key: "event.data.userId",
    period: "5s",
    timeout: "1m"  // Max wait before execution
  }
}
```

Implementation:
- Lua scripts manage debounce keys in Redis
- Events update debounce payload
- Timer schedules execution after quiet period

### Rate Limiting (GCRA)

```go
type Throttle struct {
    Key    string  // Rate limit key
    Limit  int     // Requests per period
    Burst  int     // Burst capacity
    Period int     // Seconds
}
```

### Batching

```typescript
{
  batchEvents: {
    maxSize: 100,     // Max events per batch
    timeout: "5s",    // Wait time before flush
    key: "event.data.tenantId"
  }
}
```

---

## SDK Architecture

### Go SDK (`inngestgo/`)

Key components:
- `handler.go`: HTTP handler for registration and invocation
- `funcs.go`: Function definitions and triggers
- `step/`: Step tool implementations
- `signature.go`: Request signature verification

### Python SDK (`inngest-py/`)

Frameworks supported:
- Flask, FastAPI, Django, Tornado, DigitalOcean Functions

Key pattern:
```python
@inngest_client.create_function(
    fn_id="my_fn",
    trigger=inngest.TriggerEvent(event="app/event"),
)
def my_handler(ctx: inngest.Context, step: inngest.StepSync) -> dict:
    result = step.run("my_step", lambda: do_work())
    return result
```

### Rust SDK (`inngest-rs/`)

Experimental SDK with:
- Axum web framework integration
- Async/await step tools
- Type-safe function definitions

---

## Self-Hosting

The Inngest server can be self-hosted:

1. **Dev Server**: `inngest dev` - Local development with UI
2. **Production Server**: Full server with Redis/NATS dependencies
3. **Database**: PostgreSQL for state history
4. **Redis**: State store and queue backend
5. **NATS**: Event streaming buffer

Configuration via environment variables:
- `INNGEST_SIGNING_KEY`: API authentication
- `INNGEST_EVENT_KEY`: Event ingestion auth
- `DATABASE_URL`: PostgreSQL connection
- `REDIS_URL`: Redis connection
- `NATS_URL`: NATS streaming

---

## Related Projects

### Workflow Kit

React components for building workflow editors:
- Pre-built UI components
- Workflow engine for client-side execution
- Action definitions for drag-and-drop

### Event Schemas

TypeScript definitions for event validation and documentation.

---

## Testing

From `TESTING.md`:

```bash
# E2E testing with local builds
go run ./cmd/main.go dev --no-discovery
cd tests/js && yarn dev
INNGEST_SIGNING_KEY=test API_URL=http://127.0.0.1:8288 \
  SDK_URL=http://127.0.0.1:3000/api/inngest \
  go test ./tests -v -count=1
```

---

## Key Files Summary

| File | Purpose |
|------|---------|
| `inngest/handler.go` | Go SDK HTTP handler |
| `inngest/pkg/execution/execution.go` | Core executor interface |
| `inngest/pkg/execution/state/state.go` | State management |
| `inngest/pkg/execution/queue/item.go` | Queue item definition |
| `inngest/pkg/backoff/backoff.go` | Retry backoff strategies |
| `inngest-rs/inngest/src/step_tool.rs` | Rust step tool implementation |
| `inngest-rs/inngest/src/handler.rs` | Rust HTTP handler |
