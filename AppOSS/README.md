# AppOSS Exploration Documentation Index

This directory contains comprehensive documentation for the AppOSS (Applications Open Source) collection - a set of 24+ open-source applications for workflow automation, design, low-code development, and graphics.

---

## Document Overview

### Core Documents

| Document | Lines | Description |
|----------|-------|-------------|
| [`00-zero-to-apposs.md`](./00-zero-to-apposs.md) | 898 | Beginner's guide from zero knowledge to understanding AppOSS projects. Covers JavaScript/TypeScript, backends, databases, graphics fundamentals, and deployment. |
| [`exploration.md`](./exploration.md) | 1,582 | Main exploration document covering n8n, baserow, Penpot, Budibase, Skia and more. Architecture diagrams, component breakdowns, entry points. |
| [`rust-revision.md`](./rust-revision.md) | 1,057 | Complete guide to building Rust equivalents. Project structure, data structures, rendering engines (Vello, Skia), server setup, WASM bindings. |
| [`production-grade.md`](./production-grade.md) | 1,202 | Production readiness guide. Architecture patterns, databases, caching, real-time collaboration, security, observability, Kubernetes deployment. |
| [`storage-system-guide.md`](./storage-system-guide.md) | 738 | Storage system guide for inexperienced engineers. From file storage to PostgreSQL, Redis caching, S3, backups, and scaling. |

### Deep Dive Documents

| Document | Lines | Description |
|----------|-------|-------------|
| [`deep-dives/graphics-rendering-deep-dive.md`](./deep-dives/graphics-rendering-deep-dive.md) | 1,056 | Vector graphics fundamentals, path rendering, rasterization, GPU pipelines, text rendering, image processing, animation, optimization techniques. |
| [`deep-dives/wasm-web-rendering-deep-dive.md`](./deep-dives/wasm-web-rendering-deep-dive.md) | 738 | WebAssembly for graphics. Emscripten, wasm-bindgen, CanvasKit, Penpot WASM architecture, Rive runtime, performance optimization. |
| [`deep-dives/vector-graphics-algorithms.md`](./deep-dives/vector-graphics-algorithms.md) | 932 | Mathematical foundations, Bezier curves, arc parameterization, path operations (boolean), tessellation, SVG processing, path simplification. |

### Examples

| Document | Lines | Description |
|----------|-------|-------------|
| [`examples/vector-graphics-examples.md`](./examples/vector-graphics-examples.md) | 492 | Practical code examples for drawing shapes, custom paths, gradients, transforms, boolean operations, path effects, text rendering, SVG import/export, hit testing, animation. |

---

## Total Statistics

- **Total Documents:** 10
- **Total Lines:** ~8,700
- **Source Projects:** 24+ applications in `/home/darkvoid/Boxxed/@formulas/src.AppOSS/`

---

## Learning Path

### For Beginners

1. Start with [`00-zero-to-apposs.md`](./00-zero-to-apposs.md)
2. Read [`storage-system-guide.md`](./storage-system-guide.md) for database fundamentals
3. Browse [`exploration.md`](./exploration.md) for project overviews

### For Graphics Engineers

1. Read [`deep-dives/vector-graphics-algorithms.md`](./deep-dives/vector-graphics-algorithms.md) for fundamentals
2. Study [`deep-dives/graphics-rendering-deep-dive.md`](./deep-dives/graphics-rendering-deep-dive.md) for rendering pipelines
3. Review [`deep-dives/wasm-web-rendering-deep-dive.md`](./deep-dives/wasm-web-rendering-deep-dive.md) for WebAssembly
4. Practice with [`examples/vector-graphics-examples.md`](./examples/vector-graphics-examples.md)

### For Rust Developers

1. Start with [`rust-revision.md`](./rust-revision.md) for project structure
2. Read [`production-grade.md`](./production-grade.md) for scaling patterns
3. Study deep-dives for graphics-specific patterns

### For Production Deployment

1. Read [`production-grade.md`](./production-grade.md) cover-to-cover
2. Review security, observability, and deployment sections
3. Follow the production checklist

---

## Source Projects Covered

### Workflow Automation
- n8n
- automatisch

### No-Code/Low-Code Platforms
- baserow
- Budibase
- Appsmith

### Design & Graphics
- Penpot
- Skia
- OpenPencil
- CanvasKit

### AI/ML Tools
- BrowserAI
- BentoML
- Open WebUI

### Desktop Applications
- opcode
- layrr

---

## Related Documentation

See the parent directory for other project explorations:
- `../src.rive/` - Rive animation runtime
- `../src.process-compose/` - Process orchestration
- `../tldraw/` - Whiteboard/drawing tool

---

## Contributing

To update this documentation:
1. Ensure changes follow the existing structure
2. Update line counts in this index
3. Update `../tasks.md` with completion status
