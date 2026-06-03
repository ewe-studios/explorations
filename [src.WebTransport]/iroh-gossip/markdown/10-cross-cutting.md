---
title: Cross-Cutting — Metrics, Serialization, Addressing, WASM
---

# Cross-Cutting Concerns — Metrics, Serialization, Addressing, WASM

These concerns span multiple modules and affect how iroh-gossip behaves across different scenarios.

## Metrics

Iroh-gossip exposes 18 Prometheus counters:

```rust
// iroh-gossip/src/metrics.rs
pub struct Metrics {
    // Control traffic (HyParView membership messages)
    pub msgs_ctrl_sent: Counter,      // Join, Shuffle, Neighbor, Disconnect
    pub msgs_ctrl_recv: Counter,
    pub msgs_ctrl_sent_size: Counter, // bytes
    pub msgs_ctrl_recv_size: Counter,
    // Data traffic (PlumTree broadcast messages)
    pub msgs_data_sent: Counter,      // Gossip, IHave, IWant, Graft, Prune
    pub msgs_data_recv: Counter,
    pub msgs_data_sent_size: Counter, // bytes
    pub msgs_data_recv_size: Counter,
    // Neighbor changes
    pub neighbor_up: Counter,         // peers joining active view
    pub neighbor_down: Counter,       // peers leaving active view
    // Actor loop health
    pub actor_tick_main: Counter,     // main loop iterations
    pub actor_tick_rx: Counter,       // received messages processed
    pub actor_tick_endpoint: Counter, // endpoint events
    pub actor_tick_dialer: Counter,   // dial attempts
    pub actor_tick_dialer_success: Counter,
    pub actor_tick_dialer_failure: Counter,
    pub actor_tick_in_event_rx: Counter, // API commands received
    pub actor_tick_timers: Counter,   // timer expirations
}
```

Source: `iroh-gossip/src/metrics.rs:1` — All counters are incremented in the Actor loop and protocol handlers.

## Serialization: Postcard

All protocol messages use **postcard** for serialization:

```rust
// iroh-gossip/src/proto/state.rs
// Encoding
let data: Bytes = postcard::to_allocvec(&message).unwrap().into();

// Decoding
let message: Message = postcard::from_bytes(&data)?;
```

Why postcard?
- **no_std compatible** — works in embedded/WASM environments
- **Compact** — smaller than JSON, MessagePack, or bincode for typical gossip messages
- **Serde-based** — uses `#[derive(Serialize, Deserialize)]` on all message types

Source: `iroh-gossip/Cargo.toml:1` — `postcard = { version = "1", features = ["alloc", "use-std", "experimental-derive"] }`

## Maximum Message Size

```rust
// iroh-gossip/src/proto.rs
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;  // 64 MB
pub const MIN_MAX_MESSAGE_SIZE: usize = 64 * 1024;              // 64 KB
```

Source: `iroh-gossip/src/proto.rs:1` — The 64MB default allows large payloads (file chunks, state snapshots) while the 64KB minimum prevents abuse.

## Addressing

### TopicId

Topics are identified by their BLAKE3 hash:

```rust
// iroh-gossip/src/proto.rs
pub struct TopicId([u8; 32]);

impl TopicId {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        blake3::hash(bytes).into()
    }
}
```

Source: `iroh-gossip/src/proto.rs:1` — BLAKE3 provides a 32-byte collision-resistant topic identifier.

### PeerIdentity

Peers are identified by `iroh_base::NodeId` (Ed25519 public key):

```rust
// iroh-gossip/src/proto.rs
impl PeerIdentity for iroh_base::NodeId {
    type Data = PeerData;
}
```

Source: `iroh-gossip/src/proto.rs:1` — The `PeerIdentity` trait is generic, allowing custom identity types in simulations.

## WASM Support

The `proto/` module is fully no_std compatible, meaning it compiles to WASM without modification. The `net/` module requires iroh (which has WASM support with relay-only mode).

Feature flags affecting WASM:
- `net` feature: requires iroh (WASM-compatible with relay-only)
- `rpc` feature: requires noq (WASM-compatible)
- `simulator` feature: NOT WASM-compatible (uses rayon)

Source: `iroh-gossip/Cargo.toml:1` — Feature dependencies.

## GOSSIP_ALPN

```rust
// iroh-gossip/src/net.rs
pub const GOSSIP_ALPN: &[u8] = b"/iroh-gossip/1";
```

Source: `iroh-gossip/src/net.rs:1` — The ALPN used for iroh-gossip connections. This must be registered with the iroh Router.

## Chat Example

The chat example demonstrates signed message exchange:

```bash
cargo run --example chat -- --open
# Outputs a ticket string to share with other peers

cargo run --example chat -- --join <ticket>
# Connects to the swarm and starts chatting
```

Source: `iroh-gossip/examples/chat.rs:1-318` — Full chat application with Ed25519-signed messages, ticket-based bootstrap, and subscribe loop.

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Protocol layers
- [Simulation](../markdown/07-simulation.md) — Testing framework
- [API](../markdown/06-api.md) — Application-facing API
