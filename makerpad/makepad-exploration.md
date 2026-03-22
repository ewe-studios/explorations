---
name: Makepad
description: Cross-platform UI toolkit with custom GPU rendering engine, hot reloading, and MPSL styling language for building applications on desktop, mobile, and web
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/makepad/
---

# Makepad - Cross-Platform UI Toolkit

## Overview

Makepad is a Rust-based UI toolkit for building truly cross-platform applications from a single codebase. It features a custom immediate-mode rendering engine, GPU-accelerated 2D/3D graphics, and a DSL for styling and layout. Unlike Electron or Tauri, Makepad doesn't use webviews - it renders everything natively using GPU shaders.

## Repository Structure

```
makepad/
├── audio_graph/                    # Audio synthesis & processing
│   ├── src/
│   │   ├── audio_graph.rs          # Audio node graph system
│   │   ├── audio_stream.rs         # Real-time audio streaming
│   │   ├── audio_traits.rs         # Audio trait definitions
│   │   ├── audio_unit_effect.rs    # Audio effect units
│   │   ├── audio_unit_instrument.rs # Instrument units
│   │   ├── instrument.rs           # Instrument trait
│   │   └── mixer.rs                # Audio mixer
│   └── audio_widgets/              # Audio UI widgets
│
├── code_editor/                    # Code editor component
│   └── src/
│       ├── code_editor.rs          # Main editor widget
│       ├── document.rs             # Document model
│       ├── tokenizer.rs            # Syntax highlighting
│       ├── layout.rs               # Text layout engine
│       └── widgets.rs              # Editor widgets
│
├── draw/                           # 2D/3D drawing engine
│   ├── src/
│   │   ├── shader/                 # GPU shaders (HLSL/GLSL)
│   │   │   ├── draw_color.rs       # Color fill shaders
│   │   │   ├── draw_line.rs        # Line drawing shaders
│   │   │   ├── draw_text.rs        # Text rendering shaders
│   │   │   ├── draw_quad.rs        # Quad rendering shaders
│   │   │   └── draw_trapezoid.rs   # Trapezoid fill shaders
│   │   ├── text/                   # Text shaping & layout
│   │   │   ├── font.rs             # Font loading & management
│   │   │   ├── font_atlas.rs       # Font atlas generation
│   │   │   └── font_face.rs        # Font face handling
│   │   └── geometry/               # Geometry generation
│   └── vector/                     # Vector graphics (forked libs)
│       ├── bender/                 # Path tessellation
│       │   ├── clipper/            # Path clipping
│       │   ├── filler/             # Path filling
│       │   ├── offsetter/          # Path offsetting
│       │   └── stroker/            # Path stroking
│
├── examples/                       # Example applications
│   ├── chatgpt/                    # ChatGPT-like UI
│   ├── ironfish/                   # Synthesizer application
│   ├── fractal_zoom/               # GPU fractal renderer
│   ├── news_feed/                  # Social feed UI
│   ├── snake/                      # Snake game
│   ├── slides/                     # Presentation software
│   ├── teamtalk/                   # Video conferencing
│   ├── text_flow/                  # Text layout demo
│   ├── ui_zoo/                     # Widget showcase
│   ├── web_cam/                    # Webcam capture
│   └── websocket_image/            # WebSocket streaming
│
├── libs/                           # Vendored libraries
│   ├── html/                       # HTML parsing
│   ├── rustybuzz/                  # Text shaping (fork)
│   ├── ttf-parser/                 # Font parsing (fork)
│   ├── ab_glyph_rasterizer/        # Glyph rasterization
│   ├── sdfer/                      # Signed distance field rendering
│   ├── zune-*                      # Image decoding
│   │   ├── zune-core/
│   │   ├── zune-inflate/
│   │   ├── zune-jpeg/
│   │   └── zune-png/
│   └── stitch/                     # Hot reloading runtime
│
├── platform/                       # Platform abstraction layer
│   └── src/
│       ├── android/                # Android backend (OpenGL ES)
│       ├── apple/                  # iOS/macOS backend (Metal)
│       ├── web/                    # Web/WASM backend (WebGL)
│       ├── windows/                # Windows backend (DirectX/OpenGL)
│       ├── linux/                  # Linux backend (OpenGL)
│       └── turbo/                  # High-performance primitives
│
├── studio/                         # Makepad Studio IDE
│   └── src/
│       ├── main.rs                 # IDE entry point
│       ├── editor/                 # Code editor
│       ├── preview/                # Live preview
│       └── debugger/               # Debug tools
│
├── tools/                          # Build & development tools
│   ├── cargo_makepad/              # Cargo subcommand for cross-compile
│   ├── web_server/                 # Development server
│   ├── wasm_strip/                 # WASM size optimization
│   └── shader-compiler/            # Shader compilation
│
└── widgets/                        # Widget library
    └── src/
        ├── button.rs               # Button widget
        ├── label.rs                # Label widget
        ├── text_input.rs           # Text input
        ├── scroll_view.rs          # Scrollable view
        ├── tab_bar.rs              # Tab bar
        ├── slider.rs               # Slider widget
        ├── checkbox.rs             # Checkbox widget
        ├── drop_down.rs            # Dropdown menu
        ├── modal.rs                # Modal dialogs
        └── list_view.rs            # List view widget
```

## Architecture

### Layer Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    Application Layer                              │
│  Your Makepad Application (Rust code + live_design!)             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ WidgetRef, Event handling
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Widget Layer                                   │
│  ┌─────────┐ ┌──────────┐ ┌───────────┐ ┌────────────┐         │
│  │ Button  │ │  Label   │ │ TextInput │ │ ScrollView │         │
│  └─────────┘ └──────────┘ └───────────┘ └────────────┘         │
│  ┌─────────┐ ┌──────────┐ ┌───────────┐ ┌────────────┐         │
│  │ TabBar  │ │  Slider  │ │ ListView  │ │ Modal      │         │
│  └─────────┘ └──────────┘ └───────────┘ └────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ DrawList, Walk
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Draw Layer                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │    DrawQuad     │  │   DrawText      │  │   DrawPath      │  │
│  │  (rectangles)   │  │  (shaping)      │  │  (tessellation) │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  DrawLine       │  │  DrawImage      │  │  DrawMesh       │  │
│  │  (Bresenham)    │  │  (UV mapping)   │  │  (3D)           │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ ShaderItem, Uniforms
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Shader Layer                                    │
│  ┌───────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │draw_quad  │ │draw_text │ │draw_line │ │draw_path │          │
│  │(HLSL/GLSL)│ │(SDF)     │ │(AA)      │ │(Fill)    │          │
│  └───────────┘ └──────────┘ └──────────┘ └──────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Platform API
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Platform Layer                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐│
│  │ Windows  │ │  macOS   │ │  Linux   │ │ Android  │ │  WASM  ││
│  │ (Win32)  │ │ (Metal)  │ │ (OpenGL) │ │(OpenGLES)│ │(WebGL) ││
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Core Concepts

### 1. Live Design System (Hot Reloading)

```rust
// live_design! macro enables runtime UI changes
live_design! {
    import makepad::draw::*;
    import makepad::widgets::*;

    // Define app structure in MPSL (Makepad Style Language)
    App = {{App}} {
        ui: Window = {
            body: View = {
                flow: Down,
                padding: 10,
                spacing: 10,

                label = {
                    text: "Hello Makepad!"
                    draw_text: {
                        color: #000000
                        font_size: 24.0
                    }
                }

                button = {
                    text: "Click me"
                    draw_bg: {
                        color: #3498db
                        border_radius: 4.0
                    }
                    draw_text: {
                        color: #ffffff
                        font_size: 16.0
                    }
                }
            }
        }
    }
}

// Changes to live_design! blocks are hot-reloaded at runtime
// No recompilation needed for style/layout changes
```

### 2. Widget System

```rust
use makepad_widgets::*;

// Widget definition
live_design! {
    Counter = {{Counter}} {
        count_label = {
            text: "0"
        }
        increment_button = {
            text: "+"
        }
        decrement_button = {
            text: "-"
        }
    }
}

struct Counter {
    count: i32,
    count_label: WidgetRef,
    increment_button: WidgetRef,
    decrement_button: WidgetRef,
}

impl Counter {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        match event {
            Event::Click => {
                if self.increment_button.is_click(event) {
                    self.count += 1;
                    self.count_label.set_text(&self.count.to_string());
                    self.count_label.redraw(cx);
                }
                if self.decrement_button.is_click(event) {
                    self.count -= 1;
                    self.count_label.set_text(&self.count.to_string());
                    self.count_label.redraw(cx);
                }
            }
        }
    }
}
```

### 3. Immediate Mode Rendering

```rust
// Makepad uses immediate mode - UI is rebuilt every frame
impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Process events
        if let Event::Click = event {
            // Handle click
        }

        // Redraw triggered by events or animations
        self.ui.redraw(cx);
    }

    fn tick(&mut self, cx: &mut Cx) {
        // Called every frame (60fps by default)
        // Animate, update state, etc.
    }
}

// Rendering happens via DrawList
fn draw_walk(&mut self, cx: &mut Cx2d, walk: Walk) -> DrawStep {
    let rect = Rect {
        pos: dvec2(0.0, 0.0),
        size: dvec2(100.0, 50.0),
    };

    // Draw background
    self.draw_bg.draw_all(cx, rect);

    // Draw text
    self.draw_text.draw_text(cx, "Hello", rect.pos);

    DrawStep::done()
}
```

### 4. Event System

```rust
// Event types
pub enum Event {
    Click,
    TouchStart(Touch),
    TouchMove(Touch),
    TouchEnd(Touch),
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    Scroll(ScrollEvent),
    Focus,
    Blur,
    Window(WindowEvent),
    Audio(AudioEvent),
    Network(NetworkEvent),
    Custom(CustomEvent),
}

// Event handling pattern
impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Match on event type
        match event {
            // Window events
            Event::Window(WindowEvent::MouseDown(e)) => {
                if e.button == MouseButton::Left {
                    // Handle left click
                }
            }

            // Keyboard events
            Event::KeyDown(ke) => {
                match ke.key {
                    KeyCode::Enter => {
                        // Handle enter key
                    }
                    KeyCode::Escape => {
                        // Handle escape
                    }
                    _ => {}
                }
            }

            // Touch events (mobile)
            Event::TouchStart(touch) => {
                // Handle touch start
            }

            // Custom events
            Event::Custom(custom) => {
                match custom.as_any().downcast_ref::<MyCustomData>() {
                    Some(data) => {
                        // Handle custom data
                    }
                    None => {}
                }
            }

            _ => {}
        }
    }
}
```

## Drawing Engine

### GPU Shaders

```rust
// draw_quad.rs - Quad rendering shader
// Vertex shader (HLSL/GLSL)
#[derive(Clone, Default, Pod)]
#[repr(C)]
pub struct DrawQuadInstance {
    pub rect: Vec4,      // x, y, width, height
    pub color: Vec4,     // r, g, b, a
    pub border_radius: f32,
}

pub fn draw_quad_vs(vertex: &Vertex2d, instance: &DrawQuadInstance) -> Varyings {
    let rect = instance.rect;
    let pos = vec2(
        vertex.pos.x * rect.z + rect.x,
        vertex.pos.y * rect.w + rect.y
    );

    return Varyings {
        uv: vertex.uv,
        color: instance.color,
        border_radius: instance.border_radius,
        pos: pos,
    };
}

pub fn draw_quad_fs(varyings: &Varyings) -> Vec4 {
    // Rounded rectangle with anti-aliasing
    let d = distance_to_rect(varyings.pos, varyings.border_radius);
    let alpha = smoothstep(0.0, 1.0, -d);

    return vec4(varyings.color.rgb, varyings.color.a * alpha);
}
```

### Text Rendering

```rust
// Text shaping with rustybuzz
use rustybuzz::{Buffer, UnicodeBuffer};

pub struct Font {
    face: Face,
    atlas: FontAtlas,
}

impl Font {
    pub fn shape(&self, text: &str) -> ShapedText {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);

        let shaped = shape(&self.face, &mut buffer);

        ShapedText {
            glyphs: shaped.glyph_infos().iter().map(|g| g.codepoint).collect(),
            positions: shaped.glyph_positions().iter().map(|p| (p.x_offset, p.y_offset)).collect(),
            advance: shaped.glyph_infos().iter().map(|g| g.glyph_id).collect(),
        }
    }

    pub fn rasterize(&self, glyph: GlyphId) -> GlyphBitmap {
        self.atlas.get(glyph)
    }
}

// SDF-based text rendering shader
pub fn draw_text_fs(uv: Vec2, sdf_texture: sampler2D) -> Vec4 {
    let distance = texture(sdf_texture, uv).r;
    let alpha = smoothstep(0.5 - 0.5, 0.5 + 0.5, distance);
    return vec4(1.0, 1.0, 1.0, alpha);
}
```

### Path Tessellation

```rust
// Vector path tessellation using bender
use bender::{Path, Builder};

pub struct DrawPath {
    path: Path,
    tessellator: Tessellator,
}

impl DrawPath {
    pub fn begin_path(&mut self) {
        self.path = Path::new();
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(x, y);
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(x, y);
    }

    pub fn curve_to(&mut self, cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32) {
        self.path.cubic_to(cp1x, cp1y, cp2x, cp2y, x, y);
    }

    pub fn close_path(&mut self) {
        self.path.close();
    }

    pub fn fill(&mut self, color: Vec4) -> Mesh {
        self.tessellator.tessellate(&self.path, FillRule::EvenOdd)
    }

    pub fn stroke(&mut self, width: f32, color: Vec4) -> Mesh {
        self.tessellator.stroke(&self.path, width)
    }
}
```

## Platform Abstraction

### Platform Trait

```rust
// platform/src/lib.rs
pub trait Platform: Sized {
    type Window;
    type Context;
    type Surface;

    fn create_window(title: &str, width: u32, height: u32) -> Self::Window;
    fn create_context(window: &Self::Window) -> Self::Context;
    fn make_current(context: &Self::Context);
    fn swap_buffers(context: &Self::Context);

    fn pump_events<F>(&mut self, callback: F)
    where
        F: FnMut(&Event);

    fn now() -> f64;
    fn sleep_ms(ms: u32);
}

// Windows implementation
#[cfg(target_os = "windows")]
mod windows {
    pub struct WindowsPlatform;

    impl Platform for WindowsPlatform {
        type Window = Win32Window;
        type Context = GlContext;
        type Surface = GlSurface;

        fn create_window(title: &str, width: u32, height: u32) -> Self::Window {
            // Win32 API window creation
        }

        fn create_context(window: &Self::Window) -> Self::Context {
            // OpenGL context creation
        }

        // ... rest of implementation
    }
}

// macOS implementation (Metal)
#[cfg(target_os = "macos")]
mod apple {
    pub struct ApplePlatform;

    impl Platform for ApplePlatform {
        type Window = MetalWindow;
        type Context = MetalContext;
        type Surface = MetalSurface;

        fn create_window(title: &str, width: u32, height: u32) -> Self::Window {
            // Cocoa/Metal window creation
        }

        fn create_context(window: &Self::Window) -> Self::Context {
            // Metal device creation
        }

        // ... rest of implementation
    }
}
```

### Web/WASM Platform

```rust
// platform/src/web.rs
use wasm_bindgen::prelude::*;
use web_sys::{Window, CanvasRenderingContext2d, WebGlContext};

pub struct WebPlatform {
    window: Window,
    canvas: HtmlCanvasElement,
    context: WebGlContext,
    animation_frame: Option<Closure<dyn FnMut()>>,
}

impl Platform for WebPlatform {
    fn pump_events<F>(&mut self, mut callback: F)
    where
        F: FnMut(&Event),
    {
        // Set up event listeners
        let closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            callback(&Event::Click);
        }) as Box<dyn FnMut(_)>);

        self.canvas.add_event_listener_with_callback(
            "click",
            closure.as_ref().unchecked_ref()
        ).unwrap();

        closure.forget();

        // Animation frame loop
        let frame_closure = Closure::wrap(Box::new(move || {
            callback(&Event::Frame);
        }) as Box<dyn FnMut()>);

        self.window.request_animation_frame(
            frame_closure.as_ref().unchecked_ref()
        ).unwrap();

        self.animation_frame = Some(frame_closure);
    }
}
```

## Audio System

```rust
// audio_graph/src/lib.rs
pub trait AudioProcessor: Send + 'static {
    fn process(&mut self, buffer: &mut AudioBuffer);
    fn sample_rate(&self) -> f32;
    fn set_sample_rate(&mut self, rate: f32);
}

pub struct AudioGraph {
    nodes: Vec<Box<dyn AudioProcessor>>,
    sample_rate: f32,
    buffer_size: usize,
}

impl AudioGraph {
    pub fn add_node(&mut self, node: Box<dyn AudioProcessor>) {
        self.nodes.push(node);
    }

    pub fn process(&mut self, output: &mut AudioBuffer) {
        // Process audio through all nodes
        for node in &mut self.nodes {
            node.process(output);
        }
    }
}

// Oscillator node
pub struct Oscillator {
    frequency: f32,
    phase: f32,
    waveform: Waveform,
}

pub enum Waveform {
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

impl AudioProcessor for Oscillator {
    fn process(&mut self, buffer: &mut AudioBuffer) {
        let sample_rate = buffer.sample_rate();
        let phase_increment = self.frequency / sample_rate;

        for sample in &mut buffer.samples {
            *sample = match self.waveform {
                Waveform::Sine => (self.phase * 2.0 * std::f32::consts::PI).sin(),
                Waveform::Square => if self.phase < 0.5 { 1.0 } else { -1.0 },
                Waveform::Sawtooth => 2.0 * self.phase - 1.0,
                Waveform::Triangle => 2.0 * (2.0 * self.phase - 1.0).abs() - 1.0,
            };

            self.phase = (self.phase + phase_increment) % 1.0;
        }
    }
}
```

## Build System

### Cargo.toml Configuration

```toml
[package]
name = "my-makepad-app"
version = "0.1.0"
edition = "2021"

[dependencies]
makepad-widgets = { git = "https://github.com/makepad/makepad", branch = "main" }

# Optimize for WASM size
[profile.small]
inherits = "release"
opt-level = 'z'
lto = true
codegen-units = 1
panic = 'abort'
strip = true
```

### Build Commands

```bash
# Desktop (native)
cargo run --release

# Web (WASM)
cargo makepad wasm run --release

# iOS Simulator
cargo makepad apple ios run-sim --release

# Android
cargo makepad android run --release

# Build with small profile (WASM)
cargo build --profile=small --target wasm32-unknown-unknown
```

## Performance Considerations

### WASM Size Optimization

```bash
# Before optimization: ~3MB
# After optimization: ~600KB gzipped

# Techniques:
# 1. Use profile.small (see above)
# 2. Strip symbols: wasm-strip target/wasm32-unknown-unknown/small/myapp.wasm
# 3. Use wasm-opt: wasm-opt -Oz myapp.wasm -o myapp.opt.wasm
# 4. Enable LTO in Cargo.toml
```

### Rendering Optimization

```rust
// Only redraw when needed
impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Don't redraw on every event
        match event {
            Event::Click => {
                self.update_state();
                self.ui.redraw(cx);  // Explicit redraw
            }
            Event::Scroll(_) => {
                // Scroll often doesn't need full redraw
                self.ui.redraw(cx);
            }
            _ => {}
        }
    }
}

// Use lazy layouts
live_design! {
    // Lazy evaluation - only visible items rendered
    list_view = {
        lazy: true
        item_size: 50.0
    }
}
```

## Summary

Makepad provides:
- **True cross-platform** - Single Rust codebase for all platforms
- **GPU-accelerated rendering** - Custom shaders for 2D/3D graphics
- **Hot reloading** - Live editing of UI without recompilation
- **No webview** - Native rendering, not wrapped HTML/CSS
- **Audio support** - Built-in audio graph for music/sound apps
- **Text shaping** - Advanced typography with rustybuzz
- **WASM optimized** - Small bundle sizes for web deployment
