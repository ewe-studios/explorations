---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/Others/quiche
repository: https://github.com/cloudflare/quiche
revised_at: 2026-03-19
---

# Quiche Deep Dive: QUIC and HTTP/3 Implementation

## Overview

Quiche is Cloudflare's implementation of the QUIC transport protocol and HTTP/3. It provides a low-level API for building QUIC-based applications, handling packet construction, congestion control, and stream multiplexing.

## Protocol Support

| Protocol | RFC | Status |
|----------|-----|--------|
| QUIC | RFC 9000 | Full support |
| HTTP/3 | RFC 9114 | Full support |
| DATASET | - | Extension support |
| QPACK | RFC 9204 | Full support |

## Workspace Structure

```
quiche/
├── quiche/                  # Core QUIC implementation
│   ├── src/
│   │   ├── lib.rs           # Public API, Connection, Config
│   │   ├── packet.rs        # Packet parsing and construction
│   │   ├── crypto.rs        # TLS integration (BoringSSL/OpenSSL)
│   │   ├── congestion.rs    # BBR, Cubic, Reno algorithms
│   │   ├── frame.rs         # QUIC frame types
│   │   ├── stream.rs        # Stream management
│   │   ├── ranges.rs        # Range tracking for ACKs
│   │   ├── ffi.rs           # C FFI bindings
│   │   └── ...
│   └── Cargo.toml
├── qlog/                    # QLOG logging for debugging
├── octets/                  # Octet parsing utilities
├── h3i/                     # HTTP/3 integration tests
├── apps/                    # Example applications (quiche-client, quiche-server)
├── tools/http3_test/        # HTTP/3 conformance tests
└── fuzz/                    # Fuzzing harnesses
```

## Core API Design

### Connection Object

The `Connection` struct is the central abstraction:

```rust
pub struct Connection {
    // Configuration
    version: u32,
    is_server: bool,

    // Connection IDs
    odcid: ConnectionId,
    peer_cid: ConnectionId,
    local_cid_set: LocCidSet,

    // Packet number management
    pkt_num_spaces: [PacketNumSpace; Epoch::count()],
    next_pkt_num: [u64; Epoch::count()],

    // Crypto states per epoch
    crypto_streams: [CryptoStream; Epoch::count()],
    crypto_sealers: [Option<crypto::OpenSealer>; Epoch::count()],
    crypto_openers: [Option<crypto::OpenSealer>; Epoch::count()],

    // Stream management
    streams: StreamMap,
    local_max_streams_bidi: u64,
    local_max_streams_uni: u64,

    // Congestion control
    recovery: Recovery,

    // Path management
    paths: PathManager,

    // Handshake state
    handshake_status: HandshakeStatus,
}
```

### Configuration Builder

```rust
pub struct Config {
    version: u32,
    max_datagram_size: usize,
    local_transport_params: TransportParams,
    grease: bool,
    max_frame_size: u64,
    // ...
}

impl Config {
    pub fn new(version: u32) -> Result<Self>;
    pub fn load_cert_chain_from_pem_file(
        &mut self,
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<()>;
    pub fn set_application_protos(&mut self, protos: &[u8]) -> Result<()>;
    pub fn set_initial_max_data(&mut self, v: u64);
    pub fn set_initial_max_stream_data_bidi_local(&mut self, v: u64);
    pub fn set_initial_max_stream_data_bidi_remote(&mut self, v: u64);
    pub fn set_initial_max_stream_data_uni(&mut self, v: u64);
    pub fn set_initial_max_streams_bidi(&mut self, v: u64);
    pub fn set_initial_max_streams_uni(&mut self, v: u64);
}
```

### Connection Creation

```rust
// Client-side connection
pub fn connect(
    config: &Config,
    scid: &ConnectionId,
    dcid: &ConnectionId,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
) -> Result<Connection>;

// Server-side connection
pub fn accept(
    config: &Config,
    scid: &ConnectionId,
    dcid: &ConnectionId,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
) -> Result<Connection>;
```

## Packet Layer Architecture

### Epoch-Based Key Management

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Epoch {
    Initial     = 0,  // Initial packets, TLS handshake start
    Handshake   = 1,  // Handshake packets, TLS handshake completion
    Application = 2,  // 1-RTT packets, application data
}

static EPOCHS: [Epoch; 3] = [Epoch::Initial, Epoch::Handshake, Epoch::Application];
```

Each epoch has independent:
- Packet number space
- Crypto keys (sealer/opener)
- ACK tracking
- Loss detection state

### Packet Types

```rust
pub enum Type {
    Initial,        // Version negotiation, first client hello
    Retry,          // Server retry with new connection ID
    Handshake,      // TLS handshake messages
    ZeroRTT,        // Early data (0-RTT)
    VersionNegotiation,  // Version negotiation response
    Short,          // 1-RTT encrypted packets
}

impl Type {
    pub(crate) fn from_epoch(e: Epoch) -> Type {
        match e {
            Epoch::Initial => Type::Initial,
            Epoch::Handshake => Type::Handshake,
            Epoch::Application => Type::Short,
        }
    }
}
```

### Packet Header Structure

```rust
// First byte structure
const FORM_BIT: u8 = 0x80;    // Fixed bit (1 for long header)
const FIXED_BIT: u8 = 0x40;   // Must be 1
const KEY_PHASE_BIT: u8 = 0x04;  // Key phase for 1-RTT packets

const TYPE_MASK: u8 = 0x30;   // Packet type mask
const PKT_NUM_MASK: u8 = 0x03; // Packet number length mask
```

**Long Header Format (Initial, Handshake, 0-RTT):**
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|1|  Long Header (Type) |  Ver  |  DCID Len   | Destination CID |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|               Destination CID (cont)         |  SCID Len       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|            Source CID (cont)               |  Payload Length |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Short Header Format (1-RTT):**
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|0|1|C|K|  Type  | Protected Header ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### Packet Number Encoding

```rust
pub const MAX_PKT_NUM_LEN: usize = 4;

// Packet number length encoded in first byte
fn pkt_num_len(first: u8) -> usize {
    (first & PKT_NUM_MASK) as usize + 1
}
```

Packet numbers are encoded in 1-4 bytes and used for:
- ACK generation
- Loss detection
- Reorder detection

## Cryptographic Integration

### TLS Integration

Quiche integrates with BoringSSL or OpenSSL for TLS 1.3:

```rust
// Crypto initialization
pub(crate) struct Crypto {
    tls: Ssl,  // BoringSSL/OpenSSL SSL object
    alert: Option<u8>,
    handshake_completed: bool,
}

impl Crypto {
    pub fn new(is_server: bool, config: &Config) -> Result<Self>;

    pub fn do_handshake(&mut self) -> Result<()>;
    pub fn process_post_handshake(&mut self) -> Result<()>;
    pub fn write_handshake(&mut self, buf: &mut [u8]) -> Result<(Epoch, usize)>;
}
```

### AEAD Operations

```rust
use ring::aead;

// Packet encryption
fn encrypt_packet(
    pn: u64,
    key: &aead::LessSafeKey,
    header: &mut [u8],
    payload: &mut [u8],
    aad: &[u8],
) -> Result<()> {
    let nonce = compute_nonce(pn);
    let tag = key.seal_in_place_separate_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::from(aad),
        payload,
    )?;

    // Append tag to payload
    payload[..tag.len()].copy_from_slice(tag.as_ref());
    Ok(())
}
```

### Header Protection

QUIC uses header protection to encrypt packet type and packet number:

```rust
fn apply_header_protection(
    key: &aead::LessSafeKey,
    first_byte: &mut u8,
    packet_number: &[u8],
) {
    // Sample from packet to derive mask
    let sample = &packet[packet_number_start..packet_number_start + 16];

    // Encrypt sample to get mask
    let mask = compute_mask(key, sample);

    // XOR first byte and packet number with mask
    *first_byte ^= mask[0];
    for (i, pn_byte) in packet_number.iter_mut().enumerate() {
        *pn_byte ^= mask[i + 1];
    }
}
```

## Congestion Control

### Pluggable Congestion Control

```rust
pub trait CongestionControlOps: Debug + Send {
    fn on_packet_sent(
        &self,
        rtt_stats: &RttStats,
        congestion: &mut Congestion,
        packet: SentPacket,
    );

    fn on_ack_received(
        &self,
        rtt_stats: &RttStats,
        congestion: &mut Congestion,
        acked_packet: SentPacket,
        ack_delay: Duration,
    );

    fn on_loss_detected(
        &self,
        congestion: &mut Congestion,
        lost_bytes: usize,
    );
}
```

### BBR Implementation

```rust
#[derive(Debug)]
pub struct BbrCongestionController {
    min_rtt: Duration,
    max_bw: Rate,
    pacing_gain: f64,
    cwnd_gain: f64,

    // BBR states
    state: BbrState,
    round_start: bool,
    round_count: u64,

    // Probe phases
    probe_rtt_done: bool,
    cycle_index: usize,
}

// BBR State Machine
enum BbrState {
    Startup,      // Exponential growth
    Drain,        // Drain queue
    ProbeBw,      // Probe bandwidth
    ProbeRtt,     // Probe RTT
}
```

### Cubic Implementation

```rust
#[derive(Debug)]
pub struct CubicCongestionController {
    // CUBIC state
    w_max: f64,
    k: f64,
    w_last_max: f64,

    // Cwnd
    cwnd: usize,
    ssthresh: usize,

    // RTT tracking
    min_rtt: Duration,
    latest_rtt: Duration,
}

// CUBIC function: W(t) = C * (t - K)^3 + W_max
fn cubic_function(t: Duration, k: f64, w_max: f64) -> f64 {
    const C: f64 = 0.4;
    C * (t.as_secs_f64() - k).powi(3) + w_max
}
```

## Stream Management

### Stream Types

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamType {
    Bidirectional,
    Unidirectional,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamSide {
    Local,
    Remote,
}
```

### Stream States

```rust
// Sending side states
enum SendStreamState {
    Ready,      // Stream created, no data sent
    Send,       // Sending data
    DataSent,   // All data sent, waiting for ACK
    DataRead,   // ACK received, stream complete
    ResetSent,  // Reset sent
    ResetRead,  // Reset acknowledged
}

// Receiving side states
enum RecvStreamState {
    Ready,      // Stream created
    Recv,       // Receiving data
    SizeKnown,  // Final size known
    DataRead,   // All data read
    ResetRecvd, // Reset received
}
```

### Stream Multiplexing

```rust
pub struct StreamMap {
    // Local streams
    local_bidi_next: u64,
    local_uni_next: u64,

    // Remote streams
    remote_bidi_max: u64,
    remote_uni_max: u64,

    // Stream data
    streams: HashMap<u64, Stream>,
}

pub struct Stream {
    send: SendStream,
    recv: RecvStream,
}

pub struct SendStream {
    state: SendStreamState,
    data: VecDeque<Vec<u8>>,
    offset: u64,
    max_offset: u64,
}

pub struct RecvStream {
    state: RecvStreamState,
    data: RangeMap,  // Out-of-order data
    offset: u64,
    max_offset: u64,
}
```

## Frame Types

```rust
pub enum Frame {
    Padding { len: usize },
    Ping,
    Ack {
        largest: u64,
        ack_delay: u64,
        ranges: Vec<u64>,
        ecn_counts: Option<EcnCounts>,
    },
    ResetStream {
        stream_id: u64,
        error_code: u64,
        final_size: u64,
    },
    StopSending {
        stream_id: u64,
        error_code: u64,
    },
    Crypto {
        data: Range<u64>,
    },
    NewToken { token: Vec<u8> },
    Stream {
        stream_id: u64,
        data: Vec<u8>,
        offset: u64,
        fin: bool,
    },
    MaxData(u64),
    MaxStreamData { stream_id: u64, max: u64 },
    MaxStreams { bidi: bool, max: u64 },
    DataBlocked { limit: u64 },
    StreamDataBlocked { stream_id: u64, limit: u64 },
    StreamsBlocked { bidi: bool, limit: u64 },
    NewConnectionId {
        sequence: u64,
        retire: u64,
        cid: ConnectionId,
        reset_token: [u8; 16],
    },
    RetireConnectionId { sequence: u64 },
    PathChallenge { data: [u8; 8] },
    PathResponse { data: [u8; 8] },
    ConnectionClose {
        error_code: u64,
        frame_type: Option<u64>,
        reason: Vec<u8>,
    },
    HandshakeDone,
    Datagram { data: Vec<u8> },
}
```

## Loss Detection and Recovery

### Packet Number Spaces

```rust
pub struct PacketNumSpace {
    // ACK tracking
    ack_eliciting_sent: usize,
    time_sent: Option<Instant>,

    // Loss detection
    largest_acked: Option<u64>,
    largest_acked_sent: Option<Instant>,

    // ACK generation
    recv_pkt_num: RangeSet,
    ack_delay: Duration,
}
```

### Loss Detection

```rust
const PACKET_THRESHOLD: u64 = 3;
const TIME_THRESHOLD: Duration = Duration::from_millis(333);

fn detect_lost_packets(
    pkt_num_space: &PacketNumSpace,
    rtt_stats: &RttStats,
) -> Vec<SentPacket> {
    let mut lost = Vec::new();

    let time_threshold = now - rtt_stats.smoothed_rtt() * 9 / 8;
    let packet_threshold = largest_acked - PACKET_THRESHOLD;

    for sent in sent_packets {
        if sent.pkt_num < largest_acked &&
           (sent.time_sent <= time_threshold || sent.pkt_num <= packet_threshold) {
            lost.push(sent);
        }
    }

    lost
}
```

### RTT Estimation

```rust
pub struct RttStats {
    latest_rtt: Duration,
    smoothed_rtt: Option<Duration>,
    rttvar: Duration,
    min_rtt: Duration,
    max_ack_delay: Duration,
}

impl RttStats {
    pub fn update_rtt(
        &mut self,
        ack_delay: Duration,
        ack_time: Instant,
        sent_time: Instant,
    ) {
        self.latest_rtt = ack_time - sent_time;

        // Subtract ack delay if present
        if let Some(smoothed) = self.smoothed_rtt {
            let ack_delay = min(ack_delay, self.max_ack_delay);
            let adjusted_rtt = self.latest_rtt - ack_delay;

            // Update min RTT
            self.min_rtt = min(self.min_rtt, adjusted_rtt);

            // Update smoothed RTT (EWMA)
            let rtt_variance = (smoothed - adjusted_rtt).abs();
            self.rttvar = self.rttvar * 3 / 4 + rtt_variance / 4;
            self.smoothed_rtt = Some(smoothed * 7 / 8 + adjusted_rtt / 8);
        } else {
            // First RTT sample
            self.smoothed_rtt = Some(self.latest_rtt);
            self.min_rtt = self.latest_rtt;
        }
    }

    pub fn pto(&self) -> Duration {
        self.smoothed_rtt.unwrap_or(self.latest_rtt)
            + max(4 * self.rttvar, Duration::from_millis(1))
            + self.max_ack_delay
    }
}
```

## HTTP/3 Layer (h3 crate)

### Connection Setup

```rust
use quiche::h3::Connection as H3Connection;

// Create HTTP/3 layer on top of QUIC
let mut h3_conn = H3Connection::with_transport(
    &mut quic_conn,
    &h3_config,
)?;
```

### HTTP/3 Events

```rust
pub enum Event {
    Headers {
        stream_id: u64,
        headers: Vec<Header>,
        fin: bool,
    },
    Data {
        stream_id: u64,
        data: Vec<u8>,
        fin: bool,
    },
    Finished {
        stream_id: u64,
    },
    Reset {
        stream_id: u64,
        error_code: u64,
    },
    PriorityUpdate {
        stream_id: u64,
        prioritized_element_id: Vec<u8>,
        urgency: u8,
        incremental: bool,
    },
    GoAway {
        server_initiated_stream_id: u64,
    },
    Datagram {
        data: Vec<u8>,
    },
}
```

### QPACK Integration

```rust
pub struct QpackDecoder {
    dynamic_table: DynamicTable,
    max_table_size: u64,
    blocked_streams: HashMap<u64, Vec<HeaderBlock>>,
}

impl QpackDecoder {
    pub fn decode(&mut self, encoded_headers: &[u8]) -> Result<Vec<Header>>;
    pub fn insert(&mut self, name: &str, value: &str);
    pub fn set_dynam