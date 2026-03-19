---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.Microsandbox/rxtui
repository: https://github.com/zerocore-ai/rxtui (mirror of https://github.com/microsandbox/rxtui)
explored_at: 2026-03-19
language: Rust
---

# RxTUI - Reactive Terminal User Interface Framework

## Overview

RxTUI is a reactive terminal user interface framework for Rust that brings modern component-based architecture to terminal applications. The framework combines React-like declarative UI patterns with Elm-inspired message-based state management, all rendered efficiently to the terminal through a virtual DOM with diffing algorithms.

The project is currently at version 0.1.8 and is in active development. It provides a comprehensive solution for building interactive, stateful terminal applications with features like:

- **Declarative UI** - Describe what your UI should look like using the `node!` macro
- **Component-based architecture** - Build reusable, composable UI components
- **Virtual DOM with diffing** - Efficient, minimal terminal updates through smart change detection
- **Message-based state management** - Predictable state updates similar to Elm architecture
- **Async effects system** - Handle background tasks, timers, and I/O operations via Tokio
- **Dual rendering modes** - Full-screen alternate screen mode and inline mode for CLI tools
- **Rich styling** - Colors, borders, text styles, and flexible layout control
- **Built-in components** - TextInput, ShimmerText, Spinner, and form components

## Repository

**Remote:** `git@github.com:zerocore-ai/rxtui`
**Main Branch:** `main`
**Current Version:** 0.1.8
**License:** Apache-2.0

**Recent Commits:**
- `ddd15be` chore(release): bump version to 0.1.8 (#53)
- `08829c6` feat(app): add inline rendering mode for CLI tools (#52)
- `062f4e7` fix(layout): resolve percentage widths before intrinsic size calculation (#50)
- `8e43a69` feat(div): add focus border styling support (#49)
- `d211a6e` chore(release): bump version to 0.1.7 (#48)

**Active Branches:** 8 feature branches including appcypher/* branches for async effects, enhanced macros, and release preparation.

## Directory Structure

```
rxtui/
├── Cargo.toml              # Workspace root configuration
├── API_REFERENCE.md        # Detailed API documentation
├── CONTRIBUTING.md         # Contribution guidelines
├── DEVELOPMENT.md          # Development environment setup
├── DOCS.md                 # Comprehensive framework documentation
├── IMPLEMENTATION.md       # Internal architecture details
├── LICENSE                 # Apache-2.0 license
├── plan.md                 # Feature planning and design docs
├── QUICK_REFERENCE.md      # Cheat sheet for common patterns
├── README.md               # Project overview and quick start
├── TUTORIAL.md             # Step-by-step learning guide
├── .gitignore
├── .pre-commit-config.yaml # Pre-commit hooks configuration
│
├── examples/               # Example applications demonstrating features
│   ├── README.md
│   ├── counter.rs          # Basic counter with keyboard handling
│   ├── textinput.rs        # Text input component demo
│   ├── form.rs             # Form handling example
│   ├── demo.rs             # Full feature demonstration
│   ├── demo_pages/         # Multi-page demo application
│   │   ├── mod.rs
│   │   ├── page1_overflow.rs
│   │   ├── page2_direction.rs
│   │   ├── page3_percentages.rs
│   │   ├── page4_borders.rs
│   │   ├── page5_absolute.rs
│   │   ├── page6_text_styles.rs
│   │   ├── page7_auto_sizing.rs
│   │   ├── page8_text_wrap.rs
│   │   ├── page9_element_wrap.rs
│   │   ├── page10_unicode.rs
│   │   ├── page11_content_sizing.rs
│   │   ├── page12_focus.rs
│   │   ├── page13_rich_text.rs
│   │   ├── page14_text_input.rs
│   │   ├── page15_scrollable.rs
│   │   └── page16_text_alignment.rs
│   ├── align.rs            # Alignment demonstrations
│   ├── components.rs       # Component composition examples
│   ├── gap.rs              # Gap/spacing examples
│   ├── hover.rs            # Hover state handling
│   ├── inline.rs           # Inline rendering mode demo
│   ├── progressbar.rs      # Progress bar component
│   ├── rxtui.rs            # Main rxtui example
│   ├── scroll.rs           # Scrolling implementation
│   ├── scroll_nested.rs    # Nested scrollable areas
│   ├── shimmer_text.rs     # Animated shimmer text effect
│   ├── spinner.rs          # Loading spinner component
│   ├── spinner_custom.rs   # Custom spinner implementation
│   └── stopwatch.rs        # Async timer with effects
│
├── rxtui/                  # Main library crate
│   ├── Cargo.toml
│   ├── README.md
│   ├── LICENSE
│   └── lib/                # Core library source (~15,000+ LOC)
│       ├── lib.rs          # Library root, module declarations, exports
│       ├── prelude.rs      # Common imports for convenient usage
│       │
│       ├── app/            # Application framework
│       │   ├── mod.rs
│       │   ├── core.rs     # Main App struct and event loop
│       │   ├── config.rs   # Terminal mode configuration (AlternateScreen/Inline)
│       │   ├── context.rs  # Context for component communication
│       │   ├── events.rs   # Keyboard and mouse event handling
│       │   ├── inline.rs   # Inline rendering state and algorithms
│       │   └── renderer.rs # Rendering pipeline
│       │
│       ├── component.rs    # Component trait and State/Action types
│       ├── vnode.rs        # Virtual Node enum (post-component expansion)
│       │
│       ├── node/           # Node types for building UI trees
│       │   ├── mod.rs      # Node enum with Component/Div/Text/RichText
│       │   ├── div.rs      # Div container with styles and events
│       │   ├── text.rs     # Plain text node
│       │   └── rich_text.rs# Multi-span styled text
│       │
│       ├── vdom.rs         # Virtual DOM implementation with diffing
│       ├── diff.rs         # Diffing algorithm generating Patch operations
│       │
│       ├── render_tree/    # Rendering engine
│       │   ├── mod.rs      # RenderTree management
│       │   ├── node.rs     # RenderNode with positioning (~2,371 LOC)
│       │   ├── tree.rs     # Tree structure and layout algorithms
│       │   └── tests/      # Layout, sizing, wrapping, rich_text tests
│       │
│       ├── buffer.rs       # Double buffering for flicker-free rendering
│       ├── terminal.rs     # Optimized terminal renderer
│       │
│       ├── style.rs        # Styling system (colors, spacing, borders)
│       ├── bounds.rs       # Rectangle and bounds operations
│       │
│       ├── effect/         # Async effects system
│       │   ├── mod.rs      # Effect type and module exports
│       │   ├── types.rs    # Effect trait definitions
│       │   └── runtime.rs  # Effect runtime with Tokio
│       │
│       ├── components/     # Built-in UI components
│       │   ├── mod.rs
│       │   ├── text_input.rs   # TextInput component
│       │   ├── shimmer_text.rs # Animated shimmer effect
│       │   └── spinner.rs      # Loading spinner
│       │
│       ├── macros/         # Macro-based DSL implementation
│       │   ├── mod.rs      # node! macro documentation
│       │   ├── node.rs     # Main macro implementation (~3,000 LOC)
│       │   └── internal.rs # Internal macro helpers
│       │
│       ├── key.rs          # Key and KeyWithModifiers types
│       ├── providers.rs    # Provider traits for Component macro
│       ├── utils.rs        # Terminal rendering utilities, Unicode width
│       │
│       └── tests/          # Library tests
│           └── rich_text_tests.rs
│
├── rxtui-macros/           # Procedural macro crate
│   ├── Cargo.toml
│   ├── README.md
│   ├── LICENSE
│   └── src/
│       └── lib.rs          # Component, update, view, effect derive macros
│
└── tests/
    └── macro_tests.rs      # Macro compilation tests
```

## Architecture

RxTUI follows a layered architecture inspired by React and Elm, adapted for terminal rendering:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              APPLICATION LAYER                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │  App (core.rs) - Event loop, lifecycle management, terminal I/O         │ │
│  │  - Polls for keyboard/mouse/resize events                               │ │
│  │  - Manages Context for component communication                          │ │
│  │  - Coordinates VDom updates and terminal drawing                        │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                             COMPONENT LAYER                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │   Component  │  │   Context    │  │    State     │  │     Action      │  │
│  │    (trait)   │◀─┤              │◀─┤   (Clone +   │◀─┤  (Update/Exit/  │  │
│  │              │  │  - Messages  │  │    Any +     │  │     None)       │  │
│  │  - update()  │  │  - Handlers  │  │    Send)     │  │                 │  │
│  │  - view()    │  │  - Topics    │  │              │  │                 │  │
│  │  - effects() │  │  - States    │  │              │  │                 │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────────┘  │
│                                                                              │
│  Procedural Macros (rxtui-macros/):                                         │
│  - #[derive(Component)] - Implements Component trait                        │
│  - #[update] - Auto-downcasts messages and state                            │
│  - #[view] - Auto-fetches state for rendering                               │
│  - #[effect] - Marks async methods for effect collection                    │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              UI TREE LAYER                                   │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  node! Macro (macros/node.rs) - Declarative UI builder                 │  │
│  │  - Parses div(props) [children] syntax                                  │  │
│  │  - Handles text, richtext, input, spacer elements                       │  │
│  │  - Applies properties: bg, dir, pad, w, h, border, etc.                 │  │
│  │  - Attaches event handlers: @click, @key, @char, @focus, @blur          │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                    │                                          │
│                                    ▼                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │     Node     │  │     Div      │  │     Text     │  │    RichText     │  │
│  │   (enum)     │  │  (generic)   │  │   (struct)   │  │   (struct)      │  │
│  │              │  │              │  │              │  │                 │  │
│  │  -Component  │  │  -styles     │  │  -content    │  │  -spans         │  │
│  │  -Div        │  │  -events     │  │  -style      │  │  -style         │  │
│  │  -Text       │  │  -children   │  │              │  │                 │  │
│  │  -RichText   │  │  -focusable  │  │              │  │                 │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            VIRTUAL DOM LAYER                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                       │
│  │     VDom     │  │    VNode     │  │    diff.rs   │                       │
│  │  (vdom.rs)   │  │  (vnode.rs)  │  │              │                       │
│  │              │  │              │  │  Generates   │                       │
│  │  -render()   │  │  -Div        │  │  Patches:    │                       │
│  │  -layout()   │  │  -Text       │  │  -Replace    │                       │
│  │  -apply_     │  │  -RichText   │  │  -UpdateText │                       │
│  │    patches() │  │              │  │  -UpdateProps│                       │
│  │              │  │              │  │  -AddChild   │                       │
│  │              │  │              │  │  -RemoveChild│                       │
│  └──────────────┘  └──────────────┘  └──────────────┘                       │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           RENDERING LAYER                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ RenderTree   │  │ RenderNode   │  │   buffer.rs  │  │   terminal.rs   │  │
│  │  (tree.rs)   │  │  (node.rs)   │  │              │  │                 │  │
│  │              │  │              │  │  Double      │  │  Optimized      │  │
│  │  -Layout     │  │  -Positioned │  │  Buffer:     │  │  escape         │  │
│  │  calculation │  │  -Dirty      │  │  -Front/Back │  │  sequences      │  │
│  │  -Z-sorting  │  │    tracking  │  │  -Cell-level │  │  -Cursor move   │  │
│  │  -Focus      │  │  -State-based│  │    diffing   │  │  -Color set     │  │
│  │    management│  │    styling   │  │              │  │  -Print char    │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────────┘  │
│                                                                              │
│  Terminal Modes:                                                             │
│  - AlternateScreen: Full-screen, alternate buffer                            │
│  - Inline: In-terminal, content persists after exit                          │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            EFFECTS LAYER                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                       │
│  │    Effect    │  │  Effect      │  │    Tokio     │                       │
│  │  (types.rs)  │  │  Runtime     │  │  Runtime     │                       │
│  │              │  │(runtime.rs)  │  │              │                       │
│  │  Pin<Box<dyn │  │  -spawn()    │  │  -time::     │                       │
│  │  Future +    │  │  -cleanup()  │  │    sleep     │                       │
│  │  Send>>      │  │  -Tracker:   │  │  -sync::     │                       │
│  │              │  │    Component │  │    Mutex     │                       │
│  │              │  │    lifecycle │  │              │                       │
│  └──────────────┘  └──────────────┘  └──────────────┘                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Data Flow Diagram

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Event     │────▶│   Context   │────▶│  Component  │
│  (keyboard, │     │  (message   │     │   update()  │
│   mouse,    │     │  routing)   │     │             │
│   resize)   │     │             │     │             │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                                               ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Terminal   │◀────│   Buffer    │◀────│    VDom     │
│   Render    │     │   (diff)    │     │   render()  │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
                                               ▼
                                         ┌─────────────┐
                                         │  Component  │
                                         │    view()   │
                                         │             │
                                         └─────────────┘
```

## Component Breakdown

### Core Modules

| Module | File | Lines | Purpose |
|--------|------|-------|---------|
| `app/core.rs` | `rxtui/lib/app/core.rs` | ~930 | Main `App` struct, event loop, component tree expansion |
| `app/context.rs` | `rxtui/lib/app/context.rs` | Context for messages, handlers, state, focus |
| `app/inline.rs` | `rxtui/lib/app/inline.rs` | Inline rendering state, space reservation algorithm |
| `component.rs` | `rxtui/lib/component.rs` | Component trait, Action/State/Message types |
| `vdom.rs` | `rxtui/lib/vdom.rs` | Virtual DOM, render tree creation, patch application |
| `diff.rs` | `rxtui/lib/diff.rs` | Diffing algorithm, Patch enum generation |
| `node/mod.rs` | `rxtui/lib/node/mod.rs` | Node enum (Component/Div/Text/RichText) |
| `node/div.rs` | `rxtui/lib/node/div.rs` | Generic Div with styles, events, children |
| `vnode.rs` | `rxtui/lib/vnode.rs` | VNode enum (Div/Text/RichText, post-expansion) |
| `style.rs` | `rxtui/lib/style.rs` | ~1,380 | Style, TextStyle, Color, Spacing, Border types |
| `render_tree/node.rs` | `rxtui/lib/render_tree/node.rs` | ~2,370 | RenderNode with layout, positioning, dirty tracking |
| `render_tree/tree.rs` | `rxtui/lib/render_tree/tree.rs` | ~750 | RenderTree management, layout calculation |
| `buffer.rs` | `rxtui/lib/buffer.rs` | DoubleBuffer, ScreenBuffer, cell diffing |
| `terminal.rs` | `rxtui/lib/terminal.rs` | TerminalRenderer, escape sequence optimization |
| `effect/mod.rs` | `rxtui/lib/effect/mod.rs` | Effect system exports |
| `effect/runtime.rs` | `rxtui/lib/effect/runtime.rs` | EffectRuntime with Tokio, lifecycle tracking |
| `macros/node.rs` | `rxtui/lib/macros/node.rs` | ~3,000 | node! macro and helper macros |
| `components/text_input.rs` | `rxtui/lib/components/text_input.rs` | TextInput component |
| `components/spinner.rs` | `rxtui/lib/components/spinner.rs` | Animated spinner component |
| `components/shimmer_text.rs` | `rxtui/lib/components/shimmer_text.rs` | Shimmer animation effect |

### Key Types

**Component System:**
- `Component` trait - Core trait with `update()`, `view()`, `effects()` methods
- `Action` enum - Return type from update: `Update`, `UpdateTopic`, `None`, `Exit`
- `State` trait - Auto-implemented for `Clone + Send + Sync + 'static` types
- `Message` trait - Auto-implemented for `Clone + Send + Sync + 'static` types
- `Context` - Provides handlers, state access, message sending, focus management

**Node Types:**
- `Node` enum - `Component`, `Div`, `Text`, `RichText`
- `VNode` enum - Post-component-expansion: `Div`, `Text`, `RichText`
- `Div<T>` - Generic container with styles, events, children, focus/hover state
- `Text` - Plain text with content and TextStyle
- `RichText` - Multiple TextSpan segments with optional top-level style

**Styling:**
- `Style` struct - background, direction, padding, border, position, z-index, gap, etc.
- `TextStyle` struct - color, background, bold, italic, underline, strikethrough, wrap, align
- `Color` enum - 16 named colors + RGB
- `Dimension` enum - Fixed, Percentage, Auto, Content
- `Border` struct - enabled, style (Single/Double/Thick/Rounded/Dashed), color, edges

**Rendering:**
- `VDom` - Virtual DOM manager
- `RenderTree` - Tree of positioned RenderNodes
- `RenderNode` - Positioned node with dirty tracking
- `DoubleBuffer` - Front/back buffers with cell-level diffing
- `TerminalRenderer` - Optimized escape sequence generation

**Effects:**
- `Effect` type - `Pin<Box<dyn Future<Output = ()> + Send>>`
- `EffectRuntime` - Tokio-based runtime with component lifecycle tracking

## Entry Points

### Library Entry Point

**File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.Microsandbox/rxtui/rxtui/lib/lib.rs`

The library root exports:
- `App`, `Context` from `app` module
- `Component`, `Action`, `State`, `Message` from `component` module
- `Node`, `Div`, `Text`, `RichText` from `node` module
- `Style`, `Color`, `TextStyle`, etc. from `style` module
- Procedural macros: `#[derive(Component)]`, `#[update]`, `#[view]`, `#[effect]`

### Application Entry Point

Users create applications by:

1. Defining a component with `#[derive(Component)]`
2. Implementing `update()` and `view()` methods using attribute macros
3. Optionally defining `#[effect]` async methods
4. Running with `App::new()?.run(Component)` or `App::inline()?.run(Component)`

**Example:**
```rust
use rxtui::prelude::*;

#[derive(Component)]
struct Counter;

impl Counter {
    #[update]
    fn update(&self, _ctx: &Context, msg: &str, mut count: i32) -> Action {
        match msg {
            "inc" => Action::update(count + 1),
            "dec" => Action::update(count - 1),
            _ => Action::exit(),
        }
    }

    #[view]
    fn view(&self, ctx: &Context, count: i32) -> Node {
        node! {
            div(
                pad: 2,
                align: center,
                w_frac: 1.0,
                gap: 1,
                @key(up): ctx.handler("inc"),
                @key(down): ctx.handler("dec"),
                @key(esc): ctx.handler("exit")
            ) [
                text(format!("Count: {count}"), color: white, bold),
                text("use ↑/↓ to change, esc to exit", color: bright_black)
            ]
        }
    }
}

fn main() -> std::io::Result<()> {
    App::new()?.run(Counter)
}
```

## Data Flow

### 1. Application Startup

```
main() → App::new() → terminal setup → run_loop(Component)
```

### 2. Event Loop

```
┌────────────────────────────────────────────────────────────┐
│                    Event Loop Iteration                     │
├────────────────────────────────────────────────────────────┤
│ 1. Expand component tree to VNode                          │
│    - Process pending messages                              │
│    - Call component.update() → Action                      │
│    - Update state based on Action                          │
│    - Call component.view() → Node                          │
│    - Expand Node to VNode (recursive)                      │
│                                                            │
│ 2. Render VNode to RenderTree                              │
│    - VDom.render(vnode) → diff → patches                   │
│    - Apply patches to RenderTree                           │
│    - VDom.layout(width, height)                            │
│                                                            │
│ 3. Draw to Terminal                                        │
│    - RenderTree → DoubleBuffer                             │
│    - DoubleBuffer.diff() → cell updates                    │
│    - TerminalRenderer.apply_updates()                      │
│                                                            │
│ 4. Handle Events                                           │
│    - Poll for keyboard/mouse/resize                        │
│    - Route events via handlers                             │
│    - Set needs_render = true                               │
└────────────────────────────────────────────────────────────┘
```

### 3. Message Flow

```
Event (key/mouse) → handler closure → Context.send(msg)
                  → Message queue → Component.update(msg, state)
                  → Action → State update → Re-render
```

### 4. Effect Lifecycle

```
Component mounts → effects() called → EffectRuntime.spawn(id, effects)
                → Effects run concurrently via Tokio
                → Effects send messages via ctx.send()
Component unmounts → EffectRuntime.cleanup(id) → Effects cancelled
```

## External Dependencies

### Workspace Dependencies (Cargo.toml)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `rxtui-macros` | path | Procedural macros for Component derive |
| `crossterm` | 0.28 | Terminal I/O, events, cursor control |
| `tokio` | 1.x | Async runtime for effects (optional) |
| `futures` | 0.3 | Future utilities (optional) |
| `serde` | 1.0 | Serialization (workspace dep) |
| `thiserror` | 2.0 | Error handling (workspace dep) |
| `bitflags` | 2.4 | BorderEdges bitflags |
| `unicode-width` | 0.2 | Character width calculations |
| `syn` | 2.0 | Macro parsing |
| `quote` | 1.0 | Macro code generation |
| `proc-macro2` | 1.0 | Procedural macro support |

### Key External Integrations

**crossterm** - Terminal backend:
- `terminal::enable_raw_mode()` / `disable_raw_mode()`
- `terminal::EnterAlternateScreen` / `LeaveAlternateScreen`
- `cursor::MoveTo`, `cursor::Hide`, `cursor::Show`
- `event::EnableMouseCapture`, `event::poll()`, `event::read()`
- `style::{SetForegroundColor, SetBackgroundColor, Print, ResetColor}`

**tokio** - Async runtime:
- `tokio::time::sleep()` for timers
- `tokio::sync::Mutex` for shared state
- Multi-threaded runtime for effect execution

## Configuration

### Features

| Feature | Default | Description |
|---------|---------|-------------|
| `effects` | Yes | Enable async effects system (requires Tokio) |
| `components` | Yes | Enable built-in components (TextInput, etc.) |

### Terminal Modes

**AlternateScreen (default):**
```rust
App::new()?.run(Component)
```

**Inline mode:**
```rust
App::inline()?.run(Component)
```

**Inline with config:**
```rust
let config = InlineConfig {
    height: InlineHeight::Content { max: Some(24) },
    cursor_visible: false,
    preserve_on_exit: true,
    mouse_capture: false,
};
App::inline_with_config(config)?.run(Component)
```

### Render Configuration

```rust
app.render_config(RenderConfig {
    double_buffering: true,      // Use double buffer
    cell_diffing: true,          // Cell-level diff optimization
    terminal_optimizations: true, // Optimized escape sequences
    poll_duration_ms: 100,       // Event poll timeout
})
```

Debug mode:
```rust
app.disable_all_optimizations() // For debugging
```

## Testing

### Test Structure

Tests are organized in:
- `rxtui/lib/tests/` - Library unit tests
- `rxtui/lib/render_tree/tests/` - Layout and rendering tests
- `rxtui/tests/` - Integration tests
- `rxtui-macros/tests/` - Macro tests (via macro_tests.rs)

### Test Categories

**Rich Text Tests** (`tests/rich_text_tests.rs`):
- Text span rendering
- Style application
- Wrapping behavior

**Layout Tests** (`render_tree/tests/layout_tests.rs`):
- Position calculation
- Percentage sizing
- Auto-sizing behavior

**Sizing Tests** (`render_tree/tests/sizing_tests.rs`):
- Content-based sizing
- Fixed vs. percentage dimensions
- Constraint resolution

**Wrapping Tests** (`render_tree/tests/wrapping_tests.rs`):
- Text wrapping modes (Character, Word, WordBreak)
- Line breaking behavior

**Macro Tests** (`tests/macro_tests.rs`):
- Compile-time macro validation
- Syntax verification

### Running Tests

```bash
cargo test           # All tests
cargo test --lib     # Library tests only
cargo test --test macro_tests  # Macro tests
```

## Key Insights

### 1. Virtual DOM Architecture

RxTUI uses a two-phase rendering approach:
- **Phase 1:** Component tree expands to `Node` tree, then to `VNode` tree
- **Phase 2:** `VDom` diffs `VNode` against current `RenderTree`, generating minimal patches

This is more efficient than full re-renders because:
- Only changed nodes trigger updates
- Text updates use `UpdateText` patches (no node replacement)
- Property changes use `UpdateProps` patches (preserves children)

### 2. Component Tree Expansion

Components are expanded depth-first during rendering:
- Each component instance gets a unique `ComponentId` (e.g., "0.1.2")
- Messages are routed by component ID
- Effects are spawned on mount and cleaned up on unmount
- State is stored in `Context.states` map keyed by component ID

### 3. Topic-Based Communication

Components can communicate via topics:
- `Action::UpdateTopic(topic_name, state)` updates another component's state
- Topics are idempotent - first writer becomes owner
- Unassigned messages route to root; assigned messages route to owner

### 4. Inline Rendering Algorithm

The inline mode uses a clever space reservation strategy:
1. Print N newlines to reserve space (causes scroll if needed)
2. Move cursor back up N lines to establish origin
3. Render content into reserved space using absolute coordinates
4. On re-render, move to origin and overwrite

This handles terminal scrolling gracefully because the scroll already happened during reservation.

### 5. Dirty Tracking

RenderNodes track dirty state for efficient re-layout:
- Any property change marks node dirty
- Parent nodes marked dirty when children change
- Only dirty nodes are re-laid out during render

### 6. Focus System

Focus management uses a shared atomic flag:
- `VDom` and `Context` share `focus_clear_flag`
- Focus requests queued during render
- Flag prevents clearing focus when new focus is set same cycle

### 7. Macro Design

The `node!` macro uses recursive token-tree parsing:
- `tui_parse_element!` - Parses individual elements
- `tui_parse_children!` - Recursively processes children
- `tui_apply_props!` - Applies properties via pattern matching
- Helper macros for colors, directions, keys, etc.

## Open Questions

1. **Key-based child diffing**: Current diff uses index-based comparison. Could adding keys improve reordering performance?

2. **Effect state consistency**: Effects access state via `ctx.get_state()` at call time. Should state be captured at effect spawn time for consistency?

3. **Focus traversal**: Current focus requires explicit `@focus` handlers. Should there be automatic Tab key traversal?

4. **Accessibility**: No explicit accessibility support (screen readers, etc.). Should this be added?

5. **Performance profiling**: No built-in performance metrics. Would benchmarking tools help optimize rendering?

6. **Theming system**: Styles are per-component. Should there be a global theme/stylesheet system?

7. **Component lifecycle**: Currently only mount/unmount via effects. Should there be explicit `on_mount`/`on_unmount` methods?

8. **Error boundaries**: No error recovery for panicking components. Should there be error boundaries?

9. **Server-side rendering**: Could the VDom be serialized for SSR-like scenarios (e.g., screenshot generation)?

10. **Animation system**: Currently only shimmer and spinner have animations. Should there be a general animation API?
