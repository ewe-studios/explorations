# Aipack -- Production Patterns

This document covers the production-oriented design decisions and patterns in aipack that are relevant for real-world deployment and scaling.

## Concurrency Control

### Task-Level Concurrency

```rust
// From 04-run-system.md — JoinSet pattern
let concurrency = agent.options.input_concurrency.unwrap_or(1);
let mut join_set = JoinSet::new();

for (idx, input) in inputs.into_iter().enumerate() {
    // Wait if at capacity
    while join_set.len() >= concurrency {
        if let Some(result) = join_set.join_next().await {
            outputs.push(result??);
        }
    }

    join_set.spawn(async move {
        run_agent_task_outer(task_runtime, task_agent, input, task_uids[idx]).await
    });
}
```

**Key insight:** The `JoinSet` pattern provides bounded concurrency. When `input_concurrency = 4`, at most 4 LLM API requests are in-flight simultaneously. This prevents:
- Rate limit exhaustion on LLM providers
- Memory pressure from too many concurrent task contexts
- Connection pool exhaustion

**Trade-off:** Tasks are processed in spawn order, not completion order. The output array preserves input ordering, which is important for deterministic results.

### Run-Level Concurrency

The run queue manages multiple concurrent runs:

```rust
// From runtime/queue/run_queue.rs
struct RunQueue {
    tx: flume::Sender<RunEvent>,
    rx: flume::Receiver<RunEvent>,
}
```

When a user starts a run while another is active, the second run is enqueued. The run queue processes events sequentially, starting runs when capacity becomes available.

## Cancellation Safety

### Generation Counter Design

The cancellation system uses atomic generation counters instead of a simple boolean flag:

```rust
struct CancelInner {
    generation: AtomicU64,  // Incremented on each cancel()
    notify: Notify,          // Wakes async waiters
}

struct CancelRx {
    last_seen: AtomicU64,   // Tracks last observed generation
}
```

**Why not boolean?** A boolean flag stays `true` after cancellation. If the same `Runtime` is reused for multiple runs (which it is, via `clone()`), the second run would immediately see `is_cancelled = true` from the first run's cancellation.

**Why not tokio::CancellationToken?** Same issue — once cancelled, it stays cancelled. Creating a new token per run would work, but requires plumbing a new token through every code path. The generation counter approach embeds reusability into the primitive itself.

**Cloning safety:** Each `CancelRx::clone()` creates a new receiver with `last_seen = current_generation`. This means cloned receivers only respond to fresh cancellations, not ones that occurred before the clone.

## SQLite WAL Mode

```rust
// From rt_db_setup.rs
db.execute("PRAGMA journal_mode=WAL", [])?;
```

Write-Ahead Logging (WAL) mode provides:
- **Concurrent readers:** Multiple reads can proceed in parallel without blocking each other
- **Non-blocking reads during writes:** Readers see the last committed state while a write is in progress
- **Crash recovery:** WAL file can be replayed after crashes

For aipack's workload (many reads for TUI display, occasional writes for run/task updates), WAL mode is the optimal choice.

## Prompt Caching Economics

The pricing calculator tracks three prompt token categories:

```
total_prompt_tokens = normal_tokens + cached_tokens + cache_creation_tokens
```

- **Normal tokens:** Charged at full input rate
- **Cached tokens:** Charged at reduced rate (typically 10% of full rate for Anthropic)
- **Cache creation tokens:** Charged at 1.25x full rate (Anthropic's cache write cost)

**Optimization:** Agents with large system prompts benefit from caching across runs. The cache persists across runs with the same prompt content, so repeated runs of the same agent pay the 1.25x cache creation cost once, then the reduced cached rate for subsequent runs.

**Cost tracking:** Each `AiPrice` records:
- `cost`: total cost in USD
- `cost_cache_write`: cost of creating cache entries (first run)
- `cost_cache_saving`: difference between what cached tokens would have cost at normal rate vs cached rate

## File Write Safety

### FileWriteManager

```rust
pub struct FileWriteManager {
    // Coordinates concurrent file writes
}
```

When multiple tasks write to the same output file, the `FileWriteManager` serializes writes to prevent interleaving. Without this, two tasks writing to `output.txt` simultaneously could produce corrupted output.

### Trash vs Delete

All destructive operations use `safer_trash_dir` / `safer_trash_file` instead of direct deletion:

```rust
safer_trash_dir(&pack_target_dir, Some(DeleteCheck::CONTAINS_AIPACK_BASE))?;
```

This moves files to the OS trash rather than permanently deleting them. The `DeleteCheck` verifies the path is within the expected aipack directory before trashing, preventing catastrophic bugs (e.g., trashing the wrong directory due to a path resolution error).

## Hub Event Reliability

```rust
pub async fn publish(&self, event: impl Into<HubEvent>) {
    match self.tx.send(event).await {
        Ok(_) => (),
        Err(err) => tracing::warn!("AIPACK INTERNAL WARNING - failed to send event to hub - {err}"),
    }
}
```

The Hub silently logs warnings if the send fails (e.g., TUI has crashed and the receiver is dropped). This is intentional: a failing TUI should not cascade into a failing agent run. The run continues even if the UI can't display its progress.

## Error Recovery

### Run Redo

```rust
struct RunRedoCtx {
    runtime: Runtime,
    agent: Agent,
    run_options: RunAgentOptions,
    redo_requested: bool,
    flow_redo_count: usize,
}
```

The executor captures the redo context after each run. When the user triggers a redo:
1. The context is taken from `current_redo_ctx`
2. `flow_redo_count` is incremented
3. A new run is created with the same agent and options

This allows users to re-run agents without re-specifying inputs or options. The redo context captures everything needed for reproduction.

### Graceful Degradation

When Lua scripts fail:
- The error is stored in the `err` table with the stage context
- The run is marked with `end_state = Err`
- The TUI displays the error with context

When pack installation fails:
- The temporary directory is trashed
- The error is propagated with context about what failed
- The existing installation (if any) is left intact

## Build and Deployment

### Vendored Lua

aipack vendors Lua 5.4 via `mlua` with the `vendored` feature. This means:
- No system Lua dependency
- Reproducible builds across platforms
- Larger binary size (~5MB for Lua VM)

### Edition 2024, MSRV 1.95

The project uses Rust edition 2024 with MSRV 1.95. This means:
- Access to the latest Rust features (async closures, etc.)
- Users need a relatively recent Rust toolchain
- `rustup run 1.95 cargo build` is the minimum build command

### Include-Dir Assets

```rust
use include_dir::{include_dir, Dir};
static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");
```

Bundled assets (packs, configs, templates) are compiled into the binary. This means:
- Single binary deployment (no separate asset directory)
- Works offline after installation
- Assets are versioned with the binary

## TUI Performance

### Ping Timer Refresh

The TUI refreshes every 500ms via a ping timer:

```rust
let ping_tx = start_ping_timer(Duration::from_millis(500));
```

This is a balance between responsiveness and resource usage. A faster refresh (100ms) would cause unnecessary database queries. A slower refresh (2s) would make the TUI feel sluggish during active runs.

### Database Read Patterns

The TUI reads from SQLite on each ping cycle. Since WAL mode allows concurrent readers, this doesn't block the executor's writes. The trade-off is that the TUI may display slightly stale data (up to 500ms old), but this is imperceptible for a tool that tracks operations lasting seconds to minutes.

See [Event System](08-event-system.md) for cancellation details.
See [Runtime System](11-runtime-system.md) for FileWriteManager.
