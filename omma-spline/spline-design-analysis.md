---
location: https://spline.design
repository: N/A - Third-party web application
explored_at: 2026-03-26
language: JavaScript/TypeScript, WebGL/WebGPU
---

# Spline.design Deep Analysis

## 1. What is Spline?

### 1.1 Overview

Spline is a web-based 3D design tool that allows users to create interactive 3D experiences directly in the browser. Unlike traditional desktop 3D software (Blender, Maya, Cinema 4D), Spline runs entirely in the browser with a Figma-like interface designed for web designers and developers.

### 1.2 Core Capabilities

- **Browser-Based 3D Modeling**: Create and edit 3D objects without desktop software
- **Real-Time Collaboration**: Multi-user editing similar to Figma
- **Interactive States**: Define object behaviors and interactions
- **Animation System**: Keyframe-based animation with timeline
- **Export & Integration**: Export to glTF, USDZ, or embed via iframe/React component
- **Game-like Interactions**: Event system for clicks, hovers, collisions

### 1.3 Target Use Cases

- **Web Design**: 3D headers, product showcases, interactive elements
- **Prototyping**: Quick 3D mockups for presentations
- **Landing Pages**: Immersive 3D experiences for marketing
- **UI/UX Design**: Adding 3D elements to interfaces
- **Educational Content**: Interactive 3D visualizations

---

## 2. Rendering Technology

### 2.1 WebGL/WebGPU Architecture

Spline uses a custom WebGL renderer with potential WebGPU adoption for improved performance.

```
┌─────────────────────────────────────────────────────────────┐
│                      Spline Application                      │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐   │
│  │   Editor    │  │   Scene     │  │   Asset          │   │
│  │   (Canvas   │  │   Graph     │  │   Library        │   │
│  │    + UI)    │  │   (Nodes)   │  │   (Models,       │   │
│  │             │  │             │  │    Materials)    │   │
│  └─────────────┘  └─────────────┘  └───────────────────┘   │
│                            │                                 │
│  ┌─────────────────────────▼─────────────────────────────┐  │
│  │              Spline Custom Renderer                    │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │   WebGL     │  │   Scene     │  │   Material    │  │  │
│  │  │   Context   │  │   Graph     │  │   System      │  │  │
│  │  │   Manager   │  │   (DAG)     │  │               │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │   Geometry  │  │   Lighting  │  │   Post-       │  │  │
│  │  │   Builder   │  │   Engine    │  │   Processing  │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  └─────────────────────────┬─────────────────────────────┘  │
└────────────────────────────│────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────┐
│                       GPU Hardware                          │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐   │
│  │   Vertex    │  │   Fragment  │  │   Transform       │   │
│  │   Pipeline  │  │   Pipeline  │  │   Feedback        │   │
│  └─────────────┘  └─────────────┘  └───────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Custom WebGL Renderer

Spline likely built a custom renderer rather than using Three.js directly for several reasons:

1. **Performance Control**: Direct WebGL access allows fine-tuned optimization
2. **Feature Set**: Custom features not available in Three.js
3. **Bundle Size**: Only include needed functionality
4. **WebGPU Migration Path**: Easier transition to WebGPU

```typescript
// Conceptual representation of Spline's renderer architecture
class SplineRenderer {
    private gl: WebGL2RenderingContext;
    private sceneGraph: SceneGraph;
    private materialSystem: MaterialSystem;
    private lightingEngine: LightingEngine;

    // Custom shader system
    private shaderLibrary: Map<string, ShaderProgram>;

    render(scene: Scene, camera: Camera): void {
        // Sort objects by material for batch rendering
        const renderQueue = this.sortByMaterial(scene.objects);

        // Batch draw calls by material
        for (const batch of renderQueue) {
            this.bindMaterial(batch.material);
            this.drawBatch(batch);
        }
    }
}
```

### 2.3 Key Rendering Features

#### Instanced Rendering
For performance with repeated objects:
```typescript
// Instanced drawing for trees, chairs, etc.
gl.drawArraysInstanced(
    gl.TRIANGLES,
    0,
    vertexCount,
    instanceCount
);
```

#### Level of Detail (LOD)
```typescript
class LODSystem {
    selectLOD(mesh: Mesh, distance: number): Geometry {
        if (distance < 10) return mesh.highDetail;
        if (distance < 50) return mesh.mediumDetail;
        return mesh.lowDetail;
    }
}
```

#### Occlusion Culling
```typescript
class OcclusionCuller {
    isVisible(mesh: Mesh, camera: Camera): boolean {
        // Frustum culling
        if (!this.inFrustum(mesh.boundingBox, camera.frustum)) {
            return false;
        }
        // Occlusion testing
        return !this.isOccluded(mesh);
    }
}
```

---

## 3. Animation System

### 3.1 Keyframe-Based Animation

Spline uses a keyframe animation system similar to traditional animation software:

```typescript
interface Keyframe {
    time: number;        // Timeline position in seconds
    value: number | Vector3 | Quaternion;
    interpolation: 'linear' | 'bezier' | 'step';
    easing?: EasingFunction;
}

interface AnimationTrack {
    targetPath: string;  // e.g., "mesh.position.x"
    keyframes: Keyframe[];
}

class AnimationClip {
    name: string;
    duration: number;
    tracks: AnimationTrack[];

    sample(time: number): AnimationState {
        const state = new AnimationState();
        for (const track of this.tracks) {
            const value = this.interpolateTrack(track, time);
            state.setValue(track.targetPath, value);
        }
        return state;
    }
}
```

### 3.2 Timeline System

```
┌─────────────────────────────────────────────────────────────┐
│                     Animation Timeline                       │
├─────────────────────────────────────────────────────────────┤
│  [◄◄] [►] [►►]  [● Rec]   Time: 00:02:34 / 00:10:00        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Object1.Transform                                           │
│  ├── Position ─●────●─────────●────                          │
│  ├── Rotation ───●────●────●─────                            │
│  └── Scale     ────●──────────●──                            │
│                                                              │
│  Object2.Material                                            │
│  └── Opacity ──●─────────●─────────●──                       │
│                                                              │
│  └─────────────────────────────────────────────────────────  │
│  0:00    2:00    4:00    6:00    8:00    10:00               │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 Interpolation Algorithms

#### Linear Interpolation
```typescript
function lerp(a: number, b: number, t: number): number {
    return a + (b - a) * t;
}

function lerpVector3(a: Vector3, b: Vector3, t: number): Vector3 {
    return new Vector3(
        lerp(a.x, b.x, t),
        lerp(a.y, b.y, t),
        lerp(a.z, b.z, t)
    );
}
```

#### Cubic Bezier Interpolation
```typescript
// Used by Spline for smooth animation curves
function cubicBezier(
    t: number,
    p0: number,
    p1: number,
    p2: number,
    p3: number
): number {
    const t2 = t * t;
    const t3 = t2 * t;
    const mt = 1 - t;
    const mt2 = mt * mt;
    const mt3 = mt2 * mt;

    return (
        mt3 * p0 +
        3 * mt2 * t * p1 +
        3 * mt * t2 * p2 +
        t3 * p3
    );
}

class BezierCurve {
    constructor(
        public p0: Vector2, // Start point
        public p1: Vector2, // Control point 1
        public p2: Vector2, // Control point 2
        public p3: Vector2  // End point
    ) {}

    sample(t: number): number {
        // Find x = t on curve, return y value
        // Requires numerical solving
    }
}
```

---

## 4. Interaction System

### 4.1 Event System Architecture

```typescript
interface InteractionEvent {
    type: 'click' | 'hover' | 'mousedown' | 'mouseup' | 'scroll';
    target: SceneObject;
    position: Vector3;
    normal: Vector3;
    uv: Vector2;
}

class InteractionManager {
    private raycaster: Raycaster;
    private eventListeners: Map<string, Set<EventHandler>>;

    handlePointerMove(x: number, y: number): void {
        const ray = this.raycaster.createRay(x, y);
        const hits = this.raycaster.intersectObjects(ray, this.scene.objects);

        if (hits.length > 0) {
            this.dispatchEvent('hover', hits[0]);
        }
    }

    handleClick(x: number, y: number): void {
        const ray = this.raycaster.createRay(x, y);
        const hits = this.raycaster.intersectObjects(ray, this.scene.objects);

        if (hits.length > 0) {
            this.dispatchEvent('click', hits[0]);
        }
    }
}
```

### 4.2 Raycasting Implementation

```typescript
class Raycaster {
    createRay(screenX: number, screenY: number): Ray {
        // Convert screen coordinates to NDC
        const ndcX = (screenX / canvasWidth) * 2 - 1;
        const ndcY = -(screenY / canvasHeight) * 2 + 1;

        // Unproject to world space
        const nearPoint = this.unproject(ndcX, ndcY, 0);
        const farPoint = this.unproject(ndcX, ndcY, 1);

        return new Ray(nearPoint, farPoint.subtract(nearPoint).normalize());
    }

    intersectMesh(ray: Ray, mesh: Mesh): HitTest | null {
        // Triangle intersection using Moller-Trumbore algorithm
        for (const triangle of mesh.triangles) {
            const hit = this.rayTriangleIntersect(ray, triangle);
            if (hit) return hit;
        }
        return null;
    }

    rayTriangleIntersect(ray: Ray, tri: Triangle): HitTest | null {
        const EPSILON = 1e-7;
        const edge1 = tri.v1.subtract(tri.v0);
        const edge2 = tri.v2.subtract(tri.v0);
        const h = ray.direction.cross(edge2);
        const a = edge1.dot(h);

        if (Math.abs(a) < EPSILON) return null;

        const f = 1 / a;
        const s = ray.origin.subtract(tri.v0);
        const u = f * s.dot(h);

        if (u < 0 || u > 1) return null;

        const q = s.cross(edge1);
        const v = f * ray.direction.dot(q);

        if (v < 0 || u + v > 1) return null;

        const t = f * edge2.dot(q);

        if (t > EPSILON) {
            return new HitTest(
                ray.at(t),
                t,
                tri.normal
            );
        }
        return null;
    }
}
```

---

## 5. Scene Graph Architecture

### 5.1 Transform Hierarchy

```typescript
class Transform {
    private localPosition: Vector3 = new Vector3(0, 0, 0);
    private localRotation: Quaternion = Quaternion.identity();
    private localScale: Vector3 = new Vector3(1, 1, 1);

    private worldPosition: Vector3 = new Vector3(0, 0, 0);
    private worldRotation: Quaternion = Quaternion.identity();
    private worldScale: Vector3 = new Vector3(1, 1, 1);

    private parent: Transform | null = null;
    private children: Transform[] = [];

    get worldMatrix(): Matrix4 {
        if (this.parent && !this.parent.isDirty) {
            return this.parent.worldMatrix.multiply(this.localMatrix);
        }
        return this.localMatrix;
    }

    updateWorldTransform(): void {
        this.isDirty = false;
        for (const child of this.children) {
            child.updateWorldTransform();
        }
    }
}

class SceneObject extends Transform {
    mesh?: Mesh;
    material?: Material;
    collider?: Collider;
    light?: Light;
    camera?: Camera;
}
```

### 5.2 Scene Graph Visualization

```
Scene Root
├── Camera (Main)
│   └── (view frustum defines visible area)
├── DirectionalLight (Sun)
├── Environment
│   ├── Skybox
│   └── AmbientLight
├── Building
│   ├── Walls
│   │   ├── Wall_North
│   │   ├── Wall_South
│   │   ├── Wall_East
│   │   └── Wall_West
│   ├── Roof
│   └── Windows[] (instanced)
├── Trees[] (instanced, animated)
│   └── (wind animation via vertex shader)
└── InteractiveObjects
    ├── Button (click event)
    └── Door (hover + click events)
```

---

## 6. Export Pipeline

### 6.1 Export Formats

```typescript
interface ExportOptions {
    format: 'gltf' | 'glb' | 'usdz' | 'fbx';
    quality: 'high' | 'medium' | 'low';
    animations: boolean;
    includeLights: boolean;
    bakeLighting: boolean;
}

class ExportPipeline {
    async export(scene: Scene, options: ExportOptions): Promise<Blob> {
        switch (options.format) {
            case 'gltf':
                return this.exportGLTF(scene, options);
            case 'usdz':
                return this.exportUSDZ(scene, options);
        }
    }

    private exportGLTF(scene: Scene, options: ExportOptions): Blob {
        const gltf = {
            asset: { version: '2.0', generator: 'Spline' },
            scenes: [this.convertScene(scene)],
            scene: 0,
            nodes: this.convertNodes(scene),
            meshes: this.convertMeshes(scene),
            materials: this.convertMaterials(scene),
            textures: this.convertTextures(scene),
            images: this.convertImages(scene),
            samplers: this.convertSamplers(),
            animations: options.animations ? this.convertAnimations(scene) : [],
            accessors: this.convertAccessors(scene),
            bufferViews: this.convertBufferViews(scene),
            buffers: [this.convertBuffer(scene)]
        };
        return new Blob([JSON.stringify(gltf)], { type: 'model/gltf+json' });
    }
}
```

### 6.2 Runtime Integration

```typescript
// React integration pattern
import { SplineScene } from '@splinetool/react-spline';

function App() {
    return (
        <SplineScene
            scene="https://prod.spline.design/scene-url"
            onLoad={(scene) => {
                // Access scene API
                const object = scene.findObjectByName('MyObject');
                object.addEventListener('click', () => {
                    console.log('Clicked!');
                });
            }}
        />
    );
}
```

---

## 7. Performance Optimization

### 7.1 Draw Call Batching

```typescript
class BatchRenderer {
    private batches: Map<Material, RenderBatch> = new Map();

    addToBatch(object: Renderable, material: Material): void {
        if (!this.batches.has(material)) {
            this.batches.set(material, new RenderBatch(material));
        }
        this.batches.get(material).add(object);
    }

    flush(): void {
        for (const [material, batch] of this.batches) {
            this.bindMaterial(material);
            this.drawInstanced(batch.instances);
        }
        this.batches.clear();
    }
}
```

### 7.2 Texture Atlas

```typescript
class TextureAtlas {
    private atlasCanvas: HTMLCanvasElement;
    private regions: Map<string, TextureRegion> = new Map();

    addTexture(name: string, image: HTMLImageElement): TextureRegion {
        // Pack texture into atlas using bin-packing algorithm
        const region = this.binPack(image);
        this.blit(image, region.x, region.y);
        this.regions.set(name, region);
        return region;
    }

    getUVs(name: string): Float32Array {
        const region = this.regions.get(name);
        return new Float32Array([
            region.uMin, region.vMin,
            region.uMax, region.vMin,
            region.uMax, region.vMax,
            region.uMin, region.vMax
        ]);
    }
}
```

---

## 8. WebGPU Migration Path

Spline is well-positioned to migrate to WebGPU:

```typescript
// Current WebGL
const gl = canvas.getContext('webgl2');
gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
gl.vertexAttribPointer(location, size, type, normalized, stride, offset);

// WebGPU equivalent
const device = await adapter.requestDevice();
const vertexBuffer = device.createBuffer({
    size: vertexData.byteLength,
    usage: GPUBufferUsage.VERTEX,
    mappedAtCreation: true
});
new Float32Array(vertexBuffer.getMappedRange()).set(vertexData);
vertexBuffer.unmap();

// Vertex buffer layout in pipeline
const vertexBuffers: GPUVertexBufferLayout[] = [{
    arrayStride: 12,
    attributes: [{
        shaderLocation: 0,
        offset: 0,
        format: 'float32x3'
    }]
}];
```

---

## 9. Comparison: Spline vs Traditional Tools

| Feature | Spline | Blender | Three.js |
|---------|--------|---------|----------|
| Platform | Browser | Desktop | Library |
| Learning Curve | Hours | Months | Days |
| Real-time Collaboration | Yes | No | N/A |
| Scripting | Visual + JS | Python | JavaScript |
| Rendering | WebGL | Cycles/Eevee | WebGL/WebGPU |
| Performance | Good | Excellent | Good |
| Cost | Freemium | Free | Free |
| Best For | Web designers | 3D artists | Developers |

---

## 10. Key Takeaways

1. **Browser-First Architecture**: Spline demonstrates that complex 3D tools can run entirely in the browser

2. **Custom Renderer Benefits**: Building on raw WebGL (vs Three.js) gives full control over performance and features

3. **Animation System**: Keyframe-based animation with Bezier curves provides familiar workflow for designers

4. **Event-Driven Interactions**: Raycasting + event system enables game-like interactivity

5. **Export Flexibility**: glTF/USDZ export ensures compatibility with downstream tools

6. **WebGPU Ready**: Architecture positions for seamless WebGPU migration for better performance

---

## 11. Sources

- [Spline Documentation](https://docs.spline.design/)
- [Spline React Component](https://github.com/splinetool/react-spline)
- [WebGL Fundamentals](https://webglfundamentals.org/)
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)
- [glTF Specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
