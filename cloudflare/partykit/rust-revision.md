---
title: "PartyKit Rust Revision: Complete Translation Guide"
subtitle: "From TypeScript/PartyKit to Rust with workers-rs and valtron executor"
based_on: "PartyServer packages/partyserver/, workers-rs, valtron executor"
---

# PartyKit Rust Revision: Complete Translation Guide

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Type System Design](#2-type-system-design)
3. [Server Implementation](#3-server-implementation)
4. [Connection Management](#4-connection-management)
5. [Storage Patterns](#5-storage-patterns)
6. [Valtron Executor Integration](#6-valtron-executor-integration)
7. [Complete Example](#7-complete-example)

---

## 1. Architecture Overview

### 1.1 TypeScript to Rust Mapping

| TypeScript Component | Rust Equivalent |
|---------------------|-----------------|
| `Server` class | `struct Server` + `impl DurableObject` |
| `Connection` type | `struct Connection` + `WebSocket` |
| `onConnect`, `onMessage` | Trait methods or impl methods |
| `this.ctx.storage.sql` | `ctx.storage().sql()` |
| `this.broadcast()` | `server.broadcast()` |
| `connection.setState()` | `connection.set_state()` |
| Async functions | Valtron `TaskIterator` |

### 1.2 Dependency Setup

```toml
# Cargo.toml
[package]
name = "partykit-rust"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
worker = "0.4"  # Cloudflare Workers Rust SDK
worker-macros = "0.4"
console_error_panic_hook = "0.1"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
yrs = "0.21"  # Yjs Rust port (Yrs)
uuid = { version = "1.0", features = ["v4"] }
foundation_core = { path = "../ewe_platform/backends/foundation_core" }  # Valtron
```

### 1.3 Project Structure

```
partykit-rust/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # WASM entry point
│   ├── server/
│   │   ├── mod.rs                # Server module
│   │   ├── mod.rs                # Core server struct
│   │   ├── connection.rs         # Connection handling
│   │   └── lifecycle.rs          # Lifecycle hooks
│   ├── storage/
│   │   ├── mod.rs                # Storage module
│   │   ├── sql.rs                # SQL storage
│   │   └── kv.rs                 # Key-value storage
│   ├── yjs/
│   │   ├── mod.rs                # Yjs integration
│   │   └── document.rs           # Yrs document wrapper
│   ├── valtron/
│   │   ├── mod.rs                # Valtron tasks
│   │   ├── broadcast.rs          # Broadcast task
│   │   └── heartbeat.rs          # Heartbeat task
│   └── types/
│       ├── mod.rs                # Type definitions
│       └── state.rs              # Connection state
└── tests/
    └── integration.rs
```

---

## 2. Type System Design

### 2.1 Connection State

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Connection state - stored in WebSocket attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionState {
    pub user_id: String,
    pub username: String,
    pub session_id: String,
    pub status: UserStatus,
    pub last_active: u64,
    pub cursor: Option<Cursor>,
    pub typing: bool,
    pub current_room: Option<String>,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Connecting,
    Active,
    Idle,
    Away,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub x: i32,
    pub y: i32,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            username: String::new(),
            session_id: uuid::Uuid::new_v4().to_string(),
            status: UserStatus::Connecting,
            last_active: 0,
            cursor: None,
            typing: false,
            current_room: None,
            color: "#000000".to_string(),
        }
    }
}
```

### 2.2 Connection Type

```rust
use worker::WebSocket;
use std::rc::Rc;
use std::cell::RefCell;

/// Wrapped WebSocket with state and metadata
pub struct Connection {
    pub id: String,
    pub uri: Option<String>,
    pub websocket: Rc<RefCell<WebSocket>>,
    pub state: Rc<RefCell<ConnectionState>>,
    pub tags: Vec<String>,
    pub server_name: String,
}

impl Connection {
    pub fn new(
        id: String,
        uri: Option<String>,
        websocket: WebSocket,
        server_name: String,
    ) -> Self {
        Self {
            id,
            uri,
            websocket: Rc::new(RefCell::new(websocket)),
            state: Rc::new(RefCell::new(ConnectionState::default())),
            tags: vec![id.clone()],  // Always include connection ID
            server_name,
        }
    }

    pub fn send(&self, message: &str) -> Result<(), worker::Error> {
        let ws = self.websocket.borrow();
        ws.send_with_str(message)
    }

    pub fn set_state(&self, new_state: ConnectionState) {
        *self.state.borrow_mut() = new_state;
    }

    pub fn update_state<F>(&self, f: F)
    where
        F: FnOnce(&mut ConnectionState),
    {
        let mut state = self.state.borrow_mut();
        f(&mut state);
    }

    pub fn close(&self, code: u16, reason: &str) -> Result<(), worker::Error> {
        let ws = self.websocket.borrow();
        ws.close_with_code_and_reason(code, reason)
    }
}
```

### 2.3 Message Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: u64 },

    #[serde(rename = "chat_message")]
    ChatMessage { content: String },

    #[serde(rename = "typing")]
    Typing { is_typing: bool },

    #[serde(rename = "cursor_update")]
    CursorUpdate { cursor: Cursor },

    #[serde(rename = "join_room")]
    JoinRoom { room_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "welcome")]
    Welcome { room_id: String, user_id: String },

    #[serde(rename = "user_joined")]
    UserJoined { user_id: String, username: String },

    #[serde(rename = "user_left")]
    UserLeft { user_id: String, reason: String },

    #[serde(rename = "message")]
    Message {
        id: String,
        content: String,
        sender_id: String,
        sender_name: String,
        timestamp: u64,
    },

    #[serde(rename = "typing_indicator")]
    TypingIndicator { user_id: String, username: String, is_typing: bool },

    #[serde(rename = "presence_batch")]
    PresenceBatch { updates: Vec<PresenceUpdate> },

    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceUpdate {
    pub user_id: String,
    pub cursor: Option<Cursor>,
    pub typing: bool,
    pub status: UserStatus,
}
```

---

## 3. Server Implementation

### 3.1 Server Struct

```rust
use worker::{
    durable_object, Request, Response, Result, WebSocket, WebSocketPair,
    State, Env, DurableObject
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::connection::Connection;
use crate::types::ConnectionState;

#[durable_object]
pub struct ChatServer {
    state: State,
    env: Env,
    connections: Rc<RefCell<HashMap<String, Connection>>>,
    name: RefCell<Option<String>>,
    initialized: RefCell<bool>,
}

impl ChatServer {
    /// Called when server starts or wakes from hibernation
    async fn on_start(&self, props: Option<serde_json::Value>) -> Result<()> {
        // Initialize SQLite schema
        self.state.storage().sql().exec(
            "CREATE TABLE IF NOT EXISTS messages (\
                id TEXT PRIMARY KEY,\
                room_id TEXT NOT NULL,\
                sender_id TEXT NOT NULL,\
                content TEXT NOT NULL,\
                created_at INTEGER NOT NULL\
            )",
            &[],
        )?;

        // Load props if provided
        if let Some(props) = props {
            // Handle initialization from props
        }

        console_log!("Room {} started", self.get_name());
        Ok(())
    }

    /// Called when new WebSocket connects
    async fn on_connect(&self, connection: Connection) -> Result<()> {
        console_log!("Client {} connected to {}", connection.id, self.get_name());

        // Notify other users
        self.broadcast(&ServerMessage::UserJoined {
            user_id: connection.state.borrow().user_id.clone(),
            username: connection.state.borrow().username.clone(),
        }, Some(&connection.id))?;

        // Send welcome message
        connection.send(&serde_json::to_string(&ServerMessage::Welcome {
            room_id: self.get_name(),
            user_id: connection.state.borrow().user_id.clone(),
        })?)?;

        Ok(())
    }

    /// Called when message received from client
    async fn on_message(&self, connection: &Connection, message: String) -> Result<()> {
        let client_msg: ClientMessage = serde_json::from_str(&message)?;

        match client_msg {
            ClientMessage::Heartbeat { timestamp } => {
                connection.update_state(|state| {
                    state.last_active = timestamp;
                    state.status = crate::types::UserStatus::Active;
                });
            }

            ClientMessage::ChatMessage { content } => {
                // Store in database
                let id = uuid::Uuid::new_v4().to_string();
                let state = connection.state.borrow();

                self.state.storage().sql().exec(
                    "INSERT INTO messages (id, room_id, sender_id, content, created_at)\
                     VALUES (?, ?, ?, ?, ?)",
                    &[
                        &id,
                        &self.get_name(),
                        &state.user_id,
                        &content,
                        &(Date::now() as i64),
                    ],
                )?;

                // Broadcast to all
                self.broadcast(&ServerMessage::Message {
                    id,
                    content,
                    sender_id: state.user_id.clone(),
                    sender_name: state.username.clone(),
                    timestamp: Date::now() as u64,
                }, None)?;
            }

            ClientMessage::Typing { is_typing } => {
                connection.update_state(|state| state.typing = is_typing);

                // Broadcast typing indicator (exclude sender)
                let state = connection.state.borrow();
                self.broadcast(&ServerMessage::TypingIndicator {
                    user_id: state.user_id.clone(),
                    username: state.username.clone(),
                    is_typing,
                }, Some(&connection.id))?;
            }

            ClientMessage::CursorUpdate { cursor } => {
                connection.update_state(|state| state.cursor = Some(cursor));
                // Cursor updates can be batched/throttled
            }

            ClientMessage::JoinRoom { room_id } => {
                connection.update_state(|state| state.current_room = Some(room_id));
            }
        }

        Ok(())
    }

    /// Called when connection closes
    async fn on_close(&self, connection: &Connection, code: u16, reason: String) -> Result<()> {
        console_log!("Client {} disconnected: {}", connection.id, reason);

        // Notify other users
        self.broadcast(&ServerMessage::UserLeft {
            user_id: connection.state.borrow().user_id.clone(),
            reason,
        }, None)?;

        // Remove from connections
        self.connections.borrow_mut().remove(&connection.id);

        // Set alarm for cleanup if room is empty
        if self.connections.borrow().is_empty() {
            self.state.storage().set_alarm(Date::now() + 5 * 60 * 1000).await?;
        }

        Ok(())
    }

    /// Broadcast message to all connections
    fn broadcast(&self, message: &ServerMessage, exclude_id: Option<&str>) -> Result<()> {
        let message_str = serde_json::to_string(message)?;
        let mut connections = self.connections.borrow_mut();

        for (id, connection) in connections.iter() {
            if exclude_id.map_or(true, |excl| id != excl) {
                let _ = connection.send(&message_str);
                // Ignore send errors (connection may be closed)
            }
        }

        Ok(())
    }

    fn get_name(&self) -> String {
        self.name.borrow()
            .clone()
            .unwrap_or_else(|| "<unnamed>".to_string())
    }

    async fn set_name(&self, name: String) -> Result<()> {
        self.state.storage().put("__name", &name).await?;
        *self.name.borrow_mut() = Some(name);
        Ok(())
    }

    async fn ensure_initialized(&self) -> Result<()> {
        if *self.initialized.borrow() {
            return Ok(());
        }

        // Load name from storage
        if let Some(name) = self.state.storage().get("__name").await? {
            *self.name.borrow_mut() = Some(name);
        }

        // Call on_start
        self.on_start(None).await?;

        *self.initialized.borrow_mut() = true;
        Ok(())
    }
}

#[durable_object]
impl DurableObject for ChatServer {
    fn new(state: State, env: Env) -> Self {
        console_error_panic_hook::set_once();
        Self {
            state,
            env,
            connections: Rc::new(RefCell::new(HashMap::new())),
            name: RefCell::new(None),
            initialized: RefCell::new(false),
        }
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        self.ensure_initialized().await?;

        // Check for WebSocket upgrade
        if req.headers().get("upgrade").ok().flatten().as_deref() == Some("websocket") {
            let pair = WebSocketPair::new()?;
            let server = pair.server;
            server.accept()?;

            // Generate connection ID
            let connection_id = uuid::Uuid::new_v4().to_string();

            // Create connection
            let connection = Connection::new(
                connection_id,
                Some(req.url()?.to_string()),
                server,
                self.get_name(),
            );

            // Add to connections
            self.connections.borrow_mut()
                .insert(connection.id.clone(), connection.clone());

            // Call on_connect
            self.on_connect(connection).await?;

            return Ok(Response::from_websocket(pair.client)?);
        }

        // Handle HTTP requests
        self.on_request(req).await
    }

    async fn websocket_message(
        &mut self,
        ws: WebSocket,
        message: worker::WebSocketIncomingMessage,
    ) -> Result<()> {
        self.ensure_initialized().await?;

        // Find connection
        let connections = self.connections.borrow();
        for conn in connections.values() {
            if conn.websocket.borrow().as_ref() == &ws {
                match message {
                    worker::WebSocketIncomingMessage::String(s) => {
                        self.on_message(conn, s).await?;
                    }
                    worker::WebSocketIncomingMessage::ArrayBuffer(buf) => {
                        // Handle binary messages
                    }
                }
                break;
            }
        }

        Ok(())
    }

    async fn websocket_close(
        &mut self,
        ws: WebSocket,
        code: usize,
        reason: String,
        was_clean: bool,
    ) -> Result<()> {
        self.ensure_initialized().await?;

        // Find and remove connection
        let mut connections = self.connections.borrow_mut();
        let mut to_remove = None;

        for (id, conn) in connections.iter() {
            if conn.websocket.borrow().as_ref() == &ws {
                self.on_close(conn, code as u16, reason).await?;
                to_remove = Some(id.clone());
                break;
            }
        }

        if let Some(id) = to_remove {
            connections.remove(&id);
        }

        Ok(())
    }

    async fn websocket_error(&mut self, ws: WebSocket) -> Result<()> {
        // Handle WebSocket errors
        self.websocket_close(ws, 1011, "Internal error".to_string(), false).await
    }

    async fn alarm(&mut self) -> Result<Response> {
        // Handle alarm (cleanup, snapshots, etc.)
        if self.connections.borrow().is_empty() {
            // Create snapshot
            self.create_snapshot().await?;
            console_log!("Room {} cleaned up", self.get_name());
        }

        Ok(Response::ok("ok")?)
    }
}
```

---

## 4. Connection Management

### 4.1 Connection Manager Trait

```rust
use std::rc::Rc;
use crate::connection::Connection;

pub trait ConnectionManager {
    fn count(&self) -> usize;
    fn get(&self, id: &str) -> Option<Rc<Connection>>;
    fn get_all(&self) -> Vec<Rc<Connection>>;
    fn get_by_tag(&self, tag: &str) -> Vec<Rc<Connection>>;
    fn add(&mut self, connection: Connection);
    fn remove(&mut self, id: &str);
}

/// In-memory connection manager (non-hibernating)
pub struct InMemoryConnectionManager {
    connections: HashMap<String, Rc<Connection>>,
    tags: HashMap<String, Vec<String>>,  // tag -> connection_ids
}

impl InMemoryConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            tags: HashMap::new(),
        }
    }
}

impl ConnectionManager for InMemoryConnectionManager {
    fn count(&self) -> usize {
        self.connections.len()
    }

    fn get(&self, id: &str) -> Option<Rc<Connection>> {
        self.connections.get(id).cloned()
    }

    fn get_all(&self) -> Vec<Rc<Connection>> {
        self.connections.values().cloned().collect()
    }

    fn get_by_tag(&self, tag: &str) -> Vec<Rc<Connection>> {
        self.tags.get(tag)
            .map(|ids| ids.iter().filter_map(|id| self.connections.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    fn add(&mut self, connection: Connection) {
        for tag in &connection.tags {
            self.tags.entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(connection.id.clone());
        }
        self.connections.insert(connection.id.clone(), Rc::new(connection));
    }

    fn remove(&mut self, id: &str) {
        if let Some(conn) = self.connections.remove(id) {
            for tag in &conn.tags {
                if let Some(ids) = self.tags.get_mut(tag) {
                    ids.retain(|x| x != id);
                }
            }
        }
    }
}
```

### 4.2 Heartbeat Task (Valtron)

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use std::time::Duration;
use std::rc::Rc;
use std::cell::RefCell;
use crate::connection::Connection;

/// Heartbeat checker task
pub struct HeartbeatChecker {
    connections: Rc<RefCell<HashMap<String, Rc<Connection>>>>,
    timeout_ms: u64,
    interval_ms: u64,
    last_check: u64,
}

impl HeartbeatChecker {
    pub fn new(
        connections: Rc<RefCell<HashMap<String, Rc<Connection>>>>,
        timeout_ms: u64,
        interval_ms: u64,
    ) -> Self {
        Self {
            connections,
            timeout_ms,
            interval_ms,
            last_check: 0,
        }
    }
}

impl TaskIterator for HeartbeatChecker {
    type Ready = Vec<String>;  // IDs of timed-out connections
    type Pending = Duration;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        let now = Date::now() as u64;

        if now - self.last_check < self.interval_ms {
            // Not time to check yet
            return Some(TaskStatus::Pending(
                Duration::from_millis(self.interval_ms - (now - self.last_check))
            ));
        }

        self.last_check = now;

        // Check for timed-out connections
        let mut timed_out = Vec::new();

        for (id, conn) in self.connections.borrow().iter() {
            let state = conn.state.borrow();
            if now - state.last_active > self.timeout_ms {
                timed_out.push(id.clone());
            }
        }

        Some(TaskStatus::Ready(timed_out))
    }
}
```

---

## 5. Storage Patterns

### 5.1 SQL Storage Helper

```rust
use worker::State;
use serde::Serialize;
use serde_json::Value;

pub struct SqlStorage<'a>(&'a State);

impl<'a> SqlStorage<'a> {
    pub fn new(state: &'a State) -> Self {
        Self(state)
    }

    pub fn exec(&self, query: &str, params: &[&dyn worker::SqlValue]) -> Result<(), worker::Error> {
        self.0.storage().sql().exec(query, params)
    }

    pub fn query_one<T: for<'de> serde::Deserialize<'de>>(
        &self,
        query: &str,
        params: &[&dyn worker::SqlValue],
    ) -> Result<Option<T>, worker::Error> {
        let result = self.0.storage().sql().exec(query, params)?;
        // Parse first row as T
        // This requires custom JSON parsing from the SQL result
        Ok(None)  // Placeholder
    }

    pub fn query_all<T: for<'de> serde::Deserialize<'de>>(
        &self,
        query: &str,
        params: &[&dyn worker::SqlValue],
    ) -> Result<Vec<T>, worker::Error> {
        let result = self.0.storage().sql().exec(query, params)?;
        // Parse all rows as Vec<T>
        Ok(Vec::new())  // Placeholder
    }
}

// Usage
impl ChatServer {
    fn save_message(&self, id: &str, content: &str, sender_id: &str) -> Result<(), worker::Error> {
        let sql = SqlStorage::new(&self.state);
        sql.exec(
            "INSERT INTO messages (id, room_id, sender_id, content, created_at)\
             VALUES (?, ?, ?, ?, ?)",
            &[
                &id,
                &self.get_name(),
                &sender_id,
                &content,
                &((Date::now() as i64)),
            ],
        )
    }
}
```

### 5.2 Snapshot Pattern

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct RoomSnapshot {
    pub version: u32,
    pub timestamp: u64,
    pub messages: Vec<MessageRecord>,
    pub metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub sender_id: String,
    pub content: String,
    pub created_at: u64,
}

impl ChatServer {
    async fn create_snapshot(&self) -> Result<(), worker::Error> {
        let sql = SqlStorage::new(&self.state);

        // Query all messages
        let messages = sql.query_all::<MessageRecord>(
            "SELECT id, sender_id, content, created_at FROM messages WHERE room_id = ?",
            &[&self.get_name()],
        )?;

        let snapshot = RoomSnapshot {
            version: 1,
            timestamp: Date::now() as u64,
            messages,
            metadata: serde_json::json!({}),
        };

        // Store snapshot
        self.state.storage().put("snapshot", &snapshot).await?;

        Ok(())
    }

    async fn load_snapshot(&self) -> Result<Option<RoomSnapshot>, worker::Error> {
        self.state.storage().get("snapshot").await
    }

    async fn restore_snapshot(&self, snapshot: RoomSnapshot) -> Result<(), worker::Error> {
        let sql = SqlStorage::new(&self.state);

        // Clear existing messages
        sql.exec("DELETE FROM messages WHERE room_id = ?", &[&self.get_name()])?;

        // Restore messages
        for msg in snapshot.messages {
            sql.exec(
                "INSERT INTO messages (id, room_id, sender_id, content, created_at)\
                 VALUES (?, ?, ?, ?, ?)",
                &[
                    &msg.id,
                    &self.get_name(),
                    &msg.sender_id,
                    &msg.content,
                    &((msg.created_at as i64)),
                ],
            )?;
        }

        Ok(())
    }
}
```

---

## 6. Valtron Executor Integration

### 6.1 Broadcast Task

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use std::rc::Rc;
use std::cell::RefCell;
use crate::connection::Connection;
use crate::types::ServerMessage;

/// Broadcast message to all connections
pub struct BroadcastTask {
    connections: Rc<RefCell<HashMap<String, Rc<Connection>>>>,
    message: String,
    exclude_ids: Vec<String>,
    sent_count: usize,
}

impl BroadcastTask {
    pub fn new(
        connections: Rc<RefCell<HashMap<String, Rc<Connection>>>>,
        message: String,
        exclude_ids: Vec<String>,
    ) -> Self {
        Self {
            connections,
            message,
            exclude_ids,
            sent_count: 0,
        }
    }
}

impl TaskIterator for BroadcastTask {
    type Ready = usize;  // Number of connections messaged
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        let connections = self.connections.borrow();
        let mut count = 0;

        for (id, conn) in connections.iter() {
            if self.exclude_ids.contains(id) {
                continue;
            }

            let _ = conn.send(&self.message);
            count += 1;
        }

        self.sent_count += count;
        Some(TaskStatus::Ready(count))
    }
}
```

### 6.2 Running Tasks

```rust
use foundation_core::valtron::single::{initialize, run_until_complete, spawn};
use foundation_core::valtron::FnReady;

// In your worker
#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    // Initialize valtron executor
    initialize(0);  // Seed for deterministic execution

    // Spawn broadcast task
    let connections = Rc::new(RefCell::new(HashMap::new()));
    let message = serde_json::to_string(&ServerMessage::Welcome {
        room_id: "room1".to_string(),
        user_id: "user1".to_string(),
    })?;

    spawn()
        .with_task(BroadcastTask::new(connections.clone(), message, vec![]))
        .with_resolver(Box::new(FnReady::new(|count, _| {
            console_log!("Broadcast to {} connections", count);
        })))
        .schedule()?;

    // Run to completion
    run_until_complete();

    Ok(Response::ok("ok")?)
}
```

---

## 7. Complete Example

### 7.1 Full Chat Server

```rust
use worker::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod types;
mod connection;
mod server;

use types::*;
use connection::Connection;
use server::ChatServer;

#[event(fetch, respond_with_errors)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    // Route to Durable Object
    let namespace = env.durable_object("CHAT_SERVER")?;
    let id = namespace.id_from_name("chat-room-1")?;
    let stub = id.get_stub()?;

    stub.fetch_with_request(req).await
}

// In server.rs (complete implementation)
#[durable_object]
pub struct ChatServer {
    state: State,
    env: Env,
    connections: Rc<RefCell<HashMap<String, Connection>>>,
    name: RefCell<Option<String>>,
    initialized: RefCell<bool>,
}

#[durable_object]
impl DurableObject for ChatServer {
    fn new(state: State, env: Env) -> Self {
        console_error_panic_hook::set_once();
        Self {
            state,
            env,
            connections: Rc::new(RefCell::new(HashMap::new())),
            name: RefCell::new(None),
            initialized: RefCell::new(false),
        }
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        // ... (full implementation from section 3)
        Ok(Response::ok("not implemented")?)
    }

    async fn websocket_message(
        &mut self,
        ws: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        // ... (full implementation from section 3)
        Ok(())
    }

    async fn websocket_close(
        &mut self,
        ws: WebSocket,
        code: usize,
        reason: String,
        was_clean: bool,
    ) -> Result<()> {
        // ... (full implementation from section 3)
        Ok(())
    }

    async fn alarm(&mut self) -> Result<Response> {
        // Create snapshot and cleanup
        Ok(Response::ok("ok")?)
    }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial Rust revision created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
