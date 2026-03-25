# Rust WebGPU Ecosystem

**Part of the WebGPU Exploration Series**

---

## Table of Contents

1. [Overview](#overview)
2. [wgpu Ecosystem](#wgpu-ecosystem)
3. [gpu-alloc and Related Crates](#gpu-alloc-and-related-crates)
4. [Shader Compilation with Naga](#shader-compilation-with-naga)
5. [Native GPU Bindings](#native-gpu-bindings)
6. [Community Crates and Tools](#community-crates-and-tools)
7. [Crate Dependency Graph](#crate-dependency-graph)

---

## Overview

The Rust WebGPU ecosystem is centered around the **wgpu** project, which provides a safe, idiomatic Rust API for WebGPU. The ecosystem has evolved from the earlier gfx-hal project and now includes a comprehensive set of crates for GPU programming.

### Ecosystem Map

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust WebGPU Ecosystem                     │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                   High-Level                          │  │
│  │  (Game engines, GUI toolkits, renderers)             │  │
│  │  bevy, iced, egui, vello, piet, etc.                 │  │
│  └──────────────────────────────────────────────────────┘  │
│                            │                                │
│                            ▼                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                   wgpu                                │  │
│  │  (Safe Rust WebGPU API)                               │  │
│  └──────────────────────────────────────────────────────┘  │
│                            │                                │
│                            ▼                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                  wgpu-core                            │  │
│  │  (WebGPU implementation, deferred context)           │  │
│  └──────────────────────────────────────────────────────┘  │
│                            │                                │
│                            ▼                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                  wgpu-hal                             │  │
│  │  (Hardware Abstraction Layer)                         │  │
│  └──────────────────────────────────────────────────────┘  │
│                            │                                │
│         ┌──────────────────┼──────────────────┐            │
│         ▼                  ▼                  ▼            │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐      │
│  │   Vulkan    │   │    Metal    │   │   D3D12     │      │
│  │  (ash/vk)   │   │ (objc2-metal)│  │  (d3d12)    │      │
│  └─────────────┘   └─────────────┘   └─────────────┘      │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Naga                               │  │
│  │  (Shader translation: WGSL, GLSL, HLSL, SPIR-V)      │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Key Organizations

| Organization | Description |
|--------------|-------------|
| **gfx-rs** | Original home of wgpu, gfx-hal, metal-rs |
| **wgpu** | Current wgpu development (merged with gfx-rs) |
| **linebender** | Vello, Parley, and other GPU projects |
| **bevy** | Game engine using wgpu |
| **embark-studios** | gpu-alloc, gpu-descriptor, and other utilities |

---

## wgpu Ecosystem

### Core Crates

| Crate | Description | Version |
|-------|-------------|---------|
| **wgpu** | Safe, idiomatic Rust API for WebGPU | 24.x |
| **wgpu-core** | Core WebGPU implementation | 24.x |
| **wgpu-hal** | Hardware abstraction layer | 24.x |
| **wgpu-types** | Shared types for wgpu ecosystem | 24.x |

### wgpu Crate Structure

```
wgpu/
├── wgpu/              # Main user-facing API
├── wgpu-core/         # WebGPU implementation
├── wgpu-hal/          # HAL layer
│   ├── src/
│   │   ├── dx12/      # Direct3D 12 backend
│   │   ├── metal/     # Metal backend
│   │   ├── vulkan/    # Vulkan backend
│   │   ├── gles/      # OpenGL ES backend
│   │   └── empty/     # Null backend for testing
│   └── ...
├── wgpu-types/        # Shared types
├── naga/              # Shader translation
└── d3d12/             # D3D12 bindings
```

### Usage Example

```rust
use wgpu::*;

async fn create_device() -> (Device, Queue) {
    // Create instance
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    });

    // Request adapter
    let adapter = instance.request_adapter(&RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    }).await.unwrap();

    // Request device
    let (device, queue) = adapter.request_device(
        &DeviceDescriptor {
            required_features: Features::empty(),
            required_limits: Limits::default(),
            label: None,
        },
        None,
    ).await.unwrap();

    (device, queue)
}

fn create_pipeline(device: &Device) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: TextureFormat::Bgra8UnormSrgb,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    })
}
```

### wgpu-native (FFI)

**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/src.gfx-rs/wgpu-native/`

wgpu-native provides C bindings for wgpu-core, enabling WebGPU access from other languages:

```rust
// wgpu-native FFI layer
#[no_mangle]
pub extern "C" fn wgpuInstanceCreateSurface(
    instance: &native::WGPUInstance,
    descriptor: &native::WGPUSurfaceDescriptor,
) -> native::WGPUSurface {
    // FFI implementation
}
```

**Language Bindings Using wgpu-native:**
- Python: wgpu-py
- .NET: WGPU.NET
- Java: wgpuj
- Go: webgpu
- Julia: WebGPU.jl
- Zig: wgpu_native_zig

### Backends

| Backend | Crate | Platform | Status |
|---------|-------|----------|--------|
| **Vulkan** | wgpu-hal (vulkan) | Linux, Windows, Android | Production |
| **Metal** | wgpu-hal (metal) | macOS, iOS | Production |
| **D3D12** | wgpu-hal (dx12) | Windows 10+ | Production |
| **GL** | wgpu-hal (gles) | Cross-platform | Beta |
| **WebGPU** | web-sys | Web | Production |

### Feature Support

```rust
// Check supported features
let adapter = instance.request_adapter(&RequestAdapterOptions::default()).await;
let features = adapter.features();

// Core features
assert!(features.contains(Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES));

// Optional features
if features.contains(Features::SHADER_F16) {
    // Use 16-bit floats in shaders
}

if features.contains(Features::TIMESTAMP_QUERY) {
    // Use GPU timing queries
}
```

---

## gpu-alloc and Related Crates

### gpu-alloc

**Repository:** embark-studios/gpu-alloc

gpu-alloc is a **GPU memory allocation library** built on top of wgpu. It provides efficient memory management for GPU resources.

```rust
use gpu_alloc::{AllocationFlags, GpuAllocator, MemoryBlock};

// Create allocator
let mut allocator = GpuAllocator::new(
    gpu_alloc::GpuAllocRequest {
        max_buffer_allocation_size: 1024 * 1024 * 1024,
        ..Default::default()
    },
    &device_memory_properties,
);

// Allocate memory
let allocation = allocator.alloc(
    gpu_alloc::Request {
        size: 256,
        align_mask: 255,
        usage: gpu_alloc::Usage::TRANSFER_DST,
        memory_usage: gpu_alloc::MemoryUsage::GpuOnly,
        flags: AllocationFlags::empty(),
    },
    &device,
).unwrap();
```

### gpu-descriptor

**Repository:** embark-studios/gpu-descriptor

gpu-descriptor provides **resource binding management** for wgpu applications:

```rust
use gpu_descriptor::{DescriptorAllocator, DescriptorPool, ShaderStage};

// Create descriptor allocator
let mut descriptor_allocator = DescriptorAllocator::new(
    device.limits().max_bind_groups,
    DescriptorPoolSize {
        ty: wgpu::BufferBindingType::Uniform,
        count: 1000,
    },
);

// Allocate descriptor set
let set = descriptor_allocator.allocate(
    &device,
    &layout,
    &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
    }],
).unwrap();
```

### Related Crates from Embark Studios

| Crate | Description |
|-------|-------------|
| **gpu-alloc** | GPU memory allocation |
| **gpu-descriptor** | Resource binding management |
| **gpu-alloc-types** | Shared types for gpu-alloc |
| **gpu-descriptor-types** | Shared types for gpu-descriptor |

---

## Shader Compilation with Naga

### Overview

**Naga** is a **shader translation library** that converts between shader languages. It's a core component of the wgpu ecosystem.

**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/src.gfx-rs/naga/` (archived, moved to wgpu)

### Supported Formats

**Frontends (Input):**
- WGSL (WebGPU Shading Language)
- GLSL (OpenGL Shading Language)
- HLSL (Direct3D Shading Language)
- SPIR-V (Standard Portable Intermediate Representation)

**Backends (Output):**
- WGSL
- GLSL (OpenGL, WebGL)
- HLSL (Direct3D)
- MSL (Metal Shading Language)
- SPIR-V (Vulkan)
- SPIRV-Capabilities (capability checking)

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Naga                                │
│                                                             │
│  Frontends:              Module IR:         Backends:       │
│  ┌─────────────┐        ┌───────────┐      ┌─────────────┐  │
│  │    WGSL     │───────▶│           │─────▶│    WGSL     │  │
│  │    GLSL     │───────▶│   Naga    │─────▶│    GLSL     │  │
│  │    HLSL     │───────▶│  Module   │─────▶│    HLSL     │  │
│  │   SPIR-V    │───────▶│    IR     │─────▶│    MSL      │  │
│  │             │───────▶│           │─────▶│   SPIR-V    │  │
│  └─────────────┘        └───────────┘      └─────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Usage Example

```rust
use naga::{front::wgsl, valid::Validator, back::glsl};

// Parse WGSL source
let source = r#"
    @vertex
    fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
        return vec4<f32>(position, 1.0);
    }
"#;

let module = wgsl::parse_str(source).unwrap();

// Validate the module
let info = Validator::new(
    naga::valid::ValidationFlags::all(),
    naga::valid::Capabilities::all(),
).validate(&module).unwrap();

// Generate GLSL output
let mut output = String::new();
let mut writer = glsl::Writer::new(
    &mut output,
    &module,
    &info,
    &glsl::Options {
        version: glsl::Version::Desktop(450),
        writer_flags: glsl::WriterFlags::empty(),
        binder: None,
    },
    naga::proc::BoundsCheckPolicies::default(),
).unwrap();

writer.write().unwrap();
println!("Generated GLSL:\n{}", output);
```

### Integration with wgpu

Naga is automatically used by wgpu when you provide shaders in different formats:

```rust
// WGSL (native, no translation needed)
let shader_wgsl = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
});

// SPIR-V (will be validated/translated by Naga)
let shader_spirv = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::SpirV(Cow::Borrowed(include_bytes!("shader.spv"))),
});

// GLSL (will be translated by Naga)
let shader_glsl = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Glsl {
        shader: Cow::Borrowed(include_str!("shader.glsl")),
        stage: wgpu::ShaderStage::Vertex,
        defines: Default::default(),
    },
});
```

### Naga CLI

Naga includes a command-line tool for shader translation:

```bash
# Convert WGSL to GLSL
naga input.wgsl output.glsl

# Convert HLSL to SPIR-V
naga input.hlsl output.spv

# Validate shader
naga --validate input.wgsl
```

---

## Native GPU Bindings

### ash (Vulkan)

**Repository:** ash-rs/ash

ash provides **low-level Vulkan bindings** for Rust:

```rust
use ash::{Entry, Instance, Device};

// Load Vulkan library
let entry = Entry::linked();

// Create instance
let instance = entry
    .create_instance(&create_info, None)
    .unwrap();

// Create device
let device = instance
    .create_device(physical_device, &create_info, None)
    .unwrap();
```

**Note:** wgpu-hal uses ash internally for Vulkan backend, but ash can be used directly for low-level Vulkan access.

### metal-rs (Deprecated)

**Status:** DEPRECATED - Use `objc2-metal` instead

**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/src.gfx-rs/metal-rs/`

```
⚠️ WARNING: This crate is deprecated. The `objc` ecosystem is unmaintained.

Migration path:
- Old: metal = "0.27"
- New: objc2 = "0.5" + objc2-metal = "0.2"
```

### objc2-metal (Recommended)

**Repository:** LinusS1/objc2

The recommended way to access Metal from Rust:

```rust
use objc2_metal::{MTLDevice, MTLCommandBuffer, MTLRenderCommandEncoder};

// Get default device
let device = MTLDevice::default_device().unwrap();

// Create command buffer
let command_queue = device.new_command_queue();
let command_buffer = command_queue.new_command_buffer();

// ... rendering commands ...

command_buffer.commit();
```

### d3d12-rs

**Status:** Moved to wgpu repository

**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/src.gfx-rs/d3d12-rs/`

D3D12 bindings are now maintained as part of the wgpu project for use in the D3D12 backend.

---

## Community Crates and Tools

### Rendering Libraries

| Crate | Description | Repository |
|-------|-------------|------------|
| **vello** | GPU compute 2D renderer | linebender/vello |
| **piet** | 2D graphics API (predecessor to Vello) | linebender/piet |
| **peniko** | 2D graphics primitives | linebender/peniko |
| **kurbo** | 2D geometry library | linebender/kurbo |

### Game Engines

| Crate | Description | Repository |
|-------|-------------|------------|
| **bevy** | Data-driven game engine | bevyengine/bevy |
| **ggez** | 2D game framework | ggez/ggez |
| **macroquad** | Easy-to-use game library | not-fl3/macroquad |

### GUI Toolkits

| Crate | Description | Repository |
|-------|-------------|------------|
| **iced** | Cross-platform GUI | hecrj/iced |
| **egui** | Immediate mode GUI | emilk/egui |
| **dioxus** | React-like GUI | DioxusLabs/dioxus |
| **tauri** | Desktop app framework | tauri-apps/tauri |

### Utility Crates

| Crate | Description | Repository |
|-------|-------------|------------|
| **wgpu-profiler** | GPU profiling for wgpu | alvrmeister/wgpu-profiler |
| **encase** | Shader type uniforms | EmbarkStudios/encase |
| **bytemuck** | Pod casting for GPU buffers | Lokathor/bytemuck |
| **glam** | Math library for graphics | bitshifter/glam |

### wgpu-profiler Example

```rust
use wgpu_profiler::*;

// Create profiler
let mut profiler = GpuProfiler::new(
    wgpu_profiler::GpuProfilerSettings::default(),
    device.adapter_info().backend,
    device.features(),
);

// Record queries
{
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut query = profiler
            .scope("rendering", &mut encoder, &device);

        // ... render commands ...

        profiler.resolve_query(&mut encoder);
    }
    queue.submit([encoder.finish()]);
}

// Get results
profiler.end_frame().unwrap();
```

### encase Example

```rust
use encase::{UniformBuffer, ShaderType};
use glam::Vec3;

#[derive(ShaderType)]
struct Material {
    color: Vec3,
    roughness: f32,
}

let material = Material {
    color: Vec3::new(1.0, 0.0, 0.0),
    roughness: 0.5,
};

let mut buffer = UniformBuffer::new(Vec::new());
buffer.write(&material).unwrap();

// Upload to GPU
queue.write_buffer(&uniform_buffer, 0, buffer.as_ref());
```

---

## Crate Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│                  Application Layer                           │
│  (bevy, iced, vello, custom applications)                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      wgpu                                    │
│  (Safe Rust API for WebGPU)                                 │
│  Dependencies: wgpu-core, wgpu-types, naga                  │
└─────────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
┌─────────────────────┐       ┌─────────────────────┐
│     wgpu-core       │       │       naga          │
│  (WebGPU impl)      │       │  (Shader trans.)    │
│  Dependencies:      │       │  Dependencies:      │
│  wgpu-hal,          │       │  (standalone)       │
│  gpu-alloc,         │       │                     │
│  gpu-descriptor     │       │                     │
└─────────────────────┘       └─────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│                     wgpu-hal                                 │
│  (Hardware Abstraction Layer)                                │
│  Dependencies: ash, objc2-metal, d3d12, glow               │
└─────────────────────────────────────────────────────────────┘
              │
    ┌─────────┼─────────┬─────────────┐
    ▼         ▼         ▼             ▼
┌────────┐ ┌────────┐ ┌────────┐ ┌─────────┐
│  ash   │ │objc2-  │ │ d3d12  │ │  glow   │
│(Vulkan)│ │ metal  │ │(D3D12) │ │(OpenGL) │
└────────┘ └────────┘ └────────┘ └─────────┘
```

### Key Dependencies

| wgpu Component | Dependencies |
|----------------|--------------|
| **wgpu** | wgpu-core, wgpu-types, naga |
| **wgpu-core** | wgpu-hal, gpu-alloc, gpu-descriptor, arrayvec |
| **wgpu-hal** | ash (Vulkan), objc2-metal (Metal), d3d12 (D3D12), glow (GL) |
| **naga** | bit-vec, termcolor, codespan-reporting |

---

## Best Practices

### 1. Use wgpu for Cross-Platform GPU Access

```rust
// Recommended: Use wgpu for portable GPU access
use wgpu::*;

// Avoid: Direct backend access unless you need platform-specific features
// use ash::*;  // Only if you need direct Vulkan access
```

### 2. Handle Missing Features Gracefully

```rust
// Check for features before using them
if device.features().contains(Features::SHADER_F16) {
    // Use FP16 shaders
} else {
    // Fallback to FP32
}
```

### 3. Use Naga for Shader Portability

```rust
// Write shaders in WGSL for best compatibility
let shader = device.create_shader_module(ShaderModuleDescriptor {
    label: None,
    source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
});
```

### 4. Manage Resources with gpu-alloc

For complex applications with many GPU resources:

```rust
// Use gpu-alloc for efficient memory management
let allocator = GpuAllocator::new(request, &properties);
```

---

## Resources

### Official Documentation

- [wgpu Documentation](https://docs.rs/wgpu)
- [wgpu Website](https://wgpu.rs/)
- [wgpu Examples](https://github.com/gfx-rs/wgpu/tree/master/examples)

### Community Resources

- [wgpu Matrix Chat](https://matrix.to/#/#wgpu:matrix.org)
- [Linebender Zulip](https://xi.zulipchat.com/) (Vello, Piet, etc.)
- [Bevy Discord](https://discord.gg/bevy)

### Tutorials

- [wgpu Basics](https://sotrh.github.io/learn-wgpu/)
- [Bevy Engine Tutorials](https://bevyengine.org/learn/)
- [Linebender Blog](https://raphlinus.github.io/)

---

*This document is part of the WebGPU Exploration series. See also: [exploration.md](./exploration.md), [webgpu-fundamentals.md](./webgpu-fundamentals.md), [projects-analysis.md](./projects-analysis.md), [rust-revision.md](./rust-revision.md)*
