---
title: "Sync Primitives Deep Dive"
subtitle: "Mutexes, channels, semaphores, and barriers"
parent: exploration.md
---

# Sync Primitives Deep Dive

## Introduction

This document provides a comprehensive deep dive into synchronization primitives used in concurrent systems, covering mutexes, channels, semaphores, barriers, and Embassy-sync primitives for no-std environments.

---

## Part 1: Mutexes

### What is a Mutex?

A **Mutex** (mutual exclusion) ensures only one thread can access data at a time:

```rust
use std::sync::Mutex;

let m = Mutex::new(0);

// Lock before accessing
{
    let mut guard = m.lock().unwrap();
    *guard += 1;
}  // Lock released here
```

### Mutex Implementation

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::cell::UnsafeCell;

struct SimpleMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> SimpleMutex<T> {
    fn new(data: T) -> Self {
        SimpleMutex {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    fn lock(&self) -> MutexGuard<T> {
        // Spin until we acquire lock
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }
        MutexGuard { mutex: self }
    }
}

struct MutexGuard<'a, T> {
    mutex: &'a SimpleMutex<T>,
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}
```

### RwLock (Read-Write Lock)

```rust
use std::sync::RwLock;

let rw = RwLock::new(0);

// Multiple readers
let r1 = rw.read().unwrap();
let r2 = rw.read().unwrap();

// Single writer (exclusive)
let mut w = rw.write().unwrap();
*w = 42;
```

### Embassy Mutex (no-std)

```rust
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

static MUTEX: Mutex<NoopRawMutex, u32> = Mutex::new(0);

#[embassy_executor::task]
async fn task1() {
    let mut val = MUTEX.lock().await;
    *val += 1;
}
```

---

## Part 2: Channels

### MPSC (Multi-Producer, Single-Consumer)

```rust
use tokio::sync::mpsc;

let (tx, rx) = mpsc::channel(100);

// Multiple senders
let tx1 = tx.clone();
let tx2 = tx.clone();

tokio::spawn(async move {
    tx1.send(1).await.unwrap();
});

tokio::spawn(async move {
    tx2.send(2).await.unwrap();
});

// Single consumer
tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        println!("Received: {}", msg);
    }
});
```

### Bounded vs Unbounded

| Bounded | Unbounded |
|---------|-----------|
| Fixed capacity | Unlimited |
| Backpressure | No backpressure |
| Memory bounded | Can grow infinitely |
| Sender waits if full | Sender never waits |

```rust
// Bounded
let (tx, rx) = mpsc::channel(10);
tx.send(msg).await;  // Waits if buffer full

// Unbounded
let (tx, rx) = mpsc::unbounded_channel();
tx.send(msg).unwrap();  // Never waits
```

### Broadcast Channels

```rust
use tokio::sync::broadcast;

let (tx, mut rx1) = broadcast::channel(100);
let mut rx2 = tx.subscribe();
let mut rx3 = tx.subscribe();

// Send to all
tx.send("Hello").unwrap();

// All receivers get the message
assert_eq!(rx1.recv().await.unwrap(), "Hello");
assert_eq!(rx2.recv().await.unwrap(), "Hello");
assert_eq!(rx3.recv().await.unwrap(), "Hello");
```

### Watch Channels (Latest Value)

```rust
use tokio::sync::watch;

let (tx, mut rx) = watch::channel(0);

// Only latest value kept
tx.send(1).unwrap();
tx.send(2).unwrap();
tx.send(3).unwrap();

// rx only sees 3 (latest)
assert_eq!(*rx.borrow(), 3);
```

### Embassy Channels (no-std)

```rust
use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

static CHANNEL: Channel<NoopRawMutex, u32, 4> = Channel::new();

#[embassy_executor::task]
async fn producer() {
    CHANNEL.send(42).await;
}

#[embassy_executor::task]
async fn consumer() {
    let val = CHANNEL.recv().await;
    assert_eq!(val, 42);
}
```

---

## Part 3: Semaphores

### What is a Semaphore?

A **Semaphore** limits the number of concurrent operations:

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

let sem = Arc::new(Semaphore::new(3));  // Max 3 concurrent

// Acquire permit
let permit = sem.acquire().await.unwrap();
// Do work (only 3 can be here at once)
drop(permit);  // Release
```

### Semaphore Implementation

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Waker;
use std::collections::VecDeque;

struct Semaphore {
    count: AtomicUsize,
    waiters: Mutex<VecDeque<Waker>>,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: AtomicUsize::new(count),
            waiters: Mutex::new(VecDeque::new()),
        }
    }

    async fn acquire(&self) -> Permit {
        loop {
            let current = self.count.load(Ordering::Acquire);
            if current > 0 {
                if self.count.compare_exchange(
                    current,
                    current - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ).is_ok() {
                    return Permit { semaphore: self };
                }
            }
            // Wait for permit
            tokio::task::yield_now().await;
        }
    }
}
```

### Use Cases

1. **Connection pooling** - Limit DB connections
2. **Rate limiting** - Limit requests per second
3. **Resource limiting** - Limit memory usage

```rust
// Rate limiter example
struct RateLimiter {
    sem: Arc<Semaphore>,
}

impl RateLimiter {
    fn new(max_per_second: usize) -> Self {
        RateLimiter {
            sem: Arc::new(Semaphore::new(max_per_second)),
        }
    }

    async fn acquire(&self) -> PermitGuard {
        let permit = self.sem.acquire().await.unwrap();
        PermitGuard {
            permit,
            limiter: self,
        }
    }
}
```

---

## Part 4: Barriers

### What is a Barrier?

A **Barrier** makes threads wait until all reach the barrier:

```
Thread 0: ────► [Barrier] ────►
Thread 1: ────────► [Barrier] ────►
Thread 2: ──► [Barrier] ────►

All proceed only when all arrive
```

```rust
use std::sync::{Arc, Barrier};

let barrier = Arc::new(Barrier::new(3));
let b1 = barrier.clone();
let b2 = barrier.clone();

thread::spawn(move || {
    // Do work
    b1.wait();  // Wait for others
    // All continue together
});

thread::spawn(move || {
    // Do work
    b2.wait();  // Wait for others
    // All continue together
});

barrier.wait();  // Wait for others
// All continue together
```

### Barrier Implementation

```rust
use std::sync::{Mutex, Condvar};

struct Barrier {
    count: Mutex<usize>,
    generation: Mutex<usize>,
    waiting: Condvar,
    total: usize,
}

impl Barrier {
    fn new(total: usize) -> Self {
        Barrier {
            count: Mutex::new(0),
            generation: Mutex::new(0),
            waiting: Condvar::new(),
            total,
        }
    }

    fn wait(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;

        if *count == self.total {
            // Last one arrives, release all
            *count = 0;
            *self.generation.lock().unwrap() += 1;
            self.waiting.notify_all();
        } else {
            // Wait for others
            let gen = *self.generation.lock().unwrap();
            while gen == *self.generation.lock().unwrap() {
                count = self.waiting.wait(count).unwrap();
            }
        }
    }
}
```

---

## Part 5: Atomic Operations

### Atomic Types

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

let atomic = AtomicUsize::new(0);

// Atomic operations
atomic.fetch_add(1, Ordering::SeqCst);
atomic.fetch_sub(1, Ordering::SeqCst);
atomic.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst);
```

### Memory Orderings

| Ordering | Description |
|----------|-------------|
| `SeqCst` | Sequentially consistent (strongest) |
| `AcqRel` | Acquire + Release |
| `Acquire` | Acquire (load) |
| `Release` | Release (store) |
| `Relaxed` | No ordering guarantees |

```rust
// Lock-free stack example
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

struct LockFreeStack<T> {
    head: AtomicPtr<Node<T>>,
}

struct Node<T> {
    data: T,
    next: *mut Node<T>,
}

impl<T> LockFreeStack<T> {
    fn push(&self, data: T) {
        let mut node = Box::new(Node {
            data,
            next: ptr::null_mut(),
        });

        loop {
            let head = self.head.load(Ordering::Acquire);
            node.next = head;

            let head_ptr = if head.is_null() {
                ptr::null_mut()
            } else {
                head
            };

            match self.head.compare_exchange_weak(
                head_ptr,
                &mut *node,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    Box::leak(node);
                    break;
                }
                Err(_) => continue,  // Retry
            }
        }
    }
}
```

---

## Part 6: Embassy-Sync Primitives

### No-Std Mutex

```rust
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;

// Compile-time initialized
static MUTEX: Mutex<NoopRawMutex, u32> = Mutex::new(0);

// Async lock
async fn use_mutex() {
    let mut val = MUTEX.lock().await;
    *val += 1;
}
```

### No-Std Channel

```rust
use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

// Channel with capacity 4
static CHANNEL: Channel<CriticalSectionRawMutex, u32, 4> = Channel::new();

async fn producer() {
    CHANNEL.send(42).await;
}

async fn consumer() {
    let val = CHANNEL.recv().await;
}
```

### Signal (One-shot notification)

```rust
use embassy_sync::signal::Signal;

static SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

async fn waiter() {
    let val = SIGNAL.wait().await;
    println!("Signal: {}", val);
}

fn signaller() {
    SIGNAL.signal(true);
}
```

---

*This document covers the essential sync primitives for concurrent Rust programming...*
