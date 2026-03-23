---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpaceJam/bus
revised_at: 2026-03-23
---

# Bus and Stream-Cancel Deep Dive

## Bus - Lock-Free Broadcast Channel

### Purpose

Bus provides a **lock-free, bounded, single-producer, multi-consumer broadcast channel**. Every message sent is received by all consumers.

### Architecture

```
                    ┌─────────────┐
                    │   Producer  │
                    │   (single)  │
                    └──────┬──────┘
                           │ broadcast()
                           ▼
              ┌────────────────────────┐
              │   Circular Buffer      │
              │   ┌───┬───┬───┬───┐   │
              │   │ S │ S │ S │ S │   │  Seats
              │   └───┴───┴───┴───┘   │
              └────────────────────────┘
                   │    │    │
                   ▼    ▼    ▼
              ┌────────────────────┐
              │  Consumer 1 | 2 | 3│
              └────────────────────┘
```

### Seat Design

```rust
struct Seat<T> {
    /// Number of readers that have consumed this seat
    read: AtomicUsize,

    /// Actual data + max readers expected
    state: MutSeatState<SeatState<T>>,

    /// Blocked writer thread (if buffer full)
    waiting: AtomicOption<thread::Thread>,
}

struct SeatState<T> {
    max: usize,      // Number of readers at write time
    val: Option<T>,  // The actual value
}
```

### Key Invariants

1. **Producer writes to `tail + 1` only when fence seat is free**
2. **Readers access seats between their head and producer tail**
3. **Last reader moves value (no clone), others clone**

### Broadcast Implementation

```rust
fn broadcast_inner(&mut self, val: T, block: bool) -> Result<(), T> {
    let tail = self.state.tail.load(Ordering::Relaxed);
    let fence = (tail + 1) % self.state.len;  // One slot padding

    // Wait for fence seat to be free
    loop {
        let fence_read = self.state.ring[fence]
            .read
            .load(Ordering::Acquire);

        if fence_read == self.expected(fence) {
            break;  // Space available
        }

        if block {
            // Park and wait for readers
            self.state.ring[fence]
                .waiting
                .swap(Some(Box::new(thread::current())));
            thread::park_timeout(SPINTIME);
        } else {
            return Err(val);  // Non-blocking, return error
        }
    }

    // Write to current tail
    let readers = self.readers;
    let next = &self.state.ring[tail];
    let state = unsafe { &mut *next.state.get() };
    state.max = readers;
    state.val = Some(val);
    next.read.store(0, Ordering::Release);

    // Advance tail
    let tail = (tail + 1) % self.state.len;
    self.state.tail.store(tail, Ordering::Release);

    Ok(())
}
```

### Reader Implementation

```rust
impl<T: Clone + Sync> BusReader<T> {
    fn recv_inner(&mut self, block: RecvCondition) -> Result<T, RecvError> {
        loop {
            let tail = self.bus.tail.load(Ordering::Acquire);

            // Check if data available
            if tail != self.head {
                break;
            }

            // Check if closed
            if self.bus.closed.load(Ordering::Relaxed) {
                return Err(Disconnected);
            }

            // Block or return timeout
            if let RecvCondition::Try = block {
                return Err(Timeout);
            }

            // Tell writer we're waiting
            self.waiting.send((thread::current(), self.head));
            thread::park_timeout(SPINTIME);
        }

        // Take value from seat
        let head = self.head;
        let ret = self.bus.ring[head].take();
        self.head = (head + 1) % self.bus.len;
        Ok(ret)
    }
}

fn Seat::take(&self) -> T {
    let read = self.read.load(Ordering::Acquire);
    let state = unsafe { &*self.state.get() };

    // Last reader moves, others clone
    let v = if read + 1 == state.max {
        // We're the last - notify writer if waiting
        let waiting = self.waiting.take();
        unsafe { &mut *self.state.get() }.val.take().unwrap()
    } else {
        state.val.clone().expect("empty seat")
    };

    self.read.fetch_add(1, Ordering::AcqRel);

    if let Some(t) = waiting {
        t.unpark();  // Wake blocked writer
    }

    v
}
```

### Known Issue: Busy Waiting

Bus has a known issue (#23) where it busy-waits during contention:

```rust
let mut sw = SpinWait::new();
loop {
    if !sw.spin() {
        // Only park after spinning
        thread::park_timeout(spintime);
    }
}
```

This causes elevated CPU usage under high contention.

## Stream-Cancel - Async Stream Interruption

### Purpose

Provides mechanisms for **graceful shutdown of async streams**, particularly useful for TCP listeners and long-running stream processors.

### Two Patterns

#### 1. Tripwire Pattern (Stream Combinator)

```rust
pub trait StreamExt: Stream {
    fn take_until_if(self, future: impl Future<Output = bool>) -> TakeUntilIf<Self, Fut>;
}

// Usage
let (trigger, tripwire) = Tripwire::new();
let stream = tcp_listener.take_until_if(tripwire.clone());

// When trigger is dropped, tripwire resolves to true
// Stream immediately yields None
drop(trigger);
```

#### 2. Valve Pattern (Stream Wrapper)

```rust
pub struct Valve {
    tx: watch::Sender<bool>,
}

pub struct Valved<S> {
    stream: S,
    rx: watch::Receiver<bool>,
}

impl<S: Stream> Stream for Valved<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        // Check if cancelled
        if *self.rx.borrow() {
            return Poll::Ready(None);  // Stream terminated
        }

        // Watch for cancellation
        let rx = Pin::new(&mut self.rx);
        if rx.poll(cx).is_ready() {
            return Poll::Ready(None);
        }

        // Poll inner stream
        Pin::new(&mut self.stream).poll_next(cx)
    }
}
```

### Implementation Details

```rust
// Trigger uses watch channel for broadcast
pub struct Trigger(Option<watch::Sender<bool>>);

impl Drop for Trigger {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(true);  // Signal cancellation
        }
    }
}

// Valve can wrap multiple streams
pub struct Valve {
    trigger: Trigger,
    tripwire: Tripwire,
}

impl Valve {
    pub fn new() -> (Trigger, Valve) {
        let (tx, rx) = watch::channel(false);
        (Trigger(Some(tx)), Valve { tripwire: rx })
    }

    pub fn wrap<S: Stream>(&self, stream: S) -> Valved<S> {
        Valved {
            stream,
            rx: self.tripwire.clone(),
        }
    }
}
```

### Graceful Shutdown Example

```rust
#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (exit, valve) = Valve::new();

    // Spawn server
    tokio::spawn(async move {
        let mut incoming = valve.wrap(TcpListenerStream::new(listener));

        while let Some(stream) = incoming.next().await {
            tokio::spawn(handle_connection(stream));
        }

        // Stream yields None when valve closed
        println!("Server shut down gracefully");
    });

    // ... later ...
    drop(exit);  // All wrapped streams terminate
}
```

## Production Reproduction

### Bus Crate Structure

```
bus-reproduction/
├── Cargo.toml
└── src/
    ├── lib.rs       # Public API
    ├── seat.rs      # Seat, SeatState
    ├── atomic_option.rs  # AtomicOption<T>
    └── bus.rs       # Bus, BusReader
```

### Key Dependencies

```toml
[package]
name = "bus"
version = "2.4.1"

[dependencies]
parking_lot_core = "0.9"  # SpinWait
crossbeam-channel = "0.5" # Internal channels
num_cpus = "1.6.2"        # Thread count hints
```

### Stream-Cancel Dependencies

```toml
[package]
name = "stream-cancel"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["sync"] }  # watch channel
futures-core = "0.3.0"
pin-project = "1.0.0"  # Stream wrapper
```

## Performance Considerations

### Bus

| Metric | Value |
|--------|-------|
| Single consumer | Zero allocations after init |
| Multiple consumers | Clone per message per consumer |
| Contention | SpinWait then park |
| Memory | O(buffer_size * sizeof(T)) |

**Optimization: Use Arc for expensive clones**

```rust
let mut bus = Bus::new(100);
// Instead of cloning large data:
// bus.broadcast(large_data);

// Use Arc:
let shared = Arc::new(large_data);
bus.broadcast(shared);  // Arc clone is cheap
```

### Stream-Cancel

| Pattern | Overhead | Use Case |
|---------|----------|----------|
| Tripwire | One watch per stream | Per-stream control |
| Valve | One watch shared | Multiple streams |

## Comparison to Alternatives

### Bus vs Other Channels

| Channel | Producers | Consumers | Pattern |
|---------|-----------|-----------|---------|
| `std::mpsc` | Multi | Single | Point-to-point |
| `crossbeam-channel` | Multi | Multi | Point-to-point |
| `tokio::broadcast` | Multi | Multi | Broadcast |
| `bus` | Single | Multi | Broadcast, lock-free |

### Stream-Cancel vs Alternatives

| Approach | Granularity | Cleanup |
|----------|-------------|---------|
| `CancellationToken` | Manual check | User handles |
| `select!` with abort | Per-operation | Complex |
| `Valved` | Stream level | Automatic |
