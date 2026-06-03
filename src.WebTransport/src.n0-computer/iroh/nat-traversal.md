# NAT Traversal in Iroh

## Overview

This document explores how iroh achieves connectivity across Network Address Translation (NAT) devices, ensuring reliable P2P connections even in challenging network environments.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.n0-computer/iroh/`

---

## Table of Contents

1. [Understanding NAT](#understanding-nat)
2. [NAT Types and Challenges](#nat-types-and-challenges)
3. [Iroh's Multi-Layer Approach](#irohs-multi-layer-approach)
4. [Relay Servers as Fallback](#relay-servers-as-fallback)
5. [UDP Hole Punching](#udp-hole-punching)
6. [Network Detection and Reporting](#network-detection-and-reporting)
7. [QUIC Multipath](#quic-multipath)
8. [Production NAT Traversal Patterns](#production-nat-traversal-patterns)

---

## Understanding NAT

### What is NAT?

Network Address Translation (NAT) allows multiple devices on a private network to share a single public IP address:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Internet                                  │
│                         │                                        │
│                   Public IP: 203.0.113.1                        │
│                         │                                        │
│                    ┌────▼────┐                                  │
│                    │   NAT   │  Router/Firewall                 │
│                    │ Gateway │                                   │
│                    └────┬────┘                                  │
│                         │                                        │
│    Private Network      │                                        │
│  ┌─────────┬───────────┼───────────┬─────────┐                  │
│  │         │           │           │         │                  │
│  ▼         ▼           ▼           ▼         ▼                  │
│ 192.168.1.10  192.168.1.11  192.168.1.12  ...                   │
│ (Device A)    (Device B)    (Device C)                          │
└─────────────────────────────────────────────────────────────────┘
```

### Why NAT Complicates P2P

```
Device A (Behind NAT)              Device B (Behind NAT)
   192.168.1.10:5000                   192.168.1.20:6000
         │                                   │
         │ Private address                   │ Private address
         │ Not routable on internet          │ Not routable on internet
         ▼                                   ▼
    ┌─────────┐                         ┌─────────┐
    │ NAT A   │                         │ NAT B   │
    │ 203.0.113.1:10000                 │ 198.51.100.1:20000
    └────┬────┘                         └────┬────┘
         │                                   │
         │ Public address                    │ Public address
         │ (May change per destination!)     │ (May change per destination!)
         ▼                                   ▼
    ─────────────────────────────────────────────
                     Internet

Problem: Device A doesn't know Device B's public address
         Device B doesn't know Device A's public address
         Direct packets may be blocked by NAT
```

---

## NAT Types and Challenges

### NAT Classifications

| NAT Type | Behavior | P2P Difficulty |
|----------|----------|----------------|
| **Full Cone** | Any external host can send packets to the mapped address | Easy |
| **Restricted Cone** | Only hosts that internal host contacted can send back | Moderate |
| **Port Restricted** | Same as Restricted, plus same external port required | Hard |
| **Symmetric** | Different external port for each destination | Very Hard |

### NAT Type Detection

```rust
// From iroh/src/net_report.rs

/// Results from NAT type detection
pub struct NatReport {
    /// Whether we have IPv4 connectivity
    pub have_v4: bool,

    /// Whether we have IPv6 connectivity
    pub have_v6: bool,

    /// Whether UDP works at all
    pub have_udp: bool,

    /// Our discovered public IPv4 (if any)
    pub ipv4: Option<String>,

    /// Our discovered public IPv6 (if any)
    pub ipv6: Option<String>,

    /// NAT type indication
    pub nat_type: Option<NatType>,
}

#[derive(Debug, Clone, Copy)]
pub enum NatType {
    FullCone,
    RestrictedCone,
    PortRestricted,
    Symmetric,
    Unknown,
}
```

### Detection via STUN

```rust
// STUN: Session Traversal Utilities for NAT
// RFC 5389

use std::net::{UdpSocket, SocketAddr};

/// STUN message types
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_RESPONSE: u16 = 0x0101;

/// STUN magic cookie (prevents older NATs from breaking STUN)
const MAGIC_COOKIE: u32 = 0x2112A442;

fn create_stun_request() -> Vec<u8> {
    let mut msg = vec![0u8; 20];

    // Message type: Binding Request
    msg[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());

    // Message length (0 for request)
    msg[2..4].copy_from_slice(&0u16.to_be_bytes());

    // Magic cookie
    msg[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());

    // Transaction ID (96 random bits)
    rand::rng().fill(&mut msg[8..20]);

    msg
}

/// Parse STUN binding response to get mapped address
fn parse_stun_response(data: &[u8]) -> Option<SocketAddr> {
    // Verify response type
    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != BINDING_RESPONSE {
        return None;
    }

    // Parse attributes
    let mut offset = 20;  // Skip 20-byte header
    while offset < data.len() {
        let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;

        // XOR-MAPPED-ADDRESS (0x0020) or MAPPED-ADDRESS (0x0001)
        if attr_type == 0x0020 || attr_type == 0x0001 {
            let family = data[offset];
            let port_xor = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);

            // XOR decoding with magic cookie
            let port = port_xor ^ ((MAGIC_COOKIE >> 16) as u16);

            if family == 0x01 {  // IPv4
                let mut addr = [0u8; 4];
                for i in 0..4 {
                    addr[i] = data[offset + 4 + i] ^ ((MAGIC_COOKIE >> (8 * i)) as u8);
                }
                return Some(SocketAddr::new(addr.into(), port));
            } else if family == 0x02 {  // IPv6
                let mut addr = [0u8; 16];
                for i in 0..16 {
                    let cookie_byte = match i % 4 {
                        0 => (MAGIC_COOKIE >> 24) as u8,
                        1 => (MAGIC_COOKIE >> 16) as u8,
                        2 => (MAGIC_COOKIE >> 8) as u8,
                        _ => MAGIC_COOKIE as u8,
                    };
                    addr[i] = data[offset + 4 + i] ^ cookie_byte;
                }
                return Some(SocketAddr::new(addr.into(), port));
            }
        }

        offset += attr_len;
    }

    None
}

/// Detect NAT type using multiple STUN servers
fn detect_nat_type() -> Result<NatType, NatDetectionError> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;

    // Query multiple STUN servers
    let stun_servers = [
        "stun.l.google.com:19302",
        "stun1.l.google.com:19302",
        "stun.stunprotocol.org:3478",
    ];

    let mut mapped_addrs = Vec::new();
    let mut dest_ports = Vec::new();

    for server in &stun_servers {
        let request = create_stun_request();
        socket.send_to(&request, server)?;

        let mut buf = vec![0u8; 512];
        if let Ok((len, _)) = socket.recv_from(&mut buf) {
            if let Some(addr) = parse_stun_response(&buf[..len]) {
                mapped_addrs.push(addr);
                dest_ports.push(server.split(':').last().unwrap().parse::<u16>().unwrap());
            }
        }
    }

    // Analyze results to determine NAT type
    analyze_nat_behavior(&mapped_addrs, &dest_ports)
}

fn analyze_nat_behavior(
    mapped_addrs: &[SocketAddr],
    dest_ports: &[u16],
) -> Option<NatType> {
    if mapped_addrs.is_empty() {
        return Some(NatType::Unknown);
    }

    // Check if all mapped addresses are the same
    let first_addr = mapped_addrs[0];
    let all_same_addr = mapped_addrs.iter().all(|&a| a.ip() == first_addr.ip());
    let all_same_port = mapped_addrs.iter().all(|&a| a.port() == first_addr.port());

    if all_same_addr && all_same_port {
        // Same external address:port for all destinations
        // Could be Full Cone, Restricted Cone, or Port Restricted
        // (Need additional tests to distinguish)
        Some(NatType::FullCone)  // Optimistic assumption
    } else if all_same_addr {
        // Same IP but different ports
        Some(NatType::Symmetric)
    } else {
        // Different IPs - very unusual, likely symmetric with multiple public IPs
        Some(NatType::Symmetric)
    }
}
```

---

## Iroh's Multi-Layer Approach

### Connection Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│            Iroh NAT Traversal Strategy                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Layer 1: Direct Connection (Fastest)                           │
│  ├── Try known direct addresses first                           │
│  └── Works for: Same network, public IPs, port-forwarded       │
│                                                                  │
│  Layer 2: UDP Hole Punching (Fast)                              │
│  ├── Coordinate via relay to punch holes simultaneously         │
│  ├── Works for: Full cone, restricted cone NATs                 │
│  └── Success rate: ~80-90%                                      │
│                                                                  │
│  Layer 3: Relay Fallback (Guaranteed)                           │
│  ├── Route traffic via relay server                             │
│  ├── Works for: All NAT types, symmetric NATs                   │
│  └── Success rate: ~100% (if relay reachable)                   │
│                                                                  │
│  Layer 4: Multipath QUIC (Optimization)                         │
│  ├── Use multiple paths simultaneously                          │
│  └── Automatically switch to best path                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Path Selection Algorithm

```rust
use std::collections::HashMap;

struct PathSelector {
    paths: HashMap<PathId, PathStats>,
    current_best: Option<PathId>,
}

struct PathStats {
    path_type: PathType,
    latency: Duration,
    success_rate: f64,
    last_used: Instant,
    bytes_sent: u64,
    bytes_received: u64,
}

enum PathType {
    Direct,      // Direct UDP
    HolePunched, // Successful hole punch
    Relay,       // Via relay server
}

impl PathSelector {
    /// Select the best path based on metrics
    fn select_best_path(&mut self) -> Option<PathId> {
        let mut best_score = 0.0;
        let mut best_path = None;

        for (path_id, stats) in &self.paths {
            // Score calculation:
            // - Direct paths get bonus
            // - Lower latency = higher score
            // - Higher success rate = higher score
            // - Recently used paths get slight preference

            let latency_score = 1.0 / (1.0 + stats.latency.as_secs_f64());
            let type_bonus = match stats.path_type {
                PathType::Direct => 0.3,
                PathType::HolePunched => 0.2,
                PathType::Relay => 0.0,
            };

            let score = (stats.success_rate * 0.5) + (latency_score * 0.3) + type_bonus;

            if score > best_score {
                best_score = score;
                best_path = Some(*path_id);
            }
        }

        self.current_best = best_path;
        best_path
    }

    /// Update path statistics based on results
    fn record_result(&mut self, path_id: PathId, success: bool, latency: Duration) {
        if let Some(stats) = self.paths.get_mut(&path_id) {
            // Exponential moving average for success rate
            stats.success_rate = if success {
                0.9 * stats.success_rate + 0.1
            } else {
                0.9 * stats.success_rate
            };

            // Update latency (smoothed)
            stats.latency = Duration::from_secs_f64(
                0.8 * stats.latency.as_secs_f64() + 0.2 * latency.as_secs_f64()
            );

            stats.last_used = Instant::now();
        }
    }
}
```

---

## Relay Servers as Fallback

### Why Relays Work When Direct Fails

```
Direct Connection (Fails with Symmetric NAT):
Device A                          Device B
  │                                  │
  │ Send to B's IP                   │ Send to A's IP
  │ ↓                                │ ↓
┌─┴──────────────┐              ┌────┴─────────────┐
│   NAT A        │              │    NAT B         │
│ (Symmetric)    │              │  (Symmetric)     │
│ External:      │              │ External:        │
│ 203.0.113.1:   │              │ 198.51.100.1:    │
│ DIFFERENT      │              │ DIFFERENT        │
│ port for B!    │              │ port for A!      │
└────────────────┘              └──────────────────┘
         │                              │
         └────────── ✗ ─────────────────┘
              Packets dropped
              (no NAT mapping)


Relay Connection (Always Works):
Device A                          Device B
  │                                  │
  │ Connect to relay                 │ Connect to relay
  │ ↓                                │ ↓
┌─┴──────────────┐              ┌────┴─────────────┐
│   NAT A        │    ┌─────┐   │    NAT B         │
│ (Symmetric)    │◄───┤Relay├───┤  (Symmetric)     │
│                │    └─────┘   │                  │
│ Outbound OK →  │      ↑       │ → Outbound OK    │
└────────────────┘      │       └──────────────────┘
                        │
                        ▼
              Relay forwards packets
              between A and B
```

### Relay Connection Flow

```rust
// From iroh/src/socket/transports/relay/actor.rs

/// Relay connection handling
struct RelayActor {
    /// The relay server URL
    relay_url: RelayUrl,

    /// Connection to relay (WebSocket over HTTPS)
    relay_conn: Option<RelayConnection>,

    /// Our node ID
    node_id: EndpointId,

    /// Secret key for authentication
    secret_key: SecretKey,

    /// Clients connected through this relay
    client_channels: HashMap<EndpointId, mpsc::Sender<Bytes>>,
}

impl RelayActor {
    /// Connect to relay server
    async fn connect_relay(&mut self) -> Result<(), RelayError> {
        // Establish WebSocket connection
        let url = format!("wss://{}/conn", self.relay_url);
        let (ws_stream, _) = connect_async(url).await?;

        // Perform authentication handshake
        let conn = RelayConnection::new(
            ws_stream,
            &self.secret_key,
        ).await?;

        self.relay_conn = Some(conn);
        Ok(())
    }

    /// Send data to a peer via relay
    async fn send_via_relay(
        &mut self,
        recipient: EndpointId,
        data: Bytes,
    ) -> Result<(), RelaySendError> {
        let conn = self.relay_conn
            .as_mut()
            .ok_or(RelaySendError::NotConnected)?;

        // Send datagram message to relay
        conn.send(ClientToRelayMsg::Datagrams {
            dst_endpoint_id: recipient,
            datagrams: data.into(),
        }).await?;

        Ok(())
    }

    /// Receive data from relay
    async fn recv_from_relay(&mut self) -> Option<(EndpointId, Bytes)> {
        let conn = self.relay_conn.as_mut()?;

        while let Some(msg) = conn.next().await {
            match msg.ok()? {
                RelayToClientMsg::Datagrams { remote_endpoint_id, datagrams } => {
                    return Some((remote_endpoint_id, datagrams.contents));
                }
                RelayToClientMsg::EndpointGone(id) => {
                    self.client_channels.remove(&id);
                }
                _ => {}
            }
        }

        None
    }
}
```

---

## UDP Hole Punching

### Hole Punching Process

```rust
// Simplified hole punching implementation

use std::net::{UdpSocket, SocketAddr};
use std::time::Duration;

/// Coordinate hole punch between two peers
struct HolePunchCoordinator {
    socket: UdpSocket,
    relay: RelayConnection,
}

impl HolePunchCoordinator {
    /// Execute hole punch with a peer
    async fn punch_hole(
        &mut self,
        peer_id: EndpointId,
        known_addrs: Vec<SocketAddr>,
    ) -> Result<SocketAddr, HolePunchError> {
        // Phase 1: Tell relay we want to connect to peer
        self.relay.send_start_punch(peer_id).await?;

        // Phase 2: Wait for peer to also signal interest
        let peer_external = self.relay.wait_for_peer_ready(peer_id).await?;

        // Phase 3: Send packets to all known addresses
        // This creates NAT mappings
        let punch_packet = self.create_punch_packet(peer_id);

        for addr in &known_addrs {
            let _ = self.socket.send_to(&punch_packet, addr);
        }

        // Also send to peer's external address (from relay)
        if let Some(ext_addr) = peer_external {
            let _ = self.socket.send_to(&punch_packet, ext_addr);
        }

        // Phase 4: Listen for response
        // The first response gives us the working path
        self.socket.set_read_timeout(Some(Duration::from_millis(100)))?;

        let mut buf = vec![0u8; 1500];
        for attempt in 0..50 {  // 5 seconds total
            match self.socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    if self.verify_punch_response(&buf[..len], peer_id) {
                        // Success! NAT hole punched
                        return Ok(addr);
                    }
                }
                Err(_) => {
                    // Timeout, try again
                    // Re-send punch packets periodically
                    if attempt % 5 == 0 {
                        for addr in &known_addrs {
                            let _ = self.socket.send_to(&punch_packet, addr);
                        }
                    }
                }
            }
        }

        Err(HolePunchError::Timeout)
    }

    fn create_punch_packet(&self, peer_id: EndpointId) -> Vec<u8> {
        // Create a packet that:
        // 1. Is small (fits in one UDP datagram)
        // 2. Contains enough info to identify the sender
        // 3. Can be verified as legitimate punch attempt

        let mut packet = Vec::with_capacity(64);
        packet.extend_from_slice(b"IROH-PUNCH");
        packet.extend_from_slice(self.node_id.as_bytes());
        packet.extend_from_slice(&peer_id.as_bytes());

        packet
    }

    fn verify_punch_response(&self, data: &[u8], expected_peer: EndpointId) -> bool {
        if data.len() < 42 {  // Minimum: 10 (header) + 32 (peer ID)
            return false;
        }

        if &data[0..10] != b"IROH-PUNCH" {
            return false;
        }

        let peer_bytes = &data[10..42];
        peer_bytes == expected_peer.as_bytes()
    }
}
```

### Simultaneous Open

```
The key to hole punching: BOTH sides must send packets at the same time

Time →
  │
  │  Side A                    Side B
  │    │                          │
  │    │─── "Ready to punch" ────▶│ (via relay)
  │    │◄── "Ready to punch" ─────│ (via relay)
  │    │                          │
  │    │  [NAT A creates mapping] │  [NAT B creates mapping]
  │    │                          │
  │    │─── Punch packet ────────▶│
  │    │◄── Punch packet ─────────│
  │    │    (cross in network)    │
  │    │                          │
  │    │    Both NATs now have    │
  │    │    the mapping created!  │
  │    │                          │
  │    │◄── Direct connection ───▶│
  │    │      ESTABLISHED!        │
  │
```

---

## Network Detection and Reporting

### NetReport System

```rust
// From iroh/src/net_report.rs

/// Comprehensive network condition report
#[derive(Debug, Clone)]
pub struct Report {
    /// Whether IPv4 is available
    pub ipv4: bool,

    /// Whether IPv6 is available
    pub ipv6: bool,

    /// Whether UDP works
    pub udp: bool,

    /// Our public IPv4 address (if discovered)
    pub ipv4_addr: Option<String>,

    /// Our public IPv6 address (if discovered)
    pub ipv6_addr: Option<String>,

    /// Preferred relay server
    pub preferred_relay: Option<RelayUrl>,

    /// Latency to each relay
    pub relay_latencies: HashMap<RelayUrl, Duration>,

    /// Whether we're behind a captive portal
    pub captive_portal: Option<bool>,

    /// Time this report was generated
    pub when: Instant,
}

impl Report {
    /// Check if we have direct UDP connectivity
    pub fn has_udp(&self) -> bool {
        self.udp && (self.ipv4 || self.ipv6)
    }

    /// Get the best available connection method
    pub fn best_method(&self) -> ConnectionMethod {
        if self.ipv6 {
            ConnectionMethod::DirectIpv6
        } else if self.ipv4 {
            ConnectionMethod::DirectIpv4
        } else if let Some(_) = self.preferred_relay {
            ConnectionMethod::Relay
        } else {
            ConnectionMethod::Unknown
        }
    }
}

enum ConnectionMethod {
    DirectIpv4,
    DirectIpv6,
    Relay,
    Unknown,
}
```

### Probe System

```rust
// Network probing to determine connectivity

struct ProbePlan {
    probes: Vec<Probe>,
}

enum Probe {
    /// Send UDP packet to STUN server
    StunIpv4 { server: SocketAddr },

    /// Send UDP packet to STUN server (IPv6)
    StunIpv6 { server: SocketAddr },

    /// HTTPS request to relay (tests relay connectivity)
    RelayHttps { url: RelayUrl },

    /// QUIC connection to relay (tests QUIC over UDP)
    RelayQuic { url: RelayUrl },

    /// ICMP ping to gateway
    IcmpGateway { gateway: IpAddr },
}

struct ProbeReport {
    probe: Probe,
    success: bool,
    duration: Option<Duration>,
    error: Option<String>,
    discovered_addr: Option<SocketAddr>,
}

/// Run probes and generate report
async fn run_probes(plan: ProbePlan) -> Report {
    let mut report = Report::default();
    let mut join_set = JoinSet::new();

    // Run probes in parallel
    for probe in plan.probes {
        join_set.spawn(run_single_probe(probe));
    }

    // Collect results
    while let Some(result) = join_set.join_next().await {
        if let Ok(probe_report) = result {
            update_report_from_probe(&mut report, probe_report);
        }
    }

    report
}

async fn run_single_probe(probe: Probe) -> ProbeReport {
    match probe {
        Probe::StunIpv4 { server } => {
            probe_stun_ipv4(server).await
        }
        Probe::RelayHttps { url } => {
            probe_relay_https(url).await
        }
        // ... other probes
    }
}
```

---

## QUIC Multipath

### Using Multiple Paths Simultaneously

```rust
// From iroh/src/endpoint/quic.rs

/// QUIC multipath allows using multiple network paths
/// for a single connection

struct MultipathConnection {
    /// Primary path (usually direct)
    primary: PathId,

    /// All available paths
    paths: HashMap<PathId, PathState>,

    /// Connection handle
    quic_conn: noq::Connection,
}

struct PathState {
    path_id: PathId,
    addr: SocketAddr,
    path_type: PathType,
    is_active: bool,
    rtt: Duration,
    mtu: u16,
}

impl MultipathConnection {
    /// Add a new path to the connection
    async fn add_path(&mut self, addr: SocketAddr) -> Result<PathId, AddPathError> {
        // Send PATH_CHALLENGE frame
        let path_id = self.quic_conn.add_path(addr).await?;

        self.paths.insert(path_id, PathState {
            path_id,
            addr,
            path_type: PathType::Direct,
            is_active: false,
            rtt: Duration::ZERO,
            mtu: 1500,
        });

        Ok(path_id)
    }

    /// Monitor path quality and switch if needed
    fn monitor_paths(&mut self) {
        let mut best_rtt = Duration::MAX;
        let mut best_path = self.primary;

        for (path_id, state) in &mut self.paths {
            // Update RTT measurement
            if let Some(rtt) = self.quic_conn.rtt(*path_id) {
                state.rtt = rtt;
                state.is_active = true;

                if rtt < best_rtt {
                    best_rtt = rtt;
                    best_path = *path_id;
                }
            } else {
                state.is_active = false;
            }
        }

        // Switch to better path if available
        if best_path != self.primary && self.paths.get(&best_path).map(|p| p.is_active).unwrap_or(false) {
            self.primary = best_path;
        }
    }
}
```

### Path Migration

```rust
/// Handle network changes (e.g., WiFi to cellular)

async fn handle_network_change(
    conn: &mut MultipathConnection,
    new_interface: NetworkInterface,
) {
    // Get address on new interface
    let new_addr = get_address_for_interface(&new_interface);

    // Add new path
    match conn.add_path(new_addr).await {
        Ok(path_id) => {
            println!("Added new path via {}: {:?}", new_interface.name, path_id);

            // Let the path stabilize
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Check if new path is better
            conn.monitor_paths();
        }
        Err(e) => {
            eprintln!("Failed to add path: {}", e);
        }
    }

    // Optionally remove old paths after timeout
    // (keep as backup in case new path fails)
}
```

---

## Production NAT Traversal Patterns

### Pattern 1: Always Have a Relay

```rust
use iroh::{Endpoint, endpoint::presets, RelayMode};

async fn create_resilient_endpoint() -> Result<Endpoint, Error> {
    // Always configure at least one relay
    // Even if you expect direct connections to work

    let endpoint = Endpoint::builder(presets::N0)
        .relay_mode(RelayMode::Default)  // Use n0's default relays
        .bind()
        .await?;

    // This ensures:
    // - 100% connectivity (relay always works)
    // - Better NAT traversal (relay assists hole punching)
    // - Fallback if direct fails

    Ok(endpoint)
}
```

### Pattern 2: Pre-Connect Network Check

```rust
async fn check_connectivity(endpoint: &Endpoint) -> ConnectivityReport {
    // Get current network report
    let net_report = endpoint.net_report().await;

    let mut report = ConnectivityReport {
        can_connect: true,
        methods: Vec::new(),
        warnings: Vec::new(),
    };

    if net_report.ipv4 && net_report.udp {
        report.methods.push("Direct IPv4");
    }
    if net_report.ipv6 && net_report.udp {
        report.methods.push("Direct IPv6");
    }
    if net_report.preferred_relay.is_some() {
        report.methods.push("Relay");
    }

    // Warn about potential issues
    if !net_report.udp {
        report.warnings.push("UDP blocked - only relay available");
        report.can_connect = net_report.preferred_relay.is_some();
    }

    if net_report.captive_portal == Some(true) {
        report.warnings.push("Behind captive portal");
    }

    report
}
```

### Pattern 3: Graceful Connection with Fallback

```rust
async fn connect_with_graceful_fallback(
    endpoint: &Endpoint,
    peer_id: EndpointId,
    alpn: &[u8],
) -> Result<Connection, ConnectionError> {
    use tokio::time::{timeout, Duration};

    // Try direct connection with timeout
    let direct_result = timeout(
        Duration::from_secs(3),
        endpoint.connect(peer_id, alpn)
    ).await;

    match direct_result {
        Ok(Ok(conn)) => {
            println!("Connected directly to peer");
            return Ok(conn);
        }
        Ok(Err(e)) => {
            eprintln!("Direct connection failed: {}", e);
            // Fall through to relay
        }
        Err(_) => {
            eprintln!("Direct connection timed out");
            // Fall through to relay
        }
    }

    // Try via relay (longer timeout, more reliable)
    println!("Attempting relay connection...");
    let relay_result = timeout(
        Duration::from_secs(10),
        endpoint.connect(peer_id, alpn)
    ).await;

    match relay_result {
        Ok(Ok(conn)) => {
            println!("Connected via relay");
            return Ok(conn);
        }
        _ => Err(ConnectionError::AllMethodsFailed),
    }
}
```

### Pattern 4: Monitor Connection Quality

```rust
use tokio::time::interval;

async fn monitor_connection_quality(conn: &Connection) {
    let mut check_interval = interval(Duration::from_secs(5));

    loop {
        check_interval.tick().await;

        let stats = conn.stats();

        // Check for degradation
        if stats.rtt > Duration::from_millis(500) {
            println!("High latency detected: {:?}", stats.rtt);
        }

        if stats.lost_packets > 100 {
            println!("Packet loss detected: {} packets", stats.lost_packets);
        }

        // Consider switching path or reconnecting
        if stats.rtt > Duration::from_secs(2) {
            println!("Connection severely degraded, consider reconnect");
        }
    }
}
```

### Pattern 5: Multi-Relay Redundancy

```rust
use iroh_relay::{RelayMap, RelayConfig};

async fn setup_multi_relay() -> Result<Endpoint, Error> {
    // Configure multiple relays for redundancy
    let relay_map = RelayMap::try_from_iter([
        "https://relay1.example.com",
        "https://relay2.example.com",
        "https://relay3.example.com",
    ])?;

    let endpoint = Endpoint::builder()
        .relay_map(relay_map)
        .bind()
        .await?;

    // Iroh will:
    // - Test latency to all relays
    // - Use the fastest one by default
    // - Fail over to others if needed

    Ok(endpoint)
}
```

### Pattern 6: LAN Optimization

```rust
#[cfg(feature = "address-lookup-mdns")]
async fn setup_lan_optimized() -> Result<Endpoint, Error> {
    use iroh::address_lookup;

    let endpoint = Endpoint::builder(presets::Minimal)
        // Enable mDNS for local discovery
        .address_lookup(address_lookup::MdnsAddressLookup::builder())
        // Still have relay for non-local
        .relay_mode(RelayMode::Default)
        .bind()
        .await?;

    // This setup:
    // - Uses direct LAN connections when available (lowest latency)
    // - Falls back to relay for remote peers

    Ok(endpoint)
}
```

---

## NAT Traversal Success Rates

| Scenario | Direct | Hole Punch | Relay |
|----------|--------|------------|-------|
| Same LAN | ~100% | N/A | ~100% |
| Different networks, full cone NAT | ~80% | ~95% | ~100% |
| Restricted cone NAT | ~50% | ~85% | ~100% |
| Port restricted NAT | ~30% | ~75% | ~100% |
| Symmetric NAT | ~5% | ~40% | ~100% |
| Enterprise firewall | ~0% | ~10% | ~90%* |

*Relay may be blocked by strict firewalls

---

## See Also

- [exploration.md](./exploration.md) - Main iroh exploration
- [p2p-ground-up.md](./p2p-ground-up.md) - Building P2P from scratch
- [ewe-platform-integration.md](./ewe-platform-integration.md) - Integration guide
