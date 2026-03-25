# Rive Rendering Engine: Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/rive-runtime/renderer/`

---

## Table of Contents

1. [Overview](#overview)
2. [Graphics Pipeline Architecture](#graphics-pipeline-architecture)
3. [Vector Rendering Fundamentals](#vector-rendering-fundamentals)
4. [Path to Pixels: The Rendering Pipeline](#path-to-pixels-the-rendering-pipeline)
5. [GPU Backend Implementations](#gpu-backend-implementations)
6. [MoltenVK and Cross-API Translation](#moltenvk-and-cross-api-translation)
7. [Performance Optimizations](#performance-optimizations)

---

## Overview

The Rive rendering engine is a custom-built vector and raster graphics renderer designed specifically for real-time animation playback. It supports multiple GPU APIs (Vulkan, Metal, D3D11/12, OpenGL/WebGL) through a unified abstraction layer.

### Design Goals

1. **Real-time Performance**: 60+ FPS animation playback
2. **Cross-Platform**: Single codebase targeting all major GPU APIs
3. **Quality**: Reference-quality vector rendering matching the editor
4. **Memory Efficiency**: Minimal allocations during rendering

### Key Files

| File | Purpose |
|------|---------|
| `renderer/src/rive_renderer.cpp` | Main renderer implementation (~2,300 lines) |
| `renderer/src/draw.cpp` | Draw command processing (~2,700 lines) |
| `renderer/src/gpu.cpp` | GPU resource management (~1,800 lines) |
| `renderer/src/gr_triangulator.cpp` | Path tessellation (~2,000 lines) |
| `renderer/src/gr_triangulator.hpp` | Tessellator header with algorithms |
| `renderer/src/gradient.cpp` | Gradient rendering |
| `renderer/src/intersection_board.cpp` | Spatial acceleration structure |

---

## Graphics Pipeline Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Application Layer                               │
│                    (Artboard, Shapes, Animations)                       │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       RiveRenderer (Abstract)                           │
│  - save()/restore()           - transform(matrix)                       │
│  - clipPath(path)             - drawPath(path, paint)                   │
│  - drawImage(image)           - modulateOpacity(alpha)                  │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      RenderContext (GPU Manager)                        │
│  - Frame management         - Resource allocation                       │
│  - Command buffer recording - Pipeline state objects                    │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
           ┌───────────────────────┼───────────────────────┐
           ▼                       ▼                       ▼
    ┌──────────────┐        ┌──────────────┐       ┌──────────────┐
    │   Vulkan     │        │    Metal     │       │   D3D11/12   │
    │  Backend     │        │   Backend    │       │   Backend    │
    └──────────────┘        └──────────────┘       └──────────────┘
           │                       │                       │
           └───────────────────────┼───────────────────────┘
                                   ▼
                    ┌──────────────────────────────┐
                    │      GPU Hardware            │
                    └──────────────────────────────┘
```

### Render State Stack

The renderer maintains a stack of render states for save/restore operations:

```cpp
struct RenderState {
    Mat2D matrix;                    // Current transformation
    float modulatedOpacity;          // Accumulated opacity (0-1)
    size_t clipStackHeight;          // Number of active clips
    bool clipIsEmpty;                // Early-out for empty clips
};
```

### Key Rendering Functions

```cpp
// From rive_renderer.cpp
void RiveRenderer::drawPath(RenderPath* renderPath, RenderPaint* renderPaint)
{
    // 1. Early rejection tests
    if (path->getRawPath().empty()) return;
    if (m_stack.back().clipIsEmpty) return;

    // 2. Handle feathered fills
    if (paint->getFeather() != 0 && !paint->getIsStroked()) {
        // Soften path edges for glow effects
        clipAndPushDraw(gpu::PathDraw::Make(...softened copy...));
        return;
    }

    // 3. Queue draw command
    clipAndPushDraw(gpu::PathDraw::Make(
        m_context,
        m_stack.back().matrix,
        ref_rcp(path),
        path->getFillRule(),
        paint,
        m_stack.back().modulatedOpacity,
        &m_scratchPath
    ));
}
```

---

## Vector Rendering Fundamentals

### What is Vector Rendering?

Vector rendering converts mathematical descriptions of shapes (paths, curves) into pixel values on a screen. Unlike raster graphics (stored as pixels), vector graphics are stored as geometric primitives.

### Path Representation

A path in Rive consists of:
- **Vertices**: Points in 2D space
- **Commands**: Move, Line, Cubic Bezier, Close
- **Fill Rule**: Clockwise or Even-Odd for determining interior

```
Path Example:
┌─────────────────────────────────────┐
│  MoveTo(10, 10)                     │
│  LineTo(100, 10)                    │
│  CubicTo(150, 50, 150, 150, 100, 100)  │
│  LineTo(10, 100)                    │
│  Close()                            │
└─────────────────────────────────────┘

Visual representation:
    (10,10) ──────────── (100,10)
       │                      │
       │                      │
       │                      ╲
       │                       ╲
    (10,100) ──────────── (100,100)
                          (control points at 150,50 and 150,150)
```

### Bezier Curves

Rive uses **cubic Bezier curves** defined by 4 points:
- P0: Start point
- P1: First control point
- P2: Second control point
- P3: End point

**Formula:**
```
B(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃
where t ∈ [0, 1]
```

```
Control Point Visualization:

    P₁ ●━━━━━━━● P₀
       ╱       ╱
      ╱       ╱
     ╱       ╱
    ╱       ╱
   ●━━━━━━━●
  P₂       P₃

The curve starts at P₀ heading toward P₁,
and ends at P₃ coming from P₂.
```

---

## Path to Pixels: The Rendering Pipeline

The complete rendering pipeline from vector paths to screen pixels:

### Stage 1: Path Building

```cpp
// From src/shapes/path.cpp
void Path::buildPath(RawPath& rawPath) const
{
    // Iterate through vertices
    for (size_t i = 1; i < length; i++) {
        auto vertex = vertices[i];

        if (vertex->is<CubicVertex>()) {
            // Emit cubic Bezier command
            rawPath.cubic(out, inPoint, translation);
        } else {
            // Handle straight vertices (with optional radius)
            rawPath.line(out);
        }
    }
    rawPath.close();
}
```

### Stage 2: Flattening/Tessellation

Curves must be converted to triangles for GPU rendering. Rive uses the **GrTriangulator** class.

```
Bezier Curve Flattening:

Original curve:
    ●━━━━━━━━●
   ╱          ╲
  ╱            ╲
 ●              ●

Flattened (tolerance = 2 pixels):
    ●───●───●───●
   ╱    │   │   ╲
  ╱     │   │    ╲
 ●──────●───●─────●

More segments = better quality but more triangles
```

### Stage 3: Triangulation Algorithm

The tessellator uses a **sweep-line algorithm**:

```
Algorithm Steps:

1. Linearize contours into piecewise linear segments
2. Build mesh of edges connecting vertices
3. Sort vertices (merge sort by Y, then X)
4. Simplify mesh (insert vertices at intersections)
5. Tessellate into monotone polygons
6. Triangulate polygons into vertex buffer

From gr_triangulator.hpp:
```
```cpp
// There are six stages to the basic algorithm:
// 1) pathToContours() - Linearize path contours
// 2) contoursToMesh() - Build edge mesh
// 3) SortMesh() - Sort vertices in Y (then X)
// 4) simplify() - Insert vertices at edge intersections
// 5) tessellate() - Create monotone polygons
// 6) polysToTriangles() - Generate final triangles
```

### Stage 4: GPU Upload

Triangles are uploaded to GPU buffers:

```cpp
// GPU triangle vertex format
struct TriangleVertex {
    float x, y;      // Position
    uint16_t pathID; // Path identifier for PLS
    int16_t weight;  // Winding weight for fill rules
};
```

### Stage 5: Rasterization

GPU fragment shaders determine pixel coverage:

```
Pixel Coverage Test:

   ┌───────┬───────┬───────┐
   │       │ ████  │       │  ████ = triangle covers
   │       │ ████  │       │  this pixel
   │       │ ████  │       │
   ├───────┼───────┼───────┤
   │ ████  │ ████  │       │
   │ ████  │ ████  │ ████  │
   │       │       │       │
   ├───────┼───────┼───────┤
   │       │       │       │
   │       │       │ ████  │
   │       │       │       │
   └───────┴───────┴───────┘
```

### Stage 6: Path Level Rendering (PLS)

For correct transparency with overlapping paths, Rive uses **Path Level Rendering**:

```
Without PLS (incorrect):
  Path A (50% opacity) overlaps Path B (50% opacity)
  Overlap region shows incorrect blending

With PLS:
  Each path is tracked in a per-pixel linked list
  Compositing respects true path hierarchy
  Result matches vector editor exactly
```

---

## GPU Backend Implementations

### Supported Backends

| Backend | Directory | Status |
|---------|-----------|--------|
| Vulkan | `renderer/src/vulkan/` | Production |
| Metal | `renderer/src/metal/` | Production |
| D3D11 | `renderer/src/d3d11/` | Production |
| D3D12 | `renderer/src/d3d12/` | Production |
| OpenGL/WebGL | `renderer/src/gl/` | Production |
| WebGPU | `renderer/src/webgpu/` | Development |

### Vulkan Backend

Key files in `renderer/src/vulkan/`:
- Vulkan context creation
- Pipeline state objects (PSOs)
- Descriptor set management
- Command buffer recording

### Metal Backend

Key files in `renderer/src/metal/`:
- MTLDevice and MTLCommandBuffer management
- Shader function compilation
- Resource heap allocation

### Shader Pipeline

Shaders are written in a portable format and translated:

```
Portable Shader IR
       │
       ├──► GLSL (OpenGL/WebGL)
       ├──► MSL (Metal)
       ├──► HLSL (D3D11/12)
       ├──► SPIR-V (Vulkan)
       └──► WGSL (WebGPU - future)
```

---

## MoltenVK and Cross-API Translation

### What is MoltenVK?

MoltenVK is a Vulkan-to-Metal translation layer that allows Vulkan applications to run on Apple platforms (iOS, macOS) where Metal is the only native GPU API.

```
┌─────────────────────────────────────┐
│         Vulkan Application          │
│         (Rive Runtime)              │
└─────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│           MoltenVK                  │
│  (Vulkan API → Metal commands)      │
└─────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│            Metal                    │
│    (Apple GPU API)                  │
└─────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│      Apple GPU Hardware             │
└─────────────────────────────────────┘
```

### Why Use MoltenVK?

1. **Single Codebase**: Write Vulkan, run everywhere
2. **Feature Parity**: Access Vulkan features on Metal
3. **Maintenance**: One renderer implementation, not two

### MoltenVK in Rive

```bash
# MoltenVK is built separately
cd rive-runtime/renderer
./make_moltenvk.sh
```

---

## Performance Optimizations

### 1. Batch Rendering

Multiple paths are batched into single draw calls:

```cpp
// Command batching in draw.cpp
class CommandBatch {
    std::vector<PathDraw> pathDraws;
    std::vector<ImageDraw> imageDraws;

    void flush() {
        // Submit all commands at once
        renderContext->submit(*this);
    }
};
```

### 2. GPU Resource Reuse

Buffers and textures are reused across frames:

```
Frame N:   Allocate buffers → Render → Mark for reuse
Frame N+1: Reuse buffers     → Render → Mark for reuse
Frame N+2: Reuse buffers     → Render → ...
```

### 3. Early Rejection

Multiple levels of early-out testing:

```cpp
bool shouldDraw() {
    if (path->empty()) return false;           // Empty path
    if (clipIsEmpty) return false;             // Clipped out
    if (opacity < 0.01f) return false;         // Nearly transparent
    if (boundsOutsideViewport()) return false; // Off-screen
    return true;
}
```

### 4. SIMD Operations

Vector math uses SIMD instructions:

```cpp
// From rive_renderer.cpp - AABB detection
float4 corners = {pts[0].x, pts[0].y, pts[2].x, pts[2].y};
float4 oppositeCorners = {pts[1].x, pts[1].y, pts[3].x, pts[3].y};

// Compare 4 floats in single instruction
if (simd::all(corners == oppositeCorners.zyxw)) {
    // It's a rectangle!
}
```

### 5. Tessellation Caching

Tessellated paths are cached when possible:

```
Path (unchanged) → Tessellate → Cache
                        │
                        ▼
              Next frame: use cache ✓

Path (modified) → Tessellate → Update cache
```

### 6. Scissor Testing

GPU scissor rectangles reduce overdraw:

```
┌─────────────────────────────┐
│    Viewport (full screen)   │
│  ┌─────────────────────┐    │
│  │  Scissor Rect       │    │
│  │  (only render here) │    │
│  │  ┌───────────┐      │    │
│  │  │  Content  │      │    │
│  │  └───────────┘      │    │
│  └─────────────────────┘    │
└─────────────────────────────┘
```

---

## Debugging and Profiling

### RenderDoc Integration

```bash
# Enable RenderDoc capture
cd rive-runtime/renderer
# Build with RenderDoc support
```

### Profiling Macros

```cpp
// Use RIVE_PROF_SCOPE() for timing
void renderFrame() {
    RIVE_PROF_SCOPE("Frame Render")
    // ... rendering code ...
}
```

### Disassembly Explorer

For examining generated assembly:

```bash
# VSCode task: "disassemble"
# Shows optimized CPU code paths
```

---

## Summary

The Rive rendering engine is a sophisticated GPU-accelerated vector renderer that:

1. **Abstracts GPU APIs** through a unified interface
2. **Tessellates paths** into triangles using sweep-line algorithms
3. **Renders with PLS** for correct transparency
4. **Optimizes aggressively** through batching, caching, and early rejection
5. **Supports all platforms** via Vulkan/MoltenVK or native backends

For implementation details, see:
- `vector-graphics-algorithms.md` - Path rendering algorithms
- `wasm-web-rendering.md` - Web-specific rendering
- `cpp-core-architecture.md` - Core C++ architecture
