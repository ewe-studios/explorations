# iroh-relay Deep Dive

## Overview

`iroh-relay` provides the relay server and client implementation for iroh's peer-to-peer networking system. The relay helps establish connections between devices when direct P2P connections aren't immediately possible, temporarily routing encrypted traffic until a direct path can be established.

**Version:** 0.34.1
**Repository:** https://github.com/n0-computer/iroh
**License:** MIT OR Apache-2.0

---

## Architecture and Design Decisions

### Purpose and Role

The relay server serves several critical functions in the iroh ecosystem:

1. **Hole Punching Assistant**: Helps establish direct P2P connections through NAT and firewalls
2. **Fallback Path**: Provides a reliable communication path when direct connection fails
3. **Connection Bootstrap**: Enables initial peer discovery and connection setup
4. **STUN Server**: Optionally provides STUN services for NAT traversal

### DERP Protocol Foundation

The relay protocol is based on Tailscale's DERP (Designated Encrypted Relay for Packets) protocol, with revisions for iroh's specific needs:

1. **HTTP/HTTPS Transport**: Relay traffic runs over standard HTTP/HTTPS ports (80/443)
2. **QUIC Support**: Optional QUIC endpoint for enhanced discovery
3. **Connection Forwarding**: Relays encrypted packets between connected clients
4. **Presence Detection**: Tracks which nodes are connected to which relays

### Server Architecture

The relay server uses a structured concurrency model:

```
┌─────────────────────────────────────────────────────────┐
│                   Relay Server                          │
├─────────────────────────────────────────────────────────┤
│  HTTP/HTTPS Server           QUIC Server (optional)     │
│  ┌────────────────────┐      ┌────────────────────┐     │
│  │ /relay (WebSocket) │      │ QUIC ALPN          │     │
│  │ /ping              │      │ Port: 7842         │     │
│  │ /generate_204      │      │                    │     │
│  │ /probe             │      │                    │     │
│  └────────────────────┘      └────────────────────┘     │
├─────────────────────────────────────────────────────────┤
│  STUN Server (optional)       Metrics (optional)        │
│  ┌────────────────────┐      ┌────────────────────┐     │
│  │ UDP Port: 3478     │      │ HTTP Port: 9090    │     │
│  │ RFC 5389 STUN      │      │ Prometheus format  │     │
│  └────────────────────┘      └────────────────────┘     │
├─────────────────────────────────────────────────────────┤
│                    Connection Manager                    │
│  ┌───────────────┐  ┌───────────────┐  ┌─────────────┐  │
│  │ Client Index  │  │ Packet Queue  │  │ Key Cache   │  │
│  └───────────────┘  └───────────────┘  └─────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Design Decisions

1. **Structured Concurrency**: Every spawned task is attached to a handle; tasks cannot outlive their handle
2. **Multi-Protocol Support**: HTTP, HTTPS, QUIC, and STUN on separate ports
3. **Rate Limiting**: Governor-based rate limiting per client
4. **Key Caching**: LRU cache for public key verification
5. **Access Control**: Configurable node access restrictions
6. **TLS Flexibility**: Manual certificates or ACME auto-certificates

---

## Key APIs and Data Structures

### Relay Protocol Types

```rust
/// Maximum packet size (from DERP protocol)
pub const MAX_PACKET_SIZE: usize = 64 * 1024;

/// Server message types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    /// Key of client that connected
    ServerKey { key: PublicKey },

    /// Client is connected
    Connected { node_id: NodeId },

    /// Packet to be forwarded
    ReceivePacket { from: NodeId, data: Bytes },

    /// Ping request
    Ping { id: u32 },

    /// Health status
    Health { status: String },
}

/// Client message types
#[derive(Debug, Clone)]
pub enum ClientMessage {
    /// Client wants to receive packets
    WantContent { node_id: NodeId },

    /// Client no longer wants packets
    NoContent { node_id: NodeId },

    /// Send packet to node
    SendPacket {
        dst_key: NodeId,
        data: Bytes,
    },

    /// Forward packet through relay
    ForwardPacket {
        dst_key: NodeId,
        src_key: NodeId,
        data: Bytes,
    },

    /// Ping request
    Ping { id: u32 },

    /// Pong response
    Pong { id: u32 },

    /// Note preferred relay
    NotePreferred { preferred: bool },
}

/// Packet types for wire protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    /// Forwarded data packet
    Data = 0x01,

    /// Discovery/frame packet
    Frame = 0x02,
}
```

### Server Configuration

```rust
/// Full relay server configuration
pub struct ServerConfig<EC, EA = EC>
where
    EC: fmt::Debug,
    EA: fmt::Debug,
{
    /// Relay HTTP/HTTPS configuration
    pub relay: Option<RelayConfig<EC, EA>>,

    /// STUN server configuration
    pub stun: Option<StunConfig>,

    /// QUIC server configuration
    pub quic: Option<QuicConfig>,

    /// Metrics server address
    #[cfg(feature = "metrics")]
    pub metrics_addr: Option<SocketAddr>,
}

/// Relay-specific configuration
pub struct RelayConfig<EC, EA> {
    /// HTTP bind address (usually port 80)
    pub http_bind_addr: SocketAddr,

    /// TLS configuration for HTTPS
    pub tls: Option<TlsConfig<EC, EA>>,

    /// Rate limits for clients
    pub limits: Limits,

    /// Key cache capacity
    pub key_cache_capacity: Option<usize>,

    /// Access control configuration
    pub access: AccessConfig,
}

/// Access control for nodes
pub enum AccessConfig {
    /// Allow all nodes
    Everyone,

    /// Restricted access via callback
    Restricted(Box<dyn Fn(NodeId) -> Boxed<Access> + Send + Sync + 'static>),
}

/// TLS certificate configuration
pub enum CertConfig<EC, EA> {
    /// Manual certificate files
    Manual {
        cert_path: PathBuf,
        key_path: PathBuf,
    },

    /// ACME (Let's Encrypt) certificates
    Acme {
        config: AcmeConfig<EC, EA>,
    },
}
```

### Client Implementation

```rust
/// Relay client for connecting to relay servers
pub struct Client {
    /// Node ID (public key)
    node_id: NodeId,

    /// Connection to relay
    connection: WebSocketOrQuic,

    /// Packet receiver
    packet_rx: mpsc::Receiver<Packet>,
}

impl Client {
    /// Connect to relay server
    pub async fn connect(
        url: RelayUrl,
        secret_key: SecretKey,
        quic_config: Option<RelayQuicConfig>,
    ) -> Result<Self> {
        // Establish connection
        let conn = connect_to_relay(url, &secret_key).await?;

        // Receive server key
        let server_key = conn.receive_server_key().await?;

        // Authenticate
        conn.authenticate(&secret_key).await?;

        // Start packet receiver
        let (tx, rx) = mpsc::channel(64);
        conn.start_receiver(tx).await;

        Ok(Self {
            node_id: secret_key.public(),
            connection: conn,
            packet_rx: rx,
        })
    }

    /// Send packet through relay
    pub async fn send(&self, dst: NodeId, data: &[u8]) -> Result<()> {
        self.connection
            .send(ClientMessage::SendPacket {
                dst_key: dst,
                data: data.into(),
            })
            .await
    }

    /// Receive packet from relay
    pub async fn recv(&mut self) -> Option<Packet> {
        self.packet_rx.recv().await
    }

    /// Note this relay as preferred
    pub async fn note_preferred(&self, preferred: bool) -> Result<()> {
        self.connection
            .send(ClientMessage::NotePreferred { preferred })
            .await
    }
}
```

### Relay Map

```rust
/// Map of available relay servers
pub struct RelayMap {
    /// Map of relay nodes by ID
    nodes: BTreeMap<RelayId, RelayNode>,
}

/// Individual relay node configuration
pub struct RelayNode {
    /// Relay URL
    pub url: RelayUrl,

    /// STUN address (optional)
    pub stun_addr: Option<SocketAddr>,

    /// Priority for selection
    pub priority: u16,

    /// Whether to use for QUIC discovery
    pub quic: RelayQuicConfig,
}

impl RelayMap {
    /// Create new relay map from nodes
    pub fn new(nodes: BTreeMap<RelayId, RelayNode>) -> Self;

    /// Get relay by ID
    pub fn get(&self, id: &RelayId) -> Option<&RelayNode>;

    /// Get all relays
    pub fn iter(&self) -> impl Iterator<Item = (&RelayId, &RelayNode)>;

    /// Select best relay based on priority and latency
    pub fn select_best(&self) -> Option<&RelayNode>;
}
```

---

## Protocol Details

### HTTP Relay Protocol

The relay protocol runs over WebSocket connections to `/relay`:

```
Client                              Server
  |                                    |
  |-- HTTP Upgrade to WebSocket ------->|
  |<-- 101 Switching Protocols ---------|
  |                                    |
  |<-- ServerKey (32 bytes) -----------|
  |                                    |
  |-- ClientInfo (authenticated) ----->|
  |                                    |
  |<-- Connected ----------------------|
  |                                    |
  |=== Packet Exchange =================|
  |                                    |
  |-- SendPacket --------------------->|
  |                                    |
  |<-- ReceivePacket ------------------|
  |                                    |
```

### Wire Format

```
┌─────────────────────────────────────────────────────────┐
│                    Frame Header                         │
├─────────────────────────────────────────────────────────┤
│  Type (1 byte)         │ Frame type identifier         │
├─────────────────────────────────────────────────────────┤
│  Length (2 bytes)      │ Payload length (big-endian)   │
├─────────────────────────────────────────────────────────┤
│  Payload (variable)    │ Frame-specific data           │
└─────────────────────────────────────────────────────────┘

Frame Types:
- 0x01: ServerKey
- 0x02: ClientInfo
- 0x03: Connected
- 0x04: ReceivePacket
- 0x05: Ping
- 0x06: Pong
- 0x07: NotePreferred
- 0x08: Health
```

### STUN Protocol

STUN (Session Traversal Utilities for NAT) runs on UDP port 3478:

```rust
/// STUN request handler
async fn handle_stun(socket: &UdpSocket, addr: SocketAddr, data: &[u8]) {
    let request = match stun_rs::Message::from_bytes(data) {
        Ok(req) if req.method() == Method::Binding => req,
        _ => return,
    };

    // Build response
    let mut response = request.success_response(&addr);
    response.add_software("iroh-relay");

    // Send response
    socket.send_to(response.get_bytes().as_slice(), addr).await.ok();
}
```

### QUIC Endpoint

Optional QUIC endpoint for enhanced discovery:

```rust
/// QUIC server configuration
pub struct QuicConfig {
    /// Bind address (usually port 7842)
    pub bind_addr: SocketAddr,

    /// TLS server config (must support TLS 1.3)
    pub server_config: rustls::ServerConfig,
}

/// QUIC server handle
pub struct ServerHandle {
    /// QUIC endpoint
    endpoint: Endpoint,

    /// Shutdown token
    cancel_token: CancellationToken,
}

impl ServerHandle {
    /// Accept incoming QUIC connections
    pub async fn run(self) {
        while let Some(conn) = self.endpoint.accept().await {
            tokio::spawn(handle_quic_connection(conn));
        }
    }
}
```

---

## Integration with Main Iroh Endpoint

### Endpoint Configuration

```rust
use iroh::{Endpoint, RelayMode, RelayMap};

// Create endpoint with relay
let relay_map = RelayMap::new(relays);

let endpoint = Endpoint::builder()
    .relay_mode(RelayMode::Custom(relay_map))
    .bind()
    .await?;

// Connect to peer via relay
let peer_addr = NodeAddr {
    node_id: peer_id,
    relay_url: Some(relay_url.clone()),
    direct_addresses: vec![],
};

let conn = endpoint.connect(peer_addr, b"my-alpn").await?;
```

### Network Report

The relay is used for network connectivity reporting:

```rust
/// Generate network report using relay
async fn net_report(endpoint: &Endpoint) -> NetReport {
    let mut report = NetReport::default();

    // Probe relay via HTTP
    let relay_latency = probe_relay_http(&relay_url).await;
    report.relay_latency = relay_latency;

    // Probe via STUN
    if let Some(stun_addr) = relay.stun_addr {
        let has_ipv4 = probe_stun_ipv4(stun_addr).await;
        let has_ipv6 = probe_stun_ipv6(stun_addr).await;
        report.has_ipv4 = has_ipv4;
        report.has_ipv6 = has_ipv6;
    }

    // Check UPnP/NAT-PMP
    report.upnp = check_upnp().await;

    report
}
```

### Relay Selection

```rust
/// Select optimal relay based on latency and priority
async fn select_relay(
    relays: &RelayMap,
) -> Option<(RelayId, RelayUrl)> {
    let mut best: Option<(RelayId, RelayUrl, Duration)> = None;

    for (id, node) in relays.iter() {
        // Measure latency
        let latency = measure_latency(&node.url).await;

        // Calculate score (lower is better)
        let score = latency + Duration::from_secs(node.priority as u64);

        // Update best if this is better
        if best.is_none() || score < best.as_ref().unwrap().2 {
            best = Some((id.clone(), node.url.clone(), score));
        }
    }

    best.map(|(id, url, _)| (id, url))
}
```

---

## Production Usage Patterns

### Running a Relay Server

```rust
use iroh_relay::{Server, ServerConfig, RelayConfig, StunConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Configure server
    let config = ServerConfig {
        relay: Some(RelayConfig {
            http_bind_addr: "0.0.0.0:80".parse()?,
            tls: Some(TlsConfig {
                https_bind_addr: "0.0.0.0:443".parse()?,
                cert: CertConfig::Acme { /* ... */ },
            }),
            limits: Limits::default(),
            key_cache_capacity: Some(1000),
            access: AccessConfig::Everyone,
        }),
        stun: Some(StunConfig {
            bind_addr: "0.0.0.0:3478".parse()?,
        }),
        quic: Some(QuicConfig {
            bind_addr: "0.0.0.0:7842".parse()?,
            server_config: tls_config,
        }),
        metrics_addr: Some("0.0.0.0:9090".parse()?),
    };

    // Create and run server
    let server = Server::new(config).await?;
    server.run().await?;

    Ok(())
}
```

### Configuration File

```toml
# Relay server configuration

[relay]
http_bind_addr = "0.0.0.0:80"
key_cache_capacity = 1000

[relay.tls]
https_bind_addr = "0.0.0.0:443"
quic_bind_addr = "0.0.0.0:7842"

[relay.tls.cert]
mode = "Acme"
acme_domain = "relay.example.com"
acme_email = "admin@example.com"

[relay.limits]
rate_limit_requests = 100
rate_limit_burst = 1000

[stun]
bind_addr = "0.0.0.0:3478"

[quic]
bind_addr = "0.0.0.0:7842"

[metrics]
bind_addr = "0.0.0.0:9090"
```

### Client Connection Pool

```rust
/// Maintain connections to multiple relays
struct RelayPool {
    relays: RelayMap,
    connections: HashMap<RelayId, Client>,
}

impl RelayPool {
    /// Get or create connection to relay
    async fn get_client(&mut self, id: &RelayId) -> Result<&mut Client> {
        if !self.connections.contains_key(id) {
            let relay = self.relays.get(id).ok_or("Unknown relay")?;
            let client = Client::connect(
                relay.url.clone(),
                self.secret_key.clone(),
                None,
            ).await?;
            self.connections.insert(id.clone(), client);
        }
        Ok(self.connections.get_mut(id).unwrap())
    }

    /// Send to peer via best available relay
    async fn send_to_peer(
        &mut self,
        peer: NodeId,
        data: &[u8],
    ) -> Result<()> {
        // Find relay that peer is connected to
        let relay_id = self.discovery.get_peer_relay(peer).await?;

        // Send via that relay
        let client = self.get_client(&relay_id).await?;
        client.send(peer, data).await?;

        Ok(())
    }
}
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| hyper | 1.x | HTTP server/client |
| tokio-websockets | 0.11.3 | WebSocket support |
| quinn | 0.13.0 | QUIC implementation |
| rustls | 0.23 | TLS implementation |
| stun-rs | 0.1.5 | STUN protocol |
| governor | 0.7.0 | Rate limiting |
| lru | 0.12.3 | LRU cache |
| dashmap | 6.1.0 | Concurrent HashMap |
| tokio-rustls-acme | 0.6 | ACME certificates |

### Notable Rust Patterns

1. **Structured Concurrency**: AbortOnDropHandle for task lifecycle management
2. **Type-State Pattern**: Generic parameters for TLS configuration
3. **Feature Flags**: Optional server/stun/metrics components
4. **WASM Support**: Conditional compilation for browser targets

### Performance Considerations

1. **Key Caching**: LRU cache reduces public key verification overhead
2. **Connection Pooling**: Reuse client connections where possible
3. **Rate Limiting**: Governor token bucket for API protection
4. **Zero-Copy**: Bytes type for efficient buffer management

### Potential Enhancements

1. **Multiple Relay Hops**: Support for multi-relay forwarding
2. **Load Balancing**: Dynamic load distribution across relays
3. **Geographic Routing**: Location-aware relay selection
4. **Metrics Enhancement**: Additional observability endpoints

---

## Summary

`iroh-relay` provides:

- **Relay Protocol**: DERP-based protocol for packet forwarding
- **Multi-Protocol Server**: HTTP, HTTPS, QUIC, and STUN support
- **Hole Punching**: NAT traversal assistance
- **Fallback Path**: Reliable communication when direct fails
- **Flexible Deployment**: Manual or ACME TLS, access control

The relay is essential for iroh's ability to establish connections in challenging network environments.
