# Inngest Core Server Deep Dive

## Overview

The main Inngest server (`inngest/`) is written in Go and provides the core orchestration engine for durable function execution.

**Location**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.inngest/inngest`

---

## Directory Structure

```
inngest/
├── cmd/                    # CLI entry points
│   ├── commands/           # CLI command implementations
│   │   ├── root.go         # Root command setup
│   │   ├── start.go        # Server start command
│   │   ├── dev.go          # Dev server command
│   │   └── version.go      # Version command
│   └── main.go             # Main entry point
├── pkg/                    # Core packages
│   ├── execution/          # Execution engine
│   │   ├── state/          # State management
│   │   │   ├── v2/         # State v2 API
│   │   │   ├── redis_state/# Redis implementation
│   │   │   └── state.go    # State interfaces
│   │   ├── queue/          # Queue implementation
│   │   │   ├── item.go     # Queue item structure
│   │   │   └── queue.go    # Queue interface
│   │   ├── debounce/       # Debounce manager
│   │   │   ├── debounce.go # Debounce logic
│   │   │   └── lua/        # Lua scripts for Redis
│   │   ├── concurrency/    # Concurrency control
│   │   ├── batch/          # Event batching
│   │   ├── ratelimit/      # Rate limiting
│   │   ├── runner/         # Event runner
│   │   ├── executor/       # Step executor
│   │   ├── history/        # Execution history
│   │   └── driver/         # Runtime drivers
│   ├── eventstream/        # Event streaming
│   ├── inngest/            # Function definitions
│   │   ├── function.go     # Function struct
│   │   ├── trigger.go      # Trigger definitions
│   │   ├── concurrency.go  # Concurrency config
│   │   └── batch.go        # Batching config
│   ├── devserver/          # Development server
│   ├── sdk/                # SDK protocol
│   ├── backoff/            # Retry backoff
│   ├── expressions/        # CEL expressions
│   └── publicerr/          # Public error types
├── ui/                     # Dashboard UI
├── proto/                  # Protocol buffers
├── tests/                  # E2E tests
└── docs/                   # Documentation
```

---

## Core Components

### 1. Execution Engine (`pkg/execution/`)

The executor is responsible for running function steps and managing state transitions.

**Key Interface** (`execution.go`):

```go
type Executor interface {
    // Schedule creates a new function run
    Schedule(ctx context.Context, r ScheduleRequest) (*sv2.Metadata, error)

    // Execute runs a function step
    Execute(ctx context.Context, id state.Identifier, item queue.Item,
            edge inngest.Edge) (*state.DriverResponse, error)

    // HandlePauses processes waitForEvent pauses
    HandlePauses(ctx context.Context, iter state.PauseIterator,
                 event event.TrackedEvent) (HandlePauseResult, error)

    // Cancel cancels an in-progress function run
    Cancel(ctx context.Context, id sv2.ID, r CancelRequest) error

    // Resume resumes from a pause
    Resume(ctx context.Context, p state.Pause, r ResumeRequest) error
}
```

**ScheduleRequest**:
```go
type ScheduleRequest struct {
    Function           inngest.Function
    At                 *time.Time      // Optional scheduled time
    AccountID          uuid.UUID
    WorkspaceID        uuid.UUID
    AppID              uuid.UUID
    Events             []event.TrackedEvent
    IdempotencyKey     *string
    PreventDebounce    bool
    FunctionPausedAt   *time.Time
}
```

### 2. State Management (`pkg/execution/state/`)

State is managed through a Redis-backed store with the following structure:

**Identifier** (unique run ID):
```go
type Identifier struct {
    RunID           ulid.ULID
    WorkflowID      uuid.UUID
    WorkflowVersion int
    EventID         ulid.ULID
    EventIDs        []ulid.ULID
    Key             string  // For idempotency
    AccountID       uuid.UUID
    WorkspaceID     uuid.UUID
    AppID           uuid.UUID
    PriorityFactor  *int64
    CustomConcurrencyKeys []CustomConcurrency
}
```

**CustomConcurrency**:
```go
type CustomConcurrency struct {
    Key   string  // Format: "$prefix:$id:$hash"
    Hash  string  // Expression hash
    Limit int     // Concurrency limit
}
```

### 3. Queue System (`pkg/execution/queue/`)

**Queue Item**:
```go
type Item struct {
    JobID                 *string
    GroupID               string
    WorkspaceID           uuid.UUID
    Kind                  string  // start, edge, sleep, pause, debounce
    Identifier            state.Identifier
    Attempt               int
    MaxAttempts           *int
    Payload               any
    Metadata              map[string]string
    QueueName             *string
    RunInfo               *RunInfo
    Throttle              *Throttle
    CustomConcurrencyKeys []state.CustomConcurrency
    PriorityFactor        *int64
}
```

**Queue Kinds**:
- `KindStart`: New function run (backlog)
- `KindEdge`: Step execution
- `KindSleep`: Scheduled wake-up
- `KindPause`: Waiting for event
- `KindDebounce`: Debounce delay
- `KindScheduleBatch`: Batch accumulation
- `KindEdgeError`: Error handling

### 4. Debounce Manager (`pkg/execution/debounce/`)

**DebounceItem**:
```go
type DebounceItem struct {
    AccountID       uuid.UUID
    WorkspaceID     uuid.UUID
    AppID           uuid.UUID
    FunctionID      uuid.UUID
    FunctionVersion int
    EventID         ulid.ULID
    Event           event.Event
    Timeout         int64  // Unix milliseconds
    FunctionPausedAt *time.Time
}
```

**Implementation**: Uses Lua scripts embedded in `lua/` directory for atomic Redis operations.

**Strategy**:
1. Create debounce key
2. Store current event in key
3. Create queue item linking to debounce key
4. Reset timer on new events
5. Execute after quiet period

### 5. Backoff Strategies (`pkg/backoff/`)

**Default Table**:
```go
var BackoffTable = []time.Duration{
    15 * time.Second,  // Attempt 0
    30 * time.Second,  // Attempt 1
    time.Minute,       // Attempt 2
    2 * time.Minute,   // Attempt 3
    5 * time.Minute,   // Attempt 4
    10 * time.Minute,  // Attempt 5
    20 * time.Minute,  // Attempt 6
    40 * time.Minute,  // Attempt 7
    time.Hour,         // Attempt 8
    2 * time.Hour,     // Attempt 9+
}
```

**Exponential Jitter**:
```go
func ExponentialJitterBackoff(attemptNum int) time.Time {
    backoff := float64(uint(1) << (uint(attemptNum) - 1))
    backoff += backoff * (0.15 * rand.Float64())
    backoff = backoff * 10
    dur := time.Second * time.Duration(backoff)
    if dur >= time.Hour*12 {
        jitter := time.Duration(rand.Int31n(120)) * time.Second
        return time.Now().Add(12 * time.Hour).Add(jitter)
    }
    return time.Now().Add(dur)
}
```

---

## Function Definition (`pkg/inngest/function.go`)

**Function Structure**:
```go
type Function struct {
    ID              uuid.UUID
    Name            string
    AppID           uuid.UUID
    AccountID       uuid.UUID
    WorkspaceID     uuid.UUID
    Triggers        MultipleTriggers
    Steps           map[string]Step
    Idempotency     *string
    Concurrency     *ConcurrencyLimits
    RateLimit       *RateLimit
    Throttle        *Throttle
    Debounce        *Debounce
    BatchEvents     *BatchEvents
    Cancel          []Cancel
    Timeouts        *Timeouts
}
```

**Step Definition**:
```go
type Step struct {
    ID      string
    Name    string
    Runtime StepRuntime
    Retries *StepRetries
}

type StepRuntime struct {
    Type    string  // "http"
    URL     string
    Timeout *string
}

type StepRetries struct {
    Attempts int
}
```

**Trigger Types**:
```go
type Trigger struct {
    EventTrigger *EventTrigger
    CronTrigger  *CronTrigger
}

type EventTrigger struct {
    Event      string
    Expression *string  // CEL expression for filtering
}

type CronTrigger struct {
    Cron string
}
```

---

## Event Processing Flow

### 1. Event Ingestion (`pkg/eventstream/eventstream.go`)

```go
func ParseStream(ctx context.Context, r io.Reader, stream chan StreamItem, maxSize int) error {
    d := json.NewDecoder(r)
    token, err := d.Token()

    switch delim := token.(type) {
    case '{':
        // Single event
        byt, _ := io.ReadAll(d.Buffered())
        extra, _ := io.ReadAll(r)
        data := append([]byte("{"), byt...)
        data = append(data, extra...)
        stream <- StreamItem{Item: data}

    case '[':
        // Event array
        for d.More() {
            var jsonEvt json.RawMessage
            d.Decode(&jsonEvt)
            stream <- StreamItem{Item: jsonEvt}
        }
    }
    return nil
}
```

### 2. Runner Processing

The Runner (`pkg/execution/runner/`) consumes events from NATS:

1. Event received from stream
2. Query functions with matching triggers
3. Evaluate expressions (CEL)
4. Check debounce configuration
5. Check rate limits
6. Create function run state
7. Enqueue in appropriate queue

### 3. Executor Step Processing

```go
func (e *executor) Execute(ctx context.Context, id state.Identifier,
                           item queue.Item, edge inngest.Edge) (*state.DriverResponse, error) {
    // 1. Check for cancellation
    if cancelled, _ := e.state.IsCancelled(ctx, id); cancelled {
        return nil, state.ErrFunctionCancelled
    }

    // 2. Get step configuration
    stepConfig := edge.GetStep()

    // 3. Call SDK endpoint
    resp, err := e.driver.Execute(ctx, edge, item)

    // 4. Handle response
    if resp.GenOpcode != nil {
        // Step yielded opcode (Sleep, WaitForEvent, etc.)
        e.state.SaveOpcode(ctx, id, *resp.GenOpcode)
    }

    // 5. Schedule child steps
    children := e.graph.Children(edge)
    for _, child := range children {
        e.queue.Enqueue(ctx, queue.Item{
            Kind: KindEdge,
            Identifier: id,
            Payload: child,
        })
    }

    return resp, err
}
```

---

## Lifecycle Hooks (`pkg/execution/lifecycle.go`)

Listeners for execution events:

```go
type LifecycleListener interface {
    OnFunctionScheduled(ctx, Metadata, Item, []event.TrackedEvent)
    OnFunctionSkipped(ctx, Metadata, SkipState)
    OnFunctionStarted(ctx, Metadata, Item, []json.RawMessage)
    OnFunctionFinished(ctx, Metadata, Item, []json.RawMessage, DriverResponse)
    OnFunctionCancelled(ctx, Metadata, CancelRequest, []json.RawMessage)
    OnStepScheduled(ctx, Metadata, Item, *string)
    OnStepStarted(ctx, Metadata, Item, Edge, string)
    OnStepFinished(ctx, Metadata, Item, Edge, *DriverResponse, error)
    OnWaitForEvent(ctx, Metadata, Item, GeneratorOpcode, Pause)
    OnWaitForEventResumed(ctx, Metadata, Pause, ResumeRequest)
    OnInvokeFunction(ctx, Metadata, Item, GeneratorOpcode, event.Event)
    OnInvokeFunctionResumed(ctx, Metadata, Pause, ResumeRequest)
}
```

---

## Dev Server (`pkg/devserver/`)

The development server provides:
- Local event ingestion
- Function registration endpoint
- Dashboard UI
- Event replay
- Step debugging

**Key endpoints**:
- `GET /`: Dashboard UI
- `POST /e/<key>`: Event ingestion
- `PUT /fn/register`: Function registration
- `POST /fn/run`: Step invocation
- `GET /fn/inspect`: SDK inspection

---

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `INNGEST_DEV` | Enable dev mode |
| `INNGEST_SIGNING_KEY` | API signing key |
| `INNGEST_EVENT_KEY` | Event ingestion key |
| `DATABASE_URL` | PostgreSQL connection |
| `REDIS_URL` | Redis connection |
| `NATS_URL` | NATS streaming URL |
| `INNGEST_ENV` | Environment name |

### Server Configuration

```go
type Config struct {
    // Server
    Port int
    Host string

    // Database
    DatabaseURL string

    // Redis
    RedisURL string

    // NATS
    NATSURL string

    // Telemetry
    TelemetryEnabled bool
}
```

---

## Testing

**E2E Test Structure** (`tests/`):
```bash
# Run E2E tests
INNGEST_SIGNING_KEY=test API_URL=http://127.0.0.1:8288 \
  SDK_URL=http://127.0.0.1:3000/api/inngest \
  go test ./tests -v -count=1

# Filter specific test
go test ./tests -v -count=1 -test.run TestSDKCancelNotReceived
```

---

## Key Design Patterns

### 1. Control Flow via Panic

Go SDK uses panic for control flow:
```go
// In step tools
func Run(ctx context.Context, id string, fn func() (any, error)) (any, error) {
    mgr := preflight(ctx)  // Checks context
    if targetID := getTargetStepID(ctx); targetID != nil {
        if *targetID == hashedID {
            // This is the target step, execute
            result, err := fn()
            // Return result to executor
        }
    }
    // Not target step, yield opcode
    mgr.Yield(Opcode{...})
    panic(ControlHijack{})  // Stop execution
}
```

### 2. Deterministic Step Hashing

```go
type Op struct {
    id  string
    pos uint64
}

func (op *Op) hash() string {
    key := fmt.Sprintf("%s:%d", op.id, op.pos)
    hasher := sha1.New()
    hasher.Write([]byte(key))
    return strings.ToUpper(hex.EncodeToString(hasher.Sum(nil)))
}
```

### 3. Atomic Redis Operations

All state mutations use Lua scripts for atomicity:
- Concurrency limit checks
- Debounce timer resets
- Queue item leasing
- Pause operations

---

## Performance Considerations

1. **Lua Scripts**: Minimize Redis roundtrips
2. **NATS Streaming**: Decouple ingestion from processing
3. **Multi-tier Queue**: Fair scheduling across tenants
4. **Connection Pooling**: Redis and database connections
5. **Lazy Loading**: Load step data on demand

---

## Security

1. **Request Signing**: HMAC-SHA256 signatures
2. **Event Key Validation**: Per-app event keys
3. **Trust Probe**: SDK-server trust verification
4. **CORS**: Configurable origins
5. **Rate Limiting**: Per-key throttling
