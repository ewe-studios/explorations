# Aipack -- TUI Architecture

Aipack has two TUI implementations: a new `ratatui`-based TUI (`src/tui/`) and a legacy terminal UI (`src/tui_v1/`). The new TUI is the default interactive experience.

Source: `aipack/src/tui/mod.rs` — new TUI module
Source: `aipack/src/tui/core/mod.rs` — core TUI loop
Source: `aipack/src/tui/core/tui_impl.rs` — TUI implementation
Source: `aipack/src/tui/core/app_state.rs` — application state
Source: `aipack/src/tui/core/event/app_event.rs` — TUI event types
Source: `aipack/src/tui/view/main_view.rs` — main view rendering
Source: `aipack/src/tui/view/run_main_view.rs` — run detail view
Source: `aipack/src/tui/view/run_overview.rs` — run overview
Source: `aipack/src/tui/view/run_tasks_view.rs` — task list view
Source: `aipack/src/tui/view/runs_view.rs` — runs list
Source: `aipack/src/tui/view/runs_nav_view.rs` — run navigation
Source: `aipack/src/tui/view/task_view.rs` — individual task view
Source: `aipack/src/tui/view/action_view.rs` — action panel
Source: `aipack/src/tui/view/config_view.rs` — configuration view
Source: `aipack/src/tui/view/popup_view.rs` — popup dialogs
Source: `aipack/src/tui/view/install_view.rs` — installation view

## Entry Point

```rust
// main.rs
if args.cmd.is_interactive() && args.cmd.is_tui() {
    tui::start_tui(args, executor_tx, mm).unwrap();
} else {
    let tui = TuiAppV1::new(mm);
    tui.run(args, executor_tx)?;
}
```

The new ratatui TUI is triggered when the command is both interactive and TUI-enabled (the default for `run`). Otherwise, the legacy `TuiAppV1` is used.

## Core TUI Loop

```rust
// tui_impl.rs
pub struct AppState {
    stage: AppStage,       // Current view stage
    runs: Vec<Run>,        // Cached run list
    selected_run: Option<Id>,
    selected_task: Option<Id>,
    // ... more state
}

enum AppStage {
    RunList,        // Show list of runs
    RunDetail,      // Show selected run details
    TaskDetail,     // Show selected task details
    Config,         // Configuration panel
    Install,        // Pack installation view
}

async fn start_tui(args: CliArgs, executor_tx: ExecutorTx, mm: OnceModelManager) -> Result<()> {
    // 1. Initialize terminal with raw mode
    let mut terminal = init_terminal()?;

    // 2. Create app state
    let mut app_state = AppState::new(mm);

    // 3. Start ping timer (for periodic refresh)
    let ping_tx = start_ping_timer(Duration::from_millis(500));

    // 4. Main event loop
    loop {
        // Read terminal events (key presses, resize)
        let event = read_term_event()?;

        // Handle event
        handle_app_event(&mut app_state, &mut terminal, event)?;

        // Check for quit
        if app_state.should_exit() {
            break;
        }
    }

    // 5. Restore terminal
    restore_terminal()?;
    Ok(())
}
```

## View Architecture

```
┌─────────────────────────────────────────────────┐
│ MainView                                         │
│ ┌─────────────┬───────────────────────────────┐  │
│ │ RunsNavView │ RunMainView                   │  │
│ │             │ ┌───────────────────────────┐ │  │
│ │ [Run #42]   │ │ RunOverview               │ │  │
│ │ [Run #41]   │ │ status: running           │ │  │
│ │ [Run #40]   │ │ model: sonnet             │ │  │
│ │ [Run #39]   │ │ cost: $0.0042             │ │  │
│ │             │ └───────────────────────────┘ │  │
│ │             │ ┌───────────────────────────┐ │  │
│ │             │ │ RunTasksView              │ │  │
│ │             │ │ ┌──────┬──────┬────────┐  │ │  │
│ │             │ │ │ #1   │ data │ done   │  │ │  │
│ │             │ │ │ #2   │ ai   │ done   │  │ │  │
│ │             │ │ │ #3   │ out  │ done   │  │ │  │
│ │             │ │ └──────┴──────┴────────┘  │ │  │
│ │             │ └───────────────────────────┘ │  │
│ │             │ ┌───────────────────────────┐ │  │
│ │             │ │ ActionView                │ │  │
│ │             │ │ [Redo] [Cancel] [Quit]    │ │  │
│ │             │ └───────────────────────────┘ │  │
│ └─────────────┴───────────────────────────────┘  │
│ PopupView (overlay when active)                  │
└─────────────────────────────────────────────────┘
```

### View Hierarchy

```
MainView
  ├── RunsNavView (left sidebar — run list navigation)
  └── RunMainView (right panel — active run display)
      ├── RunOverview (run metadata: status, model, cost, timing)
      ├── RunTasksView (task list with stage indicators)
      └── ActionView (action buttons: redo, cancel, quit)

TaskView (when task selected)
  ├── TaskOverview (task metadata, input/output summary)
  └── TaskContent (full task content: data, AI response, output)

PopupView (overlay)
  ├── Confirmation dialogs
  ├── Error display
  └── Help text
```

## Event Handling

```rust
// app_event_handlers.rs
fn handle_app_event(app_state: &mut AppState, terminal: &mut Terminal, event: TerminalEvent) {
    match event {
        // Keyboard input
        TerminalEvent::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
            // Go back or quit
        }
        TerminalEvent::Key(KeyEvent { code: KeyCode::Down, .. }) => {
            // Navigate down in current view
        }
        TerminalEvent::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
            // Select current item
        }

        // Ping timer (periodic refresh)
        TerminalEvent::Ping => {
            // Refresh run/task data from DB
            refresh_from_db(app_state)?;
        }

        // Hub events (via TUI's Hub event handler)
        TerminalEvent::HubEvent(HubEvent::Stage(stage_event)) => {
            // Update UI state for stage changes
            on_stage_change(app_state, &stage_event)?;
        }
    }
}
```

## Ping Timer

```rust
// ping_timer.rs
pub struct PingTimerTx {
    tx: flume::Sender<()>,
}

pub fn start_ping_timer(interval: Duration) -> PingTimerTx {
    let (tx, rx) = flume::unbounded();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            if tx.send(()).is_err() {
                break;  // receiver dropped
            }
        }
    });

    PingTimerTx { tx }
}
```

The ping timer fires every 500ms, triggering a data refresh from the SQLite database. This gives the TUI a live view of run progress without needing to poll on every keypress.

## UI Extensions

```rust
// ui_ext.rs
trait UiExt {
    fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect;
    fn bordered_block(&mut self, title: &str, area: Rect) -> Result<()>;
    fn styled_line(&mut self, text: &str, style: Style, x: u16, y: u16) -> Result<()>;
    // ... more
}
```

Helper methods for common TUI rendering patterns: centered rectangles, bordered blocks, styled text.

## Support Utilities

```rust
// formatters.rs
fn format_duration(micros: EpochUs) -> String;  // "1.23s", "456ms"
fn format_cost(cost: f64) -> String;            // "$0.0042"
fn format_tokens(tokens: i64) -> String;        // "1.2K"
fn truncate(text: &str, max: usize) -> &str;    // "This is a long..."
```

```rust
// number_utils.rs
fn format_number(n: i64) -> String;  // 1234 → "1,234"
fn to_percentage(part: f64, total: f64) -> String;  // "67%"
```

## Legacy TUI (TuiAppV1)

The legacy TUI (`src/tui_v1/`) is a simpler, line-based terminal UI:

```rust
// tui_v1/tui_app.rs
struct TuiAppV1 {
    mm: ModelManager,
    // Simple state
}

impl TuiAppV1 {
    fn run(&self, args: CliArgs, executor_tx: ExecutorTx) -> Result<()> {
        // Print status lines directly to terminal
        // No cursor management, no panels
        // Used for --old-term flag or non-TUI commands
    }
}
```

The legacy UI prints status messages sequentially without the panel-based layout of the ratatui TUI. It remains available for debugging and environments where ratatui doesn't render correctly.

See [Execution Engine](03-execution-engine.md) for how the executor dispatches to TUI.
See [Event System](08-event-system.md) for Hub event handling.
