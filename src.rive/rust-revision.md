# Rive Rust Revision: Complete Translation Plan

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/rive-rs/`

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture Comparison](#architecture-comparison)
3. [Required Crates](#required-crates)
4. [Ownership Model](#ownership-model)
5. [Safe vs. Unsafe Boundaries](#safe-vs-unsafe-boundaries)
6. [Module Structure](#module-structure)
7. [Key Translations](#key-translations)
8. [Production Considerations](#production-considerations)

---

## Overview

This document outlines how to replicate Rive's functionality in Rust, using the existing `rive-rs` as a starting point and extending it with a native Rust renderer.

### Current State of rive-rs

```
rive-rs/
├── rive-rs/src/
│   ├── ffi.rs          # FFI bindings to C++ (~550 lines)
│   ├── ffi.cpp         # C++ FFI layer (~800 lines)
│   ├── file.rs         # File loading
│   ├── artboard/       # Artboard bindings
│   ├── linear_animation.rs
│   ├── state_machine/
│   ├── vello/          # Vello renderer integration
│   └── lib.rs
├── Cargo.toml
└── examples/
```

### Current Approach

The existing `rive-rs` uses FFI to bind to the C++ runtime:
- **Pros**: Full feature parity, battle-tested C++ core
- **Cons**: C++ dependency, FFI overhead, harder to integrate with Rust ecosystems

### Proposed Pure Rust Approach

```
┌─────────────────────────────────────────────────────────────────┐
│                     Pure Rust Runtime                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  File Loader (Pure Rust)                                  │   │
│  │  - .riv parsing                                           │   │
│  │  - Object deserialization                                 │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Animation System (Pure Rust)                             │   │
│  │  - Keyframe interpolation                                 │   │
│  │  - State machines                                         │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Renderer (wgpu / Vello / Skia)                           │   │
│  │  - Path tessellation                                      │   │
│  │  - GPU rendering                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Architecture Comparison

### C++ vs. Rust Memory Model

| Aspect | C++ (rive-runtime) | Rust (proposed) |
|--------|-------------------|-----------------|
| Ownership | Reference counting (`rcp<>`) | `Arc<T>`, `Rc<T>` |
| Memory Safety | Manual (raw pointers) | Compiler-enforced |
| Null Handling | Raw pointers (nullptr) | `Option<T>` |
| Inheritance | Virtual classes | Traits + composition |
| RTTI | Custom type system | `Any`, enums |

### Translation Mapping

```
C++                          Rust
─────────────────────────────────────────────────
class Core                   trait Core + RefCnt
rcp<T>                       Arc<T> or Rc<T>
virtual void foo()           trait fn foo(&self)
dynamic_cast<T>              downcast_ref::<T>()
std::vector<T>               Vec<T>
std::string                  String
std::unique_ptr<T>           Box<T>
std::function<T>             Box<dyn Fn(T)>
```

---

## Required Crates

### Core Dependencies

```toml
[dependencies]
# Graphics / Rendering
wgpu = "0.18"              # Cross-platform GPU API
vello = "0.1"              # GPU compute-based renderer
skrifa = "0.1"             # Font reading
png = "0.17"               # PNG decoding
jpeg = "0.5"               # JPEG decoding
webp = "0.2"               # WebP decoding

# Math
glam = "0.25"              # SIMD math library
euclid = "0.22"            # 2D geometry types
kurbo = "0.9"              # 2D curves (Beziers, etc.)

# Asset loading
ron = "0.8"                # Config files
byteorder = "1.5"          # Binary reading
zerocopy = "0.7"           # Zero-copy parsing

# Concurrency
rayon = "1.8"              # Data parallelism
parking_lot = "0.12"       # Fast locks

# Utilities
thiserror = "1.0"          # Error handling
anyhow = "1.0"             # Error context
tracing = "0.1"            # Logging/profiling
smallvec = "1.11"          # Stack-allocated vectors

# Platform
cfg-if = "1.0"             # Platform-specific code
```

### Optional Dependencies

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = "0.3"
js-sys = "0.3"
console_error_panic_hook = "0.1"

[target.'cfg(target_os = "macos")'.dependencies]
metal = "0.27"

[target.'cfg(target_os = "windows")'.dependencies]
windows = "0.52"
```

---

## Ownership Model

### Reference Counting in Rust

```rust
// C++ style: rcp<T>
// Rust translation: Arc<T> for thread-safe, Rc<T> for single-threaded

use std::sync::Arc;
use std::rc::Rc;

// For multi-threaded rendering:
pub struct Artboard {
    name: String,
    objects: Vec<Arc<dyn Component>>,
    animations: Vec<Arc<LinearAnimation>>,
}

// For single-threaded (WASM):
pub struct ArtboardSingleThreaded {
    name: String,
    objects: Vec<Rc<dyn Component>>,
    animations: Vec<Rc<LinearAnimation>>,
}
```

### Component Hierarchy

```rust
use std::any::Any;
use std::sync::{Arc, Weak};

// Base trait for all objects
pub trait Component: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn parent(&self) -> Option<Weak<dyn Component>>;
    fn set_parent(&mut self, parent: Option<Weak<dyn Component>>);

    fn children(&self) -> &[Arc<dyn Component>];

    fn update(&mut self, dirty: DirtyFlags);
    fn draw(&self, renderer: &mut dyn Renderer);
}

// Helper for downcasting
pub trait ComponentExt {
    fn downcast_ref<T: Component>(&self) -> Option<&T>;
    fn downcast_mut<T: Component>(&mut self) -> Option<&mut T>;
}

impl ComponentExt for dyn Component {
    fn downcast_ref<T: Component>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    fn downcast_mut<T: Component>(&mut self) -> &mut T {
        self.as_any_mut().downcast_mut::<T>()
    }
}
```

### Dirty Flag System

```rust
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DirtyFlags: u32 {
        const NONE              = 0;
        const TRANSFORM         = 1 << 0;
        const WORLD_TRANSFORM   = 1 << 1;
        const PATH              = 1 << 2;
        const VERTEX            = 1 << 3;
        const PAINT             = 1 << 4;
        const CLIP              = 1 << 5;
    }
}

pub struct ComponentBase {
    parent: Option<Weak<dyn Component>>,
    children: Vec<Arc<dyn Component>>,
    transform: Mat2D,
    world_transform: Mat2D,
    dirty: DirtyFlags,
}

impl ComponentBase {
    pub fn add_dirty(&mut self, flags: DirtyFlags) {
        self.dirty |= flags;
        // Propagate to parent
        if let Some(parent) = self.parent.as_ref() {
            if let Some(parent) = parent.upgrade() {
                // Parent needs to know about child's dirt
                parent.mark_child_dirty(flags);
            }
        }
    }

    pub fn has_dirty(&self, flags: DirtyFlags) -> bool {
        self.dirty.intersects(flags)
    }

    pub fn clear_dirty(&mut self, flags: DirtyFlags) {
        self.dirty &= !flags;
    }
}
```

---

## Safe vs. Unsafe Boundaries

### Safe Abstractions

```rust
// Path building - entirely safe
pub struct Path {
    verbs: Vec<PathVerb>,
    points: Vec<Vec2D>,
}

impl Path {
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.verbs.push(PathVerb::Move);
        self.points.push(Vec2D::new(x, y));
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.verbs.push(PathVerb::Line);
        self.points.push(Vec2D::new(x, y));
    }

    pub fn cubic_to(
        &mut self,
        cp1x: f32, cp1y: f32,
        cp2x: f32, cp2y: f32,
        x: f32, y: f32
    ) {
        self.verbs.push(PathVerb::Cubic);
        self.points.extend([
            Vec2D::new(cp1x, cp1y),
            Vec2D::new(cp2x, cp2y),
            Vec2D::new(x, y),
        ]);
    }
}
```

### Unsafe Boundaries

```rust
// GPU buffer mapping - requires unsafe
use wgpu::*;

pub struct GpuBuffer {
    buffer: Buffer,
    size: usize,
}

impl GpuBuffer {
    /// SAFETY: Caller must ensure buffer is mapped before calling
    pub unsafe fn write<T: Copy>(&mut self, offset: usize, data: &[T]) {
        let slice = self.buffer.slice(..);
        let mapping = slice.get_mapped_range_mut();

        // SAFETY: We know the buffer is large enough
        let dest_ptr = mapping.as_ptr() as *mut T;
        std::ptr::copy_nonoverlapping(
            data.as_ptr(),
            dest_ptr.add(offset),
            data.len(),
        );
    }

    /// SAFETY: Caller must ensure no other references exist
    pub unsafe fn read<T: Copy>(&self, offset: usize, len: usize) -> &[T] {
        let slice = self.buffer.slice(..);
        let mapping = slice.get_mapped_range();

        // SAFETY: We know the buffer is large enough and properly aligned
        std::slice::from_raw_parts(
            mapping.as_ptr().add(offset) as *const T,
            len,
        )
    }
}
```

### FFI Boundary (if using C++ core)

```rust
// FFI declarations
#[repr(C)]
pub struct RiveFile {
    _private: [u8; 0],
}

#[repr(C)]
pub struct RiveArtboard {
    _private: [u8; 0],
}

extern "C" {
    fn rive_load_file(data: *const u8, len: usize) -> *mut RiveFile;
    fn rive_file_delete(file: *mut RiveFile);
    fn rive_file_artboard_by_name(
        file: *mut RiveFile,
        name: *const c_char
    ) -> *mut RiveArtboard;
}

// Safe wrapper
pub struct File {
    ptr: NonNull<RiveFile>,
}

impl File {
    pub fn load(data: &[u8]) -> Result<Self, FileError> {
        let ptr = unsafe {
            rive_load_file(data.as_ptr(), data.len())
        };

        NonNull::new(ptr)
            .map(|ptr| File { ptr })
            .ok_or(FileError::InvalidFormat)
    }

    pub fn artboard(&self, name: &str) -> Option<ArtboardRef<'_>> {
        let name_cstr = CString::new(name).ok()?;

        let ptr = unsafe {
            rive_file_artboard_by_name(self.ptr.as_mut(), name_cstr.as_ptr())
        };

        NonNull::new(ptr).map(|ptr| ArtboardRef {
            ptr,
            _marker: PhantomData,
        })
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            rive_file_delete(self.ptr.as_mut());
        }
    }
}
```

---

## Module Structure

### Proposed Crate Layout

```
rive-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Crate root
│   │
│   ├── core/               # Core traits and types
│   │   ├── mod.rs
│   │   ├── component.rs    # Component trait
│   │   ├── ref_count.rs    # Reference counting
│   │   ├── rtti.rs         # Run-time type info
│   │   └── error.rs        # Error types
│   │
│   ├── file/               # File loading
│   │   ├── mod.rs
│   │   ├── parser.rs       # .riv binary parser
│   │   ├── importer.rs     # Object importers
│   │   └── assets.rs       # Asset loading
│   │
│   ├── artboard/           # Artboard implementation
│   │   ├── mod.rs
│   │   ├── artboard.rs     # Artboard struct
│   │   └── component_list.rs
│   │
│   ├── animation/          # Animation system
│   │   ├── mod.rs
│   │   ├── linear.rs       # Linear animations
│   │   ├── keyframe.rs     # Keyframe types
│   │   ├── interpolation.rs # Interpolation algorithms
│   │   └── instance.rs     # Animation instances
│   │
│   ├── state_machine/      # State machines
│   │   ├── mod.rs
│   │   ├── machine.rs      # StateMachine struct
│   │   ├── state.rs        # State definitions
│   │   ├── transition.rs   # Transitions
│   │   ├── input.rs        # Input types
│   │   └── instance.rs     # Runtime instances
│   │
│   ├── shapes/             # Vector shapes
│   │   ├── mod.rs
│   │   ├── path.rs         # Path types
│   │   ├── shape.rs        # Shape container
│   │   ├── vertex.rs       # Vertex types
│   │   └── paint.rs        # Fill/stroke
│   │
│   ├── math/               # Math utilities
│   │   ├── mod.rs
│   │   ├── vec2.rs         # 2D vectors
│   │   ├── mat2.rs         # 2D matrices
│   │   ├── rect.rs         # Rectangles
│   │   └── bezier.rs       # Bezier curves
│   │
│   ├── renderer/           # Renderer abstraction
│   │   ├── mod.rs
│   │   ├── trait.rs        # Renderer trait
│   │   ├── path.rs         # RenderPath
│   │   └── paint.rs        # RenderPaint
│   │
│   ├── tessellation/       # Path tessellation
│   │   ├── mod.rs
│   │   ├── flattener.rs    # Curve flattening
│   │   ├── triangulator.rs # Mesh generation
│   │   └── stroke.rs       # Stroke extrusion
│   │
│   └── backends/           # Renderer backends
│       ├── mod.rs
│       ├── wgpu/           # wgpu backend
│       ├── vello/          # Vello backend
│       └── skia/           # Skia backend (optional)
│
├── tests/
│   ├── file_loading.rs
│   ├── animation_playback.rs
│   └── rendering.rs
│
└── examples/
    ├── viewer.rs           # File viewer
    ├── interactive.rs      # State machine demo
    └── web/                # WASM examples
```

---

## Key Translations

### Artboard

```rust
// C++: Artboard (~2000 lines)
// Rust translation:

use std::sync::Arc;

pub struct Artboard {
    base: ComponentBase,
    name: String,
    width: f32,
    height: f32,
    clip: bool,

    // Owned objects
    objects: Vec<Arc<dyn Component>>,
    animations: Vec<Arc<LinearAnimation>>,
    state_machines: Vec<Arc<StateMachine>>,

    // Runtime state
    animation_instances: Vec<LinearAnimationInstance>,
    state_machine_instances: Vec<StateMachineInstance>,

    // Layout
    layout: Layout,
    dirty_layout: bool,
}

impl Artboard {
    pub fn advance(&mut self, elapsed: f32) {
        // Update animations
        for instance in &mut self.animation_instances {
            if instance.is_playing() {
                instance.update(elapsed);
                instance.apply(self);
            }
        }

        // Update state machines
        for instance in &mut self.state_machine_instances {
            instance.update(elapsed);
        }

        // Update hierarchy
        self.update_hierarchy();
    }

    fn update_hierarchy(&mut self) {
        // Collect and update dirty components
        // ... (see dirt system above)
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        renderer.save();
        self.draw_components(renderer);
        renderer.restore();
    }
}
```

### Linear Animation

```rust
// C++: LinearAnimation (~450 lines)
// Rust translation:

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Loop {
    OneShot,
    Loop,
    PingPong,
}

pub struct LinearAnimation {
    name: String,
    duration: f32,        // In frames
    fps: f32,
    loop_mode: Loop,
    quantize: bool,
    speed: f32,

    work_start: u32,
    work_end: u32,
    enable_work_area: bool,

    keyed_objects: Vec<KeyedObject>,
}

impl LinearAnimation {
    pub fn create_instance(&self) -> LinearAnimationInstance {
        LinearAnimationInstance::new(self)
    }

    pub fn start_seconds(&self) -> f32 {
        if self.enable_work_area {
            self.work_start as f32 / self.fps
        } else {
            0.0
        }
    }

    pub fn end_seconds(&self) -> f32 {
        if self.enable_work_area {
            self.work_end as f32 / self.fps
        } else {
            self.duration / self.fps
        }
    }

    pub fn duration_seconds(&self) -> f32 {
        (self.end_seconds() - self.start_seconds()).abs()
    }

    /// Convert global time to local animation time
    pub fn global_to_local(&self, seconds: f32) -> f32 {
        match self.loop_mode {
            Loop::OneShot => seconds + self.start_seconds(),
            Loop::Loop => {
                positive_mod(seconds, self.duration_seconds()) + self.start_seconds()
            }
            Loop::PingPong => {
                let local = positive_mod(seconds, self.duration_seconds());
                let direction = ((seconds / self.duration_seconds()) as i32) % 2;
                if direction == 0 {
                    local + self.start_seconds()
                } else {
                    self.end_seconds() - local
                }
            }
        }
    }
}

fn positive_mod(x: f32, modulus: f32) -> f32 {
    let result = x % modulus;
    if result < 0.0 { result + modulus } else { result }
}
```

### Animation Instance

```rust
pub struct LinearAnimationInstance {
    animation: Arc<LinearAnimation>,
    time: f32,
    time_direction: f32,  // 1.0 or -1.0 for pingPong
    loop_count: u32,
    is_playing: bool,
}

impl LinearAnimationInstance {
    pub fn new(animation: Arc<LinearAnimation>) -> Self {
        Self {
            animation,
            time: 0.0,
            time_direction: 1.0,
            loop_count: 0,
            is_playing: true,
        }
    }

    pub fn update(&mut self, elapsed: f32) -> bool {
        self.time += elapsed * self.animation.speed * self.time_direction;

        let end = self.animation.end_seconds();
        let start = self.animation.start_seconds();

        if self.time >= end {
            match self.animation.loop_mode {
                Loop::OneShot => {
                    self.is_playing = false;
                    return false;
                }
                Loop::Loop => {
                    self.time = start;
                    self.loop_count += 1;
                }
                Loop::PingPong => {
                    self.time_direction = -1.0;
                }
            }
        } else if self.time <= start && self.time_direction < 0.0 {
            self.time_direction = 1.0;
            self.loop_count += 1;
        }

        true
    }

    pub fn apply(&self, artboard: &mut Artboard, mix: f32) {
        let local_time = self.animation.global_to_local(self.time);

        for keyed_object in &self.animation.keyed_objects {
            keyed_object.apply(artboard, local_time, mix);
        }
    }
}
```

### Keyed Object

```rust
pub trait KeyedProperty: Send + Sync {
    fn apply(&self, artboard: &mut Artboard, time: f32, mix: f32);
}

pub struct NumberKeyedProperty {
    target_path: PropertyPath,
    keyframes: Vec<Keyframe<f32>>,
}

impl KeyedProperty for NumberKeyedProperty {
    fn apply(&self, artboard: &mut Artboard, time: f32, mix: f32) {
        // Find surrounding keyframes
        let (before, after) = match self.find_keyframes(time) {
            Some(kfs) => kfs,
            None => return,
        };

        // Calculate interpolation factor
        let t = (time - before.time) / (after.time - before.time);

        // Apply easing
        let eased_t = before.easing.evaluate(t);

        // Interpolate value
        let value = lerp(before.value, after.value, eased_t);

        // Apply to artboard
        if let Some(target) = self.resolve_target(artboard) {
            target.set_value(value, mix);
        }
    }
}

pub struct Keyframe<T> {
    time: f32,
    value: T,
    easing: EasingCurve,
}

pub enum EasingCurve {
    Linear,
    CubicBezier(f32, f32, f32, f32),  // P1x, P1y, P2x, P2y
    Steps(u32),
}

impl EasingCurve {
    pub fn evaluate(&self, t: f32) -> f32 {
        match self {
            EasingCurve::Linear => t,
            EasingCurve::CubicBezier(p1x, p1y, p2x, p2y) => {
                cubic_bezier(t, *p1x, *p1y, *p2x, *p2y)
            }
            EasingCurve::Steps(n) => {
                (t * *n as f32).floor() / *n as f32
            }
        }
    }
}
```

### State Machine

```rust
use std::collections::HashMap;

pub enum InputValue {
    Bool(bool),
    Number(f32),
    Trigger,
}

pub struct StateMachine {
    name: String,
    layers: Vec<StateMachineLayer>,
    inputs: HashMap<String, InputValue>,
    listeners: Vec<StateMachineListener>,
}

pub struct StateMachineInstance {
    machine: Arc<StateMachine>,
    artboard: Weak<Artboard>,

    // Runtime state
    active_states: Vec<usize>,  // Indices into layers
    input_cache: Vec<InputValue>,
    transition_progress: f32,
    transitioning: bool,
}

impl StateMachineInstance {
    pub fn set_bool(&mut self, name: &str, value: bool) {
        if let Some(input) = self.machine.inputs.get_mut(name) {
            *input = InputValue::Bool(value);
            self.mark_input_dirty(name);
        }
    }

    pub fn set_number(&mut self, name: &str, value: f32) {
        if let Some(input) = self.machine.inputs.get_mut(name) {
            *input = InputValue::Number(value);
            self.mark_input_dirty(name);
        }
    }

    pub fn fire_trigger(&mut self, name: &str) {
        if let Some(input) = self.machine.inputs.get_mut(name) {
            *input = InputValue::Trigger;
            self.mark_input_dirty(name);
        }
    }

    pub fn update(&mut self, elapsed: f32) {
        // Update active states
        for &state_idx in &self.active_states {
            let state = &self.machine.layers[0].states[state_idx];
            state.update(self, elapsed);
        }

        // Check transitions
        if !self.transitioning {
            self.check_transitions();
        }

        // Update transition
        if self.transitioning {
            self.update_transition(elapsed);
        }
    }

    fn check_transitions(&mut self) {
        for &state_idx in &self.active_states {
            let state = &self.machine.layers[0].states[state_idx];

            for transition in &state.transitions {
                if transition.can_fire(self) {
                    self.start_transition(transition);
                    return;
                }
            }
        }
    }
}
```

---

## Renderer Implementation

### Renderer Trait

```rust
pub trait Renderer: Send {
    fn save(&mut self);
    fn restore(&mut self);

    fn transform(&mut self, matrix: &Mat2D);

    fn clip_path(&mut self, path: &dyn RenderPath);

    fn draw_path(&mut self, path: &dyn RenderPath, paint: &dyn RenderPaint);

    fn draw_image(
        &mut self,
        image: &dyn RenderImage,
        transform: &Mat2D,
        opacity: f32,
    );

    fn modulate_opacity(&mut self, opacity: f32);
}
```

### wgpu Backend

```rust
use wgpu::*;

pub struct WgpuRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: RenderPipeline,

    // State stack
    state_stack: Vec<RenderState>,

    // Buffers
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    uniform_buffer: Buffer,

    // Current state
    current_transform: Mat2D,
    current_opacity: f32,
}

struct RenderState {
    transform: Mat2D,
    opacity: f32,
    clip_rect: Option<Rect>,
}

impl Renderer for WgpuRenderer {
    fn save(&mut self) {
        let copy = self.state_stack.last().cloned()
            .unwrap_or(RenderState {
                transform: Mat2D::identity(),
                opacity: 1.0,
                clip_rect: None,
            });
        self.state_stack.push(copy);
    }

    fn restore(&mut self) {
        if self.state_stack.len() > 1 {
            self.state_stack.pop();
        }
    }

    fn transform(&mut self, matrix: &Mat2D) {
        let state = self.state_stack.last_mut().unwrap();
        state.transform = state.transform * matrix;
    }

    fn clip_path(&mut self, path: &dyn RenderPath) {
        // Use stencil buffer for clipping
        // ...
    }

    fn draw_path(&mut self, path: &dyn RenderPath, paint: &dyn RenderPaint) {
        // Tessellate path if needed
        // Upload vertices
        // Set pipeline state
        // Draw
    }
}
```

---

## Production Considerations

### 1. Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RiveError {
    #[error("Invalid file format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u32),

    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error("Animation not found: {0}")]
    AnimationNotFound(String),

    #[error("GPU error: {0}")]
    GpuError(#[from] wgpu::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, RiveError>;
```

### 2. Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_loading() {
        let data = include_bytes!("../test_assets/test.riv");
        let file = File::load(data).unwrap();
        assert!(file.artboard_count() > 0);
    }

    #[test]
    fn test_animation_playback() {
        let file = load_test_file();
        let artboard = file.artboard(0);
        let animation = artboard.animation("Walk").unwrap();
        let mut instance = animation.create_instance();

        instance.update(0.016);  // 60 FPS frame
        assert!(instance.is_playing());
    }

    #[test]
    fn test_state_machine() {
        let file = load_test_file();
        let artboard = file.artboard(0);
        let sm = artboard.state_machine("Player").unwrap();
        let mut instance = sm.create_instance(&artboard);

        instance.set_bool("isRunning", true);
        instance.update(0.016);
    }
}
```

### 3. Performance Profiling

```rust
use tracing::{info_span, instrument};

pub struct Profiler {
    spans: HashMap<String, Instant>,
}

impl Profiler {
    #[instrument(skip(self))]
    pub fn begin_frame(&mut self) {
        // Start frame timing
    }

    #[instrument(skip(self))]
    pub fn begin_section(&mut self, name: &str) {
        self.spans.insert(name.to_string(), Instant::now());
    }

    pub fn end_section(&mut self, name: &str) -> Duration {
        self.spans.remove(&name)
            .map(|start| start.elapsed())
            .unwrap_or_default()
    }
}

// Usage:
#[tracing::instrument(skip(renderer))]
fn render_frame(artboard: &Artboard, renderer: &mut dyn Renderer) {
    // Function is automatically timed and logged
}
```

### 4. WASM Considerations

```rust
// For WASM builds, use single-threaded types
#[cfg(target_arch = "wasm32")]
pub type Arc<T> = std::rc::Rc<T>;

#[cfg(not(target_arch = "wasm32"))]
pub type Arc<T> = std::sync::Arc<T>;

// Use wasm-bindgen for JS interop
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmRive {
    file: File,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmRive {
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Result<WasmRive, JsValue> {
        let file = File::load(data)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(Self { file })
    }

    #[wasm_bindgen]
    pub fn artboard(&self, name: &str) -> Option<WasmArtboard> {
        // ...
    }
}
```

---

## Summary

A pure Rust implementation of Rive would provide:

1. **Memory Safety**: No manual memory management, no null pointer issues
2. **Modern Concurrency**: `async/await`, `Rayon` for parallelism
3. **Better Integration**: Native Rust types, no FFI needed
4. **Cross-Platform**: Same code for desktop, mobile, web

### Implementation Phases

1. **Phase 1**: Core object system (Component, Artboard)
2. **Phase 2**: File loading and parsing
3. **Phase 3**: Animation system (keyframes, interpolation)
4. **Phase 4**: State machines
5. **Phase 5**: Renderer backends (wgpu, Vello)
6. **Phase 6**: WASM bindings

For related topics:
- `cpp-core-architecture.md` - C++ implementation details
- `rendering-engine-deep-dive.md` - GPU rendering
- `production-grade.md` - Production checklist
