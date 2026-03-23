# Ratatui Exploration

## Overview

Ratatui is a Rust library for cooking up terminal user interfaces (TUIs). It was forked from the [tui-rs](https://crates.io/crates/tui) crate in 2023 to continue its development. The library provides a set of widgets and utilities to build complex terminal-based user interfaces using an immediate mode rendering architecture.

**Version:** 0.30.0-alpha.2 (as of exploration)
**Minimum Rust Version:** 1.81.0
**License:** MIT

## Project Structure

The Ratatui project is organized as a workspace with multiple crates:

```
ratatui/
├── ratatui/              # Main crate - re-exports core, widgets, and backends
├── ratatui-core/         # Core types and traits (stable API for widget authors)
├── ratatui-widgets/     # Widget implementations
├── ratatui-crossterm/   # Crossterm backend implementation
├── ratatui-termion/     # Termion backend implementation (Unix only)
├── ratatui-termwiz/     # Termwiz backend implementation
├── ratatui-macros/      # Utility macros
└── examples/            # Example applications
```

### Crate Architecture

1. **`ratatui-core`** - The foundational crate containing:
   - `Buffer` and `Cell` types for terminal buffer management
   - `Backend` trait for terminal abstraction
   - `Terminal` and `Frame` for rendering
   - `layout` module with `Rect`, `Position`, `Size`, `Layout`, `Constraint`
   - `style` module with `Color`, `Modifier`, `Style`
   - `text` module with `Line`, `Span`, `Text`
   - `widgets` module with `Widget` and `StatefulWidget` traits

2. **`ratatui-widgets`** - Pre-built widgets including:
   - `Block` - Borders and titles
   - `Paragraph` - Text display with wrapping and alignment
   - `List` - Scrollable list with selection
   - `Table` - Tabular data with columns and rows
   - `BarChart` - Bar charts
   - `Chart` - Line and scatter charts
   - `Gauge` - Progress indicators
   - `Scrollbar` - Scroll indicators
   - `Tabs` - Tab navigation
   - `Canvas` - Drawing canvas for custom graphics
   - `Sparkline` - Mini charts
   - `Calendar` - Date display

3. **Backend Crates** - Terminal abstraction implementations:
   - `ratatui-crossterm` - Cross-platform (Windows/Mac/Linux)
   - `ratatui-termion` - Unix-only, no runtime dependency
   - `ratatui-termwiz` - WezTerm's terminal library

4. **`ratatui`** - The main crate that re-exports everything with convenient defaults

## Core Concepts

### Immediate Mode Rendering

Ratatui uses **immediate mode rendering** with intermediate buffers. This means:
- Every frame, your application must render ALL widgets
- Changes are accumulated in a buffer
- The buffer is compared to the previous frame
- Only changed cells are written to the terminal

This differs from retained mode (like HTML DOM) where widgets persist and auto-redraw.

### Double Buffering

The `Terminal` struct maintains two buffers:
1. **Current buffer** - Where widgets render
2. **Previous buffer** - The last rendered state

On each draw call:
```rust
// Simplified from Terminal::try_draw
let mut frame = self.get_frame();
render_callback(&mut frame);  // User renders to current buffer
self.flush()?;                // Compare buffers, draw diffs
self.swap_buffers();          // Swap current and previous
```

### The Buffer System

The `Buffer` is a grid of `Cell` structs representing the desired terminal state:

```rust
pub struct Buffer {
    pub area: Rect,
    pub content: Vec<Cell>,
}

pub struct Cell {
    symbol: CompactString,      // The character(s) to display
    pub fg: Color,              // Foreground color
    pub bg: Color,              // Background color
    pub modifier: Modifier,     // Text attributes (bold, italic, etc.)
    pub skip: bool,             // Skip during diff (for terminal graphics)
    #[cfg(feature = "underline-color")]
    pub underline_color: Color,
}
```

Key buffer operations:
- **`set_stringn(x, y, string, max_width, style)`** - Write text at position
- **`set_line(x, y, line, max_width)`** - Write a styled line
- **`set_style(area, style)`** - Apply style to an area
- **`diff(&self, other)`** - Compute minimal updates between buffers

The `diff` method handles multi-width characters (like emojis or CJK) correctly by tracking which cells are invalidated by preceding wide characters.

### The Widget System

Ratatui defines two widget traits:

#### `Widget` - For stateless widgets

```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized;
}
```

Widgets are consumed during rendering, acting as "commands" to draw UI elements. Since Ratatui 0.26.0, widgets can also implement `Widget` for references (`impl Widget for &MyWidget`) to allow reuse.

#### `StatefulWidget` - For stateful widgets

```rust
pub trait StatefulWidget {
    type State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

StatefulWidget is used for widgets that need to remember state between frames (e.g., selected item in a list, scroll position).

Example with List:
```rust
let mut state = ListState::default().with_selected(Some(1));
let list = List::new(vec![ListItem::new("Item 1"), ListItem::new("Item 2")]);
frame.render_stateful_widget(list, area, &mut state);
```

### Layout System

The `Layout` module uses a constraint-based solver (kasuari) to split areas:

```rust
use ratatui::layout::{Constraint, Layout};

let vertical = Layout::vertical([
    Constraint::Length(1),   // Fixed 1 row
    Constraint::Min(0),      // Take remaining space
    Constraint::Length(1),   // Fixed 1 row
]);
let [title_area, main_area, status_area] = vertical.areas(frame.area());

let horizontal = Layout::horizontal([Constraint::Fill(1); 2]);
let [left_area, right_area] = horizontal.areas(main_area);
```

Constraint types:
- **`Length(u16)`** - Fixed size
- **`Min(u16)`** - Minimum size, can grow
- **`Max(u16)`** - Maximum size
- **`Percentage(u16)`** - Percentage of parent
- **`Ratio(u16, u16)`** - Fraction of parent
- **`Fill(u16)`** - Proportional fill

The layout result is cached in a thread-local LRU cache for performance.

### Style System

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    #[cfg(feature = "underline-color")]
    pub underline_color: Option<Color>,
    pub add_modifier: Modifier,
    pub sub_modifier: Modifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifier(u16);
// BOLD, DIM, ITALIC, UNDERLINED, SLOW_BLINK, RAPID_BLINK, REVERSED, HIDDEN, CROSSED_OUT
```

Styles can be combined using the `Stylize` trait:
```rust
// Long form
Style::new().fg(Color::Green).bg(Color::White).add_modifier(Modifier::BOLD)

// Short form
"Hello".green().on_white().bold()
```

### Backend System

The `Backend` trait abstracts terminal operations:

```rust
pub trait Backend {
    type Error: Error;
    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where I: Iterator<Item = (u16, u16, &'a Cell)>;
    fn hide_cursor(&mut self) -> Result<(), Self::Error>;
    fn show_cursor(&mut self) -> Result<(), Self::Error>;
    fn get_cursor_position(&mut self) -> Result<Position, Self::Error>;
    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error>;
    fn clear(&mut self) -> Result<(), Self::Error>;
    fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error>;
    fn size(&self) -> Result<Size, Self::Error>;
    fn window_size(&mut self) -> Result<WindowSize, Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
    // Optional: scroll_region_up, scroll_region_down, append_lines
}
```

The Crossterm backend implementation converts cells to ANSI escape sequences:
```rust
fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
where I: Iterator<Item = (u16, u16, &'a Cell)>,
{
    for (x, y, cell) in content {
        // Move cursor if not adjacent
        if !matches!(last_pos, Some(p) if x == p.x + 1 && y == p.y) {
            queue!(self.writer, MoveTo(x, y))?;
        }
        // Apply style changes
        if cell.modifier != modifier { ... }
        if cell.fg != fg || cell.bg != bg {
            queue!(self.writer, SetColors(...))?;
        }
        // Print character
        queue!(self.writer, Print(cell.symbol()))?;
    }
}
```

## Event Handling

Ratatui does NOT include event handling. Applications use the backend library directly:

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

fn handle_events() -> io::Result<bool> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') => return Ok(true),  // Quit
            KeyCode::Up => { /* handle up */ }
            KeyCode::Down => { /* handle down */ }
            _ => {}
        },
        Event::Mouse(mouse) => { /* handle mouse */ }
        Event::Resize(width, height) => { /* handle resize */ }
        _ => {}
    }
    Ok(false)
}
```

## Application Patterns

### Basic Application Loop

```rust
fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame))?;
        if handle_events()? {
            break Ok(());
        }
    }
}

fn draw(frame: &mut Frame) {
    frame.render_widget(Paragraph::new("Hello World!"), frame.area());
}
```

### Using `init()` and `restore()`

Ratatui provides convenience functions:
- `ratatui::init()` - Initializes terminal, enters alternate screen, enables raw mode, sets up panic hook
- `ratatui::restore()` - Restores terminal to original state

## Special Features

### Inline Viewport

For tools that need to display content above the command line:

```rust
use ratatui::{Terminal, TerminalOptions, Viewport};

let backend = CrosstermBackend::new(stdout());
let terminal = Terminal::with_options(
    backend,
    TerminalOptions {
        viewport: Viewport::Inline(10),  // 10 rows high
    },
)?;
```

### Insert Before

For inline viewports, insert content above the viewport:

```rust
terminal.insert_before(1, |buf| {
    Paragraph::new("New log line").render(buf.area, buf);
});
```

### Scrolling Regions

When the `scrolling-regions` feature is enabled, terminals that support it can use ANSI scrolling regions for flicker-free insertions.

## Related Projects in Source Tree

### `ansi-to-tui` (v7.0.0)
A library to convert ANSI color-coded text into `ratatui::text::Text`. Uses nom parser combinators for efficient parsing.

```toml
[dependencies]
nom = "7.1"
tui = { version = "0.29", package = "ratatui" }
thiserror = "1.0"
simdutf8 = { version = "0.1", optional = true }
```

Features:
- SIMD-accelerated parsing (optional)
- Zero-copy parsing (optional)
- Handles ANSI escape sequences for colors and styles

### `better-panic` (v0.3.0)
Pretty panic backtraces inspired by Python's tracebacks. Uses:
- `backtrace` crate for stack traces
- `console` for terminal output
- Optional `syntect` for syntax highlighting

### `ratatui-macros` (v0.7.0-alpha.0)
Convenience macros for creating spans, lines, text, and layouts.

## WASM Support

The exploration found NO native WASM support in Ratatui. The codebase:
- Uses std library features throughout
- Depends on crossterm/termion/termwiz which are terminal-specific
- No wasm-bindgen or web-sys dependencies found

For web-based terminal emulation, alternatives include:
- [xterm.js](https://xtermjs.org/) with a WASM backend
- Custom ANSI parser rendering to HTML canvas
- Using Ratatui's buffer system as a reference for a WASM implementation

## Performance Considerations

1. **Buffer Diffing** - Only changed cells are drawn
2. **Layout Caching** - LRU cache (default 500 entries) for layout results
3. **CompactString** - Cells use inline storage for short strings (avoiding heap allocation)
4. **Modifier Diff** - Only changed text attributes are sent to terminal
5. **Cursor Optimization** - Cursor only moves when not at adjacent position

## Key Design Patterns

### Fluent Setters

Widgets use fluent setter pattern for configuration:

```rust
Paragraph::new(text)
    .block(Block::bordered().title("Title"))
    .style(Style::new().white().on_black())
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true })
```

### Composition

Widgets can contain other widgets:

```rust
let surrounding_block = Block::bordered().title("List");
let list = List::new(items).block(surrounding_block);
```

### Area Splitting

Layout returns areas that can be further subdivided:

```rust
let chunks = Layout::vertical([Length(1), Min(0)]).split(area);
let sub_chunks = Layout::horizontal([Fill(1); 3]).split(chunks[1]);
```

## References

- [Official Website](https://ratatui.rs/)
- [API Documentation](https://docs.rs/ratatui)
- [GitHub Repository](https://github.com/ratatui/ratatui)
- [Examples](https://github.com/ratatui/ratatui/tree/main/ratatui/examples)
- [Discord Server](https://discord.gg/pMCEU9hNEj)
- [Matrix Channel](https://matrix.to/#/#ratatui:matrix.org)
