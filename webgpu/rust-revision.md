# Rust WebGPU Replication Plan

**Part of the WebGPU Exploration Series**

---

## Table of Contents

1. [Overview](#overview)
2. [Getting Started](#getting-started)
3. [Crate Recommendations](#crate-recommendations)
4. [Project Structure](#project-structure)
5. [Building WebGPU Apps in Rust](#building-webgpu-apps-in-rust)
6. [Best Practices](#best-practices)
7. [Example Project](#example-project)
8. [Common Patterns](#common-patterns)
9. [Troubleshooting](#troubleshooting)

---

## Overview

This document provides a comprehensive guide for building WebGPU applications in Rust. It covers everything from project setup to advanced patterns, based on analysis of the WebGPU ecosystem.

### Why Rust for WebGPU?

| Advantage | Description |
|-----------|-------------|
| **Memory Safety** | No null pointers, buffer overflows, or use-after-free |
| **Zero-Cost Abstractions** | High-level API with no runtime overhead |
| **Cross-Platform** | Same code runs on Windows, macOS, Linux, Web |
| **Excellent Tooling** | cargo, rust-analyzer, clippy, rustfmt |
| **Growing Ecosystem** | Active community with many GPU-related crates |

### When to Use Rust WebGPU

**Good fit:**
- Desktop applications with GPU acceleration
- Game engines and games
- 2D/3D rendering engines
- GPU compute applications
- Cross-platform GUI toolkits

**Consider alternatives when:**
- Web-only deployment (use TypeScript/JavaScript with WebGPU)
- Rapid prototyping needed (consider higher-level engines like Bevy)
- Team lacks Rust experience

---

## Getting Started

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install rustfmt and clippy
rustup component add rustfmt clippy

# For web targets
rustup target add wasm32-unknown-unknown
```

### Minimum Requirements

| Component | Version |
|-----------|---------|
| Rust | 1.82+ (wgpu MSRV) |
| wgpu | 24.x |
| Vulkan | 1.2+ (Linux/Windows) |
| Metal | macOS 10.15+ / iOS 13+ |
| D3D12 | Windows 10+ |

### Create New Project

```bash
# Create new project
cargo new my-webgpu-app
cd my-webgpu-app

# Add dependencies
cargo add wgpu
cargo add winit  # For windowing
cargo add pollster  # For async runtime
cargo add bytemuck  # For Pod casting
cargo add glam  # For math
```

### Basic Cargo.toml

```toml
[package]
name = "my-webgpu-app"
version = "0.1.0"
edition = "2021"
resolver = "2"

[dependencies]
wgpu = "24"
winit = "0.30"
pollster = "0.4"
bytemuck = { version = "1.14", features = ["derive"] }
glam = "0.28"
log = "0.4"
env_logger = "0.11"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
console_error_panic_hook = "0.1"
web-sys = { version = "0.3", features = [
    "Document",
    "Window",
    "Element",
    "HtmlCanvasElement",
] }

[profile.release]
lto = true
opt-level = 3
```

---

## Crate Recommendations

### Core Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| **wgpu** | WebGPU API | 24.x |
| **winit** | Window creation, input | 0.30.x |
| **pollster** | Async runtime | 0.4.x |
| **futures** | Async utilities | 0.3.x |

### Math and Types

| Crate | Purpose | Version |
|-------|---------|---------|
| **glam** | Vector/math library | 0.28.x |
| **bytemuck** | Pod casting | 1.14.x |
| **encase** | Shader uniforms | 0.9.x |

### Optional Utilities

| Crate | Purpose | Version |
|-------|---------|---------|
| **wgpu-profiler** | GPU profiling | 0.19.x |
| **image** | Image loading | 0.25.x |
| **png** | PNG decoding | 0.17.x |
| **obj** | Wavefront OBJ loading | 0.10.x |
| **kamadak-exif** | EXIF parsing | 0.5.x |

### Full-Featured Template

```toml
[package]
name = "my-webgpu-app"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core
wgpu = "24"
winit = "0.30"
pollster = "0.4"

# Math and types
glam = "0.28"
bytemuck = { version = "1.14", features = ["derive"] }
encase = "0.9"

# Utilities
log = "0.4"
env_logger = "0.11"
image = "0.25"
anyhow = "1.0"

# Profiling (optional)
wgpu-profiler = "0.19"

# WASM support
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
console_error_panic_hook = "0.1"
console_log = "1.0"
web-sys = { version = "0.3", features = [
    "Document",
    "Window",
    "Element",
    "HtmlCanvasElement",
    "WebGl2RenderingContext",
] }

[build-dependencies]
# For shader compilation
naga-cli = { version = "23", package = "naga-cli" }

[profile.release]
lto = true
opt-level = 3
```

---

## Project Structure

### Recommended Layout

```
my-webgpu-app/
├── Cargo.toml
├── Cargo.lock
├── build.rs              # Optional: shader compilation
├── README.md
├── LICENSE
│
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library root (optional)
│   │
│   ├── app/
│   │   ├── mod.rs
│   │   ├── state.rs      # Application state
│   │   └── config.rs     # Configuration
│   │
│   ├── renderer/
│   │   ├── mod.rs
│   │   ├── device.rs     # Device initialization
│   │   ├── pipeline.rs   # Pipeline creation
│   │   ├── bindgroup.rs  # Bind group management
│   │   └── render.rs     # Render logic
│   │
│   ├── resource/
│   │   ├── mod.rs
│   │   ├── buffer.rs     # Buffer management
│   │   ├── texture.rs    # Texture management
│   │   └── loader.rs     # Asset loading
│   │
│   └── shader/
│       ├── mod.rs
│       ├── triangle.wgsl
│       └── compute.wgsl
│
├── assets/
│   ├── textures/
│   ├── models/
│   └── shaders/          # Compiled shaders
│
└── tests/
    ├── integration.rs
    └── screenshots.rs
```

### Minimal Structure

For simple projects:

```
my-webgpu-app/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── renderer.rs
│   └── shader.wgsl
└── assets/
```

---

## Building WebGPU Apps in Rust

### Step 1: Initialize wgpu

```rust
use wgpu::*;
use anyhow::Result;

pub struct GpuContext {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl GpuContext {
    pub async fn new() -> Result<Self> {
        // Create instance
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        // Request adapter
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("No appropriate GPU adapter found"))?;

        // Request device
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    label: Some("Main device"),
                },
                None,
            )
            .await?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}
```

### Step 2: Create a Surface

```rust
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    window::{Window, WindowId},
};

pub struct App {
    context: GpuContext,
    surface: Option<Surface<'static>>,
    window: Option<Window>,
    // ... other fields
}

impl App {
    pub fn create_surface(&mut self, window: &Window) -> Result<()> {
        self.surface = Some(
            pollster::block_on(self.context.instance.create_surface(window))?
        );
        Ok(())
    }

    pub fn configure_surface(&self, surface: &Surface, size: winit::dpi::PhysicalSize<u32>) {
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: self.context.adapter.get_surface_format(surface).0[0],
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        self.surface.configure(&self.context.device, &surface_config);
    }
}
```

### Step 3: Create Shaders

```wgsl
// shader.wgsl

// Uniform buffer structure
struct Uniforms {
    model_matrix: mat4x4<f32>,
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Vertex output
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Vertex shader
@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.projection_matrix *
                      uniforms.view_matrix *
                      uniforms.model_matrix *
                      vec4<f32>(position, 1.0);
    output.color = color;
    return output;
}

// Fragment shader
@fragment
fn fs_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color;
}
```

### Step 4: Create Pipeline

```rust
impl App {
    pub fn create_pipeline(&self, shader: &ShaderModule) -> RenderPipeline {
        let pipeline_layout = self.context.device.create_pipeline_layout(
            &PipelineLayoutDescriptor {
                label: Some("Main pipeline layout"),
                bind_group_layouts: &[&self.bind_group_layout],
                push_constant_ranges: &[],
            }
        );

        self.context.device.create_render_pipeline(
            &RenderPipelineDescriptor {
                label: Some("Main pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    buffers: &[VertexBufferLayout {
                        array_stride: 32,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[
                            VertexAttribute::new(Float32x3, 0),  // position
                            VertexAttribute::new(Float32x4, 8),  // color
                        ],
                    }],
                },
                fragment: Some(FragmentState {
                    module: shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Bgra8UnormSrgb,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            }
        )
    }
}
```

### Step 5: Render Loop

```rust
impl App {
    pub fn render(&mut self) -> Result<()> {
        let surface = self.surface.as_ref().unwrap();
        let frame = surface.get_current_texture()?;
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.context.device.create_command_encoder(
            &CommandEncoderDescriptor { label: Some("Render encoder") }
        );

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..3, 0..1);
        }

        self.context.queue.submit([encoder.finish()]);
        frame.present();

        Ok(())
    }
}
```

---

## Best Practices

### 1. Resource Management

```rust
// Use RAII for GPU resources
pub struct Resources {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub texture: Texture,
    pub bind_group: BindGroup,
}

impl Drop for Resources {
    fn drop(&mut self) {
        // Resources are automatically cleaned up by wgpu
        // No manual cleanup needed!
    }
}
```

### 2. Error Handling

```rust
use anyhow::{Result, Context};

pub fn create_buffer(
    device: &Device,
    size: u64,
    usage: BufferUsages,
    label: &str,
) -> Result<Buffer> {
    Ok(device.create_buffer(&BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    }))
}

// Set up error scope for async error handling
device.push_error_scope(ErrorFilter::Validation);
device.push_error_scope(ErrorFilter::OutOfMemory);

// Check for errors
async fn check_error(device: &Device) -> Option<Error> {
    device.pop_error_scope().await
}
```

### 3. Async Initialization

```rust
// Use pollster for blocking on async in main thread
fn main() {
    pollster::block_on(run());
}

async fn run() -> Result<()> {
    let context = GpuContext::new().await?;
    // ... rest of initialization
}

// Or use tokio for more complex async
#[tokio::main]
async fn main() {
    // ... async code
}
```

### 4. Shader Compilation

```rust
// Option 1: Include WGSL at compile time (recommended)
let shader = device.create_shader_module(ShaderModuleDescriptor {
    label: None,
    source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
});

// Option 2: Compile WGSL to SPIR-V at build time
// build.rs:
// use naga::{front::wgsl, back::spv};
// let module = wgsl::parse_str(source)?;
// let spv = spv::write_vec(&module, &info, &options)?;

// Option 3: Load compiled SPIR-V at runtime
let shader = device.create_shader_module(ShaderModuleDescriptor {
    label: None,
    source: ShaderSource::SpirV(Cow::Borrowed(include_bytes!("shader.spv"))),
});
```

### 5. Platform-Specific Code

```rust
// Handle platform differences
#[cfg(target_os = "macos")]
const BACKEND_PREFERENCE: PowerPreference = PowerPreference::HighPerformance;

#[cfg(not(target_os = "macos"))]
const BACKEND_PREFERENCE: PowerPreference = PowerPreference::HighPerformance;

// Use conditional compilation for platform-specific features
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

#[cfg(not(target_arch = "wasm32"))]
use winit::window::Window;
```

---

## Example Project

### Complete Triangle Example

```rust
// src/main.rs
use wgpu::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use anyhow::Result;

struct State {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    depth_texture: TextureView,
    window: Option<Window>,
}

impl State {
    async fn new(window: Window) -> Result<Self> {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create vertex buffer (triangle)
        let vertices: [[f32; 2]; 3] = [
            [0.0, 0.5],
            [-0.5, -0.5],
            [0.5, -0.5],
        ];

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        // Create pipeline
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(
            &PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            }
        );

        let pipeline = device.create_render_pipeline(
            &RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[VertexBufferLayout {
                        array_stride: 8,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[VertexAttribute::new(Float32x2, 0)],
                    }],
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: config.format,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            }
        );

        // Create depth texture
        let depth_texture = create_depth_texture(&device, &config, "Depth texture");

        Ok(Self {
            device,
            queue,
            surface,
            config,
            pipeline,
            vertex_buffer,
            depth_texture,
            window: Some(window),
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = create_depth_texture(&self.device, &self.config, "Depth texture");
        }
    }

    fn render(&mut self) -> Result<()> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(
            &CommandEncoderDescriptor { label: Some("Render encoder") }
        );

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit([encoder.finish()]);
        frame.present();

        Ok(())
    }
}

fn create_depth_texture(
    device: &Device,
    config: &SurfaceConfiguration,
    label: &str,
) -> TextureView {
    let size = Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    texture.create_view(&TextureViewDescriptor::default())
}

// Winit application handler
struct App {
    state: Option<State>,
    window_id: Option<WindowId>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window = event_loop
                .create_window(Window::default_attributes())
                .unwrap();
            self.window_id = Some(window.id());

            // Create state asynchronously
            let state_future = State::new(window);
            self.state = Some(pollster::block_on(state_future).unwrap());
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.resize(size);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.render().unwrap();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        state: None,
        window_id: None,
    };

    event_loop.run_app(&mut app).unwrap();
}
```

### Shader (shader.wgsl)

```wgsl
// Vertex shader
@vertex
fn vs_main(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 0.0, 1.0);
}

// Fragment shader
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);  // Red triangle
}
```

---

## Common Patterns

### Buffer Pattern

```rust
// Create buffer with initial data
let buffer = device.create_buffer_init(&BufferInitDescriptor {
    label: Some("Buffer name"),
    contents: bytemuck::cast_slice(&data),
    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
});

// Create buffer without data
let buffer = device.create_buffer(&BufferDescriptor {
    label: Some("Buffer name"),
    size: size_in_bytes,
    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    mapped_at_creation: false,
});

// Write to buffer
queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&new_data));

// Read from buffer (async)
let buffer_slice = buffer.slice(..);
let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
buffer_slice.map_async(MapMode::Read, move |result| {
    sender.send(result).unwrap();
});
device.poll(Maintain::Wait);
receiver.receive().await.unwrap().unwrap();
```

### Texture Pattern

```rust
// Create texture
let texture = device.create_texture(&TextureDescriptor {
    label: Some("Texture name"),
    size: Extent3d {
        width: 256,
        height: 256,
        depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: TextureDimension::D2,
    format: TextureFormat::Rgba8UnormSrgb,
    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
    view_formats: &[],
});

// Load image and upload to texture
async fn load_texture(device: &Device, queue: &Queue, path: &str) -> Texture {
    let image = image::open(path).unwrap().to_rgba8();
    let dimensions = image.dimensions();

    let texture = device.create_texture(&TextureDescriptor {
        label: Some(path),
        size: Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &image,
        ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: None,
        },
        Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        },
    );

    texture
}
```

### Bind Group Pattern

```rust
// Create bind group layout
let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
    label: Some("Bind group layout"),
    entries: &[
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
    ],
});

// Create bind group
let bind_group = device.create_bind_group(&BindGroupDescriptor {
    label: Some("Bind group"),
    layout: &bind_group_layout,
    entries: &[
        BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        },
        BindGroupEntry {
            binding: 1,
            resource: BindingResource::TextureView(&texture_view),
        },
        BindGroupEntry {
            binding: 2,
            resource: BindingResource::Sampler(&sampler),
        },
    ],
});
```

---

## Troubleshooting

### Common Issues

**1. "No appropriate GPU adapter found"**

```rust
// Check if WebGPU is supported
let instance = Instance::new(InstanceDescriptor::default());
let adapter = instance.request_adapter(&RequestAdapterOptions::default()).await;

if adapter.is_none() {
    eprintln!("No GPU adapter found!");
    eprintln!("Available backends: {:?}", Backends::all());

    // Try forcing fallback adapter
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            force_fallback_adapter: true,
            ..Default::default()
        })
        .await;
}
```

**2. Validation Errors**

```rust
// Enable validation layers
env_logger::init();

// Set up error callback
device.set_uncaptured_error_handler(|error| {
    eprintln!("Uncaptured error: {:?}", error);
});

// Check for specific errors
device.push_error_scope(ErrorFilter::Validation);
// ... operations ...
if let Some(error) = device.pop_error_scope().await {
    eprintln!("Validation error: {:?}", error);
}
```

**3. Shader Compilation Errors**

```rust
// Use naga-cli to validate shaders at build time
// cargo add --dev naga-cli

// build.rs
use std::{env, fs};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let shader_path = format!("{}/src/shader.wgsl", manifest_dir);

    // Validate shader
    let output = std::process::Command::new("naga")
        .arg(&shader_path)
        .output()
        .expect("Failed to run naga");

    if !output.status.success() {
        panic!("Shader validation failed:\n{}",
               String::from_utf8_lossy(&output.stderr));
    }
}
```

**4. Performance Issues**

```rust
// Profile GPU usage
use wgpu_profiler::*;

let mut profiler = GpuProfiler::new(
    GpuProfilerSettings::default(),
    adapter_info.backend,
    device.features(),
);

{
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut query = profiler.scope("render_pass", &mut encoder, &device);
        // ... render commands ...
    }
    profiler.resolve_query(&mut encoder);
    queue.submit([encoder.finish()]);
}

profiler.end_frame().unwrap();
if let Some(profiler_data) = profiler.process_finished_frame() {
    // Analyze timing data
}
```

---

## Resources

### Official Documentation

- [wgpu Documentation](https://docs.rs/wgpu)
- [wgpu Examples](https://github.com/gfx-rs/wgpu/tree/master/examples)
- [Learn wgpu](https://sotrh.github.io/learn-wgpu/)

### Example Projects

- [wgpu examples](https://github.com/gfx-rs/wgpu/tree/master/examples)
- [Vello](https://github.com/linebender/vello) - 2D renderer
- [Bevy](https://github.com/bevyengine/bevy) - Game engine
- [iced](https://github.com/iced-rs/iced) - GUI toolkit

### Community

- [wgpu Matrix](https://matrix.to/#/#wgpu:matrix.org)
- [Linebender Zulip](https://xi.zulipchat.com/)
- [Rust GameDev Discord](https://discord.gg/rust-gamedev)

---

*This document is part of the WebGPU Exploration series. See also: [exploration.md](./exploration.md), [webgpu-fundamentals.md](./webgpu-fundamentals.md), [projects-analysis.md](./projects-analysis.md), [rust-ecosystem.md](./rust-ecosystem.md)*
