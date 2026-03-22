# iroh-ffi Deep Dive

## Overview

`iroh-ffi` provides Foreign Function Interface (FFI) bindings for the iroh ecosystem, enabling integration with non-Rust languages and environments. The crate uses UniFFI for generating safe, idiomatic bindings.

**Version:** 0.35.0
**Repository:** https://github.com/n0-computer/iroh-ffi
**License:** MIT OR Apache-2.0

---

## Architecture and Design Decisions

### UniFFI-Based Bindings

The crate uses [UniFFI](https://mozilla.github.io/uniffi-rs/) for FFI generation:

1. **Automatic Binding Generation**: UniFFI generates bindings for multiple target languages
2. **Type Safety**: Generated bindings maintain Rust's type safety guarantees
3. **Async Support**: Full support for async Rust code in foreign languages
4. **Memory Safety**: Automatic memory management through UniFFI's runtime

### Supported Targets

UniFFI generates bindings for:
- **Kotlin/JVM**: Android and JVM applications
- **Swift**: iOS and macOS applications
- **Python**: Python applications and scripts
- **Ruby**: Ruby applications

### Design Principles

1. **Idiomatic Target APIs**: Generated APIs feel native to each target language
2. **Error Translation**: Rust errors are converted to target-language exceptions
3. **Callback Support**: Foreign code can implement Rust traits
4. **Minimal Boilerplate**: Most FFI boilerplate is auto-generated

### Crate Structure

```
iroh-ffi/
├── src/
│   ├── lib.rs           # Main entry point, UniFFI scaffolding
│   ├── author.rs        # Author FFI types
│   ├── blob.rs          # Blob operation bindings
│   ├── doc.rs           # Document bindings
│   ├── endpoint.rs      # Network endpoint bindings
│   ├── error.rs         # Error type definitions
│   ├── gossip.rs        # Gossip protocol bindings
│   ├── key.rs           # Key type bindings
│   ├── net.rs           # Network bindings
│   ├── node.rs          # Node bindings
│   ├── tag.rs           # Tag bindings
│   └── ticket.rs        # Ticket bindings
├── uniffi-bindgen.rs    # UniFFI binding generator binary
├── uniffi.toml          # UniFFI configuration
└── Cargo.toml
```

---

## Key APIs and Data Structures

### UniFFI Configuration

```toml
# uniffi.toml
[bindings.kotlin]
package_name = "com.n0.iroh"
cdylib_name = "iroh_ffi"

[bindings.swift]
cdylib_name = "iroh_ffi"

[bindings.python]
cdylib_name = "iroh_ffi"
```

### Main Library Setup

```rust
use uniffi::prelude::*;

// Export all modules
pub mod author;
pub mod blob;
pub mod doc;
pub mod endpoint;
pub mod error;
pub mod gossip;
pub mod key;
pub mod net;
pub mod node;
pub mod tag;
pub mod ticket;

pub use self::author::*;
pub use self::blob::*;
pub use self::doc::*;
pub use self::endpoint::*;
pub use self::error::*;
pub use self::gossip::*;
pub use self::key::*;
pub use self::net::*;
pub use self::node::*;
pub use self::tag::*;
pub use self::ticket::*;

// UniFFI scaffolding
uniffi::setup_scaffolding!();
```

### Error Types

```rust
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum IrohError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Document error: {0}")]
    Doc(String),

    #[error("Blob error: {0}")]
    Blob(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Operation cancelled")]
    Cancelled,
}

impl From<std::io::Error> for IrohError {
    fn from(e: std::io::Error) -> Self {
        IrohError::Io(e.to_string())
    }
}

pub type IrohResult<T> = Result<T, IrohError>;
```

### Key Types

```rust
/// Node public key
#[derive(Clone, uniffi::Object)]
pub struct NodeKey {
    inner: iroh::PublicKey,
}

#[uniffi::export]
impl NodeKey {
    /// Generate new random key
    #[uniffi::constructor]
    pub fn generate() -> Self {
        Self {
            inner: iroh::SecretKey::generate().public(),
        }
    }

    /// Parse from hex string
    #[uniffi::constructor]
    pub fn from_hex(hex: String) -> IrohResult<Self> {
        let bytes = hex::decode(hex).map_err(|e| IrohError::InvalidKey(e.to_string()))?;
        let inner = iroh::PublicKey::from_bytes(&bytes)
            .map_err(|e| IrohError::InvalidKey(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.inner.as_bytes())
    }

    /// Get short display string
    pub fn fmt_short(&self) -> String {
        self.inner.fmt_short()
    }
}
```

### Endpoint

```rust
/// Network endpoint for p2p communication
#[derive(uniffi::Object)]
pub struct Endpoint {
    inner: iroh::Endpoint,
}

#[uniffi::export]
impl Endpoint {
    /// Create new endpoint builder
    #[uniffi::constructor]
    pub fn builder() -> EndpointBuilder {
        EndpointBuilder::default()
    }

    /// Get node ID
    pub fn node_id(&self) -> Arc<NodeKey> {
        Arc::new(NodeKey {
            inner: self.inner.node_id(),
        })
    }

    /// Connect to a node
    pub async fn connect(&self, addr: NodeAddr) -> IrohResult<Connection> {
        self.inner
            .connect(addr.inner)
            .await
            .map(|c| Connection { inner: c })
            .map_err(|e| e.into())
    }

    /// Accept connections
    pub async fn accept(&self) -> IrohResult<IncomingConnection> {
        // Accept loop implementation
    }
}

/// Builder for endpoint configuration
#[derive(uniffi::Record)]
pub struct EndpointConfig {
    pub secret_key: Option<String>,
    pub relay_mode: RelayMode,
    pub alpn_protocols: Vec<String>,
}
```

### Document Operations

```rust
/// Document handle
#[derive(uniffi::Object)]
pub struct Document {
    inner: iroh_docs::engine::Doc<BlobStore>,
}

#[uniffi::export]
impl Document {
    /// Get document ID (namespace)
    pub fn id(&self) -> Vec<u8> {
        self.inner.namespace().as_bytes().to_vec()
    }

    /// Set content at key
    pub async fn set_bytes(
        &self,
        key: String,
        content: Vec<u8>,
    ) -> IrohResult<()> {
        self.inner
            .set_bytes(key.as_bytes(), content.into())
            .await
            .map_err(|e| IrohError::Doc(e.to_string()))
    }

    /// Get content at key
    pub async fn get_bytes(&self, key: String) -> IrohResult<Option<Vec<u8>>> {
        self.inner
            .get_latest(key.as_bytes())
            .await
            .map_err(|e| IrohError::Doc(e.to_string()))?
            .map(|entry| {
                self.inner
                    .get_content(entry.content_hash())
                    .map(|b| b.to_vec())
            })
            .transpose()
            .map_err(|e| IrohError::Doc(e.to_string()))
    }

    /// Subscribe to document events
    pub async fn subscribe(&self) -> IrohResult<DocumentEventStream> {
        // Event stream implementation
    }
}
```

### Logging

```rust
/// Logging levels
#[derive(Debug, uniffi::Enum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

/// Set global log level
#[uniffi::export]
pub fn set_log_level(level: LogLevel) {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, reload};

    let filter: tracing_subscriber::filter::LevelFilter = level.into();
    let (filter, _) = reload::Layer::new(filter);
    let mut layer = fmt::Layer::default();
    layer.set_ansi(false);
    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .init();
}
```

---

## Protocol Details

### FFI Boundary Considerations

1. **Ownership Transfer**: Clear ownership semantics across FFI boundary
2. **Error Mapping**: Rust errors mapped to target-language exceptions
3. **Async Bridging**: Async Rust methods exposed as async/await in target languages
4. **Callback Handling**: Foreign callbacks called with proper threading

### Memory Management

```rust
// Objects are reference-counted across FFI boundary
#[derive(uniffi::Object)]
pub struct Document {
    inner: Arc<iroh_docs::engine::Doc<BlobStore>>,
}

// Cloning is cheap (Arc clone)
#[uniffi::export]
impl Document {
    pub fn clone(&self) -> Arc<Self> {
        Arc::clone(self)
    }
}
```

### Callback Support

```rust
/// Callback trait for document events
#[uniffi::export(callback_interface)]
pub trait DocumentEventCallback: Send + Sync {
    fn on_event(&self, event: DocumentEvent);
}

pub struct DocumentEventStream {
    callback: Option<Arc<dyn DocumentEventCallback>>,
    // ...
}
```

---

## Integration with Main Iroh Endpoint

### Complete Example

```rust
/// Main iroh client
#[derive(uniffi::Object)]
pub struct IrohClient {
    endpoint: Arc<Endpoint>,
    docs: Arc<DocEngine>,
    blobs: Arc<BlobStore>,
}

#[uniffi::export]
impl IrohClient {
    /// Create new client
    #[uniffi::constructor]
    pub async fn new(config: ClientConfig) -> IrohResult<Self> {
        let endpoint = Endpoint::builder()
            .secret_key(config.secret_key)
            .relay_mode(config.relay_mode)
            .bind()
            .await?;

        let blobs = BlobStore::memory()?;
        let docs = DocEngine::spawn(endpoint.clone(), blobs.clone()).await?;

        Ok(Self {
            endpoint: Arc::new(endpoint),
            docs: Arc::new(docs),
            blobs: Arc::new(blobs),
        })
    }

    /// Get document by ID
    pub async fn get_document(&self, id: Vec<u8>) -> IrohResult<Arc<Document>> {
        let namespace = NamespaceId::from_bytes(&id)
            .map_err(|e| IrohError::Doc(e.to_string()))?;

        let doc = self.docs.open(namespace).await?;
        Ok(Arc::new(Document { inner: doc }))
    }

    /// Create new document
    pub async fn create_document(&self) -> IrohResult<Arc<Document>> {
        let doc = self.docs.create().await?;
        Ok(Arc::new(Document { inner: doc }))
    }
}
```

---

## Production Usage Patterns

### Kotlin/Android

```kotlin
// Android example
class IrohService : Service() {
    private lateinit var client: IrohClient
    private var document: Document? = null

    override fun onCreate() {
        super.onCreate()

        // Initialize logging
        IrohFfi.setLogLevel(LogLevel.DEBUG)

        // Create client
        lifecycleScope.launch {
            client = IrohClient(ClientConfig())

            // Create document
            document = client.createDocument()

            // Set content
            document?.setBytes("hello.txt", "Hello from Android!".toByteArray())

            // Subscribe to events
            val events = document?.subscribe()
            events?.collect { event ->
                when (event) {
                    is DocumentEvent.Insert -> {
                        Log.d("Iroh", "New entry: ${event.key}")
                    }
                }
            }
        }
    }
}
```

### Swift/iOS

```swift
// iOS example
class IrohManager: ObservableObject {
    @Published var client: IrohClient?
    @Published var document: Document?

    func setup() async throws {
        // Create client
        let config = ClientConfig()
        client = try await IrohClient.new(config: config)

        // Create document
        document = try await client?.createDocument()

        // Set content
        try await document?.setBytes(
            key: "hello.txt",
            content: "Hello from iOS!".data(using: .utf8)!
        )
    }

    func getContent(key: String) async throws -> Data? {
        try await document?.getBytes(key: key)
    }
}
```

### Python

```python
# Python example
import asyncio
from iroh_ffi import IrohClient, ClientConfig, LogLevel, set_log_level

async def main():
    # Enable debug logging
    set_log_level(LogLevel.DEBUG)

    # Create client
    config = ClientConfig()
    client = await IrohClient.new(config)

    # Create document
    doc = await client.create_document()

    # Set content
    await doc.set_bytes("hello.txt", b"Hello from Python!")

    # Get content
    content = await doc.get_bytes("hello.txt")
    print(f"Content: {content}")

    # List entries
    async for entry in doc.get_many():
        print(f"Entry: {entry.key}")

asyncio.run(main())
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| uniffi | 0.28.0 | FFI binding generation |
| iroh | 0.35 | Core iroh functionality |
| iroh-docs | 0.35 | Document operations |
| iroh-blobs | 0.35 | Blob operations |
| thiserror | 1.0 | Error type derivation |
| tokio | 1.25 | Async runtime |
| tracing-subscriber | 0.3.17 | Logging |

### Build Configuration

```toml
[lib]
name = "iroh_ffi"
crate-type = ["staticlib", "cdylib"]

[[bin]]
name = "uniffi-bindgen"
path = "uniffi-bindgen.rs"

[build-dependencies]
uniffi = { version = "0.28.0", features = ["build"] }
```

### UniFFI Interface Definition

UniFFI can also use `.udl` (Interface Definition Language) files:

```udl
// iroh.udl
namespace iroh {
    IrohClient create_client(ClientConfig config);
};

dictionary ClientConfig {
    string? secret_key;
    RelayMode relay_mode;
};

interface Document {
    sequence<u8> id();
    void set_bytes(string key, sequence<u8> content);
    sequence<u8>? get_bytes(string key);
};

callback interface DocumentEventCallback {
    void on_event(DocumentEvent event);
};
```

### Potential Enhancements

1. **More Language Targets**: Add support for additional languages
2. **Streaming APIs**: Better support for streaming in foreign languages
3. **Platform-Specific Optimizations**: Native optimizations per platform
4. **Binding Tests**: Automated testing of generated bindings

---

## Summary

`iroh-ffi` provides:

- **Multi-Language Support**: Kotlin, Swift, Python, Ruby bindings
- **Type Safety**: Generated bindings preserve Rust type safety
- **Async Support**: Full async/await in target languages
- **Memory Safety**: Automatic memory management via UniFFI
- **Callback Support**: Foreign code can implement Rust callbacks

The crate enables building native mobile and desktop applications using iroh's distributed networking capabilities.
