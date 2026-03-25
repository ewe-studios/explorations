# Rive Project: Comprehensive Exploration

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/`

**Output Directory:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/src.rive/`

---

## Table of Contents

1. [Overview](#overview)
2. [Project Structure](#project-structure)
3. [Related Deep-Dive Documents](#related-deep-dive-documents)
4. [Quick Reference](#quick-reference)

---

## Overview

Rive is a real-time interactive design and animation tool that provides a collaborative editor for creating motion graphics that respond to different states and user inputs. The runtime libraries are lightweight and open-source, allowing animations to be loaded into apps, games, and websites.

### Key Components

- **Vector Rendering Engine**: Custom-built GPU-accelerated renderer supporting Vulkan, Metal, D3D11/12, OpenGL/WebGL
- **Animation System**: State machines, linear animations, blend trees, and event-driven transitions
- **File Format**: Binary `.riv` format optimized for runtime loading
- **Cross-Platform Runtimes**: C++ core with bindings for Web (WASM), iOS, Android, Flutter, React, React Native, Unity, Unreal

### Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                     Rive Designer (Editor)                      │
│                        (Creates .riv files)                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ .riv binary file
┌─────────────────────────────────────────────────────────────────┐
│                    Rive Runtime (C++ Core)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ File Loader  │  │  Artboard    │  │  Animation System    │  │
│  │  & Parser    │──│  Hierarchy   │──│  State Machines      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│         │                │                      │               │
│         ▼                ▼                      ▼               │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Rendering Engine (GPU Abstraction)          │   │
│  │  Vulkan │ Metal │ D3D11 │ D3D12 │ OpenGL │ WebGL │ WGSL │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
   ┌──────────┐         ┌──────────┐         ┌──────────┐
   │  WASM    │         │  iOS     │         │ Android  │
   │  (Web)   │         │ (Metal)  │         │(Vulkan/  │
   └──────────┘         └──────────┘         │  OpenGL) │
                                              └──────────┘
```

---

## Project Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/
├── rive-runtime/           # C++ core runtime library
│   ├── src/                # Core implementation
│   │   ├── animation/      # Animation system (state machines, keyframes)
│   │   ├── shapes/         # Path, shape, mesh definitions
│   │   ├── renderer.cpp    # Renderer abstraction
│   │   ├── artboard.cpp    # Artboard hierarchy management
│   │   └── math/           # Math utilities (vectors, matrices)
│   ├── renderer/           # GPU rendering implementations
│   │   ├── src/
│   │   │   ├── rive_renderer.cpp   # Main renderer implementation
│   │   │   ├── gr_triangulator.cpp # Path tessellation
│   │   │   ├── draw.cpp            # Draw command processing
│   │   │   ├── gpu.cpp             # GPU resource management
│   │   │   ├── gradient.cpp        # Gradient rendering
│   │   │   └── shaders/            # GPU shader programs
│   │   ├── vulkan/         # Vulkan backend
│   │   ├── metal/          # Metal backend
│   │   ├── d3d11/          # Direct3D 11 backend
│   │   ├── d3d12/          # Direct3D 12 backend
│   │   ├── gl/             # OpenGL/WebGL backend
│   │   └── webgpu/         # WebGPU backend
│   ├── tess/               # Tessellation library
│   ├── tests/              # Unit tests (Catch2)
│   └── include/            # Public headers
│
├── rive-wasm/              # WebAssembly bindings
│   ├── wasm/               # WASM C++ bindings (Emscripten)
│   │   ├── src/
│   │   │   ├── bindings.cpp        # Main WASM bindings
│   │   │   ├── bindings_c2d.cpp    # Canvas 2D bindings
│   │   │   ├── bindings_webgl2.cpp # WebGL2 bindings
│   │   │   └── bindings_skia.cpp   # Skia bindings
│   │   └── examples/       # Example applications
│   └── js/                 # JavaScript/TypeScript API layer
│
├── rive-rs/                # Rust runtime (uses Vello renderer)
│   ├── rive-rs/src/
│   │   ├── ffi.rs          # FFI bindings to C++
│   │   ├── ffi.cpp         # C++ FFI implementation
│   │   ├── file.rs         # File loading
│   │   ├── artboard/       # Artboard bindings
│   │   ├── linear_animation.rs
│   │   ├── state_machine/
│   │   └── vello/          # Vello renderer integration
│   └── examples/
│
├── rive-ios/               # iOS runtime (Swift/Objective-C)
├── rive-android/           # Android runtime (Kotlin/Java)
├── rive-flutter/           # Flutter bindings
├── rive-react/             # React bindings
├── rive-react-native/      # React Native bindings
├── rive-unity/             # Unity game engine integration
├── rive-unreal/            # Unreal Engine integration
│
├── rive-docs/              # Documentation
│   ├── runtimes/           # Runtime-specific docs
│   ├── editor/             # Editor documentation
│   └── api-reference/      # API reference
│
├── MoltenVK/               # Vulkan-to-Metal translation layer
├── harfbuzz/               # Text shaping library
├── gifski/                 # GIF encoding library
└── animations/             # Sample animation files
```

---

## Related Deep-Dive Documents

This exploration includes the following detailed documents:

| Document | Description |
|----------|-------------|
| [`rendering-engine-deep-dive.md`](./rendering-engine-deep-dive.md) | Vector rendering, GPU pipeline, rasterization |
| [`animation-system-deep-dive.md`](./animation-system-deep-dive.md) | State machines, interpolation, timelines |
| [`vector-graphics-algorithms.md`](./vector-graphics-algorithms.md) | Path tessellation, curve rendering, anti-aliasing |
| [`wasm-web-rendering.md`](./wasm-web-rendering.md) | WASM architecture, JavaScript bindings |
| [`cpp-core-architecture.md`](./cpp-core-architecture.md) | C++ implementation, memory model, threading |
| [`rust-revision.md`](./rust-revision.md) | Complete Rust translation plan |
| [`production-grade.md`](./production-grade.md) | Production readiness checklist |
| [`storage-system-guide.md`](./storage-system-guide.md) | File format, storage/loading (for beginners) |

---

## Quick Reference

### Key Classes (C++ Runtime)

| Class | File | Purpose |
|-------|------|---------|
| `Artboard` | `src/artboard.cpp` | Root container for all scene objects |
| `LinearAnimation` | `src/animation/linear_animation.cpp` | Timeline-based keyframe animation |
| `StateMachine` | `src/animation/state_machine.cpp` | State machine for interactive animations |
| `StateMachineInstance` | `src/animation/state_machine_instance.cpp` | Runtime state machine execution |
| `Path` | `src/shapes/path.cpp` | Vector path with Bezier curves |
| `Shape` | `src/shapes/shape.cpp` | Renderable shape with fills/strokes |
| `RiveRenderer` | `renderer/src/rive_renderer.cpp` | Main renderer interface |
| `GrTriangulator` | `renderer/src/gr_triangulator.hpp` | Path tessellation engine |

### File Format (.riv)

The `.riv` format is a binary format containing:
- Header with version and metadata
- Object table with type IDs and properties
- Artboard hierarchy definitions
- Animation keyframes and state machines
- Embedded assets (images, fonts, audio)

### Supported Graphics APIs

| API | Status | Platform |
|-----|--------|----------|
| Vulkan | Production | Windows, Linux, Android |
| Metal | Production | iOS, macOS |
| D3D11 | Production | Windows |
| D3D12 | Production | Windows |
| OpenGL | Production | Cross-platform |
| WebGL 2 | Production | Web browsers |
| WebGPU | In development | Modern browsers |

### Build System

- **Primary**: Premake5 (generates platform-specific build files)
- **WASM**: Emscripten toolchain
- **Testing**: Catch2 framework

---

## Key Insights

### 1. Rendering Architecture

Rive uses a **retained-mode** rendering approach with immediate-mode command recording:
- Scene graph is retained (artboard hierarchy)
- Draw commands are recorded each frame into GPU buffers
- Uses **Path Level Rendering (PLS)** for correct transparency with overlapping paths

### 2. Animation System

- **Linear Animations**: Traditional keyframe-based animations with interpolation
- **State Machines**: Hierarchical state machines with blend trees
- **Listeners**: Event-driven system for runtime interaction
- **Data Binding**: Reactive property system connecting UI to animations

### 3. Path Rendering Pipeline

```
Path Definition (Bezier curves)
       │
       ▼
Tessellation (vertices & indices)
       │
       ▼
GPU Rasterization (fragment shaders)
       │
       ▼
Frame Buffer (with PLS for transparency)
```

### 4. Memory Model

- C++ runtime uses **reference counting** (`rcp<>` smart pointers)
- Objects organized in **arenas** for efficient allocation
- WASM uses **linear memory** with explicit imports/exports

### 5. Cross-Platform Strategy

- **Single C++ core** shared across all platforms
- **Thin bindings** per platform (minimal abstraction overhead)
- **Renderer abstraction layer** allows swapping GPU backends

---

## Getting Started

### Building the C++ Runtime

```bash
cd rive-runtime
export PATH="$PATH:$(realpath build)"
build_rive.sh release
```

### Building for Web

```bash
cd rive-wasm
./build_wasm.sh release
# Serve at localhost:5555
```

### Using Rust Runtime

```bash
cd rive-rs
git submodule update --init
cargo run --release
```

---

## Additional Resources

- **Official Docs**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/rive-docs/`
- **Community Runtimes**: Various community-maintained bindings
- **File Specification**: See `storage-system-guide.md` for .riv format details

---

*This exploration was generated from the Rive source repositories. For the most up-to-date information, refer to the official Rive documentation at https://rive.app/docs/*
