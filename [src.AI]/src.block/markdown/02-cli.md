# Goose CLI: Commands, Sessions, Recipes, Benchmarks

The `goose-cli` crate provides the command-line interface with 20+ subcommands for chat sessions, configuration, recipes, scheduling, benchmarks, and more.

Source: `crates/goose-cli/src/` — 8 modules, 40 Rust files.

## Module Structure

```
goose-cli/src/
├── lib.rs            # Re-exports: CliSession
├── main.rs           # Entry point
├── cli.rs            # CLI definition (clap)
├── logging.rs        # Logging setup
├── signal.rs         # Signal handling (Ctrl+C)
├── project_tracker.rs # Project tracking
├── commands/         # Subcommand handlers
│   ├── configure/    # Configuration
│   ├── info/         # System info
│   ├── session/      # Session management
│   ├── recipe/       # Recipe management
│   ├── schedule/     # Task scheduling
│   ├── project/      # Project defaults
│   ├── bench/        # Benchmarks
│   └── acp/          # ACP agent
├── session/          # Session building
├── recipes/          # Recipe handling
└── scenario_tests/   # Test scenarios
```

## CLI Structure

Source: `cli.rs`

```rust
#[derive(Parser)]
#[command(author, version, display_name = "", about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Args, Debug, Clone)]
pub struct Identifier {
    #[arg(short = 'n', long, value_name = "NAME",
        help = "Name for the chat session (e.g., 'project-x')")]
    pub name: Option<String>,

    #[arg(long = "session-id", alias = "id", value_name = "SESSION_ID",
        help = "Session ID (e.g., '20250921_143022')")]
    pub session_id: Option<String>,

    #[arg(long, value_name = "PATH",
        help = "Legacy: Path for the chat session")]
    pub path: Option<PathBuf>,
}
```

### Subcommands

| Command | Handler | Purpose |
|---------|---------|---------|
| (none) | `build_session` | Start chat session |
| `configure` | `handle_configure` | Configure providers |
| `info` | `handle_info` | System info |
| `session list` | `handle_session_list` | List sessions |
| `session remove` | `handle_session_remove` | Remove sessions |
| `recipe list` | `handle_list` | List recipes |
| `recipe open` | `handle_open` | Open recipe |
| `recipe validate` | `handle_validate` | Validate recipe |
| `recipe deeplink` | `handle_deeplink` | Recipe deeplink |
| `schedule list` | `handle_schedule_list` | List scheduled tasks |
| `schedule add` | `handle_schedule_add` | Add scheduled task |
| `schedule remove` | `handle_schedule_remove` | Remove scheduled task |
| `schedule run-now` | `handle_schedule_run_now` | Run task now |
| `schedule sessions` | `handle_schedule_sessions` | List scheduled sessions |
| `schedule services status` | `handle_schedule_services_status` | Service status |
| `schedule services stop` | `handle_schedule_services_stop` | Stop services |
| `schedule cron` | `handle_schedule_cron_help` | Cron help |
| `project` | `handle_projects_interactive` | Project management |
| `bench run` | `BenchRunner` | Run benchmarks |
| `bench eval` | `EvalRunner` | Run evaluations |
| `bench model` | `ModelRunner` | Run model tests |
| `bench metrics` | `MetricAggregator` | Aggregate metrics |
| `acp` | `run_acp_agent` | ACP agent mode |

### Session Resolution

```rust
async fn get_or_create_session_id(
    identifier: Option<Identifier>,
    resume: bool,
    no_session: bool,
) -> Result<Option<String>> {
    if no_session { return Ok(None) }
    // Resolves from --name, --session-id, or --path
}
```

Session ID resolution priority: `--session-id` > `--name` > extract from `--path` > create new.

## Commands

### Configure

Source: `commands/configure/`

Interactive provider configuration. Walks through available providers, collects API keys, and writes to config.

### Session

Source: `commands/session/`

```rust
pub fn handle_session_list() -> Result<()>
pub fn handle_session_remove(ids: Vec<String>) -> Result<()>
```

Lists/removes sessions by ID. Sessions stored as JSONL files.

### Recipe

Source: `commands/recipe/`

```rust
pub fn handle_list() -> Result<()>
pub fn handle_open(name: String) -> Result<()>
pub fn handle_validate(path: PathBuf) -> Result<()>
pub fn handle_deeplink(recipe: String) -> Result<()>
```

### Schedule

Source: `commands/schedule/`

```rust
pub fn handle_schedule_list() -> Result<()>
pub fn handle_schedule_add() -> Result<()>
pub fn handle_schedule_remove() -> Result<()>
pub fn handle_schedule_run_now() -> Result<()>
pub fn handle_schedule_sessions() -> Result<()>
pub fn handle_schedule_services_status() -> Result<()>
pub fn handle_schedule_services_stop() -> Result<()>
```

Scheduled tasks run on cron schedules. Services can be started/stopped independently.

### Bench

Source: `commands/bench/`

```rust
use goose_bench::runners::bench_runner::BenchRunner;
use goose_bench::runners::eval_runner::EvalRunner;
use goose_bench::runners::model_runner::ModelRunner;
use goose_bench::runners::metric_aggregator::MetricAggregator;
```

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Core Library](01-core-library.md) — agents, providers, tools
- [MCP Server](03-mcp-server.md) — MCP servers
- [HTTP Server](04-http-server.md) — REST API
- [Benchmarks](05-benchmarks.md) — Benchmark details

[Next → MCP Server](03-mcp-server.md)
