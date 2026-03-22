---
name: Eyeball
description: Observable types library for reactive programming with subscriber pattern, providing reactive state management for UI applications
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/eyeball/
---

# Eyeball - Observable Types for Reactive Rust

## Overview

Eyeball is a Rust library providing observable types for reactive programming. It implements the subscriber pattern, allowing you to subscribe to changes in a value and receive updates asynchronously. This is essential for building reactive UIs where the view needs to update automatically when state changes.

## Repository Structure

```
eyeball/
├── eyeball/                        # Core Observable type
│   ├── src/
│   │   ├── lib.rs                  # Main library entry
│   │   ├── observable.rs           # Observable<T> implementation
│   │   ├── subscriber.rs           # Subscriber handling
│   │   ├── read_guard.rs           # Read guard types
│   │   ├── shared.rs               # SharedObservable for multi-producer
│   │   ├── futures.rs              # Future integrations
│   │   └── macros.rs               # Utility macros
│   ├── Cargo.toml
│   └── tests/
│       └── eyeball.rs              # Integration tests
│
├── eyeball-im/                     # Observable collections
│   ├── src/
│   │   ├── lib.rs                  # Library entry
│   │   ├── vector.rs               # ObservableVector<T>
│   │   ├── batch.rs                # Batch updates
│   │   ├── transaction.rs          # Transactional updates
│   │   └── subscriber.rs           # Vector subscriber
│   ├── Cargo.toml
│   └── tests/
│       └── vector.rs               # Vector tests
│
└── eyeball-im-util/                # Utilities for eyeball-im
    ├── src/
    │   ├── lib.rs
    │   ├── filter.rs               # Filtered views
    │   ├── sort.rs                 # Sorted views
    │   └── adaptors.rs             # Iterator adaptors
    └── Cargo.toml
```

## Core API: Observable<T>

### Basic Usage

```rust
use eyeball::Observable;

// Create an observable value
let mut observable = Observable::new(42);

// Subscribe to changes
let mut subscriber = observable.subscribe();

// Modify the value
observable.set(100);

// Receive updates (async)
while let Some(value) = subscriber.next().await {
    println!("Value changed to: {}", value);
}
```

### Observable Implementation

```rust
// eyeball/src/observable.rs
use std::sync::{
    Arc,
    RwLock,
    atomic::{AtomicUsize, Ordering}
};
use tokio::sync::broadcast;

pub struct Observable<T> {
    inner: Arc<ObservableInner<T>>,
}

struct ObservableInner<T> {
    value: RwLock<T>,
    sender: broadcast::Sender<Arc<T>>,
    version: AtomicUsize,
}

impl<T: Clone> Observable<T> {
    /// Create a new Observable with initial value
    pub fn new(value: T) -> Self {
        let (sender, _) = broadcast::channel(100);

        Self {
            inner: Arc::new(ObservableInner {
                value: RwLock::new(value),
                sender,
                version: AtomicUsize::new(0),
            }),
        }
    }

    /// Set a new value and notify subscribers
    pub fn set(&mut self, value: T) {
        let arc_value = Arc::new(value);

        // Update internal value
        *self.inner.value.write().unwrap() = arc_value.clone();

        // Increment version
        self.inner.version.fetch_add(1, Ordering::SeqCst);

        // Notify subscribers (ignore send errors if no subscribers)
        let _ = self.inner.sender.send(arc_value);
    }

    /// Get a reference to the current value
    pub fn get(&self) -> ReadGuard<T> {
        ReadGuard {
            inner: self.inner.value.read().unwrap(),
        }
    }

    /// Subscribe to value changes
    pub fn subscribe(&self) -> Subscriber<T> {
        let receiver = self.inner.sender.subscribe();
        let current = Arc::new((*self.inner.value.read().unwrap()).clone());

        Subscriber {
            receiver,
            current,
        }
    }

    /// Get the current version number
    pub fn version(&self) -> usize {
        self.inner.version.load(Ordering::SeqCst)
    }
}

impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
```

### ReadGuard

```rust
// eyeball/src/read_guard.rs
use std::ops::Deref;
use std::sync::RwLockReadGuard;

pub struct ReadGuard<'a, T> {
    inner: RwLockReadGuard<'a, T>,
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// ReadGuard automatically releases the lock when dropped
// Allows read-only access to the observable value
```

### Subscriber

```rust
// eyeball/src/subscriber.rs
use tokio::sync::broadcast;
use std::sync::Arc;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures_core::Stream;

pub struct Subscriber<T> {
    receiver: broadcast::Receiver<Arc<T>>,
    current: Arc<T>,
}

impl<T: Clone> Subscriber<T> {
    /// Get the current value without waiting
    pub fn get(&self) -> Arc<T> {
        Arc::clone(&self.current)
    }

    /// Wait for the next update
    pub async fn next(&mut self) -> Option<Arc<T>> {
        match self.receiver.recv().await {
            Ok(value) => {
                self.current = Arc::clone(&value);
                Some(value)
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                // Subscriber was too slow, n messages were skipped
                // Still return the latest value
                Some(Arc::clone(&self.current))
            }
            Err(broadcast::error::RecvError::Closed) => None,
        }
    }

    /// Get the next update if available (non-blocking)
    pub fn next_now(&mut self) -> Option<Arc<T>> {
        match self.receiver.try_recv() {
            Ok(value) => {
                self.current = Arc::clone(&value);
                Some(value)
            }
            Err(_) => None,
        }
    }
}

impl<T: Clone> Stream for Subscriber<T> {
    type Item = Arc<T>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(value) => {
                self.current = Arc::clone(&value);
                Poll::Ready(Some(value))
            }
            Err(broadcast::error::TryRecvError::Empty) => {
                // Register waker for when new value arrives
                let waker = cx.waker().clone();
                // ... store waker for notification
                Poll::Pending
            }
            Err(_) => Poll::Ready(None),
        }
    }
}
```

## SharedObservable (Multi-Producer)

```rust
// eyeball/src/shared.rs
use std::sync::Arc;
use crate::{Observable, ObservableInner};

/// SharedObservable allows multiple producers to update the value
pub struct SharedObservable<T> {
    inner: Arc<ObservableInner<T>>,
}

impl<T: Clone> SharedObservable<T> {
    pub fn new(value: T) -> Self {
        let (sender, _) = broadcast::channel(100);

        Self {
            inner: Arc::new(ObservableInner {
                value: RwLock::new(value),
                sender,
                version: AtomicUsize::new(0),
            }),
        }
    }

    /// Set a new value (same as Observable::set)
    pub fn set(&self, value: T) {
        let arc_value = Arc::new(value);
        *self.inner.value.write().unwrap() = arc_value.clone();
        self.inner.version.fetch_add(1, Ordering::SeqCst);
        let _ = self.inner.sender.send(arc_value);
    }

    /// Update the value using a closure
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut guard = self.inner.value.write().unwrap();
        f(&mut *guard);
        let new_value = Arc::new((*guard).clone());
        drop(guard);

        self.inner.version.fetch_add(1, Ordering::SeqCst);
        let _ = self.inner.sender.send(new_value);
    }

    /// Subscribe to changes
    pub fn subscribe(&self) -> Subscriber<T> {
        let receiver = self.inner.sender.subscribe();
        let current = Arc::new((*self.inner.value.read().unwrap()).clone());

        Subscriber { receiver, current }
    }

    /// Get the current value
    pub fn get(&self) -> ReadGuard<T> {
        ReadGuard {
            inner: self.inner.value.read().unwrap(),
        }
    }
}

impl<T> Clone for SharedObservable<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// Usage example:
let obs = SharedObservable::new(0);

// Multiple threads can update
let obs1 = obs.clone();
std::thread::spawn(move || {
    obs1.set(42);
});

let obs2 = obs.clone();
std::thread::spawn(move || {
    obs2.update(|v| *v += 1);
});
```

## Observable Collections (eyeball-im)

### ObservableVector

```rust
// eyeball-im/src/vector.rs
use im::Vector as ImVector;  // Immutable vector from im crate
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// A vector that can be subscribed to for change notifications
pub struct ObservableVector<T> {
    inner: Arc<VectorInner<T>>,
}

struct VectorInner<T> {
    vector: RwLock<ImVector<T>>,
    sender: broadcast::Sender<VectorDiff<T>>,
}

/// Represents a change to the vector
#[derive(Debug, Clone)]
pub enum VectorDiff<T> {
    Append { values: ImVector<T> },
    Insert { index: usize, value: T },
    Remove { index: usize, value: T },
    Update { index: usize, value: T },
    Clear,
    Push { value: T },
    Pop { value: T },
}

impl<T: Clone> ObservableVector<T> {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);

        Self {
            inner: Arc::new(VectorInner {
                vector: RwLock::new(ImVector::new()),
                sender,
            }),
        }
    }

    /// Push a value to the end
    pub fn push_back(&self, value: T) {
        let mut vec = self.inner.vector.write().unwrap();
        vec.push_back(value.clone());
        drop(vec);

        let _ = self.inner.sender.send(VectorDiff::Push { value });
    }

    /// Insert at index
    pub fn insert(&self, index: usize, value: T) {
        let mut vec = self.inner.vector.write().unwrap();
        vec.insert(index, value.clone());
        drop(vec);

        let _ = self.inner.sender.send(VectorDiff::Insert { index, value });
    }

    /// Remove at index
    pub fn remove(&self, index: usize) -> Option<T> {
        let mut vec = self.inner.vector.write().unwrap();
        let value = vec.remove(index)?;
        drop(vec);

        let _ = self.inner.sender.send(VectorDiff::Remove {
            index,
            value: value.clone()
        });

        Some(value)
    }

    /// Update value at index
    pub fn set(&self, index: usize, value: T) -> Option<T> {
        let mut vec = self.inner.vector.write().unwrap();
        let old = vec.set(index, value.clone())?;
        drop(vec);

        let _ = self.inner.sender.send(VectorDiff::Update { index, value });

        Some(old)
    }

    /// Clear the vector
    pub fn clear(&self) {
        let mut vec = self.inner.vector.write().unwrap();
        vec.clear();
        drop(vec);

        let _ = self.inner.sender.send(VectorDiff::Clear);
    }

    /// Get a clone of the entire vector
    pub fn get(&self) -> ImVector<T> {
        self.inner.vector.read().unwrap().clone()
    }

    /// Subscribe to changes
    pub fn subscribe(&self) -> VectorSubscriber<T> {
        let receiver = self.inner.sender.subscribe();
        let current = Arc::new((*self.inner.vector.read().unwrap()).clone());

        VectorSubscriber { receiver, current }
    }
}
```

### Batch Updates

```rust
// eyeball-im/src/batch.rs
use crate::{ObservableVector, VectorDiff};

/// Batch multiple updates into a single notification
pub struct BatchBuilder<'a, T> {
    vector: &'a ObservableVector<T>,
    diffs: Vec<VectorDiff<T>>,
}

impl<'a, T: Clone> BatchBuilder<'a, T> {
    pub fn new(vector: &'a ObservableVector<T>) -> Self {
        Self {
            vector,
            diffs: Vec::new(),
        }
    }

    pub fn push(mut self, value: T) -> Self {
        self.diffs.push(VectorDiff::Push { value });
        self
    }

    pub fn insert(mut self, index: usize, value: T) -> Self {
        self.diffs.push(VectorDiff::Insert { index, value });
        self
    }

    pub fn remove(mut self, index: usize) -> Self {
        // Get the value being removed
        if let Some(value) = self.vector.get().get(index).cloned() {
            self.diffs.push(VectorDiff::Remove { index, value });
        }
        self
    }

    /// Apply all changes and notify subscribers once
    pub fn apply(self) {
        // Apply changes to the vector
        let mut vec = self.vector.inner.vector.write().unwrap();

        for diff in &self.diffs {
            match diff {
                VectorDiff::Push { value } => { vec.push_back(value.clone()); }
                VectorDiff::Insert { index, value } => { vec.insert(*index, value.clone()); }
                VectorDiff::Remove { index, .. } => { vec.remove(*index); }
                VectorDiff::Update { index, value } => { vec.set(*index, value.clone()); }
                VectorDiff::Clear => { vec.clear(); }
                _ => {}
            }
        }

        drop(vec);

        // Send all diffs as a batch
        for diff in self.diffs {
            let _ = self.vector.inner.sender.send(diff);
        }
    }
}

// Usage:
let vector = ObservableVector::new();

// Batch multiple operations
BatchBuilder::new(&vector)
    .push("item1")
    .push("item2")
    .insert(0, "item0")
    .apply();

// Subscribers receive all 3 updates in a single batch
```

## Futures Integration

```rust
// eyeball/src/futures.rs
use futures_util::StreamExt;
use crate::{Observable, Subscriber};

/// Wait for the observable to reach a specific condition
pub async fn wait_for<T, F>(observable: Observable<T>, mut predicate: F) -> T
where
    T: Clone + Send + Sync + 'static,
    F: FnMut(&T) -> bool,
{
    let mut subscriber = observable.subscribe();

    // Check current value first
    if predicate(&subscriber.get()) {
        return (*subscriber.get()).clone();
    }

    // Wait for changes
    while let Some(value) = subscriber.next().await {
        if predicate(&value) {
            return (*value).clone();
        }
    }

    unreachable!("Subscriber stream should not end")
}

/// Debounce observable updates
pub fn debounce<T, F>(
    observable: Observable<T>,
    duration: std::time::Duration,
    mut f: F,
) -> impl futures_util::stream::Stream<Item = T>
where
    T: Clone + Send + Sync + 'static,
    F: FnMut(&T) + Send + 'static,
{
    use tokio::time::{sleep, Duration};
    use futures_util::stream::unfold;

    let mut subscriber = observable.subscribe();

    unfold(subscriber, move |mut sub| {
        let duration = duration;
        async move {
            if let Some(value) = sub.next().await {
                // Wait for debounce duration
                sleep(duration).await;

                // Check if there are newer values
                while let Ok(latest) = sub.receiver.try_recv() {
                    // Discard intermediate values
                }

                Some((value, sub))
            } else {
                None
            }
        }
    })
}

// Usage example:
#[tokio::main]
async fn main() {
    let obs = Observable::new(String::new());

    // Wait for non-empty string
    let result = wait_for(obs.clone(), |s| !s.is_empty()).await;
    println!("Got non-empty string: {}", result);

    // Debounce rapid updates (e.g., search input)
    let debounced = debounce(obs.clone(), Duration::from_millis(300), |s| {
        println!("Searching for: {}", s);
    });

    debounced.for_each(|_| async {}).await;
}
```

## Usage in UI Applications (Robrix Example)

```rust
// Example from Robrix - Matrix chat client
use eyeball::SharedObservable;
use eyeball_im::ObservableVector;

// Room list with reactive updates
pub struct RoomListService {
    rooms: SharedObservable<ObservableVector<RoomListItem>>,
    client: matrix_sdk::Client,
}

impl RoomListService {
    pub fn new(client: matrix_sdk::Client) -> Self {
        Self {
            rooms: SharedObservable::new(ObservableVector::new()),
            client,
        }
    }

    /// Subscribe to room list changes
    pub fn subscribe_rooms(&self) -> impl futures_util::Stream<Item = VectorDiff<RoomListItem>> {
        let vector = self.rooms.get();
        let subscriber = vector.subscribe();

        // Transform into stream of diffs
        futures_util::stream::unfold(subscriber, |mut sub| async move {
            sub.next().await.map(|diff| (diff, sub))
        })
    }

    /// Sync rooms with server
    pub async fn sync(&self) -> matrix_sdk::Result<()> {
        // Sliding sync with Matrix
        let response = self.client.sliding_sync().await?;

        // Update observable vector (triggers UI update)
        let rooms = self.rooms.get();
        rooms.clear();

        for room in response.rooms {
            rooms.push_back(RoomListItem::from(room));
        }

        Ok(())
    }
}

// UI component that reacts to room changes
pub struct RoomListView {
    rooms: ObservableVector<RoomListItem>,
}

impl RoomListView {
    pub fn new(service: &RoomListService) -> Self {
        Self {
            rooms: service.rooms.get(),
        }
    }

    pub fn render(&self) {
        // Render room list
        // Automatically re-renders when rooms vector changes
        for room in self.rooms.get().iter() {
            println!("Room: {}", room.name);
        }
    }
}
```

## Comparison with Alternatives

| Library | Approach | Best For |
|---------|----------|----------|
| eyeball | Broadcast channel + Arc | UI state, reactive apps |
| signal-hook | Signal handling | System signals |
| notify | File system events | File watching |
| tokio::sync::watch | Single value | Simple state |
| tokio::sync::broadcast | Multi-subscriber | Event streaming |

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|------------|-------|
| `set()` | O(1) | Clone + broadcast |
| `get()` | O(1) | RwLock read |
| `subscribe()` | O(1) | New broadcast receiver |
| `next().await` | O(1) | Async receive |
| Vector `push_back()` | O(log n) | ImVector complexity |
| Vector `insert()` | O(log n) | ImVector complexity |

## Summary

Eyeball provides:
- **Observable<T>** - Single value with subscribers
- **SharedObservable<T>** - Multi-producer observable
- **ObservableVector<T>** - Reactive collections
- **Batch updates** - Group multiple changes
- **Futures integration** - Async/await support
- **Stream API** - Compatible with tokio streams

This is the foundation for reactive UI in Project Robius applications like Robrix.
