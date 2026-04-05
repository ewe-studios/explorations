# Terminal Rendering Deep Dive

**Purpose:** Understanding how ClaudOpen renders Markdown to ANSI terminal output with syntax highlighting.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Markdown Parsing](#markdown-parsing)
4. [Syntax Highlighting](#syntax-highlighting)
5. [Color System](#color-system)
6. [Animation (Spinner)](#animation-spinner)
7. [Table Rendering](#table-rendering)
8. [Rust Implementation Details](#rust-implementation-details)
9. [Production Considerations](#production-considerations)

---

## Overview

The terminal rendering system converts Markdown responses from the AI into ANSI-formatted terminal output. Key features:

- **Markdown parsing** via `pulldown-cmark`
- **Syntax highlighting** via `syntect`
- **ANSI color** via `crossterm`
- **Animated spinners** for loading states
- **Table rendering** with borders

### Dependencies

```toml
[dependencies]
pulldown-cmark = "0.13"    # Markdown parser
syntect = "5"              # Syntax highlighting
crossterm = "0.28"         # Terminal control
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       Markdown Input                            │
│                                                                 │
│   # Heading                                                     │
│   **bold** and *italic*                                         │
│   ```rust                                                       │
│   fn main() { println!("Hello"); }                              │
│   ```                                                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    pulldown-cmark Parser                        │
│                                                                 │
│   Events: Start(Heading) → Text("Heading") → End(Heading)      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   TerminalRenderer                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │  RenderState    │  │  ColorTheme     │  │  SyntaxSet     │  │
│  │  - heading_level│  │  - heading      │  │  - languages   │  │
│  │  - emphasis     │  │  - strong       │  │  - syntaxes    │  │
│  │  - quote        │  │  - code         │  │                │  │
│  │  - list_stack   │  │  - ...          │  │                │  │
│  └─────────────────┘  └─────────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    ANSI Output                                  │
│                                                                 │
│   \x1b[36mHeading\x1b[0m                                        │
│   \x1b[1m\x1b[33mbold\x1b[0m and \x1b[3mitalic\x1b[0m           │
│   \x1b[38;5;244m┌────────────────────────┐\x1b[0m               │
└─────────────────────────────────────────────────────────────────┘
```

---

## Markdown Parsing

### pulldown-cmark Events

The parser generates a stream of events:

```rust
use pulldown_cmark::{Parser, Event, Tag, TagEnd, Options};

let markdown = "# Hello\n\n**bold** text";
let options = Options::all();
let parser = Parser::new_ext(markdown, options);

for event in parser {
    match event {
        Event::Start(Tag::Heading { level: 1, .. }) => {
            // Start of h1 heading
        }
        Event::Text(text) => {
            // Text content
        }
        Event::End(TagEnd::Heading(1)) => {
            // End of h1
        }
        _ => {}
    }
}
```

### Event Types

| Event | Description |
|-------|-------------|
| `Start(Tag)` | Beginning of a block/inline element |
| `End(TagEnd)` | End of a block/inline element |
| `Text(CowStr)` | Text content |
| `Code(CowStr)` | Inline code |
| `Html(CowStr)` | Raw HTML |
| `FootnoteReference(CowStr)` | Footnote reference |
| `SoftBreak` | Soft line break |
| `HardBreak` | Hard line break |
| `Rule` | Horizontal rule |
| `TaskListMarker(bool)` | Checkbox state |

### Tags

```rust
pub enum Tag<'a> {
    Paragraph,
    Heading { level: HeadingLevel, id: Option<Cow<'a, str>>, classes: PluralVec<Cow<'a, str>> },
    BlockQuote(Option<BlockQuoteKind>),
    CodeBlock(CodeBlockKind),
    List(Option<u64>),
    Item,
    FootnoteDefinition(Cow<'a, str>),
    Table(Alignment),
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Link { link_type: LinkType, dest_url: Cow<'a, str>, title: Cow<'a, str>, id: Cow<'a, str> },
    Image { link_type: LinkType, dest_url: Cow<'a, str>, title: Cow<'a, str>, id: Cow<'a, str> },
}
```

---

## Syntax Highlighting

### syntect Integration

```rust
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Theme};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

// Load syntax definitions
let syntax_set = SyntaxSet::load_defaults_newlines();

// Load color theme
let theme_set = ThemeSet::load_defaults();
let theme = theme_set.themes.get("base16-ocean.dark").unwrap();

// Find syntax for language
let syntax = syntax_set.find_syntax_by_token("rust").unwrap();

// Create highlighter
let mut highlighter = HighlightLines::new(syntax, theme);

// Highlight code
let code = "fn main() { println!(\"Hello\"); }";
for line in LinesWithEndings::from(code) {
    let regions = highlighter.highlight_line(line, &syntax_set).unwrap();
    let escaped = as_24_bit_terminal_escaped(&regions[..], false);
    print!("{}", escaped);
}
```

### Theme Structure

```rust
pub struct Theme {
    pub name: String,
    pub author: String,
    pub settings: ThemeSettings,
    pub scopes: Vec<(Scope, ScopeSettings)>,
}

pub struct ThemeSettings {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub caret: Option<Color>,
    pub selection: Option<Color>,
    pub line_highlight: Option<Color>,
}
```

### Built-in Themes

| Theme | Description |
|-------|-------------|
| `base16-ocean.dark` | Dark theme, blue tones |
| `base16-ocean.light` | Light theme, blue tones |
| `Solarized (dark)` | Classic solarized dark |
| `Solarized (light)` | Classic solarized light |
| `monokai` | Vibrant colors |

---

## Color System

### ColorTheme Structure

```rust
#[derive(Debug, Clone, Copy)]
pub struct ColorTheme {
    pub heading: Color,           // Cyan
    pub emphasis: Color,          // Magenta
    pub strong: Color,            // Yellow
    pub inline_code: Color,       // Green
    pub link: Color,              // Blue
    pub quote: Color,             // DarkGrey
    pub table_border: Color,      // DarkCyan
    pub code_block_border: Color, // DarkGrey
    pub spinner_active: Color,    // Blue
    pub spinner_done: Color,      // Green
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            heading: Color::Cyan,
            emphasis: Color::Magenta,
            strong: Color::Yellow,
            inline_code: Color::Green,
            link: Color::Blue,
            quote: Color::DarkGrey,
            table_border: Color::DarkCyan,
            code_block_border: Color::DarkGrey,
            spinner_active: Color::Blue,
            spinner_done: Color::Green,
        }
    }
}
```

### crossterm Color API

```rust
use crossterm::style::{Color, Stylize};

// Named colors
let red = Color::Red;
let blue = Color::Blue;

// ANSI 256
let color = Color::AnsiValue(196);  // Bright red

// RGB (truecolor)
let color = Color::Rgb { r: 255, g: 0, b: 0 };

// Apply to text
let text = "Hello".stylize().with(Color::Cyan);
```

### Stylize Methods

```rust
use crossterm::style::Stylize;

let text = "Hello".stylize();

// Colors
text.red().blue().green().yellow().magenta().cyan();

// Attributes
text.bold().italic().underlined().crossed_out();

// Combined
let styled = "Important".stylize().red().bold();
```

---

## Animation (Spinner)

### Braille Spinner

```rust
pub struct Spinner {
    frame_index: usize,
}

impl Spinner {
    const FRAMES: [&str; 10] = [
        "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
    ];

    pub fn tick(&mut self, label: &str, theme: &ColorTheme, out: &mut impl Write) {
        let frame = Self::FRAMES[self.frame_index % Self::FRAMES.len()];
        self.frame_index += 1;

        // Save cursor, move to start, clear line, print frame, restore
        queue!(
            out,
            SavePosition,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(theme.spinner_active),
            Print(format!("{frame} {label}")),
            ResetColor,
            RestorePosition
        )?;

        out.flush()?;
    }

    pub fn finish(&mut self, label: &str, theme: &ColorTheme, out: &mut impl Write) {
        self.frame_index = 0;
        execute!(
            out,
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(theme.spinner_done),
            Print(format!("✔ {label}\n")),
            ResetColor
        )?;
    }
}
```

### Animation Timing

```rust
use std::time::Duration;
use std::thread;

let mut spinner = Spinner::new();
loop {
    spinner.tick("Thinking...", &theme, &mut stdout)?;
    thread::sleep(Duration::from_millis(80));  // ~12.5 FPS
}
```

---

## Table Rendering

### State Machine

```rust
#[derive(Debug, Default)]
struct TableState {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    in_head: bool,
}

impl TableState {
    fn push_cell(&mut self) {
        let cell = self.current_cell.trim().to_string();
        self.current_row.push(cell);
        self.current_cell.clear();
    }

    fn finish_row(&mut self) {
        if self.current_row.is_empty() {
            return;
        }
        let row = std::mem::take(&mut self.current_row);
        if self.in_head {
            self.headers = row;
        } else {
            self.rows.push(row);
        }
    }
}
```

### Rendering Tables

```rust
fn render_table(state: &TableState, theme: &ColorTheme, output: &mut String) {
    if state.headers.is_empty() && state.rows.is_empty() {
        return;
    }

    // Calculate column widths
    let mut widths = vec![0; state.headers.len()];
    for (i, header) in state.headers.iter().enumerate() {
        widths[i] = widths[i].max(header.len());
    }
    for row in &state.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Render header separator
    let separator = widths.iter()
        .map(|w| "─".repeat(w + 2))
        .collect::<Vec<_>>()
        .join("┼");

    // Render header row
    let header = state.headers.iter()
        .enumerate()
        .map(|(i, h)| format!(" {}{} ", h, " ".repeat(widths[i] - h.len())))
        .collect::<Vec<_>>()
        .join("│");

    output.push_str(&format!("┌{}┐\n", "─".repeat(separator.len())));
    output.push_str(&format!("│{}│\n", header.style(theme.table_border)));
    output.push_str(&format!("├{}┤\n", separator.style(theme.table_border)));

    // Render data rows
    for row in &state.rows {
        let row_str = row.iter()
            .enumerate()
            .map(|(i, c)| format!(" {}{} ", c, " ".repeat(widths[i].saturating_sub(c.len()))))
            .collect::<Vec<_>>()
            .join("│");
        output.push_str(&format!("│{}│\n", row_str));
    }

    output.push_str(&format!("└{}┘\n", "─".repeat(separator.len())));
}
```

---

## Rust Implementation Details

### RenderState

```rust
#[derive(Debug, Default)]
struct RenderState {
    emphasis: usize,           // Nesting level
    strong: usize,             // Nesting level
    heading_level: Option<u8>, // Current heading
    quote: usize,              // Nesting level
    list_stack: Vec<ListKind>, // Stack for nested lists
    link_stack: Vec<LinkState>, // Stack for links
    table: Option<TableState>, // Current table
}

#[derive(Debug, Clone)]
enum ListKind {
    Unordered,
    Ordered { next_index: u64 },
}

#[derive(Debug, Clone)]
struct LinkState {
    destination: String,
    text: String,
}
```

### Event Rendering

```rust
fn render_event(
    &self,
    event: Event<'_>,
    state: &mut RenderState,
    output: &mut String,
    code_buffer: &mut String,
    code_language: &mut String,
    in_code_block: &mut bool,
) {
    match event {
        // Headings
        Event::Start(Tag::Heading { level, .. }) => {
            self.start_heading(state, level as u8, output);
        }
        Event::End(TagEnd::Heading(..)) => {
            state.heading_level = None;
            output.push_str("\n\n");
        }

        // Text formatting
        Event::Start(Tag::Emphasis) => {
            state.emphasis += 1;
        }
        Event::End(TagEnd::Emphasis) => {
            state.emphasis = state.emphasis.saturating_sub(1);
        }
        Event::Start(Tag::Strong) => {
            state.strong += 1;
        }
        Event::End(TagEnd::Strong) => {
            state.strong = state.strong.saturating_sub(1);
        }

        // Inline code
        Event::Code(code) => {
            let styled = code.stylize()
                .with(self.color_theme.inline_code)
                .to_string();
            output.push_str(&styled);
        }

        // Code blocks
        Event::Start(Tag::CodeBlock(kind)) => {
            *in_code_block = true;
            if let CodeBlockKind::Fenced(lang) = kind {
                *code_language = lang.to_string();
            }
        }
        Event::End(TagEnd::CodeBlock) => {
            *in_code_block = false;
            let highlighted = self.highlight_code(code_buffer, code_language);
            output.push_str(&highlighted);
            code_buffer.clear();
        }
        Event::Text(text) if *in_code_block => {
            code_buffer.push_str(&text);
        }

        // Links
        Event::Start(Tag::Link { dest_url, .. }) => {
            state.link_stack.push(LinkState {
                destination: dest_url.to_string(),
                text: String::new(),
            });
        }
        Event::End(TagEnd::Link) => {
            if let Some(link) = state.link_stack.pop() {
                let styled = format!("{} ({})", link.text, link.destination)
                    .stylize()
                    .with(self.color_theme.link);
                output.push_str(&styled);
            }
        }

        // Tables
        Event::Start(Tag::Table(..)) => {
            state.table = Some(TableState::default());
        }
        Event::Start(Tag::TableCell) => {
            if let Some(table) = state.table.as_mut() {
                table.current_cell.clear();
            }
        }
        Event::End(TagEnd::TableCell) => {
            if let Some(table) = state.table.as_mut() {
                table.push_cell();
            }
        }
        Event::End(TagEnd::TableRow) => {
            if let Some(table) = state.table.as_mut() {
                table.finish_row();
            }
        }
        Event::End(TagEnd::Table) => {
            if let Some(table) = state.table.take() {
                render_table(&table, &self.color_theme, output);
            }
        }
        Event::Text(text) => {
            if let Some(table) = state.table.as_mut() {
                table.current_cell.push_str(&text);
            } else if let Some(link) = state.link_stack.last_mut() {
                link.text.push_str(&text);
            } else {
                state.append_styled(output, &text, &self.color_theme);
            }
        }

        _ => {}
    }
}
```

---

## Production Considerations

### 1. Terminal Capability Detection

```rust
fn detect_color_support() -> ColorDepth {
    // Check COLORTERM
    if let Ok(term) = env::var("COLORTERM") {
        if term == "truecolor" || term == "24bit" {
            return ColorDepth::TrueColor;
        }
    }

    // Check TERM
    if let Ok(term) = env::var("TERM") {
        if term.contains("256") {
            return ColorDepth::Ansi256;
        }
        if term.contains("16") || term.contains("color") {
            return ColorDepth::Ansi16;
        }
    }

    // Default to safe fallback
    ColorDepth::Ansi16
}
```

### 2. Performance Optimization

```rust
// Cache syntax sets to avoid reload
lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

// Pre-allocate output buffer
let mut output = String::with_capacity(markdown.len() * 2);
```

### 3. Theme Customization

```rust
#[derive(Debug)]
pub struct ColorThemes {
    pub dark: ColorTheme,
    pub light: ColorTheme,
    pub solarized: ColorTheme,
    pub catppuccin: ColorTheme,
}

impl ColorThemes {
    pub fn load(name: &str) -> ColorTheme {
        match name {
            "light" => Self::light(),
            "solarized" => Self::solarized(),
            "catppuccin" => Self::catppuccin(),
            _ => Self::dark(),
        }
    }
}
```

### 4. Streaming Output

```rust
// Buffer and render incrementally
pub struct StreamingRenderer {
    buffer: String,
    last_render: Instant,
    render_interval: Duration,
}

impl StreamingRenderer {
    pub fn push(&mut self, text: &str) {
        self.buffer.push_str(text);

        // Throttle rendering
        if self.last_render.elapsed() > self.render_interval {
            self.render();
            self.last_render = Instant::now();
        }
    }
}
```

### 5. Resize Handling

```rust
use crossterm::terminal;
use crossterm::event::{poll, read, Event};

fn handle_resize() -> io::Result<()> {
    let (width, height) = terminal::size()?;

    // Adjust layout based on width
    if width < 80 {
        // Narrow mode: truncate long lines
    } else {
        // Wide mode: full rendering
    }

    Ok(())
}

// Listen for resize events
loop {
    if poll(Duration::from_millis(100))? {
        if let Event::Resize(width, height) = read()? {
            handle_resize()?;
        }
    }
}
```

---

## What This Looks Like in Rust

### Full Example

```rust
use crossterm::{execute, style::*};
use pulldown_cmark::{Parser, Event, Tag, TagEnd, Options};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};
use std::io::{self, Write};

pub struct TerminalRenderer {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl TerminalRenderer {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: ThemeSet::load_defaults()
                .themes
                .remove("base16-ocean.dark")
                .unwrap(),
        }
    }

    pub fn render(&self, markdown: &str, out: &mut impl Write) -> io::Result<()> {
        let mut state = RenderState::default();
        let mut code_buffer = String::new();
        let mut code_lang = String::new();
        let mut in_code = false;

        for event in Parser::new_ext(markdown, Options::all()) {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    state.heading_level = Some(level as u8);
                    execute!(out, SetForegroundColor(Color::Cyan), SetAttribute(Attribute::Bold))?;
                }
                Event::End(TagEnd::Heading(_)) => {
                    state.heading_level = None;
                    execute!(out, ResetColor, Print("\n\n"))?;
                }
                Event::Start(Tag::Strong) => {
                    state.strong += 1;
                    execute!(out, SetAttribute(Attribute::Bold))?;
                }
                Event::End(TagEnd::Strong) => {
                    state.strong = state.strong.saturating_sub(1);
                    if state.strong == 0 {
                        execute!(out, ResetAttribute(Attribute::Bold))?;
                    }
                }
                Event::Code(code) => {
                    execute!(out, SetForegroundColor(Color::Green))?;
                    execute!(out, Print(&code))?;
                    execute!(out, ResetColor)?;
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code = true;
                    if let CodeBlockKind::Fenced(lang) = kind {
                        code_lang = lang.to_string();
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code = false;
                    let highlighted = self.highlight_code(&code_buffer, &code_lang);
                    execute!(out, Print(highlighted))?;
                    code_buffer.clear();
                }
                Event::Text(text) if in_code => {
                    code_buffer.push_str(&text);
                }
                Event::Text(text) => {
                    execute!(out, Print(text.to_string()))?;
                }
                _ => {}
            }
        }

        out.flush()
    }

    fn highlight_code(&self, code: &str, lang: &str) -> String {
        let syntax = self.syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut result = String::new();

        for line in LinesWithEndings::from(code) {
            let regions = highlighter.highlight_line(line, &self.syntax_set).unwrap();
            result.push_str(&as_24_bit_terminal_escaped(&regions[..], false));
        }

        result
    }
}

#[derive(Default)]
struct RenderState {
    heading_level: Option<u8>,
    strong: usize,
}

// Usage
fn main() -> io::Result<()> {
    let renderer = TerminalRenderer::new();
    let markdown = "# Hello\n\n```rust\nfn main() { println!(\"Hi\"); }\n```";

    let mut stdout = io::stdout();
    renderer.render(markdown, &mut stdout)?;

    Ok(())
}
```

---

## What a Production-Grade Version Looks Like

### Full TUI with ratatui

```rust
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Paragraph, Block, Borders},
    Frame, Terminal,
};
use tui_textarea::TextArea;

struct App {
    conversation: Vec<Message>,
    input: TextArea<'static>,
    scroll_offset: u16,
}

impl App {
    fn new() -> Self {
        Self {
            conversation: vec![],
            input: TextArea::default(),
            scroll_offset: 0,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),  // Conversation
                Constraint::Length(3), // Input
            ])
            .split(frame.size());

        // Render conversation
        let messages: Vec<Paragraph> = self.conversation
            .iter()
            .map(|msg| {
                Paragraph::new(&msg.content)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title(&msg.role))
            })
            .collect();

        let scrollable = ScrollableMessages::new(messages)
            .scroll_offset(self.scroll_offset);

        frame.render_widget(scrollable, chunks[0]);

        // Render input
        frame.render_widget(&self.input, chunks[1]);
    }
}

fn main() -> io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| app.render(f))?;

        if let Event::Key(key) = crossterm::event::read()? {
            if key.code == KeyCode::Char('q') {
                break;
            }
            app.input.input(key);
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}
```

### Key Improvements

1. **Full-screen layout** with split panes
2. **Scrollable conversation history**
3. **Interactive input** with completion
4. **Mouse support** for clicking and scrolling
5. **Resize handling** with responsive layout
6. **Status bar** with model, tokens, session info
7. **Collapsible sections** for tool results
8. **Progress bars** for streaming
9. **Color themes** with configuration
10. **Pager** for long outputs

---

## References

- [pulldown-cmark Documentation](https://docs.rs/pulldown-cmark/)
- [syntect Documentation](https://docs.rs/syntect/)
- [crossterm Documentation](https://docs.rs/crossterm/)
- [ratatui Documentation](https://docs.rs/ratatui/)

---

*Generated: 2026-04-02*
