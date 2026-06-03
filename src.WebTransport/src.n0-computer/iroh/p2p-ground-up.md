# Building Production-Grade P2P from Scratch

## Introduction

This document explains how to build a production-grade peer-to-peer (P2P) networking system from the ground up, based on the architectural patterns found in **iroh**. We'll cover each layer of the system, from basic UDP sockets to full NAT traversal.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/`

---

## Table of Contents

1. [Layer 1: UDP Foundation](#layer-1-udp-foundation)
2. [Layer 2: Identity & Cryptography](#layer-2-identity--cryptography)
3. [Layer 3: Secure Handshake](#layer-3-secure-handshake)
4. [Layer 4: Relay Fallback](#layer-4-relay-fallback)
5. [Layer 5: NAT Traversal](#layer-5-nat-traversal)
6. [Layer 6: Address Discovery](#layer-6-address-discovery)
7. [Layer 7: Connection Management](#layer-7-connection-management)
8. [Production Considerations](#production-considerations)

---

## Layer 1: UDP Foundation

### Why UDP?

UDP is the foundation of modern P2P systems because:

1. **No Connection State**: Firewalls and NATs handle UDP more permissively
2. **Lower Latency**: No TCP handshake overhead
3. **QUIC Compatibility**: QUIC runs over UDP
4. **NAT Traversal**: UDP hole-punching is well-understood

### Basic UDP Socket Setup

```rust
use std::net::{UdpSocket, SocketAddr};

// Bind to all interfaces
let socket = UdpSocket::bind("0.0.0.0:0")?;  // Port 0 = OS assigns port
let local_addr = socket.local_addr()?;

println!("Bound to: {}", local_addr);
// Example output: Bound to: 0.0.0.0:54321
```

### Socket Options for P2P

```rust
use std::net::UdpSocket;
use socket2::{Socket, Domain, Type, Protocol};

fn create_p2p_socket() -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

    // Allow address reuse (important for quick restarts)
    socket.set_reuse_address(true)?;

    // On some platforms, allow binding to same address
    #[cfg(unix)]
    socket.set_reuse_port(true)?;

    // Set buffer sizes (tune based on your needs)
    socket.set_send_buffer_size(2 * 1024 * 1024)?;  // 2MB
    socket.set_recv_buffer_size(2 * 1024 * 1024)?;  // 2MB

    // Disable ICMP errors (prevents connection refused issues)
    socket.set_ip_recv_error(false)?;

    Ok(socket.into())
}
```

### Dual-Stack (IPv4 + IPv6)

```rust
// Bind both IPv4 and IPv6
let ipv4_socket = UdpSocket::bind("0.0.0.0:0")?;
let ipv6_socket = UdpSocket::bind("[::]:0")?;

// Or use dual-stack on IPv6 socket
let socket = UdpSocket::bind("[::]:0")?;
socket.set_only_v6(false)?;  // Now handles both v4 and v6
```

---

## Layer 2: Identity & Cryptography

### Why Ed25519 (Not RSA)?

| Property | Ed25519 | RSA-2048 |
|----------|---------|----------|
| Key Size | 32 bytes | 256 bytes |
| Signature | 64 bytes | 256 bytes |
| Sign Speed | ~50,000/s | ~1,000/s |
| Verify Speed | ~15,000/s | ~5,000/s |
| Security | 128-bit | 112-bit |

### Key Generation

```rust
use ed25519_dalek::{SigningKey, VerifyingKey, Signature};
use rand::rngs::OsRng;

// Generate a new keypair
let mut csprng = OsRng;
let secret_key: SigningKey = SigningKey::generate(&mut csprng);
let public_key: VerifyingKey = secret_key.verifying_key();

// Serialize
let secret_bytes: [u8; 32] = secret_key.to_bytes();
let public_bytes: [u8; 32] = public_key.to_bytes();

// Deserialize
let secret_key = SigningKey::from_bytes(&secret_bytes);
let public_key = VerifyingKey::from_bytes(&public_bytes)?;
```

### Node ID from Public Key

```rust
/// The node's ID in the P2P network IS its public key
pub type NodeId = VerifyingKey;

pub struct Node {
    secret_key: SigningKey,
    node_id: NodeId,  // Derived from secret_key
}

impl Node {
    pub fn new(secret_key: SigningKey) -> Self {
        let node_id = secret_key.verifying_key();
        Self { secret_key, node_id }
    }

    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }
}
```

### Signing and Verification

```rust
// Signing a message
let message = b"Hello, P2P!";
let signature: Signature = secret_key.sign(message);

// Verifying a signature
let is_valid = public_key.verify(message, &signature).is_ok();

// Domain separation (critical for security!)
fn domain_sep_sign(secret: &SigningKey, domain: &str, msg: &[u8]) -> Signature {
    use blake3::derive_key;
    let key = derive_key(domain, msg);
    secret.sign(&key)
}
```

---

## Layer 3: Secure Handshake

### TLS with Raw Public Keys

Instead of X.509 certificates, use RFC 7250 (Raw Public Keys):

```rust
use rustls::{ClientConfig, ServerConfig, ServerConnection, ClientConnection};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::crypto::ring;

fn create_p2p_tls_config(secret_key: &SigningKey) -> ClientConfig {
    // Convert Ed25519 key to format rustls understands
    let key_der = convert_ed25519_to_pkcs8(secret_key);

    let mut config = ClientConfig::builder_with_provider(
        Arc::new(ring::default_provider())
    )
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .dangerous()  // We're using custom verification
    .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
    .with_client_auth_cert(vec![], key_der)
    .unwrap();

    // Enable 0-RTT for faster reconnections
    config.enable_early_data = true;

    config
}

struct NoCertificateVerification;

impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer,
        _intermediates: &[CertificateDer],
        _server_name: &rustls::pki_types::ServerName,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        // We verify the peer's key directly in the handshake
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        // Verify signature matches expected public key
        // This is where you'd check against the known peer ID
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
}
```

### Challenge-Response Authentication

For custom protocols without TLS:

```rust
use ed25519_dalek::{SigningKey, VerifyingKey, Signature};
use rand::RngCore;

// Server sends challenge
fn generate_challenge() -> [u8; 32] {
    let mut challenge = [0u8; 32];
    rand::rng().fill_bytes(&mut challenge);
    challenge
}

// Client signs challenge
fn sign_challenge(secret: &SigningKey, challenge: &[u8]) -> Signature {
    use blake3::derive_key;

    // Domain separation prevents cross-protocol attacks
    const DOMAIN: &str = "my-p2p-auth-v1";
    let msg = derive_key(DOMAIN, challenge);

    secret.sign(&msg)
}

// Server verifies
fn verify_challenge(
    public: &VerifyingKey,
    challenge: &[u8],
    signature: &Signature,
) -> bool {
    use blake3::derive_key;

    const DOMAIN: &str = "my-p2p-auth-v1";
    let msg = derive_key(DOMAIN, challenge);

    public.verify(&msg, signature).is_ok()
}
```

### Handshake Protocol

```
Client                              Server
  |                                    |
  |--- ClientHello (my_pubkey) ------>|
  |                                    |
  |<-- ServerHello (srv_pubkey,       |
  |        challenge) -----------------|
  |                                    |
  |--- AuthResponse (signature) ------>|
  |                                    |
  |<-- AuthConfirm (ok/fail) ---------|
  |                                    |
  |=== Encrypted Communication ========|
```

---

## Layer 4: Relay Fallback

### Why Relays?

Not all P2P connections can be direct:
- **Symmetric NATs**: Different external port per destination
- **Enterprise firewalls**: Block all incoming connections
- **Carrier-grade NAT**: Multiple NAT layers

### Relay Protocol Design

```rust
use bytes::{Bytes, BytesMut};

// Messages from client to relay
enum ClientMessage {
    /// Authenticate with the relay
    Auth {
        client_id: NodeId,
        signature: Signature,
    },
    /// Send data to another node
    Send {
        recipient_id: NodeId,
        data: Bytes,
    },
    /// Keep connection alive
    Ping(u64),
}

// Messages from relay to client
enum ServerMessage {
    /// Authentication result
    AuthResult { success: bool, reason: Option<String> },
    /// Received data from another node
    Data {
        sender_id: NodeId,
        data: Bytes,
    },
    /// Health status
    Health { problem: Option<String> },
    /// Pong response
    Pong(u64),
}
```

### Relay Server Implementation

```rust
use std::collections::HashMap;
use tokio::sync::mpsc;

struct RelayServer {
    /// Connected clients: NodeId -> Sender
    clients: HashMap<NodeId, mpsc::Sender<ServerMessage>>,
}

impl RelayServer {
    async fn handle_client(
        &mut self,
        client_id: NodeId,
        mut rx: mpsc::Receiver<ClientMessage>,
        tx: mpsc::Sender<ServerMessage>,
    ) {
        // Register client
        self.clients.insert(client_id, tx.clone());

        while let Some(msg) = rx.recv().await {
            match msg {
                ClientMessage::Send { recipient_id, data } => {
                    // Forward to recipient
                    if let Some(recipient_tx) = self.clients.get(&recipient_id) {
                        let _ = recipient_tx.send(ServerMessage::Data {
                            sender_id: client_id,
                            data,
                        }).await;
                    }
                }
                ClientMessage::Ping(nonce) => {
                    let _ = tx.send(ServerMessage::Pong(nonce)).await;
                }
                _ => {}
            }
        }

        // Client disconnected
        self.clients.remove(&client_id);
    }
}
```

### Relay Connection from Client

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

async fn connect_to_relay(
    relay_url: &str,
    node_id: NodeId,
    secret_key: &SigningKey,
) -> Result<RelayConnection, Error> {
    // Connect via WebSocket
    let url = Url::parse(relay_url)?;
    let (ws_stream, _) = connect_async(url).await?;

    // Send authentication
    let challenge = receive_challenge(&ws_stream).await?;
    let signature = sign_challenge(secret_key, &challenge);

    send_auth(&ws_stream, node_id, signature).await?;

    // Wait for confirmation
    let confirmed = wait_for_auth_confirm(&ws_stream).await?;

    Ok(RelayConnection { ws_stream, confirmed })
}
```

---

## Layer 5: NAT Traversal

### NAT Types

```
┌─────────────────────────────────────────────────────────────┐
│                    NAT Types                                 │
├──────────────┬──────────────────────────────────────────────┤
│ Full Cone    │ Any external host can send packets           │
│              │ to internal host via mapped address          │
├──────────────┼──────────────────────────────────────────────┤
│ Restricted   │ Only hosts that internal host sent to       │
│ Cone         │ can send packets back                        │
├──────────────┼──────────────────────────────────────────────┤
│ Port         │ Same as Restricted, but also requires       │
│ Restricted   │ same external port                           │
├──────────────┼──────────────────────────────────────────────┤
│ Symmetric    │ Different external port for each destination │
│              │ (HARDEST for P2P)                            │
└──────────────┴──────────────────────────────────────────────┘
```

### STUN Protocol

```rust
use std::net::{SocketAddr, UdpSocket};

// STUN request (minimal)
fn create_stun_request() -> Vec<u8> {
    // STUN message type: Binding Request (0x0001)
    // Magic cookie: 0x2112A442
    // Transaction ID: random 96 bits
    let mut msg = vec![0u8; 20];
    msg[0..2].copy_from_slice(&0x0001u16.to_be_bytes());  // Binding Request
    msg[2..4].copy_from_slice(&0x0000u16.to_be_bytes());  // Message length
    msg[4..8].copy_from_slice(&0x2112A442u32.to_be_bytes()); // Magic cookie
    rand::rng().fill(&mut msg[8..20]);  // Random transaction ID
    msg
}

// Parse STUN response
fn parse_stun_response(data: &[u8]) -> Option<SocketAddr> {
    // Check for Binding Response (0x0101)
    if data[0..2] != 0x0101u16.to_be_bytes() {
        return None;
    }

    // Parse attributes (XOR-MAPPED-ADDRESS is at type 0x0020)
    let mut offset = 20;  // Skip header
    while offset < data.len() {
        let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;

        if attr_type == 0x0020 {
            // XOR-MAPPED-ADDRESS
            let family = data[offset];
            let port = u16::from_be_bytes([data[offset + 2], data[offset + 3]])
                ^ (0x2112 >> 16) as u16;  // XOR with magic cookie

            let mut addr_bytes = [0u8; 4];
            for (i, byte) in addr_bytes.iter_mut().enumerate() {
                *byte = data[offset + 4 + i] ^ (0x2112A442 >> (8 * (3 - i))) as u8;
            }

            return Some(SocketAddr::new(addr_bytes.into(), port));
        }
        offset += attr_len;
    }
    None
}

// Get public address via STUN
fn get_public_addr(stun_server: &str) -> Option<SocketAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(2))).ok()?;

    let request = create_stun_request();
    socket.send_to(&request, stun_server).ok()?;

    let mut buf = vec![0u8; 512];
    let (len, _) = socket.recv_from(&mut buf).ok()?;

    parse_stun_response(&buf[..len])
}
```

### UDP Hole Punching

```rust
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

/// Coordinate hole punching between two peers
async fn hole_punch(
    local_socket: &UdpSocket,
    peer_id: NodeId,
    peer_addrs: Vec<SocketAddr>,
    coordination_server: &str,
) -> Result<SocketAddr, HolePunchError> {
    // 1. Both peers connect to coordination server
    let (my_external_addr, peer_external_addr) =
        coordinate_with_server(peer_id, coordination_server).await?;

    // 2. Start listening for incoming packets
    local_socket.set_read_timeout(Some(Duration::from_millis(100)))?;

    // 3. Send packets to all known peer addresses
    let punch_data = b"PUNCH";  // Any data works
    for addr in &peer_addrs {
        let _ = local_socket.send_to(punch_data, addr);
    }

    // 4. Also send to discovered external address
    if let Some(peer_ext) = peer_external_addr {
        let _ = local_socket.send_to(punch_data, peer_ext);
    }

    // 5. Wait for response (creates NAT mapping)
    let mut buf = vec![0u8; 1500];
    for _ in 0..50 {  // Try for 5 seconds
        if let Ok((len, addr)) = local_socket.recv_from(&mut buf) {
            if &buf[..len] == punch_data {
                // Hole punch successful!
                return Ok(addr);
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(HolePunchError::Timeout)
}

/// Coordination via relay/server
async fn coordinate_with_server(
    peer_id: NodeId,
    server: &str,
) -> Result<(SocketAddr, Option<SocketAddr>), CoordinationError> {
    // Connect to server and exchange external addresses
    // Server tells us peer's external address
    // Server tells peer our external address
    // Both sides start sending packets simultaneously

    unimplemented!()
}
```

### ICE (Interactive Connectivity Establishment)

```rust
/// ICE candidate (possible connection path)
#[derive(Debug, Clone)]
struct IceCandidate {
    foundation: String,  // Unique identifier
    component_id: u16,   // Usually 1 for RTP
    protocol: String,    // "udp" or "tcp"
    priority: u32,       // Higher = better
    addr: SocketAddr,
    typ: CandidateType,
    related_addr: Option<SocketAddr>,
}

#[derive(Debug, Clone)]
enum CandidateType {
    Host,      // Direct interface address
    Srflx,     // Server-reflexive (STUN-discovered)
    Prflx,     // Peer-reflexive (discovered during connectivity checks)
    Relay,     // Relay address
}

/// ICE agent manages connectivity checks
struct IceAgent {
    local_candidates: Vec<IceCandidate>,
    remote_candidates: Vec<IceCandidate>,
    nominated_pair: Option<(IceCandidate, IceCandidate)>,
}

impl IceAgent {
    /// Gather local candidates
    async fn gather_candidates(&mut self, stun_servers: &[&str]) {
        // Host candidates
        for iface in get_network_interfaces() {
            for addr in iface.addresses {
                self.local_candidates.push(IceCandidate {
                    foundation: compute_foundation(&addr),
                    component_id: 1,
                    protocol: "udp".to_string(),
                    priority: compute_priority(CandidateType::Host, &addr),
                    addr,
                    typ: CandidateType::Host,
                    related_addr: None,
                });
            }
        }

        // Server-reflexive candidates (via STUN)
        for stun in stun_servers {
            if let Some(srflx) = get_public_addr(stun) {
                self.local_candidates.push(IceCandidate {
                    foundation: compute_foundation(&srflx),
                    component_id: 1,
                    protocol: "udp".to_string(),
                    priority: compute_priority(CandidateType::Srflx, &srflx),
                    addr: srflx,
                    typ: CandidateType::Srflx,
                    related_addr: None,
                });
            }
        }
    }

    /// Perform connectivity checks
    async fn connectivity_checks(&mut self, socket: &UdpSocket) {
        // Sort candidate pairs by priority
        let mut pairs = self.create_candidate_pairs();
        pairs.sort_by(|a, b| b.priority().cmp(&a.priority()));

        // Check each pair
        for (local, remote) in pairs {
            if self.send_binding_request(socket, &local, &remote).await {
                self.nominated_pair = Some((local, remote));
                return;  // Found working path
            }
        }
    }
}
```

---

## Layer 6: Address Discovery

### Node Address Structure

```rust
use std::net::SocketAddr;

/// Complete addressing info for a node
#[derive(Debug, Clone)]
pub struct NodeAddress {
    /// The node's cryptographic ID
    pub node_id: NodeId,

    /// How to reach the node
    pub transport_addrs: Vec<TransportAddr>,
}

#[derive(Debug, Clone)]
pub enum TransportAddr {
    /// Via relay server
    Relay {
        url: String,
        quic_port: u16,
    },
    /// Direct IP address
    Direct(SocketAddr),
    /// Custom transport
    Custom {
        transport_id: u64,
        data: Vec<u8>,
    },
}
```

### Publishing Address

```rust
use pkarr::{PkarrClient, SignedPacket, dns::Packet};
use ed25519_dalek::SigningKey;

/// Publish node address via Pkarr
async fn publish_address(
    secret_key: &SigningKey,
    relay_url: &str,
    direct_addrs: Vec<SocketAddr>,
) -> Result<(), PublishError> {
    let public_key = secret_key.verifying_key();

    // Create DNS TXT record with encoded address info
    let mut packet = Packet::new_reply(0);

    // Encode addresses as TXT record
    let addr_data = encode_addresses(&direct_addrs, relay_url);
    packet.add_answer(&dns::ResourceRecord {
        name: format!("{}.addr.", hex::encode(public_key)),
        ttl: 300,  // 5 minutes
        data: dns::rdata::RData::TXT(dns::rdata::TXT(addr_data)),
        ..Default::default()
    });

    // Sign the packet
    let signed = SignedPacket::from_packet(secret_key, packet)?;

    // Publish to relay
    let client = PkarrClient::new("https://dns.iroh.link/pkarr")?;
    client.publish(&signed).await?;

    Ok(())
}
```

### Resolving Address

```rust
use pkarr::{PkarrClient, PublicKey};

/// Resolve node address via Pkarr
async fn resolve_address(
    node_id: &NodeId,
    resolver_url: &str,
) -> Result<NodeAddress, ResolveError> {
    let client = PkarrClient::new(resolver_url)?;

    // Lookup signed packet
    let public_key = PublicKey::from_bytes(node_id.as_bytes())?;
    let packet = client.resolve(&public_key).await?;

    // Parse DNS response
    let answer = packet.answer();
    let mut addrs = Vec::new();

    for record in answer.records {
        if let dns::rdata::RData::TXT(txt) = record.data {
            let parsed = decode_addresses(&txt)?;
            addrs.extend(parsed.transport_addrs);
        }
    }

    Ok(NodeAddress {
        node_id: *node_id,
        transport_addrs: addrs,
    })
}
```

### mDNS for Local Discovery

```rust
use mdns_sd::{ServiceDaemon, ServiceInfo};

/// Publish node on local network via mDNS
fn publish_mdns(node_id: NodeId, port: u16) -> Result<ServiceDaemon, MdnsError> {
    let daemon = ServiceDaemon::new()?;

    let service_type = "_iroh-p2p._udp.local.";
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

    Ok(daemon)
}

/// Discover nodes on local network
fn discover_mdns() -> impl Iterator<Item = NodeAddress> {
    let daemon = ServiceDaemon::new().unwrap();
    let receiver = daemon.browse("_iroh-p2p._udp.local.").unwrap();

    std::thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            if let ServiceEvent::ServiceResolved(info) = event {
                let node_id_hex = info.get_property("node_id").unwrap();
                let node_id = hex::decode(node_id_hex).unwrap();
                // ... construct NodeAddress
            }
        }
    });

    // Return iterator
    unimplemented!()
}
```

---

## Layer 7: Connection Management

### Connection State Machine

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    New,
    Connecting,
    Handshaking,
    Connected,
    Failed,
    Closed,
}

struct Connection {
    remote_node_id: NodeId,
    state: Arc<RwLock<ConnectionState>>,
    paths: Vec<ConnectionPath>,
    active_path: Option<usize>,
}

struct ConnectionPath {
    addr: SocketAddr,
    path_type: PathType,
    rtt: Duration,
    last_activity: Instant,
    stats: PathStats,
}

enum PathType {
    Direct,
    Relay { url: String },
    Reflexive,
}

impl Connection {
    /// Monitor connection health and switch paths if needed
    async fn monitor_health(&self) {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            // Check active path
            if let Some(idx) = self.active_path {
                let path = &self.paths[idx];

                if path.is_healthy() {
                    continue;
                }

                // Try to switch to backup path
                for (i, backup) in self.paths.iter().enumerate() {
                    if i != idx && backup.is_healthy() {
                        self.active_path = Some(i);
                        break;
                    }
                }
            }
        }
    }
}
```

### Multipath Support

```rust
/// Send data over multiple paths simultaneously
struct MultipathSender {
    primary: UdpSocket,
    relay: Option<RelayConnection>,
    paths: Vec<PathInfo>,
}

impl MultipathSender {
    async fn send(&self, data: &[u8], reliability: Reliability) {
        match reliability {
            Reliability::Ordered => {
                // Use QUIC stream
                self.send_ordered(data).await;
            }
            Reliability::Unordered => {
                // Send over all available paths
                // First response wins

                let mut futures = Vec::new();

                // Direct path
                for path in &self.paths {
                    futures.push(self.send_direct(path.addr, data));
                }

                // Relay path
                if let Some(relay) = &self.relay {
                    futures.push(self.send_relay(relay, data));
                }

                // Wait for first success
                futures::future::select_all(futures).await;
            }
        }
    }
}

enum Reliability {
    Ordered,    // QUIC streams
    Unordered,  // Datagrams
}
```

---

## Production Considerations

### Metrics and Observability

```rust
use prometheus::{Registry, Counter, Histogram};

struct P2PMetrics {
    connections_total: Counter,
    bytes_sent: Counter,
    bytes_received: Counter,
    connection_latency: Histogram,
    nat_type: Counter,  // Track NAT distribution
}

impl P2PMetrics {
    fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            connections_total: Counter::new("p2p_connections_total", "")?,
            bytes_sent: Counter::new("p2p_bytes_sent_total", "")?,
            bytes_received: Counter::new("p2p_bytes_received_total", "")?,
            connection_latency: Histogram::new(
                HistogramOpts::new("p2p_connection_latency_seconds", "")
                    .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
            )?,
            nat_type: Counter::new("p2p_nat_types_total", "")?,
        })
    }
}
```

### Connection Limits

```rust
use std::collections::HashMap;
use tokio::sync::Semaphore;

struct ConnectionManager {
    /// Limit total connections
    connection_semaphore: Arc<Semaphore>,
    /// Rate limit per peer
    peer_rates: Arc<RwLock<HashMap<NodeId, RateLimiter>>>,
    /// Connection timeout
    idle_timeout: Duration,
}

impl ConnectionManager {
    fn new(max_connections: usize) -> Self {
        Self {
            connection_semaphore: Arc::new(Semaphore::new(max_connections)),
            peer_rates: Arc::new(RwLock::new(HashMap::new())),
            idle_timeout: Duration::from_secs(300),  // 5 minutes
        }
    }

    async fn accept_connection(&self) -> Result<Permit, Error> {
        // Limit concurrent connections
        let permit = self.connection_semaphore.acquire().await
            .map_err(|_| Error::Shutdown)?;

        Ok(permit)
    }
}
```

### Retry and Backoff

```rust
use backon::{ExponentialBackoff, Retryable};

async fn connect_with_retry(
    node_id: NodeId,
    addrs: &[SocketAddr],
) -> Result<Connection, Error> {
    let backoff = ExponentialBackoff::default()
        .with_max_times(5)
        .with_max_delay(Duration::from_secs(30));

    (|| async {
        connect_attempt(node_id, addrs).await
    })
    .retry(backoff)
    .await
}
```

---

## Reference Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Application Layer                          │
├─────────────────────────────────────────────────────────────────┤
│                    Connection Manager                           │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐   │
│  │ Multipath    │  │ Health       │  │ Rate Limiting       │   │
│  │ Manager      │  │ Monitor      │  │ & Backpressure      │   │
│  └──────────────┘  └──────────────┘  └─────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                      Transport Layer                            │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐   │
│  │ QUIC         │  │ Relay        │  │ Custom Transports   │   │
│  │ (Direct UDP) │  │ Protocol     │  │ (WebSocket, etc.)   │   │
│  └──────────────┘  └──────────────┘  └─────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                    Security Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐   │
│  │ TLS 1.3      │  │ Ed25519      │  │ Session Management  │   │
│  │ (RFC 7250)   │  │ Signatures   │  │ (0-RTT, tickets)    │   │
│  └──────────────┘  └──────────────┘  └─────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                   Discovery Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐   │
│  │ Pkarr/DNS    │  │ mDNS         │  │ DHT (Mainline)      │   │
│  │ Resolution   │  │ (Local)      │  │                     │   │
│  └──────────────┘  └──────────────┘  └─────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                    Network Layer                                │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐   │
│  │ STUN/TURN    │  │ ICE          │  │ Network Monitoring  │   │
│  │ Hole Punch   │  │ Candidate    │  │ (IPv4/IPv6 status)  │   │
│  │              │  │ Selection    │  │                     │   │
│  └──────────────┘  └──────────────┘  └─────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                     Socket Layer                                │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              UDP Sockets (IPv4 + IPv6)                   │   │
│  │         Configured with optimal socket options           │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## See Also

- [exploration.md](./exploration.md) - Main iroh exploration
- [cryptography-keys.md](./cryptography-keys.md) - Cryptographic details
- [nat-traversal.md](./nat-traversal.md) - NAT traversal deep dive
