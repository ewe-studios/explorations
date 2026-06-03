# Iroh Integration Guide for ewe-platform

## Overview

This guide provides comprehensive instructions for integrating iroh's P2P networking capabilities into the ewe-platform project.

**Target:** `/home/darkvoid/Boxxed/@dev/ewe_platform`
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/`

---

## Table of Contents

1. [Integration Architecture](#integration-architecture)
2. [Adding Iroh Dependencies](#adding-iroh-dependencies)
3. [Basic Integration](#basic-integration)
4. [P2P Communication Layer](#p2p-communication-layer)
5. [Service Discovery](#service-discovery)
6. [Distributed Features](#distributed-features)
7. [Production Deployment](#production-deployment)
8. [Testing Strategies](#testing-strategies)

---

## Integration Architecture

### Where Iroh Fits in ewe-platform

```
┌─────────────────────────────────────────────────────────────────┐
│                     ewe-platform                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Application Layer                           │    │
│  │  (AI services, Web backends, APIs)                      │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              ▲                                   │
│  ┌───────────────────────────┼────────────────────────────────┐ │
│  │         Foundation        │        Crates                   │ │
│  │  ┌─────────────┐          │         ┌─────────────┐        │ │
│  │  │ foundation_ │          │         │ ewe_platform│        │ │
│  │  │ core        │◄─────────┼────────►│ (P2P layer) │        │ │
│  │  └─────────────┘          │         └──────┬──────┘        │ │
│  │                           │                │                │ │
│  │  ┌─────────────┐          │         ┌──────▼──────┐        │ │
│  │  │ foundation_ │          │         │   iroh      │        │ │
│  │  │ runtimes    │          │         │  (P2P)      │        │ │
│  │  └─────────────┘          │         └──────┬──────┘        │ │
│  │                           │                │                │ │
│  │  ┌─────────────┐          │         ┌──────▼──────┐        │ │
│  │  │ foundation_ │          │         │   iroh_     │        │ │
│  │  │ auth        │          │         │  blobs      │        │ │
│  │  └─────────────┘          │         └─────────────┘        │ │
│  └───────────────────────────┴────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│              ┌───────────────────────────────┐                  │
│              │        Network Layer          │                  │
│              │  Direct P2P + Relay Fallback  │                  │
│              └───────────────────────────────┘                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Proposed Crate Structure

```
ewe_platform/
├── crates/
│   ├── platform/           # Existing core
│   └── p2p/                # NEW: Iroh-based P2P layer
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── node.rs     # P2P node management
│           ├── discovery.rs # Service discovery
│           ├── sync.rs     # Data synchronization
│           └── blobs.rs    # Large data transfer
├── backends/
│   └── p2p_backend/        # NEW: P2P backend services
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── api.rs      # P2P API endpoints
│           └── services/   # P2P-enabled services
└── infrastructure/
    └── relay/              # OPTIONAL: Self-hosted relay
        ├── Cargo.toml
        └── src/
            └── main.rs
```

---

## Adding Iroh Dependencies

### Step 1: Add to Workspace

Add the new crate to `Cargo.toml`:

```toml
# /home/darkvoid/Boxxed/@dev/ewe_platform/Cargo.toml

[workspace]
members = [
  # ... existing members ...
  "crates/p2p",
  "backends/p2p_backend",
]
```

### Step 2: Create P2P Crate Dependencies

```toml
# /home/darkvoid/Boxxed/@dev/ewe_platform/crates/p2p/Cargo.toml

[package]
name = "ewe_p2p"
version = "0.0.1"
edition.workspace = true
license.workspace = true

[dependencies]
# Core iroh
iroh = { version = "0.97", features = [
  "address-lookup-pkarr-dht",
  "address-lookup-mdns",
] }
iroh-base = "0.97"
iroh-blobs = "0.97"

# Async runtime
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
postcard = "1.0"

# Error handling
thiserror = "2"
anyhow = "1.0"

# Utilities
bytes = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }

# Crypto (if needed for additional features)
ed25519-dalek = "2"

# Workspace dependencies
foundation_core = { path = "../foundation_core" }
tracing.workspace = true

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
tempfile = "3"
```

### Step 3: Feature Flags for Flexibility

```toml
[features]
default = ["relay", "blobs"]

# Full relay support
relay = ["iroh/relay"]

# Blob transfer for large data
blobs = ["dep:iroh-blobs"]

# Local network discovery
mdns = ["iroh/address-lookup-mdns"]

# DHT-based discovery
dht = ["iroh/address-lookup-pkarr-dht"]

# Metrics and observability
metrics = ["iroh/metrics"]

# Development/test utilities
test-utils = ["iroh/test-utils"]
```

---

## Basic Integration

### Minimal P2P Node

```rust
// crates/p2p/src/node.rs

use iroh::{Endpoint, SecretKey, endpoint::presets};
use anyhow::Result;

/// P2P Node for ewe-platform
pub struct P2PNode {
    /// The iroh endpoint
    endpoint: Endpoint,

    /// Our node ID (public key)
    node_id: iroh::PublicKey,
}

impl P2PNode {
    /// Create a new P2P node
    pub async fn new() -> Result<Self> {
        // Generate or load existing key
        let secret_key = Self::load_or_generate_key().await?;
        let node_id = secret_key.public();

        // Create endpoint with production settings
        let endpoint = Endpoint::builder(presets::N0)
            .secret_key(secret_key)
            .alpn_protocols(vec![b"ewe-p2p-v1".to_vec()])
            .bind()
            .await?;

        Ok(Self { endpoint, node_id })
    }

    /// Load existing key or generate new one
    async fn load_or_generate_key() -> Result<SecretKey> {
        // In production, load from secure storage
        // For now, generate new
        Ok(SecretKey::generate(&mut rand::rng()))
    }

    /// Get our node ID
    pub fn node_id(&self) -> iroh::PublicKey {
        self.node_id
    }

    /// Get the underlying endpoint
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Close the node gracefully
    pub async fn close(self) -> Result<()> {
        self.endpoint.close().await;
        Ok(())
    }
}
```

### Connection Management

```rust
// crates/p2p/src/connection.rs

use iroh::{Endpoint, Connection, EndpointId};
use anyhow::Result;

/// Protocol version for ALPN
const EWE_ALPN: &[u8] = b"ewe-p2p-v1";

/// Connect to a peer
pub async fn connect_to_peer(
    endpoint: &Endpoint,
    peer_id: EndpointId,
) -> Result<Connection> {
    let conn = endpoint.connect(peer_id, EWE_ALPN).await?;
    Ok(conn)
}

/// Accept incoming connections
pub async fn accept_connection(endpoint: &Endpoint) -> Result<(Connection, EndpointId)> {
    let incoming = endpoint.accept().await.ok_or_else(|| {
        anyhow::anyhow!("Endpoint closed")
    })?;

    let conn = incoming.accept().await?;

    // Verify ALPN
    let alpn = incoming.alpn().await?;
    if alpn != EWE_ALPN {
        anyhow::bail!("Unexpected ALPN: {:?}", alpn);
    }

    let peer_id = incoming.remote_endpoint_id();

    Ok((conn, peer_id))
}

/// Send a message over a connection
pub async fn send_message(conn: &Connection, message: &[u8]) -> Result<()> {
    let (mut send, _recv) = conn.open_bi().await?;
    send.write_all(message).await?;
    send.finish()?;
    Ok(())
}

/// Receive messages from a connection
pub async fn receive_messages(
    conn: &Connection,
) -> Result<impl futures::Stream<Item = Result<Vec<u8>>>> {
    use futures::stream::try_unfold;

    let conn = conn.clone();
    let stream = try_unfold(conn, |conn| async move {
        let (_send, mut recv) = conn.accept_bi().await?;
        let data = recv.read_to_end(1024 * 1024).await?;  // 1MB limit
        Ok(Some((data, conn)))
    });

    Ok(stream)
}
```

---

## P2P Communication Layer

### Request-Response Pattern

```rust
// crates/p2p/src/rpc.rs

use serde::{Serialize, Deserialize};
use iroh::Connection;
use bytes::Bytes;
use anyhow::Result;

/// RPC request structure
#[derive(Serialize, Deserialize)]
pub struct RpcRequest<T> {
    pub id: u64,
    pub method: String,
    pub params: T,
}

/// RPC response structure
#[derive(Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub id: u64,
    pub result: Result<T, RpcError>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

/// Send RPC request and wait for response
pub async fn rpc_call<T: Serialize, R: for<'de> Deserialize<'de>>(
    conn: &Connection,
    method: &str,
    params: T,
    request_id: u64,
) -> Result<R> {
    let request = RpcRequest {
        id: request_id,
        method: method.to_string(),
        params,
    };

    // Serialize and send
    let request_bytes = postcard::to_allocvec(&request)?;
    let (mut send, mut recv) = conn.open_bi().await?;

    // Write request with length prefix
    send.write_all(&(request_bytes.len() as u32).to_be_bytes()).await?;
    send.write_all(&request_bytes).await?;
    send.finish()?;

    // Read response
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut response_bytes = vec![0u8; len];
    recv.read_exact(&mut response_bytes).await?;

    let response: RpcResponse<R> = postcard::from_bytes(&response_bytes)?;

    response.result.map_err(|e| {
        anyhow::anyhow!("RPC error {}: {}", e.code, e.message)
    })
}

/// Handle incoming RPC requests
pub async fn handle_rpc<F, T, R>(
    conn: &Connection,
    handler: F,
) -> Result<()>
where
    F: Fn(String, T) -> anyhow::Result<R> + Send + Sync + 'static,
    T: for<'de> Deserialize<'de>,
    R: Serialize,
{
    let (_send, mut recv) = conn.accept_bi().await?;

    // Read request
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut request_bytes = vec![0u8; len];
    recv.read_exact(&mut request_bytes).await?;

    let request: RpcRequest<T> = postcard::from_bytes(&request_bytes)?;

    // Handle request
    let result = handler(request.method, request.params);

    // Send response
    let response = RpcResponse {
        id: request.id,
        result: result.map_err(|e| RpcError {
            code: -1,
            message: e.to_string(),
        }),
    };

    let response_bytes = postcard::to_allocvec(&response)?;
    // ... send response back

    Ok(())
}
```

### Pub-Sub Pattern

```rust
// crates/p2p/src/pubsub.rs

use std::collections::HashMap;
use tokio::sync::broadcast;
use iroh::{Endpoint, EndpointId, Connection};
use anyhow::Result;

/// Topic-based pub-sub over P2P
pub struct PubSub {
    /// Topic subscriptions
    topics: broadcast::Sender<Vec<u8>>,

    /// Connected peers
    peers: HashMap<EndpointId, Connection>,
}

impl PubSub {
    pub fn new() -> Self {
        let (topics, _) = broadcast::channel(1000);
        Self {
            topics,
            peers: HashMap::new(),
        }
    }

    /// Subscribe to a topic
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.topics.subscribe()
    }

    /// Publish to all connected peers
    pub async fn publish(&mut self, message: Vec<u8>) -> Result<()> {
        // Broadcast locally
        let _ = self.topics.send(message.clone());

        // Send to all peers
        for conn in self.peers.values() {
            let (mut send, _) = conn.open_uni().await?;
            send.write_all(&message).await?;
            send.finish()?;
        }

        Ok(())
    }

    /// Add a peer connection
    pub fn add_peer(&mut self, peer_id: EndpointId, conn: Connection) {
        self.peers.insert(peer_id, conn);
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, peer_id: &EndpointId) {
        self.peers.remove(peer_id);
    }
}
```

---

## Service Discovery

### Local Network Discovery (mDNS)

```rust
// crates/p2p/src/discovery.rs

use iroh::{Endpoint, address_lookup};
use anyhow::Result;

/// Enable mDNS discovery for local network
pub async fn setup_mdns_discovery(endpoint: &Endpoint) -> Result<()> {
    #[cfg(feature = "mdns")]
    {
        let mdns = address_lookup::MdnsAddressLookup::builder();
        endpoint.address_lookup()?.add(mdns);
    }

    Ok(())
}

/// Publish our presence on local network
pub fn publish_mdns(node_id: iroh::PublicKey, port: u16) -> Result<()> {
    use mdns_sd::{ServiceDaemon, ServiceInfo};

    let daemon = ServiceDaemon::new()?;

    let service_type = "_ewe-platform._tcp.local.";
    let instance_name = format!("{}.{}", hex::encode(node_id), service_type);

    let service_info = ServiceInfo::new(
        service_type,
        &instance_name,
        "local-address",
        std::net::Ipv4Addr::UNSPECIFIED,
        port,
        &[("node_id", hex::encode(node_id))],
    )?;

    daemon.register(service_info)?;

    Ok(())
}
```

### Pkarr/DNS Discovery

```rust
use iroh::address_lookup::{PkarrPublisher, DnsAddressLookup};

/// Set up global discovery via Pkarr/DNS
pub async fn setup_global_discovery(endpoint: &Endpoint) -> Result<()> {
    // Publish to n0's DNS/Pkarr service
    endpoint.address_lookup()?.add(PkarrPublisher::n0_dns());

    // Enable DNS resolution
    endpoint.address_lookup()?.add(DnsAddressLookup::n0_dns());

    Ok(())
}

/// Resolve a peer's address via DNS
pub async fn resolve_peer_dns(
    endpoint: &Endpoint,
    peer_id: iroh::PublicKey,
) -> Result<iroh::EndpointAddr> {
    // This happens automatically when you connect
    // But you can also explicitly resolve
    let lookup = endpoint.address_lookup()?;
    let stream = lookup.resolve(peer_id)
        .ok_or_else(|| anyhow::anyhow!("No lookup service"))?;

    // Get first result
    use futures::StreamExt;
    if let Some(result) = stream.take(1).collect::<Vec<_>>().await.first() {
        Ok(result?.into())
    } else {
        anyhow::bail!("No address found for peer");
    }
}
```

---

## Distributed Features

### Distributed Blob Storage

```rust
// crates/p2p/src/blobs.rs

use iroh_blobs::{Store, Provider, downloader::Downloader};
use iroh::{Endpoint, PublicKey};
use anyhow::Result;

/// Blob storage and transfer
pub struct BlobStore {
    store: Store,
    provider: Provider,
}

impl BlobStore {
    pub async fn new(endpoint: Endpoint) -> Result<Self> {
        let store = Store::memory();
        let provider = Provider::builder(store.clone())
            .bind_to(endpoint)
            .spawn()
            .await?;

        Ok(Self { store, provider })
    }

    /// Add data to the store
    pub async fn add(&self, data: impl AsRef<[u8]>) -> Result<iroh_blobs::Hash> {
        let hash = self.store.insert(data.as_ref()).await?;
        Ok(hash)
    }

    /// Get data from the store
    pub async fn get(&self, hash: &iroh_blobs::Hash) -> Result<Vec<u8>> {
        let blob = self.store.get(hash).await?;
        let data = blob.read_to_bytes().await?;
        Ok(data.to_vec())
    }

    /// Request blob from a peer
    pub async fn request_from_peer(
        &self,
        peer_id: PublicKey,
        hash: &iroh_blobs::Hash,
    ) -> Result<Vec<u8>> {
        let downloader = Downloader::new(self.store.clone());

        // This will connect to the peer and download the blob
        downloader.download(hash, peer_id).await?;

        // Now available in local store
        self.get(hash).await
    }
}
```

### Distributed State Sync

```rust
// crates/p2p/src/sync.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use iroh::{Endpoint, Connection, PublicKey};
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// CRDT-based state synchronization
pub struct StateSync<T> {
    /// Local state
    state: Arc<RwLock<T>>,

    /// Known peers
    peers: Arc<RwLock<Vec<PublicKey>>>,
}

impl<T> StateSync<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
{
    pub fn new(initial: T) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial)),
            peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get current state
    pub async fn get(&self) -> T {
        self.state.read().await.clone()
    }

    /// Update state and broadcast to peers
    pub async fn update<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut T) + Send + 'static,
    {
        // Update local state
        {
            let mut state = self.state.write().await;
            f(&mut state);
        }

        // Broadcast to peers
        self.broadcast_update().await?;

        Ok(())
    }

    /// Broadcast state to all peers
    async fn broadcast_update(&self) -> Result<()> {
        let state = self.state.read().await;
        let state_bytes = postcard::to_allocvec(&*state)?;

        let peers = self.peers.read().await.clone();

        // Send to all peers (implementation depends on connection management)
        for peer_id in peers {
            // Would use existing connection
            // send_to_peer(peer_id, &state_bytes).await?;
        }

        Ok(())
    }

    /// Apply update received from peer
    pub async fn apply_update(&self, update: Vec<u8>) -> Result<()> {
        let update: T = postcard::from_bytes(&update)?;

        // Merge with local state (CRDT merge logic depends on T)
        let mut state = self.state.write().await;
        // merge_crdT(&mut *state, &update);

        Ok(())
    }
}
```

---

## Production Deployment

### Configuration

```rust
// crates/p2p/src/config.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConfig {
    /// Relay servers to use
    pub relay_servers: Vec<String>,

    /// Enable local discovery
    pub enable_mdns: bool,

    /// Enable global discovery
    pub enable_pkarr: bool,

    /// Connection timeout
    pub connection_timeout_secs: u64,

    /// Maximum concurrent connections
    pub max_connections: u32,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            relay_servers: vec![
                "https://relay.iroh.network".to_string(),
            ],
            enable_mdns: true,
            enable_pkarr: true,
            connection_timeout_secs: 10,
            max_connections: 100,
        }
    }
}
```

### Self-Hosted Relay (Optional)

```rust
// infrastructure/relay/src/main.rs

use iroh_relay::server::Relay;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = iroh_relay::server::Config {
        addr: "0.0.0.0:8443".parse()?,
        tls: Some(iroh_relay::server::TlsConfig {
            cert: "/path/to/cert.pem".into(),
            key: "/path/to/key.pem".into(),
        }),
        // ... other config
    };

    let relay = Relay::new(config).await?;
    relay.serve().await?;

    Ok(())
}
```

### Health Monitoring

```rust
// crates/p2p/src/health.rs

use iroh::Endpoint;
use tokio::time::{interval, Duration};
use anyhow::Result;

/// Monitor P2P health
pub struct HealthMonitor {
    endpoint: Endpoint,
}

impl HealthMonitor {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }

    /// Start health monitoring
    pub async fn run(self) {
        let mut check_interval = interval(Duration::from_secs(30));

        loop {
            check_interval.tick().await;

            // Get network report
            let report = self.endpoint.net_report().await;

            // Log status
            tracing::info!(
                ipv4 = report.ipv4,
                ipv6 = report.ipv6,
                udp = report.udp,
                relay = ?report.preferred_relay,
                "P2P health check"
            );

            // Alert on issues
            if !report.udp && report.preferred_relay.is_none() {
                tracing::error!("P2P connectivity severely degraded");
            }
        }
    }
}
```

---

## Testing Strategies

### Unit Tests

```rust
// crates/p2p/src/tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::endpoint::presets;

    #[tokio::test]
    async fn test_node_creation() -> Result<()> {
        let node = P2PNode::new().await?;
        assert!(!node.node_id().as_bytes().iter().all(|&b| b == 0));
        node.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_local_connection() -> Result<()> {
        // Create two nodes
        let node1 = P2PNode::new().await?;
        let node2 = P2PNode::new().await?;

        // Connect
        let conn = connect_to_peer(
            node1.endpoint(),
            node2.node_id(),
        ).await?;

        assert!(conn.is_valid());

        Ok(())
    }
}
```

### Integration Tests

```rust
// tests/p2p_integration.rs

use ewe_p2p::{P2PNode, connect_to_peer, send_message};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_full_rpc_roundtrip() -> Result<()> {
    let node1 = P2PNode::new().await?;
    let node2 = P2PNode::new().await?;

    // Set up RPC handler on node2
    let handler_task = tokio::spawn({
        let endpoint = node2.endpoint().clone();
        async move {
            let incoming = endpoint.accept().await.unwrap();
            let conn = incoming.accept().await.unwrap();
            handle_rpc(&conn, |method, params: String| {
                Ok(format!("Echo: {}", params))
            }).await
        }
    });

    // Make RPC call from node1
    let conn = connect_to_peer(node1.endpoint(), node2.node_id()).await?;
    let response: String = timeout(
        Duration::from_secs(5),
        rpc_call(&conn, "echo", "hello".to_string(), 1),
    ).await??;

    assert_eq!(response, "Echo: hello");

    Ok(())
}
```

---

## Migration Path

### Phase 1: Basic Integration (Week 1-2)
- Add iroh dependency
- Create basic P2P node wrapper
- Enable direct connections between nodes
- Test on local network

### Phase 2: Discovery (Week 3-4)
- Implement mDNS for local discovery
- Add Pkarr/DNS for global discovery
- Test across different networks

### Phase 3: Advanced Features (Week 5-6)
- Add blob transfer for large data
- Implement pub-sub for events
- Add RPC for service calls

### Phase 4: Production Hardening (Week 7-8)
- Add monitoring and health checks
- Configure relay servers
- Load testing
- Documentation

---

## Quick Start Example

```rust
// examples/basic_p2p.rs

use ewe_p2p::{P2PNode, connect_to_peer, send_message};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create node
    let node = P2PNode::new().await?;
    println!("Node ID: {}", node.node_id());

    // In a real app, you'd share this ID and connect to other nodes
    // For now, just demonstrate the API
    println!("P2P node ready!");

    // Clean shutdown
    node.close().await?;

    Ok(())
}
```

---

## See Also

- [exploration.md](./exploration.md) - Main iroh exploration
- [p2p-ground-up.md](./p2p-ground-up.md) - Building P2P from scratch
- [iroh documentation](https://docs.rs/iroh)
- [n0.computer](https://n0.computer) - Iroh creators
