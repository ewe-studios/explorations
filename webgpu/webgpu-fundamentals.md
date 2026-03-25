# WebGPU Fundamentals

**Part of the WebGPU Exploration Series**

---

## Table of Contents

1. [What is WebGPU?](#what-is-webgpu)
2. [Relationship to Vulkan, Metal, D3D12](#relationship-to-vulkan-metal-d3d12)
3. [Browser Support](#browser-support)
4. [Use Cases](#use-cases)
5. [Core API Concepts](#core-api-concepts)
6. [Code Examples](#code-examples)

---

## What is WebGPU?

WebGPU is a **modern, low-level graphics and compute API for the web** that provides direct access to GPU functionality across different platforms and devices. It represents a paradigm shift from the WebGL API, offering:

- **Compute shader support** for general-purpose GPU computation
- **Explicit control** over GPU resources and memory
- **Modern GPU features** like bind groups, pipelines, and command encoding
- **Better performance** through reduced driver overhead
- **Safer API** with built-in validation and error handling

### Historical Context

```
WebGL 1.0 (2011) ────▶ WebGL 2.0 (2017) ────▶ WebGPU (2023)
    │                       │                      │
    ▼                       ▼                      ▼
  OpenGL ES 2.0          OpenGL ES 3.0         Modern GPU APIs
  (Direct API)           (Direct API)          (Abstracted HAL)
```

WebGL was essentially OpenGL in the browser - a direct mapping that showed its age as GPU architectures evolved. WebGPU is instead a **hardware abstraction layer** that maps to modern native APIs.

### Key Characteristics

| Feature | WebGL | WebGPU |
|---------|-------|--------|
| API Style | Immediate mode | Retained mode |
| State Management | Global state machine | Encapsulated objects |
| Compute Shaders | ❌ No | ✅ Yes |
| Bind Points | Fixed function | Bind groups |
| Validation | Always on (slow) | Optional (debug vs release) |
| Threading | Single thread | Multi-thread friendly |
| Shader Language | GLSL ES | WGSL |

---

## Relationship to Vulkan, Metal, D3D12

WebGPU is **not** a new graphics API that implementations must create from scratch. Instead, it's a **portable abstraction** that maps to existing native GPU APIs:

```
┌─────────────────────────────────────────────────────────────┐
│                      WebGPU Application                      │
│                   (JavaScript / TypeScript)                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    WebGPU Implementation                     │
│         (Dawn, wgpu, Browser Native Implementation)          │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│    Vulkan     │   │    Metal      │   │    D3D12      │
│   (Linux,     │   │   (macOS,     │   │   (Windows)   │
│   Android)    │   │    iOS)       │   │               │
└───────────────┘   └───────────────┘   └───────────────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            ▼
                    ┌───────────────┐
                    │     GPU       │
                    │   Hardware    │
                    └───────────────┘
```

### Backend Mapping

| Platform | Primary Backend | Fallback |
|----------|-----------------|----------|
| Windows 10/11 | D3D12 | Vulkan, OpenGL |
| macOS | Metal | OpenGL (legacy) |
| iOS | Metal | - |
| Linux | Vulkan | OpenGL |
| Android | Vulkan | OpenGL ES |

### Implementation Details

**Dawn** (Google's WebGPU implementation):
- Primary backend for Chromium browsers
- Written in C++
- Backends: D3D12, Metal, Vulkan, OpenGL
- Includes Tint shader compiler for WGSL

**wgpu** (Rust implementation):
- Used in Firefox
- Written in Rust
- Backends: D3D12, Metal, Vulkan, GL
- Provides both Rust API and native FFI (wgpu-native)

---

## Browser Support

### Current Browser Support (2026)

| Browser | Status | Version | Notes |
|---------|--------|---------|-------|
| **Chrome** | ✅ Production | 113+ | Default enabled |
| **Edge** | ✅ Production | 113+ | Chromium-based |
| **Firefox** | 🟡 Nightly | - | Behind flag |
| **Safari** | 🟡 Technology Preview | - | Limited support |
| **Opera** | ✅ Production | 99+ | Chromium-based |

### Platform Support Matrix

| Platform | Chrome | Firefox | Safari |
|----------|--------|---------|--------|
| Windows 10/11 | ✅ | 🟡 | N/A |
| macOS 12+ | ✅ | 🟡 | 🟡 |
| Linux | ✅ | 🟡 | N/A |
| Android 10+ | ✅ | ❌ | N/A |
| iOS 16+ | ✅* | ❌ | 🟡 |

*On iOS, Chrome uses WebKit (same as Safari) due to App Store requirements.

### Feature Levels

WebGPU defines **feature levels** that guarantee certain capabilities:

```typescript
// Check supported features
const adapter = await navigator.gpu.requestAdapter();
const features = adapter.features;

// Core features (always available)
adapter.features.has('shader-f16');
adapter.features.has('texture-compression-bc');

// Tier 1 features (common)
adapter.features.has('depth-clip-control');

// Tier 2 features (advanced)
adapter.features.has('ray-tracing');
```

### Checking WebGPU Support

```typescript
async function checkWebGPUSupport() {
  if (!navigator.gpu) {
    console.log('WebGPU not supported');
    return null;
  }

  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) {
    console.log('No appropriate GPU adapter found');
    return null;
  }

  const device = await adapter.requestDevice();
  const limits = adapter.limits;

  return { adapter, device, limits };
}
```

---

## Use Cases

WebGPU's versatility enables many application types:

### 1. 3D Graphics and Games

```typescript
// 3D rendering with custom shaders
const shaderModule = device.createShaderModule({
  code: `
    @vertex
    fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
      return vec4<f32>(position, 1.0);
    }

    @fragment
    fn fs_main() -> @location(0) vec4<f32> {
      return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
  `
});
```

**Libraries/Frameworks:**
- **three.js** - Popular 3D library with WebGPU renderer
- **Babylon.js** - Full-featured 3D engine
- **Playcanvas** - WebGPU 3D engine
- **Filament** - Google's PBR rendering engine

### 2. 2D Vector Graphics

```typescript
// Vello-style 2D rendering
// Uses compute shaders for parallel rasterization
// Prefix-sum algorithms for sorting without CPU
```

**Libraries:**
- **Vello** - GPU compute 2D renderer (Rust)
- **Blitz** - HTML/CSS rendering engine
- **Skia** - 2D graphics library with WebGPU backend

### 3. Machine Learning / AI

```typescript
// TensorFlow.js WebGPU backend
// Matrix multiplications on GPU
// Neural network inference in browser
```

**Libraries:**
- **TensorFlow.js** - ML with WebGPU backend
- **ONNX Runtime Web** - Model inference
- **transformers.js** - Hugging Face models

### 4. Video Processing

```typescript
// Smelter - Real-time video composition
// GPU-accelerated video filters
// Live streaming effects
```

**Applications:**
- **Smelter** - Video/audio composition
- **FFmpeg.wasm** - Video encoding/decoding
- Custom video filters and effects

### 5. Scientific Computing

```typescript
// GPGPU (General Purpose GPU)
// Parallel computation on large datasets
// Physics simulations
```

**Applications:**
- Fluid dynamics simulations
- N-body physics
- Image processing
- Cryptography

### 6. Data Visualization

```typescript
// Large-scale data rendering
// GPU-accelerated charting
// Real-time visualization
```

**Applications:**
- Scientific visualization
- Financial charting
- Geographic information systems

---

## Core API Concepts

### Device Model

```
┌─────────────────┐
│    Navigator    │
└────────┬────────┘
         │ .gpu
         ▼
┌─────────────────┐
│     Adapter     │  (Physical GPU)
└────────┬────────┘
         │ .requestDevice()
         ▼
┌─────────────────┐
│     Device      │  (Logical interface)
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌───────┐ ┌───────┐
│Queue  │ │Context│
└───────┘ └───────┘
```

### Resource Hierarchy

```typescript
// 1. Request adapter (physical GPU)
const adapter = await navigator.gpu.requestAdapter();

// 2. Request device (logical interface)
const device = await adapter.requestDevice({
  requiredFeatures: ['shader-f16'],
  requiredLimits: {
    maxTextureDimension2D: 4096,
  }
});

// 3. Get queue for command submission
const queue = device.queue;
```

### Bind Group Model

WebGPU uses a **bind group** system for resource binding:

```
┌─────────────────────────────────────────────────────┐
│              Pipeline Layout                         │
│  ┌─────────────────────────────────────────────┐    │
│  │            Bind Group Layout 0              │    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────────┐   │    │
│  │  │ Uniform │ │  Sampled│ │  Storage    │   │    │
│  │  │ Buffer  │ │ Texture │ │  Buffer     │   │    │
│  │  └─────────┘ └─────────┘ └─────────────┘   │    │
│  └─────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────┐    │
│  │            Bind Group Layout 1              │    │
│  │  ┌─────────┐ ┌─────────┐                    │    │
│  │  │ Sampler │ │Storage  │                    │    │
│  │  │         │ │Texture  │                    │    │
│  │  └─────────┘ └─────────┘                    │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

### Shader Model (WGSL)

WebGPU Shading Language (WGSL) is the built-in shader language:

```wgsl
// WGSL Shader Example

// Uniform buffer structure
struct Uniforms {
  model_matrix: mat4x4<f32>,
  view_matrix: mat4x4<f32>,
  projection_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Vertex shader
@vertex
fn vs_main(
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
  @location(2) uv: vec2<f32>
) -> VertexOutput {
  var output: VertexOutput;
  output.position = uniforms.projection_matrix *
                    uniforms.view_matrix *
                    uniforms.model_matrix *
                    vec4<f32>(position, 1.0);
  output.normal = normal;
  output.uv = uv;
  return output;
}

// Fragment shader
@fragment
fn fs_main(@in input: VertexOutput) -> @location(0) vec4<f32> {
  let light_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
  let diffuse = max(dot(input.normal, light_dir), 0.0);
  return vec4<f32>(diffuse, diffuse, diffuse, 1.0);
}
```

### Command Encoding

```typescript
// Commands are encoded and submitted to the queue
const commandEncoder = device.createCommandEncoder();

// Begin render pass
const renderPass = commandEncoder.beginRenderPass({
  colorAttachments: [{
    view: texture.createView(),
    clearValue: { r: 0, g: 0, b: 0, a: 1 },
    loadOp: 'clear',
    storeOp: 'store',
  }]
});

// Set pipeline and bind groups
renderPass.setPipeline(pipeline);
renderPass.setBindGroup(0, bindGroup);
renderPass.draw(3);

// End pass and submit
renderPass.end();
device.queue.submit([commandEncoder.finish()]);
```

---

## Code Examples

### Example 1: Basic Triangle Rendering

```typescript
async function initWebGPU() {
  // Initialize device
  const adapter = await navigator.gpu.requestAdapter();
  const device = await adapter.requestDevice();

  // Create shader
  const shaderCode = `
    @vertex
    fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> @builtin(position) vec4<f32> {
      var positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, -0.5)
      );
      let position = positions[vertexIndex];
      return vec4<f32>(position, 0.0, 1.0);
    }

    @fragment
    fn fs_main() -> @location(0) vec4<f32> {
      return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
  `;

  const shaderModule = device.createShaderModule({ code: shaderCode });

  // Create pipeline
  const pipeline = device.createRenderPipeline({
    layout: 'auto',
    vertex: {
      module: shaderModule,
      entryPoint: 'vs_main',
    },
    fragment: {
      module: shaderModule,
      entryPoint: 'fs_main',
      targets: [{ format: 'bgra8unorm' }],
    },
    primitive: {
      topology: 'triangle-list',
    },
  });

  return { device, pipeline };
}
```

### Example 2: Compute Shader

```typescript
// Compute shader for vector addition
const computeShader = `
  @group(0) @binding(0) var<storage, read> inputA: array<f32>;
  @group(0) @binding(1) var<storage, read> inputB: array<f32>;
  @group(0) @binding(2) var<storage, read_write> output: array<f32>;

  @compute @workgroup_size(64)
  fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    output[index] = inputA[index] + inputB[index];
  }
`;

async function runCompute(device: GPUDevice) {
  const shaderModule = device.createShaderModule({ code: computeShader });

  // Create buffers
  const inputA = device.createBuffer({
    size: 1024 * 4,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });

  const inputB = device.createBuffer({
    size: 1024 * 4,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });

  const output = device.createBuffer({
    size: 1024 * 4,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
  });

  // Create bind group
  const bindGroup = device.createBindGroup({
    layout: 'auto',
    entries: [
      { binding: 0, resource: { buffer: inputA } },
      { binding: 1, resource: { buffer: inputB } },
      { binding: 2, resource: { buffer: output } },
    ],
  });

  // Create compute pipeline
  const pipeline = device.createComputePipeline({
    layout: 'auto',
    compute: {
      module: shaderModule,
      entryPoint: 'main',
    },
  });

  // Encode and submit commands
  const commandEncoder = device.createCommandEncoder();
  const passEncoder = commandEncoder.beginComputePass();
  passEncoder.setPipeline(pipeline);
  passEncoder.setBindGroup(0, bindGroup);
  passEncoder.dispatchWorkgroups(16);
  passEncoder.end();

  device.queue.submit([commandEncoder.finish()]);
}
```

### Example 3: Texture Sampling

```typescript
// Create and sample a texture
const texture = device.createTexture({
  size: [256, 256],
  format: 'rgba8unorm',
  usage: GPUTextureUsage.TEXTURE_BINDING |
         GPUTextureUsage.COPY_DST |
         GPUTextureUsage.RENDER_ATTACHMENT,
});

const sampler = device.createSampler({
  magFilter: 'linear',
  minFilter: 'linear',
});

// In shader:
const textureShader = `
  @group(0) @binding(0) var myTexture: texture_2d<f32>;
  @group(0) @binding(1) var mySampler: sampler;

  @fragment
  fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pos.xy / vec2<f32>(256.0, 256.0);
    return textureSample(myTexture, mySampler, uv);
  }
`;
```

---

## Performance Considerations

### Best Practices

1. **Minimize state changes** - Group draw calls by pipeline and bind group
2. **Use compute shaders** - Offload parallelizable work to GPU
3. **Batch operations** - Reduce command buffer submissions
4. **Pipeline caching** - Reuse pipeline layouts when possible
5. **Async resource loading** - Don't block the main thread

### Validation Layers

```typescript
// Enable validation in development
const device = await adapter.requestDevice({
  requiredFeatures: [],
  requiredLimits: {},
});

// Set up error callback
device.pushErrorScope('validation');
device.pushErrorScope('out-of-memory');

// Check for errors periodically
const error = await device.popErrorScope();
if (error) {
  console.error('GPU Error:', error);
}
```

---

## WGSL (WebGPU Shading Language)

### Basic Syntax

```wgsl
// Type system
var position: vec3<f32>;
var color: vec4<f32>;
var matrix: mat4x4<f32>;

// Structures
struct Vertex {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>,
};

// Functions
fn normalize_vector(v: vec3<f32>) -> vec3<f32> {
  return normalize(v);
}

// Built-in variables
@vertex
fn main(
  @builtin(vertex_index) vertexIndex: u32,
  @builtin(instance_index) instanceIndex: u32
) -> @builtin(position) vec4<f32> {
  // Shader code
}
```

### Shader Stages

| Stage | Purpose | Built-ins Available |
|-------|---------|---------------------|
| `@vertex` | Transform vertices | `vertex_index`, `instance_index` |
| `@fragment` | Compute pixel colors | `position`, `front_facing` |
| `@compute` | General computation | `global_invocation_id`, `local_invocation_id` |

---

## Resources

### Official Documentation

- [WebGPU Specification](https://www.w3.org/TR/webgpu/)
- [WebGPU Developer Guide](https://webgpu.dev)
- [MDN WebGPU API](https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API)

### Tutorials and Guides

- [WebGPU Fundamentals](https://webgpufundamentals.org/)
- [Learn WebGPU](https://eliemichel.github.io/LearnWebGPU/)
- [Google WebGPU Samples](https://google.github.io/touringjs/)

### Community Resources

- [WebGPU Matrix Chat](https://matrix.to/#/#WebGPU:matrix.org)
- [r/webgpu on Reddit](https://reddit.com/r/webgpu)
- [TypeGPU Discord](https://discord.gg/8jpfgDqPcM)

---

*This document is part of the WebGPU Exploration series. See also: [exploration.md](./exploration.md), [rust-ecosystem.md](./rust-ecosystem.md), [rust-revision.md](./rust-revision.md)*
