---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpaceJam
revised_at: 2026-03-23
---

# SpaceJam Rust Revision Guide

## Overview

This guide provides a roadmap for reproducing SpaceJam's functionality in production-grade Rust. The projects covered represent state-of-the-art systems programming techniques.

## Project-by-Project Reproduction

### 1. Bus - Broadcast Channel

**Complexity:** Medium
**Production readiness:** High (already on crates.io)

#### Key Components to Reproduce

```rust
// Minimum viable implementation
pub struct Bus<T> {
    ring: Arc<Vec<Seat<T>>>,
    tail: AtomicUsize,
    readers: usize,
}

pub struct BusReader<T> {
    ring: Arc<Vec<Seat<T>>>,
    head: usize,
}

struct Seat<T> {
    read: AtomicUsize,
    value: UnsafeCell<Option<T>>,
}
```

#### Critical Implementation Details

1. **Circular buffer with fence slot**: Always keep one slot empty to distinguish full from empty
2. **Atomic reader tracking**: Each seat tracks how many readers have consumed it
3. **Last-reader optimization**: Last reader moves value instead of cloning

#### Dependencies

```toml
[dependencies]
parking_lot_core = "0.9"  # For SpinWait
crossbeam-channel = "0.5"  # For internal signaling
```

#### Production Considerations

```rust
// 1. Handle slow readers
impl<T> Bus<T> {
    /// Create with reader timeout to prevent head-of-line blocking
    pub fn with_timeout(len: usize, timeout: Duration) -> Self {
        // Track reader last-seen timestamps
        // Evict readers that fall behind
    }
}

// 2. Add metrics
pub struct BusMetrics {
    broadcasts: AtomicU64,
    clones: AtomicU64,      // Track cloning overhead
    drops: AtomicU64,       // Track dropped messages
    max_latency: AtomicU64, // Nanoseconds
}

// 3. Consider async variant
pub struct AsyncBus<T> {
    inner: Bus<T>,
    waker: AtomicWaker,
}
```

### 2. Left-Right - Lock-Free Read Copy

**Complexity:** High
**Production readiness:** High (already on crates.io)

#### Key Components to Reproduce

```rust
pub struct WriteHandle<T, O> {
    data: T,
    oplog: Vec<O>,
    epochs: Epochs,
    reader_ptr: AtomicPtr<T>,
}

pub struct ReadHandle<T> {
    data: Arc<T>,
    epoch: Arc<AtomicUsize>,
    epochs: Epochs,
}

pub trait Absorb<O> {
    fn absorb_first(&mut self, op: &mut O, other: &Self);
    fn absorb_second(&mut self, op: O, other: &Self);
    fn sync_with(&mut self, first: &Self);
}
```

#### Critical Implementation Details

1. **Epoch-based reader tracking**: Readers increment epoch on entry/exit
2. **OpLog replay**: After pointer swap, replay operations to stale copy
3. **Deterministic operations**: Operations must produce identical results

#### Production Considerations

```rust
// 1. OpLog compaction for memory efficiency
impl<T, O> WriteHandle<T, O>
where
    O: Compactable,
{
    pub fn compact(&mut self) {
        // Merge consecutive operations
        // e.g., Insert(k, v1) + Insert(k, v2) = Insert(k, v2)
        self.oplog = compact_ops(std::mem::take(&mut self.oplog));
    }
}

// 2. Support for multiple writers via Mutex
pub struct MultiWriter<T, O> {
    inner: Mutex<WriteHandle<T, O>>,
}

// 3. Add snapshot isolation
impl<T, O> ReadHandle<T> {
    pub fn snapshot(&self) -> Arc<T> {
        // Return Arc clone for long-lived reads
        // without blocking publish
    }
}
```

#### When NOT to Use Left-Right

```rust
// Bad: Non-deterministic operations
impl Absorb<TimestampedOp> for MyData {
    fn absorb_first(&mut self, op: &mut TimestampedOp, _: &Self) {
        // BAD: Uses current time - will differ between copies
        op.timestamp = SystemTime::now();
        self.apply(op);
    }
}

// Good: Capture timestamp before oplog
let timestamp = SystemTime::now();
write_handle.append(TimestampedOp { timestamp, data });
```

### 3. Rio - io_uring Bindings

**Complexity:** Very High
**Production readiness:** Medium (GPL license concerns)

#### Key Components to Reproduce

```rust
pub struct Rio {
    inner: Arc<Uring>,
}

struct Uring {
    sq: SubmissionQueue,
    cq: CompletionQueue,
    config: Config,
}

pub struct Completion {
    uring: Arc<Uring>,
    user_data: u64,
}

// Raw bindings (Linux only)
#[cfg(target_os = "linux")]
mod io_uring {
    #[repr(C)]
    pub struct io_uring_sqe { /* ... */ }

    #[repr(C)]
    pub struct io_uring_cqe { /* ... */ }
}
```

#### Critical Implementation Details

1. **Memory-mapped queues**: SQ/CQ are mmap'd shared memory with kernel
2. **Lifetime tying**: Completion borrows buffers via PhantomData
3. **Backpressure**: Prevent completion queue overflow

#### Alternative: Use Existing Crates

Instead of reproducing rio, consider:

```toml
[dependencies]
# Permissive license (MIT/Apache)
io-uring = "0.6"

# Tokio integration
tokio-uring = "0.4"

# Lower-level control
iou = "0.3"
```

#### Production Considerations

```rust
// 1. Add timeout support
impl Rio {
    pub fn read_at_with_timeout(
        &self,
        file: &File,
        buf: &mut [u8],
        offset: u64,
        timeout: Duration,
    ) -> io::Result<Completion> {
        // Use IORING_OP_TIMEOUT with link
    }
}

// 2. Add buffered writer
pub struct BufferedRioWriter {
    rio: Rio,
    file: File,
    buffer: Vec<u8>,
    position: u64,
}

impl AsyncWrite for BufferedRioWriter {
    // Batch small writes into larger io_uring submissions
}

// 3. Error recovery
impl Completion {
    pub fn wait_with_retry(self, max_retries: u32) -> io::Result<usize> {
        let mut retries = 0;
        loop {
            match self.wait() {
                Err(e) if e.kind() == io::ErrorKind::Interrupted && retries < max_retries => {
                    retries += 1;
                    continue;
                }
                result => return result,
            }
        }
    }
}
```

### 4. Stream-Cancel - Async Stream Control

**Complexity:** Low-Medium
**Production readiness:** High (already on crates.io)

#### Key Components to Reproduce

```rust
pub struct Trigger(Option<watch::Sender<bool>>);

pub struct Valve {
    trigger: Trigger,
    tripwire: watch::Receiver<bool>,
}

pub struct Valved<S> {
    stream: S,
    cancel: watch::Receiver<bool>,
}
```

#### Implementation

```rust
impl<S: Stream> Stream for Valved<S> {
    type Item = S::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // First check if cancelled
        if *self.cancel.borrow() {
            return Poll::Ready(None);
        }

        // Register waker for cancellation
        let mut cancel_recv = Pin::new(&mut self.cancel);
        if cancel_recv.as_mut().poll(cx).is_ready() {
            return Poll::Ready(None);
        }

        // Poll inner stream
        Pin::new(&mut self.stream)
            .poll_next(cx)
    }
}
```

#### Production Enhancements

```rust
// 1. Add timeout-based cancellation
pub struct TimedValve<S> {
    inner: Valved<S>,
    deadline: Instant,
}

impl<S: Stream> Stream for TimedValve<S> {
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if Instant::now() > self.deadline {
            return Poll::Ready(None);
        }
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

// 2. Add graceful drain period
pub struct DrainingValve<S> {
    stream: S,
    cancel: watch::Receiver<bool>,
    drain_timeout: Option<Instant>,
}
```

## Architecture Recommendations

### Combined System Design

For a production system using all components:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Left-Right (state)                      │   │
│  │  ┌──────────┐         ┌──────────┐                  │   │
│  │  │ Read     │         │ Write    │                  │   │
│  │  │ Handle   │         │ Handle   │                  │   │
│  │  └──────────┘         └──────────┘                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Stream-Cancel (graceful shutdown)       │   │
│  │  ┌──────────────┐  ┌──────────────┐                │   │
│  │  │ Valved TCP   │  │ Valved TCP   │                │   │
│  │  └──────────────┘  └──────────────┘                │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                  │
│                          ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Rio (async I/O)                         │   │
│  │  ┌──────────────────────────────────────────────┐   │   │
│  │  │  io_uring SQ/CQ                               │   │   │
│  │  └──────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Error Handling Strategy

```rust
// Unified error type
#[derive(Debug, Error)]
pub enum SpaceJamError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Timeout exceeded")]
    Timeout,

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Lock poisoned")]
    LockPoisoned,
}

pub type Result<T> = std::result::Result<T, SpaceJamError>;
```

### Testing Strategy

```rust
// 1. Loom for concurrency testing
#[cfg(test)]
mod tests {
    use loom::thread;

    #[test]
    #[cfg(loom)]
    fn test_bus_broadcast() {
        loom::model(|| {
            let mut bus = Bus::new(10);
            let mut rx1 = bus.add_rx();
            let mut rx2 = bus.add_rx();

            bus.broadcast(42);

            let t1 = thread::spawn(move || rx1.recv());
            let t2 = thread::spawn(move || rx2.recv());

            assert_eq!(t1.join().unwrap(), Ok(42));
            assert_eq!(t2.join().unwrap(), Ok(42));
        });
    }
}

// 2. Property-based testing with proptest
use proptest::prelude::*;

proptest! {
    #[test]
    fn bus_round_trip(items: Vec<u32>) {
        let mut bus = Bus::new(items.len() + 10);
        let mut rx = bus.add_rx();

        for item in &items {
            bus.broadcast(*item);
        }

        for expected in &items {
            prop_assert_eq!(rx.recv(), Ok(*expected));
        }
    }
}

// 3. Integration tests with timeouts
#[tokio::test]
async fn test_graceful_shutdown() {
    let (exit, valve) = Valve::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let handle = tokio::spawn(async move {
        let mut incoming = valve.wrap(TcpListenerStream::new(listener));
        while let Some(_stream) = incoming.next().await {
            // Handle connections
        }
    });

    drop(exit);

    // Should complete within 1 second
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("Shutdown timed out")
        .expect("Task panicked");
}
```

## Performance Tuning

### Benchmarking Setup

```rust
// Cargo.toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "bus_bench"
harness = false
```

```rust
// benches/bus_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bus::Bus;

fn bus_broadcast_benchmark(c: &mut Criterion) {
    c.bench_function("bus_broadcast_100", |b| {
        b.iter(|| {
            let mut bus = Bus::new(100);
            let mut rx = bus.add_rx();

            for i in 0..100 {
                bus.broadcast(i);
                black_box(rx.recv().unwrap());
            }
        })
    });
}

criterion_group!(benches, bus_broadcast_benchmark);
criterion_main!(benches);
```

### Optimization Techniques

```rust
// 1. Cache-line alignment for atomics
use std::mem::size_of;

const CACHE_LINE: usize = 64;

#[repr(align(64))]
struct PaddedAtomicUsize(AtomicUsize, [u8; CACHE_LINE - size_of::<AtomicUsize>()]);

// 2. Batch operations
impl<T, O> WriteHandle<T, O> {
    pub fn append_batch(&mut self, ops: impl Iterator<Item = O>) {
        self.oplog.extend(ops);
        // Single publish for batch
    }
}

// 3. Thread-local buffering
thread_local! {
    static LOCAL_OPLOG: RefCell<Vec<Op>> = RefCell::new(Vec::with_capacity(64));
}

fn flush_local_oplog(handle: &mut WriteHandle) {
    LOCAL_OPLOG.with(|cell| {
        let ops = cell.take();
        handle.append_batch(ops.into_iter());
    });
}
```

## License Considerations

| Project | License | Production Use |
|---------|---------|----------------|
| bus | MIT/Apache-2.0 | ✅ Safe |
| left-right | MIT/Apache-2.0 | ✅ Safe |
| stream-cancel | MIT/Apache-2.0 | ✅ Safe |
| rio | GPL-3.0 | ⚠️ Requires open sourcing |

For rio, consider using `io-uring` crate (MIT/Apache) instead.

## Summary Checklist

For production reproduction:

- [ ] **Bus**: Add slow reader detection, metrics, async variant
- [ ] **Left-Right**: Add oplog compaction, snapshot support
- [ ] **Rio**: Use io-uring crate instead, add timeout support
- [ ] **Stream-Cancel**: Add drain timeout, cleanup hooks
- [ ] **Testing**: Loom tests, property tests, integration tests
- [ ] **Docs**: Clear examples, performance characteristics
- [ ] **Benchmarks**: Criterion benchmarks for regression detection
