# WebGPU Projects Analysis

**Part of the WebGPU Exploration Series**

---

## Table of Contents

1. [Dawn - Google's WebGPU Implementation](#dawn---googles-webgpu-implementation)
2. [wgpu-native - Rust WebGPU FFI](#wgpu-native---rust-webgpu-ffi)
3. [gfx-rs Ecosystem](#gfx-rs-ecosystem)
4. [Vello - 2D GPU Renderer](#vello---2d-gpu-renderer)
5. [TypeGPU - Type-Safe WebGPU](#typegpu---type-safe-webgpu)
6. [Filament - PBR Rendering Engine](#filament---pbr-rendering-engine)
7. [Smelter - Video Composition](#smelter---video-composition)
8. [Blitz - HTML/CSS Renderer](#blitz---htmlcss-renderer)
9. [TensorFlow.js WebGPU Backend](#tensorflowjs-webgpu-backend)
10. [Canvas2D Spec Proposals](#canvas2d-spec-proposals)

---

## Dawn - Google's WebGPU Implementation

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/dawn/`

### Overview

Dawn is Google's open-source, cross-platform implementation of the WebGPU standard. It serves as the **reference implementation** for WebGPU and is the underlying engine powering WebGPU in Chromium-based browsers (Chrome, Edge, Opera).

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Dawn                                  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  WebGPU API Layer                     │   │
│  │              (webgpu.h implementation)                │   │
│  └──────────────────────────────────────────────────────┘   │
│                            │                                 │
│         ┌──────────────────┼──────────────────┐             │
│         ▼                  ▼                  ▼             │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐       │
│  │   D3D12     │   │    Metal    │   │   Vulkan    │       │
│  │   Backend   │   │   Backend   │   │   Backend   │       │
│  └─────────────┘   └─────────────┘   └─────────────┘       │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                    Tint                              │   │
│  │         (WGSL Shader Compiler/Translator)            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Key Components

| Component | Description |
|-----------|-------------|
| **webgpu.h** | C API header implementing WebGPU IDL |
| **D3D12 Backend** | Direct3D 12 implementation for Windows |
| **Metal Backend** | Metal implementation for macOS/iOS |
| **Vulkan Backend** | Vulkan implementation for Linux/Android |
| **OpenGL Backend** | Fallback implementation |
| **Tint** | WGSL shader compiler and translator |

### Directory Structure

```
dawn/
├── src/
│   ├── dawn/           # Core Dawn implementation
│   │   ├── native/     # Native API implementation
│   │   └── webgpu/     # WebGPU API layer
│   ├── tint/           # Tint shader compiler
│   └── utils/          # Utility libraries
├── include/
│   ├── dawn/           # Dawn C++ headers
│   └── webgpu/         # webgpu.h headers
├── docs/               # Documentation
├── tools/              # Build and testing tools
└── test/               # Test suites
```

### Tint Shader Compiler

Tint is a standalone shader compiler that can translate between shader languages:

```
┌─────────────────────────────────────────────────────────────┐
│                         Tint                                │
│                                                             │
│   Input Formats:          Output Formats:                  │
│   - WGSL                  - WGSL                           │
│   - SPIR-V                - SPIR-V                         │
│   - HLSL                  - HLSL                           │
│   - MSL (Metal)           - MSL (Metal)                    │
│                                                             │
│   Used by: Dawn, wgpu, and other WebGPU implementations   │
└─────────────────────────────────────────────────────────────┘
```

### Usage Example

```cpp
#include <dawn/native/DawnNative.h>
#include <dawn/webgpu.h>

int main() {
  // Initialize Dawn
  dawn::native::Instance instance;

  // Create surface
  wgpu::Surface surface = instance.CreateSurface(...);

  // Request adapter
  wgpu::Adapter adapter = instance.RequestAdapter(...);

  // Create device
  wgpu::Device device = adapter.CreateDevice();

  // Use WebGPU API...

  return 0;
}
```

### Status

- **Status:** Active Development
- **License:** BSD 3-Clause
- **Used By:** Chromium, Chrome, Edge, Electron
- **Website:** https://dawn.googlesource.com/dawn

---

## wgpu-native - Rust WebGPU FFI

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/src.gfx-rs/wgpu-native/`

### Overview

wgpu-native provides **C language bindings** for wgpu-core, enabling WebGPU functionality to be accessed from any language with C FFI support. It's a key part of the gfx-rs ecosystem for cross-language GPU access.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      wgpu-native                            │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                 wgpu.h / webgpu.h                    │   │
│  │               (C API Headers)                        │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  wgpu-core                          │   │
│  │            (Rust implementation)                    │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│         ┌──────────────────┼──────────────────┐            │
│         ▼                  ▼                  ▼            │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐      │
│  │   Vulkan    │   │    Metal    │   │   D3D12     │      │
│  │  (gfx-hal)  │   │  (gfx-hal)  │   │  (gfx-hal)  │      │
│  └─────────────┘   └─────────────┘   └─────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### Language Bindings

wgpu-native enables bindings for multiple languages:

| Language | Binding | Repository |
|----------|---------|------------|
| **Rust** | wgpu-rs | gfx-rs/wgpu |
| **Python** | wgpu-py | pygfx/wgpu-py |
| **.NET** | WGPU.NET | trivaxy/wgpu.NET |
| **Java** | wgpuj | kgpu/kgpu |
| **Go** | webgpu | go-webgpu/webgpu |
| **Julia** | WebGPU.jl | cshenton/WebGPU.jl |
| **Zig** | wgpu_native_zig | bronter/wgpu_native_zig |

### Building

```bash
# Clone repository
git clone https://github.com/gfx-rs/wgpu-native
cd wgpu-native

# Initialize submodules (webgpu-headers)
git submodule update --init

# Build
cargo build --release

# Output: target/release/libwgpu_native.*
```

### API Example

```c
#include <webgpu/webgpu.h>

#include <stdio.h>
#include <stdlib.h>

int main() {
  // Request adapter
  WGPUInstance* instance = wgpuCreateInstance(NULL);
  WGPUAdapter adapter;

  // Request adapter callback
  // ... adapter setup code ...

  // Create device
  WGPUDevice device = wgpuAdapterCreateDevice(adapter, NULL);

  // Create buffer
  WGPUBufferDescriptor bufferDesc = {
    .usage = WGPUBufferUsage_Vertex | WGPUBufferUsage_CopyDst,
    .size = 1024,
  };
  WGPUBuffer buffer = wgpuDeviceCreateBuffer(device, &bufferDesc);

  // ... more WebGPU calls ...

  return 0;
}
```

### Status

- **Status:** Active Development
- **License:** Apache 2.0 / MIT
- **MSRV:** Rust 1.82
- **Repository:** https://github.com/gfx-rs/wgpu-native

---

## gfx-rs Ecosystem

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/src.gfx-rs/`

### Overview

The gfx-rs organization is the original home of Rust GPU abstraction layers. Many projects have been moved or deprecated in favor of the wgpu ecosystem.

### Project Status Matrix

| Project | Status | Description |
|---------|--------|-------------|
| **gfx** | Deprecated | Original gfx-hal, maintenance only |
| **wgpu-native** | Active | Native WebGPU FFI |
| **metal-rs** | Deprecated | Use objc2-metal instead |
| **d3d12-rs** | Moved | Now in wgpu repository |
| **naga** | Moved | Now in wgpu repository |
| **gfx-render** | Archived | Old rendering framework |
| **gfx_gl** | Archived | OpenGL backend |

### Evolution Timeline

```
2015-2019: gfx-hal era
├── gfx (original HAL)
├── gfx-backend-vulkan
├── gfx-backend-metal
├── gfx-backend-dx12
└── gfx-backend-gl

2019-2022: wgpu era
├── wgpu-rs (safe Rust API)
├── wgpu-core (cross-backend)
└── wgpu-hal (new HAL)

2022-Present: Consolidation
├── gfx-hal → maintenance mode
├── metal-rs → deprecated
├── d3d12-rs → moved to wgpu
├── naga → moved to wgpu
└── wgpu-native → active FFI
```

### Naga - Shader Translation

**Status:** Moved to wgpu repository, still standalone usable

Naga is a **shader translation library** that converts between shader languages:

```
┌─────────────────────────────────────────────────────────────┐
│                         Naga                                │
│                                                             │
│  Frontends (Input):     Backends (Output):                 │
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │ WGSL            │    │ WGSL            │                │
│  │ GLSL            │    │ GLSL            │                │
│  │ HLSL            │    │ HLSL            │                │
│  │ SPIR-V          │    │ MSL (Metal)     │                │
│  │ Slang           │    │ SPIR-V          │                │
│  │                 │    │ SPIRV-Capabilities              │
│  └─────────────────┘    └─────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

**Usage Example (Rust):**

```rust
use naga::{front::wgsl, back::glsl};

fn translate_wgsl_to_glsl(wgsl_source: &str) -> String {
    // Parse WGSL
    let module = wgsl::parse_str(wgsl_source).unwrap();

    // Validate
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();

    // Generate GLSL
    let mut output = String::new();
    let mut writer = glsl::Writer::new(&mut output, &module, &options).unwrap();
    writer.write().unwrap();

    output
}
```

### metal-rs

**Status:** DEPRECATED

```
⚠️ WARNING: Use of this crate is deprecated as the `objc`
ecosystem of macOS system bindings are unmaintained.

For new development, use `objc2` and `objc2-metal` instead.
```

**Migration Path:**
```toml
# Old (deprecated)
[dependencies]
metal = "0.27"

# New (recommended)
[dependencies]
objc2 = "0.5"
objc2-metal = "0.2"
```

### Status

- **gfx-hal:** Maintenance mode (v0.9+)
- **License:** Apache 2.0 / MIT
- **Matrix Chat:** #gfx:matrix.org

---

## Vello - 2D GPU Renderer

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/vello/`
**Also:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/src.gfx-rs/src.velopath/`

### Overview

Vello is a **GPU compute-centric 2D renderer** written in Rust. It uses compute shaders and parallel algorithms to achieve high-performance 2D rendering, significantly outperforming traditional CPU-based renderers.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Vello                                │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │               Scene API                             │   │
│  │   (fill, stroke, layer, text, image, etc.)          │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              vello_encoding                         │   │
│  │         (Scene encoding & binning)                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              vello_shaders                          │   │
│  │        (Compute shaders for rendering)              │   │
│  │  - Prefix sum / Reduce                              │   │
│  │  - Path rasterization                               │   │
│  │  - Tile composition                                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                     wgpu                            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Key Innovations

**1. Prefix-Sum Algorithms**
Traditional 2D renderers use sequential algorithms for sorting and clipping. Vello parallelizes these using GPU compute:

```
Traditional (CPU):
  Sort paths → Clip → Rasterize → Composite  [Sequential]

Vello (GPU):
  Parallel binning → Parallel rasterization → Parallel compose
  (using prefix-sum for workload distribution)
```

**2. Compute-Centric Design**
All major rendering steps use compute shaders:
- Path tessellation
- Binning and clipping
- Tile-based rasterization
- Anti-aliasing

### Performance

| Scene | Resolution | FPS (M1 Max) |
|-------|------------|--------------|
| paris-30k | 1600x1600 | 177 FPS |
| Complex SVG | 1920x1080 | 60+ FPS |

### Usage Example

```rust
use vello::{
    kurbo::{Affine, Circle},
    peniko::{Color, Fill},
    *,
};

// Initialize wgpu
let device: wgpu::Device = /* ... */;
let queue: wgpu::Queue = /* ... */;

// Create renderer
let mut renderer = Renderer::new(
    &device,
    RendererOptions::default()
).expect("Failed to create renderer");

// Create scene
let mut scene = Scene::new();

// Draw filled circle
scene.fill(
    Fill::NonZero,
    Affine::IDENTITY,
    Color::from_rgb8(242, 140, 168),
    None,
    &Circle::new((420.0, 200.0), 120.0),
);

// Draw stroked path
scene.stroke(
    &Stroke::new(5.0),
    Affine::IDENTITY,
    Color::from_rgb8(100, 100, 255),
    None,
    &Rect::new(0.0, 0.0, 100.0, 100.0),
);

// Render to texture
renderer
    .render_to_texture(
        &device,
        &queue,
        &scene,
        &texture,
        &RenderParams {
            base_color: Color::BLACK,
            width,
            height,
            antialiasing_method: AaConfig::Msaa16,
        },
    )
    .expect("Failed to render");
```

### Project Structure

```
vello/
├── vello/              # Core renderer API
├── vello_encoding/     # Scene encoding
├── vello_shaders/      # Compute shaders
├── vello_tests/        # Test framework
├── sparse_strips/      # Experimental sparse rendering
│   └── vello_cpu/      # CPU fallback renderer
└── examples/           # Example applications
```

### Integrations

| Integration | Repository | Description |
|-------------|------------|-------------|
| **vello_svg** | linebender/vello_svg | SVG rendering |
| **velato** | linebender/velato | Lottie animation |
| **bevy_vello** | linebender/bevy_vello | Bevy engine integration |

### Status

- **Status:** Alpha (pre-1.0)
- **License:** Apache 2.0 / MIT
- **MSRV:** Rust 1.85
- **wgpu Version:** 24.0.1
- **Repository:** https://github.com/linebender/vello
- **Chat:** #vello on Linebender Zulip

---

## TypeGPU - Type-Safe WebGPU

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/TypeGPU/`

### Overview

TypeGPU is a **TypeScript library** that enhances the WebGPU API with type-safe resource management. It provides a declarative way to work with WebGPU resources while maintaining full compatibility with vanilla WebGPU.

### Key Features

1. **Type-Safe Resources** - Compile-time checking of buffer and texture types
2. **WGSL Mirroring** - TypeScript syntax that mirrors WGSL
3. **Interoperability** - Works alongside vanilla WebGPU
4. **JIT Compilation** - Runtime shader generation

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      TypeGPU                                │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              TypeScript API Layer                    │   │
│  │   (Type-safe wrappers for WebGPU resources)         │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  tgpu-jit                            │   │
│  │         (Just-In-Time shader compiler)              │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   WebGPU                             │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Repository Structure

```
TypeGPU/
├── packages/
│   ├── typegpu/              # Core library
│   ├── typegpu-color/        # Color utilities
│   ├── typegpu-noise/        # Noise functions
│   ├── tgpu-jit/             # JIT compiler
│   ├── tgpu-gen/             # Code generation CLI
│   ├── tgpu-wgsl-parser/     # WGSL parser
│   ├── unplugin-typegpu/     # Build plugin
│   └── tinyest/              # JavaScript AST types
├── apps/
│   └── typegpu-docs/         # Documentation site
└── examples/                 # Usage examples
```

### Usage Example

```typescript
import tgpu from 'typegpu';
import * as d from 'typegpu/data';

// Define a schema for your data
const ParticleData = d.struct({
  position: d.vec3f,
  velocity: d.vec3f,
  color: d.vec4f,
});

// Initialize TypeGPU
const root = await tgpu.init();

// Create typed buffer
const particleBuffer = root
  .createBuffer(d.arrayOf(ParticleData, 1000))
  .$usage('storage');

// Access raw WebGPU buffer when needed
const rawBuffer = root.unwrap(particleBuffer); // GPUBuffer

// Write shader code inline
const computeShader = tgpu.computeFn(
  'main',
  { workgroupSize: [64] as const },
  (input) => {
    const index = input.global_invocation_id.x;
    const particles = tgpu.buffer(particleBuffer);
    // ... compute logic
  }
);
```

### Library Interoperability

TypeGPU enables type-safe interoperability between WebGPU libraries:

```typescript
import tgpu from 'typegpu';
import gen from '@xyz/gen';      // Procedural generation library
import plot from '@abc/plot';    // Visualization library

const root = await tgpu.init();

// Generate heightmap (returns typed buffer)
const terrainBuffer = await gen.generateHeightMap(root, {
  width: 256,
  height: 256
});
// Type: TgpuBuffer<WgslArray<WgslArray<F32>>> & StorageFlag

// Type-safe visualization
plot.array2d(root, terrainBuffer);  // ✓ Correct type
plot.array1d(root, terrainBuffer);  // ✗ Type error!
```

### Projects Using TypeGPU

- **Chaos Master** - Procedural chaos simulation
- **Apollonian Circles** - Interactive geometry visualization
- **Strange Forms** - Generative art by Logan Zartman
- **WebGPU Stable Fluids** - Fluid simulation
- **Calm Jar** - Visual timer app (iOS)

### Status

- **Status:** Active Development
- **License:** MIT
- **Creator:** Software Mansion
- **Website:** https://docs.swmansion.com/TypeGPU
- **Repository:** https://github.com/software-mansion/TypeGPU

---

## Filament - PBR Rendering Engine

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/filament/`

### Overview

Filament is a **real-time physically based rendering (PBR) engine** developed by Google. While primarily a native C++ engine, it includes WebGL support and demonstrates modern rendering techniques applicable to WebGPU.

### Key Features

| Feature Category | Features |
|-----------------|----------|
| **Lighting** | Clustered forward renderer, HDR, IBL, Punctual lights |
| **Materials** | Cook-Torrance BRDF, Metallic workflow, Clear coat, Anisotropy |
| **Shadows** | Cascaded shadows, PCSS, EVSM, Contact shadows |
| **Post-Processing** | Bloom, DOF, Tone mapping, TAA, FXAA, FSR |
| **Effects** | SSAO, SSR, Screen-space refraction, Fog |

### Platform Support

| Platform | Backend |
|----------|---------|
| Android | Vulkan, OpenGL ES |
| iOS | Metal, OpenGL ES |
| macOS | Metal, OpenGL |
| Linux | Vulkan, OpenGL |
| Windows | D3D12, Vulkan, OpenGL |
| Web | WebGL 2.0 |

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Filament                              │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Engine (C++)                        │   │
│  │           (Core rendering engine)                    │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│         ┌──────────────────┼──────────────────┐            │
│         ▼                  ▼                  ▼            │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐      │
│  │   Vulkan    │   │    Metal    │   │    OpenGL   │      │
│  │  Backend    │   │   Backend   │   │   Backend   │      │
│  └─────────────┘   └─────────────┘   └─────────────┘      │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    matc                              │   │
│  │          (Material Compiler)                         │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Material System

Filament uses a custom material definition language compiled by `matc`:

```cpp
// Material definition
material {
    name : lit,
    shadingModel : lit,
    blending : fade,
    parameters : [
        baseColor,
        metallic,
        roughness,
    ],
};

fragment {
    void material(inout MaterialInputs material) {
        prepareMaterial(material);
        material.baseColor = texture(baseColorTex, getUV0());
        material.metallic = metallic;
        material.roughness = roughness;
    }
}
```

### Usage Example (C++)

```cpp
// Create engine and renderer
Engine* engine = Engine::create();
SwapChain* swapChain = engine->createSwapChain(nativeWindow);
Renderer* renderer = engine->createRenderer();

// Create view and scene
Camera* camera = engine->createCamera(EntityManager::get().create());
View* view = engine->createView();
Scene* scene = engine->createScene();

view->setCamera(camera);
view->setScene(scene);

// Create renderable
Material* material = Material::Builder()
    .package((void*) BAKED_MATERIAL_PACKAGE, sizeof(BAKED_MATERIAL_PACKAGE))
    .build(*engine);
MaterialInstance* materialInstance = material->createInstance();

Entity renderable = EntityManager::get().create();
RenderableManager::Builder(1)
    .boundingBox({{ -1, -1, -1 }, { 1, 1, 1 }})
    .material(0, materialInstance)
    .geometry(0, RenderableManager::PrimitiveType::TRIANGLES,
              vertexBuffer, indexBuffer, 0, 6)
    .build(*engine, renderable);
scene->addEntity(renderable);

// Render
if (renderer->beginFrame(swapChain)) {
    renderer->render(view);
    renderer->endFrame();
}
```

### Status

- **Status:** Production Ready
- **License:** Apache 2.0 / BSD
- **Creator:** Google
- **Repository:** https://github.com/google/filament
- **Documentation:** https://google.github.io/filament/

---

## Smelter - Video Composition

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/smelter/`

### Overview

Smelter is a **real-time video and audio composition toolkit** built with React and WebGPU. It enables low-latency multimedia composition for live streaming and video production.

### Features

- Combine multiple video sources
- Apply GPU-accelerated filters and effects
- Custom shaders for video processing
- React components for declarative composition
- Support for embedded websites and overlays

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Smelter                               │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              React Components                        │   │
│  │   <Compositor>, <Source>, <Effect>, <Output>        │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │            Compositor Pipeline                       │   │
│  │        (TypeScript/JavaScript logic)                │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │             WebGPU Renderer                          │   │
│  │        (GPU-accelerated composition)                │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                vk-video                             │   │
│  │         (Vulkan video coding library)               │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Repository Structure

```
smelter/
├── compositor_api/       # React API
├── compositor_chromium/  # Chromium integration
├── compositor_pipeline/  # Pipeline logic
├── compositor_render/    # Rendering engine
├── compositor_web/       # Web integration
├── vk-video/             # Vulkan video library
├── packages/             # NPM packages
└── demos/                # Example applications
```

### Usage Example (React)

```tsx
import { Compositor, Source, Effect } from '@smelter/core';

function LiveStream() {
  return (
    <Compositor output={{ width: 1920, height: 1080 }}>
      <Source
        id="camera"
        type="webcam"
        position={{ x: 0, y: 0 }}
      />
      <Source
        id="overlay"
        type="image"
        src="overlay.png"
        blendMode="multiply"
      />
      <Effect
        type="colorGrading"
        exposure={1.2}
        contrast={1.1}
      />
    </Compositor>
  );
}
```

### Status

- **Status:** Active Development
- **License:** MIT
- **Creator:** Software Mansion
- **Repository:** https://github.com/software-mansion/smelter
- **Website:** https://smelter.dev/

---

## Blitz - HTML/CSS Renderer

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/blitz/`

### Overview

Blitz is a **radically modular HTML/CSS rendering engine** written in Rust. It uses Vello as its rendering backend and aims to provide native HTML rendering for applications like documentation viewers, ebooks, and embedded web views.

### Goals

- Modern HTML layout (flexbox, grid, tables)
- Advanced CSS (selectors, media queries, variables)
- HTML form controls
- Accessibility via AccessKit
- Extensibility through custom widgets

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Blitz                                │
│                                                             │
│  High-Level Wrappers:                                       │
│  ┌─────────────┐  ┌─────────────────────────────────┐      │
│  │   blitz     │  │      dioxus-native              │      │
│  │ (Markdown)  │  │    (Dioxus Virtual DOM)         │      │
│  └─────────────┘  └─────────────────────────────────┘      │
│                                                             │
│  Core Modules:                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ blitz-dom   │  │ blitz-html  │  │ blitz-shell │        │
│  │ (DOM+Style) │  │  (Parsing)  │  │  (Window)   │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
│                                                             │
│  Rendering:                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │blitz-paint  │  │  anyrender  │  │   Vello     │        │
│  │             │  │             │  │   /wgpu     │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
│                                                             │
│  Dependencies: Stylo, Taffy, Parley, AccessKit             │
└─────────────────────────────────────────────────────────────┘
```

### Package Structure

| Package | Description |
|---------|-------------|
| **blitz** | Markdown/HTML frontend |
| **blitz-dom** | DOM abstraction with Stylo CSS |
| **blitz-html** | HTML parsing (html5ever) |
| **blitz-shell** | Windowing (winit, AccessKit) |
| **blitz-renderer-vello** | Vello rendering backend |
| **anyrender** | 2D drawing abstraction |
| **anyrender_vello** | Vello backend for anyrender |

### Dependencies

- **Stylo** - CSS parsing and resolution (from Servo)
- **Taffy** - Flexbox/Grid layout
- **Parley** - Text layout
- **AccessKit** - Accessibility
- **Winit** - Window management
- **Vello** - GPU rendering

### Status

- **Status:** Pre-alpha
- **License:** Apache 2.0 / MIT
- **Creator:** Dioxus Labs
- **Repository:** https://github.com/DioxusLabs/blitz

---

## TensorFlow.js WebGPU Backend

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/tfjs/`

### Overview

TensorFlow.js includes a **WebGPU backend** that accelerates machine learning operations in the browser. This enables training and inference using GPU compute shaders.

### Backend Comparison

| Backend | Performance | Compatibility | Use Case |
|---------|-------------|---------------|----------|
| **CPU** | Slowest | Universal | Small models, debugging |
| **WebGL** | Fast | Good | Medium models, broad support |
| **WebGPU** | Fastest | Limited | Large models, modern devices |
| **Node (TF)** | Fastest | Node.js | Server-side, full TF |

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   TensorFlow.js                             │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │               tfjs-core                              │   │
│  │            (Core Tensor API)                         │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│         ┌──────────────────┼──────────────────┐            │
│         ▼                  ▼                  ▼            │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐      │
│  │   CPU       │   │   WebGL     │   │   WebGPU    │      │
│  │  Backend    │   │   Backend   │   │   Backend   │      │
│  └─────────────┘   └─────────────┘   └─────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### Usage Example

```javascript
import * as tf from '@tensorflow/tfjs';
import '@tensorflow/tfjs-backend-webgpu';

// Set WebGPU backend
await tf.setBackend('webgpu');

// Create tensors
const a = tf.tensor2d([1, 2, 3, 4], [2, 2]);
const b = tf.tensor2d([5, 6, 7, 8], [2, 2]);

// Matrix multiplication (GPU accelerated)
const result = tf.matMul(a, b);
result.print();

// Run model inference
const model = await tf.loadLayersModel('model.json');
const prediction = model.predict(inputTensor);
```

### WebGPU-Specific Features

- **Compute Shaders** - General matrix operations
- **Storage Buffers** - Large tensor storage
- **Workgroups** - Parallel computation
- **FP16 Support** - Reduced precision for performance

### Status

- **Status:** Active Development
- **License:** Apache 2.0
- **Creator:** Google
- **Repository:** https://github.com/tensorflow/tfjs
- **Package:** @tensorflow/tfjs-backend-webgpu

---

## Canvas2D Spec Proposals

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.webgpu/canvas2D/`

### Overview

This repository contains **proposals for enhancing the Canvas 2D API**, including integration with WebGPU. The goal is to modernize the Canvas 2D specification with new features and better GPU integration.

### Active Proposals

| Proposal | Description | Status |
|----------|-------------|--------|
| **Layers** | Multi-layer canvas with compositing | In Development |
| **WebGPU Access** | Context switching between 2D and WebGPU | In Development |
| **Enhanced Text Metrics** | Better text measurement APIs | In Development |
| **Mesh2D** | Texture-mapped triangle rendering | In Development |

### Launched Features

| Feature | Description |
|---------|-------------|
| **Context Loss** | Canvas discard and restore |
| **willReadFrequently** | Performance hint for readback |
| **Text Modifiers** | CSS font properties on canvas |
| **RoundRect** | Rounded rectangle paths |
| **Conic Gradient** | Conic gradient fills |
| **Reset Function** | Canvas state reset |

### WebGPU Integration Proposal

```typescript
// Proposed API for WebGPU + Canvas2D integration
const canvas = document.getElementById('canvas');

// Get 2D context
const ctx = canvas.getContext('2d');

// Get WebGPU context from same canvas
const gpuContext = canvas.getContext('webgpu');

// Use 2D context for simple drawing
ctx.fillRect(0, 0, 100, 100);

// Use WebGPU for complex rendering
const device = await gpuContext.getDevice();
// ... WebGPU rendering ...

// Layers proposal for filter effects
const layer = ctx.createLayer();
layer.filter = 'blur(10px)';
layer.drawImage(image, 0, 0);
```

### Status

- **Status:** Ongoing Standardization
- **Organization:** W3C Web Platform
- **Repository:** Canvas 2D spec proposals
- **Related:** W3C WebGPU Working Group

---

## Summary Table

| Project | Language | Primary Use | Status |
|---------|----------|-------------|--------|
| Dawn | C++ | WebGPU implementation | Production |
| wgpu-native | Rust | WebGPU FFI | Active |
| gfx-hal | Rust | GPU abstraction | Maintenance |
| Vello | Rust | 2D rendering | Alpha |
| TypeGPU | TypeScript | Type-safe WebGPU | Active |
| Filament | C++ | PBR rendering | Production |
| Smelter | TypeScript | Video composition | Active |
| Blitz | Rust | HTML rendering | Pre-alpha |
| TensorFlow.js | JavaScript | ML inference | Production |
| Canvas2D | N/A | Web standards | Standardization |

---

*This document is part of the WebGPU Exploration series. See also: [exploration.md](./exploration.md), [webgpu-fundamentals.md](./webgpu-fundamentals.md), [rust-ecosystem.md](./rust-ecosystem.md), [rust-revision.md](./rust-revision.md)*
