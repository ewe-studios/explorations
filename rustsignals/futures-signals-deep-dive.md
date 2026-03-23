---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RustSignals/rust-signals
explored_at: 2026-03-23
---

# futures-signals Core Library Deep Dive

## Overview

`futures-signals` is a zero-cost FRP (Functional Reactive Programming) library built on top of Rust's futures crate. It provides signals - values that change over time with automatic notification of dependents.

**Crate Information:**
- **Name:** `futures-signals`
- **Version:** 0.3.34
- **License:** MIT
- **Edition:** 2018

## Core Types

### Mutable<T>

The primary mutable state container:

```rust
pub struct Mutable<A>(ReadOnlyMutable<A>);

pub struct ReadOnlyMutable<A>(Arc<MutableState<A>>);

struct MutableState<A> {
    senders: AtomicUsize,
    lock: RwLock<MutableLockState<A>>,
}

struct MutableLockState<A> {
    value: A,
    signals: Vec<Weak<ChangedWaker>>,
}
```

**Key Methods:**

| Method | Description |
|--------|-------------|
| `new(value)` | Create new Mutable |
| `set(value)` | Set new value, notify all signals |
| `set_neq(value)` | Set only if different (requires PartialEq) |
| `lock_mut()` | Get mutable lock |
| `replace(value)` | Replace value, return old |
| `swap(&other)` | Swap values between two Mutables |
| `signal()` | Create signal (Copy types) |
| `signal_cloned()` | Create signal (Clone types) |
| `signal_ref(f)` | Create signal with transform |

**Internal Mechanics:**

1. **Reference Counting:** Uses `Arc` for shared ownership, `AtomicUsize` tracks sender count
2. **Change Detection:** `ChangedWaker` tracks if value changed and stores waker
3. **Notification:** On lock drop, if mutated, calls `wake()` on all weak wakers
4. **GC:** Dead wakers (dropped signals) are removed during notification

```rust
// Simplified notification flow
impl<A> Drop for MutableLockMut<'a, A> {
    fn drop(&mut self) {
        if self.mutated {
            self.lock.notify(true);  // Wake all subscribers
        }
    }
}
```

### Signal Trait

The core signal trait:

```rust
#[must_use = "Signals do nothing unless polled"]
pub trait Signal {
    type Item;
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>;
}
```

**Signal Contract:**
1. First poll MUST return `Poll::Ready(Some(value))` - signals always have current value
2. Can return `Poll::Ready(None)` to indicate signal has ended (no more values)
3. Returns `Poll::Pending` if unchanged (waker registered for future notification)

### MutableSignal Implementation

```rust
pub struct MutableSignal<A>(MutableSignalState<A>);

struct MutableSignalState<A> {
    waker: Arc<ChangedWaker>,
    state: Arc<MutableState<A>>,
}

impl<A> Signal for MutableSignal<A> where A: Copy {
    type Item = A;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<A>> {
        // Check if value changed since last poll
        if self.waker.is_changed() {
            let value = self.state.lock.read().unwrap().value;
            Poll::Ready(Some(value))
        } else if self.state.senders.load() == 0 {
            // All Mutables dropped, signal ended
            Poll::Ready(None)
        } else {
            // Register waker for future notification
            self.waker.set_waker(cx);
            Poll::Pending
        }
    }
}
```

## SignalVec - Reactive Collections

### MutableVec

```rust
pub struct MutableVec<A> {
    // Internal state with Vec<A> + change queue
}

pub struct MutableVecLockMut<'a, A> {
    // Exclusive mutable access with change tracking
}
```

**VecDiff - Change Types:**

```rust
pub enum VecDiff<A> {
    Replace { values: Vec<A> },    // Initial or full replacement
    InsertAt { index: usize, value: A },
    UpdateAt { index: usize, value: A },
    RemoveAt { index: usize },
    Move { old_index: usize, new_index: usize },
    Push { value: A },
    Pop {},
    Clear {},
}
```

**Implementation Notes:**

1. **Change Queue:** MutableVec maintains a queue of pending changes
2. **Batch Notification:** Multiple changes can be batched before notification
3. **No-op Detection:** `retain` with no removals doesn't notify
4. **Order Guarantee:** Changes delivered in application order

```rust
// Example: push implementation
pub fn push(&mut self, value: A) {
    self.vec.push(value);
    self.queue_change(VecDiff::Push { value });
    self.notify();
}
```

### SignalVec Transformations

**map():**

```rust
pub struct Map<S, F> {
    signal: S,
    callback: F,
}

impl<S, F, A> SignalVec for Map<S, F>
where
    S: SignalVec,
    F: FnMut(S::Item) -> A,
{
    type Item = A;

    fn poll_vec_change(self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Option<VecDiff<A>>>
    {
        let this = self.project();
        match this.signal.poll_vec_change(cx)? {
            VecDiff::Push { value } =>
                Poll::Ready(Some(VecDiff::Push {
                    value: (this.callback)(value)
                })),
            // ... handle all variants
        }
    }
}
```

**filter():**

```rust
pub struct Filter<S, F> {
    indexes: Vec<usize>,  // Track which indices pass filter
    signal: S,
    callback: F,
}
```

The filter maintains an index map of items that pass the predicate. When the underlying SignalVec changes, it updates the index map and emits appropriate VecDiff for the filtered output.

## SignalMap - Reactive Key-Value Collections

### MutableBTreeMap

```rust
pub struct MutableBTreeMap<K, V> {
    // Internally uses BTreeMap<K, V> + change queue
}
```

### MapDiff

```rust
pub enum MapDiff<K, V> {
    Replace { entries: Vec<(K, V)> },
    Insert { key: K, value: V },
    Update { key: K, value: V },
    Remove { key: K },
    Clear {},
}
```

## Broadcaster - Signal Multiplexer

Splits a single Signal into multiple independent Signals:

```rust
pub struct Broadcaster<A>
where A: Signal
{
    shared_state: Arc<BroadcasterSharedState<A>>,
}

struct BroadcasterSharedState<A> {
    inner: RwLock<BroadcasterInnerState<A>>,
    notifier: Arc<BroadcasterNotifier>,
}

struct BroadcasterInnerState<A> {
    signal: Option<Pin<Box<A>>>,  // Original signal
    waker: Waker,
    value: Option<A::Item>,       // Cached value
    epoch: usize,                 // Change counter
}
```

**How it works:**
1. Polls input signal in background via waker
2. Stores most recent value with epoch counter
3. Each output signal tracks its own epoch
4. When epoch differs, returns cached value

```rust
pub fn signal(&self) -> BroadcasterSignal<A>
where A::Item: Copy
{
    BroadcasterSignal {
        state: BroadcasterState::new(&self.shared_state),
    }
}
```

## Channel - Single-Producer Signal Channel

```rust
pub fn channel<A>(initial_value: A) -> (Sender<A>, Receiver<A>)
```

**Implementation:**
- Uses `AtomicOption` for lock-free value passing
- `Sender` holds `Weak<Inner>`, `Receiver` holds `Arc<Inner>`
- When all Senders drop, Receiver gets `Poll::Ready(None)`

```rust
struct Inner<A> {
    value: AtomicOption<A>,  // Lock-free atomic pointer
    waker: AtomicOption<Waker>,
    senders: AtomicUsize,
}
```

## Macros

### map_ref!

Combines multiple signals into one:

```rust
let output = map_ref! {
    let a = signal1,
    let b = signal2,
    let c = signal3 =>
    *a + *b + *c
};
```

**Implementation:**
- Uses `gensym` to generate unique variable names
- Creates `MapRef1` struct for each input signal
- Polls all signals, only recomputes if any changed
- Uses mutable references internally (`map_mut!` variant)

## CancelableFuture

```rust
pub fn cancelable_future<A, B>(
    future: A,
    when_cancelled: B
) -> (DiscardOnDrop<CancelableFutureHandle>, CancelableFuture<A, B>)
where
    A: Future,
    B: FnOnce() -> A::Output,
```

**Use case:** When a signal changes before an async operation completes, cancel the old future and start a new one.

## Memory Management

### Weak References

Signals stored as `Weak<ChangedWaker>` in MutableState:
- Prevents reference cycles
- Dead signals automatically GC'd during notification

```rust
fn notify(&mut self, has_changed: bool) {
    self.signals.retain(|signal| {
        if let Some(signal) = signal.upgrade() {
            signal.wake(has_changed);
            true
        } else {
            false  // GC dead weak ref
        }
    });
}
```

### Drop Handling

```rust
impl<A> Drop for Mutable<A> {
    fn drop(&mut self) {
        let state = self.state();
        let old_senders = state.senders.fetch_sub(1, Ordering::SeqCst);

        if old_senders == 1 {
            // Last Mutable dropped
            let mut lock = state.lock.write().unwrap();
            lock.notify(false);  // Signal None to all subscribers
        }
    }
}
```

## Thread Safety

- `Mutable` implements `Send + Sync`
- Uses `RwLock` for value access
- Atomic operations for reference counting
- `SeqCst` ordering for consistency

## Feature Flags

| Feature | Description |
|---------|-------------|
| `default` | Enables debug + serde |
| `debug` | Enables log crate for debugging |
| `serde` | Serialize/Deserialize for Mutable |

## Testing

Comprehensive test coverage in `tests/`:
- `signal/mutable.rs` - Mutable operations
- `signal/map.rs` - map transformations
- `signal/eq.rs`, `signal/neq.rs` - Equality checks
- `signal_vec.rs` - SignalVec operations
- `signal_map.rs` - SignalMap operations
- `broadcaster.rs` - Broadcaster splitting
