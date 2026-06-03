---
title: Cross-Cutting Concerns — WASM, Portmapper, Metrics, Runtime, Custom Transports
---

# Cross-Cutting Concerns — WASM, Portmapper, Metrics, Runtime, Custom Transports

These concerns span multiple modules and affect how iroh behaves across different platforms and deployment scenarios.

## WASM Browser Support

Iroh compiles to `wasm32-unknown-unknown` with significant limitations:

### What Works
- Relay-only connectivity (no direct UDP in browsers)
- WebSocket-based relay transport
- Full TLS verification (raw public key)
- Address lookup (via HTTP to PKARR relay)

### What Doesn't Work
- Direct UDP socket binding (browser sandbox)
- Portmapper (UPnP/NAT-PMP)
- Network change detection (limited browser APIs)
- QAD probes (requires raw UDP)

### WASM-Specific Dependencies

```toml
# iroh/iroh/Cargo.toml
[target.'cfg(all(target_family = "wasm", target_os = "unknown"))'.dependencies]
wasm-bindgen-futures = "0.4"
time = { version = "0.3", features = ["wasm-bindgen"] }
getrandom = { version = "0.4", features = ["wasm_js"] }
ws_stream_wasm = "0.7.4"  # relay transport via WebSocket
```

Source: `iroh/iroh/Cargo.toml:1` — Target-specific WASM dependencies.

### Runtime Fallback

```rust
// iroh/src/runtime.rs
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
impl Runtime for WasmRuntime {
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        // Falls back to wasm_bindgen_futures::spawn_local
        wasm_bindgen_futures::spawn_local(future);
    }
}
```

Source: `iroh/src/runtime.rs:1` — WASM runtime uses `wasm_bindgen_futures` instead of tokio.

## Portmapper

The `portmapper` feature enables UPnP and NAT-PMP port mapping:

```rust
// iroh/src/portmapper.rs
#[cfg(feature = "portmapper")]
pub use portmapper::Client as Portmapper;

#[cfg(not(feature = "portmapper"))]
pub struct Portmapper( /* stub: Disabled */ );
```

Source: `iroh/src/portmapper.rs:1` — Feature gate with no-op stub for non-portmapper builds.

When enabled, the portmapper:
1. Discovers UPnP/NAT-PMP gateways on the local network
2. Requests port forwarding for the UDP socket
3. Updates the endpoint's advertised addresses

## Metrics

The `metrics` feature enables Prometheus-compatible metrics:

```rust
// iroh/src/metrics.rs
pub struct EndpointMetrics {
    pub socket: Arc<SocketMetrics>,
    pub net_report: Arc<NetReportMetrics>,
}
```

Source: `iroh/src/metrics.rs:1` — `EndpointMetrics` aggregates socket and net_report metrics via `iroh-metrics`.

Source: `iroh/iroh/Cargo.toml:features` — `metrics = ["iroh-metrics/metrics", "iroh-relay/metrics"]`

## Custom Transports (Unstable)

The `unstable-custom-transports` feature allows user-defined transport implementations:

```rust
// iroh/src/socket/transports.rs
pub enum TransportConfig {
    Ip { ... },
    Relay { ... },
    Custom { transport: Arc<dyn CustomTransport> },
}
```

Source: `iroh/iroh/Cargo.toml:features` — `unstable-custom-transport = []` (empty feature, API may change).

**Warning:** This feature is explicitly marked unstable and may change without notice.

## Runtime Management

The `Runtime` struct manages task lifecycle across platforms:

```rust
// iroh/src/runtime.rs
pub struct Runtime {
    tracker: TaskTracker,
    cancel: CancellationToken,
}
```

Source: `iroh/src/runtime.rs:1` — `Runtime` wraps `tokio_util::TaskTracker` for structured concurrency and `CancellationToken` for graceful shutdown.

Implements `noq::Runtime` trait:
- `spawn()` — spawn a background task
- `new_timer()` — create a sleep timer
- `wrap_udp_socket()` — wrap a UDP socket (for WASM)

## Network Change Detection

When the OS reports a network change:

1. The Socket's `NetworkChangeSender` fires
2. All `RemoteStateActor`s are notified
3. Each actor pings its known paths
4. Stale paths are invalidated
5. New address lookups are triggered
6. A full net_report may be triggered

Source: `iroh/src/socket.rs:1` (network change handling), `iroh/src/socket/transports/relay.rs:1` (`RelayNetworkChangeSender`).

## TLS Crypto Backend Selection

| Feature | Crypto Backend | Use Case |
|---------|---------------|----------|
| `tls-ring` | Ring | Default, works everywhere |
| `tls-aws-lc-rs` | AWS LC-RS | Post-quantum key exchange support |

Source: `iroh/iroh/Cargo.toml:features` — Only one crypto backend should be enabled. `tls-aws-lc-rs` is required for post-quantum examples (`pq-only-key-exchange`, `prefer-pq-key-exchange`).

## Examples

The iroh crate ships with 15 examples:

| Example | Features | Description |
|---------|----------|-------------|
| `echo` | — | Echo protocol (bidirectional stream) |
| `listen` | — | Listen for incoming connections |
| `connect` | — | Connect to a listening endpoint |
| `search` | — | Search/discovery example |
| `transfer` | — | File transfer example |
| `0rtt` | — | 0-RTT connection example |
| `incoming-filter` | — | Incoming connection filtering |
| `custom-transport` | `unstable-custom-transports` | Custom transport example |
| `listen-unreliable` | — | Unreliable datagram example |
| `connect-unreliable` | — | Connect with unreliable transport |
| `echo-no-router` | — | Echo without Router abstraction |
| `pq-only-key-exchange` | `tls-aws-lc-rs` | Post-quantum only |
| `prefer-pq-key-exchange` | `tls-aws-lc-rs` | Prefer post-quantum |

Source: `iroh/iroh/Cargo.toml:[[example]]` sections.

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Overall architecture
- [TLS Layer](../markdown/06-tls.md) — TLS crypto backend details
- [Socket Layer](../markdown/07-socket.md) — Transport management
