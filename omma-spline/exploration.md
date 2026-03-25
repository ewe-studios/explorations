---
location: https://omma.build, https://spline.design
repository: N/A - Third-party web applications
explored_at: 2026-03-26
language: JavaScript/TypeScript, WebGL/WebGPU, Python (ML backend)
---

# OMMA & Spline.design Combined Exploration

## Overview

This exploration covers two leading web-based 3D platforms:

1. **OMMA (omma.build)**: AI-powered text-to-3D generative modeling platform
2. **Spline.design**: Web-based 3D design tool with Figma-like interface

Both represent the cutting edge of browser-based 3D content creation, demonstrating what's possible with modern web technologies.

## Repository

- **OMMA**: https://omma.build (AI 3D generation)
- **Spline**: https://spline.design (3D design tool)
- **Primary Languages**: TypeScript, WebGL/WebGPU, Python (ML backend)
- **License**: Proprietary

## Directory Structure

```
omma-spline/
├── exploration.md                    # This file - main summary
├── omma-analysis.md                  # Deep dive into OMMA platform
├── spline-design-analysis.md         # Deep dive into Spline.design
└── rust-wasm-webgpu-replication-guide.md  # Implementation guide
```

---

## Key Differences

| Aspect | OMMA | Spline.design |
|--------|------|---------------|
| **Purpose** | AI 3D generation | Manual 3D design |
| **Input** | Text prompts | Visual editing |
| **Output** | Generated meshes | Designed scenes |
| **Technology** | ML + WebGL | Custom WebGL renderer |
| **Learning Curve** | Minutes | Hours |
| **Control** | Limited (prompt-based) | Complete (manual) |
| **Best For** | Rapid prototyping | Precise design work |

---

## Architecture Comparison

### OMMA Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      OMMA Platform                          │
├─────────────────────────────────────────────────────────────┤
│  Frontend (Browser)         │  Backend (GPU Cluster)        │
│  ┌─────────────────────┐    │  ┌─────────────────────────┐  │
│  │  Prompt Input       │────┼──►  Text Encoder           │  │
│  │  3D Viewer          │    │  │  (CLIP-like)            │  │
│  │  (WebGL Canvas)     │    │  │                         │  │
│  │  Iteration Controls │    │  ▼                         │  │
│  └─────────────────────┘    │  Diffusion Model            │  │
│                             │  (Score Distillation)       │  │
│                             │                             │  │
│                             │  ▼                         │  │
│                             │  3D Generator              │  │
│                             │  (NeRF / Mesh)             │  │
│                             │                             │  │
│                             │  ▼                         │  │
│                             │  Texture Generator         │  │
│                             │  (SD-based)                │  │
│                             │                             │  │
│                             │  ▼                         │  │
│                             │  glTF Export               │  │
│                             └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Spline.design Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Spline Platform                          │
├─────────────────────────────────────────────────────────────┤
│  Editor (Browser)                                           │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Canvas Area          │  Properties Panel             │  │
│  │  ┌─────────────────┐  │  ┌─────────────────────────┐  │  │
│  │  │  WebGL Canvas   │  │  │  Transform              │  │  │
│  │  │  (Custom        │  │  │  Material               │  │  │
│  │  │   Renderer)     │  │  │  Animation              │  │  │
│  │  └─────────────────┘  │  │  Interactions           │  │  │
│  │                       │  └─────────────────────────┘  │  │
│  ├───────────────────────┴───────────────────────────────┤  │
│  │  Timeline / Animation Panel                           │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │  Keyframes  ─●────●─────────●────               │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  Scene Graph (DAG) ─► Custom WebGL Renderer ─► GPU         │
└─────────────────────────────────────────────────────────────┘
```

---

## Rendering Technology

### WebGL Implementation Patterns

Both platforms use WebGL for browser-based 3D rendering:

#### Common Techniques

1. **Instanced Rendering**: For repeated objects (trees, chairs, etc.)
2. **Level of Detail (LOD)**: Distance-based mesh simplification
3. **Occlusion Culling**: Skip hidden objects
4. **Texture Atlasing**: Combine textures to reduce draw calls
5. **Batch Rendering**: Group objects by material

#### Spline's Custom Renderer

Spline likely built a custom renderer rather than using Three.js:

**Reasons:**
- Full control over performance optimization
- Custom features (interactive states, animations)
- Easier WebGPU migration path
- Smaller bundle size (only needed features)

```typescript
// Conceptual Spline renderer architecture
class SplineRenderer {
    private gl: WebGL2RenderingContext;
    private sceneGraph: SceneGraph;
    private materialSystem: MaterialSystem;

    render(scene: Scene, camera: Camera): void {
        // Sort by material for batch rendering
        const renderQueue = this.sortByMaterial(scene.objects);

        // Batch draw calls
        for (const batch of renderQueue) {
            this.bindMaterial(batch.material);
            this.drawBatch(batch);
        }
    }
}
```

### WebGPU Migration Path

Both platforms benefit from WebGPU:

| Feature | WebGL | WebGPU |
|---------|-------|--------|
| API Type | Immediate | Modern (like Vulkan/Metal) |
| Performance | Good | Excellent |
| Compute Shaders | Limited | Full support |
| Multi-threading | Restricted | Better support |
| Memory Control | Limited | Explicit |

---

## Animation Systems

### Keyframe-Based Animation

Both platforms use keyframe animation:

```typescript
interface Keyframe<T> {
    time: number;
    value: T;
    interpolation: 'linear' | 'bezier' | 'step';
}

class AnimationTrack {
    keyframes: Keyframe[];

    sample(time: number): any {
        // Interpolate between keyframes
        const [before, after] = this.findKeyframes(time);
        const t = (time - before.time) / (after.time - before.time);
        return this.interpolate(before.value, after.value, t);
    }
}
```

### Interpolation Types

1. **Linear**: Straight interpolation
2. **Bezier**: Smooth curves with handles (Spline uses this heavily)
3. **Step**: Discrete changes

```typescript
// Cubic Bezier for smooth animation curves
function cubicBezier(t, p0, p1, p2, p3) {
    const mt = 1 - t;
    return (
        mt*mt*mt * p0 +
        3*mt*mt*t * p1 +
        3*mt*t*t * p2 +
        t*t*t * p3
    );
}
```

---

## Interaction System

### Raycasting

Both platforms need mouse-to-3D interaction:

```typescript
class Raycaster {
    createRay(screenX, screenY) {
        // Convert to NDC (-1 to 1)
        const ndcX = (screenX / width) * 2 - 1;
        const ndcY = -(screenY / height) * 2 + 1;

        // Unproject to world space
        const near = this.unproject(ndcX, ndcY, 0);
        const far = this.unproject(ndcX, ndcY, 1);

        return new Ray(near, far.subtract(near).normalize());
    }

    // Moller-Trumbore ray-triangle intersection
    intersectTriangle(ray, tri) {
        // ... intersection algorithm
    }
}
```

### Event System

```typescript
interface InteractionEvent {
    type: 'click' | 'hover' | 'mousedown' | 'mouseup';
    target: SceneObject;
    position: Vector3;
    normal: Vector3;
    uv: Vector2;
}

class InteractionManager {
    handlePointerMove(x, y) {
        const ray = this.raycaster.createRay(x, y);
        const hits = this.raycaster.intersectObjects(ray);

        if (hits.length > 0) {
            this.dispatchEvent('hover', hits[0]);
        }
    }
}
```

---

## AI 3D Generation (OMMA)

### Text-to-3D Pipeline

OMMA uses state-of-the-art ML models:

```
Text Prompt ──► Text Encoder ──► Diffusion ──► 3D Mesh ──► Texture ──► glTF
                    │               │
                    ▼               ▼
                CLIP-like      Score
                embedding      Distillation
```

### Key ML Technologies

1. **DreamFusion**: Score Distillation Sampling (SDS)
2. **Magic3D**: Fast two-stage generation
3. **Shap-E**: Direct 3D generation
4. **TripoSR**: Fast reconstruction

### Backend Pipeline (Python)

```python
class TextTo3DPipeline:
    def generate(self, prompt, num_steps=100):
        # 1. Multi-view image generation
        views = self.generate_multiview(prompt, num_steps)

        # 2. 3D reconstruction
        mesh = self.reconstruct_from_views(views)

        # 3. Geometry optimization
        mesh = self.optimize_geometry(mesh, prompt)

        # 4. Texture generation
        texture = self.generate_texture(mesh, views)

        return mesh, texture
```

---

## Rust+WASM+WebGPU Replication

See `rust-wasm-webgpu-replication-guide.md` for comprehensive implementation details.

### Two Approaches

1. **Standard wasm-bindgen**: Use established ecosystem
2. **foundation_wasm**: Custom no_std binding generator

### Key Components

```rust
// Scene graph
pub struct SceneGraph {
    nodes: Vec<SceneNode>,
    root_id: NodeId,
}

// Renderer
pub struct WebGPURenderer {
    device: GPUDevice,
    pipelines: Vec<RenderPipeline>,
}

// Animation
pub struct AnimationSystem {
    clips: Vec<AnimationClip>,
    players: Vec<AnimationPlayer>,
}

// Interaction
pub struct InteractionManager {
    raycaster: Raycaster,
    event_handlers: EventHandlers,
}
```

---

## Performance Considerations

### Browser Performance

| Technique | Impact | Complexity |
|-----------|--------|------------|
| Instanced Rendering | High | Low |
| Frustum Culling | Medium | Medium |
| LOD System | High | Medium |
| Texture Atlas | Medium | Low |
| Batch Rendering | High | Medium |

### Memory Management

```typescript
// Object pooling for frequently created/destroyed objects
class ObjectPool<T> {
    private freeList: T[] = [];

    acquire(): T {
        return this.freeList.pop() || this.create();
    }

    release(obj: T) {
        this.freeList.push(obj);
    }
}
```

---

## Key Insights

### OMMA

1. **AI-First Paradigm**: Represents shift from manual to AI-assisted 3D creation
2. **Accessibility**: Minutes to learn vs. months for traditional tools
3. **Backend Complexity**: Heavy ML infrastructure (GPU clusters, diffusion models)
4. **Use Case**: Rapid prototyping, placeholder assets, concept visualization

### Spline.design

1. **Browser-First**: Full 3D editor running in browser
2. **Custom Renderer**: Raw WebGL for control and performance
3. **Figma-like UX**: Familiar interface for web designers
4. **Real-time Collaboration**: Multi-user editing

### Common Patterns

1. **WebGL/WebGPU**: Hardware-accelerated browser rendering
2. **Scene Graph**: Transform hierarchy (DAG)
3. **Keyframe Animation**: Industry-standard animation approach
4. **Raycasting**: Precise 3D interaction
5. **glTF Export**: Standard 3D format compatibility

---

## Open Questions

1. **OMMA's Exact Stack**: Proprietary ML pipeline details not public
2. **Spline's Renderer**: Custom implementation details not documented
3. **Performance Limits**: Maximum scene complexity for browser-based tools
4. **WebGPU Adoption**: Timeline for production WebGPU deployment

---

## Sources

- [DreamFusion Paper](https://arxiv.org/abs/2204.01145)
- [Magic3D (NVIDIA)](https://research.nvidia.com/labs/3dgen/)
- [OpenAI Shap-E](https://openai.com/research/shap-e)
- [TripoSR](https://triposr.github.io/)
- [Spline Documentation](https://docs.spline.design/)
- [WebGL Fundamentals](https://webglfundamentals.org/)
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)
- [glTF Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
