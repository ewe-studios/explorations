---
title: Data Flow — End-to-End Connection Sequences
---

# Data Flow — End-to-End Connection Sequences

This document traces the complete data flow from application protocol handler to wire-level datagrams.

## Connection Establishment Flow

```mermaid
sequenceDiagram
    participant App as Application (ProtocolHandler)
    participant Router as Router
    participant EP as Endpoint
    participant Socket as Socket
    participant Trans as Transports (IP/Relay)
    participant Noq as noq (QUIC)
    participant Remote as Remote Endpoint

    App->>Router: builder(endpoint).accept(ALPN, handler).spawn()
    Router->>EP: start accept loop
    EP->>Socket: await incoming datagrams

    Remote->>Noq: connect(addr, ALPN)
    Noq->>Trans: QUIC handshake packets
    alt direct path available
        Trans->>Socket: send via IpTransport (UDP)
    else relay path only
        Trans->>Socket: send via RelayTransport (HTTPS)
    end
    Socket->>EP: deliver packets
    EP->>EP: TLS verification (raw public key)
    EP->>Router: dispatch by ALPN
    Router->>App: ProtocolHandler::accept(connection)
    App->>Noq: connection.accept_bi()
    Noq-->>App: (send, recv) stream pair
```

Source: `iroh/src/endpoint.rs:1` (accept loop), `iroh/src/protocol.rs:1` (dispatch), `iroh/src/socket.rs:1` (transport routing).

## Outbound Connection Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant EP as Endpoint
    participant AL as AddressLookup
    participant NR as NetReport
    participant Socket as Socket
    participant Remote as Remote Endpoint

    App->>EP: connect(addr, alpn)
    EP->>AL: resolve EndpointAddr (if needed)
    AL-->>EP: EndpointAddr (direct + relay)
    EP->>NR: get current report
    NR-->>EP: preferred relay, NAT status
    EP->>Socket: initiate connection
    Socket->>Socket: try direct UDP first
    alt direct succeeds
        Socket->>Remote: QUIC handshake (direct)
    else direct fails
        Socket->>Socket: fall back to relay
        Socket->>Remote: QUIC handshake (via relay)
    end
    Remote-->>Socket: connection established
    Socket-->>EP: Connection
    EP-->>App: Connection
    loop path monitoring
        Socket->>NR: incremental report
        NR-->>Socket: updated relay latencies
        Socket->>Socket: path selector re-evaluates
    end
```

Source: `iroh/src/endpoint.rs:1` (connect), `iroh/src/socket.rs:1` (connection initiation), `iroh/src/net_report.rs:1` (reporting).

## Data Transfer Flow

```mermaid
sequenceDiagram
    participant Sender as Sending ProtocolHandler
    participant Conn as QUIC Connection
    participant Noq as noq QUIC Stack
    participant Socket as Socket
    participant Trans as Transport (IP or Relay)
    participant Remote as Remote Endpoint
    participant Recv as Receiving ProtocolHandler

    Sender->>Conn: send.write_all(data)
    Conn->>Noq: QUIC stream frame
    Noq->>Socket: encrypted datagram
    Socket->>Trans: route to best path
    alt direct path selected
        Trans->>Remote: UDP datagram
    else relay path selected
        Trans->>Remote: HTTPS/WebSocket relay
    end
    Remote->>Socket: receive datagram
    Socket->>Noq: decrypt
    Noq->>Conn: QUIC stream frame
    Conn->>Recv: recv.read(data)
```

Source: `iroh/src/socket.rs:1` (datagram routing), `iroh/src/socket/remote_map/remote_state.rs:1` (path selection).

## Path Upgrade Flow

```mermaid
sequenceDiagram
    participant EP as Endpoint
    participant NR as NetReport
    participant Socket as Socket
    participant PS as PathSelector
    participant Relay as Relay Server

    EP->>EP: connected via relay
    EP->>NR: periodic probe
    NR->>NR: discover direct address
    NR-->>EP: new direct address available
    EP->>Socket: add direct path
    Socket->>Socket: initiate hole-punch
    Socket->>Socket: measure RTT on direct path
    Socket->>PS: compare direct vs relay RTT
    alt direct is faster
        PS->>Socket: switch to direct path
        Socket->>EP: path upgraded
    else relay is faster (or equal)
        PS->>Socket: stay on relay
    end
```

Source: `iroh/src/socket/remote_map/remote_state.rs:1` (path selection), `iroh/src/net_report.rs:1` (probe results).

## Shutdown Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant Router as Router
    participant EP as Endpoint
    participant Socket as Socket
    participant AllConn as All Connections

    App->>Router: shutdown().await
    Router->>EP: cancel token triggered
    EP->>Socket: close endpoint
    Socket->>AllConn: close all paths (IP + Relay)
    AllConn-->>Socket: paths closed
    Socket-->>EP: socket closed
    EP-->>Router: endpoint closed
    Router-->>App: shutdown complete
```

Source: `iroh/src/protocol.rs:1` (shutdown), `iroh/src/endpoint.rs:1` (close).

## Related Documents

- [Endpoint](../markdown/02-endpoint.md) — The Endpoint API
- [Socket Layer](../markdown/07-socket.md) — Transport management and path selection
- [Protocol Dispatch](../markdown/03-protocol.md) — ALPN dispatch
