# Taubyte Comprehensive Exploration

## Overview

**Taubyte** (codename: Tau) is an open-source, Git-native platform-as-a-service (PaaS) for building, deploying, and scaling applications. It provides a fully self-hosted cloud infrastructure with capabilities similar to Vercel, Firebase, and Cloudflare, plus built-in AI features. The platform uses a peer-to-peer (P2P) architecture built on libp2p and features a custom WebAssembly-based virtual machine for serverless function execution.

**Key Characteristics:**
- **Git-Native**: Infrastructure is defined in Git, eliminating API calls
- **Fully Self-Hosted**: Run on your own servers/VMs
- **P2P Architecture**: Distributed services communicating via libp2p
- **WASM Runtime**: Custom VM for serverless functions supporting multiple languages
- **Multi-Language Support**: Go, Rust, AssemblyScript, and Zig for functions

---

## Repository Structure

```
src.Taubyte/
├── tau/                          # Main Tau platform (Go)
│   ├── cli/                      # CLI application
│   ├── clients/                  # HTTP and P2P clients
│   │   ├── http/                 # HTTP API clients
│   │   └── p2p/                  # P2P service clients
│   ├── config/                   # Configuration system
│   ├── core/                     # Core interfaces and abstractions
│   │   ├── builders/             # Service builders
│   │   ├── common/               # Shared components
│   │   ├── kvdb/                 # Key-value database interfaces
│   │   ├── p2p/                  # P2P primitives
│   │   ├── services/             # Service interfaces
│   │   └── vm/                   # VM interfaces
│   ├── pkg/                      # Shared packages
│   │   ├── builder/              # Build system
│   │   ├── cli/                  # CLI utilities
│   │   ├── config-compiler/      # Configuration compilation
│   │   ├── containers/           # Container management
│   │   ├── git/                  # Git operations
│   │   ├── http-auto/            # Auto HTTP routing
│   │   ├── kvdb/                 # KV database implementations
│   │   ├── poe/                  # Policy engine
│   │   ├── raft/                 # Raft consensus
│   │   ├── sensors/              # Monitoring sensors
│   │   ├── specs/                # Specification definitions
│   │   ├── starlark/             # Starlark scripting
│   │   ├── vm-* /                # VM orbit packages
│   │   └── yaseer/               # YAML parser
│   ├── p2p/                      # P2P layer
│   │   ├── peer/                 # Peer management
│   │   ├── streams/              # Stream handling
│   │   └── transport/            # Transport protocols
│   ├── services/                 # Service implementations
│   │   ├── auth/                 # Authentication service
│   │   ├── common/               # Shared service code
│   │   ├── gateway/              # API gateway
│   │   ├── hoarder/              # Storage service
│   │   ├── monkey/               # Function execution service
│   │   ├── patrick/              # Build/scheduler service
│   │   ├── seer/                 # DNS/Discovery service
│   │   ├── substrate/            # Database service
│   │   └── tns/                  # Name service
│   ├── tools/                    # CLI tools
│   │   ├── dream/                # Dream CLI (local cloud)
│   │   ├── spore-drive/          # Deployment automation
│   │   ├── tau/                  # Tau CLI
│   │   └── taucorder/            # Taucorder CLI
│   └── utils/                    # Utilities
├── rust-sdk/                     # Rust SDK for Tau VM
├── blsttc/                       # BLS threshold cryptography
├── go-sdk/                       # Go SDK for Tau VM
├── dream/                        # Dream desktop application
├── p2p/                          # Standalone P2P library
├── vm/                           # VM implementation
├── wazero/                       # Wazero WASM runtime
└── [other components]
```

---

## Core Services Architecture

Tau is composed of several microservices that communicate over P2P:

### 1. **Monkey** - Function Execution Service
- **Purpose**: Executes serverless functions (WASM, Go, Rust, Zig, AssemblyScript)
- **Key Features**:
  - SmartOps (on-demand compute)
  - Container garbage collection
  - Pub/sub integration with Patrick
  - Integration with Hoarder for artifact storage
- **Files**: `tau/services/monkey/`, `tau/core/services/monkey/`

### 2. **Patrick** - Build & Scheduler Service
- **Purpose**: Manages build jobs and scheduling
- **Key Features**:
  - Job queue management
  - Build status tracking
  - Pub/sub for job notifications
- **Files**: `tau/services/patrick/`, `tau/core/services/patrick/`

### 3. **Hoarder** - Storage Service
- **Purpose**: Distributed artifact/object storage
- **Key Features**:
  - Content-addressable storage
  - Rare/stash operations
  - P2P replication
- **Files**: `tau/services/hoarder/`, `tau/core/services/hoarder/`

### 4. **Seer** - DNS & Discovery Service
- **Purpose**: DNS resolution and service discovery
- **Key Features**:
  - Authoritative DNS server
  - Geo-location services
  - Heartbeat monitoring
  - Oracle for external data
- **Files**: `tau/services/seer/`, `tau/core/services/seer/`

### 5. **Auth** - Authentication Service
- **Purpose**: Authentication and authorization
- **Key Features**:
  - GitHub OAuth integration
  - Domain validation (ACME/Let's Encrypt)
  - Certificate management
  - Project/repository access control
- **Files**: `tau/services/auth/`, `tau/core/services/auth/`

### 6. **Gateway** - API Gateway
- **Purpose**: HTTP request routing
- **Key Features**:
  - Request handling
  - WASM function execution
  - Response processing
- **Files**: `tau/services/gateway/`

### 7. **TNS** - Tau Name Service
- **Purpose**: Distributed naming/structure service
- **Key Features**:
  - Structure management
  - ID/name resolution
  - P2P caching
- **Files**: `tau/services/tns/`, `tau/clients/p2p/tns/`

---

## Service Communication Pattern

```
┌─────────────────────────────────────────────────────────────────┐
│                         P2P Network                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Monkey     │  │   Patrick    │  │   Hoarder    │          │
│  │  (Compute)   │  │  (Scheduler) │  │  (Storage)   │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                 │                 │                   │
│  ┌──────┴─────────────────┴─────────────────┴───────┐          │
│  │              Pub/Sub (libp2p-pubsub)              │          │
│  └───────────────────────────────────────────────────┘          │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │    Seer      │  │     Auth     │  │   Gateway    │          │
│  │   (DNS)      │  │   (AuthN/Z)  │  │   (HTTP)     │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

---

## P2P Architecture

Tau uses libp2p for peer-to-peer communication:

### Components:
- **Peer Discovery**: Automatic node discovery
- **GossipSub**: Pub/sub messaging
- **Stream Protocol**: Request/response over streams
- **DHT**: Distributed hash table for content routing

### Protocol Identifiers:
```go
// From tau/services/common/ports.go
var (
    MonkeyProtocol  = "/tau/monkey/1.0.0"
    PatrickProtocol = "/tau/patrick/1.0.0"
    HoarderProtocol = "/tau/hoarder/1.0.0"
    SeerProtocol    = "/tau/seer/1.0.0"
    // ...
)
```

### Seer Beacon:
Services broadcast their presence via Seer using a beacon system:
```go
// From tau/services/common/seer_beacon.go
func StartSeerBeacon(config *Node, client *seerClient.Client, serviceType string)
```

---

## Virtual Machine (VM) Architecture

Tau's VM executes serverless functions compiled to WebAssembly:

### Supported Languages:
1. **Go** - via TinyGo
2. **Rust** - via wasm32-unknown-unknown target
3. **AssemblyScript** - TypeScript-like language
4. **Zig** - via wasm32 target

### VM Components:
- **wazero/**: Go-based WASM runtime
- **vm/**: VM service implementation
- **vm-orbit/**: VM communication layer
- **rust-sdk/**: Rust SDK for function development
- **go-sdk/**: Go SDK for function development

### Host Functions (Imports):
The VM exposes host functions to WASM modules:

| Category | Functions |
|----------|-----------|
| **Database** | `db_new`, `db_get`, `db_put`, `db_delete`, `db_list`, `db_close` |
| **Storage** | `storage_new`, `storage_get`, `storage_put`, `storage_open`, `storage_read` |
| **HTTP** | `http_client_new`, `http_send`, `http_response_*` |
| **Pub/Sub** | `pubsub_publish`, `pubsub_subscribe`, `pubsub_channel` |
| **I2MV** | Memory view operations for data transfer |
| **Utils** | Codec, conversion utilities |

---

## Rust SDK Structure

```
rust-sdk/
├── src/
│   ├── lib.rs              # Main library exports
│   ├── database/           # KV database operations
│   │   ├── mod.rs
│   │   ├── new.rs
│   │   ├── get.rs
│   │   ├── put.rs
│   │   ├── delete.rs
│   │   ├── list.rs
│   │   └── close.rs
│   ├── storage/            # Object storage operations
│   │   ├── mod.rs
│   │   ├── new.rs
│   │   ├── get.rs
│   │   ├── file/           # File operations
│   │   └── content/        # Content operations
│   ├── http/               # HTTP operations
│   │   ├── mod.rs
│   │   ├── event/          # HTTP event handling
│   │   └── client/         # HTTP client
│   ├── pubsub/             # Pub/sub operations
│   │   ├── mod.rs
│   │   ├── event/          # Event-based pubsub
│   │   └── node/           # Node-based pubsub
│   ├── i2mv/               # Inter-VM memory views
│   │   ├── mod.rs
│   │   ├── fifo/           # FIFO queues
│   │   └── memview/        # Memory views
│   ├── errno/              # Error handling
│   └── utils/              # Utility functions
└── Cargo.toml
```

### SDK Usage Pattern:
```rust
use taubyte_sdk::{
    database::Database,
    storage::Storage,
    http::event::Event,
    pubsub::Channel,
};

// Open a database
let db = Database::new("my-db");

// Get value
let value = db.get("key");

// Store in object storage
let storage = Storage::new("my-bucket");
let file = storage.open("file.txt");
```

---

## Configuration System

Tau uses a Git-native configuration system:

### Configuration Layers:
1. **Project Config**: `.tau/project.yaml`
2. **Service Config**: Generated by config-compiler
3. **Runtime Config**: Loaded by services

### Config Compiler:
```
tau/pkg/config-compiler/
├── compile/        # Compile YAML to binary format
├── decompile/      # Decompile binary to YAML
├── fixtures/       # Test fixtures
└── indexer/        # Configuration indexing
```

### POE (Policy Engine):
Starlark-based policy engine for configuration validation:
```go
// From tau/services/seer/service.go
poeFolder := os.DirFS(path.Join(config.Root, "config", "poe", "star"))
srv.poe, err = poe.New(poeFolder, "dns.star")
```

---

## Dream CLI

Dream is the local development environment:

### Commands:
- `dream new universe` - Create local cloud
- `dream start` - Start services
- `dream status` - Check service status
- `dream inject` - Inject configurations
- `dream kill` - Stop services

### Universe Concept:
A "universe" is a local Tau cloud instance with all services running.

---

## Spore Drive

Automated deployment system:

- Deploys Tau to cloud providers
- Supports DigitalOcean, AWS, etc.
- Git-based infrastructure provisioning
- IDP (Identity Provider) integration

---

## Build System (Monkey)

Monkey compiles and executes functions:

### Compilation Flow:
1. Source code (Go/Rust/AS/Zig)
2. Compile to WASM (`.wasm`)
3. Upload to Hoarder
4. Patrick schedules build
5. Monkey executes WASM

### SmartOps:
On-demand compute that scales to zero:
- Functions only run when invoked
- No cold start penalty (pre-warmed containers)
- Automatic garbage collection

---

## Testing Strategy

Tau has extensive testing:

### Test Types:
1. **Unit Tests**: Standard Go tests (`_test.go`)
2. **E2E Tests**: Full service integration tests
3. **P2P Tests**: Multi-node P2P testing
4. **Fixture Tests**: Pre-built WASM module tests

### Test Fixtures:
```
tau/services/monkey/fixtures/compile/assets/
├── lib.rs              # Rust test function
├── helloWorld.ts       # AssemblyScript test
├── ping.go             # Go test function
├── ping.zwasm          # Pre-compiled Zwasm
└── website/            # Website test files
```

---

## Key Design Patterns

### 1. Service Interface Pattern
```go
type Service interface {
    Node() peer.Node
    Close() error
}

type DBService interface {
    Service
    KV() kvdb.KVDB
}
```

### 2. Client-Service Separation
Each service has:
- Service implementation (`tau/services/`)
- P2P client (`tau/clients/p2p/`)
- HTTP client (`tau/clients/http/`)

### 3. Stream Protocol Pattern
```go
stream, err := streams.New(node, serviceName, protocolName)
stream.Start()
```

### 4. Pub/Sub Integration
```go
srv.node.PubSubSubscribe(
    patrickSpecs.PubSubIdent,
    func(msg *pubsub.Message) {
        go srv.pubsubMsgHandler(msg)
    },
)
```

---

## Memory Views (I2MV)

Inter-VM Memory Views for efficient data transfer:

### Types:
- **Closer**: Write-only memory view
- **ReadSeekCloser**: Read/seek memory view
- **FIFO**: First-in-first-out queue

### Usage:
```rust
// Rust SDK
use taubyte_sdk::i2mv::memview::{Closer, ReadSeekCloser};

// Create memory view with data
let mv = Closer::new(&data, true).unwrap();

// Read from memory view
let mut mv = ReadSeekCloser::open(id).unwrap();
```

---

## Security Features

### 1. ACME/Let's Encrypt Integration
Automatic HTTPS certificate management in Auth service.

### 2. BLS Threshold Cryptography (blsttc)
```
blsttc/
├── src/lib.rs        # BLS encryption/decryption
└── Cargo.toml
```

Encrypts data using threshold signatures:
```rust
pub fn encrypt(pk_id: u32, msg_id: u32) -> u32
pub fn decrypt(public_key_set_id: u32, shares_id: u32, cipher_text_id: u32) -> u32
```

### 3. Domain Validation
ACME challenge handling for custom domains.

### 4. GitHub OAuth
Secure authentication via GitHub.

---

## Observability

### Seer Monitoring:
- Heartbeat system for service health
- Usage tracking (CPU, memory, disk)
- DNS query metrics

### Sensors:
```go
// tau/pkg/sensors/
// Resource usage monitoring
```

---

## File Summary

### Core Platform Files:
| Path | Description |
|------|-------------|
| `tau/README.md` | Main product documentation |
| `tau/go.mod` | Go module definition |
| `tau/main.go` | Entry point (not present, uses tools/) |
| `tau/config/tau.go` | Main configuration struct |
| `tau/core/services/types.go` | Service interfaces |

### Service Files:
| Service | Main File | Client |
|---------|-----------|--------|
| Monkey | `tau/services/monkey/service.go` | `tau/clients/p2p/monkey/` |
| Patrick | `tau/services/patrick/` | `tau/clients/p2p/patrick/` |
| Seer | `tau/services/seer/service.go` | `tau/clients/p2p/seer/` |
| Hoarder | `tau/services/hoarder/service.go` | `tau/clients/p2p/hoarder/` |
| Auth | `tau/services/auth/service.go` | `tau/clients/p2p/auth/` |

### SDK Files:
| SDK | Path | Description |
|-----|------|-------------|
| Rust | `rust-sdk/` | Rust SDK for WASM functions |
| Go | `go-sdk/` | Go SDK for WASM functions |

---

## Next Steps for Deep Dives

See the following documents for detailed explorations:

1. **architecture-deep-dive.md** - Complete architecture analysis
2. **subsystems/monkey.md** - Function execution service
3. **subsystems/patrick.md** - Build scheduler service
4. **subsystems/seer.md** - DNS and discovery service
5. **subsystems/hoarder.md** - Storage service
6. **subsystems/auth.md** - Authentication service
7. **production-grade.md** - Production considerations
8. **rust-revision.md** - Rust implementation guide
