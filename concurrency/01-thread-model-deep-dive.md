---
title: "Thread Model Deep Dive"
subtitle: "Thread pools, work stealing, and scheduling"
parent: exploration.md
---

# Thread Model Deep Dive

## Introduction

This document provides a comprehensive deep dive into the thread model of concurrency, covering thread pools, work stealing, scheduling strategies, and the innovative thread-per-core architecture pioneered by frameworks like Glommio.

---

## Part 1: Thread Fundamentals

### What is a Thread?

A **thread** (thread of execution) is the smallest sequence of programmed instructions that can be managed independently by a scheduler. In modern operating systems:

```
Process
├── Thread 1 (stack, registers, program counter)
├── Thread 2 (stack, registers, program counter)
├── Thread 3 (stack, registers, program counter)
└── Shared Memory (heap, code, data)
```

### Thread States

```
                    ┌──────────────┐
                    │   Created    │
                    └──────┬───────┘
                           │
                           ▼
                    ┌──────────────┐
             ┌─────►│    Ready     │◄─────┐
             │      └──────┬───────┘      │
             │             │              │
  preempted │             │ dispatched     │ I/O complete
   (timer)  │             │     by         │ or signal
             │             │  scheduler     │
             │             ▼              │
             │      ┌──────────────┐      │
             │      │   Running    │      │
             │      └──────┬───────┘      │
             │             │              │
             │             │ blocking     │
             │             │ operation    │
             │             │ (I/O, lock)  │
             │             ▼              │
             │      ┌──────────────┐      │
             └──────│   Blocked    │──────┘
                    └──────────────┘
                           │
                           │ task completes
                           ▼
                    ┌──────────────┐
                    │   Terminated │
                    └──────────────┘
```

### Thread Creation Overhead

Creating a thread involves:

1. **Stack allocation** (typically 1-8 MB per thread)
2. **Register initialization**
3. **OS kernel structures**
4. **Scheduler registration**

```rust
// Each thread has its own stack
use std::thread;

fn main() {
    // Stack allocation happens here
    let handle = thread::Builder::new()
        .stack_size(4 * 1024 * 1024)  // 4MB stack
        .spawn(|| {
            // Thread code
        })
        .unwrap();

    handle.join().unwrap();
}
```

### Context Switching

When the OS switches between threads:

1. Save current thread's registers
2. Update memory management structures
3. Load next thread's registers
4. Resume execution

**Cost:** Typically 1-10 microseconds per switch

---

## Part 2: Thread Pools

### The Problem

Creating/destroying threads is expensive. For short-lived tasks:

```rust
// BAD: Creating threads for short tasks
for i in 0..1000 {
    thread::spawn(move || {
        // 1ms of work
    });
}
// Thread creation takes longer than the work!
```

### Thread Pool Solution

A **thread pool** maintains a fixed number of worker threads:

```
┌─────────────────────────────────────────┐
│           Thread Pool                    │
│                                          │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐   │
│  │ Worker  │ │ Worker  │ │ Worker  │   │
│  │ Thread  │ │ Thread  │ │ Thread  │   │
│  │    1    │ │    2    │ │    3    │   │
│  └────┬────┘ └────┬────┘ └────┬────┘   │
│       │          │          │          │
│       └──────────┴──────────┘          │
│            Task Queue                  │
│       [T1][T2][T3][T4][T5]...         │
└─────────────────────────────────────────┘
```

### Basic Implementation

```rust
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
```

### Sizing Thread Pools

**CPU-bound work:**
- `threads = num_cpus`
- One thread per CPU core
- Minimizes context switching

**I/O-bound work:**
- `threads = num_cpus * 2` or more
- Threads wait for I/O
- More threads keep CPUs busy

```rust
// Optimal sizing
use num_cpus;

let cpu_pool = ThreadPool::new(num_cpus::get());
let io_pool = ThreadPool::new(num_cpus::get() * 4);
```

---

## Part 3: Work Stealing

### The Problem with Work Queues

Single work queue creates contention:

```
┌─────────┐  ┌─────────┐  ┌─────────┐
│ Worker 1│  │ Worker 2│  │ Worker 3│
└────┬────┘  └────┬────┘  └────┬────┘
     │           │           │
     └───────────┴───────────┘
             │
      ┌──────▼──────┐
      │ Shared Queue│  ◄── Contention!
      │ [T1][T2][T3]│      All workers lock
      └─────────────┘      to get tasks
```

### Work Stealing Solution

Each worker has its own **deque** (double-ended queue):

```
Worker 0: [T1, T2, T3]  ◄─── pop from front (LIFO)
Worker 1: []  ◄─── idle, will steal
Worker 2: [T4, T5]
```

- Workers push/pop from **their own** deque (no locks)
- Idle workers **steal** from others' deque back (FIFO)

### Algorithm

```rust
// Conceptual work stealing
struct WorkStealingPool {
    workers: Vec<WorkerDeque>,
}

struct WorkerDeque<T> {
    deque: ConcurrentDeque<T>,  // Lock-free deque
}

impl<T> WorkerDeque<T> {
    fn push(&self, task: T) {
        self.deque.push_front(task);  // LIFO for owner
    }

    fn pop(&self) -> Option<T> {
        self.deque.pop_front()  // Owner takes from front
    }

    fn steal(&self) -> Option<T> {
        self.deque.pop_back()  // Thief takes from back
    }
}
```

### Benefits

1. **No contention** for local work
2. **Load balancing** via stealing
3. **Cache locality** (keep working on local data)
4. **Scalability** (O(1) local operations)

### Rayon's Approach

```rust
// Rayon uses work stealing internally
use rayon::prelude::*;

let sum: i32 = (0..1000000)
    .into_par_iter()  // Parallel iterator
    .sum();
```

---

## Part 4: Thread-Per-Core Architecture

### The Innovation

**Thread-per-core** assigns one thread to each CPU core, with **no work stealing**:

```
CPU 0        CPU 1        CPU 2        CPU 3
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│Executor│  │Executor│  │Executor│  │Executor│
│ Thread │  │ Thread │  │ Thread │  │ Thread │
│   0    │  │   1    │  │   2    │  │   3    │
└───┬────┘  └───┬────┘  └───┬────┘  └───┬────┘
    │           │           │           │
    │  Task Q   │  Task Q   │  Task Q   │  Task Q
    │[T1,T2,T3] │[T4,T5,T6] │[T7,T8,T9] │[...]
    │           │           │           │
```

### Glommio's Approach

Glommio implements cooperative thread-per-core:

```rust
use glommio::{LocalExecutorBuilder, Placement};

// Pin executor to CPU 0
LocalExecutorBuilder::new(Placement::Fixed(0))
    .spawn(|| async move {
        // All work runs on CPU 0
        // No cross-thread synchronization needed
    })
    .unwrap()
    .join();
```

### Why No Work Stealing?

1. **Cost of stealing** - Moving tasks between threads has overhead
2. **Cache locality** - Tasks stay on same CPU (better cache hit rates)
3. **Simplicity** - No locks needed for task queues
4. **Predictability** - Deterministic scheduling

### When to Use Thread-Per-Core

| Use Thread-Per-Core | Avoid Thread-Per-Core |
|--------------------|----------------------|
| High-throughput I/O | Irregular workloads |
| Low-latency systems | Tasks with varying duration |
| io_uring (Linux) | Cross-core communication needed |
| Predictable performance | Load balancing critical |

---

## Part 5: CPU Pinning

### What is CPU Pinning?

**CPU pinning** (affinity) binds a thread to specific CPU cores:

```rust
use std::thread;

// Bind thread to CPU 0
let handle = thread::spawn(|| {
    // This thread runs on CPU 0
});

// Set CPU affinity (platform-specific)
#[cfg(target_os = "linux")]
{
    use core_affinity;
    let cores = core_affinity::get_core_ids().unwrap();
    core_affinity::set_for_current(cores[0]);
}
```

### Benefits

1. **Cache locality** - Data stays in CPU cache
2. **NUMA awareness** - Access local memory
3. **Predictability** - No migration overhead
4. **Isolation** - Critical threads don't compete

### NUMA Considerations

```
┌──────────────────────────────────────┐
│              System                   │
│  ┌─────────────┐ ┌─────────────┐     │
│  │   NUMA 0    │ │   NUMA 1    │     │
│  │  ┌───────┐  │ │  ┌───────┐  │     │
│  │  │ CPU 0 │  │ │  │ CPU 2 │  │     │
│  │  │ CPU 1 │  │ │  │ CPU 3 │  │     │
│  │  └───┬───┘  │ │  └───┬───┘  │     │
│  │      │      │ │      │      │     │
│  │  ┌───▼───┐  │ │  ┌───▼───┐  │     │
│  │  │Memory │  │ │  │Memory │  │     │
│  │  │ Local│  │ │  │ Local│  │     │
│  │  └───────┘  │ │  └───────┘  │     │
│  └─────────────┘ └─────────────┘     │
└──────────────────────────────────────┘

Accessing remote memory: 2-3x slower!
```

---

## Part 6: Glommio Architecture Deep Dive

### Three I/O Rings

Glommio uses three `io_uring` rings per thread:

```
┌─────────────────────────────────────────┐
│          Glommio Executor                │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │         Main Ring                 │   │
│  │  - Most operations                │   │
│  │  - Determines when to park        │   │
│  └──────────────────────────────────┘   │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │        Latency Ring               │   │
│  │  - Latency-sensitive operations   │   │
│  │  - Timer-based preemption         │   │
│  │  - Wakes main ring via fd         │   │
│  └──────────────────────────────────┘   │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │         Poll Ring                 │   │
│  │  - NVMe read/write (polling)     │   │
│  │  - No interrupts needed          │   │
│  │  - Best for high IOPS            │   │
│  └──────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

### Task Queues and Shares

```rust
use glommio::{executor, Latency, Shares};

// Create task queues with different priorities
let tq1 = executor().create_task_queue(
    Shares::Static(2),  // 2/3 of CPU time
    Latency::NotImportant,
    "high_priority"
);

let tq2 = executor().create_task_queue(
    Shares::Static(1),  // 1/3 of CPU time
    Latency::Matters(Duration::from_millis(10)),
    "low_priority"
);
```

### Cooperative Yielding

```rust
use glommio::yield_if_needed;

async fn long_running_task() {
    for i in 0..1000000 {
        // Check if latency-sensitive tasks need CPU
        if i % 100 == 0 {
            yield_if_needed().await;
        }
        // ... do work ...
    }
}
```

---

## Part 7: Scheduling Strategies

### Priority-Based Scheduling

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
enum Priority {
    High,
    Normal,
    Low,
}

struct Scheduler {
    queues: Vec<Vec<Task>>,  // One queue per priority
}

impl Scheduler {
    fn schedule(&mut self) -> Option<Task> {
        // Check high priority first
        for queue in &mut self.queues {
            if let Some(task) = queue.pop() {
                return Some(task);
            }
        }
        None
    }
}
```

### Fair Scheduling (Stride)

```rust
struct StrideScheduler {
    tasks: Vec<TaskState>,
}

struct TaskState {
    stride: u64,      // Inverse of share
    pass: u64,        // Current pass value
    task: Task,
}

impl StrideScheduler {
    fn select(&mut self) -> usize {
        // Select task with lowest pass value
        let mut min_idx = 0;
        let mut min_pass = self.tasks[0].pass;

        for (i, task) in self.tasks.iter().enumerate() {
            if task.pass < min_pass {
                min_pass = task.pass;
                min_idx = i;
            }
        }

        // Increment pass by stride
        self.tasks[min_idx].pass += self.tasks[min_idx].stride;

        min_idx
    }
}
```

### Lottery Scheduling

```rust
struct LotteryScheduler {
    tasks: Vec<TaskState>,
    total_tickets: u64,
}

struct TaskState {
    tickets: u64,     // Number of lottery tickets
    task: Task,
}

impl LotteryScheduler {
    fn select(&mut self) -> usize {
        let winning = rand::random::<u64>() % self.total_tickets;

        let mut cumulative = 0;
        for (i, task) in self.tasks.iter().enumerate() {
            cumulative += task.tickets;
            if winning < cumulative {
                return i;
            }
        }

        self.tasks.len() - 1
    }
}
```

---

## Part 8: Emb
