---
title: Network Report — Probes, NAT Detection, and Relay Selection
---

# Network Report — Probes, NAT Detection, and Relay Selection

The `net_report::Client` determines what your network looks like: which relays are reachable, whether you're behind NAT, and which relay has the lowest latency.

## What net_report Does

```
┌─────────────────────────────────────────────────┐
│              net_report::Client                  │
│                                                 │
│  Probes ──▶ Report ──▶ Preferred Relay          │
│  (HTTPS,  │    │        Selection              │
│   QAD)    │    │        with Hysteresis        │
│           │    ▼                               │
│           │  History of past reports            │
│           │  (avoid flapping)                   │
└─────────────────────────────────────────────────┘
```

Source: `iroh/src/net_report.rs:1`

## The Report Structure

```rust
// iroh/src/net_report/report.rs
pub struct Report {
    /// Can we send UDP via IPv4?
    pub udp_v4: bool,
    /// Can we send UDP via IPv6?
    pub udp_v6: bool,
    /// Does our port mapping vary by destination?
    pub mapping_varies_by_dest_ipv4: Option<bool>,
    pub mapping_varies_by_dest_ipv6: Option<bool>,
    /// The preferred relay server (lowest latency).
    pub preferred_relay: Option<RelayUrl>,
    /// Per-relay latency measurements.
    pub relay_latency: RelayLatencies,
    /// Our observed global IPv4 address.
    pub global_v4: Option<SocketAddrV4>,
    /// Our observed global IPv6 address.
    pub global_v6: Option<SocketAddrV6>,
    /// Have we detected a captive portal?
    pub captive_portal: Option<bool>,
}
```

Source: `iroh/src/net_report/report.rs:1` — The `Report` struct. Note: there are NO `ipv4_can_send`, `ipv6_can_send`, `os_has_ipv4`, `os_has_ipv6`, `global_ipv4`, `global_ipv6`, or `mapping_var` fields — the actual field names differ.

## Probe Types

```rust
// iroh/src/net_report/probes.rs
pub enum Probe {
    /// HTTPS probe to a relay (measures HTTP latency).
    Https { url: RelayUrl, delay: Duration },
    /// QUIC Address Discovery probe IPv4.
    QadIpv4 { url: RelayUrl, stun_port: u16 },
    /// QUIC Address Discovery probe IPv6.
    QadIpv6 { url: RelayUrl, stun_port: u16 },
}
```

Source: `iroh/src/net_report/probes.rs` — Three probe types: HTTPS (relay latency), QAD IPv4 (NAT detection + address discovery), QAD IPv6 (same for IPv6).

## ProbePlan: Scheduling

```rust
// iroh/src/net_report/probes.rs
pub struct ProbePlan {
    initial: ProbeSet,     // First round of probes
    follow_up: ProbeSet,   // Retries if initial probes fail
}
```

Probes are scheduled with increasing delays. Failed probes are retried with backoff.

## Report Generation: The Actor

```rust
// iroh/src/net_report/reportgen.rs
struct Actor {
    // The report generation actor runs probes and builds the Report
}
```

The actor:
1. Spawns HTTPS probes to all known relays
2. Runs QAD probes for NAT type detection
3. Checks for captive portals (HTTP connectivity test)
4. Selects the preferred relay (lowest latency with hysteresis)
5. Returns the completed `Report`

Source: `iroh/src/net_report/reportgen.rs` — The actor manages probe execution with timeout and retry logic.

## RelayLatencies

```rust
// iroh/src/net_report/report.rs
pub struct RelayLatencies {
    /// IPv4 relay latencies.
    pub ipv4: BTreeMap<RelayUrl, Duration>,
    /// IPv6 relay latencies.
    pub ipv6: BTreeMap<RelayUrl, Duration>,
    /// HTTPS relay latencies.
    pub https: BTreeMap<RelayUrl, Duration>,
}
```

Source: `iroh/src/net_report/report.rs:1` — Three fields (ipv4, ipv6, https), not two.

## Hysteresis: Avoiding Relay Flapping

The preferred relay selection uses hysteresis — it won't switch to a new relay unless the new relay is significantly faster (by a margin). This prevents constant relay switching due to measurement noise.

Source: `iroh/src/net_report.rs` — The `Client` tracks report history and applies hysteresis.

## Full vs. Incremental Reports

| Report Type | When | What |
|------------|------|------|
| **Full** | Initial startup, network change | All probes, all relays, NAT detection |
| **Incremental** | Periodic updates | Relay latency refresh only |

Full reports are expensive (probe all relays). Incremental reports only measure relay latencies.

## The net_report Client API

```rust
// iroh/src/net_report.rs
pub struct Client { ... }

impl Client {
    /// Generate a full report.
    pub async fn get_report(&self, plan: ProbePlan) -> Result<Report> { ... }

    /// Generate an incremental report (relay latencies only).
    pub async fn get_report_incremental(&self) -> Result<Report> { ... }
}
```

Source: `iroh/src/net_report.rs:1`

## Default Probes and Timeouts

Source: `iroh/src/defaults.rs` — Default relay hostnames, QUIC port 443, HTTPS port 443, net_report probe timeouts.

## Related Documents

- [Endpoint](../markdown/02-endpoint.md) — How the endpoint uses net_report data
- [Relay Server](../markdown/08-iroh-relay.md) — Relay server that responds to probes
- [Socket Layer](../markdown/07-socket.md) — How path selection uses relay latency data
