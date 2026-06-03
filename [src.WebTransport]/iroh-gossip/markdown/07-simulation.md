---
title: Simulation — Discrete Event Simulation Framework
---

# Simulation — Discrete Event Simulation Framework

Iroh-gossip includes a discrete event simulation framework for testing the protocol at scale — running 1000+ node networks in a single process.

## Why Simulation

Testing P2P protocols with real networking is slow, non-deterministic, and limited to small networks. The simulation framework runs the IO-less `proto/` state machines directly with a simulated network layer.

```
┌─────────────────────────────────────────────────┐
│              Simulator                          │
│                                                 │
│  TimedEventQueue ──▶ Network<PI, R>            │
│  (sorted by time)     ├── peer 0: State        │
│                       ├── peer 1: State        │
│                       ├── peer 2: State        │
│                       ├── ...                  │
│                       └── peer N: State        │
│                                                 │
│  Each tick: pop next event, deliver to peer    │
│  Peer processes, produces OutEvents             │
│  OutEvents become new timed events              │
└─────────────────────────────────────────────────┘
```

Source: `iroh-gossip/src/proto/sim.rs:1` — `Simulator` runs a discrete event simulation.

## Network Configuration

```rust
// iroh-gossip/src/proto/sim.rs
pub struct NetworkConfig {
    /// Number of peers in the simulation.
    pub num_peers: usize,
    /// Bootstrap mode (first peer, all-to-all, etc.).
    pub bootstrap: BootstrapMode,
    /// Latency configuration.
    pub latency: LatencyConfig,
    /// Probability of message loss.
    pub drop_rate: f64,
    /// Probability of peer churn.
    pub churn_rate: f64,
}
```

Source: `iroh-gossip/src/proto/sim.rs:1` — Network configuration for simulation scenarios.

## TimedEventQueue

```rust
// iroh-gossip/src/proto/sim.rs
struct TimedEventQueue {
    events: BinaryHeap<Reverse<(Duration, Event)>>,
}
```

Events are sorted by simulation time. The simulator pops the next event, delivers it to the target peer, and any resulting OutEvents become new timed events pushed back into the queue.

Source: `iroh-gossip/src/proto/sim.rs:1` — `TimedEventQueue` drives the simulation forward.

## Bootstrap Modes

| Mode | Description |
|------|-------------|
| `FirstPeer` | First peer is the bootstrap target |
| `RandomPeer` | Random peer is the bootstrap target |
| `AllToAll` | Every peer knows every other peer |

Source: `iroh-gossip/src/proto/sim.rs:1` — `BootstrapMode` enum.

## Simulation Metrics

After each simulation round, the framework collects statistics:

```rust
// iroh-gossip/src/proto/sim.rs
pub struct RoundStats {
    /// Total messages sent.
    pub total_messages: usize,
    /// Control messages sent.
    pub ctrl_messages: usize,
    /// Data messages sent.
    pub data_messages: usize,
    /// Peers that received the broadcast.
    pub reached_peers: usize,
    /// Round-trip message count.
    pub rmr: f64,
    /// Largest duplicate header ratio.
    pub ldh: f64,
}
```

Source: `iroh-gossip/src/proto/sim.rs:1` — `RoundStats` captures per-round metrics.

## CLI Simulation Binary

```bash
# Run a single-topic gossip simulation
cargo run --features simulator -- sim run --config sim.toml

# Compare two simulation configurations
cargo run --features simulator -- sim compare --dir-a dir1 --dir-b dir2
```

Source: `iroh-gossip/src/bin/sim.rs:1` — CLI with `Run` and `Compare` commands.

## Scenarios

| Scenario | Peers | Description |
|----------|-------|-------------|
| `BigSingle` | 1000 | Single sender broadcasts to 999 receivers |
| `BigMulti` | 1000 | All peers broadcast simultaneously |
| `BigAll` | 1000 | Full mesh with all peers sending and receiving |

Source: `iroh-gossip/src/bin/sim.rs:1` — Scenario definitions.

## Simulation Results

```
Round Stats:
  Total messages:     12,345
  Control messages:   8,901
  Data messages:      3,444
  Reached peers:      999/1000
  RMR:                2.34  (ideal: 1.0)
  LDH:                0.05  (ideal: 0.0)
```

Source: `iroh-gossip/src/bin/sim.rs:1` — Result printing with `RoundStatsDiff` and `NetworkHistograms`.

## Integration Tests

```rust
// iroh-gossip/tests/sim.rs
#[test]
fn big_hyparview() { ... }

#[test]
fn big_multiple_sender() { ... }

#[test]
fn big_single_sender() { ... }

#[test]
fn big_burst() { ... }
```

Source: `iroh-gossip/tests/sim.rs:1` — Integration tests running large-scale simulations with assertions on LDH (largest duplicate header ratio), RMR (relative message redundancy), and missed messages.

**Aha:** The simulation framework's existence is only possible because of the IO-less `proto/` design. The protocol state machine has no awareness of whether it's running on real network connections or a discrete event simulator — it just processes InEvents and produces OutEvents. This is the primary benefit of the architectural separation.

## Related Documents

- [Architecture](../markdown/01-architecture.md) — IO-less state machine design
- [Protocol State](../markdown/04-topic-state.md) — The state machine being simulated
