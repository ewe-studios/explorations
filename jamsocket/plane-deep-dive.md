# Plane: Session Backend Orchestration

Plane is the core orchestration system behind Jamsocket. It manages the lifecycle of session backends, handles routing, and provides the infrastructure for real-time applications.

## Overview

**Plane** is a Rust-based system for orchestrating stateful session backends. It handles:
- Backend spawning and termination
- Docker container management
- WebSocket routing and proxying
- TLS certificate management
- Key-based session affinity

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Controller                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │   HTTP API   │  │  WebSocket   │  │  PostgreSQL  │              │
│  │  (Axum)      │  │  Listeners   │  │   Database   │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
└─────────────────────────────────────────────────────────────────────┘
         │                    │                    │
         │                    │                    │
    ┌────▼────┐         ┌────▼────┐         ┌─────▼─────┐
    │  Proxy  │         │  Drone  │         │ DNS Server│
    │ (Rust)  │         │ (Rust)  │         │  (Rust)   │
    └────┬────┘         └────┬────┘         └───────────┘
         │                   │
         │                   │
    ┌────▼────┐         ┌────▼────┐
    │ Clients │         │ Docker  │
    │  (WS)   │         │Containers│
    └─────────┘         └─────────┘
```

## Components

### 1. Controller

The controller is the central hub of Plane. It:
- Maintains the global state of all backends
- Handles spawn requests via HTTP API
- Communicates with drones via WebSocket
- Manages key-based routing
- Coordinates certificate issuance

**Key Files:**
- `plane/src/bin/controller.rs` - Main controller binary
- `plane/src/controller/mod.rs` - Controller logic
- `plane/src/database/mod.rs` - Database operations

**Database Schema:**
- `backend` - Backend state and metadata
- `key` - Key/lock management
- `drone` - Registered drones
- `proxy` - Registered proxies
- `certificate` - TLS certificate state

### 2. Drone

Drones run on machines that host session backends:

```rust
// Drone connects to controller via WebSocket
let mut socket = controller_socket.connect().await?;

// Drone receives commands
while let Some(action) = socket.recv().await {
    match action {
        BackendAction::Spawn { executable, key } => {
            spawn_backend(executable, key).await?;
        }
        BackendAction::Terminate { kind, reason } => {
            terminate_backend(kind, reason).await?;
        }
    }
}
```

**Responsibilities:**
- Connect to controller WebSocket
- Receive spawn/terminate commands
- Manage Docker containers via Docker API
- Report backend state changes
- Report metrics (CPU, memory)
- Handle key renewal

**Key Files:**
- `plane/src/bin/drone.rs` - Main drone binary
- `plane/src/drone/mod.rs` - Drone logic
- `plane/src/drone/backend_manager.rs` - Backend lifecycle

### 3. Proxy

Proxies handle external traffic:

```
Client ──TLS──> Proxy ──HTTP/WS──> Backend (Drone)
```

**Responsibilities:**
- TLS termination
- HTTP/WebSocket routing
- Token validation
- Connection tracking
- Certificate management (ACME)

**Key Files:**
- `plane/src/bin/proxy.rs` - Main proxy binary
- `dynamic-proxy/src/proxy.rs` - Proxy logic
- `dynamic-proxy/src/upgrade.rs` - WebSocket upgrade handling

### 4. DNS Server

The DNS server supports ACME DNS-01 challenges:

**Responsibilities:**
- Respond to DNS TXT queries
- Support ACME challenge validation
- Coordinate with controller for certificate renewal

## Protocol

### Typed Socket

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

### Message Types

**Drone <-> Controller:**

```rust
pub enum MessageFromDrone {
    Heartbeat(Heartbeat),
    BackendEvent(BackendStateMessage),
    BackendMetrics(BackendMetricsMessage),
    AckAction { action_id: BackendActionName },
    RenewKey(RenewKeyRequest),
}

pub enum MessageToDrone {
    Action(BackendActionMessage),
    AckEvent { event_id: BackendEventId },
    RenewKeyResponse(RenewKeyResponse),
}
```

**Proxy <-> Controller:**

```rust
pub enum MessageFromProxy {
    RouteInfoRequest(RouteInfoRequest),
    KeepAlive(BackendName),
    CertManagerRequest(CertManagerRequest),
}

pub enum MessageToProxy {
    RouteInfoResponse(RouteInfoResponse),
    CertManagerResponse(CertManagerResponse),
    BackendRemoved { backend: BackendName },
}
```

### Handshake

```rust
pub struct Handshake {
    pub version: PlaneVersionInfo,
    pub name: String,
}
```

Components check version compatibility on connection.

## Backend Lifecycle

### State Machine

```
Scheduled (10)
    │
    ▼
Loading (20)
    │
    ▼
Starting (30)
    │
    ▼
Waiting (40) ──address available──> Ready (50)
    │                                │
    │                                ▼
    │                          Terminating (60)
    │                                │
    ▼                                ▼
HardTerminating (65) <───────────────┘
    │
    ▼
Terminated (70)
```

### State Transitions

```rust
impl BackendState {
    pub fn to_loading(&self) -> BackendState;
    pub fn to_starting(&self) -> BackendState;
    pub fn to_waiting(&self, address: SocketAddr) -> BackendState;
    pub fn to_ready(&self, address: BackendAddr) -> BackendState;
    pub fn to_terminating(&self, reason: TerminationReason) -> BackendState;
    pub fn to_hard_terminating(&self, reason: TerminationReason) -> BackendState;
    pub fn to_terminated(&self, exit_code: Option<i32>) -> BackendState;
}
```

### Termination Reasons

```rust
pub enum TerminationReason {
    Swept,           // Idle timeout
    External,        // User request
    KeyExpired,      // Key deadline passed
    Lost,            // Drone lost connection
    StartupTimeout,  // Backend didn't start in time
    InternalError,   // System error
}
```

## Key Management

### Key Deadlines

```rust
pub struct KeyDeadlines {
    pub renew_at: LoggableTime,         // When to attempt renewal
    pub soft_terminate_at: LoggableTime, // When to soft terminate
    pub hard_terminate_at: LoggableTime, // When to hard terminate
}
```

### Key Acquisition

```rust
pub struct AcquiredKey {
    pub key: KeyConfig,
    pub deadlines: KeyDeadlines,
    pub token: i64,  // Fencing token
}
```

The token serves as a fencing token to prevent stale operations:
> "How to do distributed locking" - Martin Kleppmann

### Key Renewal Flow

1. Backend sends `RenewKeyRequest` to drone
2. Drone forwards to controller
3. Controller updates deadlines
4. Controller sends `RenewKeyResponse` to drone
5. Drone updates backend's key deadlines

## Routing

### RouteInfo

```rust
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

### Connection Flow

1. Client requests connection from controller
2. Controller creates bearer token, returns tokenized URL
3. Client connects to proxy with token
4. Proxy sends `RouteInfoRequest(token)` to controller
5. Controller validates token, returns `RouteInfoResponse`
6. Proxy routes connection to backend

### Token Validation

```rust
pub struct RouteInfoRequest {
    pub token: BearerToken,
}

pub struct RouteInfoResponse {
    pub token: BearerToken,
    pub route_info: Option<RouteInfo>,
}
```

## Dynamic Proxy

The `dynamic-proxy` crate provides the HTTP/WebSocket proxy:

### Proxy Client

```rust
pub struct ProxyClient {
    client: Client<TimeoutHttpConnector, SimpleBody>,
    timeout: Duration,
}

impl ProxyClient {
    pub async fn request(
        &self,
        request: Request<SimpleBody>,
    ) -> Result<(Response<SimpleBody>, Option<UpgradeHandler>), Infallible>
}
```

### WebSocket Upgrade

```rust
async fn handle_upgrade(
    &self,
    request: Request<SimpleBody>,
) -> Result<(Response<SimpleBody>, UpgradeHandler), ProxyError> {
    let (upstream_request, request_with_body) = split_request(request);
    let res = self.upstream_request(upstream_request).await?;
    let (upstream_response, response_with_body) = split_response(res);
    let upgrade_handler = UpgradeHandler::new(request_with_body, response_with_body);
    Ok((upstream_response, upgrade_handler))
}
```

### Graceful Shutdown

The proxy supports graceful connection draining:
- Stop accepting new connections
- Wait for existing connections to close
- Timeout after configured duration

## Configuration

### Environment Variables

- `CONTROLLER_HOST` - Controller bind address
- `CONTROLLER_PORT` - Controller port
- `DATABASE_URL` - PostgreSQL connection string
- `DRONE_USE_IP` - Use IP instead of hostname for backends

### Command Line

```bash
# Controller
plane controller --port 8080 --database postgres://...

# Drone
plane drone --name drone-1 --controller http://controller:8080

# Proxy
plane proxy --name proxy-1 --controller http://controller:8080

# DNS
plane dns --port 53 --controller http://controller:8080
```

## Dependencies

### Core Dependencies

```toml
axum = "0.7.7"           # Web framework
bollard = "0.17.0"       # Docker API
sqlx = "0.8.2"           # PostgreSQL
tokio = "1.33.0"         # Async runtime
tokio-tungstenite = "0.24.0"  # WebSocket
serde = "1.0.190"        # Serialization
tracing = "0.1.40"       # Logging
```

### Key Crates

- `plane-common` - Shared types and protocol
- `dynamic-proxy` - HTTP/WebSocket proxy
- `plane` - Main binary

## Testing

### Integration Tests

Plane includes integration tests in `plane/plane-tests`:

```rust
#[plane_test]
async fn test_backend_lifecycle(client: PlaneClient) {
    // Spawn a backend
    let backend = client.spawn("my-service").await?;

    // Wait for it to be ready
    client.wait_for_backend(&backend, BackendStatus::Ready).await?;

    // Connect and verify
    let connection = client.connect(&backend).await?;
    assert!(connection.is_open());

    // Terminate
    client.terminate(&backend).await?;
}
```

## Production Deployment

### Requirements

- PostgreSQL 14+
- Docker 20.10+
- Reverse proxy (nginx, Caddy, Envoy)
- DNS server (for ACME)

### Kubernetes

Plane can be deployed on Kubernetes:
- Controller as StatefulSet
- Drones as DaemonSet or Deployment
- Proxies as Deployment with LoadBalancer
- DNS as Deployment

### Docker Compose

```yaml
version: '3'
services:
  postgres:
    image: postgres:14
  controller:
    image: jamsocket/plane
    command: plane controller
    depends_on: [postgres]
  drone:
    image: jamsocket/plane
    command: plane drone
    volumes: [/var/run/docker.sock:/var/run/docker.sock]
  proxy:
    image: jamsocket/plane
    command: plane proxy
    ports: [443:443]
```

## Debugging

### Logs

Plane uses structured logging with tracing:

```bash
# Set log level
RUST_LOG=debug plane controller

# Filter by module
RUST_LOG=plane::controller=debug,plane::drone=info
```

### Metrics

Drones report:
- Memory used (excluding inactive file cache)
- Memory total
- Memory active/inactive/unevictable
- CPU nanoseconds used

### Status Endpoints

- `GET /status` - Health check
- `GET /status/json` - JSON status with version info

## Security Considerations

### API Partitioning

- `/pub/*` - Public API (no authentication needed for basic operations)
- `/ctrl/*` - Control API (requires authentication)

### Token Security

- Bearer tokens for routing
- Secret tokens for backend authentication
- Tokens expire when backend terminates

### Network Isolation

- Controllers should not be directly exposed
- Proxies handle TLS termination
- Drones only accept connections from controller
