---
location: N/A - Implementation guide
repository: N/A
created_at: 2026-03-26
language: Rust, TypeScript, GLSL/WGSL
---

# Rust+WASM+WebGPU Replication Guide

## Overview

This guide explains how to replicate OMMA-style AI 3D generation and Spline-style 3D design tools using Rust, WebAssembly (WASM), and WebGPU. We cover both approaches: using `wasm-bindgen` (standard) and using the custom `foundation_wasm` binding generator (no wasm-bindgen dependency).

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Project Setup](#project-setup)
3. [WebGPU with Rust+WASM](#webgpu-with-rustwasm)
4. [3D Rendering Engine](#3d-rendering-engine)
5. [Scene Graph Implementation](#scene-graph-implementation)
6. [Animation System](#animation-system)
7. [Interaction/Raycasting](#interactionraycasting)
8. [AI Integration (OMMA-style)](#ai-integration-omma-style)
9. [Editor UI](#editor-ui)
10. [Performance Optimization](#performance-optimization)

---

## 1. Architecture Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Browser Layer                          │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   Frontend (TypeScript)                │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │   Editor    │  │   Scene     │  │   Asset       │  │  │
│  │  │   UI        │  │   Panel     │  │   Browser     │  │  │
│  │  │   (React/   │  │   (Props,   │  │   (Models,    │  │  │
│  │  │    Sledge)  │  │   Timeline) │  │   Materials)  │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  └─────────────────────────┬─────────────────────────────┘  │
│                            │ WASM FFI                        │
│  ┌─────────────────────────▼─────────────────────────────┐  │
│  │                  Rust WASM Core                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │   WebGPU    │  │   Scene     │  │   Animation   │  │  │
│  │  │   Renderer  │  │   Graph     │  │   System      │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │  │
│  │  │   Physics   │  │   Audio     │  │   Asset       │  │  │
│  │  │   Engine    │  │   Engine    │  │   Loader      │  │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Two Approaches

#### Approach A: Standard wasm-bindgen
```toml
[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Window", "CanvasElement", "WebGl2RenderingContext"
]}
# For WebGPU (experimental)
wgpu = { version = "0.19", features = ["webgpu"] }
```

#### Approach B: foundation_wasm (Custom)
```toml
[dependencies]
foundation_wasm = { path = "../../../@dev/ewe_platform/backends/foundation_wasm" }
foundation_nostd = { path = "../../../@dev/ewe_platform/foundation_nostd" }
foundation_macros = { path = "../../../@dev/ewe_platform/foundation_macros" }

[lib]
crate-type = ["cdylib", "rlib"]
```

---

## 2. Project Setup

### 2.1 Project Structure

```
my-3d-editor/
├── Cargo.toml
├── src/
│   ├── lib.rs              # WASM entry point
│   ├── renderer/
│   │   ├── mod.rs
│   │   ├── device.rs       # WebGPU device initialization
│   │   ├── pipeline.rs     # Render pipelines
│   │   ├── buffers.rs      # Vertex/uniform buffers
│   │   └── shaders.rs      # Shader compilation
│   ├── scene/
│   │   ├── mod.rs
│   │   ├── graph.rs        # Scene graph (DAG)
│   │   ├── transform.rs    # Transform hierarchy
│   │   ├── mesh.rs         # Mesh data
│   │   └── material.rs     # Material system
│   ├── animation/
│   │   ├── mod.rs
│   │   ├── clip.rs         # Animation clips
│   │   ├── track.rs        # Animation tracks
│   │   └── sampler.rs      # Keyframe sampling
│   ├── interaction/
│   │   ├── mod.rs
│   │   ├── raycast.rs      # Raycasting
│   │   └── events.rs       # Event system
│   └── ui/
│       ├── mod.rs
│       └── ...             # UI bindings
├── web/
│   ├── index.html
│   ├── main.ts
│   └── styles.css
└── shaders/
    ├── basic.wgsl
    ├── pbr.wgsl
    └── skinning.wgsl
```

### 2.2 Basic Setup with foundation_wasm

```rust
// src/lib.rs
#![no_std]
extern crate alloc;

use foundation_wasm::prelude::*;
use foundation_macros::js_export;

mod renderer;
mod scene;
mod animation;
mod interaction;

#[js_export]
pub struct Editor3D {
    renderer: renderer::WebGPURenderer,
    scene_graph: scene::SceneGraph,
    animation_system: animation::AnimationSystem,
    interaction_manager: interaction::InteractionManager,
}

#[js_export]
impl Editor3D {
    #[js_export(constructor)]
    pub fn new(canvas_id: &str) -> Self {
        // Initialize via foundation_wasm JS bridge
        Self {
            renderer: renderer::WebGPURenderer::new(canvas_id),
            scene_graph: scene::SceneGraph::new(),
            animation_system: animation::AnimationSystem::new(),
            interaction_manager: interaction::InteractionManager::new(),
        }
    }

    #[js_export]
    pub fn load_model(&mut self, url: &str) -> Result<(), Error> {
        // Load glTF model asynchronously
        todo!()
    }

    #[js_export]
    pub fn render(&mut self) -> Result<(), Error> {
        self.renderer.render(&self.scene_graph)
    }

    #[js_export]
    pub fn handle_pointer_move(&mut self, x: f32, y: f32) {
        self.interaction_manager.on_pointer_move(x, y);
    }
}

// WASM entry point
#[cfg(target_arch = "wasm32")]
#[foundation_macros::wasm_entry]
fn main() {
    // foundation_wasm runtime initialization
}
```

---

## 3. WebGPU with Rust+WASM

### 3.1 Device Initialization

```rust
// src/renderer/device.rs
use foundation_wasm::{ExternalPointer, InternalReferenceRegistry, ReturnTypeHints};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct GPUHandle {
    ptr: ExternalPointer,
}

impl GPUHandle {
    pub fn new(ptr: ExternalPointer) -> Self {
        Self { ptr }
    }

    pub fn as_ptr(&self) -> ExternalPointer {
        self.ptr
    }
}

pub struct GPUDevice {
    handle: GPUHandle,
    adapter: GPUAdapter,
    context: GPUContext,
}

pub struct GPUAdapter {
    handle: GPUHandle,
    features: GpuFeatures,
    limits: GpuLimits,
}

pub struct GPUContext {
    canvas: ExternalPointer,
    surface: GPUHandle,
    format: TextureFormat,
}

impl GPUDevice {
    pub async fn request_device(adapter: &GPUAdapter) -> Result<Self, WebGPUError> {
        // Use foundation_wasm async pattern
        let (tx, rx) = foundation_wasm::channel();

        let callback_ptr = InternalReferenceRegistry::register(
            ReturnTypeHints::ExternalPointer,
            move |result: TaskResult<ExternalPointer>| {
                let _ = tx.send(result);
            }
        );

        // Call JS: navigator.gpu.requestDevice()
        // ... via foundation_wasm JS bridge

        // Wait for async completion
        let device_ptr = rx.recv().await??;

        Ok(Self {
            handle: GPUHandle::new(device_ptr),
            adapter: adapter.clone(),
            context: GPUContext::new()?,
        })
    }

    pub fn create_buffer(&self, desc: &BufferDescriptor) -> GPUBuffer {
        // Create buffer via JS bridge
        GPUBuffer::new(self.create_buffer_js(desc))
    }

    pub fn create_texture(&self, desc: &TextureDescriptor) -> GPUTexture {
        GPUTexture::new(self.create_texture_js(desc))
    }

    pub fn create_render_pipeline(
        &self,
        desc: &RenderPipelineDescriptor
    ) -> GPURenderPipeline {
        GPURenderPipeline::new(self.create_pipeline_js(desc))
    }
}
```

### 3.2 Vertex/Fragment Shaders

```wgsl
// shaders/basic.wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_position: vec3<f32>,
};

struct Uniforms {
    model: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    normal_matrix: mat3x3<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let world_pos = uniforms.model * vec4<f32>(input.position, 1.0);
    output.world_position = world_pos.xyz;
    output.position = uniforms.projection * uniforms.view * world_pos;
    output.normal = uniforms.normal_matrix * input.normal;
    output.uv = input.uv;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple Lambertian lighting
    let light_dir = normalize(vec3<f32>(10.0, 10.0, 10.0));
    let normal = normalize(input.normal);
    let diffuse = max(dot(normal, light_dir), 0.0);

    let base_color = vec3<f32>(0.8, 0.5, 0.3);
    let lit_color = base_color * diffuse + base_color * 0.1; // ambient

    return vec4<f32>(lit_color, 1.0);
}
```

### 3.3 Render Pipeline Setup

```rust
// src/renderer/pipeline.rs
use wgpu::{
    Device,
    ShaderModule,
    RenderPipeline,
    PipelineLayout,
    BindGroupLayout,
};

pub struct BasicPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
}

impl BasicPipeline {
    pub fn new(device: &Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("basic-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/basic.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("basic-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("basic-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("basic-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: 32, // position(12) + normal(12) + uv(8)
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4, // MSAA 4x
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn create_bind_group(
        &self,
        device: &Device,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("basic-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        })
    }
}
```

---

## 4. 3D Rendering Engine

### 4.1 Mesh Data Structure

```rust
// src/scene/mesh.rs
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec2};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub tangent: Vec3,
    pub bitangent: Vec3,
}

pub struct Mesh {
    name: String,
    vertex_buffer: GPUBuffer,
    index_buffer: GPUBuffer,
    index_count: u32,
    material_handle: MaterialHandle,
}

impl Mesh {
    pub fn new(
        device: &GPUDevice,
        vertices: &[Vertex],
        indices: &[u32],
        material: MaterialHandle,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            size: (vertices.len() * size_of::<Vertex>()) as u64,
            usage: BufferUsage::VERTEX,
            label: "mesh-vertex-buffer",
        });
        vertex_buffer.write(device, 0, bytemuck::cast_slice(vertices));

        let index_buffer = device.create_buffer(&BufferDescriptor {
            size: (indices.len() * size_of::<u32>()) as u64,
            usage: BufferUsage::INDEX,
            label: "mesh-index-buffer",
        });
        index_buffer.write(device, 0, bytemuck::cast_slice(indices));

        Self {
            name: String::new(),
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            material_handle: material,
        }
    }

    pub fn draw(&self, render_pass: &mut RenderPass, transform: &Mat4) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
```

### 4.2 glTF Model Loading

```rust
// src/scene/loader.rs
use gltf::Gltf;
use image::DynamicImage;

pub struct GltfLoader<'a> {
    device: &'a GPUDevice,
}

impl<'a> GltfLoader<'a> {
    pub fn new(device: &'a GPUDevice) -> Self {
        Self { device }
    }

    pub async fn load(&self, url: &str) -> Result<LoadedModel, Error> {
        // Fetch glTF file
        let gltf_data = fetch_binary(url).await?;
        let (gltf, buffers, images) = gltf::import_slice(&gltf_data)?;

        let mut meshes = Vec::new();
        let mut materials = Vec::new();

        // Load materials
        for material_def in gltf.materials() {
            let material = self.load_material(&material_def, &images)?;
            materials.push(material);
        }

        // Load meshes
        for mesh_def in gltf.meshes() {
            for primitive in mesh_def.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // Load vertices
                let vertices: Vec<Vertex> = reader
                    .read_positions()
                    .zip(reader.read_normals())
                    .zip(reader.read_tex_coords(0))
                    .map(|((pos, norm), uv)| Vertex {
                        position: Vec3::from(pos),
                        normal: Vec3::from(norm.unwrap_or([0.0, 1.0, 0.0])),
                        uv: Vec2::from(uv.unwrap_or([0.0, 0.0]).into()),
                        tangent: Vec3::X,
                        bitangent: Vec3::Z,
                    })
                    .collect();

                // Load indices
                let indices: Vec<u32> = reader
                    .read_indices()
                    .unwrap()
                    .into_u32()
                    .collect();

                let mesh = Mesh::new(
                    self.device,
                    &vertices,
                    &indices,
                    materials[primitive.material().index().unwrap_or(0)].clone(),
                );
                meshes.push(mesh);
            }
        }

        Ok(LoadedModel { meshes, materials })
    }

    fn load_material(
        &self,
        material_def: &gltf::Material,
        images: &[DynamicImage],
    ) -> Result<Material, Error> {
        let base_color = material_def.pbr_metallic_roughness().base_color_factor();
        let metallic = material_def.pbr_metallic_roughness().metallic_factor();
        let roughness = material_def.pbr_metallic_roughness().roughness_factor();

        // Load base color texture if present
        let base_color_texture = material_def
            .pbr_metallic_roughness()
            .base_color_texture()
            .and_then(|t| self.load_texture(&images[t.texture().index()]));

        Ok(Material {
            base_color: Vec4::new(base_color[0], base_color[1], base_color[2], base_color[3]),
            metallic,
            roughness,
            base_color_texture,
        })
    }

    fn load_texture(&self, image: &DynamicImage) -> GPUTexture {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        let texture = self.device.create_texture(&TextureDescriptor {
            size: Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::COPY_DST,
        });

        texture.write(
            &self.device,
            0,
            0,
            0,
            rgba.as_ref(),
            width as u32,
            height as u32,
        );

        texture
    }
}
```

---

## 5. Scene Graph Implementation

### 5.1 Transform Hierarchy

```rust
// src/scene/graph.rs
use glam::{Mat4, Quat, Vec3};

#[derive(Clone)]
pub struct Transform {
    local_position: Vec3,
    local_rotation: Quat,
    local_scale: Vec3,

    world_matrix: Mat4,
    dirty: bool,

    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            local_position: Vec3::ZERO,
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
            world_matrix: Mat4::IDENTITY,
            dirty: true,
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn local_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.local_scale,
            self.local_rotation,
            self.local_position,
        )
    }

    pub fn update_world(&mut self, parent_world: Option<Mat4>) {
        self.world_matrix = match parent_world {
            Some(parent) => parent * self.local_matrix(),
            None => self.local_matrix(),
        };
        self.dirty = false;
    }

    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = (target - self.local_position).normalize();
        self.local_rotation = Quat::from_rotation_arc(Vec3::Z, forward);
        self.dirty = true;
    }
}

pub type NodeId = u32;

pub struct SceneNode {
    id: NodeId,
    name: String,
    transform: Transform,
    mesh: Option<MeshHandle>,
    material: Option<MaterialHandle>,
    light: Option<Light>,
    camera: Option<Camera>,
}

pub struct SceneGraph {
    nodes: Vec<SceneNode>,
    root_id: NodeId,
    free_list: Vec<NodeId>,
}

impl SceneGraph {
    pub fn new() -> Self {
        let mut nodes = Vec::new();
        // Create root node
        nodes.push(SceneNode {
            id: 0,
            name: "root".to_string(),
            transform: Transform::identity(),
            mesh: None,
            material: None,
            light: None,
            camera: None,
        });

        Self {
            nodes,
            root_id: 0,
            free_list: Vec::new(),
        }
    }

    pub fn create_node(&mut self, name: &str) -> NodeId {
        if let Some(id) = self.free_list.pop() {
            self.nodes[id as usize].name = name.to_string();
            self.nodes[id as usize].transform = Transform::identity();
            id
        } else {
            let id = self.nodes.len() as NodeId;
            self.nodes.push(SceneNode {
                id,
                name: name.to_string(),
                transform: Transform::identity(),
                mesh: None,
                material: None,
                light: None,
                camera: None,
            });
            id
        }
    }

    pub fn add_child(&mut self, parent: NodeId, child: NodeId) {
        self.nodes[child as usize].transform.parent = Some(parent);
        self.nodes[parent as usize].transform.children.push(child);
        self.nodes[child as usize].transform.dirty = true;
    }

    pub fn update_transforms(&mut self) {
        self.update_transforms_recursive(self.root_id, None);
    }

    fn update_transforms_recursive(&mut self, node_id: NodeId, parent_world: Option<Mat4>) {
        let node = &mut self.nodes[node_id as usize];
        node.transform.update_world(parent_world);

        let world = Some(node.transform.world_matrix);
        for &child_id in &node.transform.children.clone() {
            self.update_transforms_recursive(child_id, world);
        }
    }

    pub fn render(&self, renderer: &mut WebGPURenderer) {
        for node in &self.nodes {
            if let Some(ref mesh) = node.mesh {
                renderer.draw_mesh(mesh, &node.transform.world_matrix);
            }
        }
    }
}
```

---

## 6. Animation System

### 6.1 Keyframe Animation

```rust
// src/animation/clip.rs
use glam::{Vec3, Quat};

#[derive(Clone)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
    pub interpolation: InterpolationType,
}

#[derive(Clone, Copy)]
pub enum InterpolationType {
    Linear,
    Step,
    CubicSpline,
}

pub struct AnimationTrack<T> {
    target_path: String,
    keyframes: Vec<Keyframe<T>>,
}

impl<T: Clone + Interpolate> AnimationTrack<T> {
    pub fn sample(&self, time: f32) -> T {
        if self.keyframes.is_empty() {
            panic!("No keyframes");
        }

        // Wrap time for looping
        let duration = self.keyframes.last().unwrap().time;
        let time = time.rem_euclid(duration);

        // Find surrounding keyframes
        let (before, after) = self.find_keyframes(time);

        match before.interpolation {
            InterpolationType::Linear => {
                let t = (time - before.time) / (after.time - before.time);
                T::interpolate(&before.value, &after.value, t)
            }
            InterpolationType::Step => before.value.clone(),
            InterpolationType::CubicSpline => {
                // Cubic spline interpolation
                self.sample_cubic(time, before, after)
            }
        }
    }

    fn find_keyframes(&self, time: f32) -> (&Keyframe<T>, &Keyframe<T>) {
        // Binary search for surrounding keyframes
        let idx = self.keyframes
            .binary_search_by(|k| k.time.partial_cmp(&time).unwrap())
            .unwrap_or_else(|i| i);

        let before = if idx == 0 {
            &self.keyframes[0]
        } else {
            &self.keyframes[idx - 1]
        };

        let after = if idx >= self.keyframes.len() {
            &self.keyframes.last().unwrap()
        } else {
            &self.keyframes[idx]
        };

        (before, after)
    }

    fn sample_cubic(&self, time: f32, before: &Keyframe<T>, after: &Keyframe<T>) -> T {
        // Implement cubic spline interpolation
        // For position: use tangent handles
        // For rotation: use SLERP with tangent
        todo!()
    }
}

pub trait Interpolate {
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self;
}

impl Interpolate for Vec3 {
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        a.lerp(*b, t)
    }
}

impl Interpolate for Quat {
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        a.slerp(*b, t)
    }
}

pub struct AnimationClip {
    name: String,
    duration: f32,
    position_tracks: Vec<AnimationTrack<Vec3>>,
    rotation_tracks: Vec<AnimationTrack<Quat>>,
    scale_tracks: Vec<AnimationTrack<Vec3>>,
}

impl AnimationClip {
    pub fn sample(&self, time: f32, node_id: NodeId) -> TransformState {
        TransformState {
            position: self.position_tracks[node_id as usize].sample(time),
            rotation: self.rotation_tracks[node_id as usize].sample(time),
            scale: self.scale_tracks[node_id as usize].sample(time),
        }
    }
}
```

### 6.2 Animation Player

```rust
// src/animation/player.rs
pub struct AnimationState {
    clip_handle: AnimationClipHandle,
    time: f32,
    speed: f32,
    loop_animation: bool,
    weight: f32,
    is_playing: bool,
}

pub struct AnimationPlayer {
    states: Vec<AnimationState>,
    current_blend_tree: Option<BlendTree>,
}

impl AnimationPlayer {
    pub fn update(&mut self, delta_time: f32) {
        for state in &mut self.states {
            if state.is_playing {
                state.time += delta_time * state.speed;

                if state.loop_animation {
                    let duration = state.clip_handle.duration();
                    state.time = state.time.rem_euclid(duration);
                }
            }
        }
    }

    pub fn play(&mut self, clip: AnimationClipHandle) {
        self.states.push(AnimationState {
            clip_handle: clip,
            time: 0.0,
            speed: 1.0,
            loop_animation: true,
            weight: 1.0,
            is_playing: true,
        });
    }

    pub fn blend(&mut self, from: usize, to: usize, blend_time: f32) {
        // Implement animation blending
        self.current_blend_tree = Some(BlendTree {
            from_state: from,
            to_state: to,
            blend_time,
            blend_progress: 0.0,
        });
    }

    pub fn apply_to_scene(&self, scene_graph: &mut SceneGraph) {
        for state in &self.states {
            if state.is_playing {
                let transform_state = state.clip_handle.sample(state.time);
                // Apply to scene graph node
            }
        }
    }
}
```

---

## 7. Interaction/Raycasting

### 7.1 Raycast Implementation

```rust
// src/interaction/raycast.rs
use glam::{Vec3, Mat4};

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

pub struct HitTest {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub node_id: NodeId,
    pub uv: Vec2,
}

pub struct Raycaster {
    camera: Camera,
}

impl Raycaster {
    pub fn create_ray(&self, screen_x: f32, screen_y: f32, viewport: Viewport) -> Ray {
        // Convert screen coordinates to NDC (-1 to 1)
        let ndc_x = (screen_x / viewport.width) * 2.0 - 1.0;
        let ndc_y = -(screen_y / viewport.height) * 2.0 + 1.0;

        // Unproject to world space
        let near_point = self.unproject(ndc_x, ndc_y, 0.0);
        let far_point = self.unproject(ndc_x, ndc_y, 1.0);

        let direction = (far_point - near_point).normalize();

        Ray {
            origin: near_point,
            direction,
        }
    }

    fn unproject(&self, x: f32, y: f32, z: f32) -> Vec3 {
        let inv_view_proj = (self.camera.projection * self.camera.view).inverse();

        let point = inv_view_proj * Vec4::new(x, y, z, 1.0);
        point.xyz() / point.w
    }

    pub fn intersect_scene(
        &self,
        ray: &Ray,
        scene: &SceneGraph,
    ) -> Option<HitTest> {
        let mut closest: Option<HitTest> = None;

        for node in &scene.nodes {
            if let Some(ref mesh) = node.mesh {
                if let Some(hit) = self.intersect_mesh(ray, mesh, &node.transform.world_matrix) {
                    if closest.is_none() || hit.distance < closest.as_ref().unwrap().distance {
                        closest = Some(HitTest {
                            node_id: node.id,
                            ..hit
                        });
                    }
                }
            }
        }

        closest
    }

    pub fn intersect_mesh(
        &self,
        ray: &Ray,
        mesh: &Mesh,
        world_matrix: &Mat4,
    ) -> Option<HitTest> {
        // Transform ray to local space
        let inv_world = world_matrix.inverse();
        let local_origin = inv_world.transform_point3(ray.origin);
        let local_dir = (inv_world * Vec4::new(ray.direction.x, ray.direction.y, ray.direction.z, 0.0)).xyz();

        let local_ray = Ray {
            origin: local_origin,
            direction: local_dir.normalize(),
        };

        // Test each triangle
        let mut closest_dist = f32::MAX;
        let mut closest_hit = None;

        for triangle in mesh.triangles() {
            if let Some(hit) = self.ray_triangle_intersect(&local_ray, &triangle) {
                if hit.distance < closest_dist {
                    closest_dist = hit.distance;
                    closest_hit = Some(hit);
                }
            }
        }

        closest_hit
    }

    // Moller-Trumbore ray-triangle intersection
    pub fn ray_triangle_intersect(
        &self,
        ray: &Ray,
        triangle: &Triangle,
    ) -> Option<HitTest> {
        const EPSILON: f32 = 1e-7;

        let edge1 = triangle.v1 - triangle.v0;
        let edge2 = triangle.v2 - triangle.v0;
        let h = ray.direction.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < EPSILON {
            return None; // Parallel
        }

        let f = 1.0 / a;
        let s = ray.origin - triangle.v0;
        let u = f * s.dot(h);

        if u < 0.0 || u > 1.0 {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * ray.direction.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);

        if t > EPSILON {
            let hit_point = ray.at(t);
            let normal = edge1.cross(edge2).normalize();

            Some(HitTest {
                point: hit_point,
                normal,
                distance: t,
                node_id: 0,
                uv: Vec2::ZERO, // TODO: Interpolate UVs
            })
        } else {
            None
        }
    }
}
```

---

## 8. AI Integration (OMMA-style)

### 8.1 Text-to-3D Pipeline

```rust
// For OMMA-style AI generation, we need backend integration

pub struct AIGenerationClient {
    api_endpoint: String,
    http_client: HttpClient,
}

impl AIGenerationClient {
    pub async fn generate_model(
        &self,
        prompt: &str,
        style: Option<&str>,
    ) -> Result<GenerationResult, Error> {
        let response = self.http_client
            .post(&format!("{}/generate", self.api_endpoint))
            .json(&GenerateRequest {
                prompt: prompt.to_string(),
                style: style.map(String::from),
            })
            .send()
            .await?;

        let result: GenerationResult = response.json().await?;
        Ok(result)
    }

    pub async fn get_generation_status(
        &self,
        job_id: &str,
    ) -> Result<GenerationStatus, Error> {
        self.http_client
            .get(&format!("{}/status/{}", self.api_endpoint, job_id))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn download_model(
        &self,
        model_url: &str,
    ) -> Result<Vec<u8>, Error> {
        self.http_client
            .get(model_url)
            .send()
            .await?
            .bytes()
            .await
    }
}

// Backend would run ML models like:
// - DreamFusion (Score Distillation Sampling)
// - Magic3D
// - OpenAI Shap-E
// - TripoSR
```

### 8.2 ML Backend (Python Reference)

```python
# backend/generate.py - Reference implementation
import torch
from diffusers import StableDiffusionPipeline
from shap_e import ShapEModel

class TextTo3DPipeline:
    def __init__(self):
        self.sd_pipeline = StableDiffusionPipeline.from_pretrained(
            "runwayml/stable-diffusion-v1-5"
        )
        self.shap_e = ShapEModel.from_pretrained("openai/shap-e")

    def generate(
        self,
        prompt: str,
        num_steps: int = 100,
    ) -> Tuple[trimesh.Trimesh, Dict]:
        # Step 1: Generate multi-view images
        views = self.generate_multiview(prompt, num_steps)

        # Step 2: Reconstruct 3D mesh from views
        mesh = self.reconstruct_from_views(views)

        # Step 3: Optimize geometry
        mesh = self.optimize_geometry(mesh, prompt)

        # Step 4: Generate textures
        texture = self.generate_texture(mesh, prompt, views)

        return mesh, {"views": views, "texture": texture}

    def generate_multiview(
        self,
        prompt: str,
        num_steps: int,
    ) -> List[PIL.Image]:
        # Generate images from multiple angles
        angles = [0, 45, 90, 135, 180, 225, 270, 315]
        views = []

        for angle in angles:
            view_prompt = f"{prompt}, view from {angle} degrees"
            image = self.sd_pipeline(
                view_prompt,
                num_inference_steps=num_steps,
            ).images[0]
            views.append(image)

        return views
```

---

## 9. Editor UI

### 9.1 UI Integration with Sledgehammer

```rust
// Using Sledgehammer for Rust<->JS UI binding
use sledgehammer_bindgen::bindgen;

#[bindgen(web_only)]
pub async fn init_editor(canvas: HtmlCanvasElement) {
    let editor = Editor3D::new(canvas);
    // Start render loop
    request_animation_frame(editor.render_loop());
}

// Or with traditional wasm-bindgen
#[wasm_bindgen]
pub struct EditorUI {
    on_object_selected: js_sys::Function,
    on_property_changed: js_sys::Function,
}

#[wasm_bindgen]
impl EditorUI {
    pub fn select_object(&self, object_id: u32, properties: JsValue) {
        self.on_object_selected.call1(&JsValue::NULL, &properties);
    }

    pub fn update_property(&mut self, path: String, value: JsValue) {
        // Update internal state
        // Notify renderer of changes
    }
}
```

### 9.2 Property Panel

```typescript
// web/property-panel.ts
interface PropertyDefinition {
    name: string;
    type: 'vector3' | 'color' | 'number' | 'string' | 'boolean';
    value: any;
    onChange: (value: any) => void;
}

export class PropertyPanel {
    private properties: Map<string, PropertyDefinition> = new Map();

    render(object: SceneObject) {
        this.properties.clear();

        // Transform properties
        this.properties.set('position', {
            name: 'Position',
            type: 'vector3',
            value: object.transform.position,
            onChange: (v) => object.transform.position = v,
        });

        this.properties.set('rotation', {
            name: 'Rotation',
            type: 'vector3',
            value: object.transform.rotation,
            onChange: (v) => object.transform.rotation = v,
        });

        // Render UI
        this.renderProperties();
    }

    private renderProperties() {
        for (const [key, prop] of this.properties) {
            const element = this.createPropertyElement(prop);
            this.container.appendChild(element);
        }
    }
}
```

---

## 10. Performance Optimization

### 10.1 Frustum Culling

```rust
// src/renderer/culling.rs
use glam::{Vec3, Mat4};

pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    pub fn from_matrix(proj_view: Mat4) -> Self {
        let m = proj_view.to_cols_array();

        Self {
            planes: [
                Plane::from_vec4(Vec4::new(
                    m[3] + m[0],
                    m[7] + m[4],
                    m[11] + m[8],
                    m[15] + m[12],
                )),
                Plane::from_vec4(Vec4::new(
                    m[3] - m[0],
                    m[7] - m[4],
                    m[11] - m[8],
                    m[15] - m[12],
                )),
                // ... other 4 planes
            ],
        }
    }

    pub fn contains_box(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            if !plane.contains_box(min, max) {
                return false;
            }
        }
        true
    }
}

pub fn cull_objects(
    frustum: &Frustum,
    objects: &[Renderable],
) -> Vec<usize> {
    objects
        .iter()
        .enumerate()
        .filter(|(_, obj)| frustum.contains_box(obj.bounds.min, obj.bounds.max))
        .map(|(i, _)| i)
        .collect()
}
```

### 10.2 Instanced Rendering

```rust
// src/renderer/instancing.rs
pub struct InstancedMesh {
    mesh: MeshHandle,
    instance_buffer: GPUBuffer,
    instance_count: u32,
    max_instances: u32,
}

impl InstancedMesh {
    pub fn new(
        device: &GPUDevice,
        mesh: MeshHandle,
        max_instances: u32,
    ) -> Self {
        let instance_buffer = device.create_buffer(&BufferDescriptor {
            size: (max_instances as u64) * size_of::<InstanceData>() as u64,
            usage: BufferUsage::VERTEX,
            label: "instance-buffer",
        });

        Self {
            mesh,
            instance_buffer,
            instance_count: 0,
            max_instances,
        }
    }

    pub fn update_instances(
        &mut self,
        device: &GPUDevice,
        instances: &[InstanceData],
    ) {
        self.instance_buffer.write(
            device,
            0,
            bytemuck::cast_slice(instances),
        );
        self.instance_count = instances.len() as u32;
    }

    pub fn draw(&self, render_pass: &mut RenderPass) {
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw_instanced(0..self.mesh.index_count, 0..self.instance_count);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct InstanceData {
    pub model_matrix: [[f32; 4]; 4],
    pub normal_matrix: [[f32; 3]; 3],
}
```

---

## Summary

This guide covered:

1. **Two approaches**: Standard `wasm-bindgen` and custom `foundation_wasm`
2. **WebGPU setup**: Device initialization, shader compilation, pipeline creation
3. **3D rendering**: Mesh data, glTF loading, material system
4. **Scene graph**: Transform hierarchy, node management
5. **Animation**: Keyframe system, interpolation, blending
6. **Interaction**: Raycasting, hit testing, event system
7. **AI integration**: Backend pipeline for text-to-3D generation
8. **UI integration**: Property panels, editor tools
9. **Performance**: Frustum culling, instanced rendering

For a complete implementation, refer to the `foundation_wasm-webgpu-integration-guide.md` for the custom binding generator approach, and existing projects like `wgpu`, `bevy`, and `three-rs` for reference implementations.
