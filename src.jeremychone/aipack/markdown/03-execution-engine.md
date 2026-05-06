# Aipack -- Execution Engine

The execution engine is the central action dispatcher. It receives `ExecActionEvent` messages on a flume channel, spawns each action as a separate tokio task, and publishes status events to the global Hub.

Source: `aipack/src/exec/executor.rs` — main executor
Source: `aipack/src/exec/cli/event_action.rs` — action event enum
Source: `aipack/src/exec/cli/mod.rs` — executor module

## Executor Structure

```rust
struct Executor {
    action_rx: Receiver<ExecActionEvent>,
    action_sender: ExecutorTx,
    current_redo_ctx: Arc<Mutex<Option<RunRedoCtx>>>,
    active_actions: Arc<AtomicUsize>,
    cancel_trx: Option<CancelTrx>,
    run_queue_tx: RunQueueTx,
    // ... runtime dependencies
}
```

| Field | Purpose |
|-------|---------|
| `action_rx` | Receives actions from the flume channel |
| `action_sender` | Cloned sender for dispatching child actions |
| `current_redo_ctx` | Captured context for the last run (enables redo) |
| `active_actions` | Counts in-flight actions (triggers StartExec/EndExec events) |
| `cancel_trx` | Current run's cancellation token |
| `run_queue_tx` | Sender for the run queue (sub-agent dispatch) |

## Executor Start Loop

```rust
async fn start(mut self) {
    while let Ok(action) = self.action_rx.recv_async().await {
        // Increment active action counter
        let was_zero = self.active_actions.fetch_add(1, Ordering::SeqCst) == 0;
        if was_zero {
            Hub::publish(ExecStatusEvent::StartExec).await;
        }

        // Clone sender for the spawned task
        let sender = self.action_sender.clone();

        // Spawn each action as a separate tokio task
        tokio::spawn(async move {
            let result = self.perform_action(action).await;

            // Store errors in DB and publish to hub
            if let Err(err) = &result {
                ModelManager::store_error(err).await;
                Hub::publish_err(err.clone()).await;
            }

            // Decrement active counter
            let now_zero = self.active_actions.fetch_sub(1, Ordering::SeqCst) == 1;
            if now_zero {
                Hub::publish(ExecStatusEvent::EndExec).await;
            }
        });
    }
}
```

**Key insight:** Each action runs in its own tokio task. This means multiple agents can run concurrently (e.g., a user starts one run, then starts another before the first finishes). The `active_actions` counter tracks how many actions are in-flight, emitting `StartExec` when transitioning from 0→1 and `EndExec` when transitioning from 1→0. The TUI uses these events to show/hide the "running" indicator.

## Action Dispatch

```rust
async fn perform_action(&self, action: ExecActionEvent) -> Result<(), Error> {
    match action {
        ExecActionEvent::CmdInit(args) => {
            init_wks(&args.path).await?;
            init_base().await?;
        }
        ExecActionEvent::Run(args) => {
            // Initialize workspace if needed
            ensure_wks_initialized().await?;

            // Find the agent
            let agent = AgentLocator::find_agent(&args.cmd_agent_name, &runtime).await?;

            // Execute the agent
            let response = exec_run(&runtime, agent, &args).await?;

            // Capture redo context
            if let Some(ctx) = response.redo_ctx {
                *self.current_redo_ctx.lock().await = Some(ctx);
            }
        }
        ExecActionEvent::Redo => {
            // Re-execute from captured RunRedoCtx
            let ctx = self.current_redo_ctx.lock().await.take();
            if let Some(ctx) = ctx {
                exec_redo(&runtime, ctx).await?;
            }
        }
        ExecActionEvent::CancelRun => {
            // Signal cancellation
            if let Some(cancel_trx) = &self.cancel_trx {
                cancel_trx.cancel();
            }
        }
        ExecActionEvent::RunSubAgent(params) => {
            exec_run_sub_agent(&runtime, params).await?;
        }
        ExecActionEvent::CmdList(args) => {
            exec_list(args).await?;
        }
        ExecActionEvent::CmdPack(args) => {
            exec_pack(args).await?;
        }
        ExecActionEvent::CmdInstall(args) => {
            exec_install(args).await?;
        }
        ExecActionEvent::CmdUnpack(args) => {
            exec_unpack(args).await?;
        }
        ExecActionEvent::CmdCheckKeys => {
            exec_check_keys().await?;
        }
        ExecActionEvent::WorkConfirm(work_id) => {
            // Confirm installation work item
            WorkBmc::confirm(work_id).await?;
        }
        ExecActionEvent::WorkCancel(work_id) => {
            // Cancel installation work item
            WorkBmc::cancel(work_id).await?;
        }
        ExecActionEvent::OpenAgent(agent_ref) => {
            // Open agent file in editor (VSCode)
            editor::open_file(&agent_ref.file_path).await?;
        }
    }
    Ok(())
}
```

## ExecActionEvent Enum

```rust
enum ExecActionEvent {
    // CLI commands
    CmdInit(InitArgs),
    Run(RunArgs),
    CmdList(ListArgs),
    CmdPack(PackArgs),
    CmdInstall(InstallArgs),
    CmdUnpack(UnpackArgs),
    CmdCheckKeys,
    CmdCreateGitignore(CreateGitignoreArgs),
    CmdXelfSetup(XelfSetupArgs),
    CmdXelfUpdate(XelfUpdateArgs),

    // Interactive actions
    OpenAgent(AgentRef),
    Redo,
    CancelRun,

    // Agent commands
    RunSubAgent(RunSubAgentParams),

    // Work lifecycle
    WorkConfirm(Id),
    WorkCancel(Id),
}
```

## Run Redo Context

```rust
struct RunRedoCtx {
    runtime: Runtime,
    agent: Agent,
    run_options: RunAgentOptions,
    redo_requested: bool,
    flow_redo_count: usize,
}
```

The redo context captures everything needed to re-execute a run: the runtime (with session, DB connection, etc.), the agent definition, run options, and a counter for how many times the flow has been redone. When the user triggers a redo, the executor re-creates the run with an incremented `flow_redo_count` and re-executes.

See [Run System](04-run-system.md) for the agent execution flow.
See [Event System](08-event-system.md) for Hub and cancellation.
