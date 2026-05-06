# Aipack -- Event System

Aipack uses multiple channel-based event systems: a central Hub for broadcasting status events, flume channels for executor action dispatch, and a custom cancellation primitive built on generation counters.

Source: `aipack/src/hub/mod.rs` — Hub module
Source: `aipack/src/hub/hub_impl.rs` — Hub implementation
Source: `aipack/src/hub/hub_event.rs` — Hub event types
Source: `aipack/src/event/cancel.rs` — Cancellation tokens
Source: `aipack/src/event/unbound.rs` — Unbounded channel wrapper
Source: `aipack/src/event/one_shot.rs` — One-shot channel wrapper
Source: `aipack/src/exec/event_action.rs` — Executor action events
Source: `aipack/src/exec/event_status.rs` — Executor status events

## Hub (Global Event Bus)

The Hub is a singleton (`LazyLock<Hub>`) that broadcasts events to all subscribers. It uses an internal flume channel.

```rust
// hub_impl.rs
pub struct Hub {
    tx: Tx<HubEvent>,
    rx_holder: Arc<Mutex<Option<Rx<HubEvent>>>>,
}

static HUB: LazyLock<Hub> = LazyLock::new(Hub::new);

pub fn get_hub() -> &'static Hub {
    &HUB
}
```

### Publishing Events

```rust
impl Hub {
    // Async publish (within tokio tasks)
    pub async fn publish(&self, event: impl Into<HubEvent>) {
        match self.tx.send(event).await {
            Ok(_) => (),
            Err(err) => tracing::warn!("AIPACK INTERNAL WARNING - failed to send event to hub - {err}"),
        }
    }

    // Sync publish (from Lua callbacks, sync code)
    pub fn publish_sync(&self, event: impl Into<HubEvent>) {
        match self.tx.send_sync(event) {
            Ok(_) => (),
            Err(err) => tracing::warn!("AIPACK INTERNAL WARNING - failed to send event to hub - {err}"),
        }
    }

    // Convenience: publish error with cause
    pub async fn publish_err(&self, msg: impl Into<String>, cause: Option<impl Display>) {
        match cause {
            Some(cause) => self.publish(Error::cc(msg, cause)).await,
            None => self.publish(Error::Custom(msg.into())).await,
        }
    }
}
```

The Hub silently logs warnings if send fails (e.g., no receiver). This prevents cascading failures if the TUI crashes while events are still being published.

### HubEvent Types

```rust
// hub_event.rs
enum HubEvent {
    Data(HubDataEvent),         // Log messages, text output
    Error { error: Box<Error> }, // Errors
    Stage(StageEvent),          // Progress stages (run_start, task_end, etc.)
    Model(ModelEvent),          // Database entity changes
    RtModelChange,              // Runtime model state changed
    Quit,                       // Terminate event loop
}

enum HubDataEvent {
    Text(String),               // Plain text (status messages)
    LuaPrint(Box<LuaPrint>),    // print() output from Lua scripts
    RunQueue(RunQueueEvent),    // Run queue events
}
```

### Taking the Receiver

```rust
pub fn take_rx(&self) -> Result<Rx<HubEvent>> {
    let mut rx_holder = self.rx_holder.lock().map_err(|err| ...)?;
    let rx = rx_holder.take().ok_or("Hub Rx already taken, cannot take twice")?;
    Ok(rx)
}
```

The receiver can only be taken once. This is intentional — the TUI takes the receiver at startup and owns the event loop. No other consumer can intercept Hub events.

## Cancellation System

Aipack uses a custom cancellation primitive instead of `tokio::CancellationToken`. The key reason: `tokio::CancellationToken` keeps the cancelled state forever, so reusing the same token across runs would surface stale cancellations.

```rust
// cancel.rs
struct CancelInner {
    name: &'static str,        // Diagnostic identifier
    notify: Notify,            // Tokio Notify for async waiting
    generation: AtomicU64,     // Monotonic counter
}

pub struct CancelTx(Arc<CancelInner>);
pub struct CancelRx {
    inner: Arc<CancelInner>,
    last_seen: AtomicU64,      // Tracks last observed generation
}

pub struct CancelTrx(CancelTx, CancelRx);  // Paired transmitter + receiver
```

### How Generation Counters Work

```
Initial state:  generation = 0, last_seen = 0

Thread A calls cancel():
  → generation.fetch_add(1)  → generation = 1
  → notify.notify_waiters()

Thread B checks is_cancelled():
  → generation(1) > last_seen(0) → true, cancelled

Thread B awaits cancellation:
  → loop:
      current = inner.generation()  // 1
      last_seen = self.last_seen.load()  // 0
      if current > last_seen:  // true
          self.last_seen.store(current)  // last_seen = 1
          return  // resolved
```

### Why Not tokio::CancellationToken?

```
tokio::CancellationToken problem:
  cancel() → sets internal bool = true
  subsequent runs inheriting the token see cancelled=true immediately
  no way to "reset" without creating a new token

aipack generation counter solution:
  cancel() → increments generation counter
  new CancelRx starts with last_seen = current_generation
  so new runs only see fresh cancellations, not past ones
```

### Usage in Run System

```rust
// From 04-run-system.md — run_agent()
let result = tokio::select! {
    result = run_agent_inner(runtime.clone(), agent.clone(), options.clone()) => result,
    _ = runtime.cancel_rx().cancelled() => {
        RtStep::run_end_canceled(&runtime).await;
        return Ok(RunAgentResponse::canceled());
    }
};
```

The `select!` macro races the run against the cancellation receiver. When `cancel()` is called on the transmitter, the generation counter increments, `Notify` wakes all waiters, and the `cancelled()` future resolves.

## Channel Abstractions

### Unbounded Channel

```rust
// event/unbound.rs
pub struct Tx<T>(flume::Sender<T>);
pub struct Rx<T>(flume::Receiver<T>);

pub fn new_channel<T>(name: &'static str) -> (Tx<T>, Rx<T>) {
    let (tx, rx) = flume::unbounded();
    (Tx(tx), Rx(rx))
}
```

Wraps flume with error types that include the channel name for diagnostics.

### One-Shot Channel

```rust
// event/one_shot.rs
pub fn new_oneshot<T>() -> (OneShotTx<T>, OneShotRx<T>) {
    let (tx, rx) = flume::bounded(1);
    (OneShotTx(tx), OneShotRx(rx))
}
```

Used for single-result communication (e.g., run result back to caller).

## Executor Action Events

See [Execution Engine](03-execution-engine.md) for the `ExecActionEvent` enum and dispatch logic.

## Run Queue Events

```rust
// runtime/queue/run_event.rs
enum RunEvent {
    Enqueue(RunEnqueue),
    Start { run_uid: Uuid },
    // ... more
}
```

Run events flow through a dedicated channel within the Runtime, separate from Hub events. The run queue manages scheduling of agent runs.

See [Runtime System](11-runtime-system.md) for the run queue and Runtime event dispatch.
See [Execution Engine](03-execution-engine.md) for executor action dispatch.
