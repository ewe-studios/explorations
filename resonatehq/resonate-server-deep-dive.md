# Resonate Server Deep Dive

## Overview

The Resonate Server is the central orchestrator for Resonate applications. Written in Go, it provides the durability, scheduling, and task management backbone for the Distributed Async Await programming model.

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.resonatehq/resonate/`

## Architecture

### Directory Structure

```
resonate/
├── cmd/                    # CLI entry points
│   ├── root.go            # Root command
│   ├── serve/             # Server command
│   ├── promises/          # Promise CLI
│   ├── schedules/         # Schedule CLI
│   └── tasks/             # Task CLI
├── internal/              # Internal packages
│   ├── aio/               # Async I/O layer
│   ├── api/               # HTTP/gRPC API
│   ├── app/               # Application layer
│   │   └── coroutines/    # Business logic coroutines
│   ├── kernel/            # Kernel types
│   │   ├── bus/           # Event bus
│   │   ├── system/        # System services
│   │   ├── t_aio/         # AIO types
│   │   └── t_api/         # API types
│   ├── metrics/           # Metrics collection
│   ├── util/              # Utilities
│   └── receiver/          # Message receivers
└── pkg/                   # Public packages
    ├── promise/           # Promise domain model
    ├── task/              # Task domain model
    ├── callback/          # Callback handling
    ├── schedule/          # Schedule domain
    ├── lock/              # Distributed locking
    ├── message/           # Message types
    ├── client/            # Client libraries
    └── idempotency/       # Idempotency key handling
```

## Core Components

### Promise Package (`pkg/promise/`)

**File:** `promise.go`

The Promise package defines the core data model:

```go
type Promise struct {
    Id                        string
    State                     State  // Pending, Resolved, Rejected, Canceled, Timedout
    Param                     Value  // Input parameters
    Value                     Value  // Result value
    Timeout                   int64
    IdempotencyKeyForCreate   *idempotency.Key
    IdempotencyKeyForComplete *idempotency.Key
    Tags                      map[string]string
    CreatedOn                 *int64
    CompletedOn               *int64
    SortId                    int64  // Internal ordering
}
```

**State Machine:**

```go
const (
    Pending  State = 1 << iota // 1
    Resolved                   // 2
    Rejected                   // 4
    Canceled                   // 8
    Timedout                   // 16
)

// State membership check
func (s State) In(mask State) bool {
    return s&mask != 0
}
```

### Task Package (`pkg/task/`)

Tasks represent units of work to be executed:

```go
type Task struct {
    Id            string
    ProcessId     *string       // Worker that claimed the task
    RootPromiseId string        // Root promise ID
    State         State         // Init, Claimed, Completed
    Recv          *receiver.Recv // Callback receiver
    Mesg          *message.Mesg  // Message type (Invoke, Claim, etc.)
    Timeout       int64
    Counter       int  // Retry counter
    Attempt       int  // Current attempt
    Ttl           int64 // Time to live
    ExpiresAt     int64 // Expiration time
    CreatedOn     *int64
}
```

### Coroutine System (`internal/app/coroutines/`)

The server uses coroutines for all business logic. Key coroutines:

#### CreatePromise

**File:** `createPromise.go`

Handles promise creation with optional task creation:

```go
func CreatePromise(c gocoro.Coroutine[...], r *t_api.Request) (*t_api.Response, error) {
    req := r.Payload.(*t_api.CreatePromiseRequest)
    return createPromiseAndTask(c, r, req, nil)
}

func createPromiseAndTask(...) {
    // 1. Read existing promise (check for duplicates)
    completion, err := gocoro.YieldAndAwait(c, &t_aio.Submission{
        Store: &t_aio.StoreSubmission{
            Transaction: &t_aio.Transaction{
                Commands: []t_aio.Command{
                    &t_aio.ReadPromiseCommand{Id: req.Id},
                },
            },
        },
    })

    // 2. If not exists, create it
    if result.RowsReturned == 0 {
        promiseCmd := &t_aio.CreatePromiseCommand{...}
        completion, err := gocoro.SpawnAndAwait(c, createPromise(..., promiseCmd, taskCmd))
    }

    // 3. Handle idempotency and existing promises
    // ...
}
```

#### CompletePromise

**File:** `completePromise.go`

Handles promise resolution/rejection:

```go
func CompletePromise(c gocoro.Coroutine[...], r *t_api.Request) (*t_api.Response, error) {
    req := r.Payload.(*t_api.CompletePromiseRequest)

    // 1. Read the promise
    completion, err := gocoro.YieldAndAwait(c, &t_aio.Submission{...})

    // 2. If pending, update it
    if p.State == promise.Pending {
        cmd := &t_aio.UpdatePromiseCommand{
            Id:             req.Id,
            State:          req.State,
            Value:          req.Value,
            IdempotencyKey: req.IdempotencyKey,
            CompletedOn:    c.Time(),
        }
        ok, err := gocoro.SpawnAndAwait(c, completePromise(r.Metadata, cmd))
    }

    // 3. Return appropriate response based on state
    // ...
}

func completePromise(tags map[string]string, updatePromiseCmd *t_aio.UpdatePromiseCommand, ...) {
    return func(c gocoro.Coroutine[...]) (bool, error) {
        // Execute transaction atomically:
        commands := []t_aio.Command{
            updatePromiseCmd,           // Update promise state
            &t_aio.CompleteTasksCommand{...}, // Complete associated tasks
            &t_aio.CreateTasksCommand{...},   // Create new tasks (callbacks)
            &t_aio.DeleteCallbacksCommand{...}, // Delete processed callbacks
        }
        // ...
    }
}
```

#### ClaimTask

**File:** `claimTask.go`

Handles workers claiming tasks for execution:

```go
func ClaimTask(c gocoro.Coroutine[...], r *t_api.Request) (*t_api.Response, error) {
    req := r.Payload.(*t_api.ClaimTaskRequest)

    // Search for available tasks
    completion, err := gocoro.YieldAndAwait(c, &t_aio.Submission{
        Store: &t_aio.StoreSubmission{
            Transaction: &t_aio.Transaction{
                Commands: []t_aio.Command{
                    &t_aio.SearchTasksCommand{
                        ProcessId: req.ProcessId,
                        Recv:      req.Recv,
                        Limit:     1,
                    },
                },
            },
        },
    })

    // Claim the first available task
    // ...
}
```

### Kernel Types (`internal/kernel/`)

#### AIO Types (`t_aio/`)

Defines submissions and completions for async I/O:

```go
type Submission struct {
    Kind    SubmissionKind
    Tags    map[string]string
    Store   *StoreSubmission
    Router  *RouterSubmission
    Network *NetworkSubmission
}

type StoreSubmission struct {
    Transaction *Transaction
}

type Transaction struct {
    Commands []Command
}

type Command interface {
    // ReadPromiseCommand
    // UpdatePromiseCommand
    // CreatePromiseCommand
    // CreateTaskCommand
    // CompleteTasksCommand
    // ...
}
```

#### API Types (`t_api/`)

Defines request/response types:

```go
type Request struct {
    Kind     RequestKind
    Metadata map[string]string
    Payload  interface{}
}

type Response struct {
    Status   StatusCode
    Metadata map[string]string
    Payload  interface{}
}

// Request types
type CreatePromiseRequest struct {
    Id             string
    Param          Value
    Timeout        int64
    IdempotencyKey *idempotency.Key
    Tags           map[string]string
    Strict         bool
}

type CompletePromiseRequest struct {
    Id             string
    State          promise.State
    Value          Value
    IdempotencyKey *idempotency.Key
    Strict         bool
}
```

## Storage Layer

### Store Interface

The storage layer supports ACID transactions:

```go
type Store interface {
    BeginTx(ctx context.Context, opts *sql.TxOptions) (*sql.Tx, error)
    // ...
}
```

### SQL Implementations

Resonate supports multiple SQL backends:
- SQLite (default for embedded)
- PostgreSQL (production)
- MySQL

### Read-Modify-Write Pattern

```go
// Example: CreatePromiseCommand
type CreatePromiseCommand struct {
    Id             string
    Param          Value
    Timeout        int64
    IdempotencyKey *idempotency.Key
    Tags           map[string]string
    CreatedOn      int64
}

func (c *CreatePromiseCommand) Execute(tx *sql.Tx, time int64) (Result, error) {
    query := `
        INSERT INTO promises
        (id, state, param, timeout, idempotency_key_for_create, tags, created_on)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO NOTHING
    `
    // ...
}
```

## Idempotency Handling

### Idempotency Key Package (`pkg/idempotency/`)

```go
type Key struct {
    Key string
}

func (k *Key) Match(other *Key) bool {
    if k == nil && other == nil {
        return true
    }
    if k != nil && other != nil {
        return k.Key == other.Key
    }
    return false
}
```

### Idempotency Rules

1. **Create Operations**: Deduplicated by `idempotencyKeyForCreate`
2. **Complete Operations**: Deduplicated by `idempotencyKeyForComplete`
3. **Strict Mode**: Only deduplicates if in target state
4. **Non-Strict Mode**: More permissive state transitions

## Task Management

### Task Lifecycle

1. **Init**: Task created, waiting to be claimed
2. **Claimed**: Worker has claimed the task
3. **Completed**: Task execution finished

### Heartbeat System

Workers must heartbeat to maintain task ownership:

```go
// internal/app/coroutines/heartbeatTasks.go
func HeartbeatTasks(c gocoro.Coroutine[...], r *t_api.Request) (*t_api.Response, error) {
    req := r.Payload.(*t_api.HeartbeatTasksRequest)

    // Update task heartbeat timestamp
    cmd := &t_aio.HeartbeatTasksCommand{
        ProcessId: req.ProcessId,
        TaskIds:   req.TaskIds,
    }
    // ...
}
```

### Timeout Handling

```go
// internal/app/coroutines/timeoutTasks.go
func TimeoutTasks(c gocoro.Coroutine[...]) {
    // Find expired tasks
    cmd := &t_aio.SearchTasksCommand{
        State:     []task.State{task.Claimed},
        ExpiresAt: c.Time(),
    }
    // Drop timed-out tasks for retry
    // ...
}
```

## Callback System

### Callback Registration

```go
// internal/app/coroutines/createCallback.go
func CreateCallback(c gocoro.Coroutine[...], r *t_api.Request) (*t_api.Response, error) {
    req := r.Payload.(*t_api.CreateCallbackRequest)

    // Register callback on promise
    cmd := &t_aio.CreateCallbackCommand{
        Id:        req.Id,
        PromiseId: req.PromiseId,
        Timeout:   req.Timeout,
        Recv:      req.Recv,
    }
    // ...
}
```

### Callback Receivers

Supported receiver types:

```go
// pkg/receiver/receiver.go
type Receiver struct {
    Type ReceiverType
    // Poll receiver for polling-based callbacks
    Poll *PollReceiver
    // HTTP receiver for webhook callbacks
    HTTP *HTTPReceiver
}

type PollReceiver struct {
    Group string
    Id    string
}

type HTTPReceiver struct {
    Url     string
    Headers map[string]string
}
```

## Scheduling System

### Schedule Package (`pkg/schedule/`)

```go
type Schedule struct {
    Id           string
    Description  string
    Cron         string
    Tags         map[string]string
    PromiseId    string
    PromiseTimeout int64
    PromiseParam Value
    PromiseTags  map[string]string
    LastRunTime  *int64
    NextRunTime  int64
    IdempotencyKey *idempotency.Key
    CreatedOn    *int64
}
```

### Schedule Execution

```go
// internal/app/coroutines/schedulePromises.go
func SchedulePromises(c gocoro.Coroutine[...]) {
    // Find schedules due for execution
    cmd := &t_aio.SearchSchedulesCommand{
        NextRunTime: c.Time(),
    }

    // Create promises for each schedule
    for _, schedule := range schedules {
        promiseId := generatePromiseId(schedule.PromiseId, schedule)
        createPromiseCmd := &t_aio.CreatePromiseCommand{
            Id:      promiseId,
            Timeout: schedule.PromiseTimeout,
            // ...
        }
        gocoro.SpawnAndAwait(c, createPromise(..., createPromiseCmd, nil))
    }

    // Update schedule next run time
    // ...
}
```

## CLI Commands

### Serve Command

```go
// cmd/serve/serve.go
func NewCommand() *cobra.Command {
    cmd := &cobra.Command{
        Use:   "serve",
        Short: "Start the Resonate server",
        RunE: func(cmd *cobra.Command, args []string) error {
            // Initialize stores
            // Start HTTP/gRPC servers
            // Start background workers
            // ...
        },
    }
    // Flags for configuration
    cmd.Flags().String("db", "sqlite://resonate.db", "Database URL")
    cmd.Flags().String("http-addr", ":8080", "HTTP address")
    // ...
    return cmd
}
```

## Configuration

### Environment Variables

```bash
RESONATE_DB_URL=sqlite://resonate.db
RESONATE_HTTP_ADDR=:8080
RESONATE_HEARTBEAT_INTERVAL=15s
RESONATE_POLL_FREQUENCY=5s
```

### Docker Deployment

```yaml
# docker-compose.yml
version: '3.8'
services:
  resonate:
    image: resonatehq/resonate
    environment:
      - DB_URL=postgres://user:pass@db:5432/resonate
    ports:
      - "8080:8080"
```

## Performance Considerations

1. **Connection Pooling**: SQL connection pools for concurrent access
2. **Indexing**: Indexed queries on promise ID, state, tags
3. **Pagination**: Search results paginated to prevent memory issues
4. **Batch Operations**: Transaction batching for efficiency
5. **Coroutine Concurrency**: Many coroutines can run concurrently
