---
title: Relay Server — Architecture and Operation
---

# Relay Server — Architecture and Operation

The iroh relay server forwards datagrams between endpoints that cannot establish a direct connection. It's the fallback path when hole-punching fails.

## What the Relay Does

```
┌─────────────┐                    ┌─────────────┐
│  Endpoint A  │                    │  Endpoint B  │
│  (behind NAT)│                    │  (behind NAT)│
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │  QUIC over HTTPS/WebSocket       │
       ▼                                  ▼
┌─────────────────────────────────────────────────┐
│              iroh-relay server                  │
│                                                 │
│  HTTP endpoint: probe measurement               │
│  HTTPS endpoint: datagram forwarding            │
│  WebSocket: browser/WASM clients                │
│  QUIC: native clients                           │
│  ACME: automatic TLS certificate management     │
└─────────────────────────────────────────────────┘
```

Source: `iroh-relay/Cargo.toml:1` — `iroh-relay` provides both client and server implementations.

## Server Configuration

The relay server runs as a standalone binary:

```bash
iroh-relay --config relay.toml
```

Source: `iroh-relay/src/main.rs` — requires the `server` feature flag.

### Server Feature Dependencies

| Dependency | Purpose |
|-----------|---------|
| `clap` | CLI argument parsing |
| `dashmap` | Concurrent connection map |
| `tokio-websockets` | WebSocket support (0.13) |
| `tokio-rustls-acme` | ACME/Let's Encrypt TLS |
| `rustls-cert-reloadable-resolver` | Hot certificate reload |
| `rcgen` | Self-signed cert generation |
| `toml` | Configuration file parsing |
| `tracing-subscriber` | Logging |

Source: `iroh-relay/Cargo.toml:features:server`

## ACME Certificate Management

The relay server uses `tokio-rustls-acme` for automatic TLS certificate management:

1. Server starts with a self-signed certificate
2. ACME client contacts Let's Encrypt
3. HTTP-01 or TLS-ALPN-01 challenge validation
4. Certificate obtained and loaded
5. Certificates auto-renew before expiry

Source: `iroh-relay/Cargo.toml:server` — `tokio-rustls-acme = "0.9"`, `rustls-cert-reloadable-resolver = "0.7.1"`.

## Connection Handling

The relay server manages connections from multiple endpoints:

```
Client A ──HTTPS──▶ Relay ──HTTPS──▶ Client B
                    │
                    └── dashmap<ConnectionId, ClientState>
```

Source: `iroh-relay/src/` — Connection state tracked in `dashmap` for concurrent access.

## Probe Endpoint

The relay exposes an HTTPS endpoint for net_report probes:

```
GET /ping
→ 200 OK (measures HTTPS latency)
```

Source: `iroh/src/net_report/reportgen.rs:1` — `run_https_probe()` sends an HTTPS GET to the relay's `/ping` endpoint.

## Default Relay Configuration

Source: `iroh/src/defaults.rs` — Default relay hostname is `relays.iroh.link`, ports: HTTP 80, HTTPS 443, QUIC 443.

## RelayUrl Type

```rust
// iroh-base
pub struct RelayUrl(Url);
```

Source: `iroh-base` — `RelayUrl` wraps a `Url` for type-safe relay address handling.

## Production Deployment

The production relay server runs at `dns.iroh.link` (DNS server) and `relays.iroh.link` (relay server).

Source: `iroh/README.md:1` — "DNS server implementation powering the `n0_discovery` for EndpointIds, running at dns.iroh.link."

## Related Documents

- [Network Report](../markdown/05-net_report.md) — How the client probes the relay
- [Address Lookup](../markdown/04-address-lookup.md) — DNS resolution via relay
- [Data Flow](../markdown/09-data-flow.md) — Relay forwarding sequence
