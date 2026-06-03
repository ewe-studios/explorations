# Iroh: Comprehensive Exploration

## Overview

**Iroh** is a production-grade peer-to-peer (P2P) networking library written in Rust that enables direct, encrypted connections between devices. It automatically handles connection establishment through relay servers when direct connections aren't immediately possible, then transitions to direct P2P communication.

**Source explored:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/`

### Key Characteristics

- **Protocol**: Built on QUIC (via `noq`/`quinn`) with custom extensions
- **Encryption**: TLS 1.3 with Raw Public Keys (RFC 7250) using Ed25519
- **NAT Traversal**: Automatic hole-punching with relay fallback
- **Addressing**: Public key-based addressing with optional relay and direct addresses
- **Platform Support**: Full support for desktop/server, browser support via WebSockets

---

## Architecture Overview

### Core Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        Iroh Endpoint                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐ │
│  │  SecretKey/     │  │  Transport      │  │  Address       │ │
│  │  PublicKey      │  │  Manager        │  │  Lookup        │ │
│  │  (Ed25519)      │  │  (QUIC/Relay)   │  │  (Pkarr/DNS)   │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬───────┘ │
│           │                    │                     │         │
│  ┌────────▼────────────────────▼─────────────────────▼───────┐ │
│  │                   Socket Layer                             │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │ │
│  │  │ Direct UDP   │  │ Relay        │  │ Custom          │  │ │
│  │  │ (IPv4/IPv6)  │  │ (QUIC/HTTP)  │  │ Transports      │  │ │
│  │  └──────────────┘  └──────────────┘  └─────────────────┘  │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Module Structure

| Module | Purpose |
|--------|---------|
| `iroh/` | Main endpoint API and connection management |
| `iroh-base/` | Core types: `PublicKey`, `SecretKey`, `EndpointId`, `RelayUrl` |
| `iroh-relay/` | Relay server/client protocol (DERP-based) |
| `iroh-dns-server/` | DNS/Pkarr resolution server |

---

## Cryptographic Foundation

### Ed25519 Keys (Not RSA)

Iroh uses **Ed25519** (EdDSA over Curve25519), not RSA, for all cryptographic operations:

```rust
// From iroh-base/src/key.rs
pub struct PublicKey(CompressedEdwardsY);  // 32 bytes
pub struct SecretKey(SigningKey);          // 32 bytes

// Key generation
let secret_key = SecretKey::generate(&mut rand::rng());
let public_key = secret_key.public();  // Derived from secret key
```

### Key Properties

1. **Compact**: 32 bytes for both public and private keys (vs 2048+ bits for RSA)
2. **Fast**: Signing and verification are significantly faster than RSA
3. **Secure**: 128-bit security level, equivalent to 3072-bit RSA
4. **Deterministic**: Signatures are deterministic (no RNG needed for signing)

### TLS with Raw Public Keys (RFC 7250)

Iroh implements TLS 1.3 with Raw Public Key extension:

```rust
// From iroh/src/tls.rs
pub(crate) struct TlsConfig {
    pub(crate) secret_key: SecretKey,
    cert_resolver: Arc<ResolveRawPublicKeyCert>,
    server_verifier: Arc<verifier::ServerCertificateVerifier>,
    client_verifier: Arc<verifier::ClientCertificateVerifier>,
    session_store: Arc<dyn rustls::client::ClientSessionStore>,
}
```

This allows:
- No X.509 certificates needed
- Direct use of Ed25519 keys for TLS authentication
- Encrypted connections between any two endpoints knowing each other's public keys

---

## Iroh IDs (EndpointId)

### What is an EndpointId?

An `EndpointId` is the fundamental identifier for any node in the iroh network:

```rust
// From iroh-base/src/key.rs
pub type EndpointId = PublicKey;  // Alias for PublicKey

/// Each endpoint in iroh has a unique identifier created as a cryptographic key.
/// This can be used to globally identify an endpoint. Since it is also a
/// cryptographic key it is also the mechanism by which all traffic is always
/// encrypted for a specific endpoint only.
```

### Properties of EndpointIds

1. **Globally Unique**: Generated from cryptographically secure randomness
2. **Self-Certifying**: The ID itself proves ownership (via signatures)
3. **Location-Independent**: Doesn't encode IP address or location
4. **Persistent**: Remains the same regardless of network changes

### Addressing: EndpointAddr

To actually connect, you need an `EndpointAddr` which combines:

```rust
// From iroh-base/src/endpoint_addr.rs
pub struct EndpointAddr {
    pub id: EndpointId,                    // The node's identity
    pub addrs: BTreeSet<TransportAddr>,    // How to reach it
}

pub enum TransportAddr {
    Relay(RelayUrl),    // Via relay server
    Ip(SocketAddr),     // Direct IP address
    Custom(CustomAddr), // Custom transport
}
```

---

## Relay Architecture

### Why Relays?

Relays solve the NAT traversal problem:

1. **NATs block direct connections**: Most devices are behind NATs
2. **Hole-punching isn't always possible**: Symmetric NATs, strict firewalls
3. **Relays provide fallback**: Guaranteed connectivity via relay servers

### The Relay Protocol (DERP-based)

Iroh's relay protocol is based on Tailscale's DERP (Designated Encrypted Relay for Packets):

```rust
// From iroh-relay/src/protos/relay.rs
pub enum RelayToClientMsg {
    Datagrams {
        remote_endpoint_id: EndpointId,
        datagrams: Datagrams,
    },
    EndpointGone(EndpointId),
    Health { problem: String },
    Ping([u8; 8]),
    Pong([u8; 8]),
}

pub enum ClientToRelayMsg {
    Ping([u8; 8]),
    Pong([u8; 8]),
    Datagrams {
        dst_endpoint_id: EndpointId,
        datagrams: Datagrams,
    },
}
```

### Relay Handshake

The relay handshake authenticates clients using their Ed25519 keys:

```rust
// From iroh-relay/src/protos/handshake.rs
// Two authentication methods:

// 1. Challenge-Response (always works)
// Server sends challenge -> Client signs with SecretKey -> Server verifies

// 2. TLS Key Material Export (faster, when available)
// Uses RFC 5705 to extract keying material from TLS
// Client signs extracted material -> Server verifies
```

### RelayMap Configuration

```rust
// From iroh-relay/src/relay_map.rs
let relay_map = RelayMap::try_from_iter([
    "https://relay1.example.org",
    "https://relay2.example.org",
])?;
```

---

## Network Reporting

### NetReport System

Iroh continuously monitors network conditions:

```rust
// From iroh/src/net_report.rs
pub struct Report {
    pub have_v4: bool,           // IPv4 connectivity
    pub have_v6: bool,           // IPv6 connectivity
    pub have_udp: bool,          // UDP connectivity
    pub udp: bool,               // UDP works
    pub ipv4: Option<String>,    // Public IPv4
    pub ipv6: Option<String>,    // Public IPv6
    pub preferred_relay: Option<RelayUrl>,
    pub relay_latencies: RelayLatencies,
    // ... more fields
}
```

### Probe System

The network reporter runs probes to determine:
- IPv4/IPv6 connectivity
- NAT type (full cone, restricted, symmetric)
- Relay server latencies
- Direct UDP connectivity

---

## Address Lookup

### Publishing and Discovery

Nodes publish their addressing information automatically:

```rust
// From iroh/src/address_lookup.rs
trait AddressLookup {
    fn publish(&self, data: &EndpointData);
    fn resolve(&self, endpoint_id: EndpointId) -> Option<BoxStream<Result<Item, Error>>>;
}
```

### Available Implementations

| Service | Publish | Resolve | Description |
|---------|---------|---------|-------------|
| `PkarrPublisher` | Yes | No | Publish to pkarr relay via HTTP |
| `PkarrResolver` | No | Yes | Resolve from pkarr relay |
| `DnsAddressLookup` | No | Yes | Standard DNS resolution |
| `MdnsAddressLookup` | Yes | Yes | Local network mDNS |
| `DhtAddressLookup` | Yes | Yes | Mainline DHT |
| `MemoryLookup` | Yes | Yes | In-memory, manual |

### Pkarr Integration

Pkarr (Public-Key Addressable Resource Records) uses DNS records signed with Ed25519:

```rust
// From iroh/src/address_lookup/pkarr.rs
pub const N0_DNS_PKARR_RELAY_PROD: &str = "https://dns.iroh.link/pkarr";

// Publish
let publisher = PkarrPublisher::n0_dns();
// Automatically publishes relay URL under: <public_key>.dns.iroh.link
```

---

## Connection Flow

### Full Connection Sequence

```
1. Application: endpoint.connect(remote_id)
                              │
2. Address Lookup: Query for remote_id's EndpointAddr
   - Check memory cache
   - Query DNS/Pkarr
   - Query mDNS (local)
   └─> Returns: {id, relay_urls[], direct_addrs[]}
                              │
3. Network Report: Get current network state
   - Available interfaces
   - NAT type
   - Relay latencies
                              │
4. Connection Attempt:
   ┌─────────────────────────────────────────┐
   │ Parallel Attempts:                       │
   │ - Connect via relay (QUIC over HTTPS)   │
   │ - Send UDP packets to known addresses   │
   │ - Perform NAT hole-punching             │
   └─────────────────────────────────────────┘
                              │
5. Path Selection:
   - If direct works: Use direct UDP (lower latency)
   - If relay only: Continue via relay
   - If both: Use both (multipath QUIC)
                              │
6. TLS Handshake:
   - Verify remote's PublicKey
   - Establish encrypted channel
   - Cache session for 0-RTT
                              │
7. Connected: Return Connection
```

---

## Key Files Reference

### Core Types
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh-base/src/key.rs` - Ed25519 keys
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh-base/src/endpoint_addr.rs` - Addressing
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh-base/src/relay_url.rs` - Relay URLs

### Endpoint
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh/src/endpoint.rs` - Main API (3704 lines)
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh/src/socket.rs` - Socket layer (2498 lines)
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh/src/endpoint/connection.rs` - Connection handling

### Relay
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh-relay/src/protos/relay.rs` - Relay protocol
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh-relay/src/protos/handshake.rs` - Authentication
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh-relay/src/relay_map.rs` - Relay configuration

### Address Lookup
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh/src/address_lookup.rs` - Main trait
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh/src/address_lookup/pkarr.rs` - Pkarr integration
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh/src/address_lookup/mdns.rs` - mDNS

### Network Analysis
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh/src/net_report.rs` - Network probing (1220 lines)
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/iroh/src/net_report/reportgen.rs` - Report generation

---

## Production Patterns

### Recommended Setup

```rust
use iroh::{
    Endpoint, SecretKey,
    address_lookup::{self, PkarrPublisher},
    endpoint::{RelayMode, presets},
};

// Production setup with n0's infrastructure
let ep = Endpoint::builder(presets::N0)
    .address_lookup(PkarrPublisher::n0_dns())
    .address_lookup(address_lookup::DnsAddressLookup::n0_dns())
    .bind()
    .await?;
```

### Custom Relay Setup

```rust
use iroh_relay::RelayMap;

let relay_map = RelayMap::try_from_iter([
    "https://relay.your-domain.com",
])?;

let ep = Endpoint::builder(presets::Minimal)
    .relay_map(relay_map)
    .bind()
    .await?;
```

### Connection with ALPN

```rust
const MY_ALPN: &[u8] = b"my-protocol-v1";

let ep = Endpoint::builder(presets::N0)
    .alpn_protocols(vec![MY_ALPN.to_vec()])
    .bind()
    .await?;

// Connect
let conn = ep.connect(remote_id, MY_ALPN).await?;

// Accept
let incoming = ep.accept().await?;
let conn = incoming.accept().await?;
```

---

## See Also

- [p2p-ground-up.md](./p2p-ground-up.md) - Building P2P from scratch
- [cryptography-keys.md](./cryptography-keys.md) - Cryptographic details
- [iroh-ids.md](./iroh-ids.md) - Node identification deep dive
- [nat-traversal.md](./nat-traversal.md) - NAT traversal mechanisms
- [ewe-platform-integration.md](./ewe-platform-integration.md) - Integration guide
