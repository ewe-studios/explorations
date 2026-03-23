---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/makerpad/
explored_at: 2026-03-22
revised_at: 2026-03-23
workspace: makerpad-rust-workspace
---

# Rust Revision: Makepad & Project Robius (Full Ecosystem)

## Overview

This document provides guidance for building UI applications in Rust using Makepad-style patterns, including custom GPU rendering, reactive state management, cross-platform deployment, Wasm interpretation, serialization, shader parsing, platform abstraction libraries, and AI/ML integration. Updated to cover ALL sub-projects in the Makepad ecosystem.

## Workspace Structure

```
makerpad-rust-workspace/
├── Cargo.toml                         # Workspace definition
├── crates/
│   ├── ui-toolkit/                    # UI toolkit (Makepad-like)
│   ├── render-engine/                 # GPU rendering engine
│   ├── platform-abstraction/          # Platform layer
│   ├── reactive-state/                # Observable state (eyeball-like)
│   ├── live-design/                   # Hot reload system
│   ├── mpsl-parser/                   # Style language parser (GLSL/MPSL)
│   ├── audio-graph/                   # Audio synthesis
│   ├── text-engine/                   # Text shaping & layout
│   ├── wasm-interpreter/              # Wasm interpreter (stitch-like)
│   ├── microserde/                    # Minimal serialization (JSON, RON, binary, TOML)
│   ├── platform-auth/                 # Cross-platform authentication
│   ├── platform-keychain/             # Secure credential storage
│   ├── platform-open/                 # URI opening
│   ├── platform-url-handler/          # URL scheme registration
│   ├── android-build-tools/           # Android Java compilation in build.rs
│   └── irq-safety/                    # Interrupt-safe locks (no_std)
├── examples/
│   ├── basic-app/                     # Basic application
│   ├── chat-app/                      # Chat application
│   ├── audio-app/                     # Audio application
│   └── data-dashboard/                # Data visualization
└── tools/
    ├── hot-reload-server/             # Hot reload daemon
    └── shader-compiler/               # Shader build tool
```

## Crate 1: reactive-state (Eyeball-like)

### Purpose
Observable types for reactive state management

### Cargo.toml

```toml
[package]
name = "reactive-state"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["sync"] }
futures-core = "0.3"
parking_lot = "0.12"  # Faster than std::sync
smallvec = "1"
```

### Implementation

```rust
// crates/reactive-state/src/lib.rs
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;

/// Observable single value
pub struct Observable<T> {
    inner: Arc<ObservableInner<T>>,
}

struct ObservableInner<T> {
    value: RwLock<T>,
    sender: broadcast::Sender<Arc<T>>,
}

impl<T: Clone + Send + Sync + 'static> Observable<T> {
    pub fn new(value: T) -> Self {
        let (sender, _) = broadcast::channel(100);

        Self {
            inner: Arc::new(ObservableInner {
                value: RwLock::new(value),
                sender,
            }),
        }
    }

    pub fn set(&self, value: T) {
        let arc = Arc::new(value);
        *self.inner.value.write() = arc.clone();
        let _ = self.inner.sender.send(arc);
    }

    pub fn get(&self) -> impl std::ops::Deref<Target = T> + '_ {
        self.inner.value.read()
    }

    pub fn subscribe(&self) -> Subscriber<T> {
        Subscriber {
            receiver: self.inner.sender.subscribe(),
            current: Arc::clone(&self.get()),
        }
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut guard = self.inner.value.write();
        f(&mut *guard);
        let new_val = Arc::new((*guard).clone());
        drop(guard);
        let _ = self.inner.sender.send(new_val);
    }
}

impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

/// Subscriber to observable changes
pub struct Subscriber<T> {
    receiver: broadcast::Receiver<Arc<T>>,
    current: Arc<T>,
}

impl<T: Clone> Subscriber<T> {
    pub fn get(&self) -> Arc<T> {
        Arc::clone(&self.current)
    }

    pub async fn next(&mut self) -> Option<Arc<T>> {
        match self.receiver.recv().await {
            Ok(val) => {
                self.current = Arc::clone(&val);
                Some(val)
            }
            Err(_) => None,
        }
    }

    pub fn try_next(&mut self) -> Option<Arc<T>> {
        match self.receiver.try_recv() {
            Ok(val) => {
                self.current = Arc::clone(&val);
                Some(val)
            }
            Err(_) => None,
        }
    }
}

/// Observable vector with batch updates
pub struct ObservableVec<T> {
    inner: Arc<VecInner<T>>,
}

struct VecInner<T> {
    vec: RwLock<Vec<T>>,
    sender: broadcast::Sender<VecDiff<T>>,
}

#[derive(Debug, Clone)]
pub enum VecDiff<T> {
    Insert { index: usize, value: T },
    Remove { index: usize, value: T },
    Update { index: usize, value: T },
    Clear,
    Push { value: T },
}

impl<T: Clone + Send + Sync + 'static> ObservableVec<T> {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            inner: Arc::new(VecInner {
                vec: RwLock::new(Vec::new()),
                sender,
            }),
        }
    }

    pub fn push(&self, value: T) {
        let mut vec = self.inner.vec.write();
        vec.push(value.clone());
        drop(vec);
        let _ = self.inner.sender.send(VecDiff::Push { value });
    }

    pub fn insert(&self, index: usize, value: T) {
        let mut vec = self.inner.vec.write();
        vec.insert(index, value.clone());
        drop(vec);
        let _ = self.inner.sender.send(VecDiff::Insert { index, value });
    }

    pub fn remove(&self, index: usize) -> Option<T> {
        let mut vec = self.inner.vec.write();
        let val = vec.remove(index);
        drop(vec);
        let _ = self.inner.sender.send(VecDiff::Remove { index, value: val.clone() });
        Some(val)
    }

    pub fn get(&self, index: usize) -> Option<T> {
        self.inner.vec.read().get(index).cloned()
    }

    pub fn len(&self) -> usize {
        self.inner.vec.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.vec.read().is_empty()
    }

    pub fn subscribe(&self) -> VecSubscriber<T> {
        VecSubscriber {
            receiver: self.inner.sender.subscribe(),
        }
    }

    /// Batch multiple operations
    pub fn batch<F>(&self, f: F)
    where
        F: FnOnce(&mut BatchBuilder<T>),
    {
        let mut builder = BatchBuilder::new(self);
        f(&mut builder);
        builder.apply();
    }
}

impl<T> Default for ObservableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BatchBuilder<'a, T> {
    vec: &'a ObservableVec<T>,
    diffs: Vec<VecDiff<T>>,
}

impl<'a, T: Clone> BatchBuilder<'a, T> {
    fn new(vec: &'a ObservableVec<T>) -> Self {
        Self { vec, diffs: Vec::new() }
    }

    pub fn push(&mut self, value: T) {
        self.diffs.push(VecDiff::Push { value });
    }

    pub fn insert(&mut self, index: usize, value: T) {
        self.diffs.push(VecDiff::Insert { index, value });
    }

    pub fn apply(self) {
        let mut vec = self.vec.inner.vec.write();
        for diff in &self.diffs {
            match diff {
                VecDiff::Push { value } => vec.push(value.clone()),
                VecDiff::Insert { index, value } => vec.insert(*index, value.clone()),
                _ => {}
            }
        }
        drop(vec);

        for diff in self.diffs {
            let _ = self.vec.inner.sender.send(diff);
        }
    }
}
```

## Crate 2: render-engine (GPU Rendering)

### Purpose
GPU-accelerated 2D rendering with shader-based drawing

### Cargo.toml

```toml
[package]
name = "render-engine"
version = "0.1.0"
edition = "2021"

[dependencies]
wgpu = "0.18"
bytemuck = { version = "1", features = ["derive"] }
glyphon = "0.5"  # Text rendering
image = "0.24"
nanoserde = "0.1"  # For MPSL-like parsing
```

### Implementation

```rust
// crates/render-engine/src/lib.rs
use wgpu::*;
use std::sync::Arc;

pub struct RenderEngine {
    instance: Instance,
    surface: Option<Surface<'static>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    pipelines: RenderPipelines,
    textures: TextureManager,
}

struct RenderPipelines {
    quad: RenderPipeline,
    text: RenderPipeline,
    path: RenderPipeline,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct QuadInstance {
    pub rect: [f32; 4],  // x, y, width, height
    pub color: [f32; 4], // r, g, b, a
    pub border_radius: f32,
    _padding: [f32; 3],
}

impl RenderEngine {
    pub async fn new(window: impl Into<winit::window::Window> + 'static) -> Self {
        let window = Box::new(window.into());

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::empty(),
                    limits: Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0],
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        // Create pipelines
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("quad_shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/quad.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("quad_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let quad_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("quad_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), QuadInstance::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: 4,  // MSAA
                ..Default::default()
            },
            multiview: None,
        });

        Self {
            instance,
            surface: Some(surface),
            device,
            queue,
            config,
            pipelines: RenderPipelines {
                quad: quad_pipeline,
                text: create_text_pipeline(&device),
                path: create_path_pipeline(&device),
            },
            textures: TextureManager::new(),
        }
    }

    pub fn begin_frame(&mut self) -> Frame {
        let surface = self.surface.as_ref().unwrap();
        surface.get_current_texture().unwrap()
    }

    pub fn render_quad(&mut self, rect: Rect, color: Color, border_radius: f32) {
        let instance = QuadInstance {
            rect: [rect.x, rect.y, rect.w, rect.h],
            color: [color.r, color.g, color.b, color.a],
            border_radius,
            _padding: [0.0; 3],
        };

        // Queue for batch rendering
        // ...
    }

    pub fn end_frame(&mut self, frame: Frame) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
            });

            // Draw quads
            render_pass.set_pipeline(&self.pipelines.quad);
            // ... bind instances and draw

            // Draw text
            render_pass.set_pipeline(&self.pipelines.text);
            // ...

            // Draw paths
            render_pass.set_pipeline(&self.pipelines.path);
            // ...
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
```

### WGSL Shaders

```wgsl
// shaders/quad.wgsl
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) border_radius: f32,
};

@group(0) @binding(0)
var<uniform> viewport: vec2<f32>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) border_radius: f32,
) -> VertexOutput {
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );

    let pos = positions[vertex_index];
    let x = rect.x + pos.x * rect.z;
    let y = rect.y + pos.y * rect.w;

    // Convert to clip space
    let clip_x = (x / viewport.x) * 2.0 - 1.0;
    let clip_y = ((viewport.y - y) / viewport.y) * 2.0 - 1.0;

    return VertexOutput {
        position: vec4<f32>(clip_x, clip_y, 0.0, 1.0),
        uv: pos,
        color: color,
        border_radius: border_radius,
    };
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Rounded rectangle with anti-aliasing
    let uv = input.uv;
    let br = input.border_radius;

    if (br > 0.0) {
        // Distance to corner for rounded corners
        let corner_dist = length(vec2<f32>(
            min(uv.x * 100.0, (1.0 - uv.x) * 100.0),
            min(uv.y * 100.0, (1.0 - uv.y) * 100.0)
        ) - br);

        let alpha = 1.0 - smoothstep(0.0, 1.0, corner_dist - br);
        return vec4<f32>(input.color.rgb, input.color.a * alpha);
    }

    return input.color;
}
```

## Crate 3: mpsl-parser (Style Language)

### Purpose
Parse MPSL-like style definitions

### Cargo.toml

```toml
[package]
name = "mpsl-parser"
version = "0.1.0"
edition = "2021"

[dependencies]
nom = "7"
serde = { version = "1", features = ["derive"] }
```

### Implementation

```rust
// crates/mpsl-parser/src/lib.rs
use nom::{
    bytes::complete::tag,
    character::complete::{alphanumeric1, multispace0, char},
    combinator::*,
    sequence::*,
    branch::alt,
    multi::*,
    IResult,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDefinition {
    pub name: String,
    pub properties: Vec<Property>,
    pub children: Vec<ChildElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
    Color(Color),
    Number(f32),
    String(String),
    Bool(bool),
    Vec4([f32; 4]),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildElement {
    pub name: String,
    pub block: Option<StyleBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleBlock {
    pub properties: Vec<Property>,
    pub children: Vec<ChildElement>,
}

// Parser combinators
fn parse_color(input: &str) -> IResult<&str, Color> {
    let (input, _) = char('#')(input)?;
    let (input, hex) = take(6usize)(input)?;

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();

    Ok((input, Color { r, g, b, a: 255 }))
}

fn parse_number(input: &str) -> IResult<&str, f32> {
    let (input, num): (&str, &str) = recognize(pair(
        opt(char('-')),
        recognize(pair(digit1, opt(pair(char('.'), digit1)))),
    ))(input)?;
    Ok((input, num.parse().unwrap()))
}

fn parse_property(input: &str) -> IResult<&str, Property> {
    let (input, name) = alphanumeric1(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = multispace0(input)?;

    let (input, value) = alt((
        map(parse_color, PropertyValue::Color),
        map(parse_number, PropertyValue::Number),
        // ... other value types
    ))(input)?;

    Ok((input, Property { name: name.to_string(), value }))
}

fn parse_style_block(input: &str) -> IResult<&str, StyleBlock> {
    let (input, _) = char('{')(input)?;
    let (input, _) = multispace0(input)?;

    let (input, properties) = many0(parse_property)(input)?;
    let (input, children) = many0(parse_child_element)(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = char('}')(input)?;

    Ok((input, StyleBlock { properties, children }))
}

pub fn parse_mpsl(input: &str) -> Result<Vec<StyleDefinition>, String> {
    let mut definitions = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        match parse_style_definition(remaining) {
            Ok((rest, def)) => {
                definitions.push(def);
                remaining = rest;
            }
            Err(e) => return Err(format!("Parse error: {:?}", e)),
        }
    }

    Ok(definitions)
}
```

## Example Application

```rust
// examples/basic-app/src/main.rs
use render_engine::*;
use reactive_state::*;
use mpsl_parser::*;

struct App {
    count: Observable<i32>,
    window: Option<winit::window::Window>,
}

impl App {
    fn new() -> Self {
        Self {
            count: Observable::new(0),
            window: None,
        }
    }

    async fn run(&mut self) {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = winit::window::WindowBuilder::new()
            .with_title("My Makepad-like App")
            .build(&event_loop)
            .unwrap();

        self.window = Some(window);

        let mut engine = RenderEngine::new(self.window.take().unwrap()).await;

        // Subscribe to state changes
        let mut count_sub = self.count.subscribe();

        event_loop.run(move |event, elwt| {
            match event {
                winit::event::Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),

                winit::event::Event::WindowEvent {
                    event: WindowEvent::MouseInput { state, button, .. },
                    ..
                } => {
                    if state == ElementState::Pressed && button == MouseButton::Left {
                        // Increment count on click
                        let current = *self.count.get();
                        self.count.set(current + 1);
                    }
                }

                winit::event::Event::MainEventsCleared => {
                    // Check for state updates
                    if let Some(new_count) = count_sub.try_next() {
                        // Redraw with new count
                        let frame = engine.begin_frame();
                        // ... render with updated count
                        engine.end_frame(frame);
                    }
                }

                _ => {}
            }
        });
    }
}

#[tokio::main]
async fn main() {
    let mut app = App::new();
    app.run().await;
}
```

## Crate 4: wasm-interpreter (Stitch-like)

### Purpose
Fast, lightweight Wasm interpreter using sibling call optimization for hot-reloading support.

### Cargo.toml

```toml
[package]
name = "wasm-interpreter"
version = "0.1.0"
edition = "2021"

# Zero runtime dependencies - entirely self-contained
[dev-dependencies]
criterion = "0.5"
wast = "200.0"
```

### Key Design

```rust
// Public API mirrors Wasmtime's ergonomic interface
pub struct Engine { /* configuration */ }
pub struct Store<T> { /* runtime state + user data */ }
pub struct Module { /* compiled wasm module */ }
pub struct Linker<T> { /* import resolution */ }
pub struct Instance { /* instantiated module */ }

impl Module {
    /// Decode, validate, and compile a Wasm binary
    pub fn new(engine: &Engine, bytes: &[u8]) -> Result<Self, DecodeError> {
        let decoded = decode(bytes)?;
        validate(&decoded)?;
        let code = compile(&decoded)?;
        Ok(Self { code })
    }
}

impl<T> Linker<T> {
    /// Define a host function import
    pub fn define(
        &mut self,
        module: &str,
        name: &str,
        val: impl Into<ExternVal>,
    ) -> &mut Self { /* ... */ }

    /// Instantiate a module with resolved imports
    pub fn instantiate(
        &self,
        store: &mut Store<T>,
        module: &Module,
    ) -> Result<Instance, InstantiateError> { /* ... */ }
}

// Core interpreter trick: sibling call dispatch
// Each opcode handler has the same signature and calls the next directly
// LLVM optimizes these into jumps (tail call elimination)
fn exec_i32_add(state: &mut ExecState) -> ExecResult {
    let b = state.stack.pop_i32();
    let a = state.stack.pop_i32();
    state.stack.push_i32(a.wrapping_add(b));
    // Sibling call to next instruction handler
    let next = state.code.next_handler();
    next(state)
}
```

### Rust-Specific Patterns
- **Zero dependencies** at runtime -- fully self-contained
- Relies on LLVM sibling call optimization (not guaranteed by Rust, but reliable on 64-bit)
- `AliasableBox` for self-referential structures without unsafe
- Comprehensive error types with `thiserror`-style variants

---

## Crate 5: microserde (Minimal Serialization)

### Purpose
Lightweight serialization replacing serde for reduced compile times and binary sizes.

### Design

```rust
// Traits per format (no unified Serialize/Deserialize)
pub trait SerJson {
    fn ser_json(&self, d: usize, s: &mut SerJsonState);
}

pub trait DeJson: Sized {
    fn de_json(s: &mut DeJsonState, i: &mut Chars) -> Result<Self, DeJsonErr>;
}

pub trait SerBin {
    fn ser_bin(&self, s: &mut Vec<u8>);
}

pub trait DeBin: Sized {
    fn de_bin(o: &mut usize, d: &[u8]) -> Result<Self, DeBinErr>;
}

pub trait SerRon {
    fn ser_ron(&self, d: usize, s: &mut SerRonState);
}

pub trait DeRon: Sized {
    fn de_ron(s: &mut DeRonState, i: &mut Chars) -> Result<Self, DeRonErr>;
}

// Derive macros auto-generate implementations
#[derive(SerJson, DeJson, SerBin, DeBin, SerRon, DeRon)]
pub struct MyStruct {
    field: String,
    count: u32,
}
```

### Key Rust-Specific Changes
- Per-format traits instead of serde's unified `Serialize`/`Deserialize` -- avoids the serde compilation overhead
- Hand-written recursive descent parsers for each format
- Proc-macros generate format-specific code directly (no intermediate representation)
- Binary format uses variable-length encoding for integers

---

## Crate 6: platform-auth (Cross-Platform Authentication)

### Design Pattern

```rust
// The cfg-if dispatch pattern used by all Robius platform crates
use cfg_if::cfg_if;

pub struct Context { /* platform-specific inner */ }

impl Context {
    pub fn blocking_authenticate(
        &self,
        text: Text,
        policy: &Policy,
    ) -> Result<(), Error> {
        cfg_if! {
            if #[cfg(target_vendor = "apple")] {
                self.inner.authenticate_apple(text, policy)
            } else if #[cfg(target_os = "android")] {
                self.inner.authenticate_android(text, policy)
            } else if #[cfg(target_os = "linux")] {
                self.inner.authenticate_linux(text, policy)
            } else if #[cfg(target_os = "windows")] {
                self.inner.authenticate_windows(text, policy)
            } else {
                Err(Error::Unsupported)
            }
        }
    }
}

// Policy is compile-time constructable
pub struct PolicyBuilder { /* ... */ }

impl PolicyBuilder {
    pub const fn new() -> Self { /* ... */ }
    pub const fn biometrics(self, strength: Option<BiometricStrength>) -> Self { /* ... */ }
    pub const fn password(self, enabled: bool) -> Self { /* ... */ }
    pub const fn build(self) -> Option<Policy> { /* ... */ }
}
```

### Rust-Specific Patterns
- `const fn` builders for compile-time policy construction
- `cfg_if!` macro for zero-overhead platform dispatch
- Apple: `objc2` crate for safe Objective-C interop
- Android: JNI via `jni` crate with Java companion code compiled via `android-build`
- Windows: Both WinRT (modern) and Win32 (fallback) paths

---

## Crate 7: irq-safety (Interrupt-Safe Primitives)

### Design

```rust
#![no_std]

use spin::{Mutex as SpinMutex, RwLock as SpinRwLock};

/// A Mutex that disables interrupts while held
pub struct MutexIrqSafe<T> {
    inner: SpinMutex<T>,
}

impl<T> MutexIrqSafe<T> {
    pub fn lock(&self) -> MutexIrqSafeGuard<T> {
        let held = hold_interrupts(); // Disable IRQs
        let guard = self.inner.lock();
        MutexIrqSafeGuard { guard, held }
    }
}

/// Guard that re-enables interrupts on drop (if they were enabled before)
pub struct MutexIrqSafeGuard<'a, T> {
    guard: spin::MutexGuard<'a, T>,
    held: HeldInterrupts,
}

impl<'a, T> Drop for MutexIrqSafeGuard<'a, T> {
    fn drop(&mut self) {
        // Guard drops first (releases spinlock), then HeldInterrupts
        // restores interrupt state
    }
}

/// Holds interrupts disabled; re-enables on drop if they were enabled
pub struct HeldInterrupts {
    was_enabled: bool,
}

impl Drop for HeldInterrupts {
    fn drop(&mut self) {
        if self.was_enabled {
            enable_interrupts();
        }
    }
}
```

### Rust-Specific Patterns
- RAII-based interrupt management (HeldInterrupts)
- `no_std` compatible for kernel/embedded use
- Architecture-specific interrupt enable/disable via `cfg(target_arch)`
- Drop ordering ensures spinlock is released before interrupts are re-enabled

---

## Full Ecosystem Coverage

### Sub-Project to Crate Mapping

| Source Project | Proposed Crate | Category |
|----------------|---------------|----------|
| makepad/ | ui-toolkit, render-engine, platform-abstraction, audio-graph, text-engine | Core |
| eyeball/ | reactive-state | Core |
| stitch/ | wasm-interpreter | Core |
| microserde/ | microserde | Core |
| makepad-mpsl-parser/ | mpsl-parser | Core |
| robius-authentication/ | platform-auth | Platform |
| robius-keychain/ | platform-keychain | Platform |
| robius-open/ | platform-open | Platform |
| robius-url-handler/ | platform-url-handler | Platform |
| android-build/ | android-build-tools | Platform |
| irq_safety/ | irq-safety | Low-level |
| uX/ | ux (external) | Low-level |
| robrix/ | chat-app example | Application |
| src.Moxin-Org/moly/ | ai-chat-app example | Application |
| makepad_taobao/ | taobao example | Showcase |
| makepad_wechat/ | wechat example | Showcase |
| makepad_wonderous/ | wonderous example | Showcase |
| image_viewer/ | tutorial series | Documentation |
| glui/ | (historical reference) | Legacy |
| experiments/ | experimental workspace | R&D |

## Summary

This Rust revision provides:
- **Observable state** with subscriber pattern (reactive-state)
- **GPU rendering** with wgpu (render-engine)
- **Style language parsing** (mpsl-parser)
- **Cross-platform** windowing with winit
- **Hot reload patterns** via Wasm interpretation (wasm-interpreter)
- **Audio synthesis** capabilities (audio-graph)
- **Minimal serialization** without serde overhead (microserde)
- **Platform abstraction** for auth, keychain, URIs, URLs (platform-* crates)
- **Android build tooling** for JNI/Java integration (android-build-tools)
- **Interrupt-safe primitives** for embedded/kernel contexts (irq-safety)
- **AI/ML integration** patterns from Moly/Moxin

Key patterns from the full Makepad/Robius ecosystem:
1. Immediate mode rendering with GPU-first approach
2. Observable-based reactive state (eyeball pattern)
3. DSL for styling and layout (MPSL)
4. Sibling call optimization for interpreter performance
5. cfg-if based zero-cost platform dispatch
6. RAII everywhere: lock guards, interrupt guards, draw contexts
7. Zero-dependency design where possible (stitch, microserde, mpsl-parser)
8. Batch updates for efficiency in both state and rendering
9. const fn constructors for compile-time configuration
10. Proc-macro code generation for serialization and shader processing
