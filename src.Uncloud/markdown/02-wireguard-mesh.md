---
title: WireGuard Mesh — Network Setup, Peer Discovery, NAT Traversal
---

# WireGuard Mesh — Network Setup, Peer Discovery, NAT Traversal

**Uncloud creates a WireGuard mesh network between all Docker hosts — providing encrypted communication, automatic peer discovery, and NAT traversal without manual configuration.**

## WireGuard Network Architecture

Source: `internal/machine/network/` (1,105 LOC)

```mermaid
flowchart TB
    subgraph M1["Machine 1 (192.168.1.100:51820)"]
        WG1["uncloud interface"]
        WG1_IP["10.0.0.1/24"]
    end

    subgraph M2["Machine 2 (10.0.0.50:51820)"]
        WG2["uncloud interface"]
        WG2_IP["10.0.0.2/24"]
    end

    subgraph M3["Machine 3 (NAT behind router)"]
        WG3["uncloud interface"]
        WG3_IP["10.0.0.3/24"]
    end

    WG1 <-->|"encrypted tunnel"| WG2
    WG2 <-->|"encrypted tunnel"| WG3
    WG1 <-->|"NAT traversal"| WG3
```

## WireGuard Network Setup

Source: `internal/machine/network/wireguard.go`

```go
const (
    WireGuardInterfaceName = "uncloud"
    DefaultWireGuardPort   = 51820
    WireGuardKeepaliveInterval = 25 * time.Second  // Works with most firewalls
)
```

### Key Generation

```go
func NewMachineKeys() (privKey, pubKey secret.Secret, err error) {
    wgPrivKey, err := wgtypes.GeneratePrivateKey()
    // privKey and pubKey are byte slices of the key material
}
```

Each machine generates its own WireGuard key pair on first join.

## Peer Discovery and NAT Traversal

Source: `internal/machine/network/peer.go`

```mermaid
sequenceDiagram
    participant M1 as Machine 1
    participant M2 as Machine 2
    
    M1->>M1: Generate WireGuard keys
    M1->>M2: Join cluster (share public key + endpoint)
    M2->>M2: Add M1 as WireGuard peer
    M2->>M1: Respond with own public key + endpoint
    M1->>M1: Add M2 as WireGuard peer
    
    Note over M1,M2: NAT traversal via Corrosion state sync
    M2->>M2: Detect endpoint change (new public IP)
    M2->>M1: EndpointChangeEvent via Corrosion
    M1->>M1: Update WireGuard peer endpoint
```

### Endpoint Changes

```go
type EndpointChangeEvent struct {
    PublicKey secret.Secret
    Endpoint netip.AddrPort  // New endpoint address
}
```

The 25-second keepalive interval ensures NAT bindings stay alive, and endpoint changes are propagated through Corrosion state sync.

## IP Address Assignment

Source: `internal/machine/network/address.go`

Each machine gets a unique IP in the `10.0.0.0/24` subnet:

| Machine | WireGuard IP | Purpose |
|---------|-------------|---------|
| Machine 1 | 10.0.0.1 | First cluster member |
| Machine 2 | 10.0.0.2 | Second cluster member |
| Machine 3 | 10.0.0.3 | Third cluster member |

Containers on different machines communicate directly through the WireGuard mesh — no overlay network, no port mapping needed.

## Platform-Specific Implementation

**Aha:** The WireGuard keepalive interval of 25 seconds is specifically chosen because it works with the widest range of firewalls — too short and it wastes bandwidth, too long and stateful firewalls drop the connection. This is a battle-tested value from the WireGuard project.

| File | Platform | Purpose |
|------|----------|---------|
| `wireguard.go` | All | Core WireGuard logic |
| `wireguard_linux.go` | Linux | Linux-specific wgctrl setup |
| `wireguard_darwin.go` | macOS | macOS-specific wgctrl setup |

## What's Next

- [03 — Machine & Cluster](03-machine-cluster.md) — clusterController, state management
- [01 — Architecture](01-architecture.md) — Return to architecture
- [08 — Corrosion CRDT](08-corrosion-crdt.md) — Return to Corrosion
