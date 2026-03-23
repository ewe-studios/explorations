# Aper: State Synchronization Library

Aper is a state synchronization library for building real-time collaborative applications. It works seamlessly with Stateroom for WebSocket-based multiplayer apps.

## Overview

**Aper** provides:

- **Atomic Data Structures**: Shared state primitives
- **Speculative Execution**: Optimistic UI updates
- **Server Reconciliation**: Conflict resolution
- **Listener Pattern**: Reactive updates
- **Derive Macros**: Easy state machine definitions

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Aper Client                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                  AperClient<A>                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │  │
│  │  │    Store    │  │   Intent    │  │  Version    │   │  │
│  │  │  (overlay)  │  │   Stack     │  │  Tracking   │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘   │  │
│  └───────────────────────────────────────────────────────┘  │
│                             │                                │
│                    WebSocket │                               │
│                             ▼                                │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   Aper Server                         │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │              AperServer<A>                      │  │  │
│  │  │  ┌───────────┐  ┌───────────┐                  │  │  │
│  │  │  │   Store   │  │  Version  │                  │  │  │
│  │  │  │ (single)  │  │   Counter │                  │  │  │
│  │  │  └───────────┘  └───────────┘                  │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Core Traits

### AperSync

Base trait for synchronizable state:

```rust
pub trait AperSync: Clone {
    fn attach(map: StoreHandle) -> Self;

    fn listen<F: Fn() -> bool + 'static + Send + Sync>(&self, listener: F) {
        // Default implementation does nothing
    }
}
```

### Aper

Main trait for state machines:

```rust
pub trait Aper: AperSync + 'static {
    type Intent: Clone + Serialize + for<'de> Deserialize<'de> + PartialEq;
    type Error: Debug;

    fn apply(
        &mut self,
        intent: &Self::Intent,
        metadata: &IntentMetadata,
    ) -> Result<(), Self::Error>;

    fn suspended_event(&self) -> Option<(Self::Intent, IntentMetadata)> {
        None
    }
}
```

## Data Structures

### Atom

A synchronized atomic value:

```rust
pub struct Atom<T: Serialize + DeserializeOwned + Default> {
    map: StoreHandle,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned + Default> Atom<T> {
    pub fn get(&self) -> T {
        self.map
            .get(&Bytes::new())
            .map(|bytes| bincode::deserialize(&bytes).unwrap())
            .unwrap_or_default()
    }

    pub fn set(&mut self, value: T) {
        self.map.set(
            Bytes::new(),
            Bytes::from(bincode::serialize(&value).unwrap()),
        );
    }
}
```

### AtomMap

A synchronized map:

```rust
pub struct AtomMap<K, V> {
    map: StoreHandle,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V> AtomMap<K, V>
where
    K: Serialize + DeserializeOwned + Default + Hash + Eq,
    V: Serialize + DeserializeOwned + Default,
{
    pub fn get(&self, key: &K) -> Option<V> {
        let key_bytes = Bytes::from(bincode::serialize(key).unwrap());
        self.map
            .get(&key_bytes)
            .map(|bytes| bincode::deserialize(&bytes).unwrap())
    }

    pub fn insert(&mut self, key: K, value: V) {
        let key_bytes = Bytes::from(bincode::serialize(&key).unwrap());
        let value_bytes = Bytes::from(bincode::serialize(&value).unwrap());
        self.map.set(key_bytes, value_bytes);
    }

    pub fn remove(&mut self, key: &K) {
        let key_bytes = Bytes::from(bincode::serialize(key).unwrap());
        self.map.delete(&key_bytes);
    }
}
```

## Client-Server Model

### Client

```rust
pub struct AperClient<A: Aper> {
    store: Store,
    intent_stack: VecDeque<SpeculativeIntent<A::Intent>>,
    next_client_version: u64,
    verified_client_version: u64,
    verified_server_version: u64,
}

impl<A: Aper> AperClient<A> {
    pub fn new() -> Self;
    pub fn state(&self) -> A;
    pub fn apply(&mut self, intent: &A::Intent, metadata: &IntentMetadata) -> Result<u64, A::Error>;
    pub fn mutate(&mut self, mutations: &[Mutation], client_version: Option<u64>, server_version: u64);
    pub fn connect<F: Fn(MessageToServer) + 'static>(self, callback: F) -> ClientConnection<A>;
}
```

### Server

```rust
pub struct AperServer<A: Aper> {
    map: Store,
    version: u64,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: Aper> AperServer<A> {
    pub fn new() -> Self;
    pub fn version(&self) -> u64;
    pub fn state(&self) -> A;
    pub fn state_snapshot(&self) -> Vec<Mutation>;
    pub fn apply(&mut self, intent: &A::Intent, metadata: &IntentMetadata) -> Result<Vec<Mutation>, A::Error>;
}
```

## Speculative Execution

Aper uses optimistic UI with server reconciliation:

```rust
// Client applies intent speculatively
let version = client.apply(&intent, &metadata)?;

// Intent is queued for sending to server
intent_stack.push(SpeculativeIntent {
    intent: intent.clone(),
    metadata: metadata.clone(),
    version,
});

// Server processes intent
let mutations = server.apply(&intent, &metadata)?;

// Server sends mutations back with confirmed version
client.mutate(&mutations, Some(version), server_version);
```

### Version Tracking

```
Client State:
├─ verified_client_version: 5  (last confirmed by server)
├─ next_client_version: 8      (next speculative intent)
└─ intent_stack: [v6, v7]      (pending confirmation)

Server State:
└─ version: 15                 (global mutation counter)
```

## Store System

### Overlay Architecture

```rust
pub struct Store {
    layers: Vec<Layer>,
    listeners: Vec<Listener>,
}

impl Store {
    pub fn push_overlay(&mut self);
    pub fn pop_overlay(&mut self);
    pub fn combine_down(&mut self);
    pub fn mutate(&mut self, mutations: &[Mutation]);
    pub fn notify_dirty(&mut self);
}
```

### Layer Model

```
┌─────────────────────────────────────────┐
│  Overlay 3 (speculative intent v7)      │
├─────────────────────────────────────────┤
│  Overlay 2 (speculative intent v6)      │
├─────────────────────────────────────────┤
│  Overlay 1 (verified state)             │
├─────────────────────────────────────────┤
│  Base (empty or initial state)          │
└─────────────────────────────────────────┘
```

### Mutation Structure

```rust
pub struct Mutation {
    pub prefix: Vec<Bytes>,
    pub entries: PrefixMap,
}

pub struct PrefixMap {
    // Map of key -> value pairs
}
```

## Listener Pattern

```rust
pub trait Listener: Send + Sync {
    fn on_dirty(&self);
}

impl Store {
    pub fn add_listener(&mut self, listener: Box<dyn Listener>);
    pub fn notify_dirty(&mut self);
}
```

### Usage

```rust
let state = client.state();
state.listen(|| {
    // Called when state changes
    // Return false to unsubscribe
    true
});
```

## Example: Counter

### State Definition

```rust
use aper::{Aper, AperSync, IntentMetadata};
use aper_derive::AperSync;
use serde::{Deserialize, Serialize};

#[derive(AperSync, Clone)]
struct Counter {
    value: Atom<i32>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
enum CounterIntent {
    Increment,
    Decrement,
}

#[derive(Debug)]
enum CounterError {}

impl Aper for Counter {
    type Intent = CounterIntent;
    type Error = CounterError;

    fn apply(&mut self, intent: &Self::Intent, _metadata: &IntentMetadata) -> Result<(), Self::Error> {
        match intent {
            CounterIntent::Increment => {
                let current = self.value.get();
                self.value.set(current + 1);
            }
            CounterIntent::Decrement => {
                let current = self.value.get();
                self.value.set(current - 1);
            }
        }
        Ok(())
    }
}
```

### Client Usage

```rust
let mut client = AperClient::<Counter>::new();
let state = client.state();

// Set up listener
state.listen(|| {
    println!("Counter value: {}", state.value.get());
    true
});

// Apply intent
client.apply(&CounterIntent::Increment, &IntentMetadata::now()).unwrap();

// Connect to server
let connection = client.connect(|msg| {
    // Send message to server via WebSocket
    ws.send(serde_json::to_string(&msg).unwrap()).unwrap();
});
```

### Server Usage

```rust
let mut server = AperServer::<Counter>::new();

// Handle incoming intent
let mutations = server.apply(&intent, &metadata).unwrap();

// Broadcast to all clients
let client_version = extract_client_version(&mutations);
let server_version = server.version();

for client in clients {
    client.send_mutation(&mutations, client_version, server_version);
}
```

## Integration with Stateroom

```rust
use aper_stateroom::AperStateroomService;

struct MultiplayerCounter {
    server: AperServer<Counter>,
}

impl StateroomService for MultiplayerCounter {
    fn message(&mut self, client: ClientId, message: MessagePayload, ctx: &impl StateroomContext) {
        let intent: CounterIntent = serde_json::from_str(&message.text().unwrap()).unwrap();
        let metadata = IntentMetadata::new(Some(client.into()), Utc::now());

        match self.server.apply(&intent, &metadata) {
            Ok(mutations) => {
                // Broadcast to all clients
                let update = ServerMessage::StateUpdate {
                    mutations,
                    client_version: None,
                    server_version: self.server.version(),
                };
                ctx.send_message(
                    MessageRecipient::Broadcast,
                    serde_json::to_string(&update).unwrap(),
                );
            }
            Err(_) => {
                // Send error to client
            }
        }
    }
}
```

## Connection Handling

### Client Connection

```rust
pub struct ClientConnection<A: Aper> {
    client: Arc<Mutex<AperClient<A>>>,
    message_callback: Arc<Box<dyn Fn(MessageToServer)>>,
}

impl<A: Aper> ClientConnection<A> {
    pub fn handle_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::StateUpdate { mutations, client_version, server_version } => {
                self.client.lock().unwrap().mutate(&mutations, client_version, server_version);
            }
            ServerMessage::Reject { version } => {
                // Handle rejection (conflict resolution needed)
            }
        }
    }

    pub fn apply(&mut self, intent: A::Intent) {
        let metadata = IntentMetadata::now();
        let version = self.client.lock().unwrap().apply(&intent, &metadata).unwrap();

        self.message_callback(MessageToServer::Intent {
            intent,
            metadata,
            version,
        });
    }
}
```

## Conflict Resolution

Aper handles conflicts through:

1. **Speculative Execution**: Apply locally immediately
2. **Server Authority**: Server is source of truth
3. **Re-application**: Re-apply speculative intents after server confirmation

```rust
pub fn mutate(
    &mut self,
    mutations: &[Mutation],
    client_version: Option<u64>,
    server_version: u64,
) {
    // Pop speculative overlay
    self.store.pop_overlay();

    // Apply server mutations
    self.store.mutate(mutations);

    // Push new speculative overlay
    self.store.push_overlay();

    // Re-apply unconfirmed intents
    for intent in &self.intent_stack {
        if intent.version > client_version.unwrap_or(0) {
            self.store.push_overlay();
            let mut sm = A::attach(self.store.handle());
            sm.apply(&intent.intent, &intent.metadata).ok();
            self.store.combine_down();
        }
    }
}
```

## Derive Macro

The `#[derive(AperSync)]` macro generates boilerplate:

```rust
#[derive(AperSync, Clone)]
struct GameState {
    players: AtomMap<PlayerId, Player>,
    board: Atom<Vec<Cell>>,
    turn: Atom<PlayerId>,
}

// Generates:
// - Implementation of AperSync
// - attach() method
// - listen() method
```

## Use Cases

### Multiplayer Games

```rust
#[derive(AperSync, Clone)]
struct TicTacToe {
    board: Atom<[Option<Player>; 9]>,
    current_player: Atom<Player>,
    winner: Atom<Option<Player>>,
}
```

### Collaborative Editors

```rust
#[derive(AperSync, Clone)]
struct Document {
    content: Atom<String>,
    cursors: AtomMap<UserId, CursorPosition>,
    selections: AtomMap<UserId, Selection>,
}
```

### Real-time Dashboards

```rust
#[derive(AperSync, Clone)]
struct Dashboard {
    metrics: AtomMap<MetricId, MetricValue>,
    alerts: Atom<Vec<Alert>>,
    last_updated: Atom<DateTime<Utc>>,
}
```

## Dependencies

```toml
[dependencies]
aper = "0.1"
aper_derive = "0.1"
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
bytes = "1"
chrono = { version = "0.4", features = ["serde"] }
```

## Comparison to Alternatives

| Feature | Aper | Yjs | Automerge |
|---------|------|-----|-----------|
| Language | Rust | JS | Rust/JS |
| Data Model | Custom | CRDT | CRDT |
| Conflict Resolution | Server-authoritative | CRDT merge | CRDT merge |
| Speculative Execution | Built-in | Manual | Built-in |
| WebSocket Integration | Stateroom | y-websocket | Custom |
