# Iroh Examples Summary

## Overview

The `iroh-examples` repository contains practical examples demonstrating how to use iroh and its library crates. These examples range from simple demos to full applications, showcasing the capabilities of the iroh P2P networking stack.

**Repository:** https://github.com/n0-computer/iroh-examples
**License:** MIT OR Apache-2.0

## Example Catalog

### Browser-Based Examples

#### browser-echo

**Purpose:** Demonstrate iroh running in WebAssembly in the browser

**Live Demo:** https://n0-computer.github.io/iroh-examples/main/browser-echo/index.html

**Architecture:**
```
browser-echo/
├── src/
│   ├── lib.rs      - Shared library code
│   ├── node.rs     - Iroh node implementation
│   └── wasm.rs     - WASM bindings
└── Cargo.toml
```

**Key Features:**
- Compiles Rust to WebAssembly
- Browser-based iroh node
- Demonstrates P2P connectivity from browser
- Uses wasm-bindgen for JS interop

**Technical Highlights:**
```rust
// WASM-specific endpoint configuration
let endpoint = Endpoint::builder()
    .alpns(vec![ALPN.to_vec()])
    .relay_mode(RelayMode::Custom(relay_map))
    .bind(0)
    .await?;
```

#### browser-chat

**Purpose:** Real-time chat application running in browser and CLI

**Live Demo:** https://n0-computer.github.io/iroh-examples/main/browser-chat/index.html

**Architecture:**
```
browser-chat/
├── shared/         - Shared protocol definitions
├── browser-wasm/   - WebAssembly frontend
├── cli/            - Command-line interface
└── Cargo.toml (workspace)
```

**Key Features:**
- iroh-gossip based chat
- Cross-platform (browser + CLI)
- End-to-end encrypted
- Serverless architecture

**Protocol Definition:**
```rust
#[derive(Debug, Serialize, Deserialize)]
enum ChatMessage {
    Join { nickname: String },
    Message { text: String },
    Leave,
}
```

#### dumbpipe-web

**Purpose:** Web interface for dumbpipe functionality

**Features:**
- Forward HTTP requests to dumbpipe
- Share local dev server publicly
- Browser-based tunnel management

### Protocol Examples

#### framed-messages

**Purpose:** Demonstrate message framing over bidirectional streams

**Key Concept:** Using `tokio-util` codec for message framing

```rust
use tokio_util::codec::{Framed, LengthDelimitedCodec};

// Frame chess moves on a bidirectional stream
let framed = Framed::new(stream, LengthDelimitedCodec::new());
```

**Use Case:** Send structured messages (like chess moves) over raw QUIC streams

#### iroh-automerge

**Purpose:** Integrate iroh with Automerge CRDT library

**Architecture:**
```rust
// Protocol for syncing automerge documents
#[rpc_requests(AutomergeService, message = AutomergeMessage)]
enum AutomergeProtocol {
    #[rpc(tx = oneshot::Sender<DocumentState>)]
    GetDocument(DocumentId),
    #[rpc(tx = mpsc::Sender<Change>)]
    Subscribe(DocumentId),
}
```

**Features:**
- CRDT-based document sync
- Real-time collaboration
- Conflict-free merges

#### iroh-automerge-repo

**Purpose:** Automerge repo implementation over iroh

**Key Components:**
- Codec for automerge messages
- Sync protocol implementation
- Document storage

### Infrastructure Examples

#### iroh-gateway

**Purpose:** HTTP gateway for iroh-blobs content

**Features:**
- Serve IPFS-style content over HTTP
- Range request support
- Content-type detection
- CORS headers
- systemd integration

**Architecture:**
```
iroh-gateway/
├── src/
│   ├── args.rs       - CLI arguments
│   ├── cert_util.rs  - TLS certificate handling
│   ├── ranges.rs     - HTTP range handling
│   └── main.rs       - Main application
├── systemd/          - systemd service files
└── self-signed-certs/
```

**Usage:**
```bash
# Run gateway
iroh-gateway --addr 0.0.0.0:8080

# Access content
curl http://localhost:8080/ipfs/<hash>
```

#### extism

**Purpose:** Use iroh through Extism plugin system

**Architecture:**
```
extism/
├── host/                       - Extism host application
├── iroh-extism-host-functions/ - Host function definitions
└── plugin/                     - WASM plugin using iroh
```

**Key Concept:** Extism provides safe WASM plugin hosting with iroh integration

### Application Examples

#### frosty

**Purpose:** Experiment with FROST threshold signatures for iroh

**Background:** FROST (Flexible Round-Optimized Schnorr Threshold) signatures

**Features:**
- Threshold key management
- Distributed signing
- Integration with iroh authentication

**Use Case:** Multi-party control over iroh resources

#### tauri-todos

**Purpose:** Cross-platform todo app using iroh and Tauri

**Tech Stack:**
- Tauri v2 for desktop app
- React frontend
- iroh documents for sync
- Local-first architecture

**Features:**
- Offline-first todo management
- P2P sync between devices
- No cloud dependency

## Example Categories

### By Platform

| Example | Browser | CLI | Desktop | Mobile |
|---------|---------|-----|---------|--------|
| browser-echo | ✓ | | | |
| browser-chat | ✓ | ✓ | | |
| dumbpipe-web | ✓ | | | |
| framed-messages | | ✓ | | |
| iroh-gateway | | ✓ | | |
| tauri-todos | | | ✓ | ✓ |

### By Use Case

| Category | Examples |
|----------|----------|
| Communication | browser-chat, framed-messages |
| File Transfer | dumbpipe-web, iroh-gateway |
| Collaboration | iroh-automerge, iroh-automerge-repo, tauri-todos |
| Security | frosty |
| Browser | browser-echo, browser-chat |
| Plugins | extism |

## Technical Patterns

### Pattern 1: Service Definition

Most examples define an iroh service:

```rust
#[derive(Debug, Clone)]
pub struct MyService;

impl Service for MyService {
    type Req = MyRequest;
    type Res = MyResponse;
}
```

### Pattern 2: Protocol Messages

Using `irpc` or `quic-rpc` for typed protocols:

```rust
#[rpc_requests(MyService, message = MyMessage)]
#[derive(Debug, Serialize, Deserialize)]
enum MyProtocol {
    #[rpc(tx = oneshot::Sender<Response>)]
    MyRequest(Request),
}
```

### Pattern 3: Endpoint Setup

Common endpoint configuration:

```rust
let endpoint = Endpoint::builder()
    .alpns(vec![MyProtocol::ALPN.to_vec()])
    .relay_mode(RelayMode::Default)
    .bind(0)
    .await?;
```

### Pattern 4: Connection Handling

```rust
async fn handle_connections(endpoint: Endpoint) -> Result<()> {
    loop {
        let incoming = endpoint.accept().await?;
        tokio::spawn(async move {
            handle_connection(incoming).await
        });
    }
}
```

### Pattern 5: Browser Integration

For WASM targets:

```rust
#[wasm_bindgen]
pub async fn connect(ticket: String) -> Result<JsValue> {
    let endpoint = create_endpoint().await?;
    // ... connection logic
    Ok(JsValue::from_str("connected"))
}
```

## Running Examples

### Prerequisites

- Rust 1.75+ (varies by example)
- For browser examples: `wasm-pack`, `wasm-bindgen-cli`
- For Tauri: Tauri CLI, system dependencies

### Building Browser Examples

```bash
# Install WASM toolchain
rustup target add wasm32-unknown-unknown

# Build browser-echo
cd browser-echo
wasm-pack build --target web --out-dir pkg

# Serve
python -m http.server 8080
```

### Building Desktop Examples

```bash
# Build tauri-todos
cd tauri-todos
cargo tauri dev  # Development
cargo tauri build # Production
```

## Learning Path

### Beginner

1. **framed-messages** - Simple message passing
2. **browser-echo** - Basic browser integration

### Intermediate

1. **browser-chat** - Full application with sync
2. **iroh-gateway** - HTTP integration
3. **iroh-automerge** - CRDT integration

### Advanced

1. **extism** - WASM plugin system
2. **frosty** - Cryptographic protocols
3. **tauri-todos** - Full desktop application

## Best Practices Demonstrated

### Error Handling

```rust
use anyhow::{Context, Result};

async fn example() -> Result<()> {
    let endpoint = Endpoint::builder()
        .bind(0)
        .await
        .context("failed to bind endpoint")?;
    Ok(())
}
```

### Logging

```rust
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn setup_logging() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
}
```

### Configuration

```rust
#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080")]
    addr: SocketAddr,

    #[arg(long, env = "IROH_RELAY")]
    relay_url: Option<String>,
}
```

## Integration with Other n0-computer Projects

| Example | Uses |
|---------|------|
| browser-chat | iroh-gossip |
| iroh-gateway | iroh-blobs |
| iroh-automerge | quic-rpc, automerge |
| tauri-todos | iroh-sync |
| frosty | iroh, frost |
| extism | extism, iroh |

## Future Directions

Potential new examples:

1. **Video conferencing** - Real-time media with iroh
2. **Distributed compute** - Task distribution
3. **Gaming** - Multiplayer game server
4. **IoT hub** - Device management
5. **Backup service** - P2P backup system

## Related Resources

- [Iroh Documentation](https://iroh.computer/docs)
- [Iroh Examples Repository](https://github.com/n0-computer/iroh-examples)
- [Iroh Experiments](https://github.com/n0-computer/iroh-experiments) - More experimental examples
- [Awesome Iroh](https://github.com/n0-computer/awesome-iroh) - Community projects

## Conclusion

The iroh-examples repository provides a comprehensive set of examples demonstrating:
- Browser and desktop P2P applications
- Protocol design patterns
- Integration with other ecosystems (Automerge, Tauri, Extism)
- Production-ready patterns (gateway, systemd)

These examples serve as starting points for building your own iroh-powered applications.
