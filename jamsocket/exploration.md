# Jamsocket: Serverless WebRTC/WebSocket Infrastructure

## Overview

Jamsocket is a platform for building real-time, stateful applications using session backends. It provides infrastructure for spawning, managing, and routing connections to stateful backend processes that run in isolated containers.

**Core Philosophy:** Instead of stateless backends that share data through a database, session backends keep state in memory and provide low-latency, real-time interactions for connected clients.

## Architecture Summary

Jamsocket's architecture is built around several key projects:

```
jamsocket/           # Main platform CLI and TypeScript SDKs
├── cli/             # TypeScript CLI for managing services
├── packages/        # TypeScript client libraries
└── examples/        # Example applications

plane/               # Core orchestration system (Rust)
├── common/          # Shared types and protocol definitions
├── dynamic-proxy/   # HTTP/WebSocket reverse proxy
├── plane/           # Main binary (controller, drone, proxy, DNS)
└── docs/            # Documentation

stateroom/           # WASM-based session backend framework
├── stateroom/       # Core trait definitions
├── stateroom-wasm/  # WASM guest library
├── stateroom-wasm-host/  # WASM host runtime (wasmtime)
├── stateroom-server/     # Axum-based WebSocket server
└── stateroom-cli/   # CLI for building/serving WASM services

y-sweet/             # CRDT-based real-time collaboration
├── crates/y-sweet/       # Standalone Yjs server
├── crates/y-sweet-core/  # Core sync logic
├── crates/y-sweet-worker/# Cloudflare Workers build
└── debugger/        # Yjs document debugger UI

driftdb/             # Real-time database with WebSocket sync
├── driftdb/         # Core database logic
├── driftdb-server/  # HTTP server
└── driftdb-worker/  # Cloudflare Workers build

wasmbox/             # Lightweight WASM service framework
├── wasmbox/         # Core traits
├── wasmbox-host/    # Host runtime
└── wasmbox-cli/     # CLI tooling
```

## Key Concepts

### Session Backends

Session backends are stateful server processes bound to the life of a user's interaction session:

- **Stateful**: Keep data in memory, not in a database
- **Isolated**: Each session runs in its own container
- **Temporary**: Live only as long as needed
- **Real-time**: Direct WebSocket connections to clients

**Use Cases:**
- Multiplayer collaboration (shared documents, whiteboards)
- Real-time games
- Interactive development environments
- AI agent sessions
- Large dataset processing in memory

### Backend Lifecycle

```
Scheduled -> Loading -> Starting -> Waiting -> Ready -> Terminating -> Terminated
                                                      -> HardTerminating
```

States:
- **Scheduled**: Backend assigned to a drone
- **Loading**: Docker image being pulled
- **Starting**: Container starting
- **Waiting**: Container ready, waiting for health check
- **Ready**: Accepting connections
- **Terminating**: SIGTERM sent, graceful shutdown
- **HardTerminating**: SIGKILL sent
- **Terminated**: Process exited

### Key-Based Session Routing

Jamsocket uses a "key" (formerly "lock") system for session affinity:

```typescript
// Client requests a session with a specific key
const result = await jamsocket.connect('my-service', {
  key: 'user-123'  // All clients with this key connect to same backend
});
```

This enables:
- Multiple clients joining the same session
- Session persistence across page reloads
- Automatic routing to existing sessions

## Components

### 1. Controller

The central orchestration hub:
- Maintains PostgreSQL database of all backends
- Handles spawn requests
- Routes key-based connections
- Manages backend lifecycle
- Coordinates certificate issuance (ACME)

**API Endpoints:**
- `/pub/*` - Public API (untrusted clients)
- `/ctrl/*` - Control API (trusted components)

### 2. Drone

Runs on machines that host session backends:
- Connects to controller via WebSocket
- Receives commands to spawn/terminate backends
- Reports backend state and metrics
- Manages Docker containers locally

### 3. Proxy

Routes external traffic to backends:
- TLS termination
- HTTP/WebSocket routing
- Token-based authentication
- ACME certificate management
- Connection tracking for lifecycle

### 4. DNS Server

Supports ACME DNS-01 challenges:
- TXT record management
- Certificate renewal coordination

## WebSocket Routing and Relay

### Token-Based Routing

```rust
// RouteInfo contains routing information
pub struct RouteInfo {
    pub backend_id: BackendName,
    pub address: BackendAddr,
    pub secret_token: SecretToken,
    pub cluster: ClusterName,
    pub user: Option<String>,
    pub user_data: Option<serde_json::Value>,
    pub subdomain: Option<Subdomain>,
}
```

**Flow:**
1. Client requests connection from controller
2. Controller creates bearer token
3. Client connects to proxy with token
4. Proxy validates token with controller
5. Controller returns RouteInfo
6. Proxy routes to backend

### Typed Socket Pattern

Plane uses a typed message protocol over WebSockets:

```rust
pub trait ChannelMessage: Send + Sync + 'static + DeserializeOwned + Serialize + Debug {
    type Reply: ChannelMessage<Reply = Self>;
}

pub struct TypedSocket<T: ChannelMessage> {
    send: Sender<SocketAction<T>>,
    recv: Receiver<T::Reply>,
    pub remote_handshake: Handshake,
}
```

**Message Types:**
- `MessageFromDrone`: Heartbeat, BackendEvent, BackendMetrics, AckAction, RenewKey
- `MessageToDrone`: Action, AckEvent, RenewKeyResponse
- `MessageFromProxy`: RouteInfoRequest, KeepAlive, CertManagerRequest
- `MessageToProxy`: RouteInfoResponse, CertManagerResponse, BackendRemoved

### Dynamic Proxy

The proxy handles HTTP upgrade to WebSocket:

```rust
pub async fn request(
    &self,
    request: Request<SimpleBody>,
) -> Result<(Response<SimpleBody>, Option<UpgradeHandler>), Infallible>
```

Features:
- Graceful connection draining
- HTTP/1.1 and HTTP/2 support
- WebSocket upgrade handling
- Timeout management

## Session Management System

### Key Deadlines

```rust
pub struct KeyDeadlines {
    pub renew_at: LoggableTime,         // When to renew
    pub soft_terminate_at: LoggableTime, // When to soft terminate
    pub hard_terminate_at: LoggableTime, // When to hard terminate
}
```

### Backend Actions

```rust
pub enum BackendAction {
    Spawn {
        executable: Value,
        key: AcquiredKey,
        static_token: Option<BearerToken>,
    },
    Terminate {
        kind: TerminationKind,
        reason: TerminationReason,
    },
}
```

### Connection Tracking

Proxies report active connections to the controller. The controller periodically sweeps backends that:
- Have had no connections for the idle timeout
- Have passed their key expiration deadlines
- Have been explicitly terminated

## WASM Usage Patterns

### Web Environment

In web environments, WASM modules run in the browser:
- Client-side computation
- Local state management
- Direct DOM access (when needed)

### Backend/Server Environment

Jamsocket uses WASM for session backends:

**Stateroom Pattern:**
```rust
// Guest module (stateroom-wasm)
#[stateroom_wasm]
#[derive(Default)]
struct ChatServer {
    client_to_nickname: HashMap<ClientId, String>,
}

impl StateroomService for ChatServer {
    fn connect(&mut self, client: ClientId, ctx: &impl StateroomContext) {
        ctx.send_message(client, "Welcome!");
    }

    fn message(&mut self, client: ClientId, message: MessagePayload, ctx: &impl StateroomContext) {
        ctx.send_message(MessageRecipient::Broadcast, message);
    }
}
```

**Host Runtime (stateroom-wasm-host):**
- Uses wasmtime for execution
- Provides external functions:
  - `stateroom_send`: Send messages to clients
  - `stateroom_recv`: Receive messages from clients
  - `stateroom_malloc/Free`: Memory management
- Version checking via globals:
  - `STATEROOM_API_VERSION`
  - `STATEROOM_API_PROTOCOL`

### WASM Memory Model

```rust
// Host allocates memory for messages
fn put_data(&mut self, data: &[u8]) -> Result<(u32, u32)> {
    let len = data.len() as u32;
    let pt = self.fn_malloc.call(&mut self.store, len)?;
    self.memory.write(&mut self.store, pt as usize, data)?;
    Ok((pt, len))
}

// Guest receives serialized messages
fn try_recv(&mut self, message: MessageToProcess) -> Result<()> {
    let payload = bincode::serialize(&message).unwrap();
    let (pt, len) = self.put_data(&payload)?;
    self.fn_recv.call(&mut self.store, (pt, len))?;
    self.fn_free.call(&mut self.store, (pt, len))?;
    Ok(())
}
```

## Performance Characteristics

### Design Decisions

1. **Container Isolation**: Each session backend runs in its own Docker container
   - Pros: Complete isolation, resource limits, security
   - Cons: Higher overhead than threads/processes

2. **WebSocket Communication**: All client-backend communication via WebSocket
   - Pros: Full-duplex, low latency, standard protocol
   - Cons: Connection state must be maintained

3. **Centralized Controller**: Single source of truth for backend state
   - Pros: Consistent view, easier reasoning
   - Cons: Potential bottleneck (mitigated by read replicas)

4. **Token-Based Routing**: Stateless proxies with token lookup
   - Pros: Horizontal scaling of proxies
   - Cons: Extra round-trip for initial connection

### Resource Management

- **Memory Tracking**: Drones report memory usage per backend
- **CPU Tracking**: Nanoseconds of CPU used since last message
- **Idle Sweeping**: Backends terminated after idle timeout
- **Key Expiration**: Automatic termination when keys expire

### Scaling Considerations

- **Controller**: Can be replicated with shared database
- **Proxies**: Fully stateless, horizontal scaling
- **Drones**: Each drone manages its own backends
- **Keys**: Distributed lock management for session affinity

## Comparison to Alternatives

| Feature | Jamsocket | Traditional Backend | Serverless Functions |
|---------|-----------|---------------------|----------------------|
| State | In-memory, per-session | Database/shared | Stateless |
| Latency | Low (direct connection) | Medium (DB roundtrip) | High (cold start) |
| Scaling | Per-container | Horizontal | Automatic |
| Cost | Per-running session | Per-instance | Per-invocation |
| Use Case | Real-time, collaborative | CRUD, APIs | Event-driven |

## Related Projects

### Y-Sweet
- Yjs CRDT server for real-time collaboration
- Supports S3/R2 storage for persistence
- Cloudflare Workers deployment option
- Built-in awareness/cursor tracking

### Driftdb
- Real-time database with WebSocket sync
- Key-value store with sequence numbers
- Supports Relay, Replace, Append, Compact actions
- Cloudflare Workers deployment

### Aper
- State synchronization library
- Works with Stateroom
- Provides atomic data structures
- Listener pattern for state changes
