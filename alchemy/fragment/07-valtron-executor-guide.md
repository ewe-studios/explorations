# Valtron Executor Guide: Rust Async Without async/await

**Deep Dive 07** | Valtron Execution Model
**Source:** `foundation_core/src/valtron/` | **Date:** 2026-03-27

---

## Executive Summary

Valtron is an experimental async runtime built on **iterator-based execution** rather than the conventional async/await and Future paradigm. It provides deterministic, WASM-compatible task execution without requiring an async runtime, making it uniquely suited for Lambda environments and WebAssembly where traditional async runtimes face compatibility challenges.

This guide covers the complete Valtron architecture: from the core `TaskIterator` trait through single-threaded and multi-threaded executors, combinators, and the unified execution API.

---

## 1. Why Valtron?

### The Problems with async/await

Rust's async/await model, while ergonomic, introduces several fundamental challenges:

| Problem | Impact | Valtron's Solution |
|---------|--------|-------------------|
| **Runtime dependency** | Requires tokio/async-std runtime | No runtime - pure iterator driving |
| **WASM incompatibility** | Browser WASM has no threads, limited async | Single-threaded executor works natively |
| **Non-deterministic** | async task scheduling is opaque | Deterministic, step-by-step execution |
| **Lambda cold starts** | Runtime initialization adds latency | Zero runtime overhead |
| **Thread synchronization** | Requires Arc<Mutex<T>> everywhere | Rc<RefCell<T>> in single-threaded mode |

### WASM Compatibility Issues

Traditional async code in WASM faces these constraints:

```rust
// Standard async code - problematic in WASM
async fn fetch_data() -> Result<Data, Error> {
    let response = reqwest::get(url).await?;  // May not work in WASM
    let json = response.json().await?;
    Ok(json)
}

// Valtron approach - works everywhere
struct FetchTask { url: String }
impl TaskIterator for FetchTask {
    type Ready = Data;
    type Pending = NetworkState;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        // Explicit state machine - no hidden await
        match self.poll_network() {
            NetworkState::Pending => Some(TaskStatus::Pending(NetworkState::Waiting)),
            NetworkState::Complete(data) => Some(TaskStatus::Ready(data)),
            NetworkState::Done => None,
        }
    }
}
```

### Deterministic Execution

Valtron executes tasks in **explicit steps**, making debugging and testing predictable:

```
async/await execution (opaque):
  Task.spawn() -> [runtime internals] -> ??? -> Complete

Valtron execution (transparent):
  Task.next_status() -> Pending
  Task.next_status() -> Pending
  Task.next_status() -> Ready(value)
  Task.next_status() -> None (complete)
```

### Lambda Compatibility Without Async Runtime

AWS Lambda and similar FaaS platforms benefit from Valtron's approach:

- **No runtime initialization** - Code executes immediately
- **Predictable billing** - Each `next_status()` call is measurable
- **Clean shutdown** - No dangling async tasks on freeze

```rust
// Lambda handler with Valtron
#[handler]
async fn handler(event: ApiEvent) -> Result<Response, Error> {
    // Initialize single-threaded executor (zero overhead)
    valtron::single::initialize_pool(random_seed());

    // Execute task to completion
    let response = valtron::single::spawn()
        .with_task(ProcessRequest::new(event))
        .schedule_iter(Duration::from_millis(10))?;

    // Drive to completion deterministically
    valtron::single::run_until_complete();

    Ok(response.collect().next().unwrap())
}
```

---

## 2. TaskIterator Pattern

### The TaskStatus Enum

At the heart of Valtron is `TaskStatus`, which explicitly represents all possible states of an async operation:

```rust
pub enum TaskStatus<D, P, S: ExecutionAction> {
    /// Operation is still processing
    Pending(P),

    /// Initializing - middle state before Ready
    Init,

    /// Delayed by a specific duration
    Delayed(Duration),

    /// Result is ready (may occur multiple times for streams)
    Ready(D),

    /// Request to spawn a sub-task
    Spawn(S),

    /// Skip this item (used by filters)
    Ignore,
}
```

**Comparison to Future::Poll:**

| Future::Poll<T> | TaskStatus<D, P, S> |
|-----------------|---------------------|
| `Poll::Pending` | `TaskStatus::Pending(P)` |
| `Poll::Ready(T)` | `TaskStatus::Ready(D)` |
| *(none)* | `TaskStatus::Init` |
| *(none)* | `TaskStatus::Delayed(Duration)` |
| *(none)* | `TaskStatus::Spawn(S)` |
| *(none)* | `TaskStatus::Ignore` |

### The TaskIterator Trait

```rust
pub trait TaskIterator {
    /// Value type when task is Ready
    type Ready;

    /// Value type when task is Pending
    type Pending;

    /// Type that can spawn sub-tasks
    type Spawner: ExecutionAction;

    /// Advance the task and return its current status
    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>>;
}
```

### Complete Example: Counter Task

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use std::cell::RefCell;
use std::rc::Rc;

struct Counter {
    limit: usize,
    collected: Rc<RefCell<Vec<usize>>>,
}

impl Counter {
    fn new(limit: usize, collected: Rc<RefCell<Vec<usize>>>) -> Self {
        Self { limit, collected }
    }
}

impl TaskIterator for Counter {
    type Ready = usize;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        let mut items = self.collected.borrow_mut();
        let current = items.len();

        if current >= self.limit {
            return None; // Iterator complete
        }

        items.push(current);
        Some(TaskStatus::Ready(items.len()))
    }
}
```

### Comparison: Iterator vs Stream vs TaskIterator

| Trait | Output Type | Blocking | State Awareness | Use Case |
|-------|-------------|----------|-----------------|----------|
| `Iterator<Item = T>` | `T` | Yes | None | Synchronous collections |
| `Stream<Item = T>` | `T` | No | None | Async values (no state) |
| `TaskIterator` | `TaskStatus<D, P, S>` | No | Full (Pending/Delayed/Ready) | Async operations with state |

---

## 3. Single-Threaded Executor

The single-threaded executor uses `Rc<RefCell<T>>` instead of `Arc<Mutex<T>>`, eliminating synchronization overhead and enabling WASM compatibility.

### Core API

```rust
// Initialize with a seed (deterministic RNG for reproducibility)
initialize_pool(seed: u64);

// Spawn and schedule tasks
spawn()
    .with_task(my_task)
    .schedule()?;

// Drive execution
run_once();              // Execute one step
run_until(checker);      // Execute until condition
run_until_complete();    // Execute until all tasks done
```

### Example: Complete Single-Threaded Execution

```rust
use foundation_core::valtron::single::{initialize_pool, run_until_complete, spawn};
use foundation_core::valtron::FnReady;
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let seed = 42;
    let collected = Rc::new(RefCell::new(Vec::new()));

    // Initialize executor
    initialize_pool(seed);

    // Spawn task with resolver
    spawn()
        .with_task(Counter::new(5, collected.clone()))
        .with_resolver(Box::new(FnReady::new(|value, _executor| {
            println!("Got value: {}", value);
        })))
        .schedule()
        .expect("Task scheduled");

    // Run to completion
    run_until_complete();

    assert_eq!(*collected.borrow(), vec![0, 1, 2, 3, 4]);
}
```

### Iterator-Based Execution

Valtron provides non-blocking iterators for manual control:

```rust
// Get iterator over TaskStatus values
let status_iter = spawn()
    .with_task(Counter::new(5, collected.clone()))
    .schedule_iter(Duration::from_nanos(50))?;

// Manually drive execution
for status in status_iter {
    match status {
        TaskStatus::Ready(v) => println!("Value: {}", v),
        TaskStatus::Pending(_) => continue,
        _ => {}
    }
}
```

### Blocking Iterator

For synchronous code that wants Ready values only:

```rust
let value_iter = spawn()
    .with_task(Counter::new(5, collected.clone()))
    .blocking_iter()?;

for value in value_iter {
    // Blocks internally until next Ready value
    println!("Received: {}", value);
}
```

### Rc/RefCell Instead of Arc/Mutex

**Single-threaded (Valtron):**
```rust
struct Task {
    data: Rc<RefCell<Vec<usize>>>,  // No atomic overhead
}
```

**Multi-threaded (traditional async):**
```rust
struct Task {
    data: Arc<Mutex<Vec<usize>>>,   // Atomic operations required
}
```

Performance impact: `Rc`/`RefCell` is ~10x faster than `Arc`/`Mutex` for shared state access.

---

## 4. Multi-Threaded Executor

The multi-threaded executor distributes work across threads **without work stealing** - each task is owned by its thread from spawn to completion.

### Core API

```rust
// Block on tasks with setup closure
block_on(seed, thread_count, |pool| {
    pool.spawn()
        .with_task(my_task)
        .schedule()?;
});

// Get pool handle for spawning
let pool = get_pool();
pool.spawn().with_task(task).schedule()?;

// Signal all threads to stop
pool.kill();
```

### Work Distribution Without Stealing

```
Traditional work-stealing:
  Thread 1: [A1, A2, A3] ----steals----> [B1, B2]
  Thread 2: [B1, B2, B3, B4, B5]

Valtron (no stealing):
  Thread 1: [A1, A2, A3] --> complete
  Thread 2: [B1, B2, B3] --> complete
```

**Benefits:**
- No synchronization overhead for task queues
- Predictable task-to-thread affinity
- Cache locality (task stays on one thread)

### Thread Affinity for Tasks

```rust
// Task is assigned to a thread and stays there
block_on(seed, Some(4), |pool| {
    // Each task runs on its assigned thread
    for i in 0..100 {
        pool.spawn()
            .with_task(WorkerTask::new(i))
            .schedule()?;
    }
});
```

### Send + 'static Bounds

Multi-threaded execution requires:

```rust
where
    Task: TaskIterator + Send + 'static,
    Task::Ready: Send + 'static,
    Task::Pending: Send + 'static,
    Task::Spawner: ExecutionAction + Send + 'static,
```

Example with Arc<Mutex<T>>:

```rust
use std::sync::{Arc, Mutex};

struct ThreadedCounter {
    limit: usize,
    collected: Arc<Mutex<Vec<usize>>>,
}

impl TaskIterator for ThreadedCounter {
    type Ready = usize;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        let mut items = self.collected.lock().unwrap();
        let current = items.len();

        if current >= self.limit {
            return None;
        }

        items.push(current);
        Some(TaskStatus::Ready(items.len()))
    }
}
```

---

## 5. DrivenRecvIterator and DrivenSendTaskIterator

Valtron provides **auto-driving wrappers** that handle executor calls automatically when iterating.

### DrivenRecvIterator

Wraps `RecvIterator<TaskStatus<...>>` and auto-drives the executor:

```rust
pub struct DrivenRecvIterator<T>(
    Option<RecvIterator<TaskStatus<T::Ready, T::Pending, T::Spawner>>>,
)
where
    T: TaskIterator + Send + 'static;

impl<T> Iterator for DrivenRecvIterator<T> {
    type Item = TaskStatus<T::Ready, T::Pending, T::Spawner>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut iter) = self.0.take() {
            // Auto-drive executor until progress
            iter = run_until_receiver_has_value(iter, checker);

            let next = iter.next();
            if next.is_some() {
                self.0.replace(iter);
            }
            next
        } else {
            None
        }
    }
}
```

### DrivenStreamIterator

Wraps stream-style iteration (simplified `Stream<D, P>` output):

```rust
pub struct DrivenStreamIterator<T>(
    Option<StreamRecvIterator<T::Ready, T::Pending>>,
)
where
    T: TaskIterator + Send + 'static;

impl<T> Iterator for DrivenStreamIterator<T> {
    type Item = Stream<T::Ready, T::Pending>;

    fn next(&mut self) -> Option<Self::Item> {
        // Auto-drive until stream has value
        // Return Stream::Next, Stream::Pending, etc.
    }
}
```

### TaskStatusIterator Trait

For iterator-based operations over TaskStatus:

```rust
pub trait TaskStatusIterator: Iterator<Item = TaskStatus<D, P, S>>
where
    S: ExecutionAction,
{
    /// Convert to Stream-based processing
    fn into_stream(self) -> impl StreamIterator<D, P>;
}
```

### StreamIterator Trait

```rust
pub trait StreamIterator {
    type D;  // Done/Ready type
    type P;  // Pending type

    fn next(&mut self) -> Option<Stream<Self::D, Self::P>>;
}
```

---

## 6. execute() and execute_stream()

Valtron provides a **unified executor API** that auto-selects between single-threaded and multi-threaded execution based on platform and features.

### Platform Selection Logic

```rust
pub fn execute<T>(
    task: T,
    wait_cycle: Option<Duration>,
) -> GenericResult<DrivenStreamIterator<T>>
where
    T: TaskIterator + Send + 'static,
    T::Ready: Send + 'static,
    T::Pending: Send + 'static,
    T::Spawner: ExecutionAction + Send + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        execute_single_stream(task, wait_cycle)  // WASM: single-threaded
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(feature = "multi")]
        {
            execute_multi_stream(task, wait_cycle)  // Native + multi: multi-threaded
        }

        #[cfg(not(feature = "multi"))]
        {
            execute_single_stream(task, wait_cycle)  // Native without multi: single-threaded
        }
    }
}
```

### Unified API Usage

```rust
// Same code works on WASM and native
let task = FetchModels::new(client);

let mut stream = valtron::execute(task, None)?;

for item in stream {
    match item {
        Stream::Next(models) => process(models),
        Stream::Pending(count) => println!("{count} loading..."),
        Stream::Delayed(dur) => thread::sleep(dur),
        _ => {}
    }
}
```

### Collection Patterns

```rust
// Execute multiple tasks, collect all results
let tasks = vec![task1, task2, task3];
let collected = execute_collect_all(tasks, None)?;

for result in collected {
    match result {
        Stream::Next(all_values) => println!("All done: {:?}", all_values),
        Stream::Pending(count) => println!("{count} still pending"),
        _ => {}
    }
}

// Execute with mapping when all complete
let merged = execute_map_all(tasks, |results| {
    results.into_iter().flatten().collect::<Vec<_>>()
}, None)?;
```

---

## 7. TaskIterator Combinators

Valtron provides builder-pattern combinators for transforming TaskStatus values.

### map_ready()

Transform Ready values, pass through other states unchanged:

```rust
let task = fetch_task()
    .map_ready(|models| {
        models.into_iter()
            .filter(|m| m.is_enabled())
            .collect::<Vec<_>>()
    });
```

Implementation:

```rust
pub struct MapReady<I, F> {
    inner: I,
    mapper: F,
}

impl<I, F, O> Iterator for MapReady<I, F>
where
    I: TaskIterator,
    F: Fn(I::Ready) -> O,
{
    type Item = TaskStatus<O, I::Pending, I::Spawner>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_status().map(|status| match status {
            TaskStatus::Ready(value) => TaskStatus::Ready((self.mapper)(value)),
            other => other.map_ready(|_| unreachable!()),
        })
    }
}
```

### map_pending()

Transform Pending values for progress tracking:

```rust
let task = fetch_task()
    .map_pending(|pending| {
        PendingWithTime {
            state: pending.state,
            timestamp: Instant::now(),
        }
    });
```

### stream_collect()

Collect all Ready values into a Vec (non-blocking):

```rust
let task = fetch_task()
    .stream_collect();

// Passes through Pending/Delayed states
// Only yields Vec<Ready> when complete
for item in execute(task)? {
    match item {
        Stream::Pending(_) => println!("Still fetching..."),
        Stream::Next(all_items) => println!("Got {} items", all_items.len()),
        _ => {}
    }
}
```

### Builder Pattern Chaining

```rust
let task = fetch_models_task(client)
    .map_ready(|m| m.with_enhanced_metadata())
    .map_pending(|p| p.with_timestamp(Instant::now()))
    .filter_ready(|models| !models.is_empty())
    .stream_collect();

let stream = execute(task)?;
```

---

## 8. From TypeScript Effect to Rust Valtron

Developers familiar with TypeScript's Effect library will recognize similar patterns in Valtron.

### Effect.gen to TaskIterator

**TypeScript Effect:**
```typescript
const program = Effect.gen(function* () {
    const users = yield* fetchUsers();
    const posts = yield* fetchPosts(users[0].id);
    return { users, posts };
});
```

**Rust Valtron:**
```rust
struct Program {
    step: ProgramStep,
    users: Option<Vec<User>>,
}

enum ProgramStep {
    FetchUsers,
    FetchPosts { users: Vec<User> },
    Done { users: Vec<User>, posts: Vec<Post> },
}

impl TaskIterator for Program {
    type Ready = Result<(Vec<User>, Vec<Post>), Error>;
    type Pending = ProgramStep;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match &self.step {
            ProgramStep::FetchUsers => {
                self.step = ProgramStep::FetchPosts { users: vec![] };
                Some(TaskStatus::Pending(ProgramStep::FetchUsers))
            }
            ProgramStep::FetchPosts { users } => {
                if users.is_empty() {
                    // Simulating async fetch
                    self.step = ProgramStep::Done {
                        users: vec![User::new()],
                        posts: vec![Post::new()],
                    };
                    Some(TaskStatus::Pending(ProgramStep::FetchPosts { users: vec![] }))
                } else {
                    Some(TaskStatus::Ready((users.clone(), vec![Post::new()])))
                }
            }
            ProgramStep::Done { .. } => None,
        }
    }
}
```

### yield* to TaskStatus Returns

**TypeScript:**
```typescript
const result = yield* someEffect;  // Suspends until ready
```

**Rust Valtron:**
```rust
fn next_status(&mut self) -> Option<TaskStatus<Ready, Pending, Spawner>> {
    // Explicit state return instead of yield
    Some(TaskStatus::Pending(CurrentState))  // Suspends
    Some(TaskStatus::Ready(value))           // Returns value
    None                                      // Complete
}
```

### Effect Error Channel to Result Types

**TypeScript Effect:**
```typescript
const result = yield* Effect.try({
    try: () => JSON.parse(maybeJson),
    catch: (error) => new ParseError(error)
});
```

**Rust Valtron:**
```rust
type Ready = Result<ParsedData, ParseError>;

fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
    match std::panic::catch_unwind(|| self.parse()) {
        Ok(Ok(data)) => Some(TaskStatus::Ready(Ok(data))),
        Ok(Err(e)) => Some(TaskStatus::Ready(Err(e))),
        Err(_) => Some(TaskStatus::Ready(Err(ParseError::Panic))),
    }
}
```

### Comparison Table

| TypeScript Effect | Rust Valtron |
|-------------------|--------------|
| `Effect.gen(function*())` | `impl TaskIterator` |
| `yield* effect` | `TaskStatus::Pending` / `TaskStatus::Ready` |
| `Effect.try({ try, catch })` | `Result<T, E>` in `Ready` type |
| `Effect.map()` | `.map_ready()` combinator |
| `Effect.flatMap()` | State machine transitions |
| `Effect.all([a, b, c])` | `execute_collect_all()` |
| `Effect.runPromise()` | `execute()` / `run_until_complete()` |

---

## Execution Trace Example

Full trace of a Valtron task through the executor:

```
Task: FetchUserData { user_id: 42 }

1. schedule() called
   -> Task added to executor queue
   -> Returns SpawnInfo

2. Executor calls next_status()
   -> TaskStatus::Pending(FetchingUser)
   -> Executor marks task pending, schedules retry

3. Executor calls next_status()
   -> TaskStatus::Ready(User { id: 42, name: "Alice" })
   -> Executor invokes resolver callback
   -> Callback prints "Got user: Alice"

4. Executor calls next_status()
   -> None (iterator exhausted)
   -> Executor marks task complete
   -> Resources released
```

**Multi-threaded trace:**

```
Thread 1: FetchUserData { user_id: 42 }
Thread 2: FetchUserData { user_id: 43 }
Thread 3: FetchUserData { user_id: 44 }
Thread 4: FetchUserData { user_id: 45 }

Each thread independently:
1. Pops task from shared queue
2. Calls next_status() until complete
3. Task stays on same thread (no stealing)
4. On completion, fetches next task from queue
```

---

## Summary

Valtron provides a novel approach to async execution:

1. **Iterator-based** - Tasks are explicit state machines implementing `TaskIterator`
2. **No async/await** - No runtime, no hidden awaits, full control
3. **WASM-native** - Single-threaded executor works in browsers
4. **Deterministic** - Step-by-step execution for debugging
5. **Composable** - Rich combinator library for transformations
6. **Unified API** - `execute()` auto-selects appropriate executor

The trade-off is more verbose task definitions, but the payoff is deterministic, portable, and runtime-free async execution.

---

## Further Reading

- Source: `foundation_core/src/valtron/`
- Spec: `specifications/08-valtron-async-iterators/`
- Related: `unified.rs`, `task_iters.rs`, `drivers.rs`
