---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RustSignals/haalka
explored_at: 2026-03-23
---

# haalka - Bevy UI with FRP Signals Deep Dive

## Overview

haalka (হালকা - Bengali for "light" or "easy") is an ergonomic reactive Bevy UI library powered by FRP signals from `futures-signals` and async ECS from `bevy-async-ecs`.

**Crate Info:**
- **Version:** 0.5.1
- **Edition:** 2024
- **License:** MIT OR Apache-2.0
- **Repository:** https://github.com/databasedav/haalka

**Key Features:**
- Signals integration for all entities, components, and children
- Constant time reactive updates for collections
- Alignment semantics from MoonZoon
- Pointer event handling with hover states
- Mouse wheel scroll handling
- Text input integration
- Grid layout model

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    haalka High-Level API                     │
│  (El, Column, Row, Grid, Stack components)                   │
├─────────────────────────────────────────────────────────────┤
│                 futures-signals Integration                  │
│  (Mutable, Signal, SignalVec, SignalMap primitives)          │
├─────────────────────────────────────────────────────────────┤
│                    bevy-async-ecs Runtime                    │
│  (Async world access, command dispatch)                      │
├─────────────────────────────────────────────────────────────┤
│                        Bevy Engine                           │
│  (bevy_ui, bevy_input, bevy_render, etc.)                    │
└─────────────────────────────────────────────────────────────┘
```

## Core Abstractions

### RawHaalkaEl - The Foundation

```rust
pub struct RawHaalkaEl<State> {
    // Core element state
    // Signal subscriptions
    // Bevy entity management
}
```

All higher-level components (El, Column, Row, etc.) wrap `RawHaalkaEl`.

### Element Trait

```rust
pub trait Element {
    type State;

    fn spawn(self, world: &mut World) -> Entity;
    // ...
}
```

### El - Generic Element Wrapper

```rust
pub struct El<T: Bundle> {
    raw_el: RawHaalkaEl<T::State>,
    _marker: PhantomData<T>,
}

impl<T: Bundle> El<T> {
    pub fn new() -> Self { }

    pub fn child(mut self, child: impl Element) -> Self { }

    pub fn with_node<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut T),
    { }

    pub fn text_signal(mut self, signal: impl Signal<Item = Text>) -> Self { }
}
```

## Signal Integration

### MutableVec for Collections

```rust
use haalka::prelude::*;

let items = MutableVec::new_with_values(vec!["Item 1", "Item 2"]);

// Children update in O(1) time
Column::new()
    .children_signal_vec(items.signal_vec().map(|item| {
        El::<Node>::new()
            .child(El::<Text>::new().text(Text::new(item)))
    }))
```

**Key insight:** Unlike typical Bevy UI where you'd need to manually manage child entities, haalka automatically handles additions, removals, and reordering via SignalVec.

### Signal-driven Properties

```rust
let hovered = Mutable::new(false);
let counter = Mutable::new(0);

El::<Node>::new()
    .background_color_signal(
        hovered.signal().map_bool(
            || Color::hsl(300., 0.75, 0.85),  // hovered
            || Color::hsl(300., 0.75, 0.75),  // not hovered
        )
    )
    .text_signal(counter.signal_ref(ToString::to_string))
```

## Alignment System (from MoonZoon)

### Align Enum

```rust
pub enum Align {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

impl Align {
    pub fn horizontal() -> Self { Self::Center }
    pub fn vertical() -> Self { Self::Center }
    pub fn center() -> Self { Self::Center }
}
```

### Column with Alignment

```rust
Column::new()
    .align_content(Align::center())
    .align_items(Align::Start)
    .item(child1)
    .item(child2)
```

### Grid Layout

```rust
Grid::new()
    .columns(vec![
        GridTrack::auto(),
        GridTrack::flex(1.),
        GridTrack::auto(),
    ])
    .rows(vec![GridTrack::auto()])
    .item(content)
```

## Pointer Event Handling

### Hover States

```rust
let hovered = Mutable::new(false);

El::<Node>::new()
    .hovered_sync(hovered.clone())  // Syncs hovered state
    .on_hovered_enter(|| println!("Entered!"))
    .on_hovered_leave(|| println!("Left!"))
```

**Web-style Enter/Leave:**

```rust
pub struct Enter;
pub struct Leave;

impl PointerEventAware for El<Node> {
    fn on_pointer_enter(self, handler: impl FnMut() + 'static) -> Self;
    fn on_pointer_leave(self, handler: impl FnMut() + 'static) -> Self;
}
```

### Click Handling

```rust
El::<Node>::new()
    .on_click(move || {
        *counter.lock_mut() += 1;
    })
    .on_click_outside(move || {
        // Close dropdown, etc.
    })
```

### Cursor Management

```rust
El::<Node>::new()
    .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
    .cursor_on_hover(CursorIcon::System(SystemCursorIcon::Grab))
    .cursor_on_hover_disabled(CursorIcon::System(SystemCursorIcon::NotAllowed))
```

## Scroll Handling

### MouseWheelScrollable

```rust
El::<Node>::new()
    .mouse_wheel_scrollable(ScrollDirection::Vertical)
    .on_hover_mouse_wheel_scrollable(ScrollDirection::Both)
```

### ViewportMutable

```rust
let viewport = Mutable::new(Vec2::ZERO);

El::<Node>::new()
    .viewport_mutable(viewport.clone())
    .viewport_mutable_on_scroll(Axis::Y)
```

## Text Input Integration

```rust
use haalka::prelude::*;
use bevy_ui_text_input;

let text = Mutable::new(String::new());

TextInput::new()
    .value_signal(text.signal_cloned())
    .on_change(move |new_value| {
        *text.lock_mut() = new_value;
    })
```

## Async ECS Integration

haalka uses `bevy-async-ecs` for async world access:

```rust
pub async fn async_world() -> AsyncWorld {
    // Provided by HaalkaPlugin
}

// In a system
async_world.spawn(|world: &mut World| {
    ui_root().spawn(world);
}).await;
```

## Counter Example (Complete)

```rust
use bevy::prelude::*;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, HaalkaPlugin))
        .add_systems(Startup, (camera, setup_ui));
        .run();
}

#[derive(Component)]
struct Counter(Mutable<i32>);

fn setup_ui(world: &mut World) {
    ui_root().spawn(world);
}

fn ui_root() -> impl Element {
    let counter = Mutable::new(0);

    El::<Node>::new()
        .with_node(|mut node| {
            node.height = Val::Percent(100.);
            node.width = Val::Percent(100.);
        })
        .align_content(Align::center())
        .child(
            Row::<Node>::new()
                .with_node(|mut node| node.column_gap = Val::Px(15.0))
                .item(counter_button(counter.clone(), "-", -1))
                .item(
                    El::<Text>::new()
                        .text_font(TextFont::from_font_size(25.))
                        .text_signal(counter.signal_ref(ToString::to_string).map(Text)),
                )
                .item(counter_button(counter.clone(), "+", 1))
                .update_raw_el(move |raw_el| raw_el.insert(Counter(counter)))
        )
}

fn counter_button(counter: Mutable<i32>, label: &str, step: i32) -> impl Element {
    let hovered = Mutable::new(false);

    El::<Node>::new()
        .with_node(|mut node| node.width = Val::Px(45.0))
        .align_content(Align::center())
        .border_radius(BorderRadius::MAX)
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
        .background_color_signal(
            hovered.signal()
                .map_bool(
                    || Color::hsl(300., 0.75, 0.85),
                    || Color::hsl(300., 0.75, 0.75),
                )
                .map(BackgroundColor),
        )
        .hovered_sync(hovered)
        .on_click(move || *counter.lock_mut() += step)
        .child(
            El::<Text>::new()
                .text_font(TextFont::from_font_size(25.))
                .text(Text::new(label)),
        )
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
```

## Macro System (derive feature)

```rust
// Generate helper methods for custom elements
impl_haalka_methods! {
    struct MyElement {
        // Fields become configurable via signals
        value: Mutable<i32>,
        label: String,
    }
}
```

## Eventual Consistency Model

**Important:** haalka's reactivity is **eventually consistent**:

```
ECS State Change → Signal Notification → Async Command Queue → Next Frame Apply
```

This means:
- UI updates may lag by one frame
- Frame-perfect timing not guaranteed
- Acceptable for most UI use cases
- For frame-perfect, use native Bevy systems

## WASM Deployment

Examples are deployed as WASM:
- WebGL2 builds available
- WebGPU builds (check compatibility)
- GitHub Pages hosting

```bash
# Build for WASM
cargo build --target wasm32-unknown-unknown --release

# Using just/cargo-all-features
just build-web
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `default` | Enables `text_input`, `ui`, `utils` |
| `ui` | High-level UI abstractions |
| `text_input` | Text input widget |
| `derive` | Macro for custom element methods |
| `utils` | Async/signal utilities |
| `debug` | Debug UI overlay toggle (F1) |
| `webgpu` | WebGPU rendering support |

## Dependencies

```toml
[dependencies]
bevy_app = "0.16"
bevy_ecs = "0.16"
bevy_ui = "0.16"
bevy_input = "0.16"
bevy_render = "0.16"
bevy-async-ecs = "0.8"
futures-signals = "0.3"
haalka_futures_signals_ext = "0.0.3"
apply = "0.3"
cfg-if = "1.0"
enclose = "1.2"
```

## Signal Extensions

haalka includes `haalka_futures_signals_ext`:

```rust
// Additional Signal methods beyond futures-signals
trait SignalExt {
    fn map_bool<T, F, G>(self, if_true: F, if_false: G) -> MapBool<Self, F, G>;
    // ...
}
```

## Examples Gallery

| Example | Description |
|---------|-------------|
| `counter` | Basic counter with +/- buttons |
| `button` | Basic button (Bevy example port) |
| `align` | Alignment API demo |
| `scroll` | Scrollability demo |
| `scroll_grid` | Infinite scroll grid |
| `snake` | Snake game with adjustable settings |
| `calculator` | Calculator app |
| `inventory` | Inventory management UI |
| `key_values_sorted` | Text inputs + sorted lists |
| `character_editor` | Character customization UI |
