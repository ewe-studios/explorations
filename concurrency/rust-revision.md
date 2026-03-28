---
title: "Rust Revision"
subtitle: "Rust translation guide for concurrency patterns"
parent: exploration.md
---

# Rust Revision: Translation Guide

## Introduction

This document provides guidance on translating concurrency patterns from the source projects (Embassy, Glommio, futures-concurrency, moro) into Rust implementations using the valtron executor with the TaskIterator pattern.

**Note:** The source projects are already written in Rust. This guide focuses on translating their concurrency patterns to valtron's TaskIterator approach.

---

## Part 1: TaskIterator Fundamentals

### The TaskIterator Trait

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};

struct Counter {
    current: usize,
    max: usize,
}

impl TaskIterator for Counter {
    type Pending = ();
    type Ready = usize;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.current >= self.max {
            return None;  // Task complete
        }

        self.current += 1;
        Some(TaskStatus::Ready(self.current))
    }
}
```

### Comparison: Async vs TaskIterator

| Async/Tokio | Valtron TaskIterator |
|-------------|---------------------|
| `async fn` | `impl TaskIterator` |
| `.await` | `TaskStatus::Pending` |
| Return value | `TaskStatus::Ready(T)` |
| `tokio::spawn` | `spawn().with_task(task)` |
| `JoinHandle` | Iterator over status |

---

## Part 2: Translating Embassy Patterns

### Embassy Task → TaskIterator

**Embassy:**
```rust
#[embassy_executor::task]
async fn blink(mut led: Output<'static, AnyPin>) {
    loop {
        led.set_high();
        Timer::after_millis(150).await;
        led.set_low();
        Timer::after_millis(150).await;
    }
}
```

**Valtron:**
```rust
struct BlinkTask {
    led: Output,
    state: bool,
    timer: Timer,
}

impl TaskIterator for BlinkTask {
    type Pending = Duration;
    type Ready = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        // Toggle LED
        if self.state {
            self.led.set_high();
        } else {
            self.led.set_low();
        }
        self.state = !self.state;

        // Return pending with timer duration
        Some(TaskStatus::Pending(Duration::from_millis(150)))
    }
}
```

### Embassy Spawner → Valtron Spawner

**Embassy:**
```rust
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    spawner.spawn(my_task()).unwrap();
}
```

**Valtron:**
```rust
fn main() {
    initialize_pool(seed);

    spawn()
        .with_task(MyTask::new())
        .with_resolver(Box::new(FnReady::new(|item, _| {
            tracing::info!("Task completed: {:?}", item);
        })))
        .schedule()
        .expect("should deliver task");

    run_until_complete();
}
```

---

## Part 3: Translating Glommio Patterns

### Glommio Executor → Valtron Multi-Threaded

**Glommio:**
```rust
use glommio::{LocalExecutorBuilder, Placement};

LocalExecutorBuilder::new(Placement::Fixed(0))
    .spawn(|| async move {
        // Work on CPU 0
    })
    .unwrap()
    .join();
```

**Valtron:**
```rust
use foundation_core::valtron::multi::{block_on, get_pool};

block_on(seed, None, |pool| {
    pool.spawn()
        .with_task(MyTask::new())
        .with_resolver(Box::new(FnReady::new(|item, _| {
            println!("Result: {:?}", item);
        })))
        .schedule()
        .expect("should deliver task");
});
```

### Glommio Task Queues → Valtron Priority

**Glommio:**
```rust
let tq1 = executor().create_task_queue(
    Shares::Static(2),
    Latency::Matters(Duration::from_millis(10)),
    "high_priority"
);
```

**Valtron:**
```rust
// Use priority ordering in executor configuration
// Valtron uses PriorityOrder enum
```

---

## Part 4: Translating futures-concurrency Patterns

### Join Operation

**futures-concurrency:**
```rust
use futures_concurrency::prelude::*;

let a = async { 1 };
let b = async { 2 };
let c = async { 3 };
let result = (a, b, c).join().await;
```

**Valtron:**
```rust
use foundation_core::valtron::execute;

// Execute multiple tasks and collect results
let mut results = Vec::new();

for task in [Task1, Task2, Task3] {
    let stream = execute(task)?;
    for item in stream {
        results.push(item);
    }
}
```

### Merge Streams

**futures-concurrency:**
```rust
use futures_concurrency::prelude::*;

let merged = (stream1, stream2, stream3).merge();
while let Some(item) = merged.next().await {
    println!("{:?}", item);
}
```

**Valtron:**
```rust
use foundation_core::valtron::{execute_stream, Stream};

// Merge streams using multi-iterator
let mut merged = MultiIterator::new();
merged.add(execute_stream(stream1)?);
merged.add(execute_stream(stream2)?);

for item in merged {
    println!("{:?}", item);
}
```

---

## Part 5: Translating moro Patterns

### Scoped Async

**moro:**
```rust
use moro::async_scope;

let data = vec![1, 2, 3];
let result = async_scope!(|scope| {
    let future = scope.spawn(async {
        data.iter().sum::<i32>()
    });
    future.await
}).await;
```

**Valtron:**
```rust
// Valtron uses Rc/RefCell for shared state in single-threaded mode
use std::rc::Rc;
use std::cell::RefCell;

let data = Rc::new(RefCell::new(vec![1, 2, 3]));
let data_clone = data.clone();

initialize_pool(seed);

spawn()
    .with_task(SumTask::new(data_clone))
    .schedule()
    .expect("should deliver task");

run_until_complete();

let result = data.borrow().iter().sum();
```

---

## Part 6: Channel Patterns

### Tokio MPSC → Valtron

**Tokio:**
```rust
use tokio::sync::mpsc;

let (tx, rx) = mpsc::channel(100);

tx.send(msg).await.unwrap();
let msg = rx.recv().await.unwrap();
```

**Valtron:**
```rust
// Valtron uses concurrent queues
use concurrent_queue::ConcurrentQueue;

let queue = Arc::new(ConcurrentQueue::unbounded());

// Producer
let tx = queue.clone();
spawn()
    .with_task(ProducerTask { queue: tx, msg })
    .schedule()?;

// Consumer
spawn()
    .with_task(ConsumerTask { queue })
    .schedule()?;

run_until_complete();
```

---

## Part 7: Error Handling

### Result-based Tasks

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus};

struct FallibleTask {
    step: usize,
}

impl TaskIterator for FallibleTask {
    type Pending = ();
    type Ready = Result<String, Error>;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.do_work() {
            Ok(result) => Some(TaskStatus::Ready(Ok(result))),
            Err(e) => Some(TaskStatus::Ready(Err(e))),
        }
    }
}
```

---

## Part 8: State Machines

### Explicit State Machine

```rust
enum TaskState {
    Init,
    Waiting(Duration),
    Processing,
    Done(Result<String, Error>),
}

struct StateMachineTask {
    state: TaskState,
}

impl TaskIterator for StateMachineTask {
    type Pending = Duration;
    type Ready = Result<String, Error>;
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match &mut self.state {
            TaskState::Init => {
                self.state = TaskState::Waiting(Duration::from_millis(100));
                Some(TaskStatus::Pending(Duration::from_millis(100)))
            }
            TaskState::Waiting(_) => {
                self.state = TaskState::Processing;
                Some(TaskStatus::Pending(Duration::ZERO))
            }
            TaskState::Processing => {
                let result = self.do_work();
                self.state = TaskState::Done(result.clone());
                Some(TaskStatus::Ready(result))
            }
            TaskState::Done(_) => None,
        }
    }
}
```

---

## Part 9: Combining Tasks

### Sequential Execution

```rust
struct Sequential<A, B> {
    first: Option<A>,
    second: Option<B>,
    phase: u8,
}

impl<A, B> TaskIterator for Sequential<A, B>
where
    A: TaskIterator,
    B: TaskIterator,
{
    type Pending = ();
    type Ready = (A::Ready, B::Ready);
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.phase {
            0 => {
                // Run first task
                if let Some(ref mut first) = self.first {
                    match first.next() {
                        Some(TaskStatus::Ready(val)) => {
                            self.phase = 1;
                            // Store first result
                        }
                        Some(TaskStatus::Pending(p)) => {
                            return Some(TaskStatus::Pending(p));
                        }
                        None => {
                            self.phase = 1;
                        }
                    }
                }
                Some(TaskStatus::Pending(()))
            }
            1 => {
                // Run second task
                // Similar logic
                Some(TaskStatus::Pending(()))
            }
            _ => None,
        }
    }
}
```

---

## Part 10: Testing Patterns

### Unit Testing TaskIterator

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_task() {
        let mut task = Counter { current: 0, max: 5 };

        // First iteration
        match task.next() {
            Some(TaskStatus::Ready(1)) => {},
            _ => panic!("Expected Ready(1)"),
        }

        // Second iteration
        match task.next() {
            Some(TaskStatus::Ready(2)) => {},
            _ => panic!("Expected Ready(2)"),
        }

        // Continue until complete
        for i in 3..=5 {
            match task.next() {
                Some(TaskStatus::Ready(n)) => assert_eq!(n, i),
                _ => panic!("Expected Ready({})", i),
            }
        }

        // Should be complete
        assert!(task.next().is_none());
    }
}
```

---

*This guide provides patterns for translating common concurrency patterns to valtron's TaskIterator approach...*
