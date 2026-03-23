# Ratatui-Core Deep Dive

## Overview

`ratatui-core` is the foundational crate of the Ratatui project, providing the essential building blocks for creating terminal user interfaces in Rust. It was split from the main `ratatui` crate to offer better stability for widget library authors.

**Version:** 0.1.0-alpha.3
**Purpose:** Core types and traits for widget library authors
**Key Design Goal:** Stable API that changes less frequently than the main ratatui crate

## Crate Structure

```
ratatui-core/
├── src/
│   ├── backend/       # Backend trait and implementations
│   ├── buffer/        # Buffer and Cell types
│   ├── layout/        # Layout, Rect, Constraint, etc.
│   ├── style/         # Color, Modifier, Style
│   ├── symbols/       # Unicode symbols for borders, bars, etc.
│   ├── terminal/      # Terminal and Frame types
│   ├── text/          # Line, Span, Text, Masked
│   └── widgets/       # Widget and StatefulWidget traits
└── Cargo.toml
```

## Module Deep Dives

### Buffer Module (`buffer/`)

The buffer module implements the double-buffering system that is central to Ratatui's rendering.

#### `Buffer`

A `Buffer` represents a rectangular area of the terminal that can be rendered to:

```rust
pub struct Buffer {
    pub area: Rect,
    pub content: Vec<Cell>,
}
```

**Key Methods:**

```rust
// Construction
Buffer::empty(area: Rect) -> Self           // All cells set to default
Buffer::filled(area: Rect, cell: Cell) -> Self  // All cells initialized same
Buffer::with_lines(lines: Iter) -> Self     // Create from string lines

// Access
buffer[(x, y)]                         // Index into buffer (panics if out of bounds)
buffer.cell(position) -> Option<&Cell> // Safe access
buffer.cell_mut(position) -> Option<&mut Cell>

// Writing content
buffer.set_string(x, y, string, style)           // Write string
buffer.set_stringn(x, y, string, max_width, style) // Write with max width
buffer.set_line(x, y, line, max_width)           // Write styled line
buffer.set_span(x, y, span, max_width)           // Write a span
buffer.set_style(area, style)                    // Apply style to area

// Buffer operations
buffer.resize(area: Rect)         // Resize buffer
buffer.reset()                    // Reset all cells to default
buffer.merge(&other)              // Merge another buffer
buffer.diff(&self, other) -> Vec<(u16, u16, &Cell)> // Compute changes
```

**The `diff` Algorithm:**

The `diff` method is crucial for performance. It compares two buffers and returns only the cells that need to be updated:

```rust
pub fn diff<'a>(&self, other: &'a Self) -> Vec<(u16, u16, &'a Cell)> {
    let mut updates: Vec<(u16, u16, &Cell)> = vec![];
    let mut invalidated: usize = 0;
    let mut to_skip: usize = 0;

    for (i, (current, previous)) in other.content.iter().zip(self.content.iter()).enumerate() {
        if !current.skip && (current != previous || invalidated > 0) && to_skip == 0 {
            let (x, y) = self.pos_of(i);
            updates.push((x, y, &other.content[i]));
        }

        to_skip = current.symbol().width().saturating_sub(1);
        let affected_width = cmp::max(current.symbol().width(), previous.symbol().width());
        invalidated = cmp::max(affected_width, invalidated).saturating_sub(1);
    }
    updates
}
```

**Multi-width Character Handling:**

The diff algorithm handles multi-width characters (like emojis or CJK characters):

```
Example 1: Replacing wide char with narrow chars
(Index:) 01
Prev:    コ  (wide character)
Next:    aa  (two narrow characters)
Updates: 0: a, 1: a

Example 2: Replacing narrow with wide char
(Index:) 01
Prev:    a
Next:    コ
Updates: 0: コ (skip index 1)
```

#### `Cell`

A `Cell` represents a single terminal cell with its visual attributes:

```rust
pub struct Cell {
    symbol: CompactString,      // The character(s) to display
    pub fg: Color,              // Foreground color
    pub bg: Color,              // Background color
    #[cfg(feature = "underline-color")]
    pub underline_color: Color,
    pub modifier: Modifier,     // Text attributes
    pub skip: bool,             // Skip during diff
}
```

**CompactString Usage:**

Cells use `CompactString` instead of `String` for the symbol field. This is a wrapper that stores short strings inline (up to 24 bytes on 64-bit) without heap allocation. Since most terminal cells contain a single character, this significantly reduces memory usage and allocation overhead.

### Terminal Module (`terminal/`)

The terminal module provides the interface between Ratatui and the terminal backend.

#### `Terminal`

```rust
pub struct Terminal<B>
where
    B: Backend,
{
    backend: B,
    buffers: [Buffer; 2],       // Double buffering
    current: usize,             // Index of current buffer
    hidden_cursor: bool,
    viewport: Viewport,
    viewport_area: Rect,
    last_known_area: Rect,
    last_known_cursor_pos: Position,
    frame_count: usize,
}
```

**Key Methods:**

```rust
// Creation
Terminal::new(backend: B) -> Result<Self, B::Error>
Terminal::with_options(backend: B, options: TerminalOptions) -> Result<Self, B::Error>

// Rendering
terminal.draw(|frame| { ... }) -> Result<CompletedFrame, B::Error>
terminal.try_draw(|frame| { ... }) -> Result<CompletedFrame, B::Error>

// Buffer access
terminal.get_frame() -> Frame
terminal.current_buffer_mut() -> &mut Buffer
terminal.flush() -> Result<(), B::Error>

// Terminal operations
terminal.resize(area: Rect) -> Result<(), B::Error>
terminal.autoresize() -> Result<(), B::Error>
terminal.clear() -> Result<(), B::Error>
terminal.hide_cursor() -> Result<(), B::Error>
terminal.show_cursor() -> Result<(), B::Error>
terminal.set_cursor_position(position) -> Result<(), B::Error>
```

**The Draw Cycle:**

```rust
pub fn try_draw<F, E>(&mut self, render_callback: F) -> Result<CompletedFrame, B::Error>
where
    F: FnOnce(&mut Frame) -> Result<(), E>,
    E: Into<B::Error>,
{
    // 1. Autoresize if needed
    self.autoresize()?;

    // 2. Get a Frame for rendering
    let mut frame = self.get_frame();

    // 3. Call user's render callback
    render_callback(&mut frame).map_err(Into::into)?;

    // 4. Extract cursor position from frame
    let cursor_position = frame.cursor_position;

    // 5. Flush the buffer to terminal
    self.flush()?;

    // 6. Handle cursor
    match cursor_position {
        None => self.hide_cursor()?,
        Some(position) => {
            self.show_cursor()?;
            self.set_cursor_position(position)?;
        }
    }

    // 7. Swap buffers
    self.swap_buffers();

    // 8. Flush backend
    self.backend.flush()?;

    // 9. Return completed frame
    let completed_frame = CompletedFrame {
        buffer: &self.buffers[1 - self.current],
        area: self.last_known_area,
        count: self.frame_count,
    };

    self.frame_count = self.frame_count.wrapping_add(1);
    Ok(completed_frame)
}
```

#### `Frame`

A `Frame` is a view into the terminal state for a single render pass:

```rust
pub struct Frame<'a> {
    pub(crate) cursor_position: Option<Position>,
    pub(crate) viewport_area: Rect,
    pub(crate) buffer: &'a mut Buffer,
    pub(crate) count: usize,
}
```

**Key Methods:**

```rust
frame.area() -> Rect                       // Get renderable area
frame.render_widget(widget, area)          // Render a widget
frame.render_stateful_widget(widget, area, state) // Render stateful widget
frame.set_cursor_position(position)        // Set cursor for after render
frame.buffer_mut() -> &mut Buffer          // Get raw buffer access
frame.count() -> usize                     // Get frame count
```

#### `Viewport`

The viewport determines what area of the terminal is used:

```rust
pub enum Viewport {
    Fullscreen,            // Entire terminal screen
    Inline(u16),           // Inline with specified height
    Fixed(Rect),           // Fixed rectangular area
}
```

**Inline Viewport:**

The inline viewport is special - it renders content inline with terminal output, like a enhanced prompt:

```
+---------------------+
| pre-existing output |
+---------------------+
|   inline viewport   |  <- Your TUI renders here
+---------------------+
```

When you call `insert_before()` with an inline viewport, content is inserted above the viewport:

```
+---------------------+
| pre-existing output |
|   inserted line 1   |  <- Inserted content
|   inserted line 2   |
+---------------------+
|   inline viewport   |
+---------------------+
```

### Backend Module (`backend/`)

The backend module defines the abstraction layer for different terminal libraries.

#### `Backend` Trait

```rust
pub trait Backend {
    type Error: Error;

    // Draw content (iterator of x, y, cell tuples)
    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>;

    // Line insertion (optional)
    fn append_lines(&mut self, _n: u16) -> Result<(), Self::Error> {
        Ok(())
    }

    // Cursor control
    fn hide_cursor(&mut self) -> Result<(), Self::Error>;
    fn show_cursor(&mut self) -> Result<(), Self::Error>;
    fn get_cursor_position(&mut self) -> Result<Position, Self::Error>;
    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error>;

    // Clearing
    fn clear(&mut self) -> Result<(), Self::Error>;
    fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error>;

    // Size information
    fn size(&self) -> Result<Size, Self::Error>;
    fn window_size(&mut self) -> Result<WindowSize, Self::Error>;

    // Flush output
    fn flush(&mut self) -> Result<(), Self::Error>;

    // Scrolling regions (optional, behind feature flag)
    #[cfg(feature = "scrolling-regions")]
    fn scroll_region_up(&mut self, region: Range<u16>, line_count: u16) -> Result<(), Self::Error>;
    #[cfg(feature = "scrolling-regions")]
    fn scroll_region_down(&mut self, region: Range<u16>, line_count: u16) -> Result<(), Self::Error>;
}
```

#### `ClearType`

```rust
pub enum ClearType {
    All,             // Clear entire screen
    AfterCursor,     // Clear from cursor to end
    BeforeCursor,    // Clear from start to cursor
    CurrentLine,     // Clear current line
    UntilNewLine,    // Clear to end of line
}
```

#### `TestBackend`

A backend for testing that stores rendered output in memory:

```rust
pub struct TestBackend {
    buffer: Buffer,
    cursor: Option<Position>,
    viewport: Viewport,
}

impl TestBackend {
    pub fn new(width: u16, height: u16) -> Self;
    pub fn buffer(&self) -> &Buffer;
    pub fn resize(&mut self, width: u16, height: u16);
    pub fn assert_buffer(&self, expected: &Buffer);  // For testing
}
```

### Layout Module (`layout/`)

The layout module provides constraint-based layout using the kasuari solver.

#### `Layout`

```rust
pub struct Layout {
    direction: Direction,
    constraints: Vec<Constraint>,
    margin: Margin,
    flex: Flex,
    spacing: Spacing,
}
```

**Constraint Types:**

```rust
pub enum Constraint {
    Percentage(u16),      // Percentage of parent (0-100)
    Ratio(u16, u16),      // Fraction of parent (numerator, denominator)
    Length(u16),          // Fixed length
    Min(u16),             // Minimum length, can grow
    Max(u16),             // Maximum length, can shrink
    Fill(u16),            // Proportional fill (weight)
}
```

**Layout Solving:**

The layout system uses the kasuari constraint solver. Constraints are converted to linear equations and inequalities:

```rust
// Example constraint conversion
Constraint::Length(10)  => length == 10
Constraint::Min(5)      => length >= 5
Constraint::Max(20)     => length <= 20
Constraint::Fill(2)     => length = 2 * share_size
```

**Caching:**

Layout results are cached in a thread-local LRU cache:

```rust
thread_local! {
    static LAYOUT_CACHE: RefCell<Cache> = RefCell::new(
        LruCache::new(DEFAULT_CACHE_SIZE)
    );
}

type Cache = LruCache<(Rect, Layout), (Segments, Spacers)>;
```

### Widgets Module (`widgets/`)

The widgets module defines the traits for renderable components.

#### `Widget` Trait

```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized;
}
```

**Blanket Implementations:**

```rust
// String types
impl Widget for &str { ... }
impl Widget for String { ... }

// Option types
impl<W: Widget> Widget for Option<W> { ... }

// References (for reusable widgets)
impl<W: WidgetRef> Widget for &W { ... }
```

#### `StatefulWidget` Trait

```rust
pub trait StatefulWidget {
    type State: ?Sized;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

**Example State Implementation:**

```rust
// From the List widget
pub struct ListState {
    offset: usize,      // Scroll offset
    selected: Option<usize>, // Selected item index
}

impl ListState {
    pub fn selected(&self) -> Option<usize>;
    pub fn select(&mut self, index: Option<usize>);
    pub fn offset(&self) -> usize;
}
```

### Style Module (`style/`)

The style module provides types for terminal styling.

#### `Color`

```rust
pub enum Color {
    Reset,
    Black, Red, Green, Yellow, Blue, Magenta, Cyan, Gray,
    DarkGray, LightRed, LightGreen, LightBlue, LightYellow,
    LightMagenta, LightCyan, White,
    Indexed(u8),        // 256-color palette index
    Rgb(u8, u8, u8),    // True color
}
```

#### `Modifier`

```rust
pub struct Modifier(u16);

impl Modifier {
    pub const BOLD: Self = Self(1 << 0);
    pub const DIM: Self = Self(1 << 1);
    pub const ITALIC: Self = Self(1 << 2);
    pub const UNDERLINED: Self = Self(1 << 3);
    pub const SLOW_BLINK: Self = Self(1 << 4);
    pub const RAPID_BLINK: Self = Self(1 << 5);
    pub const REVERSED: Self = Self(1 << 6);
    pub const HIDDEN: Self = Self(1 << 7);
    pub const CROSSED_OUT: Self = Self(1 << 8);
}
```

Modifiers use bitflags, allowing combination:

```rust
let style = Modifier::BOLD | Modifier::UNDERLINED;
```

#### `Style`

```rust
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    #[cfg(feature = "underline-color")]
    pub underline_color: Option<Color>,
    pub add_modifier: Modifier,
    pub sub_modifier: Modifier,
}
```

**Style Patching:**

Styles can be combined using `patch()`:

```rust
let base = Style::new().fg(Color::White).bg(Color::Black);
let overlay = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
let combined = base.patch(overlay);
// Result: fg=Red, bg=Black, add_modifier=BOLD
```

### Text Module (`text/`)

The text module provides types for styled text.

#### `Span`

```rust
pub struct Span<'a> {
    pub content: Cow<'a, str>,
    pub style: Style,
}
```

#### `Line`

```rust
pub struct Line<'a> {
    pub spans: Vec<Span<'a>>,
    pub style: Style,
    pub alignment: Option<Alignment>,
}
```

#### `Text`

```rust
pub struct Text<'a> {
    pub lines: Vec<Line<'a>>,
    pub style: Style,
}
```

**Conversions:**

```rust
// String literals become single-line text
let text: Text = "Hello".into();

// Vec of strings becomes multi-line text
let text: Text = vec!["Line 1", "Line 2"].into();

// Spans can be combined into lines
let line: Line = vec!["Hello ".into(), "World".red()].into();
```

## Production Patterns

### Custom Widget Implementation

Here's how to implement a custom widget following Ratatui patterns:

```rust
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;

/// A simple greeting widget
pub struct Greeting<'a> {
    name: &'a str,
    style: Style,
}

impl<'a> Greeting<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            style: Style::default(),
        }
    }

    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }
}

impl Widget for Greeting<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render text with style
        buf.set_string(
            area.x,
            area.y,
            format!("Hello, {}!", self.name),
            self.style,
        );
    }
}

// Also implement for references to allow reuse
impl Widget for &Greeting<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_string(
            area.x,
            area.y,
            format!("Hello, {}!", self.name),
            self.style,
        );
    }
}
```

### Stateful Widget Implementation

```rust
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;

pub struct ClickCounter {
    clicks: u32,
}

pub struct ClickCounterState {
    count: u32,
}

impl StatefulWidget for ClickCounter {
    type State = ClickCounterState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_string(
            area.x,
            area.y,
            format!("Clicks: {}", state.count),
            Default::default(),
        );
    }
}

// Usage:
// let mut state = ClickCounterState { count: 0 };
// frame.render_stateful_widget(ClickCounter { clicks: 0 }, area, &mut state);
// state.count += 1;  // Update state based on events
```

## References

- [ratatui-core on crates.io](https://crates.io/crates/ratatui-core)
- [API Documentation](https://docs.rs/ratatui-core)
- [Kasuari Solver](https://crates.io/crates/kasuari)
- [CompactString](https://crates.io/crates/compact_str)
