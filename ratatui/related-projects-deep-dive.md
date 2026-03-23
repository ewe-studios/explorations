# Related Projects Deep Dive

## ansi-to-tui

### Overview

`ansi-to-tui` is a library that converts ANSI color-coded text into `ratatui::text::Text`. It's useful for displaying command output or logs that already contain ANSI escape sequences for colors and styles.

**Version:** 7.0.0
**Author:** Uttarayan Mondal
**Repository:** https://github.com/uttarayan21/ansi-to-tui
**License:** MIT

### Dependencies

```toml
[dependencies]
nom = "7.1"                      # Parser combinator library
tui = { version = "0.29", package = "ratatui" }  # Ratatui TUI library
thiserror = "1.0"                # Error handling
simdutf8 = { version = "0.1", optional = true }  # SIMD UTF-8 validation
smallvec = { version = "1.10.0", features = ["const_generics"] }  # Stack-allocated vectors
```

### Features

- **simd** - SIMD-accelerated UTF-8 validation (default)
- **zero-copy** - Zero-copy parsing where possible (default)

### Architecture

The library uses nom parser combinators to parse ANSI escape sequences:

```
ANSI Input → Parser → Token Stream → Text Builder → ratatui::Text
```

### Usage Example

```rust
use ansi_to_tui::IntoText;

let ansi_text = "\x1b[31mRed text\x1b[0m and \x1b[32mgreen text\x1b[0m";
let text = ansi_text.into_text().unwrap();

// Use in Ratatui
frame.render_widget(Paragraph::new(text), area);
```

### Performance

The library provides benchmarks via criterion:

```shell
cargo bench --bench parsing
```

Features like SIMD and zero-copy parsing improve performance for large inputs.

### Error Handling

Uses `thiserror` for structured error types:

```rust
#[derive(Debug, Error)]
pub enum AnsiParseError {
    #[error("Invalid ANSI sequence: {0}")]
    InvalidSequence(String),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] Utf8Error),
}
```

---

## better-panic

### Overview

`better-panic` provides pretty panic backtraces inspired by Python's tracebacks. It's designed to make Rust panic output more readable and informative.

**Version:** 0.3.0
**Authors:** Armin Ronacher, Joel Höner
**Repository:** https://github.com/mitsuhiko/better-panic
**License:** MIT

### Dependencies

```toml
[dependencies]
backtrace = "0.3.37"                          # Stack trace capture
console = { version = "0.15.0", default-features = false }  # Terminal output
syntect = { version = "4.6.0", optional = true }  # Syntax highlighting (optional)
```

### Usage

```rust
use better_panic::Settings;

// Install the panic hook
Settings::auto().install();

// Now panics will show formatted backtraces
fn main() {
    panic!("Something went wrong!");
}
```

### Output Format

```
thread 'main' panicked at 'Something went wrong!', src/main.rs:10:5

Stack backtrace:
   0: main
         at src/main.rs:10:5
   1: std::rt::lang_start
         at /rustc/.../src/libstd/rt.rs:XX:5
```

### Integration with Ratatui

When building TUI applications, you might want to:
1. Install better-panic BEFORE initializing the terminal
2. Or restore terminal state in panic hook

```rust
use std::panic;
use better_panic::Settings;

fn main() {
    // Install panic hook that restores terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();  // Restore terminal first
        original_hook(panic_info);
    }));

    Settings::auto().install();

    // Initialize terminal after panic hook
    let mut terminal = ratatui::init();
    // ... application code
}
```

---

## ratatui-macros

### Overview

`ratatui-macros` provides convenience macros for creating Ratatui types with less boilerplate.

**Version:** 0.7.0-alpha.0
**Repository:** https://github.com/ratatui-org/ratatui-macros
**License:** MIT

### Dependencies

```toml
[dependencies]
ratatui-core = "0.1.0-alpha.2"
ratatui-widgets = "0.3.0-alpha.1"
```

### Provided Macros

#### `text!` - Create Text

```rust
use ratatui_macros::text;

let t = text!("Hello World");
let t = text!("Hello\nWorld");  // Multi-line
let t = text!(Style::new().red(), "Red text");
```

#### `line!` - Create Line

```rust
use ratatui_macros::line;

let l = line!("Hello");
let l = line!("Hello", " ", "World");
let l = line!(Style::new().bold(), "Bold text");
```

#### `span!` - Create Span

```rust
use ratatui_macros::span;

let s = span!("text");
let s = span!(Style::new().green(), "green text");
```

#### `layout!` - Create Layout

```rust
use ratatui_macros::layout;

let layout = layout!(
    Direction::Vertical,
    [
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ]
);
```

#### `style!` - Create Style

```rust
use ratatui_macros::style;

let s = style!(fg: Red, bg: Black);
let s = style!(fg: Red, bg: Black, bold, italic);
```

---

## atuin

### Overview

Atuin is a replacement for the shell history that uses Ratatui for its TUI interface. It records additional history information and provides a search interface.

**Location in source tree:** `src.ratatui/atuin/`

### Architecture

```
atuin/
├── crates/
│   ├── atuin/              # Main binary
│   ├── atuin-client/       # Client library
│   ├── atuin-common/       # Shared types
│   ├── atuin-daemon/       # Daemon for history sync
│   ├── atuin-dotfiles/     # Dotfiles management
│   ├── atuin-history/      # History management
│   ├── atuin-scripts/      # Script support
│   ├── atuin-server/       # Server implementation
│   ├── atuin-server-database/  # Database layer
│   └── atuin-server-postgres/  # PostgreSQL backend
└── Cargo.toml
```

### Ratatui Usage

Atuin uses Ratatui for:
- History search interface
- Interactive selection
- Status display

### Key Features

- Encrypted history sync
- Contextual history search
- Shell integration
- Statistics and analysis

---

## Other Related Projects

### csvlens

A CSV viewer using Ratatui for terminal-based CSV file navigation and inspection.

### television

A terminal searcher/tool using Ratatui for its interface.

### oha

A terminal-based HTTP load testing tool with Ratatui UI.

### kasuari

A constraint solver library used by Ratatui for layout calculations. Based on the Cassowary algorithm.

```toml
[dependencies]
kasuari = "0.4.0"
```

### unicode-truncate

Used for proper text truncation considering Unicode grapheme clusters.

```toml
[dependencies]
unicode-truncate = "2"
```

---

## Integration Patterns

### Using ansi-to-tui with Ratatui

```rust
use ansi_to_tui::IntoText;
use ratatui::widgets::Paragraph;

fn render_command_output(frame: &mut Frame, area: Rect, output: &str) {
    let text = output.into_text().unwrap_or_else(|_| Text::from(output));
    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, area);
}
```

### Using better-panic in TUI Apps

```rust
use std::panic;
use better_panic::Settings;

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Always restore terminal first
        ratatui::restore();
        eprintln!("\n");  // Add newline after restore
        original_hook(panic_info);
    }));
    Settings::auto().install();
}
```

### Custom Macro Integration

You can create your own macros that combine ratatui-macros with custom logic:

```rust
macro_rules! styled_text {
    ($($text:expr => $style:expr),*) => {{
        use ratatui::text::{Text, Line, Span};
        Text::from(vec![
            $(Line::from(Span::styled($text, $style)),)*
        ])
    }};
}

let text = styled_text!(
    "Error: " => Style::new().red().bold(),
    "File not found" => Style::new().white()
);
```

---

## References

- [ansi-to-tui on crates.io](https://crates.io/crates/ansi-to-tui)
- [better-panic on crates.io](https://crates.io/crates/better-panic)
- [ratatui-macros on crates.io](https://crates.io/crates/ratatui-macros)
- [Atuin GitHub](https://github.com/ellie/atuin)
- [Kasuari on crates.io](https://crates.io/crates/kasuari)
