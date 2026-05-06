# Aipack -- CLI Structure

Aipack's CLI is built with `clap` (derive API). The entry point parses arguments into a `CliCommand` enum, which is then converted to an `ExecActionEvent` and dispatched through the executor's flume channel.

Source: `aipack/src/main.rs` — entry point, TUI dispatch
Source: `aipack/src/exec/cli/args.rs` — CLI argument definitions

## Entry Point

```rust
// main.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    // Lazy singleton for SQLite model manager
    let mm = OnceModelManager::new();

    // Create flume channel for executor actions
    let (action_tx, action_rx) = flume::unbounded();
    let executor_tx = ExecutorTx::new(action_tx);

    // Spawn executor task
    let executor = Executor::new(action_rx, /* ... */);
    tokio::spawn(executor.start());

    // Route to TUI or legacy terminal UI
    if args.cmd.is_interactive() && args.cmd.is_tui() {
        tui::start_tui(args, executor_tx, mm).unwrap();
    } else {
        let tui = TuiAppV1::new(mm);
        tui.run(args, executor_tx)?;
    }

    Ok(())
}
```

The entry point creates three key components:
1. **`OnceModelManager`** — lazy-initialized SQLite singleton via `tokio::sync::OnceCell`
2. **Executor** — spawned on a tokio task, listens on flume channel
3. **TUI** — either the new ratatui TUI or the legacy `TuiAppV1`

## CLI Commands

```rust
#[derive(Parser)]
struct CliArgs {
    #[command(subcommand)]
    cmd: CliCommand,
}

enum CliCommand {
    /// Initialize .aipack/ workspace + ~/.aipack-base/
    Init(InitArgs),
    /// Initialize only ~/.aipack-base/
    InitBase,
    /// Execute an agent (core command)
    Run(RunArgs),
    /// List installed packs
    List(ListArgs),
    /// Pack a directory into .aipack file
    Pack(PackArgs),
    /// Install a .aipack file
    Install(InstallArgs),
    /// Unpack a pack into workspace
    Unpack(UnpackArgs),
    /// Check available API keys in environment
    CheckKeys,
    /// Create .gitignore from template
    CreateGitignore(CreateGitignoreArgs),
    /// Self-management (setup, update)
    Xelf(XelfSetupArgs),
    XelfUpdate(XelfUpdateArgs),
}
```

### Run Command (Most Complex)

```rust
struct RunArgs {
    /// Agent name (e.g., "fix-bug", "ns@pack/agent")
    cmd_agent_name: Option<String>,
    /// Input data (key=value pairs)
    on_inputs: Option<String>,
    /// Input files
    on_files: Vec<String>,
    /// Watch mode (re-run on file changes)
    watch: bool,
    /// Verbose output
    verbose: bool,
    /// Open agent file in editor
    open: bool,
    /// Dry mode: "req" (show request) or "res" (show response)
    dry_mode: Option<String>,
    /// Single shot (no redo loop)
    single_shot: bool,
    /// TUI experience mode
    xp_tui: bool,
    /// Use legacy terminal UI
    old_term: bool,
}
```

**Dry mode** is a debugging feature: `--dry-mode req` shows the prompt that would be sent to the LLM without actually sending it; `--dry-mode res` sends the prompt but shows the raw response without processing it through output scripts.

**Watch mode** re-runs the agent whenever relevant files change, using a file watcher to detect modifications.

### Action Dispatch

```rust
impl From<CliCommand> for ExecActionEvent {
    fn from(cmd: CliCommand) -> Self {
        match cmd {
            CliCommand::Init(args) => ExecActionEvent::CmdInit(args),
            CliCommand::Run(args) => ExecActionEvent::Run(args),
            CliCommand::List(args) => ExecActionEvent::CmdList(args),
            CliCommand::Pack(args) => ExecActionEvent::CmdPack(args),
            CliCommand::Install(args) => ExecActionEvent::CmdInstall(args),
            CliCommand::Unpack(args) => ExecActionEvent::CmdUnpack(args),
            CliCommand::CheckKeys => ExecActionEvent::CmdCheckKeys,
            // ... more
        }
    }
}
```

The `From` impl converts CLI commands into executor action events. The executor's `perform_action` match-dispatches each event to its handler.

### ExecutorTx

```rust
struct ExecutorTx {
    action_tx: Sender<ExecActionEvent>,
}

impl ExecutorTx {
    // Async send (preferred in async context)
    async fn send(&self, event: ExecActionEvent) -> Result<(), Error>;

    // Sync flume send (for sync contexts)
    fn send_sync(&self, event: ExecActionEvent) -> Result<(), Error>;

    // Sync call that blocks on tokio handle to spawn the send async
    fn send_sync_spawn_and_block(&self, event: ExecActionEvent) -> Result<(), Error>;
}
```

The three send methods handle the different calling contexts: async (within tokio tasks), sync (from Lua callbacks), and sync-that-needs-async (from sync code that must dispatch through the async executor).

See [Execution Engine](03-execution-engine.md) for the executor loop.
See [TUI Architecture](10-tui-architecture.md) for TUI dispatch.
