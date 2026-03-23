# Reproducing Jamsocket in Rust: A Production Guide

This guide explains how to reproduce Jamsocket's functionality in Rust at production level.

## System Overview

Jamsocket consists of several components that work together:

```
┌─────────────────────────────────────────────────────────────────┐
│                      Production System                          │
│                                                                  │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │   Client    │───>│    Proxy    │───>│   Backend   │        │
│  │  (Browser)  │    │  (Rust)     │    │ (Container) │        │
│  └─────────────┘    └─────────────┘    └─────────────┘        │
│        │                  │                  │                 │
│        │                  │                  │                 │
│        ▼                  ▼                  ▼                 │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                    Controller                            │  │
│  │  - PostgreSQL database                                   │  │
│  │  - WebSocket to Drones                                   │  │
│  │  - HTTP API for spawning                                 │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                      Drone                               │  │
│  │  - WebSocket to Controller                               │  │
│  │  - Docker API for containers                             │  │
│  │  - Reports metrics                                       │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components to Implement

### 1. Controller Service

The controller is the central orchestration hub.

```rust
// Cargo.toml dependencies
[dependencies]
axum = { version = "0.7", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "chrono"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tower-http = { version = "0.6", features = ["cors", "trace"] }
```

**Key Endpoints:**

```rust
use axum::{
    routing::{get, post},
    Router,
    extract::{State, WebSocketUpgrade, ws::Message},
};

pub struct ControllerState {
    db: PgPool,
    drones: DashMap<NodeId, DroneConnection>,
    backends: DashMap<BackendName, BackendState>,
}

pub fn create_router(state: ControllerState) -> Router {
    Router::new()
        // Health check
        .route("/status", get(status_handler))

        // Public API
        .route("/pub/connect/:service", post(connect_handler))
        .route("/pub/spawn/:service", post(spawn_handler))

        // Control API (requires auth)
        .route("/ctrl/drone", post(register_drone))
        .route("/ctrl/proxy", post(register_proxy))
        .route("/ctrl/backend/:id/state", post(backend_state_update))

        // WebSocket endpoints
        .route("/ctrl/drone/ws", get(drone_ws_handler))
        .route("/ctrl/proxy/ws", get(proxy_ws_handler))

        .with_state(state)
}

async fn status_handler() -> &'static str {
    "ok"
}
```

**Backend Spawning:**

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SpawnRequest {
    pub service: String,
    pub key: Option<String>,  // For session affinity
    pub cluster: Option<String>,
    pub env: HashMap<String, String>,
    pub max_idle_seconds: Option<u32>,
    pub lifetime_limit_seconds: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct SpawnResponse {
    pub backend_id: BackendName,
    pub token: BearerToken,
    pub url: String,
}

async fn spawn_handler(
    State(state): State<ControllerState>,
    Json(req): Json<SpawnRequest>,
) -> Result<Json<SpawnResponse>, ApiError> {
    // 1. Find or create backend for key
    let backend_id = if let Some(key) = &req.key {
        state.get_or_create_backend_for_key(key, &req.service).await?
    } else {
        state.create_backend(&req.service).await?
    };

    // 2. Generate bearer token for routing
    let token = generate_bearer_token();

    // 3. Store routing info
    state.store_route_info(&backend_id, &token, &req).await?;

    // 4. Return connection URL
    let url = format!("wss://proxy.jamsocket.dev/connect/{}", token);

    Ok(Json(SpawnResponse {
        backend_id,
        token,
        url,
    }))
}
```

**Drone Communication:**

```rust
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

pub enum DroneMessage {
    Spawn {
        backend_id: BackendName,
        image: String,
        env: HashMap<String, String>,
        port: u16,
    },
    Terminate {
        backend_id: BackendName,
        kind: TerminationKind,
    },
    RenewKey {
        backend_id: BackendName,
        deadlines: KeyDeadlines,
    },
}

async fn drone_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ControllerState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_drone_socket(socket, state))
}

async fn handle_drone_socket(socket: WebSocket, state: ControllerState) {
    let (mut send, mut recv) = socket.split();

    // Register drone and get NodeId
    let drone_id = state.register_drone().await;

    // Spawn task to send messages to drone
    let send_task = {
        let mut rx = state.subscribe_to_drone_messages(drone_id);
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let ws_msg = serde_json::to_string(&msg).unwrap();
                send.send(Message::Text(ws_msg)).await.unwrap();
            }
        })
    };

    // Handle messages from drone
    while let Some(msg) = recv.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let drone_msg: DroneToController = serde_json::from_str(&text).unwrap();
                state.handle_drone_message(drone_id, drone_msg).await;
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
        }
    }

    send_task.abort();
    state.unregister_drone(drone_id).await;
}
```

### 2. Drone Service

The drone runs on machines that host backends.

```rust
// Cargo.toml dependencies
[dependencies]
bollard = "0.17"  // Docker API
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.24"
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
```

**Drone Implementation:**

```rust
use bollard::{
    Docker,
    container::{Config, CreateContainerOptions, StartContainerOptions},
    image::CreateImageOptions,
};

pub struct Drone {
    docker: Docker,
    controller_url: String,
    drone_id: Option<NodeId>,
    backends: DashMap<BackendName, ContainerHandle>,
}

impl Drone {
    pub async fn connect(&mut self) -> Result<()> {
        let mut socket = connect_to_controller(&self.controller_url).await?;

        // Send handshake
        let handshake = DroneHandshake {
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        socket.send(serde_json::to_string(&handshake)?).await?;

        // Main loop
        while let Some(msg) = socket.next().await {
            match msg? {
                Message::Text(text) => {
                    let action: DroneAction = serde_json::from_str(&text)?;
                    self.handle_action(action).await?;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_action(&self, action: DroneAction) -> Result<()> {
        match action {
            DroneAction::Spawn { backend_id, image, env, port } => {
                self.spawn_backend(backend_id, image, env, port).await?;
            }
            DroneAction::Terminate { backend_id, kind } => {
                self.terminate_backend(backend_id, kind).await?;
            }
        }
        Ok(())
    }

    async fn spawn_backend(
        &self,
        backend_id: BackendName,
        image: String,
        env: HashMap<String, String>,
        port: u16,
    ) -> Result<()> {
        // Pull image if needed
        self.docker
            .create_image(Some(CreateImageOptions {
                from_image: &image,
                ..Default::default()
            }), None)
            .await?;

        // Create container
        let config = Config {
            image: Some(&image),
            env: Some(env.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect()),
            exposed_ports: Some([(format!("{}/tcp", port), Default::default())].into_iter().collect()),
            host_config: Some(HostConfig {
                port_bindings: Some({
                    let mut map = HashMap::new();
                    map.insert(format!("{}/tcp", port), Some(vec![PortBinding {
                        host_ip: Some("0.0.0.0"),
                        host_port: Some("0"),  // Dynamic port
                    }]));
                    map
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let container = self.docker
            .create_container(Some(CreateContainerOptions {
                name: format!("backend-{}", backend_id),
                platform: None,
            }), config)
            .await?;

        // Start container
        self.docker
            .start_container(&container.id, None::<StartContainerOptions>)
            .await?;

        // Get assigned port
        let info = self.docker.inspect_container(&container.id, None).await?;
        let assigned_port = info
            .network_settings
            .and_then(|ns| ns.ports)
            .and_then(|ports| ports.get(&format!("{}/tcp", port)))
            .and_then(|bindings| bindings.as_ref().and_then(|b| b.first()))
            .and_then(|b| b.host_port.as_ref())
            .and_then(|p| p.parse().ok())
            .ok_or("Could not determine assigned port")?;

        // Report backend state to controller
        self.report_state(BackendState::Waiting {
            address: BackendAddr::new("127.0.0.1", assigned_port),
        }).await?;

        self.backends.insert(backend_id, ContainerHandle {
            id: container.id,
            port: assigned_port,
        });

        Ok(())
    }

    async fn terminate_backend(
        &self,
        backend_id: BackendName,
        kind: TerminationKind,
    ) -> Result<()> {
        if let Some(handle) = self.backends.remove(&backend_id) {
            match kind {
                TerminationKind::Soft => {
                    // Send SIGTERM
                    self.docker.stop_container(&handle.id, None).await?;
                }
                TerminationKind::Hard => {
                    // Send SIGKILL
                    self.docker.kill_container(&handle.id, None).await?;
                }
            }
        }
        Ok(())
    }
}
```

### 3. Proxy Service

The proxy routes external traffic to backends.

```rust
// Cargo.toml dependencies
[dependencies]
axum = { version = "0.7", features = ["ws"] }
hyper = { version = "1", features = ["full"] }
hyper-util = { version = "0.1", features = ["client", "http1", "http2"] }
tokio = { version = "1", features = ["full"] }
tower = "0.5"
rustls = "0.23"
acme2-eab = "0.5"  # For ACME certificate management
```

**Proxy Implementation:**

```rust
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

pub struct Proxy {
    controller_url: String,
    routes: DashMap<BearerToken, RouteInfo>,
    client: Client<HttpConnector, Body>,
}

impl Proxy {
    pub async fn connect(&mut self) -> Result<()> {
        let mut socket = connect_to_controller(&self.controller_url).await?;

        // Register proxy
        let proxy_id = self.register_with_controller(&mut socket).await?;

        // Main loop
        while let Some(msg) = socket.next().await {
            match msg? {
                Message::Text(text) => {
                    let msg: ControllerToProxy = serde_json::from_str(&text)?;
                    self.handle_message(msg).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_request(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, Infallible> {
        // Extract token from path or header
        let token = extract_token(&req);

        // Get route info
        let route_info = match self.routes.get(&token) {
            Some(info) => info.clone(),
            None => {
                // Fetch from controller
                match self.fetch_route_info(token.clone()).await? {
                    Some(info) => info,
                    None => return Ok(not_found()),
                }
            }
        };

        // Proxy to backend
        match self.proxy_to_backend(req, &route_info).await {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::warn!(?e, "Proxy error");
                Ok(bad_gateway())
            }
        }
    }

    async fn proxy_to_backend(
        &self,
        mut req: Request<Body>,
        route_info: &RouteInfo,
    ) -> Result<Response<Body>> {
        // Rewrite URI
        let backend_url = format!("http://{}:{}", route_info.address.ip(), route_info.address.port());
        let uri = format!("{}{}", backend_url, req.uri().path());
        *req.uri_mut() = uri.parse()?;

        // Add authentication header
        req.headers_mut().insert(
            "X-Backend-Token",
            route_info.secret_token.clone().parse()?,
        );

        // Forward request
        let response = self.client.request(req).await?;
        Ok(response)
    }

    async fn handle_websocket(
        &self,
        ws: WebSocketUpgrade,
        token: BearerToken,
    ) -> Response {
        ws.on_upgrade(move |socket| self.relay_websocket(socket, token))
    }

    async fn relay_websocket(&self, client_ws: WebSocket, token: BearerToken) {
        let route_info = match self.fetch_route_info(token).await.unwrap() {
            Some(info) => info,
            None => return,
        };

        // Connect to backend
        let backend_url = format!("ws://{}:{}/ws", route_info.address.ip(), route_info.address.port());
        let (backend_ws, _) = connect_async(&backend_url).await.unwrap();
        let (mut backend_send, mut backend_recv) = backend_ws.split();
        let (mut client_send, mut client_recv) = client_ws.split();

        // Relay messages both directions
        let client_to_backend = async {
            while let Some(msg) = client_recv.next().await {
                if let Ok(msg) = msg {
                    backend_send.send(msg).await.unwrap();
                }
            }
        };

        let backend_to_client = async {
            while let Some(msg) = backend_recv.next().await {
                if let Ok(msg) = msg {
                    client_send.send(msg).await.unwrap();
                }
            }
        };

        tokio::select! {
            _ = client_to_backend => {}
            _ = backend_to_client => {}
        }
    }
}
```

### 4. Session Backend Framework

A framework for building session backends.

```rust
// Cargo.toml dependencies
[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.24"
axum = { version = "0.7", features = ["ws"] }
serde = { version = "1", features = ["derive"] }
dashmap = "6.1"
```

**Service Trait:**

```rust
use std::sync::Arc;
use tokio::sync::broadcast;

pub trait SessionService: Send + Sync + 'static {
    type State: Default + Send + Sync;

    async fn connect(state: &Arc<Self::State>, client_id: ClientId);
    async fn message(state: &Arc<Self::State>, client_id: ClientId, message: String);
    async fn disconnect(state: &Arc<Self::State>, client_id: ClientId);
}
```

**Service Host:**

```rust
pub struct ServiceHost<S: SessionService> {
    state: Arc<S::State>,
    clients: DashMap<ClientId, broadcast::Sender<String>>,
    next_client_id: AtomicU32,
}

impl<S: SessionService> ServiceHost<S> {
    pub fn new() -> Self {
        Self {
            state: Arc::new(S::State::default()),
            clients: DashMap::new(),
            next_client_id: AtomicU32::new(1),
        }
    }

    pub async fn handle_socket(&self, socket: WebSocket) {
        let client_id = ClientId(self.next_client_id.fetch_add(1, Ordering::Relaxed));
        let (tx, mut rx) = broadcast::channel(100);
        self.clients.insert(client_id, tx);

        let (mut send, mut recv) = socket.split();
        let state = self.state.clone();

        // Notify service of connection
        S::connect(&state, client_id).await;

        // Send task
        let send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if send.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Receive task
        while let Some(msg) = recv.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    S::message(&state, client_id, text).await;
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }

        send_task.abort();
        self.clients.remove(&client_id);
        S::disconnect(&state, client_id).await;
    }
}
```

### 5. Key-Based Session Affinity

Implement key-based routing for session affinity.

```rust
use sqlx::PgPool;
use chrono::{DateTime, Utc, Duration};

pub struct KeyManager {
    db: PgPool,
}

impl KeyManager {
    pub async fn acquire_key(
        &self,
        service: &str,
        key: &str,
        existing_backend: Option<BackendName>,
    ) -> Result<AcquiredKey> {
        // Check if key is already held
        if let Some(backend) = self.get_backend_for_key(service, key).await? {
            if backend.is_healthy() {
                return Ok(AcquiredKey {
                    backend: backend.id,
                    token: backend.token,
                    deadlines: backend.deadlines,
                });
            }
        }

        // Need to spawn new backend
        let backend = self.spawn_backend_for_key(service, key).await?;

        // Calculate deadlines
        let now = Utc::now();
        let deadlines = KeyDeadlines {
            renew_at: now + Duration::seconds(30),
            soft_terminate_at: now + Duration::seconds(300),
            hard_terminate_at: now + Duration::seconds(360),
        };

        Ok(AcquiredKey {
            backend: backend.id,
            token: backend.token,
            deadlines,
        })
    }

    pub async fn renew_key(
        &self,
        service: &str,
        key: &str,
        backend_id: BackendName,
    ) -> Result<Option<KeyDeadlines>> {
        // Verify key is still held by this backend
        let current = self.get_backend_for_key(service, key).await?;
        if current.map(|b| b.id) != Some(backend_id) {
            return Ok(None);  // Key no longer held
        }

        // Update deadlines
        let deadlines = self.calculate_deadlines().await;
        self.update_key_deadlines(key, &deadlines).await?;

        Ok(Some(deadlines))
    }
}
```

## WASM Integration

For running WASM-based services like Stateroom:

```rust
// Cargo.toml dependencies
[dependencies]
wasmtime = "14"
wasi-common = "14"
bincode = "1.3"
```

**WASM Host:**

```rust
use wasmtime::{Engine, Module, Store, Linker};
use wasi_common::sync::WasiCtxBuilder;

pub struct WasmServiceHost {
    engine: Engine,
    module: Module,
}

impl WasmServiceHost {
    pub fn new(wasm_path: &str) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, wasm_path)?;
        Ok(Self { engine, module })
    }

    pub fn instantiate(&self, room_id: &str, context: Arc<ServiceContext>) -> Result<WasmInstance> {
        let wasi = WasiCtxBuilder::new().inherit_stdio().build();
        let mut store = Store::new(&self.engine, wasi);

        let mut linker = Linker::new(&self.engine);
        wasi_common::sync::add_to_linker(&mut linker, |s| s)?;

        // Add service-specific functions
        linker.func_wrap("env", "service_send", move |ptr, len| {
            // Send message to clients
            context.send(ptr, len);
            Ok(0)
        })?;

        let instance = linker.instantiate(&mut store, &self.module)?;

        Ok(WasmInstance {
            store,
            instance,
            room_id: room_id.to_string(),
        })
    }
}
```

## Production Considerations

### Database Schema

```sql
-- Backends table
CREATE TABLE backends (
    id TEXT PRIMARY KEY,
    service TEXT NOT NULL,
    drone_id TEXT REFERENCES drones(id),
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Keys table (for session affinity)
CREATE TABLE keys (
    service TEXT NOT NULL,
    key TEXT NOT NULL,
    backend_id TEXT REFERENCES backends(id),
    token BIGINT NOT NULL,
    renew_at TIMESTAMPTZ,
    soft_terminate_at TIMESTAMPTZ,
    hard_terminate_at TIMESTAMPTZ,
    PRIMARY KEY (service, key)
);

-- Drones table
CREATE TABLE drones (
    id TEXT PRIMARY KEY,
    last_heartbeat TIMESTAMPTZ,
    capacity INT,
    available_capacity INT
);

-- Route info (for proxy lookup)
CREATE TABLE route_info (
    token TEXT PRIMARY KEY,
    backend_id TEXT REFERENCES backends(id),
    secret_token TEXT NOT NULL,
    user_data JSONB
);

CREATE INDEX idx_keys_backend ON keys(backend_id);
CREATE INDEX idx_backends_service ON backends(service);
CREATE INDEX idx_backends_drone ON backends(drone_id);
```

### Health Checks

```rust
async fn health_check_loop(drone_id: NodeId, state: ControllerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;

        // Check drone health
        let drones_to_sweep = state.get_unhealthy_drones().await;
        for drone in drones_to_sweep {
            state.mark_drone_lost(drone).await;
        }

        // Check backend health
        let backends_to_sweep = state.get_idle_backends().await;
        for backend in backends_to_sweep {
            state.terminate_backend(backend.id, TerminationReason::Swept).await;
        }
    }
}
```

### Metrics Collection

```rust
pub struct BackendMetrics {
    pub backend_id: BackendName,
    pub mem_used: u64,
    pub mem_total: u64,
    pub cpu_used: u64,
}

async fn report_metrics(
    drone_id: NodeId,
    metrics: BackendMetrics,
    state: ControllerState,
) {
    state.store_metrics(drone_id, metrics).await;
}
```

### Certificate Management

Use the `acme2-eab` crate for ACME certificate management:

```rust
use acme2_eab::{Directory, AccountBuilder};

async fn get_certificate(domain: &str) -> Result<Certificate> {
    let dir = Directory::from_acme_server(
        "https://acme-v02.api.letsencrypt.org/directory"
    ).await?;

    let mut builder = AccountBuilder::new(dir);
    builder.contact(&[format!("mailto:{}", ADMIN_EMAIL)]);

    let account = builder.build().await?;
    let order = account.new_order(&[domain], &[]).await?;

    // Complete DNS-01 challenge
    let auth = order.authorizations.first().unwrap();
    let challenge = auth.challenges.iter().find(|c| c.typ == "dns-01").unwrap();

    let (key, value) = challenge.get_dns_txt_value().unwrap();
    set_dns_txt_record(key, value).await?;

    challenge.validate(Default::default()).await?;
    let cert = order.finalize(RequestParams::default()).await?;

    Ok(cert)
}
```

## Deployment

### Docker Compose

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:14
    environment:
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data

  controller:
    image: myapp/controller
    environment:
      DATABASE_URL: postgres://postgres:${DB_PASSWORD}@postgres/myapp
    depends_on:
      - postgres

  proxy:
    image: myapp/proxy
    environment:
      CONTROLLER_URL: http://controller:8080
    ports:
      - "443:443"
    depends_on:
      - controller

  drone:
    image: myapp/drone
    environment:
      CONTROLLER_URL: http://controller:8080
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    depends_on:
      - controller

volumes:
  pgdata:
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: controller
spec:
  replicas: 3
  selector:
    matchLabels:
      app: controller
  template:
    metadata:
      labels:
        app: controller
    spec:
      containers:
      - name: controller
        image: myapp/controller
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: database-secret
              key: url
---
apiVersion: v1
kind: Service
metadata:
  name: controller
spec:
  selector:
    app: controller
  ports:
  - port: 8080
    targetPort: 8080
```

## Summary

To reproduce Jamsocket in Rust:

1. **Controller**: Central orchestrator with PostgreSQL, HTTP API, WebSocket to drones/proxies
2. **Drone**: Docker management, backend lifecycle, metrics reporting
3. **Proxy**: TLS termination, token-based routing, WebSocket relay
4. **Session Framework**: Service trait, client management, broadcast messaging
5. **Key Management**: Session affinity, deadlines, renewal
6. **WASM Support**: wasmtime integration for dynamic services
7. **Certificates**: ACME integration for TLS

Key design decisions:
- Use WebSocket for controller-drone/proxy communication
- Token-based routing for stateless proxies
- PostgreSQL for persistent state
- Docker for backend isolation
- Timed deadlines for automatic cleanup
