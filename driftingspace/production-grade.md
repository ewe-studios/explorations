---
title: "Production-Grade Driftingspace/Aper"
subtitle: "Performance, scaling, and deployment considerations"
prerequisites: [rust-revision.md](rust-revision.md)
next: [05-valtron-integration.md](05-valtron-integration.md)
---

# Production-Grade Driftingspace/Aper

This document covers production considerations for deploying Aper-based applications at scale.

## Table of Contents

1. [Performance Optimizations](#1-performance-optimizations)
2. [Memory Management](#2-memory-management)
3. [Connection Pooling](#3-connection-pooling)
4. [State Persistence](#4-state-persistence)
5. [Monitoring and Observability](#5-monitoring-and-observability)
6. [Scaling Strategies](#6-scaling-strategies)
7. [Security Considerations](#7-security-considerations)

---

## 1. Performance Optimizations

### Binary Serialization

Switch from JSON to bincode for reduced payload size:

```rust
// JSON (default)
let json = serde_json::to_string(&message).unwrap();
ctx.send_message(recipient, json.as_str());

// Bincode (binary)
let bytes = bincode::serialize(&message).unwrap();
ctx.send_binary(recipient, &bytes);

// Handle both formats
fn message(&mut self, client_id: ClientId, message: &str, ctx: &impl StateroomContext) {
    // Text = JSON
    let message: MessageToServer<P> = serde_json::from_str(message).unwrap();
    self.process_message(message, Some(client_id), ctx);
}

fn binary(&mut self, client_id: ClientId, message: &[u8], ctx: &impl StateroomContext) {
    // Binary = bincode
    let message: MessageToServer<P> = bincode::deserialize(message).unwrap();
    self.process_message(message, Some(client_id), ctx);
}
```

### Message Batching

Batch multiple transitions for high-throughput scenarios:

```rust
#[derive(Serialize, Deserialize, Debug)]
pub enum MessageToServer<S: StateMachine> {
    DoTransition {
        transition_number: ClientTransitionNumber,
        transition: S::Transition,
    },
    BatchTransitions {
        transitions: Vec<(ClientTransitionNumber, S::Transition)>,
    },
    RequestState,
}

impl<S: StateMachine> StateServer<S> {
    pub fn receive_message(&mut self, message: MessageToServer<S>) -> StateServerMessageResponse<S> {
        match message {
            MessageToServer::BatchTransitions { transitions } => {
                let mut replies = Vec::new();
                for (transition_number, transition) in transitions {
                    match self.state.apply(&transition) {
                        Ok(state) => {
                            self.state = state;
                            self.version.0 += 1;
                            replies.push(MessageToClient::ConfirmTransition {
                                transition_number,
                                version: self.version,
                            });
                        }
                        Err(_) => {
                            // Handle conflict
                        }
                    }
                }
                // Return batched response
            }
            // ...
        }
    }
}
```

### Delta Compression

Send only state deltas instead of full state:

```rust
#[derive(Serialize, Deserialize, Debug)]
pub enum MessageToClient<S> {
    SetState { state: S, version: StateVersionNumber },
    DeltaState {
        patches: Vec<StatePatch>,
        version: StateVersionNumber,
    },
    // ...
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatePatch {
    path: String,  // JSON path like "tasks.3.completed"
    value: serde_json::Value,
}
```

### Connection Keep-Alive

Prevent WebSocket timeouts:

```rust
// Send ping every 30 seconds
fn start_keepalive(ctx: &impl StateroomContext) {
    let interval = Duration::from_secs(30);
    ctx.set_timer(interval.as_millis() as u32);
}

fn timer(&mut self, ctx: &impl StateroomContext) {
    // Send ping
    ctx.send_message(
        MessageRecipient::Broadcast,
        "{\"type\": \"ping\"}",
    );
    // Schedule next ping
    ctx.set_timer(30000);
}
```

---

## 2. Memory Management

### State Size Limits

Prevent unbounded state growth:

```rust
pub const MAX_STATE_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB

impl<S: StateMachine> StateServer<S> {
    pub fn receive_message(&mut self, message: MessageToServer<S>) -> StateServerMessageResponse<S> {
        // Check state size after applying
        let estimated_size = bincode::serialized_size(&self.state).unwrap_or(0);
        if estimated_size > MAX_STATE_SIZE_BYTES {
            // Reject transition or trigger compaction
            return StateServerMessageResponse {
                reply_message: MessageToClient::Conflict {
                    transition_number: /* ... */,
                    conflict: S::Conflict::StateTooLarge,
                },
                broadcast_message: None,
            };
        }
        // ...
    }
}
```

### Transition Queue Limits

Limit pending transitions on clients:

```rust
pub const MAX_PENDING_TRANSITIONS: usize = 100;

impl<S: StateMachine> StateClient<S> {
    pub fn push_transition(&mut self, transition: S::Transition) -> Result<MessageToServer<S>, S::Conflict> {
        if self.transitions.len() >= MAX_PENDING_TRANSITIONS {
            // Queue full - request state reset or drop oldest
            return Err(S::Conflict::QueueFull);
        }
        // ...
    }
}
```

### Garbage Collection for Lists

Clean up deleted list entries:

```rust
impl<T: StateMachine + PartialEq> List<T> {
    pub fn compact(&mut self) {
        // Remove entries from pool that have been deleted
        let active_ids: HashSet<Uuid> = self.items.values().copied().collect();
        self.pool.retain(|id, _| active_ids.contains(id));
    }
}
```

---

## 3. Connection Pooling

### Reconnecting Clients

Handle disconnections gracefully:

```rust
pub struct ReconnectingClient<S: StateProgram> {
    client: Option<AperWebSocketStateProgramClient<S>>,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
}

impl<S: StateProgram> ReconnectingClient<S> {
    pub fn reconnect(&mut self, url: &str, callback: impl Fn(Rc<S>, Duration, ClientId) + 'static) {
        if self.reconnect_attempts >= self.max_reconnect_attempts {
            // Give up
            return;
        }

        let delay = 2u64.pow(self.reconnect_attempts); // Exponential backoff
        self.reconnect_attempts += 1;

        // Schedule reconnection
        std::thread::sleep(Duration::from_secs(delay));

        self.client = Some(AperWebSocketStateProgramClient::new(url, callback).unwrap());
    }
}
```

### Connection Pool for Multiple Rooms

```rust
use std::collections::HashMap;

pub struct ConnectionPool {
    connections: HashMap<RoomId, WebSocketConnection>,
}

impl ConnectionPool {
    pub fn get_or_create(&mut self, room_id: &RoomId) -> &mut WebSocketConnection {
        // Reuse existing connection or create new one
        self.connections.entry(room_id.clone())
            .or_insert_with(|| WebSocketConnection::new(room_id))
    }

    pub fn cleanup_inactive(&mut self) {
        // Remove connections with no activity
        self.connections.retain(|_, conn| conn.is_active());
    }
}
```

---

## 4. State Persistence

### SQLite Persistence

Persist state to SQLite:

```rust
use rusqlite::{Connection, params};

pub struct StatePersistence<S: StateMachine> {
    conn: Connection,
    _phantom: PhantomData<S>,
}

impl<S: StateMachine> StatePersistence<S> {
    pub fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS states (
                room_id TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                version INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS transitions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                room_id TEXT NOT NULL,
                transition TEXT NOT NULL,
                version INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn,
            _phantom: PhantomData,
        })
    }

    pub fn save_state(&self, room_id: &str, state: &S, version: StateVersionNumber) -> Result<(), rusqlite::Error> {
        let state_json = serde_json::to_string(state).unwrap();
        self.conn.execute(
            "INSERT OR REPLACE INTO states (room_id, state, version, updated_at)
             VALUES (?1, ?2, ?3, strftime('%s', 'now'))",
            params![room_id, state_json, version.0],
        )?;
        Ok(())
    }

    pub fn load_state(&self, room_id: &str) -> Result<Option<(S, StateVersionNumber)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT state, version FROM states WHERE room_id = ?1"
        )?;

        let result = stmt.query_row(params![room_id], |row| {
            let state_json: String = row.get(0)?;
            let version: i64 = row.get(1)?;
            Ok((state_json, version))
        });

        match result {
            Ok((state_json, version)) => {
                let state: S = serde_json::from_str(&state_json).unwrap();
                Ok(Some((state, StateVersionNumber(version as u32))))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn record_transition(&self, room_id: &str, transition: &S::Transition, version: StateVersionNumber) -> Result<(), rusqlite::Error> {
        let transition_json = serde_json::to_string(transition).unwrap();
        self.conn.execute(
            "INSERT INTO transitions (room_id, transition, version, created_at)
             VALUES (?1, ?2, ?3, strftime('%s', 'now'))",
            params![room_id, transition_json, version.0],
        )?;
        Ok(())
    }
}
```

### Event Sourcing with Replay

Store transitions and replay for recovery:

```rust
pub struct EventStore<S: StateMachine> {
    persistence: StatePersistence<S>,
}

impl<S: StateMachine> EventStore<S> {
    pub fn recover_state(&self, room_id: &str, initial_state: S) -> Result<(S, StateVersionNumber), rusqlite::Error> {
        let mut state = initial_state;
        let mut version = StateVersionNumber(0);

        // Load all transitions for this room
        let mut stmt = self.persistence.conn.prepare(
            "SELECT transition FROM transitions
             WHERE room_id = ?1
             ORDER BY id ASC"
        )?;

        let transitions = stmt.query_map(params![room_id], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        for transition_result in transitions {
            let transition_json = transition_result?;
            let transition: S::Transition = serde_json::from_str(&transition_json).unwrap();
            if let Ok(new_state) = state.apply(&transition) {
                state = new_state;
                version.0 += 1;
            }
        }

        Ok((state, version))
    }
}
```

---

## 5. Monitoring and Observability

### Metrics Collection

Track key metrics:

```rust
use prometheus::{IntCounter, IntGauge, Histogram, register_int_counter, register_int_gauge, register_histogram};

pub struct AperMetrics {
    pub transitions_total: IntCounter,
    pub conflicts_total: IntCounter,
    pub connected_clients: IntGauge,
    pub state_size_bytes: Histogram,
    pub transition_latency_ms: Histogram,
}

impl AperMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        Ok(Self {
            transitions_total: register_int_counter!(
                "aper_transitions_total",
                "Total number of transitions processed"
            )?,
            conflicts_total: register_int_counter!(
                "aper_conflicts_total",
                "Total number of conflicts"
            )?,
            connected_clients: register_int_gauge!(
                "aper_connected_clients",
                "Number of connected clients"
            )?,
            state_size_bytes: register_histogram!(
                "aper_state_size_bytes",
                "Size of state in bytes"
            )?,
            transition_latency_ms: register_histogram!(
                "aper_transition_latency_ms",
                "Time to process a transition"
            )?,
        })
    }
}

// Use in server
impl<S: StateMachine> StateServer<S> {
    pub fn receive_message(&mut self, message: MessageToServer<S>, metrics: &AperMetrics) -> StateServerMessageResponse<S> {
        let start = std::time::Instant::now();

        match message {
            MessageToServer::DoTransition { .. } => {
                metrics.transitions_total.inc();
            }
            _ => {}
        }

        let result = self.process_message(message);

        let latency = start.elapsed().as_secs_f64() * 1000.0;
        metrics.transition_latency_ms.observe(latency);

        // Track state size
        let state_size = bincode::serialized_size(&self.state).unwrap_or(0);
        metrics.state_size_bytes.observe(state_size as f64);

        result
    }
}
```

### Logging

Structured logging for debugging:

```rust
use log::{info, warn, error};
use tracing::{instrument, Span};

#[instrument(skip(self, ctx), fields(client_id = ?client_id))]
fn message(&mut self, client_id: ClientId, message: &str, ctx: &impl StateroomContext) {
    let parsed: Result<MessageToServer<P>, _> = serde_json::from_str(message);

    match parsed {
        Ok(msg) => {
            info!(transition_type = ?msg, "Processing transition");
            self.process_message(msg, Some(client_id), ctx);
        }
        Err(e) => {
            error!(error = ?e, "Failed to parse message");
        }
    }
}
```

### Health Checks

Expose health endpoints:

```rust
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub connected_clients: usize,
    pub uptime_seconds: u64,
    pub last_transition: Option<u64>,
}

impl HealthStatus {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

// HTTP health endpoint (separate from WebSocket)
fn handle_health(metrics: &AperMetrics) -> String {
    let status = HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        connected_clients: metrics.connected_clients.get() as usize,
        uptime_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        last_transition: None, // Track last transition timestamp
    };
    status.to_json()
}
```

---

## 6. Scaling Strategies

### Horizontal Scaling with Redis Pub/Sub

Scale across multiple server instances:

```rust
use redis::{Client, PubSub, publish};

pub struct ScaledStateServer<S: StateMachine> {
    local_server: StateServer<S>,
    redis_client: Client,
    instance_id: String,
}

impl<S: StateMachine> ScaledStateServer<S> {
    pub fn broadcast_transition(&self, transition: &S::Transition, version: StateVersionNumber) {
        let mut conn = self.redis_client.get_connection().unwrap();
        let message = serde_json::to_string(&transition).unwrap();

        // Publish to all instances
        publish(&mut conn, "aper:transitions", message).unwrap();
    }

    pub fn receive_remote_transition(&mut self, transition: S::Transition, version: StateVersionNumber) {
        // Apply transition from another instance
        if let Ok(state) = self.local_server.state.apply(&transition) {
            self.local_server.state = state;
            self.local_server.version = version;
        }
    }
}
```

### Sharding by Room

Distribute rooms across servers:

```rust
use consistent_hash::ConsistentHash;

pub struct ShardedServer<S: StateMachine> {
    hash_ring: ConsistentHash<String>,
    servers: HashMap<String, ScaledStateServer<S>>,
}

impl<S: StateMachine> ShardedServer<S> {
    pub fn get_server_for_room(&self, room_id: &str) -> &ScaledStateServer<S> {
        let server_id = self.hash_ring.get(room_id).unwrap();
        &self.servers[server_id]
    }
}
```

### Load Balancing

Use reverse proxy for WebSocket load balancing:

```nginx
# nginx.conf example
upstream aper_servers {
    least_conn;
    server aper-1:8080;
    server aper-2:8080;
    server aper-3:8080;
}

server {
    listen 80;

    location /ws {
        proxy_pass http://aper_servers;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

---

## 7. Security Considerations

### Authentication

Validate client identity:

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct AuthToken {
    user_id: String,
    expires_at: u64,
    signature: String,
}

fn validate_token(token: &str, secret: &[u8]) -> Option<AuthToken> {
    // Verify HMAC signature
    // Check expiration
    // Return validated token
}

fn connect(&mut self, client_id: ClientId, message: &str, ctx: &impl StateroomContext) {
    // Extract token from connection query params
    let token = ctx.get_query_param("token");

    match validate_token(&token, self.secret) {
        Some(auth) => {
            // Store authenticated user ID
            self.authenticated_users.insert(client_id, auth.user_id);
        }
        None => {
            // Reject connection
            ctx.close();
        }
    }
}
```

### Authorization

Check permissions for transitions:

```rust
impl StateServer<DropFourGame> {
    pub fn receive_message(&mut self, message: MessageToServer<DropFourGame>, client_id: ClientId) {
        if let MessageToServer::DoTransition { transition, .. } = &message {
            // Check if client is allowed to make this move
            let game_state = self.state.state();
            if let PlayState::Playing { next_player, player_map, .. } = game_state {
                let player_color = player_map.color_of_player(client_id);
                if player_color != Some(*next_player) {
                    // Not this player's turn - reject
                    return;
                }
            }
        }
        // Process normally
    }
}
```

### Rate Limiting

Prevent abuse:

```rust
use std::collections::HashMap;
use std::time::{Instant, Duration};

pub struct RateLimiter {
    requests: HashMap<ClientId, Vec<Instant>>,
    limit: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn allow(&mut self, client_id: ClientId) -> bool {
        let now = Instant::now();
        let window_start = now - self.window;

        // Remove old requests
        self.requests.entry(client_id).or_default()
            .retain(|&t| t > window_start);

        // Check if under limit
        let requests = self.requests.get_mut(&client_id).unwrap();
        if requests.len() >= self.limit {
            return false;
        }

        requests.push(now);
        true
    }
}

// Use in server
fn message(&mut self, client_id: ClientId, message: &str, ctx: &impl StateroomContext) {
    if !self.rate_limiter.allow(client_id) {
        // Rate limited - ignore or send error
        return;
    }
    // Process normally
}
```

---

## Summary

| Area | Key Considerations |
|------|-------------------|
| Performance | Binary serialization, batching, delta compression |
| Memory | Size limits, queue limits, garbage collection |
| Connections | Reconnection, pooling, keep-alive |
| Persistence | SQLite, event sourcing, replay |
| Monitoring | Metrics, logging, health checks |
| Scaling | Redis pub/sub, sharding, load balancing |
| Security | Authentication, authorization, rate limiting |

---

## Next Steps

Continue to [05-valtron-integration.md](05-valtron-integration.md) for Lambda deployment without async/tokio.
