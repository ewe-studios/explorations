# PlayCanvas WebGL Game Engine - Comprehensive Exploration

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.playcanvas/`

**Date:** 2026-03-25

**Engine Version:** 2.18.0-beta.0

---

## Table of Contents

1. [Overview](#overview)
2. [Project Structure](#project-structure)
3. [Core Architecture](#core-architecture)
4. [Related Documents](#related-documents)

---

## Overview

PlayCanvas is an open-source WebGL/WebGPU game engine written in JavaScript. It's a fully-featured 3D engine used by thousands of developers worldwide for creating games, interactive experiences, AR/VR applications, and visualizations.

### Key Features

- **Graphics**: Advanced 2D + 3D graphics engine built on WebGL2 & WebGPU
- **Animation**: State-based animations for characters and arbitrary scene properties
- **Physics**: Full integration with Ammo.js (Bullet physics port)
- **Input**: Mouse, keyboard, touch, gamepad and VR controller APIs
- **Sound**: 3D positional sounds built on the Web Audio API
- **Assets**: Asynchronous streaming system built on glTF 2.0, Draco and Basis compression
- **Scripts**: Write game behaviors in TypeScript or JavaScript

### Major Companies Using PlayCanvas

Animech, Arm, BMW, Disney, Facebook, King, Miniclip, Mozilla, Nickelodeon, Samsung, Snap, Zynga, and many more.

---

## Project Structure

The PlayCanvas monorepo contains multiple projects:

```
src.playcanvas/
├── engine/                    # Core engine source code
│   ├── src/
│   │   ├── core/             # Core utilities, math, data structures
│   │   ├── platform/         # Platform-specific code (graphics, audio, input)
│   │   │   ├── graphics/     # Graphics API abstraction (WebGL/WebGPU)
│   │   │   ├── input/        # Input handling
│   │   │   ├── sound/        # Audio system
│   │   │   └── net/          # Networking
│   │   ├── scene/            # Scene graph, rendering, materials, shaders
│   │   └── framework/        # High-level components and application framework
│   ├── test/                 # Unit tests
│   └── examples/             # Example projects
│
├── editor/                    # PlayCanvas Editor (web-based IDE)
├── editor-api/                # Editor API
├── editor-test/               # Editor test infrastructure
├── editor-mcp-server/         # Editor MCP Server
│
├── api-reference/             # API documentation
├── developer-site/            # User manual and tutorials
├── blog/                      # Blog posts
│
├── create-playcanvas/         # CLI tool for creating new projects
├── pcui/                      # PlayCanvas UI component library
├── pcui-graph/                # PCUI graph editor
├── react/                     # React integration
│
├── ammo.js/                   # Physics engine (Bullet port to WASM)
├── basis_universal/           # Texture compression
├── attribute-parser/          # GLTF attribute parser
├── canvas-mock/               # Canvas mock for testing
│
├── supersplat/                # 3D content creation tool
├── supersplat-viewer/         # Viewer for SuperSplat content
├── splat-transform/           # Gaussian splat transformation tools
│
├── vscode-extension/          # VSCode integration
└── ... (various tools and utilities)
```

---

## Core Architecture

### Module Dependency Hierarchy

The codebase follows a strict hierarchical structure:

```
core → platform → scene → framework
```

**Rules:**
- Lower-level modules cannot import from higher-level modules
- `core/` cannot import from `platform/`, `scene/`, or `framework/`
- `platform/` cannot import from `scene/` or `framework/`
- `scene/` cannot import from `framework/`
- Exception: `CameraComponent` (from `framework/`) is used in some `scene/` places

### Engine Build System

```javascript
// Build configurations from package.json
npm run build           # Build all flavors
npm run build:release   # Production build
npm run build:debug     # Debug build with extra checks
npm run build:profiler  # Build with profiling support
npm run build:types     # Generate TypeScript declarations
```

The engine uses:
- **Rollup** for bundling
- **SWC** for fast transpilation
- **TypeScript** for type definitions (generated from JSDoc)
- **ES Modules** as the module system

---

## Related Documents

This exploration includes the following in-depth documents:

| Document | Description |
|----------|-------------|
| [ecs-architecture.md](./ecs-architecture.md) | Entity-Component-System pattern implementation |
| [rendering-engine.md](./rendering-engine.md) | WebGL/WebGPU rendering pipeline |
| [animation-system.md](./animation-system.md) | Skeletal animation, morph targets, state machines |
| [physics-system.md](./physics-system.md) | Ammo.js physics integration |
| [editor-architecture.md](./editor-architecture.md) | Editor implementation details |
| [asset-pipeline.md](./asset-pipeline.md) | Asset loading, compression, management |
| [rust-revision.md](./rust-revision.md) | Rust replication plan using wgpu, bevy, etc. |
| [production-grade.md](./production-grade.md) | Production considerations and optimizations |

---

## Quick Start Example

```javascript
import * as pc from 'playcanvas';

const canvas = document.createElement('canvas');
document.body.appendChild(canvas);

const app = new pc.Application(canvas);

// Create box entity
const box = new pc.Entity('cube');
box.addComponent('model', { type: 'box' });
app.root.addChild(box);

// Create camera
const camera = new pc.Entity('camera');
camera.addComponent('camera', { clearColor: new pc.Color(0.1, 0.2, 0.3) });
app.root.addChild(camera);
camera.setPosition(0, 0, 3);

// Create light
const light = new pc.Entity('light');
light.addComponent('light');
app.root.addChild(light);

// Update loop
app.on('update', dt => box.rotate(10 * dt, 20 * dt, 30 * dt));

app.start();
```

---

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | JavaScript (ES2022) with JSDoc |
| Module System | ES Modules |
| Build System | Rollup + SWC |
| Testing | Mocha + Chai + Sinon |
| Linting | ESLint |
| Graphics APIs | WebGL 2.0, WebGPU |
| Physics | Ammo.js (Bullet WASM port) |
| Asset Formats | glTF 2.0, Draco, Basis Universal |
| Minimum Node.js | 18.0.0 |

---

## Key Design Principles

1. **Performance-Critical**: Object pooling, minimal allocations in hot paths, typed arrays
2. **Multi-Backend**: Both WebGL2 and WebGPU supported with abstraction layers
3. **API Stability**: Proper deprecation cycles, backward compatibility
4. **Documentation**: Comprehensive JSDoc for TypeScript type generation
5. **Modular Architecture**: Clean separation between core, platform, scene, and framework layers
