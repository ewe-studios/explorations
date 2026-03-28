---
title: "Zero to QUIC Engineer"
subtitle: "Understanding QUIC from first principles - TCP, TLS, and the path to HTTP/3"
---

# Zero to QUIC Engineer

## Introduction

This guide takes you from understanding basic network protocols to comprehending why QUIC was designed and how it solves fundamental problems with TCP+TLS. We'll build up knowledge layer by layer.

## Table of Contents

1. [The Problem with TCP](#1-the-problem-with-tcp)
2. [TLS Overhead](#2-tls-overhead)
3. [QUIC Design Principles](#3-quic-design-principles)
4. [QUIC vs TCP+TLS Comparison](#4-quic-vs-tcptls-comparison)
5. [HTTP/3 Motivation](#5-http3-motivation)
6. [First QUIC Connection](#6-first-quic-connection)

---

## 1. The Problem with TCP

### 1.1 TCP Basics

TCP (Transmission Control Protocol) has been the backbone of Internet transport since 1981. It provides:

- **Reliable delivery** - packets arrive in order, no loss
- **Flow control** - sender doesn't overwhelm receiver
- **Congestion control** - sender adapts to network conditions
- **Connection-oriented** - stateful connection with handshake

```
Client                          Server
   |                              |
   |-------- SYN ---------------->|  (1) Client initiates
   |                              |
   |<------- SYN-ACK -------------|  (2) Server acknowledges
   |                              |
   |-------- ACK ---------------->|  (3) Connection established
   |                              |
   |-------- DATA --------------->|  (4) Data transfer
   |<------- ACK -----------------|
   |                              |
```

**TCP Three-Way Handshake:** 1 RTT before any data can be sent.

### 1.2 Head-of-Line Blocking

The fundamental problem with TCP is **head-of-line (HOL) blocking**:

```
Application sends:  [Request 1] [Request 2] [Request 3]
                     (Stream A) (Stream B) (Stream C)

TCP sees:          [Packet 1] [Packet 2] [Packet 3] [Packet 4] [Packet 5]
                     (Seq 0)  (Seq 1)  (Seq 2)  (Seq 3)  (Seq 4)

Network loses:               [Packet 2] ❌

Receiver gets:   [Packet 1]         [Packet 3] [Packet 4] [Packet 5]
                  (Seq 0)          (Seq 2)  (Seq 3)  (Seq 4)

Application waits... waits... waits... for Packet 2 retransmission

ALL streams blocked waiting for ONE lost packet
```

**The Problem:** TCP delivers bytes in order. If packet 2 is lost, packets 3-5 sit in the kernel buffer even though they belong to different application streams (B and C).

### 1.3 HTTP/1.1 and HTTP/2 over TCP

**HTTP/1.1:** One request/response at a time per connection.

```
Client: GET /a.html
Server: [sends a.html]
Client: GET /b.css     (must wait for a.html to complete)
Server: [sends b.css]
```

**HTTP/2:** Multiplexed streams over single TCP connection.

```
Client: GET /a.html (stream 1)
Client: GET /b.css  (stream 2)
Client: GET /c.js   (stream 3)

Server: [interleaved frames from all streams]
```

**BUT:** HTTP/2 still suffers from TCP HOL blocking:

```
Single TCP connection carrying 100 HTTP/2 streams

One packet lost → ALL 100 streams blocked

Even though streams are independent at HTTP/2 layer,
TCP doesn't know this and delivers bytes in order.
```

### 1.4 Connection Establishment Latency

TCP + TLS 1.2 = **2-3 RTTs** before first byte:

```
RTT 0: TCP SYN, SYN-ACK, ACK
RTT 1: TLS ClientHello, ServerHello, Certificate
RTT 2: TLS ClientKeyExchange, Finished
RTT 3: Application data (finally!)
```

For a mobile user with 100ms RTT, that's **300ms minimum** before any data.

---

## 2. TLS Overhead

### 2.1 TLS 1.2 Handshake

```
Client                          Server
   |                              |
   |-------- ClientHello -------->|
   |                              |
   |<------- ServerHello ---------|
   |<------- Certificate ---------|
   |<------- ServerKeyExchange ---|
   |                              |
   |-------- ClientKeyExchange -->|
   |-------- ChangeCipherSpec --->|
   |-------- Finished ----------->|
   |                              |
   |<------- ChangeCipherSpec ----|
   |<------- Finished ------------|
   |                              |
   |======== ENCRYPTED DATA ======|
```

**Problems:**
1. Multiple round trips
2. Full handshake every new connection
3. No connection migration (new connection = new handshake)

### 2.2 TLS 1.3 Improvements

TLS 1.3 reduced to **1 RTT**:

```
Client                          Server
   |                              |
   |-------- ClientHello -------->| (includes key share)
   |                              |
   |<------- ServerHello ---------| (includes key share)
   |<------- Certificate ---------|
   |<------- Finished ------------|
   |                              |
   |======== ENCRYPTED DATA ======| (can send immediately)
```

**0-RTT Resumption:** If you've connected before, send data with first flight:

```
Client                          Server
   |                              |
   |-------- ClientHello + Data ->| (0-RTT early data)
   |                              |
   |<------- ServerHello ---------|
   |<------- Finished ------------|
   |                              |
   |======== ENCRYPTED DATA ======|
```

**BUT:** TLS 1.3 0-RTT still runs over TCP, so you still need the TCP handshake first.

---

## 3. QUIC Design Principles

### 3.1 What is QUIC?

**QUIC** (Quick UDP Internet Connections) is a transport protocol designed by Google, now standardized by the IETF in RFC 9000.

**Key insight:** Move congestion control, reliability, and security into **application space** over UDP, not kernel TCP.

### 3.2 Design Goals

1. **Eliminate HOL blocking** - Independent streams at transport layer
2. **Reduce latency** - Combine handshake with crypto
3. **Connection migration** - Survive network changes
4. **Improved congestion control** - Modern algorithms, faster deployment
5. **Encryption by default** - Everything encrypted, including most metadata

### 3.3 QUIC Architecture

```
+----------------------------------+
|          Application             |
|     (HTTP/3, custom protocol)    |
+----------------------------------+
|           QUIC                   |
|  +---------------------------+   |
|  |  Stream Multiplexing      |   |
|  +---------------------------+   |
|  |  Reliability / ACKs       |   |
|  +---------------------------+   |
|  |  Congestion Control       |   |
|  +---------------------------+   |
|  |  TLS 1.3 Handshake      |   |
|  +---------------------------+   |
+----------------------------------+
|            UDP                   |
+----------------------------------+
|            IP                    |
+----------------------------------+
```

### 3.4 QUIC Connection Establishment

**Combined crypto + transport handshake = 1 RTT:**

```
Client                          Server
   |                              |
   |-------- ClientHello -------->| (QUIC Initial packet)
   |         (crypto handshake)   |
   |                              |
   |<------- ServerHello ---------| (QUIC Handshake packets)
   |         (crypto + transport) |
   |                              |
   |======== ENCRYPTED DATA ======| (1-RTT packets)
```

**0-RTT:** If you have session ticket, send application data immediately:

```
Client                          Server
   |                              |
   |-------- ClientHello + Data ->| (0-RTT QUIC packet)
   |                              |
   |<------- ServerHello ---------|
   |                              |
   |======== ENCRYPTED DATA ======|
```

### 3.5 Connection IDs

QUIC uses **Connection IDs** instead of (IP, port) tuples:

```
TCP connection: (192.168.1.100:54321, 93.184.216.34:443)
   ↓
Client moves to WiFi (IP changes)
   ↓
Connection broken! Must reconnect.

QUIC connection: Connection ID: 0xba9c4e2f
   ↓
Client moves to WiFi (IP changes)
   ↓
Send packet with same Connection ID from new IP
   ↓
Server recognizes Connection ID, continues!
```

**Connection Migration:**
```rust
// quiche handles this automatically
let recv_info = quiche::RecvInfo {
    from: new_peer_address,  // Changed!
    to: local_address,
};
conn.recv(&mut buf, recv_info)?;  // Connection survives
```

---

## 4. QUIC vs TCP+TLS Comparison

### 4.1 Round Trip Comparison

| Scenario | TCP+TLS 1.2 | TCP+TLS 1.3 | QUIC | QUIC 0-RTT |
|----------|-------------|-------------|------|------------|
| First connection | 3 RTT | 2 RTT | 1 RTT | N/A |
| Returning client | 3 RTT | 1 RTT | 1 RTT | 0 RTT |
| Connection migration | 3 RTT | 2 RTT | 0 RTT | 0 RTT |

### 4.2 Head-of-Line Blocking

**TCP:**
```
Packet loss → All application streams blocked

HTTP/2 streams: [1] [2] [3] [4] [5]
TCP packets:    [A] [B] [C] [D] [E]
                      ❌
Stream 3,4,5 blocked even though their data arrived
```

**QUIC:**
```
Packet loss → Only affected stream blocked

QUIC streams: [1] [2] [3] [4] [5]
QUIC packets: [A] [B] [C] [D] [E]
    (stream 1) (stream 2) (stream 3)
        ❌
Stream 1 blocked, streams 2-5 continue normally
```

### 4.3 Protocol Headers

**TCP + TLS overhead:**
```
Ethernet (14) + IP (20) + TCP (20) = 54 bytes
+ TLS record header (5) = 59 bytes overhead
+ Most headers visible (sequence numbers, flags)
```

**QUIC overhead:**
```
Ethernet (14) + IP (20) + UDP (8) + QUIC (variable) = ~45 bytes
+ All headers encrypted (except first byte)
+ Connection ID instead of port-based routing
```

### 4.4 Error Codes

**TCP:** No application-level error codes. Reset is abrupt.

**QUIC:** Rich error codes:
```rust
pub enum Error {
    NoError = 0x0,
    InternalError = 0x1,
    ConnectionRefused = 0x2,
    FlowControlError = 0x3,
    StreamLimitError = 0x4,
    StreamStateError = 0x5,
    FinalSizeError = 0x6,
    FrameEncodingError = 0x7,
    TransportParameterError = 0x8,
    // ... and many more
}
```

---

## 5. HTTP/3 Motivation

### 5.1 Why HTTP/3?

HTTP/3 is HTTP semantics over QUIC instead of TCP:

```
HTTP/2 over TCP:
- Multiplexed streams at application layer
- Single TCP connection = single HOL blocking domain
- TCP doesn't understand HTTP semantics

HTTP/3 over QUIC:
- Multiplexed streams at transport layer
- Each stream independent
- QUIC understands stream priorities
```

### 5.2 Stream Types in HTTP/3

```
Bidirectional streams:
- Stream 0, 4, 8, ... (client-initiated)
- Stream 3, 7, 11, ... (server-initiated)
- Used for request/response pairs

Unidirectional streams:
- Type 0x00: Control stream (SETTINGS, GOAWAY)
- Type 0x01: Push stream (server push)
- Type 0x02: QPACK encoder stream
- Type 0x03: QPACK decoder stream
```

### 5.3 QPACK vs HPACK

**HTTP/2 uses HPACK:**
```
Dynamic table shared across all streams
Table update on stream 1 affects stream 100
HOL blocking for header compression state!
```

**HTTP/3 uses QPACK:**
```
Separate unidirectional streams for encoder/decoder
Encoder sends updates on encoder stream
Decoder sends acknowledgments on decoder stream
Request/response streams reference table state
No HOL blocking between header compression and data
```

---

## 6. First QUIC Connection

### 6.1 Minimal QUIC Client

```rust
use quiche;
use std::net::UdpSocket;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create configuration
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;

    // Configure ALPN (Application Layer Protocol Negotiation)
    config.set_application_protos(&[b"h3"])?;

    // Flow control limits
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);

    // For testing, skip peer verification
    config.verify_peer(false);

    // 2. Generate source connection ID
    let scid = quiche::ConnectionId::from_ref(&[0xba; 16]);

    // 3. Create client connection
    let peer_addr = "cloudflare-quic.com:443".parse()?;
    let local_addr = "0.0.0.0:0".parse()?;

    let mut conn = quiche::connect(
        Some("cloudflare-quic.com"),
        &scid,
        local_addr,
        peer_addr,
        &mut config,
    )?;

    // 4. Create UDP socket
    let socket = UdpSocket::bind(local_addr)?;
    socket.set_nonblocking(true)?;

    // 5. Generate initial packet
    let mut buf = vec![0; 1500];
    let (write, send_info) = conn.send(&mut buf)?;
    socket.send_to(&buf[..write], send_info.to)?;

    // 6. Main event loop
    let mut recv_buf = vec![0; 65535];

    loop {
        // Check if connection is established
        if conn.is_established() {
            println!("Connection established!");

            // Can now send/receive stream data
            if let Ok(stream_id) = conn.stream_send(0, b"GET / HTTP/3", true) {
                println!("Sent request on stream {}", stream_id);
            }
            break;
        }

        // Receive packets
        match socket.recv_from(&mut recv_buf) {
            Ok((read, peer)) => {
                let recv_info = quiche::RecvInfo {
                    from: peer,
                    to: local_addr,
                };
                conn.recv(&mut recv_buf[..read], recv_info)?;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No more packets, check timeout
                if let Some(timeout) = conn.timeout() {
                    std::thread::sleep(timeout);
                    conn.on_timeout();
                }
            }
            Err(e) => return Err(e.into()),
        }

        // Send any queued packets
        loop {
            match conn.send(&mut buf) {
                Ok((write, send_info)) => {
                    socket.send_to(&buf[..write], send_info.to)?;
                }
                Err(quiche::Error::Done) => break,
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(())
}
```

### 6.2 Connection States

```
Initial → Handshake → Established → Closed
                ↓
            Early (0-RTT possible)

State transitions:
1. Initial: ClientHello sent, waiting for response
2. Handshake: Processing server response, validating certificates
3. Established: Handshake complete, can send application data
4. Closed: Connection terminated (graceful or error)
```

### 6.3 Key QUIC APIs in quiche

**Configuration:**
```rust
let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;
config.set_application_protos(&[b"h3", b"http/0.9"])?;
config.set_max_idle_timeout(30_000);  // milliseconds
config.set_cc_algorithm(quiche::CongestionControlAlgorithm::CUBIC);
config.enable_early_data();  // Enable 0-RTT
```

**Connection operations:**
```rust
// Create connection
let conn = quiche::connect(server_name, &scid, local, peer, &mut config)?;

// Process incoming packet
conn.recv(&mut buf, recv_info)?;

// Generate outgoing packet
let (written, send_info) = conn.send(&mut out)?;

// Check state
if conn.is_established() { /* ready for data */ }
if conn.should_close() { /* time to cleanup */ }

// Timeout handling
if let Some(timeout) = conn.timeout() {
    std::thread::sleep(timeout);
    conn.on_timeout();
}
```

**Stream operations:**
```rust
// Send data
conn.stream_send(stream_id, b"hello", fin)?;

// Receive data
let (read, fin) = conn.stream_recv(stream_id, &mut buf)?;

// Check stream states
for stream_id in conn.readable() { /* has data */ }
for stream_id in conn.writable() { /* can write */ }

// Shutdown stream
conn.stream_shutdown(stream_id, Shutdown::Read, 0)?;
```

---

## Summary

### Key Takeaways

1. **TCP HOL blocking** is the fundamental problem - one lost packet blocks all application streams
2. **TLS adds latency** - 1-3 RTTs before any application data
3. **QUIC combines** transport + crypto into single 1-RTT handshake
4. **Connection IDs** enable migration without reconnection
5. **Stream multiplexing at transport layer** eliminates HOL blocking
6. **HTTP/3 + QPACK** removes header compression HOL blocking

### Next Steps

Continue to [01-quic-protocol-deep-dive.md](01-quic-protocol-deep-dive.md) for detailed coverage of:
- QUIC packet structure
- Frame types and encoding
- Connection ID management
- Stream states and flow control

---

## Further Reading

- [RFC 9000 - QUIC: A UDP-Based Multiplexed Transport](https://www.rfc-editor.org/rfc/rfc9000.html)
- [RFC 9001 - Using TLS to Secure QUIC](https://www.rfc-editor.org/rfc/rfc9001.html)
- [The Story of QUIC (Cloudflare blog)](https://blog.cloudflare.com/the-story-of-quic/)
- [HTTP/3 Explained (Daniel Stenberg)](https://http3-explained.haxx.se/)
