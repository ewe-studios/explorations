# Driftdb: Real-Time Database with WebSocket Sync

Driftdb is a real-time database that synchronizes over WebSockets. It provides a simple key-value store with sequence-based updates.

## Overview

**Driftdb** is a minimalist real-time database:

- **Key-Value Store**: Simple data model
- **Sequence Numbers**: Ordered updates per key
- **WebSocket Sync**: Real-time client synchronization
- **Action Types**: Relay, Replace, Append, Compact
- **Cloudflare Workers**: Edge deployment support

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Driftdb Server                           │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   HTTP Router                         │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐              │  │
│  │  │   /     │  │   /ws   │  │  /debug │              │  │
│  │  └─────────┘  └─────────┘  └─────────┘              │  │
│  └───────────────────────────────────────────────────────┘  │
│                             │                                │
│  ┌──────────────────────────▼────────────────────────────┐  │
│  │                    Database                           │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │                  Store                          │  │  │
│  │  │  ┌───────────┐  ┌───────────┐  ┌───────────┐   │  │  │
│  │  │  │ Key: foo  │  │ Key: bar  │  │ Key: baz  │   │  │  │
│  │  │  │ [v1, v2]  │  │ [v1]      │  │ [v1, v2]  │   │  │  │
│  │  │  └───────────┘  └───────────┘  └───────────┘   │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
│                             │                                │
│  ┌──────────────────────────▼────────────────────────────┐  │
│  │               Connections                             │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐              │  │
│  │  │  Sub:   │  │  Sub:   │  │  Debug  │              │  │
│  │  │  [foo]  │  │  [bar]  │  │  [all]  │              │  │
│  │  └─────────┘  └─────────┘  └─────────┘              │  │
│  └────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Data Model

### Keys and Values

```rust
pub type Key = String;
pub type Value = ciborium::Value;
```

Values are CBOR-encoded for efficient serialization.

### Sequence Numbers

Each key maintains a sequence of values:

```rust
pub type SequenceNumber = u64;

pub struct SequenceValue {
    pub seq: SequenceNumber,
    pub value: Value,
}
```

### Actions

```rust
pub enum Action {
    /// Ephemeral message, not stored
    Relay,

    /// Replace entire value
    Replace,

    /// Append to stream
    Append,

    /// Compact stream from sequence
    Compact { seq: SequenceNumber },
}
```

## Database Operations

### Push

```rust
pub fn push(
    &mut self,
    key: &Key,
    value: &Value,
    action: &Action,
) -> Option<MessageFromDatabase>
```

**Behavior by Action:**

- **Relay**: Broadcast to subscribers, don't store
- **Replace**: Replace value, increment sequence
- **Append**: Add to stream, increment sequence
- **Compact**: Remove values before sequence

### Subscribe

```rust
pub fn subscribe(&mut self, key: &Key, connection: Weak<Connection>)
```

Subscribers receive updates for specific keys.

### Get

```rust
pub fn get(&self, key: &Key, seq: SequenceNumber) -> Option<MessageFromDatabase>
```

Returns values from sequence number onwards.

## Message Protocol

### Client to Server

```rust
pub enum MessageToDatabase {
    /// Subscribe to key and get current state
    Get {
        key: Key,
        seq: Option<SequenceNumber>,
    },

    /// Push an update
    Push {
        key: Key,
        value: Value,
        action: Action,
    },
}
```

### Server to Client

```rust
pub enum MessageFromDatabase {
    /// Initial state (all values for key)
    Init {
        key: Key,
        data: Vec<SequenceValue>,
    },

    /// New update
    Push {
        key: Key,
        value: Value,
        seq: SequenceNumber,
    },

    /// Stream size changed
    StreamSize {
        key: Key,
        size: usize,
    },
}
```

## Connection Handling

### Connection Structure

```rust
pub struct Connection {
    callback: Arc<Box<dyn Fn(&MessageFromDatabase) + Send + Sync>>,
    db: Weak<Mutex<DatabaseInner>>,
}

impl Connection {
    pub fn new<F>(
        callback: F,
        db: Arc<Mutex<DatabaseInner>>,
    ) -> Self
    where
        F: Fn(&MessageFromDatabase) + 'static + Send + Sync,
    {
        Self {
            callback: Arc::new(Box::new(callback)),
            db: Arc::downgrade(&db),
        }
    }
}
```

### Weak References

Connections use `Weak` references to avoid memory leaks:

```rust
pub struct DatabaseInner {
    subscriptions: HashMap<Key, Vec<Weak<Connection>>>,
    debug_connections: Vec<Weak<Connection>>,
}

// Cleanup disconnected connections
listeners.retain(|conn| {
    if let Some(conn) = conn.upgrade() {
        (conn.callback)(&message);
        true
    } else {
        false  // Connection dropped, remove from list
    }
});
```

## Action Types

### Relay (Ephemeral)

Messages are broadcast but not stored:

```rust
// Client sends
{ "type": "push", "key": "chat", "value": "Hello!", "action": "Relay" }

// Subscribers receive
{ "type": "push", "key": "chat", "value": "Hello!", "seq": 1 }

// New subscriber gets empty Init
{ "type": "init", "key": "chat", "data": [] }
```

Use case: Chat messages, presence updates

### Replace

Values are replaced entirely:

```rust
// Client sends
{ "type": "push", "key": "cursor", "value": {"x": 100, "y": 200}, "action": "Replace" }

// Sequence increments
// Only latest value kept
```

Use case: Cursor positions, current state

### Append

Values are appended to a stream:

```rust
// Client sends
{ "type": "push", "key": "history", "value": "edit1", "action": "Append" }
{ "type": "push", "key": "history", "value": "edit2", "action": "Append" }

// Stream grows
// StreamSize message sent
```

Use case: Edit history, event logs

### Compact

Remove old values from stream:

```rust
// Client sends
{ "type": "push", "key": "history", "action": "Compact", "seq": 5 }

// Values with seq < 5 are removed
```

Use case: Log rotation, memory management

## Store Implementation

### In-Memory Store

```rust
pub struct Store {
    data: HashMap<Key, Vec<SequenceValue>>,
}

impl Store {
    pub fn apply(
        &mut self,
        key: &Key,
        value: Value,
        action: &Action,
    ) -> ApplyResult {
        match action {
            Action::Relay => ApplyResult::broadcast(value),
            Action::Replace => self.replace(key, value),
            Action::Append => self.append(key, value),
            Action::Compact { seq } => self.compact(key, *seq),
        }
    }

    pub fn get(&self, key: &Key, seq: SequenceNumber) -> Vec<SequenceValue> {
        self.data
            .get(key)
            .map(|values| values.iter().filter(|v| v.seq >= seq).cloned().collect())
            .unwrap_or_default()
    }
}
```

### Apply Result

```rust
pub struct ApplyResult {
    pub broadcast: Option<SequenceValue>,
    pub stream_size: usize,
}

impl ApplyResult {
    pub fn mutates(&self) -> bool {
        self.broadcast.is_some()
    }
}
```

## WebSocket Handler

### Server Implementation

```rust
async fn handle_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<ServerState>) {
    let (mut send, mut recv) = socket.split();
    let (tx, mut rx) = mpsc::channel(100);

    // Create connection
    let conn = state.db.connect(move |msg| {
        tx.try_send(msg).ok();
    });

    // Handle incoming messages
    tokio::spawn(async move {
        while let Some(msg) = recv.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(req) = serde_json::from_str(&text) {
                    handle_request(req, &conn);
                }
            }
        }
    });

    // Send outgoing messages
    while let Some(msg) = rx.next().await {
        if let Ok(text) = serde_json::to_string(&msg) {
            send.send(Message::Text(text)).await.unwrap();
        }
    }
}
```

## Cloudflare Workers

Driftdb can be deployed to Cloudflare Workers:

### Worker Configuration

```toml
# wrangler.toml
name = "driftdb-worker"
compatibility_date = "2023-01-01"
main = "src/lib.rs"
```

### Worker Entry Point

```rust
use driftdb_worker::config::Config;
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let config = Config::from_env(&env)?;
    let db = ctx.state().db.clone();
    config.handle_request(req, db).await
}
```

## JavaScript Client

### Basic Usage

```typescript
import { Driftdb } from 'driftdb';

const db = new Driftdb('ws://localhost:8080');

// Subscribe to a key
const sub = db.subscribe('my-key', (message) => {
    console.log('Update:', message);
});

// Push updates
db.push('my-key', { hello: 'world' }, 'Replace');

// Unsubscribe
sub.unsubscribe();
```

### React Hook

```typescript
import { useDriftdb } from 'driftdb-react';

function Counter({ roomId }) {
    const [count, setCount] = useState(0);

    useDriftdb(`room-${roomId}`, (msg) => {
        if (msg.type === 'push') {
            setCount(msg.value.count);
        }
    });

    const increment = () => {
        db.push(`room-${roomId}`, { count: count + 1 }, 'Replace');
    };

    return <button onClick={increment}>{count}</button>;
}
```

## Testing

### Unit Tests

```rust
#[test]
fn test_ephemeral_message() {
    let db = Database::new();
    let (stash, callback) = MessageStash::new();
    let conn = db.connect(callback);

    subscribe(&conn, "foo");

    // Initial state is empty
    assert_eq!(
        Some(MessageFromDatabase::Init { data: vec![], key: "foo".into() }),
        stash.next()
    );

    // Relay message is broadcast
    push(&conn, "foo", json!({ "bar": "baz" }), Action::Relay);

    assert_eq!(
        Some(MessageFromDatabase::Push {
            key: "foo".into(),
            value: json_to_cbor(json!({ "bar": "baz" })),
            seq: SequenceNumber(1),
        }),
        stash.next()
    );
}
```

### Integration Tests

```rust
#[test]
fn test_durable_message_sent_to_later_connection() {
    let db = Database::new();

    // First connection
    let (stash, callback) = MessageStash::new();
    let conn = db.connect(callback);
    subscribe(&conn, "foo");

    // Push durable message
    push(&conn, "foo", json!({ "bar": "baz" }), Action::Replace);

    // Second connection should receive existing state
    let (stash2, callback2) = MessageStash::new();
    let conn2 = db.connect(callback2);
    subscribe(&conn2, "foo");

    assert_eq!(
        Some(MessageFromDatabase::Init {
            data: vec![SequenceValue {
                value: json_to_cbor(json!({ "bar": "baz" })),
                seq: SequenceNumber(1),
            }],
            key: "foo".into()
        }),
        stash2.next()
    );
}
```

## Dependencies

```toml
[dependencies]
ciborium = "0.2"        # CBOR serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = ["ws"] }
worker = "0.1"          # Cloudflare Workers
```

## Use Cases

### Real-time Counters

```rust
// Key: "counter:room-123"
// Action: Replace
{ "count": 42 }
```

### Presence System

```rust
// Key: "presence:user-456"
// Action: Relay (ephemeral)
{ "status": "online", "cursor": { "x": 100, "y": 200 } }
```

### Collaborative State

```rust
// Key: "board:canvas-1"
// Action: Append (for history)
{ "type": "draw", "path": [[0, 0], [10, 10]] }
```

### Chat Messages

```rust
// Key: "chat:room-1"
// Action: Relay (ephemeral, stored elsewhere)
{ "user": "alice", "text": "Hello!" }
```

## Comparison to Alternatives

| Feature | Driftdb | Redis Pub/Sub | Firebase RTDB |
|---------|---------|---------------|---------------|
| Data Model | Key-Value | Key-Value | JSON Tree |
| Sequencing | Per-key | None | Global |
| Persistence | In-memory | Optional | Built-in |
| WebSocket | Built-in | External | Built-in |
| Edge Deploy | Yes | No | Limited |
