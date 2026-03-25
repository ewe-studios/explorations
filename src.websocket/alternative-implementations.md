# Alternative WebSocket Implementations

## rust-websocket

### Overview

`rust-websocket` is an older WebSocket implementation that predates `tungstenite-rs`. While feature-complete, it's based on outdated dependencies and is no longer recommended for new projects.

**Status:** Maintenance mode - consider `tungstenite-rs` or `tokio-tungstenite` instead.

### Architecture

```
rust-websocket/
├── src/
│   ├── lib.rs                  # Main exports
│   ├── client/
│   │   ├── mod.rs              # Client module
│   │   ├── builder.rs          # ClientBuilder
│   │   ├── sync.rs             # Synchronous client
│   │   └── async.rs            # Asynchronous client
│   ├── server/
│   │   ├── mod.rs              # Server module
│   │   ├── sync.rs             # Synchronous server
│   │   ├── async.rs            # Asynchronous server
│   │   └── upgrade/            # HTTP upgrade handling
│   ├── codec/
│   │   ├── mod.rs              # Codec traits
│   │   ├── http.rs             # HTTP codec
│   │   └── ws.rs               # WebSocket codec
│   ├── header/                 # HTTP headers
│   ├── message.rs              # Message types
│   ├── dataframe.rs            # Frame types
│   └── stream.rs               # Stream traits
└── websocket-base/             # Core implementation
    └── src/
        ├── ws/                 # WebSocket logic
        ├── codec/              # Codecs
        └── message/            # Messages
```

### Key Differences from tungstenite-rs

| Feature | rust-websocket | tungstenite-rs |
|---------|---------------|----------------|
| Dependencies | Hyper 0.10, Tokio 0.1 | Modern hyper, tokio 1.x |
| API Style | Builder pattern | Direct functions |
| Async Model | Futures 0.1 | Futures 0.3, async/await |
| TLS | native-tls only | native-tls + rustls |
| Active Development | No | Yes |

### ClientBuilder Pattern

```rust
use websocket::ClientBuilder;

// Parse URL
let client = ClientBuilder::new("ws://localhost:8080/socket")
    .unwrap()
    .async_connect()  // or .connect() for sync
    .await
    .unwrap();

// With custom headers
let client = ClientBuilder::new("ws://localhost:8080/socket")
    .unwrap()
    .add_header("Authorization", "Bearer token")
    .add_header("Sec-WebSocket-Protocol", "chat")
    .async_connect()
    .await
    .unwrap();
```

### Synchronous API

```rust
use websocket::sync::Client;

let mut client = ClientBuilder::new("ws://localhost:8080/")
    .unwrap()
    .connect_insecure()
    .unwrap();

// Send message
client.send_message(&Message::text("Hello")).unwrap();

// Receive message
let msg = client.recv_message().unwrap();
```

### Asynchronous API (Legacy Futures)

```rust
use websocket::async::Client;
use futures::{Stream, Future};

let client = ClientBuilder::new("ws://localhost:8080/")
    .unwrap()
    .async_connect()
    .wait()
    .unwrap();

// Split into sink and stream
let (sink, stream) = client.split();

// Process messages
stream.for_each(|msg| {
    println!("Received: {:?}", msg);
    Ok(())
}).wait();
```

### Server Implementation

```rust
use websocket::sync::Server;

// Bind server
let mut server = Server::bind("127.0.0.1:8080").unwrap();

// Accept connections
for connection in server.incoming_connections() {
    let client = connection.accept().unwrap();

    // Handle client
    std::thread::spawn(move || {
        for msg in client.incoming_messages() {
            println!("Received: {:?}", msg);
        }
    });
}
```

### Message and Frame Types

```rust
use websocket::message::{Message, OwnedMessage};
use websocket::dataframe::{DataFrame, Opcode};

// High-level message
let msg = Message::text("Hello");
let msg = Message::binary(vec![0x01, 0x02, 0x03]);

// Low-level frames
let frame = DataFrame::new(Opcode::Text)
    .with_data(b"Hello")
    .into_owned();
```

### Limitations

1. **Outdated Tokio** - Uses tokio-core/tokio-io (pre-0.2)
2. **Futures 0.1** - Incompatible with modern async/await
3. **Hyper 0.10** - Very old HTTP library
4. **Limited TLS** - Only native-tls support
5. **No active maintenance** - Issues and PRs unanswered

---

## Sunrise / Sunrise-DOM

### Overview

Sunrise is a TypeScript/JavaScript library for spreadsheet-like dataflow programming. It's not a WebSocket implementation but appears in this directory structure due to shared authorship (Snapview).

**Purpose:** Reactive programming with cells and formulas

### Core Concepts

```typescript
import { cell, formula, swap, deref } from '@snapview/sunrise'

// Source cell
const x = cell<number>(1)

// Formula cell (derived)
const y = formula(a => a + 1, x)

// Side-effect formula
const printCell = formula(console.log, y)

// Update value
swap(x, v => v + 1)

// Read value
deref(x) // 2
deref(y) // 3
```

### Cell Types

**Source Cells:**
- Mutable containers for values
- Created with `cell<T>(initialValue)`
- Updated via `reset(newValue, cell)` or `swap(fn, cell)`

**Formula Cells:**
- Derived values based on other cells
- Automatically update when dependencies change
- Created with `formula(computeFn, ...deps)`

### Built-in Formula Helpers

```typescript
// Extract object field
const user = cell({ name: 'Alice', age: 30 })
const name = field('name', user)

// Array element by index
const items = cell(['a', 'b', 'c'])
const first = byIndex(0, items)

// Boolean conversion
const truthy = toBool(cell(1))  // true

// Negation
const notX = not(cell(true))  // false

// History (previous + current)
const hist = history(cell(1))
// [undefined, 1] initially, then [1, 2] after update
```

### Software Transactional Memory

```typescript
// Atomic updates across multiple cells
const x = cell(0)
const y = cell(0)
const sum = formula((a, b) => a + b, x, y)

// Both updates happen atomically
reset(1, x)
reset(2, y)
// sum updates once with final values
```

### Cell Lifecycle

```typescript
const x = cell(1)
const y = formula(a => a * 2, x)

// Subscribe to changes
const unsubscribe = subscribe(x, (newVal, oldVal) => {
    console.log(`Changed from ${oldVal} to ${newVal}`)
})

// Destroy cell (also destroys dependents)
destroy(x)  // Both x and y are destroyed

// Operations on destroyed cells throw
deref(x)  // Throws OperationOnDestroyedCellError
```

### Relation to WebSockets

Sunrise-DOM appears to be a DOM binding layer that could use WebSocket connections for real-time data:

```
WebSocket ──► Sunrise Cells ──► DOM Updates
    │              │
    │              └──► Formula derivations
    │
    └──► Real-time data stream
```

---

## Comparison Summary

| Aspect | tungstenite-rs | rust-websocket | Sunrise |
|--------|---------------|----------------|---------|
| Language | Rust | Rust | TypeScript |
| Purpose | WebSocket protocol | WebSocket protocol | Dataflow programming |
| Status | Active | Legacy | Active |
| Async Model | Modern async/await | Legacy futures | Reactive |
| Recommendation | ✅ Use | ⚠️ Avoid | Context-dependent |

## When to Use Each

### tungstenite-rs / tokio-tungstenite
- Modern Rust WebSocket projects
- Performance-critical applications
- When you need full RFC 6455 compliance

### rust-websocket
- Maintaining legacy code
- Educational purposes (comparing implementations)
- Not recommended for new projects

### Sunrise
- TypeScript/JavaScript reactive applications
- Spreadsheet-like dataflow requirements
- Real-time UI updates
