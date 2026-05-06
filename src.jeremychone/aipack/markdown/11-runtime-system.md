# Aipack -- Runtime System

The Runtime is the central clonable context object passed throughout aipack. It holds the database connection, genai client, directory context, executor sender, and cancellation tokens — all wrapped in `Arc<RuntimeInner>` for cheap cloning.

Source: `aipack/src/runtime/runtime_impl.rs` — Runtime definition
Source: `aipack/src/runtime/runtime_inner.rs` — RuntimeInner (the Arc-wrapped data)
Source: `aipack/src/runtime/rt_model.rs` — Runtime model operations
Source: `aipack/src/runtime/rt_log.rs` — Runtime logging facade
Source: `aipack/src/runtime/rt_step.rs` — Step timing tracking
Source: `aipack/src/runtime/runtime_path_resolver.rs` — Path resolution in runtime context
Source: `aipack/src/runtime/runtime_rec_lua.rs` — Recursive Lua execution
Source: `aipack/src/runtime/queue/run_queue.rs` — Run queue
Source: `aipack/src/runtime/support/file_write_manager.rs` — File write coordination

## Runtime Structure

```rust
// runtime_impl.rs
#[derive(Debug, Clone)]
pub struct Runtime {
    inner: Arc<RuntimeInner>,
}

pub struct RuntimeInner {
    dir_context: DirContext,       // Directory paths and resolution
    genai_client: Client,          // genai multi-provider LLM client
    executor_tx: ExecutorTx,       // Sender for dispatching actions
    run_tx: RunQueueTx,            // Sender for run queue events
    session: Session,              // Session UUID (v7, time-ordered)
    mm: ModelManager,              // SQLite connection
    file_write_manager: FileWriteManager,  // Coordinated file writes
    cancel_trx: Option<CancelTrx>, // Cancellation token pair
}
```

The `Runtime` is designed to be cloned, not constructed anew. Every component that needs access to the database, directory context, or LLM client receives a `Runtime` clone.

## Session

```rust
pub struct Session {
    uuid: Uuid,
    cached_str: Arc<str>,  // Pre-computed string representation
}

impl Session {
    pub fn new() -> Self {
        let uuid = Uuid::now_v7();  // Time-ordered UUID
        let cached_str = Arc::from(uuid.to_string().as_str());
        Self { uuid, cached_str }
    }
}
```

Sessions use UUIDv7 for time-ordering, which is useful for sorting runs chronologically. The string representation is cached in `Arc<str>` to avoid repeated formatting.

## Runtime Facades

```rust
impl Runtime {
    pub fn rt_log(&self) -> RtLog<'_> {
        RtLog::new(self)
    }
    pub fn rt_model(&self) -> RtModel<'_> {
        RtModel::new(self)
    }
    pub fn rt_step(&self) -> RtStep<'_> {
        RtStep::new(self)
    }
}
```

Each facade provides a borrowed reference to `Runtime` with specialized methods. This avoids bloating `Runtime` with dozens of methods while keeping the facade objects lightweight.

## RtModel — Runtime Database Operations

```rust
// rt_model.rs
pub struct RtModel<'a> {
    runtime: &'a Runtime,
}

impl RtModel<'_> {
    pub fn create_run(runtime: &Runtime, agent: &Agent, options: &RunAgentOptions) -> Result<Uuid> {
        let run_c = RunForCreate {
            agent_name: Some(agent.name().to_string()),
            agent_path: Some(agent.file_path().to_string()),
            has_prompt_parts: Some(agent.has_prompt_parts()),
            has_task_stages: Some(agent.has_task_stages()),
            ..Default::default()
        };
        let id = RunBmc::create(runtime.mm(), run_c)?;
        let RunForUids { uid, .. } = RunBmc::get_uids(runtime.mm(), id)?;
        Ok(uid)
    }

    pub fn create_tasks_batch(runtime: &Runtime, inputs: &[TaskInput]) -> Result<Vec<Uuid>> {
        // Batch create task records, return UIDs
    }

    pub fn update_task_usage(runtime: &Runtime, usage: &Usage) -> Result<()> {
        // Update token counts and cost on task record
    }

    pub fn update_task_cost(runtime: &Runtime, pricing: &AiPrice) -> Result<()> {
        // Update cost on task record
    }

    pub fn set_run_end_error(runtime: &Runtime, error: &Error) -> Result<()> {
        RunBmc::set_end_error(runtime.mm(), run_id, stage, error)?;
    }
}
```

## RtLog — Runtime Logging Facade

```rust
// rt_log.rs
pub struct RtLog<'a> {
    runtime: &'a Runtime,
}

impl RtLog<'_> {
    pub fn log(&self, content: impl Into<String>) {
        // Create log record in DB
        // Also publish to Hub for TUI display
    }

    pub fn log_task(&self, task_id: Id, content: impl Into<String>) {
        // Task-scoped log
    }
}
```

Logs are dual-written: stored in SQLite for persistence and published to the Hub for real-time TUI display.

## RtStep — Step Timing

```rust
// rt_step.rs
pub struct RtStep<'a> {
    runtime: &'a Runtime,
}

impl RtStep<'_> {
    pub async fn run_start(runtime: &Runtime) {
        // Update run.start = now()
        RtModel::update_run(runtime, RunForUpdate {
            start: Some(now_micro()),
            ..Default::default()
        }).await;
    }

    pub async fn run_end_ok(runtime: &Runtime) {
        // Update run.end = now(), end_state = Ok
    }

    pub async fn run_end_canceled(runtime: &Runtime) {
        // Update run.end = now(), end_state = Skip
    }

    pub async fn run_end_err(runtime: &Runtime) {
        // end_state = Err (error ID set separately)
    }

    // Task steps
    pub async fn task_data_start(runtime: &Runtime) { ... }
    pub async fn task_data_end(runtime: &Runtime) { ... }
    pub async fn task_ai_start(runtime: &Runtime) { ... }
    pub async fn task_ai_gen_start(runtime: &Runtime) { ... }
    pub async fn task_ai_gen_end(runtime: &Runtime) { ... }
    pub async fn task_ai_end(runtime: &Runtime) { ... }
    pub async fn task_out_start(runtime: &Runtime) { ... }
    pub async fn task_out_end(runtime: &Runtime) { ... }
}
```

Every step updates the corresponding timestamp field in the database. The timing hierarchy:

```
run
  ├── ba_start / ba_end         (before_all)
  ├── tasks_start / tasks_end   (all tasks)
  │   ├── task #1
  │   │   ├── data_start / data_end
  │   │   ├── ai_start / ai_gen_start / ai_gen_end / ai_end
  │   │   └── out_start / out_end
  │   └── task #2
  │       └── ...
  └── aa_start / aa_end         (after_all)
```

## Run Queue

```rust
// runtime/queue/run_queue.rs
pub struct RunQueue {
    tx: flume::Sender<RunEvent>,
    rx: flume::Receiver<RunEvent>,
}

impl RunQueue {
    pub fn new() -> Self { ... }
    pub fn start(&mut self) -> Result<RunQueueTx> {
        // Spawn the queue processing loop
        tokio::spawn(self.process_loop());
        Ok(self.tx.clone())
    }

    async fn process_loop(&mut self) {
        while let Ok(event) = self.rx.recv_async().await {
            match event {
                RunEvent::Enqueue(run_enqueue) => {
                    // Add to queue, start if capacity available
                }
                RunEvent::Start { run_uid } => {
                    // Start the run via executor
                }
            }
        }
    }
}
```

The run queue manages concurrency for multiple runs. When a user starts a run while another is active, the second run is enqueued and started when the first completes.

## File Write Manager

```rust
// support/file_write_manager.rs
pub struct FileWriteManager {
    // Coordinates file writes to prevent races
    // e.g., multiple tasks writing to the same output file
}
```

The FileWriteManager ensures that concurrent task outputs don't interleave or corrupt each other.

See [Event System](08-event-system.md) for cancellation tokens.
See [Database Schema](09-database-schema.md) for entity definitions.
