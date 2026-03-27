---
title: "Bubble Tea: Rust Revision - Ratatui Translation Guide"
subtitle: "Complete guide to translating Bubble Tea patterns to Rust with ratatui"
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/bubbletea/
target: Rust with ratatui + crossterm
---

# Bubble Tea: Rust Revision

## 1. Overview

### 1.1 What We're Translating

Bubble Tea is a Go framework for building TUIs using the Elm Architecture. The Rust equivalent uses **ratatui** (successor to tui-rs) with **crossterm** for terminal backend.

| Go (Bubble Tea) | Rust (Ratatui) |
|-----------------|----------------|
| `tea.Model` interface | `App` struct with state |
| `tea.Cmd` functions | `Result` returns + events |
| `tea.Msg` types | `Event` enum (crossterm) |
| `tea.Batch`, `tea.Sequence` | `tokio::join!`, async combinators |
| Bubbles components | ratatui widgets, tui-react |
| Lip Gloss styling | ratatui `Style`, tui-rs styling |
| `tea.Program` | `Terminal` + `EventLoop` |

### 1.2 Key Design Decisions

#### Architecture Comparison

```go
// Go: Bubble Tea Elm Architecture
type Model interface {
    Init() Cmd
    Update(Msg) (Model, Cmd)
    View() string
}
```

```rust
// Rust: Ratatui with crossterm event loop
use ratatui::Frame;
use crossterm::event::{Event, KeyEvent};

struct App {
    running: bool,
    counter: i32,
}

impl App {
    fn update(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => self.running = false,
                KeyCode::Up => self.counter += 1,
                KeyCode::Down => self.counter -= 1,
                _ => {}
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let text = format!("Counter: {}", self.counter);
        frame.render_str(text, frame.size());
    }
}
```

#### Ownership Strategy

```go
// Go: GC handles everything
type Model struct {
    items  []string
    viewport viewport.Model
}
```

```rust
// Rust: Explicit ownership
use std::rc::Rc;
use ratatui::widgets::List;

struct App {
    items: Vec<String>,
    selected: usize,
    scroll_offset: u16,
}

// Or with shared state
struct SharedState {
    items: Rc<Vec<String>>,
}
```

#### Async Patterns

```go
// Go: tea.Cmd for async
func loadData() tea.Cmd {
    return func() tea.Msg {
        data, _ := http.Get("/api/data")
        return DataLoadedMsg{data}
    }
}
```

```rust
// Rust: tokio async (if using async runtime)
// OR valtron TaskIterator (no async/await)

// With tokio:
async fn load_data() -> Result<Vec<Item>, Error> {
    reqwest::get("/api/data").await?.json().await
}

// With valtron (no async):
struct LoadDataTask {
    url: String,
    state: LoadState,
}

enum LoadState {
    Init,
    Waiting(std::time::Instant),
    Done(Result<Vec<Item>, Error>),
}

impl TaskIterator for LoadDataTask {
    type Ready = Result<Vec<Item>, Error>;
    type Pending = ();

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &self.state {
            LoadState::Init => {
                // Start HTTP request (non-blocking)
                self.state = LoadState::Waiting(std::time::Instant::now());
                Some(TaskStatus::Pending(()))
            }
            LoadState::Waiting(start) => {
                if start.elapsed() > TIMEOUT {
                    self.state = LoadState::Done(Err(Error::Timeout));
                }
                Some(TaskStatus::Pending(()))
            }
            LoadState::Done(result) => {
                Some(TaskStatus::Ready(result.clone()))
            }
        }
    }
}
```

---

## 2. Type System Design

### 2.1 App State Structure

```rust
use ratatui::style::{Color, Style, Modifier};
use ratatui::widgets::{Block, Borders, List, ListItem};
use crossterm::event::KeyEvent;

/// Main application state
pub struct App {
    /// Whether the app should continue running
    pub running: bool,

    /// Application state
    pub counter: i32,
    pub items: Vec<String>,
    pub selected: usize,

    /// UI state
    pub scroll_offset: u16,
    pub viewport_height: u16,

    /// Component state
    pub input_value: String,
    pub input_focus: bool,

    /// Async state
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            counter: 0,
            items: vec!["Item 1".to_string(), "Item 2".to_string()],
            selected: 0,
            scroll_offset: 0,
            viewport_height: 10,
            input_value: String::new(),
            input_focus: false,
            loading: false,
            error: None,
        }
    }
}
```

### 2.2 Event Handling

```rust
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEvent, MouseButton};

/// Application events
pub enum AppEvent {
    /// Terminal event from crossterm
    Crossterm(Event),

    /// Custom application events
    DataLoaded(Vec<String>),
    SaveComplete,
    Error(String),

    /// Tick for animations/timers
    Tick,
}

impl App {
    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Crossterm(Event::Key(key)) => self.handle_key(key),
            AppEvent::Crossterm(Event::Mouse(mouse)) => self.handle_mouse(mouse),
            AppEvent::Crossterm(Event::Resize(width, height)) => {
                self.handle_resize(width, height)
            }
            AppEvent::DataLoaded(items) => {
                self.items = items;
                self.loading = false;
            }
            AppEvent::Error(msg) => {
                self.error = Some(msg);
                self.loading = false;
            }
            AppEvent::Tick => {
                // Animation tick
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false
            }
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected < self.items.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                // Select item
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.button {
            MouseButton::Left => {
                // Handle click
            }
            MouseButton::WheelUp => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            MouseButton::WheelDown => {
                if self.selected < self.items.len().saturating_sub(1) {
                    self.selected += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_resize(&mut self, width: u16, height: u16) {
        self.viewport_height = height.saturating_sub(4); // Reserve space for header/footer
    }
}
```

### 2.3 Rendering

```rust
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

impl App {
    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Main content
                Constraint::Length(3),  // Footer
            ])
            .split(frame.size());

        self.render_header(frame, chunks[0]);
        self.render_main(frame, chunks[1]);
        self.render_footer(frame, chunks[2]);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new("My Application")
            .style(Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)))
            .alignment(ratatui::style::Alignment::Center);

        frame.render_widget(title, area);
    }

    fn render_main(&mut self, frame: &mut Frame, area: Rect) {
        // Loading state
        if self.loading {
            let loading = Paragraph::new("Loading...")
                .alignment(ratatui::style::Alignment::Center);
            frame.render_widget(loading, area);
            return;
        }

        // Error state
        if let Some(ref error) = self.error {
            let error_text = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)));
            frame.render_widget(error_text, area);
            return;
        }

        // List of items
        let items: Vec<ListItem> = self.items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let prefix = if i == self.selected { "> " } else { "  " };
                ListItem::new(format!("{}{}", prefix, item)).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Items"));

        frame.render_widget(list, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let footer = Paragraph::new("↑/↓: Navigate | Enter: Select | q: Quit")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::style::Alignment::Center);

        frame.render_widget(footer, area);
    }
}
```

---

## 3. Component Patterns

### 3.1 Spinner Component

```rust
use std::time::{Duration, Instant};

pub struct Spinner {
    frames: Vec<&'static str>,
    current: usize,
    last_update: Instant,
    interval: Duration,
}

impl Default for Spinner {
    fn default() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current: 0,
            last_update: Instant::now(),
            interval: Duration::from_millis(100),
        }
    }
}

impl Spinner {
    pub fn update(&mut self) {
        if self.last_update.elapsed() >= self.interval {
            self.current = (self.current + 1) % self.frames.len();
            self.last_update = Instant::now();
        }
    }

    pub fn render(&self) -> &str {
        self.frames[self.current]
    }
}

// Usage in App
struct App {
    spinner: Spinner,
    loading: bool,
}

fn render(&mut self, frame: &mut Frame, area: Rect) {
    if self.loading {
        self.spinner.update();
        let text = format!("{} Loading...", self.spinner.render());
        frame.render_str(text, area);
    }
}
```

### 3.2 Progress Bar Component

```rust
pub struct ProgressBar {
    percent: f64,
    width: u16,
    filled_char: char,
    empty_char: char,
}

impl ProgressBar {
    pub fn new(width: u16) -> Self {
        Self {
            percent: 0.0,
            width,
            filled_char: '█',
            empty_char: '░',
        }
    }

    pub fn set_percent(&mut self, percent: f64) {
        self.percent = percent.clamp(0.0, 1.0);
    }

    pub fn render(&self) -> String {
        let filled = ((self.percent * self.width as f64) as u16).min(self.width);
        let empty = self.width - filled;

        format!(
            "[{}{}] {:.0}%",
            self.filled_char.to_string().repeat(filled as usize),
            self.empty_char.to_string().repeat(empty as usize),
            self.percent * 100.0
        )
    }
}
```

### 3.3 Text Input Component

```rust
use crossterm::event::KeyEvent;

pub struct TextInput {
    value: String,
    cursor: usize,
    placeholder: String,
    focus: bool,
    width: u16,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            placeholder: String::new(),
            focus: false,
            width: 30,
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) {
        if !self.focus {
            return;
        }

        match key.code {
            KeyCode::Char(c) => {
                self.value.insert(self.cursor, c);
                self.cursor += 1;
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.value.remove(self.cursor);
                }
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor < self.value.len() {
                    self.cursor += 1;
                }
            }
            _ => {}
        }
    }

    pub fn render(&self) -> String {
        let display = if self.value.is_empty() {
            self.placeholder.clone()
        } else {
            self.value.clone()
        };

        if self.focus {
            format!("[{}>]", display)
        } else {
            format!(" {}", display)
        }
    }

    pub fn focus(&mut self) {
        self.focus = true;
    }

    pub fn blur(&mut self) {
        self.focus = false;
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
```

---

## 4. Ratatui vs Bubble Tea Comparison

### 4.1 Architecture Comparison

| Aspect | Bubble Tea (Go) | Ratatui (Rust) |
|--------|-----------------|----------------|
| **Model** | Interface with Init/Update/View | Struct with methods |
| **Messages** | `tea.Msg` interface | `crossterm::Event` enum |
| **Commands** | `tea.Cmd` functions | Async tasks / TaskIterator |
| **Rendering** | String output | `Frame` widget rendering |
| **Styling** | Lip Gloss | ratatui `Style` |
| **Components** | Bubbles library | ratatui widgets + custom |
| **Event Loop** | Built-in | Manual with crossterm |
| **Terminal** | Built-in raw mode | crossterm backend |

### 4.2 Code Translation Examples

**Counter App:**

```go
// Go: Bubble Tea
type counterModel struct {
    counter int
}

func (m counterModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyMsg:
        switch msg.String() {
        case "q", "ctrl+c":
            return m, tea.Quit
        case "+":
            m.counter++
        case "-":
            m.counter--
        }
    }
    return m, nil
}

func (m counterModel) View() string {
    return fmt.Sprintf("Counter: %d\n\n+/-: Increment/Decrement\nq: Quit\n", m.counter)
}
```

```rust
// Rust: Ratatui
struct CounterApp {
    counter: i32,
    running: bool,
}

impl CounterApp {
    fn update(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => self.running = false,
                KeyCode::Char('+') => self.counter += 1,
                KeyCode::Char('-') => self.counter -= 1,
                _ => {}
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let text = format!(
            "Counter: {}\n\n+/-: Increment/Decrement\nq: Quit\n",
            self.counter
        );
        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        frame.render_widget(paragraph, frame.size());
    }
}
```

**List App:**

```go
// Go: Bubble Tea with Bubbles List
type Model struct {
    list list.Model
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    var cmd tea.Cmd
    m.list, cmd = m.list.Update(msg)

    if k, ok := msg.(tea.KeyMsg); ok && k.String() == "enter" {
        selected := m.list.SelectedItem().(Item)
        // Handle selection
    }

    return m, cmd
}
```

```rust
// Rust: Ratatui with custom list
struct ListApp {
    items: Vec<String>,
    selected: usize,
    running: bool,
}

impl ListApp {
    fn update(&mut self, event: Event) {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => self.running = false,
                KeyCode::Up => {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.selected < self.items.len().saturating_sub(1) {
                        self.selected += 1;
                    }
                }
                KeyCode::Enter => {
                    let selected = &self.items[self.selected];
                    // Handle selection
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let items: Vec<ListItem> = self.items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(item.as_str()).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Items"));

        frame.render_widget(list, frame.size());
    }
}
```

---

## 5. Valtron Integration (No Async/Tokio)

### 5.1 TaskIterator for TUI Events

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus};

/// Task that polls for terminal events
pub struct InputPollTask {
    timeout: Duration,
    last_poll: Instant,
}

impl TaskIterator for InputPollTask {
    type Ready = Option<crossterm::event::Event>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        if self.last_poll.elapsed() >= self.timeout {
            self.last_poll = Instant::now();

            // Non-blocking poll
            if crossterm::event::poll(Duration::ZERO).unwrap() {
                let event = crossterm::event::read().unwrap();
                Some(TaskStatus::Ready(Some(event)))
            } else {
                Some(TaskStatus::Ready(None))
            }
        } else {
            Some(TaskStatus::Pending(()))
        }
    }
}
```

### 5.2 TUI Event Loop with Valtron

```rust
use foundation_core::valtron::{execute, TaskIterator};

struct TuiExecutor {
    app: App,
    input_task: InputPollTask,
    tick_task: TickTask,
}

impl TuiExecutor {
    fn run(mut self) -> Result<()> {
        // Initialize terminal
        terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

        while self.app.running {
            // Render
            terminal.draw(|f| self.app.render(f))?;

            // Poll for events (non-blocking)
            match self.input_task.next() {
                Some(TaskStatus::Ready(Some(event))) => {
                    self.app.handle_event(event);
                }
                Some(TaskStatus::Ready(None)) => {}
                Some(TaskStatus::Pending(_)) => {}
                None => break,
            }

            // Handle tick for animations
            match self.tick_task.next() {
                Some(TaskStatus::Ready(_)) => {
                    self.app.on_tick();
                }
                _ => {}
            }

            // Small sleep to prevent busy-waiting
            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }

        // Cleanup
        execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen, cursor::Show)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }
}
```

### 5.3 HTTP Request without Async

```rust
use ureq; // Synchronous HTTP client

pub struct HttpGetTask {
    url: String,
    state: HttpState,
}

enum HttpState {
    Init,
    Waiting(ureq::Request),
    Done(Result<String, Box<dyn std::error::Error>>),
}

impl TaskIterator for HttpGetTask {
    type Ready = Result<String, Box<dyn std::error::Error>>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match &mut self.state {
            HttpState::Init => {
                let request = ureq::get(&self.url);
                self.state = HttpState::Waiting(request);
                Some(TaskStatus::Pending(()))
            }
            HttpState::Waiting(request) => {
                // In a real implementation, this would use non-blocking I/O
                // For now, we'll do a blocking call (not ideal for TUI)
                let result = request.call()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                    .and_then(|resp| {
                        resp.into_string()
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                    });

                self.state = HttpState::Done(result.clone());
                Some(TaskStatus::Ready(result))
            }
            HttpState::Done(result) => {
                // Already returned result
                None
            }
        }
    }
}
```

---

## 6. Production Considerations

### 6.1 Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Terminal error: {0}")]
    Terminal(#[from] crossterm::ErrorKind),

    #[error("Ratatui error: {0}")]
    Ratatui(#[from] ratatui::errors::RatatuiError),

    #[error("HTTP error: {0}")]
    Http(#[from] ureq::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
```

### 6.2 Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_increment() {
        let mut app = App::default();
        app.handle_event(AppEvent::Crossterm(Event::Key(KeyEvent::new(
            KeyCode::Char('+'),
            KeyModifiers::empty(),
        ))));
        assert_eq!(app.counter, 1);
    }

    #[test]
    fn test_quit_on_q() {
        let mut app = App::default();
        app.handle_event(AppEvent::Crossterm(Event::Key(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::empty(),
        ))));
        assert!(!app.running);
    }

    #[test]
    fn test_navigation_bounds() {
        let mut app = App {
            items: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            selected: 0,
            ..Default::default()
        };

        // Can't go up from first item
        app.handle_event(AppEvent::Crossterm(Event::Key(KeyEvent::new(
            KeyCode::Up,
            KeyModifiers::empty(),
        ))));
        assert_eq!(app.selected, 0);

        // Navigate down
        app.handle_event(AppEvent::Crossterm(Event::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::empty(),
        ))));
        assert_eq!(app.selected, 1);
    }
}
```

### 6.3 Dependencies (Cargo.toml)

```toml
[package]
name = "my-tui-app"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.24"
crossterm = "0.27"
thiserror = "1.0"

# Optional: for async support
# tokio = { version = "1", features = ["full"] }

# Optional: for valtron (no async)
foundation_core = { path = "/home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core" }

# Optional: for HTTP
ureq = "2.9"  # Synchronous
# reqwest = { version = "0.11", features = ["json"] }  # Async

[dev-dependencies]
pretty_assertions = "1.4"
```

---

## Key Takeaways

1. **Elm Architecture translates well**: Model-View-Update pattern works in Rust
2. **ratatui is more imperative**: Direct widget rendering vs string output
3. **Ownership is explicit**: No GC, use Rc/Arc for shared state
4. **valtron avoids async**: TaskIterator for non-blocking operations
5. **Error handling with Result**: Type-safe errors vs Go's error returns
6. **Testing is easier**: Pure functions, no hidden state

---

*Continue to [production-grade.md](production-grade.md) for deployment patterns.*
