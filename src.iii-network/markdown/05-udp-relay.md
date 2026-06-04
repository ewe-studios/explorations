---
title: UDP Relay — Non-DNS UDP Outside smoltcp
---

# UDP Relay — Non-DNS UDP Outside smoltcp

**The UDP relay handles non-DNS UDP datagrams outside smoltcp, bridging them directly to the host network.**

## UDP Relay Flow

Source: `udp_relay.rs` (309 lines)

```mermaid
flowchart TD
    A[Guest sends UDP] --> B{Port 53?}
    B -->|Yes| C[DNS interceptor]
    B -->|No| D[UDP relay]
    D --> E[Connect to host target]
    E --> F[Forward datagram]
    F --> G[Receive response]
    G --> H[Forward back to guest]
```

**Aha:** smoltcp's UDP support is limited — it doesn't handle dynamic socket creation well. By handling non-DNS UDP outside smoltcp via the relay, we avoid socket management complexity while still providing full UDP connectivity to the guest.

## Implementation

```mermaid
sequenceDiagram
    participant Guest as Guest VM
    participant Relay as UdpRelay
    participant Host as Host UDP socket

    Guest->>Relay: UDP datagram (non-DNS)
    Relay->>Relay: Lookup or create relay entry
    Relay->>Host: Send to target
    Host-->>Relay: Receive response
    Relay-->>Guest: Forward response via smoltcp
```

| Method | Purpose |
|--------|---------|
| `UdpRelay::new` | Create relay with tokio runtime |
| `handle_udp_datagram` | Forward to host, receive response |
| `cleanup` | Remove stale relay entries |

## What's Next

- [06 — Cross-Cutting](06-cross-cutting.md) — Backend, network orchestrator
- [03 — TCP Proxy](03-tcp-proxy.md) — Return to TCP proxy
- [00 — Overview](00-overview.md) — Return to overview
