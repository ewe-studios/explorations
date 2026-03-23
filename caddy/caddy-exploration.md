# Caddy Core - Deep Dive Exploration

## Module System Architecture

### Module Registration

Caddy's module system is one of its most powerful features. Modules are registered using a global registry:

```go
// From modules.go
var modules = make(map[string]ModuleInfo)

func RegisterModule(instance Module) {
    mod := instance.CaddyModule()
    // Validation...
    modules[string(mod.ID)] = mod
}
```

**Module Info Structure:**
```go
type ModuleInfo struct {
    ID  ModuleID      // Unique identifier (e.g., "http.handlers.file_server")
    New func() Module // Constructor returning empty instance
}
```

**Module Interfaces:**

1. **Module** (required) - Basic identification
```go
type Module interface {
    CaddyModule() ModuleInfo
}
```

2. **Provisioner** - Setup after loading
```go
type Provisioner interface {
    Provision(Context) error
}
```

3. **Validator** - Configuration validation
```go
type Validator interface {
    Validate() error
}
```

4. **CleanerUpper** - Resource cleanup
```go
type CleanerUpper interface {
    Cleanup() error
}
```

### Module Loading Flow

```
1. Configuration JSON received
       │
       ▼
2. Parse ModuleMap (map[string]json.RawMessage)
       │
       ▼
3. For each module:
   ├─ Extract module name from key
   ├─ Get ModuleInfo from registry
   ├─ Call ModuleInfo.New() for empty instance
   ├─ Unmarshal JSON into instance
   ├─ If Provisioner: call Provision()
   ├─ If Validator: call Validate()
   └─ Type-assert to expected interface
       │
       ▼
4. Module ready for use
```

### Module Namespaces

Modules are organized in a hierarchical namespace:

```
Namespace              Module Name    Full ID
─────────────────────  ────────────   ─────────────────────────────
http                   (none)         http
http.handlers          file_server    http.handlers.file_server
caddy.logging          json           caddy.logging.encoders.json
tls.certificates       automate       tls.certificates.automate
```

## Listener Management

### NetworkAddress Structure

```go
type NetworkAddress struct {
    Network   string  // "tcp", "unix", "udp", etc.
    Host      string  // hostname, IP, or socket path
    StartPort uint    // Start of port range (inclusive)
    EndPort   uint    // End of port range (inclusive)
}
```

### Listener Reuse Strategy

Caddy supports overlapping listeners for graceful reloads:

**Unix Systems (SO_REUSEPORT):**
```go
// From listen_unix.go
func (na NetworkAddress) listenUnixgram(...) {
    // SO_REUSEPORT allows multiple processes to bind
    // to the same address, kernel load-balances
}
```

**Windows (Virtual Close):**
- Uses timeout-based virtualization
- Listeners appear closed but remain functional

**Unix Sockets:**
- File descriptor duplication
- Socket unlinked before rebinding

### Listener Key System

Listeners are deduplicated by key:
```go
func listenerKey(network, addr string) string {
    return network + "/" + addr
}
```

Multiple requests for the same listener return the same underlying socket.

### Port Ranges

NetworkAddress supports port ranges:
```go
// Expand returns one NetworkAddress per port
func (na NetworkAddress) Expand() []NetworkAddress {
    size := na.PortRangeSize()
    addrs := make([]NetworkAddress, size)
    for portOffset := range size {
        addrs[portOffset] = na.At(portOffset)
    }
    return addrs
}
```

## Context System

The `caddy.Context` provides modules access to:
- Configuration values
- Other modules (via `App()` method)
- Storage backend
- Events system
- Logging

**Context Methods:**
```go
type Context struct {
    // Load a module by name
    LoadModule(interface{}, json.RawMessage) (any, error)

    // Get another app instance
    App(name string) (any, error)

    // Access global storage
    Storage() Storage

    // Get logger
    Logger() *zap.Logger

    // Replace environment variables
    NewReplacer() *Replacer
}
```

## Configuration System

### JSON Configuration Structure

```json
{
  "apps": {
    "http": {
      "servers": {
        "srv0": {
          "listen": [":443"],
          "routes": [...]
        }
      }
    },
    "tls": {
      "certificates": {
        "automate": ["example.com"]
      }
    }
  }
}
```

### Admin API

Caddy exposes a runtime configuration API:

```
POST /load          - Load new configuration
POST /config/...    - Modify specific paths
GET  /config/...    - Read configuration
DELETE /config/...  - Remove configuration
```

**Dynamic Updates:**
- Changes applied without restart
- Old config gracefully drained
- Resources from old config cleaned up

## HTTP Server Architecture

### Route Structure

HTTP routes are processed as a middleware chain:

```go
type Route struct {
    // Match conditions
    Match []RequestMatcher

    // Handlers to execute
    Handlers []MiddlewareHandler
}
```

**Request Flow:**
```
Request
   │
   ▼
┌──────────────────────────────┐
│  Match (all must succeed)    │
└──────────────────────────────┘
   │ Match?
   ├─ No ──► Next Route
   │
   ▼ Yes
┌──────────────────────────────┐
│  Middleware Chain Execution  │
│  (in order specified)        │
└──────────────────────────────┘
   │
   ▼
Response
```

### Middleware Handlers

```go
type MiddlewareHandler interface {
    ServeHTTP(http.ResponseWriter, *http.Request, Handler) error
}

type Handler interface {
    ServeHTTP(http.ResponseWriter, *http.Request) error
}
```

Middleware wraps the next handler:
```go
func (h MyMiddleware) ServeHTTP(w http.ResponseWriter, r *http.Request, next Handler) error {
    // Pre-processing
    err := next.ServeHTTP(w, r)  // Call next in chain
    // Post-processing
    return err
}
```

## Logging System

### Logger Hierarchy

Loggers are hierarchical with named scopes:

```go
// Parent logger
logger := ctx.Logger()

// Named child logger
httpLogger := logger.Named("http")
tlsLogger := logger.Named("tls")
```

### Log Encoder Options

- **Console**: Human-readable format
- **JSON**: Machine-parseable format
- **Filter**: Redact sensitive fields

### Custom Log Fields

Modules can add structured fields:
```go
logger.Info("request handled",
    zap.String("method", r.Method),
    zap.String("path", r.URL.Path),
    zap.Duration("duration", elapsed),
)
```

## Storage Interface

### Storage Abstraction

```go
type Storage interface {
    // Basic CRUD
    Store(ctx, key, value) error
    Load(ctx, key) ([]byte, error)
    Delete(ctx, key) error
    Exists(ctx, key) bool

    // Listing
    List(ctx, prefix string) ([]string, error)
    Stat(ctx, key) (KeyInfo, error)

    // Distributed locking
    Lock(ctx, key string) error
    Unlock(ctx, key string) error
}
```

### File Storage Implementation

Default storage uses filesystem:
```
$HOME/.local/share/certmagic/
├── acme/
│   └── acme-v02.api.letsencrypt.org-directory/
│       └── accounts/
│           └── {account-key-hash}/
│               └── meta.json
├── certificates/
│   └── {domain}/
│       ├── {domain}.crt
│       ├── {domain}.key
│       └── {domain}.meta.json
└── ocsp/
    └── {hash}.resp
```

## Event System

### Event Emission

```go
// Emit event
events.Emit(ctx, "cert_obtained", map[string]any{
    "identifier": "example.com",
    "issuer": "letsencrypt",
})

// Subscribe to events
events.Subscribe(func(ctx context.Context, event string, data map[string]any) error {
    if event == "cert_obtained" {
        // Handle event
    }
    return nil
})
```

### Event Types

| Event | Description |
|-------|-------------|
| `cert_obtaining` | Certificate about to be obtained |
| `cert_obtained` | Certificate successfully obtained |
| `cert_failed` | Certificate obtain/renew failed |
| `cert_ocsp_revoked` | Certificate's OCSP shows revocation |
| `tls_get_certificate` | TLS handshake GetCertificate phase |

## Signal Handling

### Graceful Shutdown

Caddy handles OS signals for graceful shutdown:

**POSIX Signals:**
- `SIGINT`, `SIGTERM`: Initiate shutdown
- `SIGHUP`: Reload configuration (alternative to API)

**Windows:**
- `Ctrl+C`, `Ctrl+Break`: Shutdown
- Service control messages for Windows Service mode

### Shutdown Sequence

```
1. Signal received
       │
       ▼
2. Admin API stops accepting new requests
       │
       ▼
3. Existing connections drained (with timeout)
       │
       ▼
4. Modules' Cleanup() methods called
       │
       ▼
5. Listeners closed
       │
       ▼
6. Process exits
```

## File System Abstraction

### Filesystem Interface

```go
type FileSystem interface {
    Open(path string) (fs.File, error)
    Stat(path string) (fs.FileInfo, error)
    ReadDir(path string) ([]fs.DirEntry, error)
}
```

### Built-in Filesystems

1. **osfs**: Standard OS filesystem
2. **rootfs**: Rooted at specific path
3. **Memory fs**: For testing

## REPLACER System

### String Replacement

Replacer handles variable substitution:

```go
repl := caddy.NewReplacer()

// Set values
repl.Set("http.request.uri", r.URL.RequestURI())
repl.Set("http.request.host", r.Host)

// Replace in strings
result := repl.ReplaceAll("Hello {http.request.host}!")
```

### Placeholder Syntax

- `{http.request.uri}` - Request URI
- `{http.request.host}` - Host header
- `{http.request.method}` - HTTP method
- `{time.now}` - Current timestamp
- `{env.VAR_NAME}` - Environment variable

## Testing Infrastructure

### Test Helpers

Caddy provides testing utilities:

```go
// caddytest package
type Tester struct {
    // Load configuration
    LoadConfig(config string)

    // Make test requests
    AssertResponse(request, expectedStatus, expectedBody)
}
```

### Integration Tests

Test directories:
- `caddytest/integration/`: End-to-end tests
- `caddytest/caddyfile_adapt/`: Caddyfile parsing tests

## Performance Optimizations

### Connection Pooling

```go
// Shared transport for HTTP client
var defaultTransport = &http.Transport{
    Proxy: http.ProxyFromEnvironment,
    DialContext: (&net.Dialer{
        Timeout:   30 * time.Second,
        KeepAlive: 2 * time.Minute,
    }).DialContext,
    TLSHandshakeTimeout:   30 * time.Second,
    ResponseHeaderTimeout: 30 * time.Second,
    ExpectContinueTimeout: 2 * time.Second,
    ForceAttemptHTTP2:     true,
    MaxIdleConnsPerHost:   100,
}
```

### Usage Pool

Prevents duplicate operations:

```go
type usagePool struct {
    items map[string]*usageItem
    mu    sync.Mutex
}

func (p *usagePool) Get(key string) (item *usageItem, existing bool) {
    p.mu.Lock()
    defer p.mu.Unlock()
    // ...
}
```

## Error Handling

### Error Types

```go
// Error with failure status
type HandlerError struct {
    StatusCode int
    Err        error
}

func (h HandlerError) Error() string {
    return h.Err.Error()
}
```

### Error Propagation

Middleware returns errors up the chain:
```go
func (h Handler) ServeHTTP(w http.ResponseWriter, r *http.Request, next Handler) error {
    err := next.ServeHTTP(w, r)
    if err != nil {
        // Handle error
        return HandlerError{
            StatusCode: http.StatusInternalServerError,
            Err:        err,
        }
    }
    return nil
}
```

## Key Design Decisions

### 1. Immutable Config Snapshots

Each config load creates a new snapshot. Old configs run in parallel until connections drain.

### 2. Explicit Module Interfaces

Modules must explicitly implement interfaces. No implicit interface satisfaction.

### 3. Structured Logging Only

All logging uses zap for structured output. No printf-style logging.

### 4. No Global State

Configuration is passed through Context. Modules don't access global state directly.

### 5. Concurrency Safety

All shared state protected by mutexes. Read-heavy paths use RWMutex.
