---
title: "Async Model Deep Dive"
subtitle: "Futures, executors, wakers, and task scheduling"
parent: exploration.md
---

# Async Model Deep Dive

## Introduction

This document provides a comprehensive deep dive into the async model of concurrency, covering futures, executors, wakers, task scheduling, and the embedded async architecture of Embassy.

---

## Part 1: Futures Fundamentals

### What is a Future?

A **Future** represents a value that may not be ready yet. It's a core abstraction for asynchronous computation:

```rust
// A Future is a value that will eventually produce a result
let future = async {
    let data = fetch_data().await;
    process(data)
};
```

### The Future Trait

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Future {
    type Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pub enum Poll<T> {
    Ready(T),    // Value is ready
    Pending,     // Not ready yet
}
```

### How Poll Works

```
1. Executor calls poll()
2. Future does some work
3. If ready: return Poll::Ready(value)
4. If not ready:
   - Register waker with I/O system
   - return Poll::Pending
5. When I/O completes, waker notifies executor
6. Executor polls again
```

```rust
struct MyFuture {
    data: Option<String>,
    io_handle: IoHandle,
}

impl Future for MyFuture {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(data) = self.data.take() {
            return Poll::Ready(data);
        }

        // Register waker to be notified when data arrives
        self.io_handle.set_waker(cx.waker());

        // Try to get data
        match self.io_handle.try_read() {
            Some(data) => Poll::Ready(data),
            None => Poll::Pending,  // Will be woken later
        }
    }
}
```

### Pin and Why It Matters

**Pin** prevents a value from being moved in memory. This is crucial because:

1. Futures may hold self-references
2. Moving would invalidate those references
3. Pin ensures stable memory location

```rust
use std::pin::Pin;

// Before pinning - can move
let mut future = async { 42 };
let future2 = future;  // OK

// After pinning - cannot move
let mut future = Box::pin(async { 42 });
let future_ref = future.as_mut();  // Get pinned reference
// let future2 = future;  // ERROR: cannot move pinned value
```

---

## Part 2: Executors

### What is an Executor?

An **executor** is a runtime that drives futures to completion by repeatedly polling them:

```
┌─────────────────────────────────────┐
│            Executor                  │
│                                      │
│  ┌───────────────────────────────┐  │
│  │        Task Queue              │  │
│  │  [Task A][Task B][Task C]     │  │
│  └───────────────────────────────┘  │
│              │                       │
│              ▼                       │
│  ┌───────────────────────────────┐  │
│  │        Poll Loop               │  │
│  │  poll A → Pending             │  │
│  │  poll B → Ready(x) ✓          │  │
│  │  poll C → Pending             │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

### Basic Executor Implementation

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

struct Task {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    waker: Option<Waker>,
}

struct Executor {
    task_queue: Arc<Mutex<VecDeque<Task>>>,
}

impl Executor {
    fn new() -> Self {
        Executor {
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Task {
            future: Box::pin(future),
            waker: None,
        };
        self.task_queue.lock().unwrap().push_back(task);
    }

    fn run(&self) {
        loop {
            // Get next task
            let mut queue = self.task_queue.lock().unwrap();
            let mut task = match queue.pop_front() {
                Some(task) => task,
                None => break,  // No more tasks
            };

            // Create waker
            let waker = create_waker(self.task_queue.clone());
            let cx = &mut Context::from_waker(&waker);

            // Poll the future
            let future = &mut task.future;
            match future.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    // Task completed
                }
                Poll::Pending => {
                    // Task not ready, re-queue
                    task.waker = Some(waker);
                    queue.push_back(task);
                }
            }
        }
    }
}

fn create_waker(queue: Arc<Mutex<VecDeque<Task>>>) -> Waker {
    // Simplified - real implementation needs more care
    let raw_waker = std::task::RawWaker::new(
        Arc::into_raw(queue) as *const (),
        &VTABLE
    );
    unsafe { Waker::from_raw(raw_waker) }
}
```

### Tokio Executor

```rust
// Tokio's multi-threaded executor
#[tokio::main]
async fn main() {
    // Tokio runtime created automatically
    let handle1 = tokio::spawn(async {
        // Runs on Tokio executor
    });

    let handle2 = tokio::spawn(async {
        // Runs on Tokio executor
    });

    handle1.await.unwrap();
    handle2.await.unwrap();
}
```

### Embassy Executor

```rust
// Embassy's embedded executor (no-std, no heap)
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // All tasks use static memory
    // No runtime allocation
}

#[embassy_executor::task]
async fn my_task() {
    // Runs on Embassy executor
}
```

---

## Part 3: Wakers

### What is a Waker?

A **Waker** is a handle that allows a future to notify the executor when it's ready to be polled again:

```
┌───────────────────┐
│     Future        │
│                   │
│  poll() → Pending │
│  (registers waker)│
└─────────┬─────────┘
          │
          │ I/O completes
          │
          ▼
┌───────────────────┐
│     Waker         │
│                   │
│  wake()           │◄── I/O system calls this
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│    Executor       │
│                   │
│  Re-schedules     │
│  the future       │
└───────────────────┘
```

### Waker Implementation

```rust
use std::task::{RawWaker, RawWakerVTable, Waker};
use std::sync::Arc;

struct MyWaker {
    task_id: usize,
    scheduler: Arc<Scheduler>,
}

impl MyWaker {
    fn wake(&self) {
        // Tell scheduler to re-schedule this task
        self.scheduler.schedule(self.task_id);
    }
}

// VTable defines waker operations
const VTABLE: RawWakerVTable = RawWakerVTable::new(
    clone,   // Clone the waker
    wake,    // Wake the task
    wake_by_ref,  // Wake without consuming
    drop,    // Drop the waker
);

unsafe fn clone(ptr: *const ()) -> RawWaker {
    let arc = Arc::from_raw(ptr as *const MyWaker);
    let new = Arc::clone(&arc);
    std::mem::forget(arc);
    RawWaker::new(Arc::into_raw(new) as *const (), &VTABLE)
}

unsafe fn wake(ptr: *const ()) {
    let arc = Arc::from_raw(ptr as *const MyWaker);
    arc.wake();
}

fn create_waker(waker: Arc<MyWaker>) -> Waker {
    let raw = RawWaker::new(Arc::into_raw(waker) as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw) }
}
```

### Waker Optimization: Atomic Waker

For single-threaded executors, wakers can be optimized:

```rust
use atomic_waker::AtomicWaker;
use std::sync::Arc;

struct SharedState {
    waker: AtomicWaker,
    data: Arc<Mutex<Option<Data>>>,
}

// When I/O completes
fn on_io_complete(state: &SharedState) {
    state.waker.wake();  // Atomic wake, no locks needed
}
```

---

## Part 4: Task Scheduling

### Task Structure

```rust
struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
    state: TaskState,
    waker: Waker,
}

enum TaskState {
    Ready,      // Ready to poll
    Running,    // Currently polling
    Pending,    // Waiting for event
    Completed,  // Done
}
```

### Scheduling Algorithms

#### 1. FIFO (First-In-First-Out)

```rust
struct FifoScheduler {
    queue: VecDeque<TaskId>,
}

impl Scheduler for FifoScheduler {
    fn schedule(&mut self, task: TaskId) {
        self.queue.push_back(task);
    }

    fn next(&mut self) -> Option<TaskId> {
        self.queue.pop_front()
    }
}
```

#### 2. Priority Queue

```rust
use std::collections::BinaryHeap;
use std::cmp::Ordering;

struct PriorityTask {
    priority: u32,  // Higher = more important
    task: TaskId,
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

struct PriorityScheduler {
    queue: BinaryHeap<PriorityTask>,
}

impl Scheduler for PriorityScheduler {
    fn schedule(&mut self, task: TaskId, priority: u32) {
        self.queue.push(PriorityTask { priority, task });
    }

    fn next(&mut self) -> Option<TaskId> {
        self.queue.pop().map(|t| t.task)
    }
}
```

#### 3. Multi-Level Feedback Queue

```rust
// Tasks start at high priority, get demoted if they use too much CPU
struct MlFqScheduler {
    queues: Vec<VecDeque<TaskId>>,  // Level 0 = highest priority
    current_level: usize,
}

impl MlFqScheduler {
    fn schedule(&mut self, task: TaskId) {
        // New tasks start at highest priority
        self.queues[0].push_back(task);
    }

    fn demote(&mut self, task: TaskId) {
        // Move to lower priority queue
        let next_level = (self.current_level + 1).min(self.queues.len() - 1);
        self.queues[next_level].push_back(task);
    }

    fn next(&mut self) -> Option<TaskId> {
        // Round-robin between levels
        for (level, queue) in self.queues.iter_mut().enumerate() {
            if let Some(task) = queue.pop_front() {
                self.current_level = level;
                return Some(task);
            }
        }
        None
    }
}
```

---

## Part 5: Embassy Deep Dive

### Embassy Architecture

Embassy is an embedded async runtime that works without std or heap:

```
┌───────────────────────────────────────┐
│         Embassy Executor              │
│                                       │
│  Static task pool (compile-time)      │
│  ┌──────┐ ┌──────┐ ┌──────┐          │
│  │Task 0│ │Task 1│ │Task 2│ ...      │
│  └──────┘ └──────┘ └──────┘          │
│                                       │
│  No dynamic allocation                │
│  All memory known at compile time     │
└───────────────────────────────────────┘
```

### Embassy Task Macro

```rust
// Embassy task - runs on embassy executor
#[embassy_executor::task]
async fn blink_led(mut led: Output<'static, AnyPin>) {
    loop {
        led.set_high();
        Timer::after_millis(150).await;
        led.set_low();
        Timer::after_millis(150).await;
    }
}

// Main function - embassy entry point
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // Spawn task
    spawner.spawn(blink_led(
        Output::new(p.P0_13.degrade(), Level::Low, OutputDrive::Standard)
    )).unwrap();
}
```

### No Heap Allocation

Embassy uses compile-time allocation:

```rust
// Static executor state
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // No Box, no Vec, no dynamic allocation
    // All tasks use static memory
}

// Task pool defined at compile time
#[embassy_executor::task_pool]
const TASK_POOL: TaskPool<Task> = TaskPool::new();
```

### Timer Implementation

```rust
// Embassy timer - no std::time
use embassy_time::{Duration, Timer};

async fn blink() {
    loop {
        // Uses hardware timer, no OS needed
        Timer::after(Duration::from_millis(100)).await;
    }
}
```

---

## Part 6: Futures-Concurrency

### Structured Concurrency

futures-concurrency provides operations for combining futures:

```rust
use futures_concurrency::prelude::*;
use std::future;

// Join multiple futures
let a = future::ready(1u8);
let b = future::ready("hello");
let c = future::ready(3u16);
assert_eq!((a, b, c).join().await, (1, "hello", 3));
```

### Operations

| Operation | Description |
|-----------|-------------|
| `join` | Wait for all futures |
| `try_join` | Wait for all, short-circuit on error |
| `race` | First future wins |
| `race_ok` | First Ok wins |
| `merge` | Merge streams |
| `zip` | Pair items from streams |

### Join Implementation

```rust
// Conceptual join implementation
struct Join3<A, B, C> {
    a: Option<A>,
    b: Option<B>,
    c: Option<C>,
}

impl<A, B, C> Future for Join3<A, B, C>
where
    A: Future,
    B: Future,
    C: Future,
{
    type Output = (A::Output, B::Output, C::Output);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_ready = true;

        // Poll each future
        if let Poll::Ready(a) = self.a.as_mut().unwrap().poll(cx) {
            self.a = None;
        } else {
            all_ready = false;
        }

        // ... same for b and c ...

        if all_ready {
            Poll::Ready((/* collected outputs */))
        } else {
            Poll::Pending
        }
    }
}
```

---

## Part 7: Moro - Async Scoping

### The Problem

Standard async doesn't allow borrowing from stack:

```rust
// This doesn't work in standard async
fn parent() {
    let data = vec![1, 2, 3];

    tokio::spawn(async move {
        // ERROR: data may not live long enough
        println!("{:?}", data);
    });
}
```

### Moro Solution

Moro provides scoped async:

```rust
use moro::async_scope;

let data = vec![1, 2, 3];
let result = async_scope!(|scope| {
    let future = scope.spawn(async {
        // OK: scope ensures task completes before returning
        data.iter().sum::<i32>()
    });
    future.await
}).await;

assert_eq!(result, 6);
```

### How Moro Works

1. Spawned tasks run **concurrently** (not parallel)
2. Tasks are tied to scope lifetime
3. Scope waits for all tasks before returning
4. No `'static` requirement

---

## Part 8: Async Patterns

### Fan-Out/Fan-In

```rust
use tokio::task::JoinSet;

async fn fan_out_fan_in(urls: Vec<&str>) -> Vec<String> {
    let mut set = JoinSet::new();

    // Fan out
    for url in urls {
        set.spawn(async move {
            fetch(url).await
        });
    }

    // Fan in
    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        results.push(result.unwrap());
    }

    results
}
```

### Bounded Concurrency

```rust
use futures::stream::{StreamExt, FuturesUnordered};

async fn bounded_concurrency<I, O, F>(
    items: Vec<I>,
    limit: usize,
    f: impl Fn(I) -> F,
) -> Vec<O>
where
    F: Future<Output = O>,
{
    let mut results = Vec::new();
    let mut futures = FuturesUnordered::new();
    let mut iter = items.into_iter();

    // Seed initial batch
    for _ in 0..limit {
        if let Some(item) = iter.next() {
            futures.push(f(item));
        }
    }

    // Process remaining
    while let Some(result) = futures.next().await {
        results.push(result);

        if let Some(item) = iter.next() {
            futures.push(f(item));
        }
    }

    results
}
```

### Timeout

```rust
use tokio::time::{timeout, Duration};

async fn with_timeout<T, F>(future: F, dur: Duration) -> Option<T>
where
    F: Future<Output = T>,
{
    match timeout(dur, future).await {
        Ok(result) => Some(result),
        Err(_) => None,  // Timed out
    }
}
```

### Retry

```rust
async fn retry<T, E, F, Fut>(
    mut f: F,
    max_retries: u32,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut last_error = None;

    for _ in 0..=max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap())
}
```

---

## Part 9: Performance Considerations

### Async Overhead

| Operation | Cost |
|-----------|------|
| Creating Future | ~0 (compile-time) |
| Poll (no I/O) | ~10ns |
| Context switch | ~100ns |
| Thread context switch | ~1-10μs |

### Memory Layout

```
Future State Machine (compiled):

async fn example() -> i32 {
    let x = 1;        // State 0
    let y = fetch().await;  // State 1 (suspend point)
    x + y             // State 2
}

// Compiler generates:
enum ExampleFuture {
    State0 { x: i32 },
    State1 { x: i32, y: Waiting },
    State2 { result: i32 },
    Complete,
}
```

### Best Practices

1. **Minimize await points** - Each await is a suspension point
2. **Batch operations** - Reduce context switches
3. **Use bounded channels** - Prevent memory growth
4. **Yield periodically** - Be a good citizen
5. **Pin judiciously** - Only when needed

---

## Part 10: Embassy vs Tokio Comparison

| Aspect | Embassy | Tokio |
|--------|---------|-------|
| Target | Embedded (no-std) | Servers (std) |
| Memory | Static allocation | Heap allocation |
| Wakers | Atomic, no alloc | Heap-allocated |
| Timer | Hardware-based | OS-based |
| Threading | Single-threaded | Multi-threaded |
| Size | ~10KB | ~1MB+ |

---

*This document is a living guide. Revisit as concepts become clearer through practice.*
