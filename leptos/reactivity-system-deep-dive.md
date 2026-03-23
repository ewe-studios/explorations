# Reactivity System Deep Dive

## Overview

Leptos's reactivity system (`reactive_graph`) is a fine-grained reactive library that enables efficient, automatic UI updates. It's designed around the principle that **effects are expensive**, so the system minimizes effect re-runs at the cost of slightly more complex signal propagation.

---

## Core Concepts

### The Reactive Graph

The reactive system is a directed acyclic graph (DAG) with three node types:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Sources    │ ──► │ Computed     │ ──► │  Subscribers │
│   (Signals)  │     │ (Memos)      │     │  (Effects)   │
└──────────────┘     └──────────────┘     └──────────────┘
      │                    │                    │
      │                    ▼                    │
      └─────────────────────────────────────────┘
                    (direct dependencies)
```

**Node Categories:**

1. **Source Nodes**: Values that can be directly mutated
   - `RwSignal`, `ReadSignal`, `WriteSignal`, `ArcTrigger`

2. **Subscriber Nodes**: Values that depend on sources
   - `Memo`, `Effect`, `RenderEffect`, `Resource`

3. **Observer**: The currently-running effect/computation
   - Thread-local state tracking active subscriber

---

## Signal Types

### Arena-Allocated Signals (`Copy`)

```rust
use reactive_graph::signal::{signal, RwSignal};

// Tuple of read/write handles
let (count, set_count) = signal(0);

// Combined read/write signal
let count_rw = RwSignal::new(0);

// Local-only (not Send/Sync)
let (local_read, local_write) = signal_local(0);
```

**Storage Mechanism:**
- Signals stored in a `SlotMap` arena
- Handle contains index + version
- `Copy` - cheaply passed to closures
- Disposed when owner cleans up

### Reference-Counted Signals (`Clone`)

```rust
use reactive_graph::signal::{arc_signal, ArcRwSignal};

let (count, set_count) = arc_signal(0);
let count_rw = ArcRwSignal::new(0);
```

**Storage Mechanism:**
- Reference-counted with `Arc<RwLock<T>>`
- Lives as long as any reference exists
- Can outlive reactive scope

---

## Traits System

The reactivity system uses a trait-based architecture for composability:

### Base Traits

```rust
/// Read value without tracking
pub trait ReadUntracked: DefinedAt {
    type Value: Deref;
    fn try_read_untracked(&self) -> Option<Self::Value>;
}

/// Mutate value
pub trait Write: DefinedAt + Notify {
    type Value: Sized + 'static;
    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>>;
}

/// Subscribe to changes
pub trait Track {
    fn track(&self);
}

/// Notify subscribers
pub trait Notify {
    fn notify(&self);
}
```

### Derived Traits (Auto-Implemented)

```rust
/// Get value with tracking
pub trait Get: DefinedAt {
    type Value: Clone;
    fn get(&self) -> Self::Value;  // Uses Track + Clone
}

/// Update with notification
pub trait Update {
    type Value;
    fn update(&self, fun: impl FnOnce(&mut Self::Value));
}

/// Set new value
pub trait Set {
    type Value;
    fn set(&self, value: Self::Value);
}
```

**Blanket Implementations:**
```rust
impl<T> Read for T where T: Track + ReadUntracked { /* ... */ }
impl<T> Get for T where T: With, T::Value: Clone { /* ... */ }
impl<T> Update for T where T: Write { /* ... */ }
```

---

## Effect System

### Effect Types

#### `Effect` - General Side Effects

```rust
use reactive_graph::effect::Effect;

let name = RwSignal::new("Alice");

Effect::new(move |_| {
    // Runs whenever `name` changes
    println!("Name changed to: {}", name.get());
});
```

**Characteristics:**
- Runs after current task yields (async scheduling)
- Cancels previous run if still pending
- Cleans up on owner disposal

#### `RenderEffect` - Immediate DOM Updates

```rust
use reactive_graph::effect::RenderEffect;

let count = RwSignal::new(0);

RenderEffect::new(move |prev| {
    // Runs synchronously when count changes
    // Returns previous value for optimization
    let new_count = count.get();
    println!("Rendering count: {}", new_count);
    new_count
});
```

**Characteristics:**
- Runs synchronously
- Used for DOM updates
- Returns previous state for diffing

#### `Effect::new_isomorphic` - Universal Effects

```rust
Effect::new_isomorphic(move |_| {
    // Works in SSR, CSR, and hydration modes
    // Doesn't run during SSR
});
```

### Effect Lifecycle

```rust
Effect::new(move |_| {
    // 1. Effect created, schedules first run

    // 2. On first run:
    let value = signal.get();  // Tracks dependency

    // 3. On signal change:
    //    - Marked dirty
    //    - Scheduled for next tick

    // 4. On cleanup (owner disposed):
    //    - Cleanup functions run
    //    - Dependencies detached
});

// Register cleanup
Owner::on_cleanup(|| {
    println!("Effect cleaned up");
});
```

---

## Memos

### What is a Memo?

A memo is a **lazy, cached computation** that:
1. Runs only once per change (no matter how many readers)
2. Only notifies if value actually changed
3. Is lazy - doesn't run until read

```rust
use reactive_graph::computed::Memo;

let count = RwSignal::new(0);

// Memo tracks count automatically
let doubled = Memo::new(move |_| {
    println!("Computing...");  // Only logs when count changes AND memo is read
    count.get() * 2
});

// Access memo
let result = doubled.get();  // Computation runs here
```

### Memo vs Derived Signal

```rust
let count = RwSignal::new(0);

// Derived signal - runs every time it's called
let derived = move || count.get() * 2;

// Memo - cached, only recomputes when count changes
let memo = Memo::new(move |_| count.get() * 2);

Effect::new(move |_| {
    // derived() called twice = computation runs twice
    println!("{}", derived());
    println!("{}", derived());

    // memo.get() called twice = computation runs once
    println!("{}", memo.get());
    println!("{}", memo.get());
});
```

### ArcMemo Implementation

```rust
pub struct ArcMemo<T, S = SyncStorage> {
    inner: Arc<RwLock<ArcMemoInner<T>>>,
    _storage: PhantomData<S>,
}

struct ArcMemoInner<T> {
    value: Option<T>,
    dirty: bool,
    sources: SourceSet,
    subscribers: SubscriberSet,
}

impl<T> ArcMemo<T> {
    pub fn new<F>(fun: F) -> Self
    where F: Fn(Option<&T>) -> T + Send + Sync + 'static
    {
        let inner = Arc::new(RwLock::new(ArcMemoInner {
            value: None,
            dirty: true,  // Lazy - needs recomputation
            sources: SourceSet::new(),
            subscribers: SubscriberSet::new(),
        }));

        // ... setup subscriber trait impl
        Self { inner, _storage: PhantomData }
    }

    pub fn get(&self) -> T
    where T: Clone
    {
        // Track current effect as subscriber
        if let Some(observer) = Observer::get() {
            self.add_subscriber(observer);
        }

        // Check if dirty
        let mut inner = self.inner.write().or_poisoned();
        if inner.dirty {
            // Recompute
            inner.value = Some(compute(&mut inner));
            inner.dirty = false;
        }

        inner.value.clone().unwrap()
    }
}
```

---

## Dependency Tracking

### Automatic Tracking

Dependencies are tracked **automatically** at runtime:

```rust
let a = RwSignal::new(1);
let b = RwSignal::new(2);
let use_a = RwSignal::new(true);

let result = Memo::new(move |_| {
    if use_a.get() {
        a.get()  // Only 'a' is tracked as dependency
    } else {
        b.get()  // Only 'b' is tracked as dependency
    }
});

// Changing unused signal doesn't trigger memo
use_a.set(false);  // Now depends on b
a.set(100);        // Doesn't trigger - not a dependency!
```

### How Tracking Works

```rust
thread_local! {
    static OBSERVER: RefCell<Option<AnySubscriber>> = RefCell::new(None);
}

impl<T: Clone> Get for RwSignal<T> {
    fn get(&self) -> T {
        // 1. Check if there's an active observer (effect/memo)
        if let Some(observer) = Observer::get() {
            // 2. Register this signal as a source of the observer
            observer.add_source(self.to_any_source());

            // 3. Register observer as subscriber to this signal
            self.add_subscriber(observer);
        }

        // 4. Return value
        self.inner.read().clone()
    }
}
```

### Dynamic Dependencies

Dependencies are **cleared and re-established** on each effect run:

```rust
Effect::new(move |_| {
    // First run: depends on signal_a
    if condition.get() {
        println!("{}", signal_a.get());
    }
    // Second run (condition changed): depends on signal_b
    else {
        println!("{}", signal_b.get());
    }
});
```

---

## Owner System

### What is an Owner?

The `Owner` manages:
1. **Effect cancellation** - Cancels child effects when disposed
2. **Cleanup functions** - Runs registered cleanup code
3. **Arena allocation** - Stores `Copy` signals
4. **Context** - Provides dependency injection

```rust
let parent = Owner::new();
parent.with(|| {
    let child = Owner::new();  // Automatically becomes child of parent
    child.with(|| {
        // Reactive nodes created here belong to child
    });
    // Child disposed here
});
```

### Owner Hierarchy

```
Root Owner
├── Effect 1
│   └── Memo 1.1
├── Effect 2
│   ├── Memo 2.1
│   └── Child Owner
│       └── Effect 2.1.1
└── Effect 3
```

### Cleanup Process

```rust
let owner = Owner::new();
owner.with(|| {
    let signal = RwSignal::new(0);

    Effect::new(move |_| {
        println!("{}", signal.get());

        // Register cleanup
        Owner::on_cleanup(|| {
            println!("Cleaning up effect");
        });
    });

    // Stored value in arena
    let stored = StoredValue::new(vec![1, 2, 3]);
});

// When owner is dropped:
// 1. All child effects are cancelled
// 2. All on_cleanup callbacks run
// 3. All arena items are disposed
// 4. All stored values are dropped
```

---

## Graph Node Implementation

### Source Node (Signal)

```rust
pub struct ArcRwSignal<T> {
    inner: Arc<ArcRwSignalInner<T>>,
}

struct ArcRwSignalInner<T> {
    value: RwLock<T>,
    subscribers: SubscriberSet,
}

impl<T> Source for ArcRwSignal<T> {
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        self.inner.subscribers.insert(subscriber);
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        self.inner.subscribers.remove(subscriber);
    }

    fn mark_dirty(&self) {
        // Notify all subscribers
        for sub in &self.inner.subscribers {
            sub.mark_dirty();
        }
    }
}
```

### Subscriber Node (Effect)

```rust
pub struct Effect {
    inner: Arc<RwLock<EffectInner>>,
}

struct EffectInner {
    observer: Sender,      // Who to notify when dirty
    sources: SourceSet,    // What we depend on
    dirty: bool,           // Whether we need to re-run
}

impl Subscriber for EffectInner {
    fn add_source(&self, source: AnySource) {
        self.sources.insert(source);
    }

    fn clear_sources(&self) {
        // Remove ourselves from all source subscriber lists
        self.sources.clear();
    }
}

impl ReactiveNode for EffectInner {
    fn update_if_necessary(&self) -> bool {
        if self.dirty {
            self.dirty = false;
            return true;  // Should run
        }
        false
    }

    fn mark_check(&self) {
        // Propagate up the chain
        self.observer.notify();
    }
}
```

---

## Update Propagation

### The Reactively Algorithm

Leptos uses an algorithm based on [Reactively](https://github.com/modderme123/reactively):

1. **Mark Phase**: Mark all descendants of changed signal
2. **Check Phase**: Check which effects actually need to run
3. **Run Phase**: Execute dirty effects

```
Signal changes
     │
     ▼
┌─────────────────┐
│ Mark subscribers│ (recursively mark children dirty)
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ Check effects   │ (only run if sources actually changed)
└─────────────────┘
     │
     ▼
┌─────────────────┐
│ Run dirty       │ (execute effect callbacks)
│ effects         │
└─────────────────┘
```

### Example Propagation

```rust
let a = RwSignal::new(1);
let b = Memo::new(move |_| a.get() * 2);
let c = Memo::new(move |_| b.get() + 10);

Effect::new(move |_| {
    println!("Result: {}", c.get());
});

// When a.set(5) is called:
// 1. a marks b dirty
// 2. b marks c dirty
// 3. c marks effect dirty
// 4. Effect scheduled to run
// 5. On next tick:
//    - c checks if b changed (yes, 2 → 10)
//    - c computes new value (10 + 10 = 20)
//    - Effect runs, prints "Result: 20"
```

---

## Async Reactivity

### Resources

Resources integrate async code with reactivity:

```rust
use reactive_graph::computed::Resource;

let query = RwSignal::new("rust".to_string());

// Resource automatically refetches when query changes
let results = Resource::new(
    move || query.get(),  // Dependency tracker
    |query| async move {  // Fetcher
        fetch_results(&query).await
    }
);

// Read resource (returns Option<Data>)
let data = results.get();

// Or await
let data = results.await;
```

### Resource Internals

```rust
pub struct Resource<T, S> {
    value: ArcRwSignal<Option<T>>,
    loading: ArcRwSignal<bool>,
    error: ArcRwSignal<Option<Error>>,
    refetch: ArcTrigger,
}

impl<T> Resource<T> {
    fn schedule_fetch(&self) {
        let value = self.value.clone();
        let loading = self.loading.clone();

        spawn(async move {
            loading.set(true);
            match fetch().await {
                Ok(data) => value.set(Some(data)),
                Err(e) => /* handle error */,
            }
            loading.set(false);
        });
    }
}
```

---

## Threading Model

### Send vs Local Signals

```rust
// Send + Sync (works across threads)
let signal = RwSignal::new(0);  // SyncStorage

// !Send + !Sync (local only, faster)
let local = RwSignal::new_local(0);  // LocalStorage

// Reference-counted (thread-safe)
let arc = ArcRwSignal::new(0);
```

### Storage Types

```rust
pub trait Storage<T>: Clone + Send + Sync + 'static {
    fn get(id: usize) -> Option<T>;
    fn insert(value: T) -> usize;
    fn remove(id: usize);
}

// Thread-safe storage
pub struct SyncStorage;

// Thread-local storage (faster, !Send)
pub struct LocalStorage;
```

---

## Diagnostics

### Debug Mode Features

```rust
#[cfg(any(debug_assertions, leptos_debuginfo))]
{
    // Track where signals were created
    defined_at: Location::caller(),

    // Warn on untracked reads
    if !SpecialNonReactiveZone::is_inside() {
        console::warn("Signal accessed outside reactive context");
    }

    // Track effect ancestry for debugging
    effect.ancestry()
}
```

### Tracing Integration

```rust
#[cfg(feature = "tracing")]
use tracing::instrument;

impl<T> Memo<T> {
    #[instrument(level = "trace", skip_all)]
    pub fn new(fun: impl Fn(Option<&T>) -> T) -> Self {
        // ...
    }
}
```

---

## Performance Considerations

### Avoiding Common Pitfalls

**❌ Bad: Reading signal in closure without tracking**
```rust
let count = RwSignal::new(0);
let doubled = move || count.get() * 2;  // Creates new fn each time

view! { <span>{doubled()}</span> }  // Not reactive!
```

**✅ Good: Pass function for tracking**
```rust
let count = RwSignal::new(0);
view! { <span>{move || count.get() * 2}</span> }  // Reactive!
```

### Memoization Strategies

```rust
// 1. Derived signal (cheap, no caching)
let double = move || count.get() * 2;

// 2. Memo (cached, lazy)
let double = Memo::new(move |_| count.get() * 2);

// 3. Selector (only notify on specific changes)
let is_even = Selector::new(move || count.get() % 2 == 0);
```

### Batch Updates

```rust
// Multiple signals update
set_a.update(|a| *a += 1);
set_b.update(|b| *b += 1);
set_c.update(|c| *c += 1);
// Effect runs once after all updates
```

---

## Testing

### Unit Testing Reactivity

```rust
#[cfg(test)]
mod tests {
    use reactive_graph::prelude::*;

    #[test]
    fn test_memo() {
        let owner = Owner::new();
        owner.set();  // Make current

        let count = RwSignal::new(0);
        let doubled = Memo::new(move |_| count.get() * 2);

        assert_eq!(doubled.get(), 0);
        count.set(5);
        assert_eq!(doubled.get(), 10);
    }
}
```

---

## Resources

- [reactive_graph source](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/reactive_graph)
- [Reactively algorithm](https://github.com/modderme123/reactively)
- [SolidJS reactivity](https://www.solidjs.com/docs/latest/api) (inspiration)
