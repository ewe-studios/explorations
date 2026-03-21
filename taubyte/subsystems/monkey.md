# Monkey Service - Function Execution Deep Dive

## Overview

**Monkey** is Taubyte's function execution service responsible for compiling, deploying, and executing serverless functions. It supports multiple languages (Go, Rust, Zig, AssemblyScript) compiled to WebAssembly.

---

## Service Architecture

### Core Components

```
tau/services/monkey/
├── service.go           # Main service implementation
├── type.go              # Service type definitions
├── monkey.go            # Function runtime management
├── job.go               # Job handling
├── pubsub.go            # Pub/sub integration
├── stream.go            # P2P stream handling
├── api_monkeys.go       # HTTP API endpoints
├── helpers.go           # Utility functions
├── common/
│   └── iface.go         # Interface definitions
├── dream/
│   └── init.go          # Dream integration
├── fixtures/
│   └── compile/         # Test fixtures and compilers
├── jobs/
│   └── [job handlers]
└── tests/
    └── [integration tests]
```

### Service Structure

```go
// tau/services/monkey/type.go
type Service struct {
    ctx           context.Context
    node          peer.Node
    clientNode    peer.Node
    config        *tauConfig.Node
    dev           bool
    stream        *streams.Service
    patrickClient *patrick.Client
    tnsClient     *tns.Client
    hoarderClient *hoarder.Client
    monkeys       map[string]*Monkey
    recvJobs      map[string]time.Time
    dvPublicKey   crypto.PublicKey
}

type Monkey struct {
    ID       string
    Runtime  *vm.Runtime
    Config   *FunctionConfig
    Status   MonkeyStatus
}
```

---

## Service Initialization

```go
// tau/services/monkey/service.go
func New(ctx context.Context, config *tauConfig.Node) (*Service, error) {
    srv := &Service{
        ctx:    ctx,
        dev:    config.DevMode,
        config: config,
    }

    // Start container garbage collection
    err := ci.Start(ctx, ci.DefaultInterval, ci.DefaultMaxAge)
    if err != nil {
        return nil, err
    }

    // Initialize P2P node
    if config.Node == nil {
        srv.node, err = tauConfig.NewLiteNode(ctx, config,
            path.Join(config.Root, protocolCommon.Monkey))
    } else {
        srv.node = config.Node
    }

    // Subscribe to Patrick job notifications
    err = srv.subscribe()
    if err != nil {
        return nil, err
    }

    // Setup P2P stream
    srv.stream, err = streams.New(srv.node, protocolCommon.Monkey,
        protocolCommon.MonkeyProtocol)
    srv.setupStreamRoutes()
    srv.stream.Start()

    // Initialize clients
    srv.patrickClient, err = NewPatrick(ctx, srv.clientNode)
    srv.tnsClient, err = tnsClient.New(ctx, srv.clientNode)
    srv.hoarderClient, err = hoarder.New(ctx, srv.clientNode)

    return srv, nil
}
```

---

## Function Compilation Pipeline

### Compilation Flow

```
┌─────────────────────────────────────────────────────────────┐
│                  COMPILATION PIPELINE                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Source Fetch                                            │
│     ┌──────────┐                                           │
│     │  Hoarder │ ← Source code from Git                    │
│     └────┬─────┘                                           │
│          │                                                  │
│  2. Language Detection                                      │
│     ┌──────────┐                                           │
│     │  go.mod  │ → Go                                      │
│     │Cargo.toml│ → Rust                                    │
│     │ build.zig│ → Zig                                     │
│     │  asconfig│ → AssemblyScript                          │
│     └────┬─────┘                                           │
│          │                                                  │
│  3. Compilation                                             │
│     ┌──────────────────────────────────────────┐           │
│     │  TinyGo  │ rustc │ zig build │ asc       │           │
│     └──────────┴───────┴───────────┴───────────┘           │
│          │                                                  │
│  4. WASM Optimization                                       │
│     ┌──────────┐                                           │
│     │ wasm-opt │ → Strip debug, minimize                  │
│     └────┬─────┘                                           │
│          │                                                  │
│  5. Artifact Storage                                        │
│     ┌──────────┐                                           │
│     │ Hoarder  │ → Store WASM with CID                    │
│     └──────────┘                                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### SmartOps Builder

```go
// tau/services/monkey/fixtures/compile/smartops.go
type SmartOpsBuilder struct {
    ID         string
    Language   string
    Source     string
    Wasm       []byte
    WasmCID    string
}

func (b *SmartOpsBuilder) Build(ctx context.Context) error {
    switch b.Language {
    case "go":
        return b.buildGo()
    case "rust":
        return b.buildRust()
    case "zig":
        return b.buildZig()
    case "assemblyscript":
        return b.buildAssemblyScript()
    }
}

func (b *SmartOpsBuilder) buildGo() error {
    // Write source to temp directory
    // Run: tinygo build -o output.wasm -target wasm main.go
    // Read WASM binary
    // Upload to Hoarder
}
```

### Rust Compilation

```go
// tau/services/monkey/fixtures/compile/function_rs_test.go
func compileRust(source string) ([]byte, error) {
    // Create Cargo project
    cargoToml := `[package]
name = "function"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
taubyte-sdk = "0.1.6"
`
    // Write Cargo.toml and lib.rs
    // Run: cargo build --target wasm32-unknown-unknown --release
    // Read: target/wasm32-unknown-unknown/release/function.wasm
    // Optimize with wasm-opt
}
```

### Compilation Fixtures

```
tau/services/monkey/fixtures/compile/assets/
├── Cargo.toml          # Rust test project
├── lib.rs              # Rust function source
├── helloWorld.ts       # AssemblyScript source
├── ping.go             # Go function source
├── ping.zwasm          # Pre-compiled Zwasm
├── release.wasm        # Pre-compiled WASM
├── library/            # Library functions
│   ├── ping1.go
│   ├── ping2.go
│   └── ping3.go
└── website/            # Website test files
    └── index.html
```

---

## Function Execution

### WASM Runtime

```go
// tau/services/monkey/monkey.go
type Monkey struct {
    ID          string
    Runtime     *vm.Runtime
    Config      *FunctionConfig
    LastUsed    time.Time
    RequestCount int64
}

func (m *Monkey) Execute(request *Request) (*Response, error) {
    // Check if runtime needs initialization
    if m.Runtime == nil {
        err := m.initializeRuntime()
        if err != nil {
            return nil, err
        }
    }

    // Execute WASM handler
    return m.Runtime.Call("handle", request)
}

func (m *Monkey) initializeRuntime() error {
    // Fetch WASM from Hoarder
    wasm, err := m.hoarder.Get(m.Config.WasmCID)

    // Create VM instance
    m.Runtime, err = vm.New(wasm, vm.Config{
        MemoryLimit: m.Config.MemoryLimit,
        Timeout:     m.Config.Timeout,
    })

    // Initialize host functions
    m.Runtime.RegisterHostFunctions(m.createHostBindings())
}
```

### Request Handling

```go
// tau/services/monkey/stream.go
func (srv *Service) handleExecute(stream network.Stream) {
    // Decode request
    var req ExecuteRequest
    json.NewDecoder(stream).Decode(&req)

    // Get or create Monkey instance
    monkey, err := srv.getOrCreateMonkey(req.FunctionID)

    // Execute function
    resp, err := monkey.Execute(&req)

    // Send response
    json.NewEncoder(stream).Encode(resp)
}
```

---

## Job Processing

### Patrick Integration

```go
// tau/services/monkey/pubsub.go
func (srv *Service) pubsubMsgHandler(msg *pubsub.Message) {
    var job patrick.Job
    json.Unmarshal(msg.Data, &job)

    switch job.Type {
    case "build":
        srv.handleBuildJob(&job)
    case "deploy":
        srv.handleDeployJob(&job)
    case "delete":
        srv.handleDeleteJob(&job)
    }
}

func (srv *Service) handleBuildJob(job *patrick.Job) {
    logger.Info("Processing build job:", job.ID)

    // Fetch source from Hoarder
    source, err := srv.hoarderClient.Get(job.SourceCID)

    // Compile to WASM
    builder := &SmartOpsBuilder{
        Language: job.Language,
        Source:   string(source),
    }
    err = builder.Build(srv.ctx)

    // Store WASM artifact
    wasmCID, err := srv.hoarderClient.Put(builder.Wasm)

    // Update job status
    srv.patrickClient.UpdateJob(job.ID, patrick.Status{
        State:   "completed",
        WasmCID: wasmCID,
    })
}
```

### Job Queue

```go
// tau/services/monkey/job.go
type JobQueue struct {
    jobs     chan *Job
    workers  int
    monkeys  *MonkeyManager
}

func (q *JobQueue) Start() {
    for i := 0; i < q.workers; i++ {
        go q.worker()
    }
}

func (q *JobQueue) worker() {
    for job := range q.jobs {
        q.processJob(job)
    }
}

func (q *JobQueue) processJob(job *Job) {
    // Track job start time
    q.monkeys.recvJobs[job.ID] = time.Now()

    // Execute job
    result := job.Execute()

    // Track completion
    delete(q.monkeys.recvJobs, job.ID)
}
```

---

## Container Management

### Garbage Collection

```go
// tau/pkg/containers/gc/gc.go
type GCConfig struct {
    Interval time.Duration  // Default: 5 minutes
    MaxAge   time.Duration  // Default: 15 minutes
}

var DefaultInterval = 5 * time.Minute
var DefaultMaxAge = 15 * time.Minute

func Start(ctx context.Context, interval, maxAge time.Duration) error {
    ticker := time.NewTicker(interval)
    go func() {
        for range ticker.C {
            cleanupOldContainers(maxAge)
        }
    }()
    return nil
}

func cleanupOldContainers(maxAge time.Duration) {
    now := time.Now()
    for id, monkey := range monkeys {
        if now.Sub(monkey.LastUsed) > maxAge {
            monkey.Close()
            delete(monkeys, id)
            logger.Info("GC: Removed container", id)
        }
    }
}
```

### SmartOps Behavior

```
┌─────────────────────────────────────────────────────────────┐
│                    SMARTOPS LIFECYCLE                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐ │
│  │ Created │───▶│  Warm   │───▶│  Idle   │───▶│   GC    │ │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘ │
│       │              │              │              │        │
│       │              │              │              │        │
│       ▼              ▼              ▼              ▼        │
│   Allocate    Pre-initialize    Wait for      Remove if   │
│   Container   Handler          Request       idle > max   │
│                                                             │
│  Request → Warm Container (no cold start)                  │
│  No Request → GC after MaxAge                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## HTTP API

### API Endpoints

```go
// tau/services/monkey/api_monkeys.go
func (srv *Service) setupHTTPRoutes() {
    srv.http.HandleFunc("/api/monkey/list", srv.handleList)
    srv.http.HandleFunc("/api/monkey/info/{id}", srv.handleInfo)
    srv.http.HandleFunc("/api/monkey/execute", srv.handleExecute)
    srv.http.HandleFunc("/api/monkey/build", srv.handleBuild)
}

func (srv *Service) handleList(w http.ResponseWriter, r *http.Request) {
    monkeys := srv.monkeys.List()
    json.NewEncoder(w).Encode(monkeys)
}

func (srv *Service) handleExecute(w http.ResponseWriter, r *http.Request) {
    var req ExecuteRequest
    json.NewDecoder(r.Body).Decode(&req)

    monkey, err := srv.getOrCreateMonkey(req.FunctionID)
    resp, err := monkey.Execute(&req)

    json.NewEncoder(w).Encode(resp)
}
```

---

## P2P Protocol

### Stream Handlers

```go
// tau/services/monkey/stream.go
func (srv *Service) setupStreamRoutes() {
    srv.stream.HandleFunc("execute", srv.handleExecute)
    srv.stream.HandleFunc("build", srv.handleBuild)
    srv.stream.HandleFunc("list", srv.handleList)
    srv.stream.HandleFunc("info", srv.handleInfo)
}

func (srv *Service) handleExecute(stream network.Stream) {
    // Read request
    var req ExecuteRequest
    if err := json.NewDecoder(stream).Decode(&req); err != nil {
        stream.Reset()
        return
    }

    // Execute function
    monkey, err := srv.getMonkey(req.FunctionID)
    resp, err := monkey.Execute(&req)

    // Write response
    json.NewEncoder(stream).Encode(resp)
    stream.Close()
}
```

### Protocol Messages

```go
// tau/services/monkey/type.go
type ExecuteRequest struct {
    FunctionID string                 `json:"function_id"`
    Path       string                 `json:"path"`
    Method     string                 `json:"method"`
    Headers    map[string]string      `json:"headers"`
    Body       []byte                 `json:"body"`
}

type ExecuteResponse struct {
    StatusCode int                    `json:"status_code"`
    Headers    map[string]string      `json:"headers"`
    Body       []byte                 `json:"body"`
    Error      string                 `json:"error,omitempty"`
}

type BuildRequest struct {
    Language   string                 `json:"language"`
    SourceCID  string                 `json:"source_cid"`
    Config     BuildConfig            `json:"config"`
}
```

---

## Host Function Bindings

Monkey exposes host functions to WASM modules:

### Database Functions

| Function | Description |
|----------|-------------|
| `tau_db_new(name)` | Create/open database |
| `tau_db_get(db, key)` | Get value from database |
| `tau_db_put(db, key, value)` | Put value to database |
| `tau_db_delete(db, key)` | Delete key from database |
| `tau_db_list(db, prefix)` | List keys with prefix |
| `tau_db_close(db)` | Close database |

### Storage Functions

| Function | Description |
|----------|-------------|
| `tau_storage_new(bucket)` | Create/open bucket |
| `tau_storage_open(storage, path)` | Open file for reading |
| `tau_storage_put(storage, path, data)` | Write file |
| `tau_storage_delete(storage, path)` | Delete file |
| `tau_storage_list(storage, prefix)` | List files |

### HTTP Functions

| Function | Description |
|----------|-------------|
| `tau_http_client_new()` | Create HTTP client |
| `tau_http_send(client, request)` | Send HTTP request |
| `tau_http_response_status(response)` | Get status code |
| `tau_http_response_body(response)` | Get response body |

### Pub/Sub Functions

| Function | Description |
|----------|-------------|
| `tau_pubsub_publish(channel, data)` | Publish message |
| `tau_pubsub_subscribe(channel)` | Subscribe to channel |

### I2MV Functions

| Function | Description |
|----------|-------------|
| `tau_i2mv_memview_new(data)` | Create memory view |
| `tau_i2mv_memview_read(id)` | Read from memory view |
| `tau_i2mv_fifo_new()` | Create FIFO queue |

---

## Testing

### Unit Tests

```go
// tau/services/monkey/service_test.go
func TestServiceNew(t *testing.T) {
    config := &tauConfig.Node{
        DevMode: true,
        Root:    t.TempDir(),
    }

    srv, err := New(context.Background(), config)
    if err != nil {
        t.Fatal(err)
    }
    defer srv.Close()

    if srv.node == nil {
        t.Error("node should be initialized")
    }
}
```

### Integration Tests

```go
// tau/services/monkey/tests/p2p_test.go
func TestMonkeyP2PExecution(t *testing.T) {
    // Start Monkey service
    config := createTestConfig()
    monkey, err := New(ctx, config)

    // Create test function
    wasm := compileTestFunction()

    // Execute via P2P stream
    stream, err := node.NewStream(monkeyProtocol)
    sendExecuteRequest(stream, wasm)

    // Verify response
    resp := readResponse(stream)
    assert.Equal(t, 200, resp.StatusCode)
}
```

### Fixture Tests

```go
// tau/services/monkey/fixtures/compile/function_go_test.go
func TestGoFunctionCompilation(t *testing.T) {
    source := `
package main
import "github.com/taubyte/go-sdk/http"

func handle(event http.Event) {
    event.Write([]byte("Hello, World!"))
}
`
    wasm, err := compileGo(source)
    if err != nil {
        t.Fatal(err)
    }

    if len(wasm) == 0 {
        t.Error("WASM should not be empty")
    }
}
```

---

## Configuration

### Function Configuration

```yaml
# .tau/functions/api/function.yaml
name: api-handler
language: rust
memory: 128MB
timeout: 30s
routes:
  - path: /api/*
    methods: [GET, POST, PUT, DELETE]
environment:
  LOG_LEVEL: info
  DB_NAME: main-db
```

### Service Configuration

```yaml
# config/monkey.yaml
monkey:
  max_containers: 100
  container_max_age: 15m
  build:
    go:
      version: 1.21
      tinygo_version: 0.30
    rust:
      version: 1.75
      target: wasm32-unknown-unknown
    zig:
      version: 0.11
    assemblyscript:
      version: 0.27
```

---

## Performance Considerations

### Optimization Strategies

1. **WASM Optimization**
   - Use `wasm-opt -Oz` for size optimization
   - Strip debug symbols
   - Enable LTO for Rust

2. **Container Pre-warming**
   - Keep hot containers initialized
   - Predictive warming based on traffic patterns

3. **Memory Management**
   - Set appropriate memory limits
   - GC idle containers promptly

4. **Caching**
   - Cache WASM modules in memory
   - Cache compilation results

---

## Troubleshooting

### Common Issues

1. **Compilation Failures**
   - Check language version compatibility
   - Verify dependencies are available
   - Review compiler logs

2. **Runtime Errors**
   - Check WASM imports match host functions
   - Verify memory limits are sufficient
   - Review VM logs

3. **Performance Issues**
   - Monitor container GC frequency
   - Check WASM module size
   - Review host function latency

---

## Related Documents

- `../exploration.md` - Main exploration
- `patrick.md` - Build scheduler
- `hoarder.md` - Storage service
- `../rust-revision.md` - Rust implementation guide
