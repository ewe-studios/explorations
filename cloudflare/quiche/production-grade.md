---
title: "Production-Grade QUIC Implementation"
subtitle: "Performance, monitoring, deployment, and operational considerations"
---

# Production-Grade QUIC Implementation

## Introduction

This document covers production considerations for deploying quiche-based QUIC/HTTP3 systems. We explore performance optimizations, memory management, monitoring, and deployment patterns.

## Table of Contents

1. [Performance Optimizations](#1-performance-optimizations)
2. [Memory Management](#2-memory-management)
3. [qlog Integration](#3-qlog-integration)
4. [Monitoring and Observability](#4-monitoring-and-observability)
5. [Deployment Patterns](#5-deployment-patterns)
6. [Security Considerations](#6-security-considerations)
7. [Troubleshooting](#7-troubleshooting)

---

## 1. Performance Optimizations

### 1.1 Batch Processing

Process multiple packets per event loop iteration:

```rust
const BATCH_SIZE: usize = 32;

pub struct PacketBatch {
    packets: Vec<(Vec<u8>, SocketAddr)>,
}

impl PacketBatch {
    pub fn receive_from_socket(
        socket: &UdpSocket,
        local_addr: SocketAddr,
    ) -> Result<Vec<(Vec<u8>, RecvInfo)>> {
        let mut results = Vec::with_capacity(BATCH_SIZE);

        for _ in 0..BATCH_SIZE {
            let mut buf = vec![0u8; 1500];
            match socket.recv_from(&mut buf) {
                Ok((len, from)) => {
                    buf.truncate(len);
                    results.push((buf, RecvInfo {
                        from,
                        to: local_addr,
                    }));
                }
                Err(ref e) if e.kind() == WouldBlock => break,
                Err(e) => return Err(e),
            }
        }

        Ok(results)
    }
}

// Process batch
for (buf, info) in batch {
    conn.recv(&mut buf, info)?;
}
```

### 1.2 Send Coalescing

Combine multiple QUIC packets into single UDP datagram:

```rust
impl Connection {
    pub fn send_coalesced(
        &mut self,
        out: &mut [u8],
        max_size: usize,
    ) -> Result<(usize, SendInfo)> {
        let mut written = 0;
        let mut send_info = None;

        // Keep writing packets until buffer full or no more packets
        loop {
            match self.send(&mut out[written..max_size]) {
                Ok((n, info)) => {
                    written += n;
                    send_info = Some(info);

                    // Check if we can fit another packet
                    if written + MIN_PKT_SIZE > max_size {
                        break;
                    }
                }
                Err(Error::Done) => break,
                Err(e) => return Err(e),
            }
        }

        if written == 0 {
            Err(Error::Done)
        } else {
            Ok((written, send_info.unwrap()))
        }
    }
}
```

### 1.3 Zero-Copy I/O with io_uring (Linux)

```rust
use io_uring::{IoUring, opcode};

pub struct QuicIoUring {
    ring: IoUring,
    recv_bufs: Vec<Vec<u8>>,
    send_bufs: Vec<Vec<u8>>,
}

impl QuicIoUring {
    pub fn submit_recv(
        &mut self,
        socket_fd: i32,
        buf_id: usize,
    ) {
        let recv = opcode::RecvFrom::new(
            socket_fd,
            self.recv_bufs[buf_id].as_mut_ptr(),
            self.recv_bufs[buf_id].len() as u32,
        )
        .build()
        .user_data(buf_id as u64);

        unsafe {
            self.ring.submission().push(&recv).unwrap();
        }
    }

    pub fn process_completions<F>(&mut self, mut handler: F)
    where
        F: FnMut(usize, &[u8], SocketAddr),
    {
        for cqe in self.ring.completion() {
            let buf_id = cqe.user_data() as usize;
            let len = cqe.result() as usize;

            handler(
                buf_id,
                &self.recv_bufs[buf_id][..len],
                // Extract peer address from recv result
            );
        }
    }
}
```

### 1.4 Connection Timeout Optimization

Use efficient timer wheel for many connections:

```rust
use timer_wheel::{TimerWheel, TimerHandle};

pub struct ConnectionManager {
    connections: HashMap<ConnectionId, Connection>,
    timers: TimerWheel<ConnectionId>,
}

impl ConnectionManager {
    pub fn update_timeouts(&mut self, now: Instant) {
        // Expire all timeouts since last check
        for conn_id in self.timers.expire(now) {
            if let Some(conn) = self.connections.get_mut(&conn_id) {
                conn.on_timeout();

                // Reschedule next timeout
                if let Some(next) = conn.timeout() {
                    self.timers.insert(now + next, conn_id);
                }
            }
        }
    }

    pub fn schedule_timeout(
        &mut self,
        conn_id: ConnectionId,
        delay: Duration,
    ) {
        self.timers.insert(Instant::now() + delay, conn_id);
    }
}
```

---

## 2. Memory Management

### 2.1 Connection Memory Limits

```rust
pub struct ConnectionLimits {
    /// Maximum connections
    max_connections: usize,
    /// Maximum memory per connection
    max_memory_per_conn: usize,
    /// Maximum stream memory
    max_stream_buffer: usize,
}

impl ConnectionLimits {
    pub fn check_memory(&self, current_usage: usize) -> bool {
        current_usage < self.max_connections * self.max_memory_per_conn
    }
}

pub struct Connection {
    /// Track memory usage
    memory_usage: AtomicUsize,
    limits: Arc<ConnectionLimits>,
}

impl Connection {
    pub fn allocate_buffer(&self, size: usize) -> Result<Vec<u8>> {
        let new_usage = self.memory_usage.fetch_add(size, Relaxed) + size;

        if !self.limits.check_memory(new_usage) {
            self.memory_usage.fetch_sub(size, Relaxed);
            return Err(Error::OutOfMemory);
        }

        Ok(vec![0u8; size])
    }
}
```

### 2.2 Stream Buffer Management

```rust
pub struct RecvBuf<F: BufFactory> {
    data: Vec<u8>,
    max_len: usize,
    /// Memory tracker
    mem_tracker: Arc<MemoryTracker>,
}

impl<F: BufFactory> RecvBuf<F> {
    pub fn write(&mut self, buf: &[u8], offset: u64) -> Result<()> {
        let new_len = (offset as usize + buf.len()).max(self.data.len());

        if new_len > self.max_len {
            return Err(Error::FlowControl);
        }

        // Grow buffer if needed
        if new_len > self.data.len() {
            self.mem_tracker.allocate(new_len - self.data.len())?;
            self.data.resize(new_len, 0);
        }

        self.data[offset as usize..offset as usize + buf.len()]
            .copy_from_slice(buf);

        Ok(())
    }
}
```

### 2.3 Garbage Collection for Completed Streams

```rust
impl StreamMap {
    /// Collect completed streams to free memory
    pub fn collect_garbage(&mut self) -> usize {
        let mut collected = 0;

        self.streams.retain(|id, stream| {
            if stream.is_completed() {
                self.collected.insert(*id);
                collected += 1;
                false  // Remove from active streams
            } else {
                true  // Keep
            }
        });

        collected
    }

    /// Periodic GC trigger
    pub fn maybe_gc(&mut self, gc_threshold: usize) {
        if self.collected.len() > gc_threshold {
            self.collect_garbage();
        }
    }
}
```

---

## 3. qlog Integration

### 3.1 Enabling qlog

```rust
// Cargo.toml
[dependencies]
quiche = { version = "0.26", features = ["qlog"] }

// Application code
use quiche::qlog::{QlogWriter, QlogLevel};

let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;

// Create qlog writer
let qlog_writer = QlogWriter::new(
    File::create("connection.qlog")?,
    QlogLevel::Base,
);

// Create connection with qlog
let mut conn = quiche::connect(...)?;
conn.set_qlog(qlog_writer);
```

### 3.2 qlog Event Structure

```rust
// From quiche qlog integration
use qlog::events::{Event, EventImportance, EventType};

pub struct QlogWriter {
    file: BufWriter<File>,
    level: QlogLevel,
}

impl QlogWriter {
    pub fn write_event(&mut self, event: Event) {
        if self.importance(event.importance) {
            let json = serde_json::to_string(&event).unwrap();
            writeln!(self.file, "{}", json).unwrap();
        }
    }

    /// Log packet sent
    pub fn packet_sent(
        &mut self,
        packet_type: PacketType,
        pn: u64,
        size: usize,
    ) {
        use qlog::events::quic::*;

        let event = Event {
            event_type: EventType::PacketSent,
            data: EventData::PacketSent(PacketSent {
                packet_type: packet_type.to_qlog(),
                packet_number: pn,
                raw: RawInfo {
                    length: Some(size as u64),
                    ..Default::default()
                },
                frames: vec![],  // Populate with sent frames
            }),
            importance: EventImportance::Base,
        };

        self.write_event(event);
    }
}
```

### 3.3 Analyzing qlog Files

```bash
# Use qvis to visualize
curl https://qvis.quictools.app/ -F "file=@connection.qlog"

# Or use qlog-dancer (included with quiche)
cargo run --bin qlog-dancer -- connection.qlog

# Analyze with custom tool
use qlog::QlogReader;

let reader = QlogReader::open("connection.qlog")?;
for event in reader.events() {
    match event.event_type {
        EventType::PacketSent => {
            // Track send rate
        }
        EventType::PacketReceived => {
            // Track RTT
        }
        EventType::LossTimerExpired => {
            // Detect loss patterns
        }
        _ => {}
    }
}
```

---

## 4. Monitoring and Observability

### 4.1 Key Metrics

```rust
use prometheus::{IntCounter, IntGauge, Histogram, Registry};

pub struct QuicMetrics {
    /// Active connections
    pub connections: IntGauge,
    /// Total packets sent
    pub packets_sent: IntCounter,
    /// Total packets received
    pub packets_received: IntCounter,
    /// Packet loss rate
    pub packet_loss: Histogram,
    /// RTT distribution
    pub rtt: Histogram,
    /// Stream creation rate
    pub streams_created: IntCounter,
    /// Bytes sent/received
    pub bytes_sent: IntCounter,
    pub bytes_received: IntCounter,
}

impl QuicMetrics {
    pub fn register(registry: &Registry) -> Result<Self> {
        Ok(Self {
            connections: IntGauge::new(
                "quic_connections",
                "Active QUIC connections"
            )?,
            packets_sent: IntCounter::new(
                "quic_packets_sent_total",
                "Total packets sent"
            )?,
            // ... register all metrics
        })
    }
}
```

### 4.2 Connection Health Checks

```rust
pub struct ConnectionHealth {
    /// Last packet received time
    last_rx: Instant,
    /// Consecutive timeouts
    timeout_count: usize,
    /// Packet loss rate (exponential moving average)
    loss_rate_ema: f64,
    /// RTT trend
    rtt_samples: Vec<Duration>,
}

impl ConnectionHealth {
    pub fn check(&self) -> HealthStatus {
        let idle = Instant::now() - self.last_rx;

        if idle > Duration::from_secs(300) {
            return HealthStatus::Dead;
        }

        if self.timeout_count > 5 {
            return HealthStatus::Unhealthy;
        }

        if self.loss_rate_ema > 0.1 {
            return HealthStatus::Degraded;
        }

        HealthStatus::Healthy
    }

    pub fn update_loss(&mut self, lost: usize, acked: usize) {
        let rate = lost as f64 / (lost + acked) as f64;
        // EMA with alpha = 0.1
        self.loss_rate_ema = 0.1 * rate + 0.9 * self.loss_rate_ema;
    }
}
```

### 4.3 Tracing Integration

```rust
use tracing::{info, warn, error, span, Level};

impl Connection {
    #[instrument(skip(self, buf), fields(conn_id = %self.trace_id))]
    pub fn recv(&mut self, buf: &mut [u8], info: RecvInfo) -> Result<usize> {
        let span = span!(Level::DEBUG, "packet_recv", from = %info.from);
        let _enter = span.enter();

        match self.recv_inner(buf, info) {
            Ok(n) => {
                debug!(bytes = n, "received packet");
                Ok(n)
            }
            Err(Error::Done) => {
                trace!("no more packets to process");
                Err(Error::Done)
            }
            Err(e) => {
                warn!(error = %e, "receive error");
                Err(e)
            }
        }
    }
}
```

---

## 5. Deployment Patterns

### 5.1 Multi-Connection Server

```rust
pub struct QuicServer {
    config: Arc<Config>,
    socket: Arc<UdpSocket>,
    connections: DashMap<ConnectionId, Connection>,
    accept_queue: mpsc::Sender<Connection>,
}

impl QuicServer {
    pub async fn run(&self) -> Result<()> {
        let mut recv_buf = vec![0u8; 65535];

        loop {
            // Receive packets from all clients
            let (len, from) = self.socket.recv_from(&mut recv_buf).await?;

            // Extract connection ID
            let cid = extract_connection_id(&recv_buf[..len])?;

            // Get or create connection
            let mut conn = self.connections
                .entry(cid.clone())
                .or_insert_with(|| {
                    Connection::accept(&cid, /*...*/).unwrap()
                });

            // Process packet
            let recv_info = RecvInfo { from, to: self.local_addr };
            match conn.recv(&mut recv_buf[..len], recv_info) {
                Ok(_) => {
                    // Send responses
                    self.send_packets(&mut conn).await?;
                }
                Err(Error::Done) => {}
                Err(_) => {
                    // Remove failed connection
                    self.connections.remove(&cid);
                }
            }
        }
    }
}
```

### 5.2 Connection Migration Handling

```rust
impl Connection {
    /// Handle client IP change
    pub fn handle_migration(
        &mut self,
        new_from: SocketAddr,
        new_to: SocketAddr,
    ) -> Result<()> {
        // Validate new path
        self.send_path_challenge(new_from)?;

        // Don't migrate yet - wait for validation
        self.pending_migration = Some(PendingMigration {
            new_from,
            new_to,
            challenge_sent: Instant::now(),
        });

        Ok(())
    }

    /// Complete migration after path validation
    pub fn complete_migration(&mut self) {
        if let Some(migration) = self.pending_migration.take() {
            self.paths.active().peer_addr = migration.new_from;
            self.paths.active().local_addr = migration.new_to;

            // Reset congestion control for new path
            self.recovery.on_path_change();

            info!(
                from = %migration.new_from,
                "Connection migrated to new path"
            );
        }
    }
}
```

### 5.3 Graceful Shutdown

```rust
pub struct GracefulShutdown {
    connections: Arc<DashMap<ConnectionId, Connection>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl GracefulShutdown {
    pub async fn initiate(&self, timeout: Duration) {
        // Signal no new connections
        self.shutdown_tx.send(()).unwrap();

        // Send GOAWAY to all connections
        for mut conn in self.connections.iter_mut() {
            conn.send_goaway();
        }

        // Wait for connections to drain
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.connections.is_empty() {
                break;
            }

            // Remove closed connections
            self.connections.retain(|_, c| !c.should_close());

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Force close remaining
        self.connections.clear();
    }
}
```

---

## 6. Security Considerations

### 6.1 Rate Limiting

```rust
use governor::{Quota, RateLimiter};

pub struct RateLimitedServer {
    limiter: RateLimiter<Quota>,
    per_ip_limiter: DashMap<IpAddr, RateLimiter>,
}

impl RateLimitedServer {
    pub fn check_rate_limit(&self, from: IpAddr) -> bool {
        // Global rate limit
        if self.limiter.check().is_err() {
            return false;
        }

        // Per-IP rate limit
        let ip_limiter = self.per_ip_limiter
            .entry(from)
            .or_insert_with(|| {
                RateLimiter::direct(Quota::per_second(nonzero!(100u32)))
            });

        ip_limiter.check().is_ok()
    }

    pub fn cleanup_old_limiters(&self) {
        // Remove limiters for IPs not seen recently
        self.per_ip_limiter.retain(|_, l| {
            l.check().is_ok()  // Keep if still active
        });
    }
}
```

### 6.2 Certificate Validation

```rust
use webpki::{ServerCertVerifier, ServerCertVerified};
use rustls::{Certificate, ServerName};

pub struct StrictCertVerifier {
    roots: RootCertStore,
    required_algorithms: Vec<SignatureAlgorithm>,
}

impl ServerCertVerifier for StrictCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &Certificate,
        intermediates: &[Certificate],
        server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        ocsp_response: &[u8],
        now: std::time::SystemTime,
    ) -> Result<ServerCertVerified, Error> {
        // Verify certificate chain
        let cert = webpki::EndEntityCert::try_from(end_entity.0.as_slice())?;

        cert.verify_is_valid_tls_server_cert(
            &SUPPORTED_SIG_ALGS,
            &self.roots,
            intermediates,
            now,
        )?;

        // Additional checks
        if !self.required_algorithms.contains(&cert.signature_algorithm()) {
            return Err(Error::UnsupportedSignatureAlgorithm);
        }

        // Check OCSP stapling
        if !ocsp_response.is_empty() {
            self.verify_ocsp(end_entity, intermediates, ocsp_response, now)?;
        }

        Ok(ServerCertVerified::assertion())
    }
}
```

### 6.3 Anti-Amplification

```rust
impl Connection {
    /// Check anti-amplification limit
    pub fn check_amplification_limit(
        &self,
        bytes_to_send: usize,
        bytes_received: usize,
    ) -> bool {
        let amplification_factor = self.config.max_amplification_factor;

        bytes_to_send <= bytes_received * amplification_factor
    }

    /// Before sending, check amplification
    pub fn send_with_amplification_check(
        &mut self,
        out: &mut [u8],
    ) -> Result<(usize, SendInfo)> {
        let bytes_received = self.stats.bytes_received;
        let bytes_sent = self.stats.bytes_sent;

        // Estimate packet size
        let estimated_size = out.len().min(1500);

        if !self.check_amplification_limit(
            bytes_sent + estimated_size,
            bytes_received,
        ) {
            return Err(Error::Done);  // Blocked by amplification limit
        }

        self.send(out)
    }
}
```

---

## 7. Troubleshooting

### 7.1 Common Issues

```rust
/// Connection fails handshake
/// Check:
/// 1. Certificate chain is valid
/// 2. ALPN protocols match
/// 3. UDP packets not blocked by firewall

/// High packet loss
/// Check:
/// 1. MTU issues - enable PMTUD
/// 2. Congestion - monitor CWND
/// 3. Network path - check routing

/// Connection timeout
/// Check:
/// 1. PTO configuration
/// 2. Keep-alive settings
/// 3. NAT rebinding handling
```

### 7.2 Debug Logging

```rust
use env_logger::Env;

fn init_logging() {
    env_logger::Builder::from_env(
        Env::default().default_filter_or("quiche=debug,myapp=info")
    )
    .format_timestamp_millis()
    .init();
}

// Run with RUST_LOG=quiche=debug cargo run
```

### 7.3 Connection Diagnostics

```rust
impl Connection {
    pub fn dump_state(&self) -> ConnectionState {
        ConnectionState {
            state: self.state,
            local_cwnd: self.recovery.cwnd(),
            local_rtt: self.recovery.rtt(),
            peer_max_data: self.peer_max_data,
            local_max_data: self.local_max_data,
            streams: self.streams.dump(),
            paths: self.paths.dump(),
        }
    }
}

#[derive(Serialize)]
pub struct ConnectionState {
    pub state: String,
    pub local_cwnd: usize,
    pub local_rtt: Duration,
    pub peer_max_data: u64,
    pub local_max_data: u64,
    pub streams: StreamState,
    pub paths: Vec<PathState>,
}
```

---

## Summary

### Key Takeaways

1. **Performance** - Batch processing, send coalescing, io_uring for I/O
2. **Memory** - Connection limits, stream buffer management, GC
3. **qlog** - Enable for debugging, analyze with qvis
4. **Monitoring** - Prometheus metrics, health checks, tracing
5. **Deployment** - Multi-connection servers, migration handling, graceful shutdown
6. **Security** - Rate limiting, certificate validation, anti-amplification

---

## Further Reading

- [qlog Specification](https://datatracker.ietf.org/doc/html/draft-ietf-quic-qlog-main-schema)
- [Prometheus Rust Client](https://docs.rs/prometheus/)
- [governor Rate Limiter](https://docs.rs/governor/)
- [Cloudflare QUIC Deployment Guide](https://blog.cloudflare.com/http3-the-past-present-and-future/)
