# Ratatui-Crossterm Backend Deep Dive

## Overview

The `ratatui-crossterm` crate provides a backend implementation for Ratatui using the Crossterm library. Crossterm is a cross-platform terminal manipulation library that works on Linux, macOS, and Windows.

**Version:** 0.1.0-alpha.2
**Backend Type:** Terminal manipulation via ANSI escape sequences
**Platform Support:** Windows, macOS, Linux

## Architecture

```
ratatui-crossterm/
├── src/
│   ├── crossterm.rs    # Main backend implementation
│   ├── lib.rs          # Module exports
│   └── ...
└── Cargo.toml
```

## CrosstermBackend Implementation

### Structure

```rust
pub struct CrosstermBackend<W: Write> {
    writer: W,
}
```

The backend wraps any type implementing `Write`, typically `stdout()` or `stderr()`.

### Backend Trait Implementation

#### Draw Method

The `draw` method converts Ratatui cells to ANSI escape sequences:

```rust
fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
where
    I: Iterator<Item = (u16, u16, &'a Cell)>,
{
    let mut fg = Color::Reset;
    let mut bg = Color::Reset;
    #[cfg(feature = "underline-color")]
    let mut underline_color = Color::Reset;
    let mut modifier = Modifier::empty();
    let mut last_pos: Option<Position> = None;

    for (x, y, cell) in content {
        // Move cursor if not adjacent to previous position
        if !matches!(last_pos, Some(p) if x == p.x + 1 && y == p.y) {
            queue!(self.writer, MoveTo(x, y))?;
        }
        last_pos = Some(Position { x, y });

        // Apply modifier changes
        if cell.modifier != modifier {
            let diff = ModifierDiff {
                from: modifier,
                to: cell.modifier,
            };
            diff.queue(&mut self.writer)?;
            modifier = cell.modifier;
        }

        // Apply color changes
        if cell.fg != fg || cell.bg != bg {
            queue!(
                self.writer,
                SetColors(CrosstermColors::new(
                    cell.fg.into_crossterm(),
                    cell.bg.into_crossterm(),
                ))
            )?;
            fg = cell.fg;
            bg = cell.bg;
        }

        // Apply underline color (if feature enabled)
        #[cfg(feature = "underline-color")]
        if cell.underline_color != underline_color {
            let color = cell.underline_color.into_crossterm();
            queue!(self.writer, SetUnderlineColor(color))?;
            underline_color = cell.underline_color;
        }

        // Print the character
        queue!(self.writer, Print(cell.symbol()))?;
    }

    // Reset all styles at the end
    #[cfg(feature = "underline-color")]
    return queue!(
        self.writer,
        SetForegroundColor(CrosstermColor::Reset),
        SetBackgroundColor(CrosstermColor::Reset),
        SetUnderlineColor(CrosstermColor::Reset),
        SetAttribute(CrosstermAttribute::Reset),
    );
    #[cfg(not(feature = "underline-color"))]
    return queue!(
        self.writer,
        SetForegroundColor(CrosstermColor::Reset),
        SetBackgroundColor(CrosstermColor::Reset),
        SetAttribute(CrosstermAttribute::Reset),
    );
}
```

**Optimization Strategies:**

1. **Cursor Movement Minimization** - Only moves cursor when not at adjacent position
2. **Style Caching** - Tracks current style to avoid redundant escape sequences
3. **Modifier Diff** - Calculates minimal modifier changes needed
4. **Batched Output** - Uses `queue!` macro for buffered output, flushed later

#### Cursor Operations

```rust
fn hide_cursor(&mut self) -> io::Result<()> {
    execute!(self.writer, Hide)
}

fn show_cursor(&mut self) -> io::Result<()> {
    execute!(self.writer, Show)
}

fn get_cursor_position(&mut self) -> io::Result<Position> {
    crossterm::cursor::position()
        .map(|(x, y)| Position { x, y })
        .map_err(io::Error::other)
}

fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
    let Position { x, y } = position.into();
    execute!(self.writer, MoveTo(x, y))
}
```

#### Clear Operations

```rust
fn clear(&mut self) -> io::Result<()> {
    self.clear_region(ClearType::All)
}

fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
    execute!(
        self.writer,
        Clear(match clear_type {
            ClearType::All => crossterm::terminal::ClearType::All,
            ClearType::AfterCursor => crossterm::terminal::ClearType::FromCursorDown,
            ClearType::BeforeCursor => crossterm::terminal::ClearType::FromCursorUp,
            ClearType::CurrentLine => crossterm::terminal::ClearType::CurrentLine,
            ClearType::UntilNewLine => crossterm::terminal::ClearType::UntilNewLine,
        })
    )
}
```

#### Size Operations

```rust
fn size(&self) -> io::Result<Size> {
    let (width, height) = terminal::size()?;
    Ok(Size { width, height })
}

fn window_size(&mut self) -> io::Result<WindowSize> {
    let crossterm::terminal::WindowSize {
        columns,
        rows,
        width,
        height,
    } = terminal::window_size()?;
    Ok(WindowSize {
        columns_rows: Size { width: columns, height: rows },
        pixels: Size { width, height },
    })
}
```

#### Line Operations

```rust
fn append_lines(&mut self, n: u16) -> io::Result<()> {
    for _ in 0..n {
        queue!(self.writer, Print("\n"))?;
    }
    self.writer.flush()
}
```

#### Scrolling Regions (Optional Feature)

When the `scrolling-regions` feature is enabled:

```rust
#[cfg(feature = "scrolling-regions")]
fn scroll_region_up(&mut self, region: Range<u16>, amount: u16) -> io::Result<()> {
    queue!(
        self.writer,
        ScrollUpInRegion {
            first_row: region.start,
            last_row: region.end.saturating_sub(1),
            lines_to_scroll: amount,
        }
    )?;
    self.writer.flush()
}

#[cfg(feature = "scrolling-regions")]
fn scroll_region_down(&mut self, region: Range<u16>, amount: u16) -> io::Result<()> {
    queue!(
        self.writer,
        ScrollDownInRegion {
            first_row: region.start,
            last_row: region.end.saturating_sub(1),
            lines_to_scroll: amount,
        }
    )?;
    self.writer.flush()
}
```

**Custom ANSI Commands:**

The scrolling region implementation uses custom ANSI escape sequences:

```rust
// ScrollUpInRegion writes:
// ^[[X;Yr   - Set scrolling region from line X to Y
// ^[[NS     - Scroll up N lines
// ^[[r      - Reset scrolling region to full screen
```

## Color Conversion

### Color Mapping

Ratatui colors are converted to Crossterm colors:

```rust
impl IntoCrossterm<CrosstermColor> for Color {
    fn into_crossterm(self) -> CrosstermColor {
        match self {
            Self::Reset => CrosstermColor::Reset,
            Self::Black => CrosstermColor::Black,
            Self::Red => CrosstermColor::DarkRed,
            Self::Green => CrosstermColor::DarkGreen,
            Self::Yellow => CrosstermColor::DarkYellow,
            Self::Blue => CrosstermColor::DarkBlue,
            Self::Magenta => CrosstermColor::DarkMagenta,
            Self::Cyan => CrosstermColor::DarkCyan,
            Self::Gray => CrosstermColor::Grey,
            Self::DarkGray => CrosstermColor::DarkGrey,
            Self::LightRed => CrosstermColor::Red,
            Self::LightGreen => CrosstermColor::Green,
            Self::LightBlue => CrosstermColor::Blue,
            Self::LightYellow => CrosstermColor::Yellow,
            Self::LightMagenta => CrosstermColor::Magenta,
            Self::LightCyan => CrosstermColor::Cyan,
            Self::White => CrosstermColor::White,
            Self::Indexed(i) => CrosstermColor::AnsiValue(i),
            Self::Rgb(r, g, b) => CrosstermColor::Rgb { r, g, b },
        }
    }
}
```

**Note on Color Naming:**

There's a mapping discrepancy between Ratatui and ANSI color names:
- Ratatui `Red` → Crossterm `DarkRed` (ANSI color 1)
- Ratatui `LightRed` → Crossterm `Red` (ANSI bright color 9)

This is historical and matches common terminal emulator conventions.

### Modifier Conversion

Modifiers (text attributes) are converted between Ratatui and Crossterm:

```rust
impl FromCrossterm<CrosstermAttributes> for Modifier {
    fn from_crossterm(value: CrosstermAttributes) -> Self {
        let mut res = Self::empty();
        if value.has(CrosstermAttribute::Bold) { res |= Self::BOLD; }
        if value.has(CrosstermAttribute::Dim) { res |= Self::DIM; }
        if value.has(CrosstermAttribute::Italic) { res |= Self::ITALIC; }
        if value.has(CrosstermAttribute::Underlined)
            || value.has(CrosstermAttribute::DoubleUnderlined)
            || value.has(CrosstermAttribute::Undercurled)
            || value.has(CrosstermAttribute::Underdotted)
            || value.has(CrosstermAttribute::Underdashed)
        { res |= Self::UNDERLINED; }
        if value.has(CrosstermAttribute::SlowBlink) { res |= Self::SLOW_BLINK; }
        if value.has(CrosstermAttribute::RapidBlink) { res |= Self::RAPID_BLINK; }
        if value.has(CrosstermAttribute::Reverse) { res |= Self::REVERSED; }
        if value.has(CrosstermAttribute::Hidden) { res |= Self::HIDDEN; }
        if value.has(CrosstermAttribute::CrossedOut) { res |= Self::CROSSED_OUT; }
        res
    }
}
```

### ModifierDiff

The `ModifierDiff` struct calculates minimal changes between modifier states:

```rust
struct ModifierDiff {
    pub from: Modifier,
    pub to: Modifier,
}

impl ModifierDiff {
    fn queue<W>(self, mut w: W) -> io::Result<()>
    where W: io::Write,
    {
        // Calculate removed modifiers
        let removed = self.from - self.to;
        if removed.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CrosstermAttribute::NoReverse))?;
        }
        if removed.contains(Modifier::BOLD) || removed.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CrosstermAttribute::NormalIntensity))?;
            // Reapply if needed after reset
            if self.to.contains(Modifier::DIM) {
                queue!(w, SetAttribute(CrosstermAttribute::Dim))?;
            }
            if self.to.contains(Modifier::BOLD) {
                queue!(w, SetAttribute(CrosstermAttribute::Bold))?;
            }
        }
        // ... handle other removed modifiers

        // Calculate added modifiers
        let added = self.to - self.from;
        if added.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CrosstermAttribute::Reverse))?;
        }
        if added.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CrosstermAttribute::Bold))?;
        }
        // ... handle other added modifiers

        Ok(())
    }
}
```

## Features

### `underline-color`

Enables underline color support (not available on Windows 7):

```toml
[features]
underline-color = ["ratatui-core/underline-color"]
```

### `scrolling-regions`

Enables terminal scrolling region support for flicker-free insertions:

```toml
[features]
scrolling-regions = ["ratatui-core/scrolling-regions"]
```

### `unstable-backend-writer`

Provides access to the underlying writer:

```rust
#[instability::unstable(feature = "backend-writer")]
pub const fn writer(&self) -> &W { &self.writer }

#[instability::unstable(feature = "backend-writer")]
pub const fn writer_mut(&mut self) -> &mut W { &mut self.writer }
```

## Usage Patterns

### Basic Setup

```rust
use std::io::stdout;
use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

let backend = CrosstermBackend::new(stdout());
let mut terminal = Terminal::new(backend)?;

enable_raw_mode()?;
stdout().execute(EnterAlternateScreen)?;

// Use terminal...

stdout().execute(LeaveAlternateScreen)?;
disable_raw_mode()?;
```

### Using stderr

```rust
use std::io::stderr;
use ratatui::backend::CrosstermBackend;

// Use stderr instead of stdout
let backend = CrosstermBackend::new(stderr());
let terminal = Terminal::new(backend)?;
```

**When to use stderr:**
- When stdout is used for application output that should be preserved
- When building CLI tools that pipe data to stdout
- When you want TUI output to be separate from application output

See the [Ratatui FAQ](https://ratatui.rs/faq/#should-i-use-stdout-or-stderr) for more details.

### Custom Writer

```rust
use std::io::{Write, Cursor};
use ratatui::backend::CrosstermBackend;

// Use a custom buffer for testing
let mut buffer = Vec::new();
let backend = CrosstermBackend::new(buffer);
```

## ANSI Escape Sequences Used

The Crossterm backend generates these ANSI escape sequences:

| Operation | Sequence | Description |
|-----------|----------|-------------|
| Move cursor | `ESC[{row};{col}H` | Move to position |
| Hide cursor | `ESC[?25l` | Hide cursor |
| Show cursor | `ESC[?25h` | Show cursor |
| Clear screen | `ESC[2J` | Clear all |
| Clear from cursor | `ESC[J` | Clear to end |
| Set FG color (16) | `ESC[{n}m` | Standard/bright colors |
| Set FG color (256) | `ESC[38;5;{n}m` | 256-color palette |
| Set FG color (RGB) | `ESC[38;2;{r};{g};{b}m` | True color |
| Set BG color | `ESC[48;2;{r};{g};{b}m` | True color background |
| Set attribute | `ESC[{n}m` | Bold, italic, etc. |
| Reset | `ESC[0m` | Reset all attributes |
| Print character | UTF-8 bytes | The character itself |

## Performance Considerations

1. **Buffered Output** - Uses `queue!` macro to buffer commands, flushed once per frame
2. **Minimal Cursor Movement** - Only moves cursor when necessary
3. **Style Diffing** - Only sends style changes, not full style each cell
4. **Adjacent Cell Optimization** - No cursor move for horizontally adjacent cells

## Limitations

1. **No True Inline Mode** - Unlike some terminal libraries, Crossterm doesn't support true inline output
2. **Windows 7** - Underline color not supported
3. **Terminal Dependent** - Some features depend on terminal emulator capabilities
4. **Raw Mode Required** - Must enable raw mode for proper key handling

## References

- [ratatui-crossterm on crates.io](https://crates.io/crates/ratatui-crossterm)
- [Crossterm Documentation](https://docs.rs/crossterm)
- [ANSI Escape Sequences](https://en.wikipedia.org/wiki/ANSI_escape_code)
- [Terminal Colors](https://github.com/termstandard/colors)
