# Iroh P2P Exploration

Comprehensive exploration of [iroh](https://github.com/n0-computer/iroh) - production-grade peer-to-peer networking in Rust.

**Source explored:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/`

---

## Documents

| Document | Purpose |
|----------|---------|
| [exploration.md](./exploration.md) | **Start here** - Complete overview of iroh architecture, components, and capabilities |
| [p2p-ground-up.md](./p2p-ground-up.md) | How to build production-grade P2P from scratch (UDP, cryptography, NAT traversal, relays) |
| [cryptography-keys.md](./cryptography-keys.md) | Deep dive into Ed25519 cryptography, TLS with Raw Public Keys, authentication |
| [iroh-ids.md](./iroh-ids.md) | How EndpointIds work - node identification, addressing, and connection establishment |
| [nat-traversal.md](./nat-traversal.md) | NAT traversal strategies - hole punching, relay fallback, STUN/ICE |
| [ewe-platform-integration.md](./ewe-platform-integration.md) | Integration guide for adding iroh to ewe-platform project |

---

## Key Takeaways

### 1. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Iroh Endpoint                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Ed25519     │  │ QUIC +      │  │ Address Lookup      │  │
│  │ Keys        │  │ Relay       │  │ (Pkarr/DNS/mDNS)    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 2. Cryptography

- **Ed25519** (not RSA) - 32-byte keys, fast signatures, deterministic
- **TLS 1.3 with Raw Public Keys** (RFC 7250) - no X.509 certificates needed
- **Self-certifying identifiers** - the public key IS the node ID

### 3. Connectivity

- **Direct UDP** when possible (lowest latency)
- **UDP hole punching** for NAT traversal (~80-90% success)
- **Relay fallback** for guaranteed connectivity (~100% success)
- **Multipath QUIC** for seamless path switching

### 4. Address Discovery

- **mDNS** for local network discovery
- **Pkarr/DNS** for global discovery via signed DNS records
- **DHT** for decentralized lookup (Mainline DHT)

---

## Quick Reference

### Creating an Endpoint

```rust
use iroh::{Endpoint, SecretKey, endpoint::presets};

let endpoint = Endpoint::builder(presets::N0)
    .address_lookup(PkarrPublisher::n0_dns())
    .address_lookup(address_lookup::DnsAddressLookup::n0_dns())
    .bind()
    .await?;

println!("My node ID: {}", endpoint.node_id());
```

### Connecting to a Peer

```rust
// Connect using just the peer's ID
// (address lookup happens automatically)
let peer_id: EndpointId = "ae58ff883324..."
    .parse()?;

let conn = endpoint.connect(peer_id, b"my-alpn").await?;

// Use QUIC streams
let (mut send, mut recv) = conn.open_bi().await?;
send.write_all(b"Hello, P2P!").await?;
```

### Accepting Connections

```rust
while let Some(incoming) = endpoint.accept().await {
    let conn = incoming.accept().await?;

    tokio::spawn(async move {
        // Handle connection
        let (mut send, mut recv) = conn.accept_bi().await?;
        // ... process data
    });
}
```

---

## Production Patterns

### Recommended Setup

```rust
use iroh::{
    Endpoint,
    address_lookup::{self, PkarrPublisher, AddrFilter},
    endpoint::{RelayMode, presets},
};

let endpoint = Endpoint::builder(presets::N0)
    // Always use relays for fallback
    .relay_mode(RelayMode::Default)
    // Publish to global DNS
    .address_lookup(PkarrPublisher::n0_dns())
    // Resolve from DNS
    .address_lookup(address_lookup::DnsAddressLookup::n0_dns())
    // Enable local discovery
    .address_lookup(address_lookup::MdnsAddressLookup::builder())
    // Filter: relay-only for privacy, or unfiltered for direct IPs
    .addr_filter(AddrFilter::relay_only())
    .bind()
    .await?;
```

### Connection with Timeout

```rust
use tokio::time::{timeout, Duration};

let conn = timeout(
    Duration::from_secs(10),
    endpoint.connect(peer_id, ALPN)
).await??;
```

### Monitor Network Status

```rust
let report = endpoint.net_report().await;
println!("IPv4: {}, IPv6: {}, UDP: {}",
    report.ipv4, report.ipv6, report.udp);
println!("Preferred relay: {:?}", report.preferred_relay);
```

---

## File Reference

### Core Source Files

| File | Lines | Purpose |
|------|-------|---------|
| `iroh/src/endpoint.rs` | 3704 | Main endpoint API |
| `iroh/src/socket.rs` | 2498 | Socket/transport layer |
| `iroh/src/net_report.rs` | 1220 | Network detection |
| `iroh/src/address_lookup.rs` | 1137 | Address discovery |
| `iroh-base/src/key.rs` | 526 | Ed25519 key types |
| `iroh-relay/src/protos/relay.rs` | 784 | Relay protocol |
| `iroh-relay/src/protos/handshake.rs` | 847 | Authentication |

---

## Comparison with Alternatives

| Feature | iroh | libp2p | Tailscale | WebRTC |
|---------|------|--------|-----------|--------|
| Language | Rust | Rust/JS/Go | Go | JS/native |
| Crypto | Ed25519 | Various | Ed25519 | DTLS |
| NAT Traversal | Relay + Hole Punch | Various | Relay (DERP) | STUN/TURN |
| Relay Protocol | DERP-based | Various | DERP | TURN |
| Address Discovery | Pkarr/DNS/mDNS | Kademlia | Central | Signaling |
| QUIC Support | Yes (native) | Yes | No | No |
| Browser Support | Limited | Limited | No | Yes |

---

## Resources

- **Source:** https://github.com/n0-computer/iroh
- **Documentation:** https://docs.rs/iroh
- **n0.computer:** https://n0.computer
- **Pkarr:** https://pkarr.org

---

## Integration Status

For ewe-platform integration, see [ewe-platform-integration.md](./ewe-platform-integration.md).

Recommended integration phases:
1. ✅ Basic P2P connectivity
2. ⏳ Service discovery
3. ⏳ Blob transfer
4. ⏳ Distributed features
