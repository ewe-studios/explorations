---
title: "Congestion Control Deep Dive"
subtitle: "BBR, Cubic, pacing, and recovery in quiche"
---

# Congestion Control Deep Dive

## Introduction

This document provides a comprehensive deep dive into congestion control and loss recovery in quiche. We'll explore loss detection, congestion control algorithms (Cubic, BBR2), pacing, and the recovery architecture.

## Table of Contents

1. [Loss Detection](#1-loss-detection)
2. [Recovery Architecture](#2-recovery-architecture)
3. [Cubic Congestion Control](#3-cubic-congestion-control)
4. [BBR2 Congestion Control](#4-bbr2-congestion-control)
5. [Pacing](#5-pacing)
6. [HyStart++](#6-hystart)
7. [Proportional Rate Reduction](#7-proportional-rate-reduction)

---

## 1. Loss Detection

### 1.1 RFC 9002 Loss Detection

QUIC loss detection per RFC 9002 uses both packet threshold and time threshold:

```rust
// From quiche/src/recovery/mod.rs
// Constants from RFC 9002
const INITIAL_PACKET_THRESHOLD: u64 = 3;      // Packets before declaring loss
const MAX_PACKET_THRESHOLD: u64 = 20;         // Maximum threshold
const INITIAL_TIME_THRESHOLD: f64 = 9.0 / 8.0;  // 1.125x RTT
const GRANULARITY: Duration = Duration::from_millis(1);  // Minimum granularity

// Loss detection timer
struct LossDetectionTimer {
    time: Option<Instant>,
}
```

### 1.2 Loss Detection Algorithm

```
Packet Loss Detection:

Sent packets: [1] [2] [3] [4] [5] [6] [7] [8]
                │         │
                │         └─ largest_acked = 8
                │
                └─ packet 3 not acknowledged

Detection conditions:
1. Packet threshold: 3+ packets sent after lost packet
   - Packets 4,5,6,7,8 sent after 3 = 5 packets > threshold
   - Packet 3 declared lost

2. Time threshold: packet outstanding > 1.125 * RTT
   - If packet 3 sent at T0, now is T0 + 1.125*RTT
   - Packet 3 declared lost
```

### 1.3 Loss Detection Implementation

```rust
// From quiche/src/recovery/mod.rs
impl RecoveryOps for LegacyRecovery {
    fn detect_lost_packets(
        &mut self,
        epoch: packet::Epoch,
        now: Instant,
    ) -> Vec<Sent> {
        let largest_acked = self.largest_acked[epoch]?;
        let rtt = self.rtt[epoch];

        // Time threshold = max(1.125 * RTT, granularity)
        let time_threshold = cmp::max(
            rtt.smoothed * INITIAL_TIME_THRESHOLD,
            GRANULARITY,
        );

        let loss_time = now - time_threshold;

        let mut lost = Vec::new();

        for packet in &self.sent[epoch] {
            // Packet threshold check
            if largest_acked >= packet.pkt_num + INITIAL_PACKET_THRESHOLD {
                lost.push(packet.clone());
                continue;
            }

            // Time threshold check
            if packet.time_sent <= loss_time {
                lost.push(packet.clone());
            }
        }

        lost
    }
}
```

### 1.4 PTO (Probe Timeout)

```rust
// From quiche/src/recovery/mod.rs
impl RecoveryOps {
    /// Calculate Probe Timeout
    fn pto(&self) -> Duration {
        let rtt = self.rtt.smoothed;
        let rttvar = self.rtt.variance;

        // PTO = smoothed_RTT + 4 * rttvar + max(granularity, 1ms)
        let pto = rtt + 4 * rttvar + cmp::max(GRANULARITY, Duration::from_millis(1));

        // Apply backoff multiplier for consecutive PTOs
        pto * (2 ^ self.pto_count)
    }

    /// PTO timer expired
    fn on_loss_detection_timeout(
        &mut self,
        now: Instant,
    ) -> OnLossDetectionTimeoutOutcome {
        let earliest_epoch = self.earliest_loss_time();

        if earliest_epoch.is_some() {
            // Time-threshold loss detected
            let outcome = self.detect_lost_packets(earliest_epoch, now);
            self.congestion_event(earliest_epoch, now);
        } else {
            // PTO - send probe packets
            self.pto_count += 1;
            self.send_probe_packets();
        }

        self.set_loss_detection_timer();
        outcome
    }
}
```

### 1.5 RTT Estimation

```rust
// From quiche/src/recovery/rtt.rs
pub struct RttStats {
    /// Latest RTT measurement
    latest: Duration,
    /// Smoothed RTT (EWMA)
    smoothed: Option<Duration>,
    /// RTT variance
    variance: Option<Duration>,
    /// Minimum RTT observed
    min: Duration,
    /// Maximum RTT observed
    max: Duration,
}

impl RttStats {
    pub fn update(&mut self, rtt: Duration, max_ack_delay: Duration) {
        self.latest = rtt;

        // Update min RTT (no filtering)
        if self.min.is_zero() || rtt < self.min {
            self.min = rtt;
        }

        // Update max RTT
        if rtt > self.max {
            self.max = rtt;
        }

        match self.smoothed {
            None => {
                // First measurement
                self.smoothed = Some(rtt);
                self.variance = Some(rtt / 2);
            }
            Some(smoothed) => {
                // EWMA update (RFC 6298)
                let rttvar_sample = if smoothed > rtt {
                    smoothed - rtt
                } else {
                    rtt - smoothed
                };

                self.variance = Some(self.variance.unwrap() * 3/4 + rttvar_sample / 4);
                self.smoothed = Some(smoothed * 7/8 + rtt / 8);
            }
        }
    }
}
```

---

## 2. Recovery Architecture

### 2.1 Recovery Enum with enum_dispatch

quiche uses enum_dispatch for zero-cost dispatch between CC algorithms:

```rust
// From quiche/src/recovery/mod.rs
use enum_dispatch::enum_dispatch;

#[enum_dispatch::enum_dispatch(RecoveryOps)]
#[derive(Debug)]
pub(crate) enum Recovery {
    Legacy(LegacyRecovery),
    GCongestion(GRecovery),
}

pub trait RecoveryOps {
    fn lost_count(&self) -> usize;
    fn bytes_lost(&self) -> u64;

    fn on_packet_sent(
        &mut self,
        pkt: Sent,
        epoch: packet::Epoch,
        now: Instant,
    );

    fn on_ack_received(
        &mut self,
        ranges: &RangeSet,
        ack_delay: u64,
        epoch: packet::Epoch,
        now: Instant,
    ) -> Result<OnAckReceivedOutcome>;

    fn on_loss_detection_timeout(
        &mut self,
        now: Instant,
    ) -> OnLossDetectionTimeoutOutcome;

    fn cwnd(&self) -> usize;
    fn cwnd_available(&self) -> usize;
    fn rtt(&self) -> Duration;
    fn delivery_rate(&self) -> Bandwidth;
    // ... 40+ methods
}
```

### 2.2 RecoveryConfig

```rust
// From quiche/src/recovery/mod.rs
#[derive(Clone, Copy, PartialEq)]
pub struct RecoveryConfig {
    pub initial_rtt: Duration,
    pub max_send_udp_payload_size: usize,
    pub max_ack_delay: Duration,
    pub cc_algorithm: CongestionControlAlgorithm,
    pub custom_bbr_params: Option<BbrParams>,
    pub hystart: bool,
    pub pacing: bool,
    pub max_pacing_rate: Option<u64>,
    pub initial_congestion_window_packets: usize,
    pub enable_relaxed_loss_threshold: bool,
    pub enable_cubic_idle_restart_fix: bool,
}

impl RecoveryConfig {
    pub fn from_config(config: &Config) -> Self {
        Self {
            initial_rtt: config.initial_rtt,
            max_send_udp_payload_size: config.max_send_udp_payload_size,
            cc_algorithm: config.cc_algorithm,
            custom_bbr_params: config.custom_bbr_params,
            hystart: config.hystart,
            pacing: config.pacing,
            max_pacing_rate: config.max_pacing_rate,
            initial_congestion_window_packets:
                config.initial_congestion_window_packets,
            // ...
        }
    }
}
```

### 2.3 Congestion Control Algorithm Selection

```rust
// From quiche/src/recovery/mod.rs
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CongestionControlAlgorithm {
    Reno     = 0,
    CUBIC    = 1,
    Bbr2Gcongestion = 4,  // Gap for removed algorithms
}

impl Config {
    pub fn set_cc_algorithm(&mut self, algo: CongestionControlAlgorithm) {
        self.cc_algorithm = algo;
    }

    pub fn set_cc_algorithm_name(&mut self, name: &str) -> Result<()> {
        self.cc_algorithm = match name.to_lowercase().as_str() {
            "reno" => CongestionControlAlgorithm::Reno,
            "cubic" => CongestionControlAlgorithm::CUBIC,
            "bbr2" | "bbr" => CongestionControlAlgorithm::Bbr2Gcongestion,
            _ => return Err(Error::CongestionControl),
        };
        Ok(())
    }
}

impl Recovery {
    pub fn new_with_config(config: &RecoveryConfig) -> Self {
        // Try GRecovery first (supports all algorithms)
        // Fall back to LegacyRecovery for Reno/CUBIC
        match config.cc_algorithm {
            CongestionControlAlgorithm::Bbr2Gcongestion => {
                Recovery::GCongestion(GRecovery::new(config))
            }
            _ => {
                // Reno/CUBIC can use either
                if cfg!(feature = "gcongestion") {
                    Recovery::GCongestion(GRecovery::new(config))
                } else {
                    Recovery::Legacy(LegacyRecovery::new(config))
                }
            }
        }
    }
}
```

### 2.4 Sent Packet Tracking

```rust
// From quiche/src/recovery/mod.rs
#[derive(Debug, Clone)]
pub struct Sent {
    /// Packet number
    pub pkt_num: u64,
    /// Frames contained in packet
    pub frames: SmallVec<[Frame; 1]>,
    /// Time packet was sent
    pub time_sent: Instant,
    /// Size of packet (including UDP/IP headers for CC)
    pub size: usize,
    /// Whether packet counts toward bytes in flight
    pub in_flight: bool,
    /// Whether packet is ACK-eliciting
    pub ack_eliciting: bool,
    /// Path ID (for multipath QUIC)
    pub path_id: usize,
}

impl Recovery {
    /// Record packet sent
    fn on_packet_sent(
        &mut self,
        pkt: Sent,
        epoch: packet::Epoch,
        now: Instant,
    ) {
        // Add to sent packets list
        self.sent[epoch].push_back(pkt);

        // Update bytes in flight
        if self.sent[epoch].back().unwrap().in_flight {
            self.bytes_in_flight += self.sent[epoch].back().unwrap().size;
        }

        // Call CC algorithm hook
        self.cc_ops.on_packet_sent(
            &mut self.congestion,
            self.sent[epoch].back().unwrap().size,
            self.bytes_in_flight,
            now,
        );
    }
}
```

---

## 3. Cubic Congestion Control

### 3.1 CUBIC Overview

CUBIC is the default congestion control in quiche:

```
CUBIC Window Function:
W(t) = C * (t - K)^3 + W_max

Where:
- t = time since last congestion event
- K = cubic_root((W_max - cwnd) / C)
- W_max = window at last congestion event
- C = 0.4 (CUBIC constant)
```

### 3.2 CUBIC Implementation

```rust
// From quiche/src/recovery/congestion/cubic.rs
const BETA_CUBIC: f64 = 0.7;  // Multiplicative decrease factor
const C: f64 = 0.4;           // CUBIC scaling factor
const ALPHA_AIMD: f64 = 3.0 * (1.0 - BETA_CUBIC) / (1.0 + BETA_CUBIC);

#[derive(Debug, Default)]
pub struct State {
    k: f64,          // Time to reach W_max
    w_max: f64,      // Window at last congestion event
    w_est: f64,      // Estimated window
    alpha_aimd: f64, // AIMD parameter
    cwnd_inc: usize, // CWND increment during CA
    prior: PriorState,
}

impl State {
    // K = cubic_root((W_max - cwnd) / C)
    fn cubic_k(&self, cwnd: usize, max_datagram_size: usize) -> f64 {
        let w_max = self.w_max / max_datagram_size as f64;
        let cwnd = cwnd as f64 / max_datagram_size as f64;
        libm::cbrt((w_max - cwnd) / C)
    }

    // W_cubic(t) = C * (t - K)^3 + W_max
    fn w_cubic(&self, t: Duration, max_datagram_size: usize) -> f64 {
        let w_max = self.w_max / max_datagram_size as f64;
        (C * (t.as_secs_f64() - self.k).powi(3) + w_max) *
            max_datagram_size as f64
    }
}
```

### 3.3 Congestion Event

```rust
// From quiche/src/recovery/congestion/cubic.rs
fn congestion_event(
    r: &mut Congestion,
    _lost_bytes: usize,
    bytes_in_flight: usize,
    now: Instant,
) {
    // Save state for potential rollback
    r.cubic_state.prior.congestion_window = r.cwnd;
    r.cubic_state.prior.ssthresh = r.ssthresh;
    r.cubic_state.prior.w_max = r.cubic_state.w_max;
    r.cubic_state.prior.k = r.cubic_state.k;
    r.cubic_state.prior.epoch_start = r.congestion_recovery_start_time;

    // Multiplicative decrease
    r.cubic_state.w_max = r.cwnd as f64;
    r.ssthresh = (r.cwnd as f64 * BETA_CUBIC) as usize;
    r.ssthresh = cmp::max(r.ssthresh, MINIMUM_WINDOW_PACKETS * r.max_datagram_size);

    // Reset cwnd to ssthresh
    r.cwnd = r.ssthresh;

    // Calculate K for new cubic function
    r.cubic_state.k = r.cubic_state.cubic_k(r.cwnd, r.max_datagram_size);

    // Start new congestion recovery epoch
    r.congestion_recovery_start_time = Some(now);
}
```

### 3.4 Window Increase

```rust
// From quiche/src/recovery/congestion/cubic.rs
fn on_packets_acked(
    r: &mut Congestion,
    acked_packets: &[Acked],
    _bytes_in_flight: usize,
    now: Instant,
) {
    let elapsed = now - r.congestion_recovery_start_time.unwrap();
    let max_datagram_size = r.max_datagram_size;

    for acked in acked_packets {
        if r.cwnd < r.ssthresh {
            // Slow Start
            r.cwnd += acked.size;
        } else {
            // Congestion Avoidance - CUBIC
            let target = r.cubic_state.w_cubic(elapsed, max_datagram_size);

            // W_est increment
            let w_est_inc = r.cubic_state.w_est_inc(
                acked.size,
                r.cwnd,
                max_datagram_size,
            );
            r.cubic_state.w_est += w_est_inc;

            // Use larger of cubic window or W_est
            let cubic_window = target - r.cwnd as f64;
            let increment = cmp::max(cubic_window as usize, w_est_inc as usize);

            r.cubic_state.cwnd_inc += increment;

            // Apply increment when we've ACKed enough data
            if r.cubic_state.cwnd_inc >= max_datagram_size {
                r.cwnd += max_datagram_size;
                r.cubic_state.cwnd_inc -= max_datagram_size;
            }
        }
    }
}
```

### 3.5 Idle Restart Fix

```rust
// From quiche/src/recovery/congestion/cubic.rs
fn on_packet_sent(
    r: &mut Congestion,
    sent_bytes: usize,
    bytes_in_flight: usize,
    now: Instant,
) {
    // Don't adjust epoch for non-data packets
    if sent_bytes == 0 && r.enable_cubic_idle_restart_fix {
        return;
    }

    let cubic = &mut r.cubic_state;

    // First transmit when no packets in flight
    if bytes_in_flight == 0 {
        if let Some(recovery_start_time) = r.congestion_recovery_start_time {
            // Measure idle from most recent activity
            let idle_start = if r.enable_cubic_idle_restart_fix {
                cmp::max(cubic.last_ack_time, cubic.last_sent_time)
            } else {
                cubic.last_sent_time
            };

            if let Some(idle_start) = idle_start {
                if idle_start < now {
                    let delta = now - idle_start;
                    // Shift epoch to keep cwnd growth on cubic curve
                    r.congestion_recovery_start_time =
                        Some(recovery_start_time + delta);
                }
            }
        }
    }

    cubic.last_sent_time = Some(now);
}
```

---

## 4. BBR2 Congestion Control

### 4.1 BBR Overview

BBR (Bottleneck Bandwidth and RTT) is a model-based CC algorithm:

```
BBR Model:
- BtlBw (Bottleneck Bandwidth) - maximum delivery rate
- RTProp (Round Trip Propagation) - minimum RTT
- BDP = BtlBw * RTProp (Bandwidth-Delay Product)

BBR operates in 4 phases:
1. Startup - exponentially increase cwnd until bandwidth plateau
2. Drain - drain queue built during startup
3. ProbeBW - cycle through gains to probe for bandwidth
4. ProbeRTT - periodically reduce cwnd to measure RTProp
```

### 4.2 BBR2 State Machine

```rust
// From quiche/src/recovery/gcongestion/bbr2/mode.rs
pub enum Mode {
    Startup,
    Drain,
    ProbeBw(ProbeBwPhase),
    ProbeRtt,
}

pub enum ProbeBwPhase {
    Startup,     // Probe up
    ProbeDown,   // Probe down
    ProbeCruise, // Maintain
    ProbeRefill, // Refill pipe
    ProbeUp,     // Probe up again
}

impl Mode {
    /// Get pacing gain for current state
    pub fn pacing_gain(&self, params: &BbrParams) -> f32 {
        match self {
            Mode::Startup => params.startup_pacing_gain,  // 2.89
            Mode::Drain => params.drain_pacing_gain,      // 0.7
            Mode::ProbeBw(ProbeBwPhase::ProbeUp) => {
                params.probe_bw_probe_up_pacing_gain  // 1.25
            }
            Mode::ProbeBw(ProbeBwPhase::ProbeDown) => {
                params.probe_bw_probe_down_pacing_gain  // 0.75
            }
            Mode::ProbeBw(_) => params.probe_bw_default_pacing_gain,  // 1.0
            Mode::ProbeRtt => params.probe_rtt_pacing_gain,  // 0.5
        }
    }

    /// Get cwnd gain for current state
    pub fn cwnd_gain(&self, params: &BbrParams) -> f32 {
        match self {
            Mode::Startup => params.startup_cwnd_gain,  // 2.89
            Mode::Drain => params.drain_cwnd_gain,      // 2.0
            Mode::ProbeBw(ProbeBwPhase::ProbeUp) => {
                params.probe_bw_up_cwnd_gain  // 2.0
            }
            Mode::ProbeBw(_) => params.probe_bw_cwnd_gain,  // 2.0
            Mode::ProbeRtt => params.probe_rtt_cwnd_gain,  // 1.0
        }
    }
}
```

### 4.3 BBR2 Network Model

```rust
// From quiche/src/recovery/gcongestion/bbr2/network_model.rs
pub struct BBRv2NetworkModel {
    /// Max bandwidth filter (windowed maximum)
    max_bw_filter: WindowedMaxFilter,

    /// Min RTT filter (windowed minimum)
    min_rtt_filter: MinRttFilter,

    /// Current bandwidth estimate
    bw: Bandwidth,

    /// Current RTT estimate
    rtt: Duration,

    /// BDP estimate
    bdp: usize,
}

impl BBRv2NetworkModel {
    /// Update model with new sample
    pub fn on_sample(
        &mut self,
        delivered_bytes: u64,
        interval: Duration,
        rtt: Duration,
    ) {
        let sample_bw = Bandwidth::from_bytes_per_duration(
            delivered_bytes,
            interval,
        );

        // Update max bandwidth filter
        self.max_bw_filter.update(sample_bw);

        // Update min RTT filter
        self.min_rtt_filter.update(rtt);

        // Get current estimates
        self.bw = self.max_bw_filter.get_best();
        self.rtt = self.min_rtt_filter.get_best();

        // Calculate BDP
        self.bdp = (self.bw.as_bytes_per_sec() as f64 *
            self.rtt.as_secs_f64()) as usize;
    }

    /// Check if bandwidth has plateaued (exit Startup)
    pub fn has_bandwidth_plateaued(
        &self,
        threshold: f32,
        rounds: usize,
    ) -> bool {
        let current = self.bw;
        let baseline = self.bw_at_startup_entry;

        let growth = (current.as_bytes_per_sec() as f32 -
            baseline.as_bytes_per_sec() as f32) /
            baseline.as_bytes_per_sec() as f32;

        growth < threshold && self.rounds_without_growth >= rounds
    }
}
```

### 4.4 BBR2 Startup Exit

```rust
// From quiche/src/recovery/gcongestion/bbr2/startup.rs
impl Startup {
    /// Check if should exit Startup
    pub fn should_exit(
        &self,
        model: &BBRv2NetworkModel,
        params: &BbrParams,
    ) -> Option<ExitReason> {
        // Check bandwidth plateau
        if model.has_bandwidth_plateaued(
            params.full_bw_threshold,
            params.startup_full_bw_rounds,
        ) {
            return Some(ExitReason::BandwidthPlateau);
        }

        // Check loss threshold
        if self.loss_rate > params.loss_threshold {
            return Some(ExitReason::ExcessiveLoss);
        }

        // Check queueing (bytes_in_flight > BDP)
        if self.queueing_rounds > params.max_startup_queue_rounds {
            return Some(ExitReason::QueueFull);
        }

        None
    }
}
```

### 4.5 BBR2 Pacing

```rust
// From quiche/src/recovery/gcongestion/pacer.rs
pub struct Pacer {
    /// Token bucket for pacing
    tokens: f64,
    /// Last update time
    last_update: Instant,
    /// Current pacing rate
    pacing_rate: Bandwidth,
}

impl Pacer {
    /// Calculate when next packet can be sent
    pub fn next_send_time(
        &mut self,
        packet_size: usize,
        now: Instant,
        cwnd: usize,
        bytes_in_flight: usize,
    ) -> ReleaseTime {
        // Update tokens
        let elapsed = now - self.last_update;
        self.tokens += self.pacing_rate.as_bytes_per_sec() as f64 *
            elapsed.as_secs_f64();

        // Cap tokens at cwnd
        self.tokens = self.tokens.min(cwnd as f64);

        self.last_update = now;

        // Check if we can send
        if bytes_in_flight >= cwnd {
            // Window limited
            return ReleaseTime::WindowLimited;
        }

        if self.tokens >= packet_size as f64 {
            // Have tokens, can send now
            self.tokens -= packet_size as f64;
            return ReleaseTime::Now;
        }

        // Need to wait for tokens
        let wait_time = Duration::from_secs_f64(
            (packet_size as f64 - self.tokens) /
            self.pacing_rate.as_bytes_per_sec() as f64,
        );

        ReleaseTime::Timer(now + wait_time)
    }

    /// Set pacing rate
    pub fn set_rate(&mut self, rate: Bandwidth) {
        self.pacing_rate = rate;
    }
}
```

---

## 5. Pacing

### 5.1 ReleaseTime and ReleaseDecision

```rust
// From quiche/src/recovery/mod.rs
#[derive(Debug, Clone, Copy)]
pub struct ReleaseTime {
    pub time: Instant,
}

#[derive(Debug, Clone, Copy)]
pub struct ReleaseDecision {
    pub time: Instant,
    pub bytes: usize,
}

impl Recovery {
    /// Get packet send time (pacing)
    pub fn get_packet_send_time(&self, now: Instant) -> Instant {
        // Ask CC algorithm for pacing time
        self.cc_ops.get_packet_send_time(now)
    }
}
```

### 5.2 Pacing in Connection

```rust
// From quiche/src/lib.rs
impl Connection {
    pub fn send(&mut self, out: &mut [u8]) -> Result<(usize, SendInfo)> {
        // Check pacing
        let now = Instant::now();
        let pacing_time = self.recovery.get_packet_send_time(now);

        if pacing_time > now {
            // Not time to send yet
            return Err(Error::Done);
        }

        // Generate packet
        let (written, send_info) = self.send_packet(out, now)?;

        // Update pacing for next packet
        self.next_pacing_time = self.recovery.get_packet_send_time(now);

        Ok((written, SendInfo {
            to: send_info.to,
            from: send_info.from,
            at: self.next_pacing_time,  // Pacing hint
        }))
    }
}
```

---

## 6. HyStart++

### 6.1 Slow-Start Exit

HyStart++ provides early exit from slow start based on RTT increase:

```rust
// From quiche/src/recovery/congestion/hystart.rs
pub struct HyStart {
    /// Whether HyStart is enabled
    enabled: bool,

    /// Current round trip number
    round: usize,

    /// Minimum RTT in current round
    round_min_rtt: Option<Duration>,

    /// RTT threshold for early exit
    rtt_threshold: Duration,

    /// Whether we've exited slow start
    slow_start_exit: bool,
}

impl HyStart {
    /// Check if should exit slow start
    pub fn should_exit_slow_start(
        &mut self,
        rtt: Duration,
    ) -> bool {
        if self.slow_start_exit {
            return true;
        }

        // Update round min RTT
        match self.round_min_rtt {
            None => self.round_min_rtt = Some(rtt),
            Some(min) => {
                if rtt < min {
                    self.round_min_rtt = Some(rtt);
                }
            }
        }

        // Check RTT increase
        if let Some(min_rtt) = self.round_min_rtt {
            if rtt > min_rtt + self.rtt_threshold {
                // RTT increased significantly - possible congestion
                self.slow_start_exit = true;
                return true;
            }
        }

        false
    }

    /// Start new round
    pub fn start_round(&mut self) {
        self.round += 1;
        self.round_min_rtt = None;
    }
}
```

---

## 7. Proportional Rate Reduction

### 7.1 PRR Algorithm

PRR controls rate reduction during congestion:

```rust
// From quiche/src/recovery/congestion/prr.rs
pub struct Prr {
    /// Number of packets delivered since congestion event
    deliver_count: usize,

    /// Target number of packets to send
    send_target: usize,

    /// Number of packets already sent
    send_count: usize,
}

impl Prr {
    /// Update after congestion event
    pub fn on_congestion_event(
        &mut self,
        cwnd_before: usize,
        cwnd_after: usize,
        bytes_in_flight: usize,
    ) {
        // Calculate how many packets we should send
        // to reduce cwnd proportionally
        self.send_target = cwnd_after;
        self.deliver_count = 0;
        self.send_count = 0;
    }

    /// Called when packet is ACKed during recovery
    pub fn on_packet_acked(&mut self) {
        self.deliver_count += 1;
    }

    /// Check if can send packet
    pub fn can_send(&self) -> bool {
        self.send_count < self.send_target
    }

    /// Record packet sent
    pub fn on_packet_sent(&mut self) {
        self.send_count += 1;
    }
}
```

---

## Summary

### Key Takeaways

1. **Loss detection** - Packet threshold (3) and time threshold (1.125x RTT)
2. **Recovery architecture** - enum_dispatch for zero-cost CC polymorphism
3. **Cubic** - Cubic window function, multiplicative decrease, AIMD
4. **BBR2** - Model-based CC with Startup/Drain/ProbeBW/ProbeRTT phases
5. **Pacing** - Token bucket pacer with ReleaseTime hints
6. **HyStart++** - RTT-based slow-start exit detection
7. **PRR** - Proportional rate reduction during recovery

### Next Steps

Continue to [rust-revision.md](rust-revision.md) for:
- Zero-copy buffer design
- Intrusive collections for performance
- FFI boundaries
- Replication patterns for ewe_platform

---

## Further Reading

- [RFC 9002 - QUIC Loss Detection](https://www.rfc-editor.org/rfc/rfc9002.html)
- [RFC 8312 - CUBIC](https://www.rfc-editor.org/rfc/rfc8312.html)
- [BBR: Congestion-Based Congestion Control](https://queue.acm.org/detail.cfm?id=3022184)
- [quiche source - recovery/mod.rs](quiche/src/recovery/mod.rs)
- [quiche source - recovery/gcongestion/](quiche/src/recovery/gcongestion/)
