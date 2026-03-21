# Taubyte Architecture Deep Dive

## Executive Summary

Taubyte (Tau) is a distributed, Git-native platform-as-a-service built on a peer-to-peer architecture. This document provides a comprehensive analysis of the system architecture, design patterns, and component interactions.

---

## System Architecture Overview

### High-Level Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              TAU PLATFORM                                   │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      P2P NETWORK LAYER                               │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐            │   │
│  │  │  Monkey  │  │ Patrick  │  │ Hoarder  │  │  Seer    │            │   │
│  │  │  Nodes   │  │  Nodes   │  │  Nodes   │  │  Nodes   │            │   │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘            │   │
│  │       │             │             │             │                    │   │
│  │  ┌────┴─────────────┴─────────────┴─────────────┴────┐              │   │
│  │  │           LIBP2P COMMUNICATION LAYER              │              │   │
│  │  │  • GossipSub Pub/Sub  • Kademlia DHT             │              │   │
│  │  │  • mDNS Discovery     • TCP/QUIC Transport       │              │   │
│  │  └───────────────────────────────────────────────────┘              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      SERVICE LAYER                                   │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐            │   │
│  │  │  Gateway │  │   Auth   │  │   TNS    │  │ Substrate│            │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      VM LAYER                                        │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │              WASM RUNTIME (wazero)                            │   │   │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │   │   │
│  │  │  │   Go     │  │  Rust    │  │ Zig      │  │  AS      │     │   │   │
│  │  │  │  SDK     │  │  SDK     │  │  SDK     │  │  SDK     │     │   │   │
│  │  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      STORAGE LAYER                                   │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐            │   │
│  │  │  Hoarder │  │ Substrate│  │  TNS     │  │  Local   │            │   │
│  │  │  (S3)    │  │ (KV DB)  │  │ (Cache)  │  │   FS     │            │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Service Architecture

### Service Interface Hierarchy

```go
// tau/core/services/types.go

// Base Service interface
type Service interface {
    Node() peer.Node
    Close() error
}

// Database-backed service
type DBService interface {
    Service
    KV() kvdb.KVDB
}

// HTTP-capable service
type HttpService interface {
    Service
    Http() http.Service
}

// GitHub authentication support
type GitHubAuth interface {
    GitHubTokenHTTPAuth(ctx http.Context) (interface{}, error)
    GitHubTokenHTTPAuthCleanup(ctx http.Context) (interface{}, error)
}
```

### Service Implementation Pattern

Every service follows a consistent pattern:

```go
// 1. Service struct
type Service struct {
    ctx      context.Context
    node     peer.Node
    config   *tauConfig.Node
    stream   *streams.Stream
    kv       kvdb.KVDB
    // Service-specific fields
}

// 2. Constructor
func New(ctx context.Context, config *tauConfig.Node, opts ...Options) (*Service, error) {
    // Validate config
    // Initialize node (lite or full)
    // Setup P2P stream
    // Register handlers
    // Start background processes
}

// 3. Close method
func (srv *Service) Close() error {
    // Stop stream
    // Close KV store
    // Cleanup resources
}
```

---

## P2P Communication Architecture

### Protocol Stack

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│  (Monkey, Patrick, Hoarder, etc.)       │
├─────────────────────────────────────────┤
│         Stream Protocol                 │
│  /tau/<service>/1.0.0                   │
├─────────────────────────────────────────┤
│         Transport                       │
│  TCP / QUIC / WebSocket                 │
├─────────────────────────────────────────┤
│         libp2p Core                     │
│  • Host     • Routing  • Security       │
├─────────────────────────────────────────┤
│         Network                         │
│  IPv4 / IPv6                            │
└─────────────────────────────────────────┘
```

### Protocol Identifiers

```go
// tau/services/common/ports.go
const (
    MonkeyProtocol  = "/tau/monkey/1.0.0"
    PatrickProtocol = "/tau/patrick/1.0.0"
    HoarderProtocol = "/tau/hoarder/1.0.0"
    SeerProtocol    = "/tau/seer/1.0.0"
    AuthProtocol    = "/tau/auth/1.0.0"
    GatewayProtocol = "/tau/gateway/1.0.0"
    TNSProtocol     = "/tau/tns/1.0.0"
)
```

### Stream Service

```go
// tau/p2p/streams/service/
type Stream struct {
    node     peer.Node
    name     string
    protocol string
    handlers map[string]StreamHandler
}

func (s *Stream) Start() {
    s.node.SetStreamHandler(s.protocol, s.handleStream)
}

func (s *Stream) handleStream(stream network.Stream) {
    // Route to appropriate handler based on message type
}
```

---

## Pub/Sub Architecture

Tau uses libp2p-pubsub (GossipSub) for event-driven communication:

### Pub/Sub Topics

| Topic | Publisher | Subscribers | Purpose |
|-------|-----------|-------------|---------|
| `patrick.jobs` | Patrick | Monkey | Job notifications |
| `seer.heartbeat` | All Services | Seer | Health monitoring |
| `auth.events` | Auth | Gateway | Auth events |

### Subscription Pattern

```go
// tau/services/monkey/service.go
func (srv *Service) subscribe() error {
    return srv.node.PubSubSubscribe(
        patrickSpecs.PubSubIdent,  // Topic
        func(msg *pubsub.Message) {
            go srv.pubsubMsgHandler(msg)
        },
        func(err error) {
            // Reconnect on error
            if err.Error() != "context canceled" {
                logger.Error("Subscription error:", err)
                srv.subscribe()
            }
        },
    )
}
```

---

## Seer Beacon System

Services broadcast their presence to Seer for discovery:

### Beacon Structure

```go
// tau/services/common/seer_beacon.go
type BeaconConfig struct {
    ServiceType string
    Meta        map[string]string
    Interval    time.Duration
}

func StartSeerBeacon(config *Node, client *seerClient.Client, serviceType string) {
    ticker := time.NewTicker(beaconInterval)
    for range ticker.C {
        usage := collectUsage()
        client.Announce(usage)
    }
}
```

### Usage Data

```go
// tau/clients/p2p/seer/usage_client.go
type Usage struct {
    CPU    float64
    Memory uint64
    Disk   uint64
    Meta   map[string]string
}
```

---

## Virtual Machine Architecture

### VM Component Diagram

```
┌────────────────────────────────────────────────────────────────┐
│                         TAU VM                                  │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    WASM MODULE                           │  │
│  │  ┌────────────────────────────────────────────────────┐  │  │
│  │  │                 User Code                          │  │  │
│  │  │  (Go / Rust / Zig / AssemblyScript)                │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  │                          │                                │  │
│  │  ┌───────────────────────┴────────────────────────────┐  │  │
│  │  │              WASI Imports                          │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  │                          │                                │  │
│  │  ┌───────────────────────┴────────────────────────────┐  │  │
│  │  │            Tau Host Functions                      │  │  │
│  │  │  • database_*  • storage_*  • http_*              │  │  │
│  │  │  • pubsub_*    • i2mv_*     • utils_*             │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────┴──────────────────────────────┐  │
│  │                    wazero Runtime                        │  │
│  │  • Compilation  • Instantiation  • Execution             │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────┴──────────────────────────────┐  │
│  │                    Host Bindings                         │  │
│  │  • memory.NewBuffer  • imports.Function                 │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

### Host Function Registration

```go
// tau/vm/wasm.go (simplified)
func registerHostFunctions(store *.Store) {
    // Database
    store.AddFunction("tau_db_new", dbNew)
    store.AddFunction("tau_db_get", dbGet)
    store.AddFunction("tau_db_put", dbPut)

    // Storage
    store.AddFunction("tau_storage_new", storageNew)
    store.AddFunction("tau_storage_open", storageOpen)

    // HTTP
    store.AddFunction("tau_http_send", httpSend)

    // Pub/Sub
    store.AddFunction("tau_pubsub_publish", pubsubPublish)

    // I2MV
    store.AddFunction("tau_i2mv_memview_new", memviewNew)
}
```

### Memory View System (I2MV)

I2MV (Inter-VM Memory Views) enables efficient data transfer:

```rust
// rust-sdk/src/i2mv/mod.rs
pub mod fifo;      // FIFO queues
pub mod memview;   // Memory views

// Memory view types:
// - Closer: Write-only, auto-close
// - ReadSeekCloser: Read/seek with close
// - FIFO Read/Write Closers: Queue operations
```

---

## Data Flow Architecture

### Request Flow (HTTP → Function)

```
1. Client Request
       │
       ▼
2. Gateway Service
       │
       ▼
3. Auth Service (if authenticated route)
       │
       ▼
4. TNS (resolve function ID)
       │
       ▼
5. Monkey (execute function)
       │
       ▼
6. WASM Runtime
       │
       ▼
7. Host Functions (DB/Storage/HTTP)
       │
       ▼
8. Response → Gateway → Client
```

### Build Flow (Git → Deployment)

```
1. Git Push
       │
       ▼
2. Webhook → Patrick
       │
       ▼
3. Patrick creates build job
       │
       ▼
4. Hoarder fetches source
       │
       ▼
5. Monkey compiles (Go/Rust/Zig → WASM)
       │
       ▼
6. Hoarder stores WASM artifact
       │
       ▼
7. Patrick updates function registry
       │
       ▼
8. TNS propagates changes
```

---

## Storage Architecture

### Hoarder (Object Storage)

```
┌─────────────────────────────────────────────────────┐
│                    HOARDER                          │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌─────────────────────────────────────────────┐   │
│  │              Content Store                   │   │
│  │  • Content-addressable (CID-based)          │   │
│  │  • Deduplication                            │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  ┌─────────────────────────────────────────────┐   │
│  │              Rare Cache                      │   │
│  │  • Least-recently-used eviction             │   │
│  │  • Hot content caching                      │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  ┌─────────────────────────────────────────────┐   │
│  │              Stash                           │   │
│  │  • Long-term storage                        │   │
│  │  • P2P replication                          │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Substrate (Key-Value Database)

```go
// tau/core/services/substrate/
type Database interface {
    Get(key []byte) ([]byte, error)
    Put(key, value []byte) error
    Delete(key []byte) error
    List(prefix []byte) ([][]byte, error)
}

// Implementation uses Pebble (LevelDB-compatible)
```

### TNS Cache

```go
// tau/clients/p2p/tns/cache.go
type Cache struct {
    data   map[string][]byte
    ttl    time.Duration
    mutex  sync.RWMutex
}
```

---

## Configuration Architecture

### Git-Native Configuration

```
.tau/
├── project.yaml          # Project definition
├── functions/            # Function configurations
│   ├── api/
│   │   └── function.yaml
│   └── web/
│       └── function.yaml
├── databases/            # Database definitions
│   └── main.yaml
├── storage/              # Bucket definitions
│   └── assets.yaml
└── domains/              # Domain configurations
    └── example.com.yaml
```

### Config Compiler Pipeline

```
┌──────────────┐
│ YAML Config  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Compiler   │  → Validates schema
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Indexer    │  → Creates indexes
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Binary Blob  │  → Efficient loading
└──────────────┘
```

---

## Security Architecture

### Authentication Flow

```
┌─────────────────────────────────────────────────────────────┐
│                   AUTH SERVICE                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  GitHub OAuth                                               │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│  │  Client  │───▶│  GitHub  │───▶│  Token   │             │
│  └──────────┘    └──────────┘    └──────────┘             │
│                                                             │
│  Domain Validation (ACME)                                   │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│  │  Domain  │───▶│   ACME   │───▶│ Certificate│            │
│  │  Owner   │    │  Server  │    │  Store    │            │
│  └──────────┘    └──────────┘    └──────────┘             │
│                                                             │
│  Secrets Management                                         │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│  │  Secret  │───▶│ Encrypted│───▶│  Hoarder │             │
│  │  Value   │    │  Storage │    │  Storage  │            │
│  └──────────┘    └──────────┘    └──────────┘             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### ACME Certificate Flow

```go
// tau/services/auth/acme/store/store.go
type CertStore struct {
    kv       kvdb.KVDB
    domains  map[string]*Certificate
}

func (s *CertStore) GetCertificate(domain string) (*Certificate, error) {
    // Check cache
    // Load from KV if needed
    // Renew if expiring
}
```

### BLS Threshold Encryption

```rust
// blsttc/src/lib.rs
// Encrypt with public key
pub fn encrypt(pk_id: u32, msg_id: u32) -> u32 {
    // Load public key from memory view
    // Encrypt using blsttc
    // Return cipher text memory view ID
}

// Decrypt with threshold shares
pub fn decrypt(public_key_set_id: u32, shares_id: u32, cipher_text_id: u32) -> u32 {
    // Combine decryption shares
    // Decrypt cipher text
    // Return plain text memory view ID
}
```

---

## Observability Architecture

### Seer Monitoring System

```
┌─────────────────────────────────────────────────────────────┐
│                     SEER SERVICE                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Heartbeat Collection                                       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Service Beacons (every 30s)                        │   │
│  │  • CPU Usage                                        │   │
│  │  • Memory Usage                                     │   │
│  │  • Disk Usage                                       │   │
│  │  • Custom Metrics                                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  DNS Metrics                                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  • Query count                                      │   │
│  │  • Response time                                    │   │
│  │  • Cache hit rate                                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Geo-Location Service                                       │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  • IP to location mapping                           │   │
│  │  • Latency-based routing                            │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Usage Collection

```go
// tau/clients/p2p/seer/usage_client.go
func AnnounceUsage(client *Client, usage *Usage) error {
    // Collect local metrics
    // Send to Seer via P2P
}

func collectUsage() *Usage {
    cpu := getCPUUsage()
    mem := getMemoryUsage()
    disk := getDiskUsage()
    return &Usage{CPU: cpu, Memory: mem, Disk: disk}
}
```

---

## Container Management

### Garbage Collection (Monkey)

```go
// tau/pkg/containers/gc/
type GCConfig struct {
    Interval time.Duration
    MaxAge   time.Duration
}

func Start(ctx context.Context, interval, maxAge time.Duration) error {
    ticker := time.NewTicker(interval)
    for range ticker.C {
        cleanupOldContainers(maxAge)
    }
}
```

### Container Lifecycle

```
1. Create → WASM instance from Hoarder
2. Warm   → Pre-initialize (optional)
3. Execute → Run handler function
4. Idle   → Wait for next invocation
5. GC     → Remove if idle > MaxAge
```

---

## Network Architecture

### Port Assignments

```go
// tau/services/common/ports.go
var DefaultPorts = map[string]int{
    "monkey":  7777,
    "patrick": 7778,
    "hoarder": 7779,
    "seer":    7780,
    "auth":    7781,
    "gateway": 7782,
    "tns":     7783,
    "dns":     53,
}
```

### Dream (Local Cloud) Networking

```go
// tau/tools/dream/cli/new/universe.go
func createUniverse(name string) error {
    // Create isolated network namespace
    // Assign unique ports
    // Start all services
    // Setup P2P bootstrap
}
```

---

## Build System Architecture

### Monkey Compilation Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                MONKEY BUILD SYSTEM                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Language-Specific Compilers                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  TinyGo  │  │  rustc   │  │  zig     │  │  asc     │   │
│  │  → wasm  │  │  → wasm  │  │  → wasm  │  │  → wasm  │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
│       │             │             │             │          │
│       └─────────────┴─────────────┴─────────────┘          │
│                           │                                 │
│  ┌────────────────────────┴────────────────────────────┐   │
│  │              WASM Optimizer                          │   │
│  │  • wasm-opt  • strip debug  • minify                │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│  ┌────────────────────────┴────────────────────────────┐   │
│  │              Artifact Store (Hoarder)                │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### SmartOps Builder

```go
// tau/services/monkey/fixtures/compile/smartops.go
type SmartOpsBuilder struct {
    language string
    source   []byte
    wasm     []byte
}

func (b *SmartOpsBuilder) Compile() error {
    switch b.language {
    case "go":
        return b.compileGo()
    case "rust":
        return b.compileRust()
    case "zig":
        return b.compileZig()
    case "assemblyscript":
        return b.compileAS()
    }
}
```

---

## Scaling Architecture

### Horizontal Scaling

Each service can scale independently:

```
┌────────────────────────────────────────────────────────────┐
│                    Monkey Cluster                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Monkey-1 │  │ Monkey-2 │  │ Monkey-3 │  │ Monkey-N │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │
│         │             │             │             │        │
│         └─────────────┴─────────────┴─────────────┘        │
│                           │                                 │
│                  ┌────────┴────────┐                       │
│                  │  Patrick (LB)   │                       │
│                  └─────────────────┘                       │
└────────────────────────────────────────────────────────────┘
```

### P2P Discovery

```go
// tau/p2p/peer/
func bootstrap(ctx context.Context, bootstrappers []multiaddr.Multiaddr) error {
    // Connect to bootstrap nodes
    // Discover peers via mDNS
    // Join GossipSub topics
    // Start DHT bootstrap
}
```

---

## Key Design Decisions

### 1. Git-Native Over API-First
- **Decision**: Store all configuration in Git
- **Rationale**: Version control, audit trail, GitOps workflows
- **Trade-off**: Requires Git knowledge, slower for quick changes

### 2. P2P Over Centralized
- **Decision**: Use libp2p for all service communication
- **Rationale**: No single point of failure, horizontal scaling
- **Trade-off**: More complex networking, eventual consistency

### 3. WASM Over Containers
- **Decision**: Use WASM for serverless functions
- **Rationale**: Fast startup, language agnostic, secure sandbox
- **Trade-off**: WASM limitations (no threads, limited syscalls)

### 4. Content-Addressable Storage
- **Decision**: Use CIDs for artifact storage
- **Rationale**: Deduplication, integrity verification
- **Trade-off**: No human-readable names (requires TNS)

### 5. SmartOps Over Cold Start
- **Decision**: Pre-warm containers, GC when idle
- **Rationale**: No cold start latency
- **Trade-off**: Resource usage during idle

---

## Future Architecture Considerations

### Potential Improvements

1. **WASI 2.0 Support**: Better system call support
2. **Component Model**: Modular WASM components
3. **Quic Go**: Faster P2P transport
4. **eBPF Integration**: Better observability
5. **Edge Computing**: Geo-distributed execution

---

## Related Documents

- `exploration.md` - Overview exploration
- `subsystems/*.md` - Individual service deep dives
- `production-grade.md` - Production considerations
- `rust-revision.md` - Rust implementation guide
