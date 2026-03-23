---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpaceJam/left-right
revised_at: 2026-03-23
---

# Left-Right Deep Dive

## Purpose

Left-right is a concurrency primitive for **high concurrency reads over a single-writer data structure**. It enables wait-free reads that scale linearly with CPU cores in the absence of writes.

## Core Architecture

### The Two-Copy Design

```
Readers ──► Pointer ──► T (Read Copy)
                          ▲
                          │
                    publish() swaps
                          │
                          ▼
Writer  ──► OpLog ──► T (Write Copy)
```

1. **Read Copy**: All readers access this copy through an atomic pointer
2. **Write Copy**: Single writer modifies this copy and records operations
3. **Operational Log (OpLog)**: Records all modifications for replay

### The Publish Cycle

```rust
// 1. Writer appends to oplog and modifies write copy
write_handle.append(Op::Insert(key, value));

// 2. Publish atomically swaps reader pointer
write_handle.publish();
// - Atomic pointer swap (cache line invalidation #1)
// - Wait for all reader epochs to change
// - Replay oplog to stale copy (now write copy)
// - Second pointer swap would happen on next publish (cache line invalidation #2)
```

### Epoch-Based Reader Tracking

```rust
// Each reader has an epoch counter
struct ReaderEpoch {
    epoch: AtomicUsize,  // Incremented on enter/exit
}

// Reader flow:
fn enter(&self) -> Option<ReadGuard<T>> {
    self.epoch.fetch_add(1, Release);  // "I'm starting"
    fence(SeqCst);
    // ... read data ...
    self.epoch.fetch_add(1, Release);  // "I'm done"
}

// Writer waits for all epochs to change:
fn wait_for_readers(&self, old_epochs: &[usize]) {
    loop {
        let all_changed = epochs.iter().all(|e| {
            let current = e.load(Acquire);
            current != old_epochs[i]  // Reader has exited
        });
        if all_changed { break; }
        thread::yield_now();
    }
}
```

## The Absorb Trait

```rust
pub trait Absorb<O> {
    /// Apply operation to FIRST copy (write side)
    fn absorb_first(&mut self, operation: &mut O, other: &Self);

    /// Apply operation to SECOND copy (read side after swap)
    /// other is one "publish cycle" ahead
    fn absorb_second(&mut self, mut operation: O, other: &Self) {
        Self::absorb_first(self, &mut operation, other)
    }

    /// Drop first copy (may need to forget for dedup)
    fn drop_first(self: Box<Self>) {}

    /// Initial sync before first publish
    fn sync_with(&mut self, first: &Self);
}
```

### Key Considerations

1. **Determinism**: Operations must produce identical results on both copies
2. **Non-deterministic pitfalls**:
   - HashMap iteration order (RandomState)
   - Timestamp generation
   - Random number generation

## Production-Level Rust Reproduction

### Crate Structure

```
left-right-reproduction/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public API
    ├── absorb.rs        # Absorb trait
    ├── write.rs         # WriteHandle
    ├── read.rs          # ReadHandle, ReadGuard
    ├── sync.rs          # Arc, Mutex abstractions
    └── oplog.rs         # Operational log
```

### Core Implementation

```rust
// src/lib.rs
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use slab::Slab;

type Epochs = Arc<Mutex<Slab<Arc<AtomicUsize>>>>;

pub struct WriteHandle<T, O> {
    data: T,
    oplog: Vec<O>,
    epochs: Epochs,
    // ... tracking fields
}

pub struct ReadHandle<T> {
    data: Arc<T>,
    epoch: Arc<AtomicUsize>,
    epochs: Epochs,
}

pub struct ReadGuard<'a, T> {
    data: &'a T,
    epoch: &'a AtomicUsize,
    enter_epoch: usize,
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { self.data }
}
```

### Memory Ordering

```rust
// Reader entry - critical for correctness
fn enter(&self) -> Option<ReadGuard<T>> {
    // Signal "I'm starting read" - must be visible before we read data
    self.epoch.store(self.next_epoch, Ordering::Release);

    // Ensure epoch store happens before data access
    std::sync::atomic::fence(Ordering::SeqCst);

    // Now safe to read - writer seeing our epoch knows we're in-flight
    ReadHandle::get_data(&self.data)
}

// Writer publish cycle
fn publish(&mut self) {
    // 1. Swap pointer (Release ensures oplog writes visible)
    let old = self.reader_ptr.swap(new_ptr, Ordering::Release);

    // 2. Wait for epochs to change
    self.wait_for_readers();

    // 3. Replay oplog to stale copy
    for op in &mut self.oplog {
        self.stale_copy.absorb_second(op.take(), self.fresh_copy);
    }
}
```

## Performance Optimizations

### 1. OpLog Batching

```rust
// Batch multiple operations before publish
impl<T, O> WriteHandle<T, O> {
    pub fn append(&mut self, op: O) {
        self.oplog.push(op);
        // Don't publish yet - batch more operations
    }

    pub fn publish(&mut self) {
        if self.oplog.is_empty() { return; }
        // Single publish for N operations
        self.publish_inner();
    }
}
```

### 2. Reader-Local Epochs

```rust
// Avoid contention on global epoch structure
struct LocalEpoch {
    slot_id: usize,
    epoch: Arc<AtomicUsize>,
}

impl Drop for LocalEpoch {
    fn drop(&mut self) {
        // Remove from global slab on thread exit
        epochs.lock().remove(self.slot_id);
    }
}
```

### 3. Deduplication for Memory Efficiency

```rust
// For types that can share data between copies
struct DedupVec<T> {
    left: Vec<T>,
    right: Vec<T>,
    shared: Arc<Vec<T>>,  // Shared until modification
}

impl<T: Clone> Absorb<Op<T>> for DedupVec<T> {
    fn absorb_first(&mut self, op: &mut Op<T>, _: &Self) {
        // Ensure unique ownership before modification
        if Arc::strong_count(&self.shared) > 1 {
            self.left = (*self.shared).clone();
        }
        // Apply to left...
    }
}
```

## Edge Cases and Safety

### 1. Writer Drop

```rust
impl<T, O> Drop for WriteHandle<T, O> {
    fn drop(&mut self) {
        // Signal to readers that data is invalid
        self.closed.store(true, Ordering::Release);
        // Readers will return None on next enter()
    }
}
```

### 2. Reentrant Reads

```rust
// Prevent deadlock if read tries to access write handle
// Solution: Document as unsafe or use thread-local state
thread_local! {
    static IN_READ: Cell<bool> = Cell::new(false);
}

fn assert_no_reentrant() {
    IN_READ.with(|in_read| {
        assert!(!in_read.get(), "Cannot access WriteHandle from ReadGuard closure");
    });
}
```

### 3. Memory Leak Prevention

```rust
// OpLog must not grow unbounded
impl<T, O> WriteHandle<T, O> {
    fn publish_inner(&mut self) {
        // After replay, oplog is cleared
        self.oplog.clear();
        self.oplog.shrink_to_fit();  // Release memory
    }
}
```

## Comparison to Alternatives

| Pattern | Read Scale | Write Scale | Memory | Use Case |
|---------|-----------|-------------|--------|----------|
| `RwLock<T>` | O(Cores) | O(1) | 1x | General purpose |
| `Arc<T>` + COW | O(Cores) | O(1) alloc | 2x | Infrequent writes |
| `left-right` | O(Cores) wait-free | O(OpLog) | 2x + oplog | Read-heavy, deterministic ops |
| `evmap` | O(Cores) | O(1) amortized | 2x | Key-value maps |

## When to Use Left-Right

**Good fit:**
- Read-heavy workloads (90%+ reads)
- Deterministic operations (no RNG, timestamps in ops)
- Need wait-free reads (real-time requirements)
- Can tolerate 2x memory

**Poor fit:**
- Write-heavy workloads
- Non-deterministic operations
- Memory-constrained environments
- Multiple writers (need external Mutex)
