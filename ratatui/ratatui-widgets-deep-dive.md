# Ratatui-Widgets Deep Dive

## Overview

`ratatui-widgets` is a collection of pre-built widgets for building terminal user interfaces using Ratatui. It provides common UI components like lists, tables, charts, and more.

**Version:** 0.3.0-alpha.2
**Purpose:** Reusable widget implementations
**Widgets Count:** 15+ built-in widgets

## Widget Catalog

| Widget | Description | Stateful |
|--------|-------------|----------|
| `Block` | Borders and titles | No |
| `Paragraph` | Text display with wrapping | No |
| `List` | Scrollable list | Yes |
| `Table` | Tabular data | Yes |
| `BarChart` | Bar charts | No |
| `Chart` | Line/scatter plots | No |
| `Gauge` | Progress indicators | No |
| `Scrollbar` | Scroll indicators | No |
| `Tabs` | Tab navigation | No |
| `Canvas` | Drawing canvas | No |
| `Sparkline` | Mini sparkline charts | No |
| `Calendar` | Calendar display | No |
| `Clear` | Clear widget area | No |
| `Logo` | Ratatui logo | No |
| `Mascot` | Animated mascot | Yes |

## Block Widget

The `Block` widget is a container that adds borders and titles around other widgets.

### Structure

```rust
pub struct Block<'a> {
    titles: Vec<(Option<Position>, Line<'a>)>,
    titles_style: Style,
    titles_alignment: Alignment,
    titles_position: Position,
    borders: Borders,
    border_style: Style,
    border_set: border::Set,
    style: Style,
    padding: Padding,
}
```

### Border Types

```rust
pub struct Borders(u8);

impl Borders {
    pub const NONE: Self = Self(0b0000);
    pub const TOP: Self = Self(0b0001);
    pub const BOTTOM: Self = Self(0b0010);
    pub const LEFT: Self = Self(0b0100);
    pub const RIGHT: Self = Self(0b1000);
    pub const ALL: Self = Self(0b1111);
}
```

### Border Sets

```rust
pub const PLAIN: Set = Set {
    top_left: "┌", top_right: "┐", bottom_left: "└", bottom_right: "┘",
    vertical_left: "│", vertical_right: "│", horizontal_top: "─", horizontal_bottom: "─",
};

pub const ROUNDED: Set = Set {
    top_left: "╭", top_right: "╮", bottom_left: "╰", bottom_right: "╯",
    vertical_left: "│", vertical_right: "│", horizontal_top: "─", horizontal_bottom: "─",
};

pub const DOUBLE: Set = Set {
    top_left: "╔", top_right: "╗", bottom_left: "╚", bottom_right: "╝",
    vertical_left: "║", vertical_right: "║", horizontal_top: "═", horizontal_bottom: "═",
};

pub const THICK: Set = Set {
    top_left: "┏", top_right: "┓", bottom_left: "┗", bottom_right: "┛",
    vertical_left: "┃", vertical_right: "┃", horizontal_top: "━", horizontal_bottom: "━",
};
```

### Usage

```rust
use ratatui::widgets::{Block, Borders, BorderType};

Block::new()
    .borders(Borders::ALL)
    .border_type(BorderType::Rounded)
    .border_style(Style::new().blue())
    .style(Style::new().bg(Color::Black))
    .title("My Title")
    .title_alignment(Alignment::Center)
    .padding(Padding::new(1, 1, 0, 0));  // left, right, top, bottom
```

### Inner Area Calculation

```rust
let block = Block::bordered();
let inner_area = block.inner(outer_area);

// This calculates the area inside the borders, accounting for:
// - Left/right borders
// - Top/bottom borders
// - Padding
```

## Paragraph Widget

The `Paragraph` widget displays styled text with optional wrapping and alignment.

### Structure

```rust
pub struct Paragraph<'a> {
    block: Option<Block<'a>>,
    style: Style,
    wrap: Option<Wrap>,
    text: Text<'a>,
    scroll: Position,
    alignment: Alignment,
}

pub struct Wrap {
    pub trim: bool,  // Trim leading whitespace when wrapping
}
```

### Rendering Logic

The paragraph rendering involves several steps:

1. **Render Block** - If present, render the surrounding block first
2. **Calculate Inner Area** - Get area inside block borders
3. **Apply Scrolling** - Skip lines/columns based on scroll offset
4. **Line Composition** - Either wrap or truncate lines based on configuration
5. **Alignment** - Apply left/center/right alignment to each line
6. **Render to Buffer** - Draw the composed lines to the buffer

### Text Wrapping

```rust
// Word wrapping (breaks at word boundaries)
Paragraph::new(text).wrap(Wrap { trim: true });

// Line truncation (default)
Paragraph::new(text);  // No wrap call = truncation

// Custom wrapping with trimming
Paragraph::new(text)
    .wrap(Wrap { trim: true });  // Trim leading whitespace on wrapped lines
```

### Scrolling

```rust
Paragraph::new(text)
    .scroll((offset_y, offset_x));  // (vertical, horizontal)
```

### Usage Example

```rust
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::text::{Text, Line, Span};
use ratatui::style::{Style, Color};

let text = Text::from(vec![
    Line::from(vec![
        Span::styled("Hello ", Style::default().fg(Color::Yellow)),
        Span::raw("World!"),
    ]),
    Line::from("Second line".red()),
]);

Paragraph::new(text)
    .block(Block::bordered().title("Output"))
    .style(Style::new().white().on_black())
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true })
    .scroll((0, 5));  // Scroll 5 columns right
```

## List Widget

The `List` widget displays a scrollable list of items with optional selection.

### Structure

```rust
pub struct List<'a> {
    block: Option<Block<'a>>,
    items: Vec<ListItem<'a>>,
    style: Style,
    highlight_style: Style,
    highlight_symbol: Option<&'a str>,
    start_corner: Corner,  // Where selection starts
}

pub struct ListItem<'a> {
    content: Text<'a>,
    style: Style,
}

pub struct ListState {
    offset: usize,      // First visible item index
    selected: Option<usize>,  // Selected item index
}
```

### Rendering Behavior

The list implements natural scrolling:
- Selected item stays visible as you navigate
- Scroll offset is preserved between renders
- Highlight symbol appears next to selected item

### Usage Example

```rust
use ratatui::widgets::{List, ListItem, ListState};

// Create items
let items = vec![
    ListItem::new("Item 1"),
    ListItem::new("Item 2").style(Style::new().red()),
    ListItem::new(Line::from(vec!["Styled ".blue(), "Item".green()])),
];

// Create list
let list = List::new(items)
    .block(Block::bordered().title("Items"))
    .highlight_style(Style::new().bold().bg(Color::Blue))
    .highlight_symbol(">> ");

// Manage state
let mut state = ListState::default();
state.select(Some(0));  // Select first item

// Render
frame.render_stateful_widget(list, area, &mut state);

// Navigate
state.select(Some(state.selected().map_or(0, |i| (i + 1) % items.len())));
```

### ListState Methods

```rust
impl ListState {
    pub fn selected(&self) -> Option<usize>;
    pub fn select(&mut self, index: Option<usize>);
    pub fn offset(&self) -> usize;
    pub fn offset_mut(&mut self) -> &mut usize;
}
```

## Table Widget

The `Table` widget displays tabular data with columns and rows.

### Structure

```rust
pub struct Table<'a> {
    block: Option<Block<'a>>,
    header: Option<Row<'a>>,
    rows: Vec<Row<'a>>,
    widths: Vec<Constraint>,
    style: Style,
    highlight_style: Style,
    highlight_symbol: Option<&'a str>,
    column_spacing: u16,
    width_constraints: Vec<Constraint>,
    header_style: Style,
    row_highlight_style: Style,
}

pub struct Row<'a> {
    cells: Vec<Cell<'a>>,
    style: Style,
    bottom_margin: u16,
    height: u16,
}

pub struct Cell<'a> {
    content: Text<'a>,
    style: Style,
}

pub struct TableState {
    offset: usize,
    selected: Option<usize>,
}
```

### Column Width Constraints

```rust
use ratatui::layout::Constraint;

let table = Table::new(
    rows,
    [
        Constraint::Length(10),   // Fixed 10 characters
        Constraint::Percentage(30), // 30% of available width
        Constraint::Min(5),        // At least 5 characters
        Constraint::Max(20),       // At most 20 characters
    ],
);
```

### Usage Example

```rust
use ratatui::widgets::{Table, Row, Cell, TableState};

let rows = vec![
    Row::new(vec![
        Cell::from("Row 1, Col 1"),
        Cell::from("Row 1, Col 2"),
    ]).style(Style::new().on_black()),
    Row::new(vec![
        Cell::from("Row 2, Col 1"),
        Cell::from("Row 2, Col 2").style(Style::new().red()),
    ]),
];

let header = Row::new(vec![
    Cell::from("Header 1").style(Style::new().bold()),
    Cell::from("Header 2").style(Style::new().bold()),
]);

let table = Table::new(rows, [Constraint::Percentage(50), Constraint::Percentage(50)])
    .header(header)
    .block(Block::bordered().title("Data Table"))
    .column_spacing(3)
    .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");

let mut state = TableState::default();
state.select(Some(0));

frame.render_stateful_widget(table, area, &mut state);
```

### Highlight Spacing

```rust
pub enum HighlightSpacing {
    Always,   // Always show highlight column
    WhenSelected,  // Only show when item selected
    Never,    // Never show highlight column
}

Table::new(rows, constraints)
    .highlight_spacing(HighlightSpacing::WhenSelected);
```

## Chart Widget

The `Chart` widget displays data as line or scatter plots.

### Structure

```rust
pub struct Chart<'a> {
    block: Option<Block<'a>>,
    datasets: Vec<Dataset<'a>>,
    x_axis: Axis<'a>,
    y_axis: Axis<'a>,
    area: Option<Rect>,
    style: Style,
    hidden_legend_constraints: (Constraint, Constraint),
    legend_position: Option<Position>,
}

pub struct Dataset<'a> {
    name: &'a str,
    data: &[(f64, f64)],
    graph_type: GraphType,
    style: Style,
    marker: Symbol,
}

pub enum GraphType {
    Line,
    Scatter,
}

pub struct Axis<'a> {
    title: Option<Line<'a>>,
    style: Style,
    bounds: [f64; 2],
    labels: Vec<Line<'a>>,
}
```

### Usage Example

```rust
use ratatui::widgets::{Chart, Dataset, GraphType, Axis};
use ratatui::symbols;

let data = vec![
    (0.0, 0.0),
    (1.0, 1.0),
    (2.0, 4.0),
    (3.0, 9.0),
];

let dataset = Dataset::default()
    .name("y = x²")
    .data(&data)
    .graph_type(GraphType::Line)
    .style(Style::default().yellow())
    .marker(symbols::Marker::Dot);

let chart = Chart::new(vec![dataset])
    .block(Block::bordered().title("Chart"))
    .x_axis(
        Axis::default()
            .title("X")
            .bounds([0.0, 3.0])
            .labels(vec!["0".into(), "1".into(), "2".into(), "3".into()]),
    )
    .y_axis(
        Axis::default()
            .title("Y")
            .bounds([0.0, 10.0])
            .labels(vec!["0".into(), "5".into(), "10".into()]),
    );

frame.render_widget(chart, area);
```

### Chart Markers

```rust
pub mod Marker {
    pub const DOT: char = '·';
    pub const BRAILLE: [[char; 2]; 2] = [
        ['⠁', '⠂'],
        ['⠄', '⡀'],
    ];  // For high-density plots
    pub const BAR: char = '▇';
    pub const FULL: char = '█';
}
```

## Gauge Widget

The `Gauge` widget displays a progress bar or percentage indicator.

### Structure

```rust
pub struct Gauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,  // 0.0 to 1.0
    label: Option<Line<'a>>,
    style: Style,
    gauge_style: Style,
    use_unicode: bool,
    symbols: GaugeSymbols,
}
```

### Usage Example

```rust
use ratatui::widgets::Gauge;

// Basic gauge
let gauge = Gauge::default()
    .ratio(0.66)  // 66%
    .block(Block::bordered().title("Progress"))
    .label("Processing...")
    .style(Style::default().fg(Color::White))
    .gauge_style(Style::default().fg(Color::Blue));

frame.render_widget(gauge, area);

// Unicode gauge (smoother)
let gauge = Gauge::default()
    .ratio(0.66)
    .use_unicode(true);  // Uses ▏▎▍▌▋▊▉ characters
```

## BarChart Widget

The `BarChart` widget displays data as vertical bars.

### Structure

```rust
pub struct BarChart<'a> {
    block: Option<Block<'a>>,
    data: Vec<BarGroup<'a>>,
    bar_width: u16,
    bar_gap: u16,
    bar_style: Style,
    bar_set: symbols::bar::Set,
    value_style: Style,
    label_style: Style,
    max: Option<u64>,
    direction: Direction,
}

pub struct BarGroup<'a> {
    label: Option<Line<'a>>,
    bars: Vec<Bar<'a>>,
}

pub struct Bar<'a> {
    label: Option<Line<'a>>,
    value: u64,
    style: Style,
    text_style: Style,
    value_style: Style,
}
```

### Usage Example

```rust
use ratatui::widgets::{BarChart, Bar, BarGroup};

let chart = BarChart::default()
    .block(Block::bordered().title("Sales"))
    .bar_width(3)
    .bar_gap(1)
    .bar_style(Style::default().fg(Color::Green))
    .data(&[
        BarGroup::default()
            .label("Q1".into())
            .bars(&[
                Bar::value(100).label("Jan"),
                Bar::value(150).label("Feb"),
                Bar::value(200).label("Mar"),
            ]),
    ]);

frame.render_widget(chart, area);
```

## Canvas Widget

The `Canvas` widget provides a drawing surface for custom graphics.

### Structure

```rust
pub struct Canvas<'a, 'b> {
    block: Option<Block<'a>>,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    layers: Vec<CanvasLayer<'b>>,
    background_color: Color,
}

pub struct CanvasLayer<'a> {
    items: Vec<CanvasItem<'a>>,
}

pub enum CanvasItem<'a> {
    Line { x1: f64, y1: f64, x2: f64, y2: f64, color: Color },
    Rectangle { x: f64, y: f64, width: f64, height: f64, color: Color },
    Circle { x: f64, y: f64, radius: f64, color: Color },
    Point { x: f64, y: f64, color: Color },
    Text { x: f64, y: f64, text: &'a str, color: Color },
}
```

### Usage Example

```rust
use ratatui::widgets::canvas::{Canvas, Circle, Line, Map, Rectangle};

let canvas = Canvas::default()
    .block(Block::bordered())
    .x_bounds([-180.0, 180.0])
    .y_bounds([-90.0, 90.0])
    .layer(|ctx| {
        ctx.draw(&Map);
        ctx.draw(&Line {
            x1: 0.0, y1: 0.0,
            x2: 100.0, y2: 50.0,
            color: Color::Red,
        });
        ctx.draw(&Circle {
            x: 50.0, y: 50.0,
            radius: 10.0,
            color: Color::Blue,
        });
        ctx.draw(&Rectangle {
            x: 20.0, y: 20.0,
            width: 30.0, height: 20.0,
            color: Color::Green,
        });
    });

frame.render_widget(canvas, area);
```

## Scrollbar Widget

The `Scrollbar` widget displays scroll position indicators.

### Structure

```rust
pub struct Scrollbar<'a> {
    orientation: ScrollbarOrientation,
    begin_symbol: Option<&'a str>,
    end_symbol: Option<&'a str>,
    thumb_symbol: Option<&'a str>,
    track_symbol: Option<&'a str>,
    style: Style,
    thumb_style: Style,
    position: u16,
    content_length: u16,
    viewport_content_length: u16,
}

pub enum ScrollbarOrientation {
    Vertical { begin: bool, end: bool },
    Horizontal { begin: bool, end: bool },
}
```

### Usage Example

```rust
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

// Create state
let mut scrollbar_state = ScrollbarState::new(100)  // content length
    .position(25);  // current position

// Create scrollbar
let scrollbar = Scrollbar::default()
    .orientation(ScrollbarOrientation::Vertical { begin: true, end: true })
    .begin_symbol(Some("↑"))
    .end_symbol(Some("↓"))
    .thumb_symbol("█")
    .track_symbol("│");

// Render outside the main area
frame.render_stateful_widget(
    scrollbar,
    area,  // The scrollbar's own area
    &mut scrollbar_state,
);
```

## Sparkline Widget

The `Sparkline` widget displays a mini line chart.

### Structure

```rust
pub struct Sparkline<'a> {
    block: Option<Block<'a>>,
    data: Vec<u64>,
    style: Style,
    max: Option<u64>,
    direction: Direction,
}
```

### Usage Example

```rust
use ratatui::widgets::Sparkline;

let sparkline = Sparkline::default()
    .block(Block::bordered().title("Trend"))
    .data(&[1, 3, 2, 5, 4, 7, 6, 9, 8])
    .style(Style::default().fg(Color::Green))
    .max(Some(10));

frame.render_widget(sparkline, area);
```

## Tabs Widget

The `Tabs` widget displays tab navigation.

### Structure

```rust
pub struct Tabs<'a> {
    block: Option<Block<'a>>,
    titles: Vec<Line<'a>>,
    select: usize,  // Selected tab index
    style: Style,
    highlight_style: Style,
    divider: Span<'a>,
}
```

### Usage Example

```rust
use ratatui::widgets::Tabs;

let tabs = Tabs::new(vec!["Tab 1", "Tab 2", "Tab 3"])
    .block(Block::bordered())
    .select(0)  // Select first tab
    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
    .divider(Span::raw(" | "));

frame.render_widget(tabs, area);
```

## Composition Patterns

### Nesting Widgets

```rust
fn render(frame: &mut Frame, area: Rect) {
    // Outer block
    let outer_block = Block::bordered()
        .title("Outer")
        .style(Style::new().blue());

    // Get inner area
    let inner_area = outer_block.inner(area);

    // Render outer block
    frame.render_widget(outer_block, area);

    // Inner content
    let inner_block = Block::bordered()
        .title("Inner")
        .style(Style::new().green());

    frame.render_widget(inner_block, inner_area);
}
```

### Layout with Widgets

```rust
fn render(frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // Title
        Constraint::Min(0),     // Content
        Constraint::Length(1),  // Status
    ]).split(area);

    // Title
    let title = Paragraph::new("My App")
        .style(Style::new().bold());
    frame.render_widget(title, chunks[0]);

    // Main content
    let content = List::new(items)
        .block(Block::bordered());
    frame.render_widget(content, chunks[1]);

    // Status bar
    let status = Paragraph::new("Ready")
        .block(Block::bordered().border_type(BorderType::Double));
    frame.render_widget(status, chunks[2]);
}
```

### Custom Composite Widget

```rust
pub struct Card<'a> {
    title: &'a str,
    content: Paragraph<'a>,
}

impl<'a> Widget for Card<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create layout
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
        ]).split(area);

        // Render title
        let title = self.title.bold();
        buf.set_string(chunks[0].x, chunks[0].y, title, Style::new().bold());

        // Render content with border
        self.content
            .block(Block::bordered())
            .render(chunks[1], buf);
    }
}
```

## References

- [ratatui-widgets on crates.io](https://crates.io/crates/ratatui-widgets)
- [API Documentation](https://docs.rs/ratatui-widgets)
- [Examples Directory](https://github.com/ratatui/ratatui/tree/main/examples)
