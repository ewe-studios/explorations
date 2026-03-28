---
title: "Zero to Concurrency Engineer"
subtitle: "First principles: threads, async, actors, and CSP"
parent: exploration.md
---

# Zero to Concurrency Engineer: First Principles

## Introduction

Welcome to the complete guide to understanding concurrency from first principles. This document assumes **zero prior knowledge** of concurrency and builds up to advanced concepts used in production systems.

By the end of this document, you will understand:
- What concurrency is and why it matters
- The three main concurrency models (threads, async, actors)
- How CSP (Communicating Sequential Processes) works
- When to use each model
- How Rust implements these patterns safely

---

## Part 1: What is Concurrency?

### The Problem

Imagine you're building a web server. You need to handle multiple client requests at the same time. Without concurrency, your server would:
1. Accept request A
2. Process request A (maybe 100ms for database query)
3. Send response A
4. Accept request B
5. ... and so on

This is **sequential** processing. Your server handles one request at a time. If request A waits for a database, the server does nothing during that wait.

### The Solution: Concurrency

**Concurrency** means making progress on multiple tasks "at the same time". Note: "at the same time" doesn't always mean literally simultaneous (that's **parallelism**). Concurrency is about **managing multiple tasks** that may overlap in time.

```
Sequential:
Request A: [=====processing=====][==waiting==][==done==]
Request B:                         [=====processing=====][==waiting==][==done==]
Request C:                                              [=====processing=====]...

Concurrent:
Request A: [==proc==][wait][==proc==][wait][done]
Request B:          [==proc==][wait][==proc==][wait][done]
Request C:                    [==proc==][wait][==proc==][wait][done]
```

### Concurrency vs Parallelism

| Concurrency | Parallelism |
|-------------|-------------|
| Multiple tasks **in progress** | Multiple tasks **executing simultaneously** |
| Can happen on single CPU | Requires multiple CPUs |
| About **structure** | About **execution** |
| Task switching | True simultaneity |

**Key insight:** You can have concurrency without parallelism (single-core multitasking), and parallelism without concurrency (batch processing on multiple cores).

---

## Part 2: The Thread Model

### What is a Thread?

A **thread** is the smallest unit of execution that can be scheduled by an operating system. Think of it as an independent "worker" that runs your code.

```rust
use std::thread;

fn main() {
    // Spawn a new thread
    let handle = thread::spawn(|| {
        println!("Hello from the thread!");
    });

    // Wait for thread to complete
    handle.join().unwrap();
}
```

### Thread Lifecycle

```
┌──────────┐
│  Created │
└────┬─────┘
     │
     ▼
┌──────────┐     timeout     ┌──────────┐
│  Ready   │ ──────────────► │ Blocked  │
└────┬─────┘                 └────┬─────┘
     ▲                            │
     │                            │ I/O complete
     │ scheduler                  │
     └────────────────────────────┘
     │
     ▼
┌──────────┐
│ Running  │
└────┬─────┘
     │
     │ task completes
     ▼
┌──────────┐
│  Done    │
└──────────┘
```

### Thread Pools

Creating threads is expensive. A **thread pool** pre-creates a set of worker threads and reuses them:

```
┌─────────────────────────────────────────┐
│            Thread Pool                   │
│  ┌────────┐ ┌────────┐ ┌────────┐       │
│  │Worker 1│ │Worker 2│ │Worker 3│  ... │
│  └───┬────┘ └───┬────┘ └───┬────┘       │
│      │          │          │            │
│      └──────────┴──────────┘            │
│              Task Queue                  │
└─────────────────────────────────────────┘
```

```rust
// Conceptual example
let pool = ThreadPool::new(4);  // 4 worker threads

for i in 0..100 {
    pool.execute(move || {
        println!("Task {} running", i);
    });
}
```

### Work Stealing

In **work stealing**, idle threads steal tasks from busy threads:

```
Thread 0: [Task A][Task B][Task C]  ◄─── Thread 1 steals from here
Thread 1: []  ◄─── idle, looking for work
Thread 2: [Task D]
```

This balances load automatically without central coordination.

### When to Use Threads

| Use Threads When | Avoid Threads When |
|-----------------|-------------------|
| CPU-bound work | Mostly I/O-bound |
| Need true parallelism | Many concurrent connections |
| Blocking operations | Low-latency requirements |
| Simple mental model | Memory is constrained |

---

## Part 3: The Async Model

### The Problem with Threads

Threads have overhead:
- Each thread needs a stack (typically 1-8 MB)
- Context switching between threads is expensive
- OS limits the number of threads (~1000s, not millions)

For high-concurrency servers (handling 10,000+ connections), threads don't scale.

### Enter Async/Await

**Async/await** is a programming model that allows many tasks to run **cooperatively** on a smaller number of threads.

```rust
// Synchronous (blocking)
fn fetch_data() -> Data {
    let response = http.get(url);  // Blocks here
    response.json()
}

// Asynchronous (non-blocking)
async fn fetch_data() -> Data {
    let response = http.get(url).await;  // Yields here
    response.json().await
}
```

### Futures and Poll

At the heart of async Rust is the **Future** trait:

```rust
trait Future {
    type Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

enum Poll<T> {
    Ready(T),      // Value is ready
    Pending,       // Not ready yet, try again later
}
```

**How it works:**
1. You create a Future (async function)
2. An **executor** polls the Future
3. If `Pending`, the Future registers a **Waker**
4. When the I/O completes, the Waker is called
5. Executor polls again, gets `Ready(value)`

### Executors

An **executor** is the runtime that drives Futures to completion:

```rust
// Tokio example
#[tokio::main]
async fn main() {
    // Tokio executor runs this
    let task1 = tokio::spawn(async { /* ... */ });
    let task2 = tokio::spawn(async { /* ... */ });

    task1.await.unwrap();
    task2.await.unwrap();
}
```

```
┌──────────────────────────────────┐
│           Executor                │
│  ┌──────────────────────────┐    │
│  │     Task Queue            │    │
│  │  [Task A][Task B][Task C] │    │
│  └──────────────────────────┘    │
│              │                    │
│              ▼                    │
│  ┌──────────────────────────┐    │
│  │     Poll Loop             │    │
│  │  poll A → Pending         │    │
│  │  poll B → Ready(x)        │    │
│  │  poll C → Pending         │    │
│  └──────────────────────────┘    │
└──────────────────────────────────┘
```

### Cooperative Scheduling

Async in Rust is **cooperative**: tasks must explicitly yield control via `.await`:

```rust
// Bad: hogs the executor
async fn bad_loop() {
    for i in 0..1_000_000 {
        // Never yields!
        process(i);
    }
}

// Good: yields periodically
async fn good_loop() {
    for i in 0..1_000_000 {
        if i % 100 == 0 {
            tokio::task::yield_now().await;  // Yield point
        }
        process(i);
    }
}
```

### Wakers

A **Waker** tells the executor "this task is ready to be polled again":

```rust
// Conceptual flow
1. Future returns Poll::Pending
2. Future saves Waker callback with I/O system
3. I/O system calls Waker when data arrives
4. Waker notifies executor to re-poll the Future
5. Executor polls, gets Poll::Ready(value)
```

### When to Use Async

| Use Async When | Avoid Async When |
|---------------|-----------------|
| High concurrency (1000s of connections) | Simple, sequential code |
| I/O-bound work | CPU-bound work |
| Low latency requirements | Blocking libraries |
| Resource constrained | Need true parallelism |

---

## Part 4: The Actor Model

### The Problem with Shared State

Traditional concurrency uses **shared memory** with locks:

```rust
// Shared state with mutex
let counter = Arc::new(Mutex::new(0));

thread::spawn({
    let counter = counter.clone();
    move || {
        *counter.lock().unwrap() += 1;  // Lock, modify, unlock
    }
});
```

Problems:
- Lock contention (threads wait for locks)
- Deadlocks (circular wait)
- Race conditions (incorrect lock usage)
- Hard to reason about

### The Actor Solution

The **Actor Model** eliminates shared state. Instead:
- **Actors** are independent units with isolated state
- Actors communicate via **messages**
- No shared memory = no locks needed

```
┌────────────┐    message    ┌────────────┐
│  Actor A   │ ─────────────►│  Actor B   │
│  (state)   │               │  (state)   │
│  [mailbox] │               │  [mailbox] │
└────────────┘               └────────────┘
```

### Actor Structure

```rust
// Conceptual actor
struct CounterActor {
    count: u64,
    mailbox: Receiver<Message>,
}

enum Message {
    Increment,
    GetCount(Sender<u64>),
}

impl CounterActor {
    async fn run(mut self) {
        while let Some(msg) = self.mailbox.recv().await {
            match msg {
                Message::Increment => self.count += 1,
                Message::GetCount(reply_to) => {
                    reply_to.send(self.count).await.unwrap();
                }
            }
        }
    }
}
```

### Message Passing

Actors communicate only through messages:

```rust
// Send a message
actor_mailbox.send(Message::Increment).await?;

// Ask for data
let (sender, receiver) = channel();
actor_mailbox.send(Message::GetCount(sender)).await?;
let count = receiver.recv().await?;
```

### Supervision

Actors can be organized in **supervision trees**:

```
         Supervisor
        /     |     \
    Actor A Actor B Actor C
     /  \           /  \
    D    E         F    G
```

If Actor D crashes:
1. Supervisor is notified
2. Supervisor decides: restart D, escalate, or ignore
3. Other actors (E, F, G) are unaffected

### When to Use Actors

| Use Actors When | Avoid Actors When |
|----------------|------------------|
| Distributed systems | Simple, local state |
| Fault tolerance needed | Performance critical |
| Complex state management | Tight coupling needed |
| Event-driven architecture | Batch processing |

---

## Part 5: CSP (Communicating Sequential Processes)

### What is CSP?

**CSP** is a formal language for describing concurrency patterns. Key ideas:
- **Processes** run independently
- Processes communicate via **channels**
- **Synchronous** channels: sender and receiver must both be ready
- **Asynchronous** channels: buffer messages

### Channels in CSP

```
Process A          Channel           Process B
    │                 │                  │
    │──send(x)───────►│                  │
    │                 │◄──────recv()─────│
    │                 │                  │
    │                 │──────x──────────►│
```

### Channels in Rust

```rust
use std::sync::mpsc;

let (tx, rx) = mpsc::channel();

// Sender
tx.send("Hello").unwrap();

// Receiver
let msg = rx.recv().unwrap();
```

### Bounded vs Unbounded

| Bounded Channel | Unbounded Channel |
|-----------------|-------------------|
| Fixed buffer size | Unlimited buffer |
| Sender blocks if full | Sender never blocks |
| Backpressure | No backpressure |
| Memory bounded | Memory unbounded |

```rust
// Bounded channel (capacity 5)
let (tx, rx) = mpsc::sync_channel(5);
tx.send(x).unwrap();  // Blocks if buffer full

// Unbounded channel
let (tx, rx) = mpsc::channel();
tx.send(x).unwrap();  // Never blocks
```

### Select Patterns

CSP introduces **select** for choosing between channels:

```rust
// Conceptual select
select! {
    msg = channel1.recv() => handle(msg),
    msg = channel2.recv() => handle(msg),
    timeout = timer => handle_timeout(),
}
```

---

## Part 6: Model Comparison

### Thread Model vs Async Model

| Aspect | Thread Model | Async Model |
|--------|--------------|-------------|
| Memory per task | ~1-8 MB stack | ~KB for Future |
| Context switch | OS kernel, expensive | User-space, cheap |
| Max concurrent | ~1000s | ~1,000,000+ |
| Programming model | Preemptive | Cooperative |
| Best for | CPU-bound | I/O-bound |

### Async Model vs Actor Model

| Aspect | Async Model | Actor Model |
|--------|-------------|-------------|
| State sharing | Shared memory (with care) | Message passing only |
| Failure isolation | Limited | Strong |
| Distribution | Requires effort | Natural |
| Message overhead | None | Serialization needed |

### CSP vs Actor Model

| Aspect | CSP | Actor Model |
|--------|-----|-------------|
| Communication | Channels | Mailboxes |
| Addressing | Channel-based | Actor address |
| State | External to channels | Internal to actor |
| Supervision | Not defined | Built-in |

---

## Part 7: Rust's Safety Guarantees

### Ownership and Concurrency

Rust's ownership system prevents data races at compile time:

```rust
// This does NOT compile - prevents data race
fn data_race() {
    let mut data = 0;

    thread::spawn(|| {
        data += 1;  // Error: may outlive main thread
    });

    data += 1;  // Error: borrowed value
}

// This compiles - safe with Arc<Mutex<>>
use std::sync::{Arc, Mutex};

fn safe_concurrency() {
    let data = Arc::new(Mutex::new(0));

    let data_clone = data.clone();
    thread::spawn(move || {
        *data_clone.lock().unwrap() += 1;
    });

    *data.lock().unwrap() += 1;
}
```

### Send and Sync

Rust uses two traits for thread safety:

```rust
// Send: can be moved to another thread
unsafe trait Send {}

// Sync: can be shared between threads (&T is Send)
unsafe trait Sync {}

// Examples
let x: i32 = 42;  // i32: Send + Sync
let y: Rc<i32>;   // Rc: !Send, !Sync (reference counted, not thread-safe)
let z: Arc<i32>;  // Arc: Send + Sync (atomic reference counted)
```

| Type | Send | Sync |
|------|------|------|
| `i32`, `String` | ✓ | ✓ |
| `Rc<T>` | ✗ | ✗ |
| `Arc<T>` | ✓ | ✓ |
| `Mutex<T>` | ✓ | ✓ |
| `Cell<T>` | ✓ | ✗ |

---

## Part 8: Choosing the Right Model

### Decision Tree

```
                    Start
                      │
                      ▼
           ┌──────────────────┐
           │ CPU or I/O bound?│
           └────────┬─────────┘
                    │
         ┌──────────┴──────────┐
         │                     │
       CPU                   I/O
         │                     │
         ▼                     ▼
    ┌──────────┐        ┌──────────┐
    │Parallel? │        │Concurrent│
    └────┬─────┘        │connections?│
         │              └────┬─────┘
    ┌────┴─────┐             │
    │          │      ┌──────┴──────┐
   Yes        No    Few (<100)  Many (>1000)
    │          │       │            │
    ▼          │       ▼            ▼
┌────────┐    │  ┌────────┐  ┌──────────┐
│Threads │    │  │Threads │  │  Async   │
│(pool)  │    │  │        │  │Executor  │
└────────┘    │  └────────┘  └──────────┘
              │
              ▼
         ┌──────────┐
         │Distributed│
         │or isolated│
         │  state?   │
         └────┬──────┘
              │
       ┌──────┴──────┐
       │             │
      Yes           No
       │             │
       ▼             │
   ┌────────┐        │
   │ Actors │        │
   └────────┘        │
                     ▼
              ┌──────────┐
              │   CSP    │
              │ Channels │
              └──────────┘
```

### Practical Examples

| Scenario | Recommended Model | Why |
|----------|-------------------|-----|
| Web server (10k connections) | Async | High concurrency, I/O-bound |
| Video encoding | Thread pool | CPU-bound, parallel |
| Chat system | Actors | Distributed, fault-tolerant |
| Database connection pool | CSP channels | Bounded resources |
| Game physics | Thread pool | Parallel computation |
| IoT device (embedded) | Embassy async | No-std, resource constrained |

---

## Part 9: Next Steps

### Hands-On Exercises

1. **Build a thread pool** - Implement basic work stealing
2. **Create a simple executor** - Understand Poll and Waker
3. **Implement actors** - Message passing without locks
4. **Channel patterns** - Producer/consumer with backpressure

### Further Reading

- [Rust Book: Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [Tokio Documentation](https://tokio.rs/)
- [Embassy Book](https://embassy.dev/book/)
- [Glommio Documentation](https://docs.rs/glommio/)
- "Communicating Sequential Processes" by C.A.R. Hoare

### Moving Forward

Continue with:
- [Thread Model Deep Dive](01-thread-model-deep-dive.md) - Thread pools, work stealing, scheduling
- [Async Model Deep Dive](02-async-model-deep-dive.md) - Futures, executors, wakers
- [Actor Model Deep Dive](03-actor-model-deep-dive.md) - Message passing, supervision
- [Sync Primitives Deep Dive](04-sync-primitives-deep-dive.md) - Mutexes, channels, semaphores

---

*This document is a living guide. Revisit as concepts become clearer through practice.*
