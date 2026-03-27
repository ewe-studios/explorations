---
title: "Valtron Integration: TUI Backend Patterns for Lambda Deployment"
subtitle: "Building TUI backends with valtron TaskIterator - No async/await, no tokio"
based_on: "Bubble Tea + Valtron executors"
target: /home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron/
---

# Valtron Integration: TUI Backend

## 1. Overview

### 1.1 Purpose and Scope

This document describes how to build TUI backends using the **valtron executor** pattern from ewe_platform. The key constraint is **no async/await, no tokio** - instead we use **TaskIterator** for non-blocking operations.

**Why This Approach:**

| Traditional Async | Valtron TaskIterator |
|------------------|---------------------|
| `async/await` syntax | Iterator-based |
| Tokio runtime | Single-threaded or custom executor |
| Hidden state machines | Explicit state management |
| Heap allocations | Stack-friendly |
| Complex debugging | Simple step-through |

**Use Cases:**

1. **Lambda backends**: TUI-like interfaces over HTTP
2. **Embedded systems**: No runtime overhead
3. **Deterministic execution**: Predictable timing
4. **Educational**: Clear state machine visualization

### 1.2 Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                   TUI Frontend                           │
│  (Bubble Tea / ratatui / custom)                        │
│  - Handles rendering                                    │
│  - Captures input                                       │
│  - Maintains UI state                                   │
└─────────────────────────┬───────────────────────────────┘
                          │
                          │ JSON over HTTP/WebSocket
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   TUI Backend                            │
│  (Valtron Executor with TaskIterator)                   │
│  - Processes input events                               │
│  - Updates application state                            │
│  - Returns render instructions                          │
└─────────────────────────┬───────────────────────────────┘
                          │
                          │ Data fetching, persistence
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  External Services                       │
│  - Databases                                            │
│  - APIs                                                 │
│  - File storage                                         │
└─────────────────────────────────────────────────────────┘
```

---

## 2. TaskIterator Fundamentals

### 2.1 Core Concepts

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, ExecutionAction};

/// TaskIterator is the core abstraction
///
/// Instead of:
/// ```ignore
/// async fn fetch_data() -> Result<Data, Error> { ... }
/// ```
///
/// We write:
/// ```ignore
/// struct FetchTask { state: FetchState }
///
/// impl TaskIterator for FetchTask {
///     type Ready = Result<Data, Error>;
///     type Pending = ();
///
///     fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
///         // Return Pending or Ready
///     }
/// }
/// ```
```

### 2.2 TaskStatus Types

```rust
/// TaskStatus represents the current state of a task
pub enum TaskStatus<Ready, Pending, Spawner = NoSpawner> {
    /// Task is ready with a result
    Ready(Ready),

    /// Task is still pending
    Pending(Pending),

    /// Task is delayed until a specific time
    Delayed(std::time::Instant, Pending),

    /// Task needs initialization
    Init,

    /// Task wants to spawn a sub-task
    Spawn(Spawner),

    /// Task should be ignored this iteration
    Ignore,
}
```

### 2.3 Execution Flow

```
┌─────────────────────────────────────────────────────────┐
│                  Executor Loop                           │
│                                                          │
│  for task in tasks:                                      │
│      match task.next():                                  │
│          Some(TaskStatus::Ready(value)) =>              │
│              // Task complete, return value              │
│              results.push(value)                         │
│                                                          │
│          Some(TaskStatus::Pending(_)) =>                │
│              // Still working, check again later         │
│              continue                                    │
│                                                          │
│          Some(TaskStatus::Delayed(until, _)) =>         │
│              // Wait until specific time                 │
│              schedule(task, until)                       │
│                                                          │
│          Some(TaskStatus::Init) =>                      │
│              // First run, initialize                    │
│              task.setup()                                │
│                                                          │
│          Some(TaskStatus::Spawn(spawner)) =>            │
│              // Spawn sub-task                           │
│              spawn(spawner)                              │
│                                                          │
│          Some(TaskStatus::Ignore) =>                    │
│              // Skip this iteration                      │
│              continue                                    │
│                                                          │
│          None =>                                        │
│              // Task completed                           │
│              remove(task)                                │
└─────────────────────────────────────────────────────────┘
```

---

## 3. TUI Event TaskIterator

### 3.1 Input Polling Task

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use crossterm::event::{Event, KeyCode};
use std::time::{Duration, Instant};

/// Task that polls for terminal input events
pub struct InputPollTask {
    poll_interval: Duration,
    last_poll: Instant,
    pending_event: Option<Event>,
}

impl InputPollTask {
    pub fn new(poll_interval: Duration) -> Self {
        Self {
            poll_interval,
            last_poll: Instant::now() - poll_interval, // Poll immediately
            pending_event: None,
        }
    }
}

impl TaskIterator for InputPollTask {
    type Ready = Option<Event>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        // Check if it's time to poll
        if self.last_poll.elapsed() >= self.poll_interval {
            self.last_poll = Instant::now();

            // Non-blocking poll
            if crossterm::event::poll(Duration::ZERO).unwrap_or(false) {
                match crossterm::event::read() {
                    Ok(event) => {
                        return Some(TaskStatus::Ready(Some(event)));
                    }
                    Err(_) => {
                        return Some(TaskStatus::Ready(None));
                    }
                }
            }
        }

        // No event available, return pending
        Some(TaskStatus::Pending(()))
    }
}
```

### 3.2 Timer Task

```rust
/// Task that produces tick events at regular intervals
pub struct TickTask {
    interval: Duration,
    next_tick: Instant,
}

impl TickTask {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            next_tick: Instant::now() + interval,
        }
    }
}

impl TaskIterator for TickTask {
    type Ready = Instant;  // Returns the tick time
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        let now = Instant::now();

        if now >= self.next_tick {
            // Time for a tick
            let tick_time = self.next_tick;
            self.next_tick = now + self.interval;

            Some(TaskStatus::Ready(tick_time))
        } else {
            // Wait until next tick
            Some(TaskStatus::Delayed(self.next_tick, ()))
        }
    }
}
```

### 3.3 HTTP Request Task (Non-Blocking)

```rust
use ureq::Agent;

/// HTTP GET task using synchronous but non-blocking pattern
pub struct HttpGetTask {
    agent: Agent,
    url: String,
    state: HttpState,
}

enum HttpState {
    Init,
    Requesting,
    Waiting { start: Instant, timeout: Duration },
    Done(Result<String, Box<dyn std::error::Error + Send + Sync>>),
}

impl HttpGetTask {
    pub fn new(url: String) -> Self {
        Self {
            agent: ureq::Agent::new(),
            url,
            state: HttpState::Init,
        }
    }
}

impl TaskIterator for HttpGetTask {
    type Ready = Result<String, Box<dyn std::error::Error + Send + Sync>>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match &mut self.state {
            HttpState::Init => {
                // Start the request
                self.state = HttpState::Requesting;
                Some(TaskStatus::Init)
            }

            HttpState::Requesting => {
                // In a real implementation, this would use async I/O
                // For Lambda, we use a simple timeout pattern
                self.state = HttpState::Waiting {
                    start: Instant::now(),
                    timeout: Duration::from_secs(30),
                };
                Some(TaskStatus::Pending(()))
            }

            HttpState::Waiting { start, timeout } => {
                // Check for timeout
                if start.elapsed() > *timeout {
                    self.state = HttpState::Done(Err(
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "Request timed out",
                        ))
                    ));
                }

                // In Lambda, we'd check the HTTP client here
                // For now, simulate with a blocking call (not ideal for TUI)
                match self.agent.get(&self.url).call() {
                    Ok(response) => {
                        match response.into_string() {
                            Ok(body) => {
                                self.state = HttpState::Done(Ok(body));
                            }
                            Err(e) => {
                                self.state = HttpState::Done(Err(Box::new(e)));
                            }
                        }
                    }
                    Err(e) => {
                        self.state = HttpState::Done(Err(Box::new(e)));
                    }
                }

                // Return the result
                match &self.state {
                    HttpState::Done(result) => {
                        Some(TaskStatus::Ready(result.clone()))
                    }
                    _ => Some(TaskStatus::Pending(()))
                }
            }

            HttpState::Done(_) => {
                // Already returned, task complete
                None
            }
        }
    }
}
```

---

## 4. TUI State Machine

### 4.1 Application State

```rust
use std::collections::HashMap;

/// TUI Application state
pub struct TuiApp {
    /// Running state
    pub running: bool,

    /// Current view
    pub view: View,

    /// View-specific state
    pub list_state: ListState,
    pub input_state: InputState,

    /// Data
    pub items: Vec<String>,
    pub selected: usize,

    /// Async state
    pub loading: bool,
    pub error: Option<String>,

    /// Frame counter for animations
    pub frame: u64,
}

pub enum View {
    List,
    Detail,
    Input,
    Error,
}

pub struct ListState {
    pub scroll_offset: usize,
    pub cursor: usize,
}

pub struct InputState {
    pub value: String,
    pub cursor: usize,
    pub focus: bool,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            running: true,
            view: View::List,
            list_state: ListState {
                scroll_offset: 0,
                cursor: 0,
            },
            input_state: InputState {
                value: String::new(),
                cursor: 0,
                focus: false,
            },
            items: Vec::new(),
            selected: 0,
            loading: false,
            error: None,
            frame: 0,
        }
    }
}
```

### 4.2 Event Handler

```rust
use crossterm::event::{Event, KeyCode, KeyEvent};

impl TuiApp {
    /// Handle input event
    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(mouse) => self.handle_mouse(mouse),
            Event::Resize(width, height) => self.handle_resize(width, height),
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.view {
            View::List => self.handle_list_key(key),
            View::Input => self.handle_input_key(key),
            _ => self.handle_global_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Ctrl('c') => {
                self.running = false;
            }

            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }

            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < self.items.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }

            KeyCode::Enter => {
                // Navigate to detail view
                self.view = View::Detail;
            }

            KeyCode::Char('n') => {
                // Open input for new item
                self.view = View::Input;
                self.input_state.focus = true;
            }

            KeyCode::Char('r') => {
                // Refresh data
                self.loading = true;
                self.error = None;
            }

            _ => self.handle_global_key(key),
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_state.focus = false;
                self.view = View::List;
            }

            KeyCode::Enter => {
                // Submit input
                let value = self.input_state.value.clone();
                if !value.is_empty() {
                    self.items.push(value);
                }
                self.input_state.value.clear();
                self.input_state.focus = false;
                self.view = View::List;
            }

            KeyCode::Char(c) => {
                self.input_state.value.push(c);
                self.input_state.cursor += 1;
            }

            KeyCode::Backspace => {
                if self.input_state.cursor > 0 {
                    self.input_state.cursor -= 1;
                    self.input_state.value.remove(self.input_state.cursor);
                }
            }

            _ => {}
        }
    }

    fn handle_global_key(&mut self, _key: KeyEvent) {
        // Global key handling
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        // Mouse handling
    }

    fn handle_resize(&mut self, width: u16, height: u16) {
        // Handle terminal resize
    }

    /// Animation tick handler
    pub fn on_tick(&mut self) {
        self.frame += 1;
    }
}
```

---

## 5. Executor Integration

### 5.1 TUI Executor

```rust
use foundation_core::valtron::{execute, TaskIterator, TaskStatus};

/// Main TUI executor
pub struct TuiExecutor {
    app: TuiApp,
    input_task: InputPollTask,
    tick_task: TickTask,
    pending_tasks: Vec<Box<dyn TaskIterator<Ready = AppEvent>>>,
}

enum AppEvent {
    Input(crossterm::event::Event),
    Tick(Instant),
    DataLoaded(Result<Vec<String>, String>),
}

impl TuiExecutor {
    pub fn new() -> Self {
        Self {
            app: TuiApp::default(),
            input_task: InputPollTask::new(Duration::from_millis(16)), // 60 FPS
            tick_task: TickTask::new(Duration::from_millis(100)),
            pending_tasks: Vec::new(),
        }
    }

    /// Run the TUI executor
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize terminal
        terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

        while self.app.running {
            // 1. Render current state
            terminal.draw(|f| self.app.render(f))?;

            // 2. Process input events
            match self.input_task.next() {
                Some(TaskStatus::Ready(Some(event))) => {
                    self.app.handle_event(event);
                }
                Some(TaskStatus::Ready(None)) => {}
                Some(TaskStatus::Pending(_)) | Some(TaskStatus::Delayed(_, _)) => {}
                None => {}
            }

            // 3. Process tick events
            match self.tick_task.next() {
                Some(TaskStatus::Ready(_)) => {
                    self.app.on_tick();
                }
                _ => {}
            }

            // 4. Process pending tasks
            let mut completed = Vec::new();
            for (i, task) in self.pending_tasks.iter_mut().enumerate() {
                match task.next() {
                    Some(TaskStatus::Ready(event)) => {
                        // Handle task result
                        match event {
                            AppEvent::DataLoaded(Ok(items)) => {
                                self.app.items = items;
                                self.app.loading = false;
                            }
                            AppEvent::DataLoaded(Err(e)) => {
                                self.app.error = Some(e);
                                self.app.loading = false;
                            }
                            _ => {}
                        }
                        completed.push(i);
                    }
                    _ => {}
                }
            }

            // Remove completed tasks (in reverse order to preserve indices)
            for i in completed.into_iter().rev() {
                self.pending_tasks.remove(i);
            }

            // 5. Small sleep to prevent busy-waiting
            std::thread::sleep(Duration::from_millis(16));
        }

        // Cleanup
        execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen, cursor::Show)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }

    /// Start a data fetch task
    pub fn fetch_data(&mut self, url: String) {
        let task = HttpGetTask::new(url);
        // Wrap as AppEvent task
        // (Implementation depends on your specific needs)
    }
}
```

### 5.2 Lambda Deployment

```rust
use aws_lambda_events::{alb::AlbTargetGroupRequest,alb::AlbTargetGroupResponse};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

/// Lambda handler for TUI backend
pub async fn function_handler(event: LambdaEvent<AlbTargetGroupRequest>) -> Result<AlbTargetGroupResponse, Error> {
    // Parse request
    let request = event.payload;

    // Get action from query params or body
    let action = request.query_string_parameters
        .first("action")
        .unwrap_or("render");

    // Process based on action
    let response_body = match action {
        "render" => render_view(&request),
        "input" => handle_input(&request).await,
        "data" => fetch_data(&request).await,
        _ => render_view(&request),
    };

    // Build response
    Ok(AlbTargetGroupResponse {
        status_code: 200,
        body: Some(response_body),
        headers: Default::default(),
        multi_value_headers: Default::default(),
        is_base64_encoded: false,
    })
}

/// Render current view state
fn render_view(request: &AlbTargetGroupRequest) -> String {
    // Get state from DynamoDB or similar
    let state = get_state_from_db();

    // Render to JSON
    serde_json::json!({
        "view": state.view,
        "items": state.items,
        "selected": state.selected,
        "loading": state.loading,
        "error": state.error,
    }).to_string()
}

/// Handle user input
async fn handle_input(request: &AlbTargetGroupRequest) -> String {
    // Parse input from body
    let input: InputRequest = serde_json::from_str(&request.body).unwrap();

    // Update state
    update_state(&input);

    serde_json::json!({"status": "ok"}).to_string()
}

/// Fetch data from external API
async fn fetch_data(request: &AlbTargetGroupRequest) -> String {
    // Use reqwest or similar
    let data = reqwest::get("https://api.example.com/items")
        .await
        .unwrap()
        .json::<Vec<Item>>()
        .await
        .unwrap();

    serde_json::json!({"items": data}).to_string()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
```

---

## 6. State Synchronization

### 6.1 Frontend-Backend Protocol

```rust
/// Message from frontend to backend
#[derive(Serialize, Deserialize)]
pub enum FrontendMessage {
    /// User input event
    Input {
        key: Option<String>,
        mouse: Option<MouseEvent>,
    },

    /// Request render
    Render,

    /// Subscribe to updates
    Subscribe {
        session_id: String,
    },

    /// Unsubscribe
    Unsubscribe {
        session_id: String,
    },
}

/// Message from backend to frontend
#[derive(Serialize, Deserialize)]
pub enum BackendMessage {
    /// Render instructions
    Render {
        view: String,
        content: Vec<Line>,
        cursor: Option<CursorPos>,
    },

    /// State update
    StateUpdate {
        items: Vec<String>,
        selected: usize,
        loading: bool,
        error: Option<String>,
    },

    /// Error
    Error {
        message: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Line {
    pub spans: Vec<Span>,
}

#[derive(Serialize, Deserialize)]
pub struct Span {
    pub content: String,
    pub style: Option<Style>,
}

#[derive(Serialize, Deserialize)]
pub struct Style {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct CursorPos {
    pub row: usize,
    pub col: usize,
}
```

### 6.2 WebSocket Integration

```rust
use tokio::sync::broadcast;

/// WebSocket handler for TUI backend
pub struct WebSocketHandler {
    state: Arc<RwLock<TuiState>>,
    tx: broadcast::Sender<BackendMessage>,
}

impl WebSocketHandler {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            state: Arc::new(RwLock::new(TuiState::default())),
            tx,
        }
    }

    pub async fn handle(&self, ws: WebSocket) {
        let (mut sender, mut receiver) = ws.split();

        // Spawn receiver task
        let mut rx = self.tx.subscribe();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let json = serde_json::to_string(&msg).unwrap();
                sender.send(Message::Text(json)).await.unwrap();
            }
        });

        // Handle incoming messages
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                let frontend_msg: FrontendMessage = serde_json::from_str(&text).unwrap();
                self.handle_message(frontend_msg).await;
            }
        }
    }

    async fn handle_message(&self, msg: FrontendMessage) {
        match msg {
            FrontendMessage::Input { key, mouse } => {
                // Update state based on input
                let mut state = self.state.write().await;
                if let Some(key) = key {
                    state.handle_key(&key);
                }

                // Broadcast state update
                let _ = self.tx.send(BackendMessage::StateUpdate {
                    items: state.items.clone(),
                    selected: state.selected,
                    loading: state.loading,
                    error: state.error.clone(),
                });
            }

            FrontendMessage::Render => {
                let state = self.state.read().await;
                let _ = self.tx.send(BackendMessage::Render {
                    view: format!("{:?}", state.view),
                    content: state.render_lines(),
                    cursor: None,
                });
            }

            _ => {}
        }
    }
}
```

---

## 7. Comparison: Bubble Tea vs Valtron TUI

### 7.1 Architecture Comparison

| Aspect | Bubble Tea | Valtron TUI |
|--------|------------|-------------|
| **Execution Model** | Concurrent goroutines | Single-threaded iterator |
| **Commands** | `tea.Cmd` functions | `TaskIterator` trait |
| **Async** | Goroutines + channels | TaskIterator states |
| **State** | Immutable Model | Mutable TuiApp |
| **Rendering** | String output | JSON/WebSocket |
| **Backend** | Local terminal | Lambda/HTTP |

### 7.2 Code Pattern Comparison

```go
// Bubble Tea: Command pattern
func loadData() tea.Cmd {
    return func() tea.Msg {
        data, err := http.Get("/api/data")
        if err != nil {
            return ErrorMsg{err}
        }
        return DataLoadedMsg{data}
    }
}

// In Update:
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case DataLoadedMsg:
        m.data = msg.Data
        return m, nil
    }
    return m, loadData()  // Start command
}
```

```rust
// Valtron: TaskIterator pattern
struct LoadDataTask {
    state: LoadState,
}

enum LoadState {
    Init,
    Waiting,
    Done(Result<Vec<String>, Error>),
}

impl TaskIterator for LoadDataTask {
    type Ready = Result<Vec<String>, Error>;
    type Pending = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &self.state {
            LoadState::Done(result) => {
                Some(TaskStatus::Ready(result.clone()))
            }
            _ => Some(TaskStatus::Pending(())),
        }
    }
}

// In executor:
fn update(&mut self, event: Event) {
    match event {
        Event::DataLoaded(Ok(items)) => {
            self.app.items = items;
            self.app.loading = false;
        }
        _ => {}
    }
}
```

---

## 8. Deployment Checklist

### 8.1 Lambda Deployment

```yaml
# serverless.yml or SAM template
Resources:
  TuiBackendFunction:
    Type: AWS::Serverless::Function
    Properties:
      CodeUri: ./target/lambda/tui_backend/
      Handler: bootstrap
      Runtime: provided.al2
      MemorySize: 256
      Timeout: 30
      Events:
        Api:
          Type: Api
          Properties:
            Path: /{proxy+}
            Method: ANY
```

### 8.2 Build Script

```bash
#!/bin/bash
# build-lambda.sh

# Install cross-compile target
rustup target add x86_64-unknown-linux-musl

# Install lambda runtime
cargo install cargo-lambda

# Build for Lambda
cargo lambda build --release --target x86_64-unknown-linux-musl

# Deploy
sam deploy --guided
```

---

## Key Takeaways

1. **TaskIterator replaces async**: Iterator-based state machines instead of async/await
2. **Single-threaded execution**: No tokio runtime needed
3. **Lambda-compatible**: Can deploy TUI backends to Lambda
4. **State synchronization**: JSON protocol for frontend-backend communication
5. **WebSocket for realtime**: Push updates to connected clients
6. **No goroutines**: All concurrency through TaskIterator combinators

---

*This completes the Bubble Tea exploration series. See [exploration.md](exploration.md) for the index.*
