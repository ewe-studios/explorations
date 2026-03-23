---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpaceJam
explored_at: 2026-03-23
---

# SpaceJam Exploration

## Overview

SpaceJam is a collection of high-performance, systems-level projects primarily authored by Tyler Neely (spacejam) and Jon Gjengset (jonhoo). The projects focus on:

1. **Lock-free concurrency primitives** for Rust
2. **High-performance I/O** using Linux io_uring
3. **Distributed systems** (KV stores, consensus algorithms)
4. **Event-driven architectures** (Event Gateway)
5. **Stream processing utilities**

The codebase represents cutting-edge Rust systems programming with emphasis on performance, safety guarantees, and novel concurrency patterns.

## Sub-Projects

| Project | Description | Language |
|---------|-------------|----------|
| `bus` | Lock-free SPSC broadcast channel | Rust |
| `left-right` | High-concurrency read copy-on-write primitive | Rust |
| `stream-cancel` | Async stream interruption utilities | Rust |
| `rio` | io_uring bindings for Linux async I/O | Rust |
| `rasputin` | Distributed linearizable KV store | Rust |
| `event-gateway` | Event-driven FaaS routing proxy | Go |
| `tikv` | Distributed transactional KV database | Rust |
| `minuteman` | Metrics collection system | Go |
| `loghisto` | Logarithmic histograms for timeseries | Go |
| `src.komora-io` | Various experimental projects (sled, crdt, etc.) | Rust |
| `src.GlueSQL` | SQL database engine | Rust |
| `elements-of-rust` | Rust programming patterns guide | Documentation |

## Architecture

### Concurrency Primitive Stack

```
┌─────────────────────────────────────────────────────────────────┐
│                    Application Layer                            │
├─────────────────────────────────────────────────────────────────┤
│  stream-cancel (async stream control)                          │
│  left-right (lock-free read copies)                            │
│  bus (broadcast channels)                                      │
├─────────────────────────────────────────────────────────────────┤
│  rio (io_uring syscalls)                                       │
├─────────────────────────────────────────────────────────────────┤
│                    Linux Kernel (io_uring)                      │
└─────────────────────────────────────────────────────────────────┘
```

### Event Gateway Architecture

```
┌──────────────────┐
│     Client       │
└────────┬─────────┘
         │ Event
         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Event Gateway Cluster                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │   Events    │  │   Config    │  │  Function   │             │
│  │     API     │  │     API     │  │  Discovery  │             │
│  │   (:4000)   │  │   (:4001)   │  │             │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
┌────────────────┐  ┌────────────────┐  ┌────────────────┐
│   AWS Lambda   │  │  HTTP Endpoint │  │  Kinesis/SQS   │
└────────────────┘  └────────────────┘  └────────────────┘
```

## Key Design Patterns

### 1. Operational Log (OpLog) Pattern - left-right

The left-right crate uses an operational log to keep two copies of a data structure in sync:

- **Write side**: Accepts new operations, appends to oplog
- **Read side**: Servicing all concurrent readers
- **Publish**: Atomically swaps pointers, waits for readers to drain, replays oplog

```rust
// Pattern from left-right
impl Absorb<CounterAddOp> for i32 {
    fn absorb_first(&mut self, operation: &mut CounterAddOp, _: &Self) {
        *self += operation.0;  // Apply to write copy
    }
    fn absorb_second(&mut self, operation: CounterAddOp, _: &Self) {
        *self += operation.0;  // Apply to read copy after swap
    }
    fn sync_with(&mut self, first: &Self) {
        *self = *first;  // Initial sync
    }
}
```

### 2. Completion-based I/O - rio

rio uses Rust's type system to prevent use-after-free bugs with io_uring:

- `Completion` type borrows buffers for the lifetime of the I/O operation
- Destructor blocks until operation completes before freeing resources
- Type-level guarantees for mutable vs immutable buffers

```rust
// rio prevents this from compiling:
let completion = rio.write_at(&file, &buffer, 0);
drop(buffer);  // Compilation error - buffer lifetime tied to completion
completion.wait();
```

### 3. Broadcast Channel with Atomic Reference Counting - bus

The bus crate implements a single-producer, multi-consumer broadcast channel:

- Circular buffer with atomic seat management
- Each seat tracks reader count via `AtomicUsize`
- Last reader moves value instead of cloning (optimization)
- Uses `SpinWait` for low-latency blocking

```rust
// Seat structure from bus
struct Seat<T> {
    read: atomic::AtomicUsize,      // Track reader progress
    state: MutSeatState<T>,         // Actual data
    waiting: AtomicOption<thread::Thread>,  // Blocked writer
}
```

### 4. Tripwire/Valve Pattern - stream-cancel

For graceful shutdown of async streams:

```rust
// Tripwire pattern
let (trigger, tripwire) = Tripwire::new();
let stream = listener.take_until_if(tripwire);
drop(trigger);  // Stream yields None and terminates
```

## Performance Characteristics

### left-right

- **Reads**: Wait-free, scale linearly with CPU cores
- **Writes**: Slower due to oplog + dual application
- **Memory**: 2x data structure + oplog overhead
- **Cache behavior**: Two cache line invalidations per publish

### rio (io_uring)

- **Syscall reduction**: 0-syscall submission with SQPOLL
- **Batching**: Hundreds of operations in single syscall
- **O_DIRECT performance**: 300%+ improvement over thread pools
- **Completion polling**: Configurable for latency trading

### bus

- **Lock-free**: Uses atomics instead of mutexes
- **Broadcast**: All consumers see all messages
- **Busy-waiting**: Known issue causing CPU usage spikes

## WASM Usage

None of the SpaceJam projects directly target WASM. The projects are focused on:
- Linux-specific syscalls (io_uring)
- High-performance systems programming
- Distributed databases requiring OS-level I/O

However, `stream-cancel` could be used in WASM contexts as it only depends on tokio and futures-rs.

## Configuration and Environment

### rio

```rust
// Requires Linux 5.1+ for io_uring
let config = Config::default()
    .entries(1024)  // Ring buffer size
    .start()?;
```

### event-gateway

```yaml
# Requires etcd/zookeeper/consul for state
ports:
  events: 4000
  config: 4001
backing_store: etcd://localhost:2379
```

## Dependencies

### Rust Projects

| Crate | Key Dependencies |
|-------|-----------------|
| bus | `parking_lot_core`, `crossbeam-channel`, `num_cpus` |
| left-right | `slab`, `loom` (testing) |
| stream-cancel | `tokio`, `futures-core`, `pin-project` |
| rio | `libc` (Linux only) |

### Go Projects (event-gateway)

- `libkv` - Abstraction for etcd/zookeeper/consul
- Custom event routing and function discovery

## Testing Strategy

### bus
- Uses `loom` for concurrency testing
- Property-based tests for broadcast semantics

### left-right
- Loom integration for model checking
- Slab-based epoch tracking for reader management

### rio
- O_DIRECT examples with aligned buffers
- TCP echo server benchmarks

### stream-cancel
- Tokio runtime tests
- Multi-listener shutdown scenarios

## Entry Points

### bus
```rust
use bus::Bus;
let mut bus = Bus::new(10);
let mut rx = bus.add_rx();
bus.broadcast("message");
```

### left-right
```rust
let (write, read) = left_right::new::<T, Op>();
write.append(Op::Insert(key, value));
write.publish();  // Make visible to readers
```

### rio
```rust
let ring = rio::new()?;
let completion = ring.read_at(&file, &mut buf, offset);
completion.await?;  // or .wait() for blocking
```

### stream-cancel
```rust
let (exit, valve) = Valve::new();
let stream = valve.wrap(TcpListenerStream::new(listener));
drop(exit);  // Graceful shutdown
```

## Related Projects

- **sled**: Tyler Neely's embedded database (uses rio)
- **evmap**: Jon Gjengset's event-driven map (uses left-right)
- **TiKV**: Production distributed KV store (CNCF graduated)
