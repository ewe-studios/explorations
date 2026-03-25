# Rust Replication Plan for Spline 3D

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Spline3d/`

This document provides a comprehensive plan for replicating Spline 3D functionality in Rust.

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Crate Ecosystem](#crate-ecosystem)
4. [Core Components](#core-components)
5. [WASM Strategy](#wasm-strategy)
6. [Web Integration](#web-integration)
7. [Implementation Roadmap](#implementation-roadmap)
8. [Production Considerations](#production-considerations)

---

## Executive Summary

### Feasibility Assessment

| Component | Complexity | Rust Maturity | Recommendation |
|-----------|------------|---------------|----------------|
| Scene Graph | Medium | High | Implement |
| Spline Algorithms | Medium | High | Use existing crates |
| Mesh Processing | High | Medium | Implement |
| WebGL Rendering | Medium | High | Use web-sys |
| WebGPU Rendering | Medium | High | Use wgpu |
| WASM Export | Low | High | Use wasm-bindgen |
| React Integration | Low | Medium | JS bridge |

### Estimated Effort

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| 1. Core Math | 2-3 weeks | Vector math, transformations |
| 2. Spline Engine | 4-6 weeks | Bezier, B-Spline, NURBS |
| 3. Scene Graph | 3-4 weeks | Object hierarchy, transforms |
| 4. Rendering | 6-8 weeks | wgpu pipeline, shaders |
| 5. WASM/JS | 2-3 weeks | Bindings, JS API |
| 6. Polish | 4-6 weeks | Optimization, docs |

**Total: 21-30 weeks for MVP**

---

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Rust Spline 3D Architecture                   │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   JavaScript Layer                         │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │   React     │  │   Vanilla   │  │   Framework     │   │  │
│  │  │   Components│  │   JS API    │  │   Adapters      │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                    wasm-bindgen Bridge                           │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   WebAssembly Module                       │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │                  Public API Layer                    │  │  │
│  │  │  ┌───────────┐  ┌───────────┐  ┌─────────────────┐  │  │  │
│  │  │  │  Scene    │  │  Object   │  │   Animation     │  │  │  │
│  │  │  │  API      │  │  API      │  │   API           │  │  │  │
│  │  │  └───────────┘  └───────────┘  └─────────────────┘  │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │                  Core Engine                         │  │  │
│  │  │  ┌───────────┐  ┌───────────┐  ┌─────────────────┐  │  │  │
│  │  │  │  Scene    │  │  Spline   │  │    Geometry     │  │  │  │
│  │  │  │  Graph    │  │  Engine   │  │    Processing   │  │  │  │
│  │  │  └───────────┘  └───────────┘  └─────────────────┘  │  │  │
│  │  │  ┌───────────┐  ┌───────────┐  ┌─────────────────┐  │  │  │
│  │  │  │  Render   │  │  Material │  │    Animation    │  │  │  │
│  │  │  │  Engine   │  │  System   │  │    System       │  │  │  │
│  │  │  └───────────┘  └───────────┘  └─────────────────┘  │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │                  Graphics Backend                    │  │  │
│  │  │  ┌───────────┐  ┌───────────┐  ┌─────────────────┐  │  │  │
│  │  │  │   wgpu    │  │   WebGL   │  │    Software     │  │  │  │
│  │  │  │  (WebGPU) │  │  (web-sys)│  │    Fallback     │  │  │  │
│  │  │  └───────────┘  └───────────┘  └─────────────────┘  │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Module Structure

```
spline3d/
├── Cargo.toml
├── crates/
│   ├── spline3d-core/        # Core types and traits
│   ├── spline3d-math/        # Math primitives
│   ├── spline3d-curves/      # Bezier, B-Spline, NURBS
│   ├── spline3d-scene/       # Scene graph
│   ├── spline3d-geometry/    # Mesh processing
│   ├── spline3d-render/      # Rendering engine
│   ├── spline3d-wasm/        # WASM bindings
│   └── spline3d-js/          # JavaScript package
```

---

## Crate Ecosystem

### Core Dependencies

```toml
# crates/spline3d-core/Cargo.toml
[package]
name = "spline3d-core"
version = "0.1.0"
edition = "2021"

[dependencies]
# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
log = "0.4"
tracing = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"

# Smart pointers
arc-swap = "1.6"
parking_lot = "0.12"  # Faster Mutex
```

```toml
# crates/spline3d-math/Cargo.toml
[package]
name = "spline3d-math"
version = "0.1.0"
edition = "2021"

[dependencies]
# Linear algebra
nalgebra = "0.32"
cgmath = "0.18"

# SIMD optimization
wide = "0.7"

[features]
simd = ["wide"]
```

```toml
# crates/spline3d-curves/Cargo.toml
[package]
name = "spline3d-curves"
version = "0.1.0"
edition = "2021"

[dependencies]
spline3d-math = { path = "../spline3d-math" }
nalgebra = "0.32"
bezier-rs = "0.4"  # Existing bezier implementation
nurbs = "0.5"      # Existing NURBS implementation
```

```toml
# crates/spline3d-render/Cargo.toml
[package]
name = "spline3d-render"
version = "0.1.0"
edition = "2021"

[dependencies]
wgpu = "0.19"
winit = "0.29"
pollster = "0.3"  # For async runtime
bytemuck = { version = "1.14", features = ["derive"] }
```

```toml
# crates/spline3d-wasm/Cargo.toml
[package]
name = "spline3d-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
    "Window",
    "Document",
    "HtmlCanvasElement",
    "WebGlRenderingContext",
    "WebGl2RenderingContext",
] }
console_error_panic_hook = "0.1"
spline3d-core = { path = "../spline3d-core" }
spline3d-scene = { path = "../spline3d-scene" }
spline3d-render = { path = "../spline3d-render" }

[profile.release]
opt-level = 3
lto = true
```

---

## Core Components

### 1. Math Foundation

```rust
// crates/spline3d-math/src/lib.rs
pub use nalgebra::{
    Vector2, Vector3, Vector4,
    Matrix3, Matrix4,
    Point2, Point3,
    Isometry3, Similarity3,
};

/// Transformation trait for 3D objects
pub trait Transform3D: Sized {
    fn translation(&self) -> Vector3<f64>;
    fn rotation(&self) -> Vector3<f64>;
    fn scale(&self) -> Vector3<f64>;

    fn set_translation(&mut self, t: Vector3<f64>);
    fn set_rotation(&mut self, r: Vector3<f64>);
    fn set_scale(&mut self, s: Vector3<f64>);

    fn to_matrix(&self) -> Matrix4<f64>;
    fn from_matrix(matrix: Matrix4<f64>) -> Self;
}

/// Bounding volume
#[derive(Debug, Clone)]
pub enum BoundingVolume {
    Sphere { center: Point3<f64>, radius: f64 },
    Box { min: Point3<f64>, max: Point3<f64> },
    OrientedBox { center: Point3<f64>, half_extents: Vector3<f64>, rotation: Matrix3<f64> },
}

impl BoundingVolume {
    pub fn contains(&self, point: &Point3<f64>) -> bool {
        match self {
            BoundingVolume::Sphere { center, radius } => {
                point.distance(center) <= *radius
            }
            BoundingVolume::Box { min, max } => {
                point.x >= min.x && point.x <= max.x &&
                point.y >= min.y && point.y <= max.y &&
                point.z >= min.z && point.z <= max.z
            }
            BoundingVolume::OrientedBox { .. } => todo!(),
        }
    }

    pub fn intersects(&self, other: &BoundingVolume) -> bool {
        // Implement intersection test
        todo!()
    }
}
```

### 2. Spline Engine

```rust
// crates/spline3d-curves/src/lib.rs
use nalgebra::{Vector3, Matrix4};

/// Curve trait for all spline types
pub trait Curve: Send + Sync {
    /// Evaluate curve at parameter t (0..=1)
    fn evaluate(&self, t: f64) -> Vector3<f64>;

    /// Evaluate derivative at t
    fn derivative(&self, t: f64) -> Vector3<f64>;

    /// Get curve bounds
    fn bounds(&self) -> (Vector3<f64>, Vector3<f64>);

    /// Tessellate into line segments
    fn tessellate(&self, segments: usize) -> Vec<Vector3<f64>> {
        (0..=segments)
            .map(|i| self.evaluate(i as f64 / segments as f64))
            .collect()
    }
}

/// Cubic Bezier curve
#[derive(Debug, Clone)]
pub struct CubicBezier {
    p0: Vector3<f64>,
    p1: Vector3<f64>,
    p2: Vector3<f64>,
    p3: Vector3<f64>,
}

impl CubicBezier {
    pub fn new(p0: Vector3<f64>, p1: Vector3<f64>, p2: Vector3<f64>, p3: Vector3<f64>) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// De Casteljau evaluation
    pub fn de_casteljau(&self, t: f64) -> Vector3<f64> {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let t2 = t * t;
        let t3 = t2 * t;

        mt3 * self.p0 + 3.0 * mt2 * t * self.p1 + 3.0 * mt * t2 * self.p2 + t3 * self.p3
    }

    /// Split curve at parameter t
    pub fn split(&self, t: f64) -> (Self, Self) {
        let mt = 1.0 - t;

        // First level interpolation
        let p01 = self.p0.lerp(&self.p1, t);
        let p12 = self.p1.lerp(&self.p2, t);
        let p23 = self.p2.lerp(&self.p3, t);

        // Second level
        let p012 = p01.lerp(&p12, t);
        let p123 = p12.lerp(&p23, t);

        // Final point (on curve)
        let p0123 = p012.lerp(&p123, t);

        let left = Self::new(self.p0, p01, p012, p0123);
        let right = Self::new(p0123, p123, p23, self.p3);

        (left, right)
    }
}

impl Curve for CubicBezier {
    fn evaluate(&self, t: f64) -> Vector3<f64> {
        self.de_casteljau(t)
    }

    fn derivative(&self, t: f64) -> Vector3<f64> {
        let mt = 1.0 - t;
        3.0 * mt * mt * (self.p1 - self.p0) +
        6.0 * mt * t * (self.p2 - self.p1) +
        3.0 * t * t * (self.p3 - self.p2)
    }

    fn bounds(&self) -> (Vector3<f64>, Vector3<f64>) {
        // Compute bounding box from control points
        let min = Vector3::new(
            self.p0.x.min(self.p1.x).min(self.p2.x).min(self.p3.x),
            self.p0.y.min(self.p1.y).min(self.p2.y).min(self.p3.y),
            self.p0.z.min(self.p1.z).min(self.p2.z).min(self.p3.z),
        );
        let max = Vector3::new(
            self.p0.x.max(self.p1.x).max(self.p2.x).max(self.p3.x),
            self.p0.y.max(self.p1.y).max(self.p2.y).max(self.p3.y),
            self.p0.z.max(self.p1.z).max(self.p2.z).max(self.p3.z),
        );
        (min, max)
    }
}

/// NURBS curve
#[derive(Debug, Clone)]
pub struct NurbsCurve {
    control_points: Vec<Vector3<f64>>,
    weights: Vec<f64>,
    knots: Vec<f64>,
    degree: usize,
}

impl NurbsCurve {
    pub fn new(
        control_points: Vec<Vector3<f64>>,
        weights: Vec<f64>,
        knots: Vec<f64>,
        degree: usize,
    ) -> Result<Self, NurbsError> {
        // Validate inputs
        if control_points.len() != weights.len() {
            return Err(NurbsError::WeightMismatch);
        }
        if knots.len() != control_points.len() + degree + 1 {
            return Err(NurbsError::KnotCountMismatch);
        }

        Ok(Self {
            control_points,
            weights,
            knots,
            degree,
        })
    }

    /// Cox-de Boor basis function
    fn basis(&self, i: usize, p: usize, t: f64) -> f64 {
        if p == 0 {
            let ui = self.knots[i];
            let ui1 = self.knots[i + 1];
            let is_last = i == self.knots.len() - 2;

            if t >= ui && (t < ui1 || (is_last && t == ui1)) {
                1.0
            } else {
                0.0
            }
        } else {
            let ui = self.knots[i];
            let ui_p = self.knots[i + p];
            let ui1 = self.knots[i + 1];
            let ui_p1 = self.knots[i + p + 1];

            let c1 = if (ui_p - ui).abs() > 1e-10 {
                ((t - ui) / (ui_p - ui)) * self.basis(i, p - 1, t)
            } else {
                0.0
            };

            let c2 = if (ui_p1 - ui1).abs() > 1e-10 {
                ((ui_p1 - t) / (ui_p1 - ui1)) * self.basis(i + 1, p - 1, t)
            } else {
                0.0
            };

            c1 + c2
        }
    }
}

impl Curve for NurbsCurve {
    fn evaluate(&self, t: f64) -> Vector3<f64> {
        let n = self.control_points.len() - 1;
        let p = self.degree;

        let mut point = Vector3::zeros();
        let mut weight_sum = 0.0;

        for i in 0..=n {
            let basis = self.basis(i, p, t);
            let weight = self.weights[i];
            point += weight * basis * self.control_points[i];
            weight_sum += weight * basis;
        }

        point / weight_sum
    }

    fn derivative(&self, t: f64) -> Vector3<f64> {
        // Implement NURBS derivative
        todo!()
    }

    fn bounds(&self) -> (Vector3<f64>, Vector3<f64>) {
        // Convex hull of weighted control points
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NurbsError {
    #[error("Control points and weights count mismatch")]
    WeightMismatch,
    #[error("Knot vector count mismatch")]
    KnotCountMismatch,
    #[error("Invalid knot vector")]
    InvalidKnots,
}
```

### 3. Scene Graph

```rust
// crates/spline3d-scene/src/lib.rs
use std::sync::Arc;
use parking_lot::RwLock;
use spline3d_math::{Matrix4, Vector3, BoundingVolume};
use spline3d_curves::Curve;

/// Scene object ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(u64);

impl ObjectId {
    pub fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// Scene object types
#[derive(Debug, Clone)]
pub enum ObjectType {
    Mesh { geometry: Arc<Geometry>, material: Arc<Material> },
    Curve { curve: Arc<dyn Curve>, samples: usize },
    Surface { surface: Arc<dyn Surface>, u_samples: usize, v_samples: usize },
    Group { children: Vec<ObjectId> },
    Light { light_type: LightType, intensity: f32 },
    Camera { fov: f32, near: f32, far: f32 },
}

/// Scene object
#[derive(Debug)]
pub struct SceneObject {
    pub id: ObjectId,
    pub name: String,
    pub object_type: ObjectType,
    pub transform: Matrix4<f64>,
    pub visible: bool,
    pub cast_shadow: bool,
    pub receive_shadow: bool,
    bounds: RwLock<Option<BoundingVolume>>,
}

impl SceneObject {
    pub fn new(name: String, object_type: ObjectType) -> Self {
        Self {
            id: ObjectId::new(),
            name,
            object_type,
            transform: Matrix4::identity(),
            visible: true,
            cast_shadow: true,
            receive_shadow: true,
            bounds: RwLock::new(None),
        }
    }

    pub fn world_transform(&self, parent: Option<&SceneObject>) -> Matrix4<f64> {
        if let Some(parent) = parent {
            parent.transform * self.transform
        } else {
            self.transform
        }
    }

    pub fn translation(&self) -> Vector3<f64> {
        Vector3::new(self.transform.m14, self.transform.m24, self.transform.m34)
    }

    pub fn set_translation(&mut self, t: Vector3<f64>) {
        self.transform.m14 = t.x;
        self.transform.m24 = t.y;
        self.transform.m34 = t.z;
    }
}

/// Scene graph
pub struct Scene {
    root: SceneObject,
    objects: rustc_hash::FxHashMap<ObjectId, SceneObject>,
    children: rustc_hash::FxHashMap<ObjectId, Vec<ObjectId>>,
    parents: rustc_hash::FxHashMap<ObjectId, ObjectId>,
}

impl Scene {
    pub fn new() -> Self {
        let root = SceneObject::new("root".to_string(), ObjectType::Group { children: vec![] });
        let mut objects = rustc_hash::FxHashMap::default();
        objects.insert(root.id, root);

        Self {
            root: objects.get(&ObjectId(0)).unwrap().clone(),
            objects,
            children: rustc_hash::FxHashMap::default(),
            parents: rustc_hash::FxHashMap::default(),
        }
    }

    pub fn add_object(&mut self, parent: ObjectId, mut object: SceneObject) -> ObjectId {
        let id = object.id;
        self.objects.insert(id, object);

        self.children.entry(parent).or_default().push(id);
        self.parents.insert(id, parent);

        id
    }

    pub fn get_object(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.get(&id)
    }

    pub fn get_object_mut(&mut self, id: ObjectId) -> Option<&mut SceneObject> {
        self.objects.get_mut(&id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<ObjectId> {
        self.objects.values().find(|o| o.name == name).map(|o| o.id)
    }

    pub fn remove_object(&mut self, id: ObjectId) {
        // Remove children recursively
        if let Some(children) = self.children.remove(&id) {
            for child_id in children {
                self.remove_object(child_id);
            }
        }

        // Remove from parent's children list
        if let Some(&parent_id) = self.parents.get(&id) {
            if let Some(parent_children) = self.children.get_mut(&parent_id) {
                parent_children.retain(|&child_id| child_id != id);
            }
            self.parents.remove(&id);
        }

        self.objects.remove(&id);
    }

    /// Traverse scene graph
    pub fn traverse<F: FnMut(&SceneObject)>(&self, mut f: F) {
        self.traverse_recursive(self.root.id, &mut f);
    }

    fn traverse_recursive<F: FnMut(&SceneObject)>(&self, id: ObjectId, f: &mut F) {
        if let Some(object) = self.objects.get(&id) {
            f(object);

            if let Some(children) = self.children.get(&id) {
                for &child_id in children {
                    self.traverse_recursive(child_id, f);
                }
            }
        }
    }
}
```

### 4. Rendering Engine

```rust
// crates/spline3d-render/src/lib.rs
use wgpu::*;
use std::sync::Arc;

/// Render configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    pub sample_count: u32,
    pub present_mode: PresentMode,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            sample_count: 1,
            present_mode: PresentMode::AutoVsync,
        }
    }
}

/// Camera for rendering
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: nalgebra::Vector3<f64>,
    pub target: nalgebra::Vector3<f64>,
    pub up: nalgebra::Vector3<f64>,
    pub fov: f64,
    pub aspect: f64,
    pub near: f64,
    pub far: f64,
}

impl Camera {
    pub fn view_matrix(&self) -> nalgebra::Matrix4<f64> {
        nalgebra::Matrix4::look_at_rh(
            &nalgebra::Point3::from(self.position),
            &nalgebra::Point3::from(self.target),
            &self.up,
        )
    }

    pub fn projection_matrix(&self) -> nalgebra::Matrix4<f64> {
        nalgebra::Matrix4::new_perspective(
            self.aspect as f64,
            self.fov,
            self.near,
            self.far,
        )
    }
}

/// Renderer
pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Option<Surface<'static>>,
    config: SurfaceConfiguration,
    pipeline: RenderPipeline,
    depth_texture: Texture,
    camera: Camera,
}

impl Renderer {
    pub async fn new<W: HasWindowHandle + HasDisplayHandle>(
        window: W,
        config: RenderConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];

        let size = window.window_handle()?.window_size();
        let mut renderer_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: config.present_mode,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &renderer_config);

        // Create pipeline
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Spline Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: 32, // position(12) + normal(12) + uv(8)
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![
                        0 => Float32x3,  // position
                        1 => Float32x3,  // normal
                        2 => Float32x2,  // uv
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                ..Default::default()
            }),
            multisample: MultisampleState {
                count: config.sample_count,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        });

        // Create depth texture
        let depth_texture = Self::create_depth_texture(&device, config.width, config.height);

        Ok(Self {
            device,
            queue,
            surface: Some(surface),
            config: renderer_config,
            pipeline,
            depth_texture,
            camera: Camera {
                position: nalgebra::Vector3::new(0.0, 0.0, 5.0),
                target: nalgebra::Vector3::new(0.0, 0.0, 0.0),
                up: nalgebra::Vector3::new(0.0, 1.0, 0.0),
                fov: std::f64::consts::PI / 4.0,
                aspect: config.width as f64 / config.height as f64,
                near: 0.1,
                far: 1000.0,
            },
        })
    }

    fn create_depth_texture(device: &Device, width: u32, height: u32) -> Texture {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let desc = TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        device.create_texture(&desc)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.camera.aspect = width as f64 / height as f64;

        if let Some(surface) = &self.surface {
            surface.configure(&self.device, &self.config);
        }

        self.depth_texture = Self::create_depth_texture(&self.device, width, height);
    }

    pub fn render(&mut self, scene: &spline3d_scene::Scene) -> Result<(), SurfaceError> {
        let surface = self.surface.as_ref().unwrap();
        let output = surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        let depth_view = self.depth_texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.pipeline);

            // Set view/projection matrices as push constants or uniforms
            let view_matrix = self.camera.view_matrix();
            let proj_matrix = self.camera.projection_matrix();

            // Render scene
            scene.traverse(|object| {
                if !object.visible {
                    return;
                }

                // Set model matrix and draw
                // ...
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
```

---

## WASM Strategy

### Build Configuration

```toml
# .cargo/config.toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "link-arg=--export-table"]

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### wasm-bindgen Bindings

```rust
// crates/spline3d-wasm/src/lib.rs
use wasm_bindgen::prelude::*;
use spline3d_scene::{Scene, SceneObject, ObjectType};
use spline3d_curves::{CubicBezier, Curve};
use nalgebra::Vector3;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

/// Spline Application
#[wasm_bindgen]
pub struct Application {
    scene: Scene,
    canvas: web_sys::HtmlCanvasElement,
    renderer: Option<spline3d_render::Renderer>,
}

#[wasm_bindgen]
impl Application {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Result<Application, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<web_sys::HtmlCanvasElement>()?;

        Ok(Application {
            scene: Scene::new(),
            canvas,
            renderer: None,
        })
    }

    /// Load scene from URL
    #[wasm_bindgen]
    pub async fn load(&mut self, url: &str) -> Result<(), JsValue> {
        // Fetch and parse scene file
        let response = gloo_net::http::Request::get(url)
            .send()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let data = response.binary().await.map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Parse .splinecode format
        self.parse_scene(&data)?;

        // Initialize renderer
        self.init_renderer().await?;

        Ok(())
    }

    /// Find object by name
    #[wasm_bindgen]
    pub fn find_object_by_name(&self, name: &str) -> Option<JsValue> {
        self.scene.find_by_name(name).map(|id| {
            // Return JS object with object methods
            // ...
            JsValue::NULL
        })
    }

    /// Find object by ID
    #[wasm_bindgen(js_name = findObjectById)]
    pub fn find_object_by_id(&self, id: u64) -> Option<JsValue> {
        todo!()
    }

    async fn init_renderer(&mut self) -> Result<(), JsValue> {
        // Initialize wgpu renderer
        // ...
        Ok(())
    }

    fn parse_scene(&mut self, data: &[u8]) -> Result<(), JsValue> {
        // Parse binary .splinecode format
        todo!()
    }
}

/// Scene Object wrapper
#[wasm_bindgen]
pub struct SceneObjectWrapper {
    id: spline3d_scene::ObjectId,
    scene: Arc<RwLock<Scene>>,
}

#[wasm_bindgen]
impl SceneObjectWrapper {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        let scene = self.scene.read();
        scene.get_object(self.id).map(|o| o.name.clone()).unwrap_or_default()
    }

    #[wasm_bindgen(getter)]
    pub fn position(&self) -> JsValue {
        let scene = self.scene.read();
        if let Some(object) = scene.get_object(self.id) {
            let pos = object.translation();
            js_sys::Object::with_properties(&[
                ("x", &JsValue::from_f64(pos.x)),
                ("y", &JsValue::from_f64(pos.y)),
                ("z", &JsValue::from_f64(pos.z)),
            ]).into()
        } else {
            JsValue::NULL
        }
    }

    #[wasm_bindgen(setter)]
    pub fn set_position(&self, pos: &JsValue) {
        let mut scene = self.scene.write();
        if let Some(object) = scene.get_object_mut(self.id) {
            if let Some(x) = pos.as_f64() {
                // Handle single value
            } else if let Some(obj) = pos.dyn_ref::<js_sys::Object>() {
                // Handle {x, y, z} object
            }
        }
    }

    /// Emit event on object
    #[wasm_bindgen(js_name = emitEvent)]
    pub fn emit_event(&self, event_name: &str) {
        // Trigger animation/action
        todo!()
    }
}
```

### JavaScript Package

```javascript
// crates/spline3d-js/src/index.js
import init, { Application } from './spline3d_wasm.js';

let wasmModule = null;

export async function initWasm(moduleUrl) {
  if (!wasmModule) {
    wasmModule = await init(moduleUrl);
  }
  return wasmModule;
}

export class SplineApp {
  constructor(canvasId) {
    this.app = new Application(canvasId);
  }

  async load(sceneUrl) {
    await this.app.load(sceneUrl);
    return this;
  }

  findObjectByName(name) {
    return this.app.find_object_by_name(name);
  }

  findObjectById(id) {
    return this.app.find_object_by_id(BigInt(id));
  }

  dispose() {
    // Cleanup
  }
}

export default SplineApp;
```

```json
// crates/spline3d-js/package.json
{
  "name": "@spline3d/runtime",
  "version": "0.1.0",
  "type": "module",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "files": [
    "dist"
  ],
  "scripts": {
    "build": "wasm-pack build --release --target web",
    "publish": "npm publish"
  },
  "dependencies": {}
}
```

---

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-3)

```
Week 1-2: Math Foundation
├── Vector/Matrix operations
├── Transformations
├── Bounding volumes
└── Ray casting

Week 3: Project Setup
├── Cargo workspace
├── Module structure
├── CI/CD pipeline
└── Testing infrastructure
```

### Phase 2: Spline Engine (Weeks 4-9)

```
Week 4-5: Bezier Curves
├── Cubic Bezier implementation
├── De Casteljau algorithm
├── Curve splitting
└── Tessellation

Week 6-7: B-Splines
├── Knot vectors
├── Cox-de Boor algorithm
├── Basis functions
└── Evaluation

Week 8-9: NURBS
├── Rational weights
├── NURBS curves
├── NURBS surfaces
└── Conic sections (circles, etc.)
```

### Phase 3: Scene Graph (Weeks 10-13)

```
Week 10-11: Object System
├── SceneObject struct
├── Transform hierarchy
├── Object types (Mesh, Light, Camera)
└── Scene management

Week 12-13: Geometry
├── Mesh data structures
├── Buffer management
├── Geometry loading (OBJ, glTF)
└── Mesh processing
```

### Phase 4: Rendering (Weeks 14-21)

```
Week 14-15: wgpu Setup
├── Device/Queue initialization
├── Surface configuration
├── Basic pipeline
└── Shader compilation

Week 16-17: Pipeline
├── Vertex/Fragment shaders
├── Uniform buffers
├── Camera matrices
└── Lighting

Week 18-19: Materials
├── PBR materials
├── Texture loading
├── Samplers
└── Material system

Week 20-21: Advanced
├── Shadow mapping
├── instancing
├── LOD system
└── Post-processing
```

### Phase 5: WASM/JS (Weeks 22-24)

```
Week 22: WASM Bindings
├── wasm-bindgen setup
├── JS API surface
├── Async loading
└── Error handling

Week 23: JavaScript Package
├── npm package structure
├── TypeScript definitions
├── Module bundling
└── Documentation

Week 24: Integration
├── React component
├── Example applications
├── Testing
└── Performance tuning
```

---

## Production Considerations

### Performance Optimization

1. **WASM Size**
   - Enable LTO and size optimization
   - Strip debug symbols
   - Tree-shaking
   - Target: < 500KB compressed

2. **Runtime Performance**
   - Use SIMD where possible
   - Minimize JS/WASM crossings
   - Batch render calls
   - Use web workers for heavy computation

3. **Memory Management**
   - Reuse buffers
   - Implement object pooling
   - Avoid allocations in render loop

### Browser Compatibility

```javascript
// Feature detection
async function checkSupport() {
  const support = {
    webgl: !!document.createElement('canvas').getContext('webgl2'),
    webgpu: !!navigator.gpu,
    wasm: typeof WebAssembly === 'object',
  };

  if (!support.wasm) {
    throw new Error('WebAssembly not supported');
  }

  return support;
}
```

### Fallback Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                  Rendering Fallback Strategy                     │
│                                                                  │
│  1. WebGPU (wgpu)  ──> Best performance, modern browsers        │
│         │                                                        │
│         ▼ (fallback)                                             │
│  2. WebGL 2 (web-sys) ──> Good performance, most browsers       │
│         │                                                        │
│         ▼ (fallback)                                             │
│  3. WebGL 1 ──> Basic support, older browsers                   │
│         │                                                        │
│         ▼ (fallback)                                             │
│  4. Canvas 2D ──> Minimal support, all browsers                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## References

1. **wgpu** - https://github.com/gfx-rs/wgpu
2. **wasm-bindgen** - https://rustwasm.github.io/wasm-bindgen/
3. **nalgebra** - https://nalgebra.org/
4. **bezier-rs** - https://crates.io/crates/bezier-rs
