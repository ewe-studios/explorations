# PlayCanvas in Rust - Implementation Plan

## Overview

This document outlines how to replicate PlayCanvas's functionality in Rust, leveraging the Rust game development ecosystem including wgpu, Bevy, and other crates.

---

## Architecture Comparison

| PlayCanvas Component | JavaScript/Web | Rust Equivalent |
|---------------------|----------------|-----------------|
| Graphics Backend | WebGL 2 / WebGPU | wgpu |
| ECS | Custom component system | Bevy ECS / custom |
| Asset Loading | XHR/Fetch + parsers | reqwest + ron/serde |
| Physics | Ammo.js (Bullet WASM) | rapier3d |
| Audio | Web Audio API | rodio / cpal |
| Math | gl-matrix | glam / nalgebra |
| Shaders | GLSL / WGSL | WGSL / naga |
| UI | PCUI (custom) | egui / iyes_ui |

---

## Core Engine Structure

```
playcanvas-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Engine entry point
│   │
│   ├── core/               # Core utilities
│   │   ├── mod.rs
│   │   ├── math.rs         # Math types (glam)
│   │   ├── color.rs        # Color types
│   │   ├── debug.rs        # Debug utilities
│   │   ├── error.rs        # Error handling
│   │   └── time.rs         # Time/delta time
│   │
│   ├── ecs/                # Entity Component System
│   │   ├── mod.rs
│   │   ├── entity.rs       # Entity type
│   │   ├── component.rs    # Component trait
│   │   ├── system.rs       # System trait
│   │   ├── world.rs        # ECS world
│   │   └── hierarchy.rs    # Parent-child relationships
│   │
│   ├── gfx/                # Graphics (wgpu)
│   │   ├── mod.rs
│   │   ├── device.rs       # Graphics device
│   │   ├── texture.rs      # Texture resources
│   │   ├── buffer.rs       # Vertex/Uniform buffers
│   │   ├── shader.rs       # Shader management
│   │   ├── material.rs     # Material system
│   │   ├── mesh.rs         # Mesh geometry
│   │   ├── renderer.rs     # Forward renderer
│   │   ├── light.rs        # Lighting
│   │   └── camera.rs       # Camera component
│   │
│   ├── asset/              # Asset system
│   │   ├── mod.rs
│   │   ├── registry.rs     # Asset registry
│   │   ├── handle.rs       # Asset handles
│   │   ├── loader.rs       # Asset loader
│   │   └── formats/        # Format parsers
│   │       ├── gltf.rs
│   │       ├── texture.rs
│   │       └── audio.rs
│   │
│   ├── scene/              # Scene management
│   │   ├── mod.rs
│   │   ├── scene.rs        # Scene container
│   │   ├── node.rs         # Scene graph node
│   │   ├── transform.rs    # Transform component
│   │   └── visibility.rs   # Culling/visibility
│   │
│   ├── physics/            # Physics (rapier3d)
│   │   ├── mod.rs
│   │   ├── rigid_body.rs
│   │   ├── collider.rs
│   │   ├── world.rs
│   │   └── query.rs        # Raycasting
│   │
│   ├── animation/          # Animation system
│   │   ├── mod.rs
│   │   ├── clip.rs
│   │   ├── track.rs
│   │   ├── evaluator.rs
│   │   ├── skinning.rs
│   │   └── morph.rs
│   │
│   ├── input/              # Input handling
│   │   ├── mod.rs
│   │   ├── keyboard.rs
│   │   ├── mouse.rs
│   │   └── gamepad.rs
│   │
│   ├── audio/              # Audio system
│   │   ├── mod.rs
│   │   ├── source.rs
│   │   ├── listener.rs
│   │   └── spatial.rs
│   │
│   └── app/                # Application framework
│       ├── mod.rs
│       ├── app.rs          # Main application
│       ├── plugin.rs       # Plugin trait
│       └── state.rs        # Application state
│
└── examples/
    ├── basic.rs
    ├── physics.rs
    └── animation.rs
```

---

## Core Implementation

### Cargo.toml Dependencies

```toml
[package]
name = "playcanvas-rs"
version = "0.1.0"
edition = "2021"
description = "A WebGL/WebGPU game engine inspired by PlayCanvas"

[dependencies]
# Math
glam = "0.28"
nalgebra = "0.33"

# Graphics
wgpu = "22"
winit = "0.30"
pollster = "0.4"  # For async polling

# ECS (optional - can use Bevy or custom)
bevy_ecs = "0.14"  # Or custom implementation

# Asset loading
image = "0.25"
rodio = "0.19"
gltf = "1.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
ron = "0.8"

# Async runtime
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }

# Physics
rapier3d = "0.21"

# Audio
cpal = "0.15"

# Utilities
anyhow = "1.0"
thiserror = "2"
log = "0.4"
tracing = "0.1"
parking_lot = "0.12"  # For mutexes
uuid = { version = "1", features = ["v4"] }

# Shaders (compile WGSL at build time)
naga = "22"

[dev-dependencies]
env_logger = "0.11"
```

---

## ECS Implementation

### Using Bevy ECS

```rust
// src/ecs/mod.rs

use bevy_ecs::prelude::*;
use glam::{Vec3, Quat};

/// Entity ID wrapper
#[derive(Component, Clone, Copy, Debug)]
pub struct EntityId(pub uuid::Uuid);

/// Transform component (like GraphNode)
#[derive(Component, Clone, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub dirty: bool,
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            dirty: true,
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn compute_world_matrix(&self, parent: Option<&Transform>) -> Mat4 {
        let local = Mat4::from_scale_rotation_translation(
            self.scale,
            self.rotation,
            self.translation,
        );

        match parent {
            Some(parent) => parent.compute_world_matrix(None) * local,
            None => local,
        }
    }
}

/// System to update hierarchy transforms
pub fn transform_system(
    mut query: Query<(&Transform, &mut Transform)>,
    children: Query<&Children>,
) {
    // Implementation of hierarchy transform propagation
}
```

### Custom ECS (Alternative)

```rust
// src/ecs/world.rs

use std::any::TypeId;
use std::collections::HashMap;
use uuid::Uuid;

/// Entity ID
pub type EntityId = Uuid;

/// Component trait
pub trait Component: Send + Sync + 'static {}

/// Component storage
pub struct ComponentStore<T: Component> {
    data: HashMap<EntityId, T>,
}

/// ECS World
pub struct World {
    entities: HashMap<EntityId, EntityInfo>,
    components: HashMap<TypeId, Box<dyn AnyComponentStore>>,
}

struct EntityInfo {
    alive: bool,
    components: Vec<TypeId>,
    parent: Option<EntityId>,
    children: Vec<EntityId>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            components: HashMap::new(),
        }
    }

    pub fn create_entity(&mut self) -> EntityId {
        let id = Uuid::new_v4();
        self.entities.insert(
            id,
            EntityInfo {
                alive: true,
                components: Vec::new(),
                parent: None,
                children: Vec::new(),
            },
        );
        id
    }

    pub fn add_component<T: Component>(&mut self, entity: EntityId, component: T) {
        let type_id = TypeId::of::<T>();

        let store = self
            .components
            .entry(type_id)
            .or_insert_with(|| Box::new(ComponentStore::<T>::new()));

        store
            .downcast_mut::<ComponentStore<T>>()
            .unwrap()
            .data
            .insert(entity, component);

        if let Some(info) = self.entities.get_mut(&entity) {
            info.components.push(type_id);
        }
    }

    pub fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let store = self.components.get(&type_id)?;
        let store = store.downcast_ref::<ComponentStore<T>>()?;
        store.data.get(&entity)
    }

    pub fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        let store = self.components.get_mut(&type_id)?;
        let store = store.downcast_mut::<ComponentStore<T>>()?;
        store.data.get_mut(&entity)
    }
}
```

---

## Graphics Implementation (wgpu)

### Graphics Device

```rust
// src/gfx/device.rs

use wgpu::*;
use anyhow::Result;

pub struct GraphicsDevice {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Option<Surface>,
    config: SurfaceConfiguration,

    // Capabilities
    max_texture_size: u32,
    max_anisotropy: f32,
}

impl GraphicsDevice {
    pub async fn new(window: &winit::window::Window) -> Result<Self> {
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
            .request_device(
                &DeviceDescriptor {
                    label: Some("PlayCanvas Device"),
                    required_limits: Limits::default(),
                    features: Features::empty(),
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];

        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: AlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let limits = adapter.limits();

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface: Some(surface),
            config,
            max_texture_size: limits.max_texture_dimension_2d,
            max_anisotropy: limits.max_anisotropy,
        })
    }

    pub fn create_texture(&self, desc: TextureDescriptor) -> Texture {
        self.device.create_texture(&desc)
    }

    pub fn create_shader_module(&self, source: &str) -> ShaderModule {
        self.device
            .create_shader_module(ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Wgsl(source.into()),
            })
    }

    pub fn create_render_pipeline(
        &self,
        layout: Option<&PipelineLayout>,
        vertex: VertexState,
        fragment: Option<FragmentState>,
        primitive: PrimitiveState,
        depth_stencil: Option<DepthStencilState>,
    ) -> RenderPipeline {
        self.device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: None,
                layout,
                vertex,
                fragment,
                primitive,
                depth_stencil,
                multisample: MultisampleState::default(),
                multiview: None,
            })
    }

    pub fn begin_render_pass(
        &mut self,
        color_attachments: &[RenderPassColorAttachment],
        depth_attachment: Option<RenderPassDepthStencilAttachment>,
    ) -> RenderPass {
        let encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments,
            depth_stencil_attachment: depth_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass
    }

    pub fn submit(&mut self, encoder: CommandEncoder) {
        self.queue.submit(Some(encoder.finish()));
    }
}
```

### Material System

```rust
// src/gfx/material.rs

use wgpu::*;
use crate::gfx::{Texture, Shader};

/// Material types
#[derive(Clone, Copy, PartialEq)]
pub enum MaterialType {
    Unlit,
    Lambert,
    Phong,
    Blinn,
    Standard, // PBR
}

/// Standard material (PBR)
pub struct Material {
    pub name: String,
    pub material_type: MaterialType,

    // Colors
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub emissive: [f32; 3],

    // PBR properties
    pub metalness: f32,
    pub roughness: f32,
    pub opacity: f32,

    // Textures
    pub diffuse_map: Option<Texture>,
    pub normal_map: Option<Texture>,
    pub roughness_map: Option<Texture>,
    pub metalness_map: Option<Texture>,

    // Rendering options
    pub cull_mode: Face,
    pub blend_mode: BlendMode,
    pub depth_write: bool,
    pub depth_test: CompareFunction,

    // Pipeline
    pub pipeline: Option<RenderPipeline>,
    pub bind_group: Option<BindGroup>,
}

#[derive(Clone, Copy)]
pub enum BlendMode {
    None,
    Alpha,
    Additive,
    Multiplicative,
}

impl Material {
    pub fn standard() -> Self {
        Self {
            name: String::from("Standard"),
            material_type: MaterialType::Standard,
            diffuse: [1.0, 1.0, 1.0],
            specular: [1.0, 1.0, 1.0],
            emissive: [0.0, 0.0, 0.0],
            metalness: 0.0,
            roughness: 0.5,
            opacity: 1.0,
            diffuse_map: None,
            normal_map: None,
            roughness_map: None,
            metalness_map: None,
            cull_mode: Face::Back,
            blend_mode: BlendMode::None,
            depth_write: true,
            depth_test: CompareFunction::Less,
            pipeline: None,
            bind_group: None,
        }
    }

    pub fn create_pipeline(&mut self, device: &Device, shader: &Shader, layout: &PipelineLayout) {
        let blend_state = match self.blend_mode {
            BlendMode::None => BlendState::REPLACE,
            BlendMode::Alpha => BlendState::ALPHA_BLENDING,
            BlendMode::Additive => BlendState::ADDITIVE,
            BlendMode::Multiplicative => BlendState::MULTIPLICATIVE,
        };

        self.pipeline = Some(device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("MaterialPipeline-{}", self.name)),
            layout: Some(layout),
            vertex: shader.vertex_state(),
            fragment: Some(shader.fragment_state()),
            primitive: PrimitiveState {
                cull_mode: Some(self.cull_mode),
                ..Default::default()
            },
            depth_stencil: if self.depth_write {
                Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: self.depth_test,
                    ..Default::default()
                })
            } else {
                None
            },
            multisample: MultisampleState::default(),
            multiview: None,
        }));
    }
}
```

### Mesh and Rendering

```rust
// src/gfx/mesh.rs

use wgpu::*;
use glam::Vec3;
use crate::gfx::{VertexBuffer, IndexBuffer, Material};

/// Vertex format
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub fn vertex_layout() -> VertexBufferLayout {
    VertexBufferLayout {
        array_stride: size_of::<Vertex>() as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 24,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 40,
                shader_location: 3,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 48,
                shader_location: 4,
            },
        ],
    }
}

/// Mesh primitive
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: Option<IndexBuffer>,
    pub primitive_type: PrimitiveTopology,
    pub index_count: u32,
    pub bounding_box: BoundingBox,
}

impl Mesh {
    pub fn new(device: &Device, vertices: &[Vertex], indices: Option<&[u32]>) -> Self {
        let vertex_buffer = VertexBuffer::new(device, vertices, &vertex_layout());

        let (index_buffer, index_count) = if let Some(indices) = indices {
            (
                Some(IndexBuffer::new(device, indices)),
                indices.len() as u32,
            )
        } else {
            (None, vertices.len() as u32)
        };

        // Calculate bounding box
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for v in vertices {
            let pos = Vec3::from_slice(&v.position);
            min = min.min(pos);
            max = max.max(pos);
        }

        Self {
            name: String::from("Mesh"),
            vertex_buffer,
            index_buffer,
            primitive_type: PrimitiveTopology::TriangleList,
            index_count,
            bounding_box: BoundingBox { min, max },
        }
    }

    pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        if let Some(ref index_buffer) = self.index_buffer {
            render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        } else {
            render_pass.draw(0..self.index_count, 0..1);
        }
    }
}

/// Mesh instance
pub struct MeshInstance {
    pub mesh: Mesh,
    pub material: Material,
    pub world_matrix: Mat4,
    pub cast_shadow: bool,
    pub receive_shadow: bool,
}
```

---

## Asset System

```rust
// src/asset/registry.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Asset handle
#[derive(Clone)]
pub struct AssetHandle<T> {
    id: Uuid,
    registry: Arc<AssetRegistry>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> AssetHandle<T> {
    pub fn get(&self) -> Option<Arc<T>> {
        self.registry.get(self.id)
    }
}

/// Asset trait
pub trait Asset: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn size_bytes(&self) -> u64;
}

/// Asset registry
pub struct AssetRegistry {
    assets: RwLock<HashMap<Uuid, Arc<dyn Any>>>,
    handles: RwLock<HashMap<String, Uuid>>,  // name -> id
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            assets: RwLock::new(HashMap::new()),
            handles: RwLock::new(HashMap::new()),
        }
    }

    pub fn add<T: Asset>(&self, id: Uuid, name: String, asset: T) -> AssetHandle<T> {
        // TODO: Insert and create handle
    }

    pub fn get<T: Asset>(&self, id: Uuid) -> Option<Arc<T>> {
        let assets = self.assets.blocking_read();
        let any = assets.get(&id)?;
        any.downcast_cloned()
    }

    pub async fn load<T: Asset + 'static>(
        &self,
        url: &str,
        loader: impl FnOnce(&str) -> anyhow::Result<T>,
    ) -> anyhow::Result<AssetHandle<T>> {
        // Load from URL
        let asset = loader(url)?;
        let id = Uuid::new_v4();

        let mut assets = self.assets.write().await;
        let mut handles = self.handles.write().await;

        assets.insert(id, Arc::new(asset));
        handles.insert(url.to_string(), id);

        Ok(AssetHandle {
            id,
            registry: Arc::new(self.clone()),
            _phantom: std::marker::PhantomData,
        })
    }
}
```

---

## Physics (rapier3d)

```rust
// src/physics/world.rs

use rapier3d::prelude::*;
use glam::{Vec3, Quat};

/// Physics world
pub struct PhysicsWorld {
    gravity: Vec3,
    pipeline: PhysicsPipeline,
    integration: IntegrationParameters,
    islands: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    ccd: CCDSolver,
    query_pipeline: QueryPipeline,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        let integration = IntegrationParameters::default();
        integration.dt = 1.0 / 60.0;

        Self {
            gravity,
            pipeline: PhysicsPipeline::new(),
            integration,
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            ccd: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }

    pub fn create_rigid_body(&mut self, desc: RigidBodyDesc) -> RigidBodyHandle {
        self.bodies.insert(desc.build())
    }

    pub fn create_collider(
        &mut self,
        shape: ColliderDesc,
        body: Option<RigidBodyHandle>,
    ) -> ColliderHandle {
        if let Some(body_handle) = body {
            self.colliders
                .insert_with_parent(shape.build(), body_handle, &mut self.bodies)
        } else {
            self.colliders.insert(shape.build())
        }
    }

    pub fn step(&mut self) {
        self.pipeline.step(
            &self.gravity,
            &self.integration,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut (),  // No events
            &(),
            &self.ccd,
            &(),
        );

        // Update query pipeline
        self.query_pipeline.update(&self.bodies, &self.colliders);
    }

    pub fn raycast(&self, from: Vec3, to: Vec3) -> Option<RayCastHit> {
        let ray = Ray::new(nalgebra::Vector3::new(from.x, from.y, from.z),
                          nalgebra::Vector3::new(to.x - from.x, to.y - from.y, to.z - from.z));

        self.query_pipeline
            .cast_ray_and_get_normal(&self.bodies, &self.colliders, &ray, f32::MAX, true, None)
            .map(|(handle, toi, normal)| RayCastHit {
                handle,
                distance: toi,
                normal: Vec3::new(normal.x, normal.y, normal.z),
            })
    }
}

pub struct RayCastHit {
    pub handle: ColliderHandle,
    pub distance: f32,
    pub normal: Vec3,
}
```

---

## Animation System

```rust
// src/animation/clip.rs

use glam::{Vec3, Quat};

/// Animation curve interpolation type
#[derive(Clone, Copy)]
pub enum InterpolationType {
    Linear,
    Step,
    Cubic,
}

/// Animation curve
pub struct AnimCurve {
    pub name: String,
    pub paths: Vec<String>,  // Bone paths
    pub times: Vec<f32>,
    pub values: Vec<f32>,
    pub interpolation: InterpolationType,
}

impl AnimCurve {
    pub fn evaluate(&self, time: f32) -> Vec<f32> {
        // Find surrounding keys
        let mut i = 0;
        while i < self.times.len() && self.times[i] < time {
            i += 1;
        }

        if i == 0 {
            return self.get_value(0);
        }
        if i >= self.times.len() {
            return self.get_value(self.times.len() - 1);
        }

        let prev = i - 1;
        let next = i;
        let t = (time - self.times[prev]) / (self.times[next] - self.times[prev]);

        match self.interpolation {
            InterpolationType::Linear => {
                self.lerp(self.get_value(prev), self.get_value(next), t)
            }
            InterpolationType::Step => self.get_value(prev),
            InterpolationType::Cubic => self.cubic(prev, next, t),
        }
    }

    fn get_value(&self, index: usize) -> Vec<f32> {
        // Extract value from values array
        todo!()
    }

    fn lerp(&self, a: Vec<f32>, b: Vec<f32>, t: f32) -> Vec<f32> {
        a.iter()
            .zip(b.iter())
            .map(|(&x, &y)| x + (y - x) * t)
            .collect()
    }

    fn cubic(&self, prev: usize, next: usize, t: f32) -> Vec<f32> {
        // Cubic interpolation with tangents
        todo!()
    }
}

/// Animation clip
pub struct AnimClip {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<AnimCurve>,
    pub loops: bool,
}

impl AnimClip {
    pub fn new(name: &str, duration: f32, tracks: Vec<AnimCurve>) -> Self {
        Self {
            name: name.to_string(),
            duration,
            tracks,
            loops: true,
        }
    }
}

/// Animation evaluator
pub struct AnimEvaluator {
    clips: Vec<PlayingClip>,
    targets: HashMap<String, AnimationTarget>,
}

struct PlayingClip {
    clip: AnimClip,
    time: f32,
    speed: f32,
    weight: f32,
    playing: bool,
}

struct AnimationTarget {
    value: Vec<f32>,
    blend_count: u32,
}

impl AnimEvaluator {
    pub fn add_clip(&mut self, clip: AnimClip) {
        self.clips.push(PlayingClip {
            clip,
            time: 0.0,
            speed: 1.0,
            weight: 1.0,
            playing: true,
        });
    }

    pub fn update(&mut self, dt: f32) {
        for clip in &mut self.clips {
            if !clip.playing {
                continue;
            }

            clip.time += dt * clip.speed;

            if clip.loops {
                clip.time %= clip.clip.duration;
            } else {
                clip.time = clip.time.min(clip.clip.duration);
            }
        }
    }

    pub fn evaluate(&mut self) -> HashMap<String, Vec<f32>> {
        // Blend all playing clips
        let mut results: HashMap<String, Vec<f32>> = HashMap::new();

        for clip in &self.clips {
            if !clip.playing {
                continue;
            }

            for track in &clip.clip.tracks {
                let value = track.evaluate(clip.time);

                // Blend with existing value
                let entry = results.entry(track.paths[0].clone()).or_insert(vec![0.0; value.len()]);
                for (i, &v) in value.iter().enumerate() {
                    entry[i] = entry[i] * (1.0 - clip.weight) + v * clip.weight;
                }
            }
        }

        results
    }
}
```

---

## Application Framework

```rust
// src/app/app.rs

use winit::window::Window;
use crate::gfx::GraphicsDevice;
use crate::ecs::World;
use crate::asset::AssetRegistry;
use crate::physics::PhysicsWorld;

/// Application configuration
pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: String::from("PlayCanvas RS"),
            width: 1280,
            height: 720,
            vync: true,
        }
    }
}

/// Main application
pub struct App {
    window: Window,
    device: GraphicsDevice,
    world: World,
    assets: AssetRegistry,
    physics: PhysicsWorld,

    // State
    delta_time: f32,
    running: bool,
}

impl App {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let window = Window::builder()
            .with_title(config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .build()?;

        let device = GraphicsDevice::new(&window).await?;

        Ok(Self {
            window,
            device,
            world: World::new(),
            assets: AssetRegistry::new(),
            physics: PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0)),
            delta_time: 0.0,
            running: true,
        })
    }

    pub fn run(&mut self) {
        let mut last_time = std::time::Instant::now();

        while self.running {
            // Calculate delta time
            let current_time = std::time::Instant::now();
            self.delta_time = current_time.duration_since(last_time).as_secs_f32();
            last_time = current_time;

            // Process events
            self.process_events();

            // Update
            self.update(self.delta_time);

            // Render
            self.render();
        }
    }

    fn process_events(&mut self) {
        // Handle window events
    }

    fn update(&mut self, dt: f32) {
        // Update systems
        self.physics.step();

        // ECS systems update
        // ...
    }

    fn render(&mut self) {
        // Render the scene
    }
}
```

---

## WGSL Shader Example

```wgsl
// shaders/standard.wgsl

struct Uniforms {
    view_projection: mat4x4<f32>,
    world_matrix: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var diffuse_texture: texture_2d<f32>;

@group(0) @binding(2)
var diffuse_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.world_position = (uniforms.world_matrix * vec4<f32>(in.position, 1.0)).xyz;
    out.clip_position = uniforms.view_projection * vec4<f32>(out.world_position, 1.0);
    out.normal = (uniforms.normal_matrix * vec4<f32>(in.normal, 0.0)).xyz;
    out.uv = in.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let diffuse_color = textureSample(diffuse_texture, diffuse_sampler, in.uv).rgb;
    let normal = normalize(in.normal);

    // Simple Lambertian lighting
    let light_dir = normalize(vec3<f32>(5.0, 10.0, 5.0));
    let ndotl = max(dot(normal, light_dir), 0.0);

    let final_color = diffuse_color * ndotl;

    return vec4<f32>(final_color, 1.0);
}
```

---

## Summary

This Rust implementation plan covers:

1. **Graphics**: wgpu for cross-platform GPU rendering (WebGPU/Vulkan/Metal/DX12)
2. **ECS**: Bevy ECS or custom implementation for entity-component-system
3. **Assets**: Async loading with tokio, glTF for models, image crate for textures
4. **Physics**: rapier3d for 3D rigid body physics
5. **Animation**: Custom animation system with clips, tracks, and blending
6. **Shaders**: WGSL shaders compiled with naga
7. **Application**: winit window management with game loop

Key advantages of Rust implementation:
- Type safety and compile-time error checking
- No garbage collection pauses
- Better performance through zero-cost abstractions
- Memory safety without runtime overhead
- Cross-platform through wgpu
