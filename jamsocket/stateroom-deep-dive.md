# Stateroom: WASM-Based Session Backend Framework

Stateroom is a minimalist framework for building lightweight, single-threaded WebSocket services that can run natively or as WebAssembly modules.

## Overview

**Stateroom** provides a simple abstraction for stateful WebSocket services:

- **Minimalist API**: Just implement the `StateroomService` trait
- **WASM Support**: Compile services to WASM for dynamic loading
- **Single-threaded**: Simple concurrency model, no locks needed
- **Message-driven**: Events for connect, disconnect, message, timer

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   Stateroom Server                          │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   Axum Router                         │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐              │  │
│  │  │  GET /  │  │ GET /ws │  │  Static │              │  │
│  │  │ (health)│  │ (WS)    │  │  Files  │              │  │
│  │  └─────────┘  └─────────┘  └─────────┘              │  │
│  └───────────────────────────────────────────────────────┘  │
│                             │                                │
│  ┌──────────────────────────▼────────────────────────────┐  │
│  │                  ServerState                           │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │              Service Actor                       │  │  │
│  │  │  ┌───────────────────────────────────────────┐  │  │  │
│  │  │  │          StateroomService                  │  │  │  │
│  │  │  │  - connect(client_id)                      │  │  │  │
│  │  │  │  - disconnect(client_id)                   │  │  │  │
│  │  │  │  - message(client_id, payload)             │  │  │  │
│  │  │  │  - timer()                                 │  │  │  │
│  │  │  └───────────────────────────────────────────┘  │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Core Traits

### StateroomService

The main trait for implementing a service:

```rust
pub trait StateroomService: Send + Sync + 'static {
    /// Called when the service is created, before any client connects
    fn init(&mut self, context: &impl StateroomContext) {}

    /// Called each time a client connects
    fn connect(&mut self, client: ClientId, context: &impl StateroomContext) {}

    /// Called each time a client disconnects
    fn disconnect(&mut self, client: ClientId, context: &impl StateroomContext) {}

    /// Called each time a client sends a message
    fn message(
        &mut self,
        client: ClientId,
        message: MessagePayload,
        context: &impl StateroomContext,
    ) {}

    /// Called when timer expires
    fn timer(&mut self, context: &impl StateroomContext) {}
}
```

### StateroomContext

The interface for sending messages back to the host:

```rust
pub trait StateroomContext: Send + Sync + Clone + 'static {
    /// Send a message to a client or broadcast
    fn send_message(
        &self,
        recipient: impl Into<MessageRecipient>,
        message: impl Into<MessagePayload>,
    );

    /// Set a timer to invoke timer() after delay
    fn set_timer(&self, ms_delay: u32);
}
```

### Message Recipients

```rust
pub enum MessageRecipient {
    Broadcast,        // All connected clients
    EveryoneExcept(ClientId),  // All except one
    Client(ClientId), // Specific client
}
```

### Message Payloads

```rust
pub enum MessagePayload {
    Text(String),
    Bytes(Vec<u8>),
}
```

## Example: Chat Server

```rust
use stateroom::*;
use std::collections::HashMap;

#[derive(Default)]
struct ChatServer {
    client_to_nickname: HashMap<ClientId, String>,
}

impl StateroomService for ChatServer {
    fn connect(&mut self, client: ClientId, ctx: &impl StateroomContext) {
        let username = format!("client{}", u32::from(client));

        // Send welcome message
        ctx.send_message(client,
            format!("Welcome! Your name is {}. Send /nick <name> to change.", username));

        // Broadcast to others
        ctx.send_message(MessageRecipient::Broadcast,
            format!("{} has joined the chat", username));
    }

    fn disconnect(&mut self, client: ClientId, ctx: &impl StateroomContext) {
        let username = self.client_to_nickname.remove(&client).unwrap();
        ctx.send_message(MessageRecipient::Broadcast,
            format!("{} has left the chat", username));
    }

    fn message(&mut self, client: ClientId, message: MessagePayload, ctx: &impl StateroomContext) {
        let Some(message) = message.text() else { return };

        if let Some(new_nick) = message.strip_prefix("/nick ") {
            let old_nick = self.client_to_nickname.insert(client, new_nick.to_string()).unwrap();
            ctx.send_message(MessageRecipient::Broadcast,
                format!("{} is now known as {}", old_nick, new_nick));
        } else {
            let username = self.client_to_nickname.get(&client).unwrap();
            ctx.send_message(MessageRecipient::Broadcast,
                format!("{}: {}", username, message));
        }
    }
}
```

## WASM Integration

### Guest Module (stateroom-wasm)

Services compile to WASM using the `#[stateroom_wasm]` macro:

```rust
use stateroom_wasm::*;

#[stateroom_wasm]
#[derive(Default)]
struct EchoServer;

impl StateroomService for EchoServer {
    fn connect(&mut self, client_id: ClientId, ctx: &impl StateroomContext) {
        ctx.send_message(client_id, format!("User {:?} connected.", client_id));
    }

    fn message(&mut self, client_id: ClientId, message: MessagePayload, ctx: &impl StateroomContext) {
        ctx.send_message(
            MessageRecipient::Broadcast,
            format!("User {:?} sent '{}'", client_id, message.text().unwrap_or("")),
        );
    }

    fn disconnect(&mut self, client_id: ClientId, ctx: &impl StateroomContext) {
        ctx.send_message(
            MessageRecipient::Broadcast,
            format!("User {:?} left.", client_id),
        );
    }
}
```

### Host Runtime (stateroom-wasm-host)

The host runtime uses wasmtime to execute WASM modules:

```rust
pub struct WasmHost {
    store: Store<WasiCtx>,
    memory: Memory,
    fn_malloc: TypedFunc<u32, u32>,
    fn_free: TypedFunc<(u32, u32), ()>,
    fn_recv: TypedFunc<(u32, u32), ()>,
}
```

### External Functions

The host provides these functions to WASM:

| Function | Purpose |
|----------|---------|
| `stateroom_send` | Send message to clients |
| `stateroom_recv` | Receive message from host |
| `stateroom_malloc` | Allocate memory |
| `stateroom_free` | Free memory |

### Version Checking

WASM modules must export these globals:

```rust
const EXPECTED_API_VERSION: i32 = 1;
const EXPECTED_PROTOCOL_VERSION: i32 = 0;

// Globals checked at load time:
// STATEROOM_API_VERSION
// STATEROOM_API_PROTOCOL
```

### Message Serialization

Messages are serialized using bincode:

```rust
// Host sends message to WASM
fn try_recv(&mut self, message: MessageToProcess) -> Result<()> {
    let payload = bincode::serialize(&message).unwrap();
    let (pt, len) = self.put_data(&payload)?;
    self.fn_recv.call(&mut self.store, (pt, len))?;
    self.fn_free.call(&mut self.store, (pt, len))?;
    Ok(())
}

// WASM sends message to host
linker.func_wrap(ENV, EXT_FN_SEND, move |caller, start, len| {
    let memory = get_memory(&mut caller);
    let message = get_u8_vec(&caller, &memory, start, len);
    let message: MessageFromProcess = bincode::deserialize(message).unwrap();
    match message {
        MessageFromProcess::Message { recipient, message } => {
            context.send_message(recipient, message);
        }
        MessageFromProcess::SetTimer { ms_delay } => {
            context.set_timer(ms_delay);
        }
    }
    Ok(())
});
```

## Server Architecture

### ServerState

The server maintains state for all connected clients:

```rust
pub struct ServerState {
    pub handle: JoinHandle<()>,
    pub inbound_sender: Sender<Event>,
    pub senders: Arc<DashMap<ClientId, Sender<Message>>>,
    pub next_client_id: AtomicU32,
}
```

### Event Loop

```rust
pub enum Event {
    Message { client: ClientId, message: Message },
    Join { client: ClientId },
    Leave { client: ClientId },
    Timer,
}

// Service runs in a tokio task
let handle = tokio::spawn(async move {
    let context = Arc::new(ServerStateroomContext {
        senders: senders_.clone(),
        event_sender: Arc::new(tx_),
        timer_handle: Arc::new(Mutex::new(None)),
    });

    let mut service = factory.build("", context.clone()).unwrap();
    service.init(context.as_ref());

    loop {
        let msg = rx.recv().await;
        match msg {
            Some(Event::Message { client, message }) => {
                service.message(client, payload, context.as_ref())
            }
            Some(Event::Join { client }) => service.connect(client, context.as_ref()),
            Some(Event::Leave { client }) => service.disconnect(client, context.as_ref()),
            Some(Event::Timer) => service.timer(context.as_ref()),
            None => break,
        }
    }
});
```

### WebSocket Handler

```rust
async fn handle_socket(mut socket: WebSocket, state: Arc<ServerState>) {
    let (send, mut recv, client_id) = state.connect();

    loop {
        select! {
            msg = recv.recv() => {
                match msg {
                    Some(msg) => socket.send(msg).await.unwrap(),
                    None => break,
                }
            },
            msg = socket.recv() => {
                match msg {
                    Some(Ok(msg)) => send.send(Event::Message { client: client_id, message: msg }).await.unwrap(),
                    Some(Err(err)) => tracing::warn!(?err, "Error receiving message"),
                    None => break,
                }
            }
        }
    }

    state.remove(&client_id);
}
```

## Service Factory Pattern

```rust
pub trait StateroomServiceFactory: Send + Sync + 'static {
    type Service: StateroomService;
    type Error: std::fmt::Debug;

    fn build(
        &self,
        room_id: &str,
        context: Arc<impl StateroomContext>,
    ) -> Result<Self::Service, Self::Error>;
}
```

### Default Factory

```rust
#[derive(Default)]
pub struct DefaultStateroomFactory<T: StateroomService + Default> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: StateroomService + Default> StateroomServiceFactory for DefaultStateroomFactory<T> {
    type Service = T;
    type Error = Infallible;

    fn build(&self, _: &str, _: Arc<impl StateroomContext>) -> Result<Self::Service, Self::Error> {
        Ok(T::default())
    }
}
```

### WASM Host Factory

```rust
pub struct WasmHostFactory {
    module: Module,
    engine: Engine,
}

impl WasmHostFactory {
    pub fn new(path: &str) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, path)?;
        Ok(Self { module, engine })
    }
}

impl StateroomServiceFactory for WasmHostFactory {
    type Service = WasmHost;
    type Error = anyhow::Error;

    fn build(&self, room_id: &str, context: Arc<impl StateroomContext>) -> Result<Self::Service> {
        WasmHost::new(room_id, &self.module, &self.engine, context)
    }
}
```

## CLI Tool

The `stateroom-cli` provides commands for building and serving:

### Commands

```bash
# Build a WASM module
stateroom build

# Serve a WASM module locally
stateroom serve module.wasm --port 8080

# Development mode (build + serve)
stateroom dev
```

### Configuration

```toml
# stateroom.toml
name = "my-service"
version = "0.1.0"
port = 8080
```

## Timer System

Stateroom provides a simple timer primitive:

```rust
// Set a timer for 1000ms
ctx.set_timer(1000);

// Timer fires after delay
fn timer(&mut self, ctx: &impl StateroomContext) {
    // Handle timeout
}
```

### Implementation

```rust
fn set_timer(&self, ms_delay: u32) {
    let sender = self.event_sender.clone();
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(ms_delay as u64)).await;
        sender.send(Event::Timer).await.unwrap();
    });

    // Cancel previous timer
    let mut timer_handle = self.timer_handle.lock().unwrap();
    if let Some(h) = timer_handle.take() {
        h.abort();
    }
    *timer_handle = Some(handle);
}
```

Only one timer can be outstanding at a time. Complex timer behavior can be built using state.

## Room System

Each WebSocket connection creates or joins a "room":

```
/ws/room-name  -> Room "room-name"
```

- Any string is a valid room ID
- Connecting to a non-existent room creates it
- Multiple clients in the same room share state

### Room Isolation

Rooms are isolated from each other:
- Each room has its own service instance
- Messages don't cross room boundaries
- State is per-room

## Use Cases

### Real-time Chat

```rust
struct ChatRoom {
    messages: Vec<String>,
    clients: HashMap<ClientId, String>,
}
```

### Collaborative Editing

```rust
struct CollaborativeEditor {
    document: String,
    cursors: HashMap<ClientId, CursorPosition>,
}
```

### Multiplayer Games

```rust
struct GameState {
    players: HashMap<ClientId, Player>,
    board: Board,
    turn: ClientId,
}
```

### Live Dashboards

```rust
struct Dashboard {
    metrics: HashMap<String, Metric>,
    subscribers: Vec<ClientId>,
}
```

## Dependencies

```toml
[dependencies]
stateroom = "0.1.0"        # Core traits
stateroom-wasm = "0.1.0"   # WASM guest
stateroom-wasm-host = "0.1.0"  # WASM host
stateroom-server = "0.1.0" # Axum server
stateroom-cli = "0.1.0"    # CLI tool

# Internally
axum = { version = "0.7", features = ["ws"] }
tokio = { version = "1.33", features = ["sync", "time"] }
wasmtime = "14.0"          # WASM runtime
wasi-common = "14.0"       # WASI support
dashmap = "6.1"            # Concurrent HashMap
bincode = "1.3"            # Serialization
```

## Limitations

1. **Single-threaded**: Services run on a single thread
2. **In-memory state**: State is lost on restart
3. **No persistence**: Built-in persistence not provided
4. **WASM limits**: WASM modules have memory limits

## Integration with Aper

Stateroom integrates with [Aper](https://github.com/aper-dev/aper) for state synchronization:

```rust
use aper_stateroom::AperStateroomService;

struct MyAperService {
    state_machine: StateMachine<MyApp>,
}

impl StateroomService for MyAperService {
    // Aper handles state sync automatically
}
```

Aper provides:
- Atomic data structures (Atom, AtomMap)
- Listener pattern
- Transactional updates
- Conflict resolution

## Comparison to Alternatives

| Feature | Stateroom | SignalR | Socket.IO |
|---------|-----------|---------|-----------|
| Language | Rust | .NET | JS/TS |
| WASM Support | Yes | No | No |
| Threading | Single-threaded | Multi-threaded | Event loop |
| Rooms | Built-in | Built-in | Built-in |
| Persistence | External | External | External |
