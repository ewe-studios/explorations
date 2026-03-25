# Spline 3D Project - Comprehensive Exploration

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Spline3d/`

**Date:** 2026-03-25

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Project Structure Overview](#project-structure-overview)
3. [3D Modeling Fundamentals](#3d-modeling-fundamentals)
4. [Deep Dive Documents](#deep-dive-documents)
5. [Architecture Analysis](#architecture-analysis)
6. [Rust Replication Strategy](#rust-replication-strategy)

---

## Executive Summary

Spline is a collaborative 3D design tool for the web that provides a comprehensive ecosystem for creating, editing, and rendering 3D scenes. The project consists of multiple interconnected components:

- **Web Runtime**: JavaScript/TypeScript runtime with WASM support for 3D rendering
- **React Integrations**: Two packages (`react-spline` and `r3f-spline`) for React integration
- **iOS/macOS Native**: Swift framework for native Apple platform support
- **VTK Integration**: Visualization Toolkit for advanced scientific visualization
- **vtk-js**: JavaScript implementation of VTK for web-based rendering

### Key Technologies

| Component | Technology | Purpose |
|-----------|------------|---------|
| Rendering | WebGL, WebGPU | Browser-based 3D graphics |
| Runtime | WASM + JavaScript | High-performance execution |
| React | React Three Fiber | Declarative 3D scenes |
| iOS/macOS | Metal, Swift | Native Apple rendering |
| VTK | C++ → WASM | Scientific visualization |

---

## Project Structure Overview

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.Spline3d/
├── VTK/                    # C++ Visualization Toolkit source
│   ├── Rendering/          # Rendering pipeline
│   ├── Filters/            # Data filters and algorithms
│   ├── Common/             # Core utilities
│   ├── IO/                 # File format readers/writers
│   └── Interaction/        # User interaction handling
│
├── vtk-js/                 # JavaScript VTK implementation
│   ├── Sources/            # VTK class implementations
│   │   ├── Rendering/Core/ # Core rendering classes
│   │   ├── Filters/        # JavaScript filters
│   │   ├── IO/             # JavaScript IO
│   │   └── Interaction/    # JavaScript interaction
│   ├── Documentation/      # API docs and tutorials
│   └── Examples/           # Usage examples
│
├── react-spline/           # React component library
│   ├── src/
│   │   ├── Spline.tsx      # Main Spline component
│   │   ├── ParentSize.tsx  # Responsive container
│   │   └── next/           # Next.js SSR support
│   └── example/            # Example React app
│
├── r3f-spline/             # React Three Fiber hook
│   ├── src/
│   │   └── useSpline.ts    # Spline loading hook
│   └── example/            # R3F example app
│
├── spline-ios/             # iOS/macOS framework
│   └── SplineRuntime.xcframework/
│       ├── ios-arm64/      # iOS device binary
│       ├── ios-arm64-simulator/
│       ├── macos-arm64_x86_64/
│       └── visionOS/       # Apple Vision Pro support
│
└── 3D-startup-app/         # Example starter application
```

### File Statistics

- **Total files**: ~28,461
- **vtk-js**: Largest component with full VTK reimplementation
- **VTK (C++)**: Reference implementation with extensive filter library

---

## 3D Modeling Fundamentals

### Mesh Representation

Spline uses a **PolyData** structure for surface mesh representation:

```
PolyData Structure:
├── Points (vtkPoints)          # Vertex positions (x, y, z)
├── Verts (vtkCellArray)        # Point vertices
├── Lines (vtkCellArray)        # Polylines
├── Polys (vtkCellArray)        # Polygons (triangles, quads)
├── Strips (vtkCellArray)       # Triangle strips
├── PointData                   # Per-point attributes
│   ├── Scalars (colors, temperature)
│   ├── Vectors (normals, velocities)
│   └── TCoords (UV mapping)
└── CellData                    # Per-cell attributes
```

**JavaScript Representation:**
```javascript
{
  vtkClass: 'vtkPolyData',
  points: {
    vtkClass: 'vtkPoints',
    numberOfComponents: 3,
    dataType: 'Float32Array',
    values: new Float32Array([x0, y0, z0, x1, y1, z1, ...])
  },
  polys: {
    vtkClass: 'vtkCellArray',
    values: new Uint32Array([3, 0, 1, 2, 3, 3, 4, 5]) // 2 triangles
  }
}
```

### VTK Data Pipeline Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Source    │───>│   Filter    │───>│   Mapper    │───>│   Actor     │
│ (ConeSource)│    │ (Clip, Cut) │    │ (Geometry)  │    │ (Render)    │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
     │                  │                  │                  │
     ▼                  ▼                  ▼                  ▼
  vtkPolyData       vtkPolyData       vtkPolyData       vtkProp3D
   (Input)          (Processing)      (Rendering)       (Scene)
```

### Rendering Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                      Render Window                               │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                       Renderer                              │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │ │
│  │  │    Actor 1   │  │    Actor 2   │  │    Actor N   │      │ │
│  │  │   + Mapper   │  │   + Mapper   │  │   + Mapper   │      │ │
│  │  │   + Property │  │   + Property │  │   + Property │      │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘      │ │
│  │                        + Camera + Lights                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                    ┌─────────▼─────────┐                        │
│                    │   OpenGL/WebGL    │                        │
│                    │    (or WebGPU)    │                        │
│                    └───────────────────┘                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Deep Dive Documents

This exploration includes the following specialized documents:

| Document | Description |
|----------|-------------|
| [spline-algorithms.md](./spline-algorithms.md) | Bezier, B-Spline, NURBS algorithms |
| [vtk-integration.md](./vtk-integration.md) | VTK pipeline and architecture |
| [web-rendering.md](./web-rendering.md) | Three.js, WebGL, WASM rendering |
| [react-integration.md](./react-integration.md) | react-spline and r3f-spline integration |
| [ios-implementation.md](./ios-implementation.md) | Metal rendering and native iOS |
| [rust-revision.md](./rust-revision.md) | Rust implementation plan |
| [production-grade.md](./production-grade.md) | Production considerations |

---

## Architecture Analysis

### Component Interaction Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         Spline Editor                             │
│                    (Design Tool - Cloud)                          │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ Export (.splinecode)
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                      Spline Runtime                               │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    @splinetool/loader                       │  │
│  │              (Scene file parser + decoder)                  │  │
│  └────────────────────────────────────────────────────────────┘  │
│                              │                                    │
│              ┌───────────────┴───────────────┐                   │
│              ▼                               ▼                   │
│  ┌──────────────────────┐      ┌──────────────────────────┐     │
│  │   @splinetool/react  │      │   @splinetool/r3f-spline │     │
│  │   -spline            │      │                          │     │
│  │   - Canvas rendering │      │   - Three.js integration │     │
│  │   - Event handling   │      │   - React Three Fiber    │     │
│  │   - Object querying  │      │   - Scene graph access   │     │
│  └──────────────────────┘      └──────────────────────────┘     │
│              │                               │                   │
│              └───────────────┬───────────────┘                   │
│                              ▼                                    │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                   @splinetool/runtime                       │  │
│  │         (Core rendering engine + WASM modules)              │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐    │  │
│  │  │   WebGL     │  │   WebGPU    │  │      WASM       │    │  │
│  │  │  Renderer   │  │  Renderer   │  │   (Optional)    │    │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘    │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              │ Native Bindings
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                    SplineRuntime.xcframework                      │
│                  (iOS/macOS/visionOS Support)                     │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    Metal Rendering                          │  │
│  │              Touch Interaction Handling                     │  │
│  │                  Native Optimizations                       │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### React Integration Patterns

#### react-spline (Direct Canvas)
```jsx
import Spline from '@splinetool/react-spline';

function App() {
  return (
    <Spline
      scene="https://prod.spline.design/xxx/scene.splinecode"
      onLoad={(spline) => {
        const obj = spline.findObjectByName('Cube');
        obj.position.x += 10;
      }}
      onSplineMouseDown={(e) => console.log(e)}
    />
  );
}
```

#### r3f-spline (React Three Fiber)
```jsx
import useSpline from '@splinetool/r3f-spline';
import { Canvas } from '@react-three/fiber';

function Scene() {
  const { nodes, materials } = useSpline('url/scene.spline');

  return (
    <group dispose={null}>
      <mesh geometry={nodes.Cube.geometry} material={materials.Cube} />
    </group>
  );
}

function App() {
  return (
    <Canvas>
      <Suspense fallback={null}>
        <Scene />
      </Suspense>
    </Canvas>
  );
}
```

---

## Rust Replication Strategy

### Core Components to Implement

| Component | Rust Crate | Status |
|-----------|------------|--------|
| Scene Graph | Custom + nalgebra | - |
| Mesh Data | custom + cgmath/nalgebra | - |
| Spline Curves | `nurbs`, `bezier-rs` | Available |
| Rendering | `wgpu`, `nannou` | Available |
| WASM Export | `wasm-bindgen` | Available |
| React Integration | N/A (JS boundary) | Bridge needed |

### Recommended Crate Ecosystem

```toml
[dependencies]
# Math and geometry
nalgebra = "0.32"        # Linear algebra
cgmath = "0.18"          # Alternative math lib
bezier-rs = "0.4"        # Bezier curves
nurbs = "0.5"            # NURBS surfaces

# Rendering
wgpu = "0.19"            # GPU abstraction
winit = "0.29"           # Window management
nannou = "0.18"          # Creative coding framework

# WASM
wasm-bindgen = "0.2"     # JS interop
js-sys = "0.3"           # JS bindings
web-sys = "0.3"          # Web APIs

# Scene management
egui = "0.24"            # Immediate mode GUI
```

### Architecture Considerations

1. **Data Pipeline**: Implement VTK-style pipeline with Rust traits
2. **WASM First**: Design for WASM from the start
3. **React Bridge**: Create npm wrapper for Rust WASM module
4. **Metal Backend**: Use `wgpu` which supports Metal natively

---

## Key Findings

### Strengths of Current Architecture

1. **Multi-platform**: Web (WebGL/WebGPU), iOS (Metal), React integration
2. **Progressive Enhancement**: Works with or without WASM
3. **Developer Experience**: Simple React API, full scene graph access
4. **VTK Heritage**: Decades of visualization expertise
5. **Scene Export**: `.splinecode` format is compact and portable

### Areas for Improvement

1. **Bundle Size**: WASM adds ~2-5MB download
2. **React SSR**: Requires Next.js specific component
3. **TypeScript**: Partial type coverage in runtime
4. **Mobile Performance**: WASM startup latency on mobile

### Lessons for Rust Implementation

1. Design for WASM from day one
2. Provide both low-level (wgpu) and high-level (nannou) APIs
3. Create seamless JavaScript/TypeScript bindings
4. Support both direct rendering and React Three Fiber patterns

---

## References

- [Spline Documentation](https://docs.spline.design/)
- [VTK.js Documentation](https://kitware.github.io/vtk-js/docs/)
- [React Three Fiber](https://github.com/pmndrs/react-three-fiber)
- [Three.js](https://threejs.org/)
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)
