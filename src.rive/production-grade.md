# Production-Grade Rive Implementation

**What it takes to make Rive production-ready**

---

## Table of Contents

1. [Overview](#overview)
2. [Production Checklist](#production-checklist)
3. [Performance Requirements](#performance-requirements)
4. [Memory Management](#memory-management)
5. [Error Handling](#error-handling)
6. [Testing Strategy](#testing-strategy)
7. [CI/CD Pipeline](#cicd-pipeline)
8. [Documentation](#documentation)
9. [Platform Support Matrix](#platform-support-matrix)

---

## Overview

"Production-grade" means the software meets requirements for reliability, performance, and maintainability in real-world applications. This document defines what production-grade looks like for a Rive runtime implementation.

### Production vs. Prototype

| Aspect | Prototype | Production-Grade |
|--------|-----------|------------------|
| Error Handling | Panics on failure | Graceful degradation |
| Performance | "Works on my machine" | Benchmarked, optimized |
| Memory Leaks | Acceptable | Zero tolerance |
| Testing | Manual | Automated, >80% coverage |
| Documentation | Minimal | Comprehensive |
| Platform Support | One platform | All target platforms |
| Dependencies | Whatever works | Vetted, maintained |

---

## Production Checklist

### Core Functionality

- [ ] **File Loading**
  - [ ] Load all valid .riv files
  - [ ] Graceful error on corrupt files
  - [ ] Version compatibility handling
  - [ ] Memory-bounded loading (no OOM on huge files)

- [ ] **Artboard Rendering**
  - [ ] Correct transform hierarchy
  - [ ] Clip path support
  - [ ] All blend modes
  - [ ] Opacity modulation

- [ ] **Animation Playback**
  - [ ] Linear animations (oneShot, loop, pingPong)
  - [ ] State machines (all input types)
  - [ ] Blend trees
  - [ ] Nested animations

- [ ] **Shape Rendering**
  - [ ] All path types (lines, cubics)
  - [ ] Fill rules (winding, even-odd)
  - [ ] Stroke styles (miter, round, bevel)
  - [ ] Gradient fills (linear, radial)

### Performance

- [ ] **Frame Rate**
  - [ ] 60 FPS on target hardware
  - [ ] Graceful degradation on slow devices
  - [ ] Frame timing under budget (16.67ms for 60 FPS)

- [ ] **Memory**
  - [ ] No memory leaks (verified with tools)
  - [ ] Bounded memory usage
  - [ ] Efficient resource cleanup

- [ ] **Startup Time**
  - [ ] File load < 100ms for typical files
  - [ ] First frame < 200ms

### Reliability

- [ ] **Crash Testing**
  - [ ] No panics in normal operation
  - [ ] Recovery from GPU context loss
  - [ ] Thread safety where applicable

- [ ] **Edge Cases**
  - [ ] Empty files
  - [ ] Malformed files
  - [ ] Extremely large files
  - [ ] Zero-duration animations

---

## Performance Requirements

### Target Hardware

| Platform | Device | GPU | Target FPS |
|----------|--------|-----|------------|
| Desktop | Mid-range PC | GTX 1060 | 60 |
| Mobile | iPhone 8+ | Apple A11 | 60 |
| Mobile | Mid-range Android | Adreno 540 | 60 |
| Web | Modern browser | Integrated | 60 |

### Benchmarks

```rust
// Benchmark suite structure

#[cfg(test)]
mod benchmarks {
    #[bench]
    fn load_simple_file(b: &mut Bencher) {
        let data = include_bytes!("test_assets/simple.riv");
        b.iter(|| File::load(data));
    }

    #[bench]
    fn render_artboard(b: &mut Bencher) {
        let file = load_test_file();
        let artboard = file.artboard(0);
        let mut renderer = create_test_renderer();

        b.iter(|| {
            artboard.draw(&mut renderer);
        });
    }

    #[bench]
    fn update_animation(b: &mut Bencher) {
        let file = load_test_file();
        let artboard = file.artboard(0);
        let anim = artboard.animation("Walk").unwrap();
        let mut instance = anim.create_instance();

        b.iter(|| {
            instance.update(0.016);
        });
    }
}
```

### Performance Budget

| Metric | Budget | Measurement |
|--------|--------|-------------|
| Frame time | < 16ms | Frame profiler |
| Draw calls | < 100 per frame | GPU counters |
| Memory | < 50MB typical | Heap profiler |
| Load time | < 100ms | File loading timer |

### Optimization Techniques

```rust
// 1. Batching draw calls
pub struct RenderBatch {
    paths: Vec<PathDraw>,
    current_paint: Option<PaintState>,
}

impl RenderBatch {
    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        // Batch if paint state matches
        if self.current_paint.as_ref() == Some(paint) {
            self.paths.push(PathDraw::new(path));
        } else {
            self.flush();
            self.current_paint = Some(paint.clone());
            self.paths.push(PathDraw::new(path));
        }
    }

    fn flush(&mut self) {
        if !self.paths.is_empty() {
            self.renderer.draw_batch(&self.paths, &self.current_paint);
            self.paths.clear();
        }
    }
}

// 2. Object pooling for allocations
pub struct ObjectPool<T> {
    pool: Vec<T>,
}

impl<T: Default> ObjectPool<T> {
    pub fn acquire(&mut self) -> T {
        self.pool.pop().unwrap_or_default()
    }

    pub fn release(&mut self, obj: T) {
        if self.pool.len() < 100 {
            self.pool.push(obj);
        }
    }
}

// 3. Spatial caching
use std::collections::HashMap;

pub struct TessellationCache {
    cache: HashMap<PathId, TessellatedPath>,
    memory_limit: usize,
    current_memory: usize,
}

impl TessellationCache {
    pub fn get_or_tessellate<F>(&mut self, id: PathId, tessellate: F) -> &TessellatedPath
    where
        F: FnOnce() -> TessellatedPath,
    {
        use std::collections::hash_map::Entry;

        match self.cache.entry(id) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                // Evict if needed
                while self.current_memory > self.memory_limit {
                    self.evict_lru();
                }

                let path = tessellate();
                self.current_memory += path.memory_size();
                e.insert(path)
            }
        }
    }
}
```

---

## Memory Management

### Leak Detection

```rust
// Memory leak testing
#[cfg(test)]
mod leak_tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[track_caller]
    fn assert_no_leaks<F: FnOnce()>(test: F) {
        let before = ALLOCATION_COUNT.load(Ordering::SeqCst);
        test();
        let after = ALLOCATION_COUNT.load(Ordering::SeqCst);
        assert_eq!(before, after, "Memory leak detected!");
    }

    #[test]
    fn test_file_loading_leaks() {
        assert_no_leaks(|| {
            for _ in 0..100 {
                let data = include_bytes!("test.riv");
                let _file = File::load(data).unwrap();
                // File is dropped here
            }
        });
    }

    #[test]
    fn test_animation_instance_leaks() {
        assert_no_leaks(|| {
            let file = load_test_file();
            let artboard = file.artboard(0);

            for _ in 0..100 {
                let anim = artboard.animation("Walk").unwrap();
                let _instance = anim.create_instance();
                // Instance is dropped here
            }
        });
    }
}
```

### Resource Cleanup

```rust
use std::sync::Arc;

// Explicit resource cleanup pattern
pub struct GpuResources {
    buffers: Vec<Buffer>,
    textures: Vec<Texture>,
    pipelines: Vec<RenderPipeline>,
}

impl GpuResources {
    pub fn clear(&mut self) {
        // Explicit drop in specific order
        self.pipelines.clear();
        self.textures.clear();
        self.buffers.clear();
    }
}

impl Drop for GpuResources {
    fn drop(&mut self) {
        // Ensure cleanup even if clear() wasn't called
        self.clear();
    }
}

// RAII for GPU resources
pub struct MappedBuffer<'a> {
    slice: BufferSlice<'a>,
    mapping: BufferMapping,
}

impl<'a> MappedBuffer<'a> {
    pub fn map(slice: BufferSlice<'a>) -> Self {
        let mapping = slice.map();
        Self { slice, mapping }
    }

    pub fn write<T: Copy>(&mut self, data: &[T]) {
        let dest = self.mapping.as_mut_slice();
        dest.copy_from_slice(bytemuck::cast_slice(data));
    }
}

impl<'a> Drop for MappedBuffer<'a> {
    fn drop(&mut self) {
        // Unmap automatically when dropped
        self.mapping.unmap();
    }
}
```

---

## Error Handling

### Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum RiveError {
    // File errors
    #[error("Invalid file format: expected '{expected}', found '{found}'")]
    InvalidFormat { expected: String, found: String },

    #[error("Unsupported file version: {0}")]
    UnsupportedVersion(u32),

    #[error("Corrupt file: {0}")]
    CorruptFile(String),

    // Object errors
    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error("Animation not found: {0}")]
    AnimationNotFound(String),

    #[error("State machine not found: {0}")]
    StateMachineNotFound(String),

    // Rendering errors
    #[error("GPU device lost")]
    DeviceLost,

    #[error("Shader compilation failed: {0}")]
    ShaderCompilation(String),

    #[error("Texture size exceeded: requested {requested}, max {max}")]
    TextureSizeExceeded { requested: u32, max: u32 },

    // Resource errors
    #[error("Out of memory: requested {requested} bytes")]
    OutOfMemory { requested: usize },

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
}

pub type Result<T> = std::result::Result<T, RiveError>;
```

### Graceful Degradation

```rust
pub enum RenderingQuality {
    High,      // Full quality, MSAA
    Medium,    // No MSAA, reduced tessellation
    Low,       // Minimal tessellation, simplified gradients
}

pub struct Renderer {
    quality: RenderingQuality,
    capabilities: GpuCapabilities,
}

impl Renderer {
    pub fn detect_quality(device: &Device) -> RenderingQuality {
        let limits = device.limits();

        if limits.max_texture_dimension_2d >= 8192 {
            RenderingQuality::High
        } else if limits.max_texture_dimension_2d >= 2048 {
            RenderingQuality::Medium
        } else {
            RenderingQuality::Low
        }
    }

    pub fn draw_path(&mut self, path: &Path, paint: &Paint) -> Result<()> {
        match self.quality {
            RenderingQuality::High => {
                self.draw_path_high(path, paint)
            }
            RenderingQuality::Medium => {
                self.draw_path_medium(path, paint)
            }
            RenderingQuality::Low => {
                // Simplified rendering for low-end devices
                self.draw_path_simplified(path, paint)
            }
        }
    }

    fn draw_path_simplified(&mut self, path: &Path, paint: &Paint) -> Result<()> {
        // Fall back to simpler rendering techniques
        // This should never fail on valid input
        Ok(())
    }
}
```

---

## Testing Strategy

### Test Categories

```
Testing Pyramid:

           /\
          /  \
         / E2E\        End-to-End (few tests)
        /______\
       /        \
      /Integration\    Integration (more tests)
     /______________\
    /                \
   /    Unit Tests    \  Unit (many tests)
  /____________________\
```

### Unit Tests

```rust
#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn test_path_move_to() {
        let mut path = Path::new();
        path.move_to(10.0, 20.0);

        assert_eq!(path.commands().len(), 1);
        assert_eq!(path.commands()[0], PathCommand::MoveTo(10.0, 20.0));
    }

    #[test]
    fn test_path_bounds() {
        let mut path = Path::new();
        path.move_to(0.0, 0.0);
        path.line_to(100.0, 0.0);
        path.line_to(100.0, 100.0);
        path.close();

        let bounds = path.bounds();
        assert_eq!(bounds.left, 0.0);
        assert_eq!(bounds.top, 0.0);
        assert_eq!(bounds.right, 100.0);
        assert_eq!(bounds.bottom, 100.0);
    }

    #[test]
    fn test_cubic_bezier_point_at() {
        // P0=(0,0), P1=(0,100), P2=(100,100), P3=(100,0)
        let curve = CubicBezier::new(
            Vec2D::new(0.0, 0.0),
            Vec2D::new(0.0, 100.0),
            Vec2D::new(100.0, 100.0),
            Vec2D::new(100.0, 0.0),
        );

        let mid = curve.point_at(0.5);
        // At t=0.5, should be at (50, 75) approximately
        assert!((mid.x - 50.0).abs() < 0.1);
        assert!((mid.y - 75.0).abs() < 0.1);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    #[test]
    fn test_animation_playback() {
        let file = load_test_file("character.riv");
        let artboard = file.artboard("Character").unwrap();
        let animation = artboard.animation("Walk").unwrap();

        let mut instance = animation.create_instance();

        // Play for 2 seconds
        for _ in 0..120 {
            instance.update(1.0 / 60.0);
            instance.apply(&artboard);
        }

        // Verify animation completed
        match animation.loop_mode() {
            Loop::OneShot => assert!(!instance.is_playing()),
            Loop::Loop | Loop::PingPong => assert!(instance.is_playing()),
        }
    }

    #[test]
    fn test_state_machine_transitions() {
        let file = load_test_file("character.riv");
        let artboard = file.artboard("Character").unwrap();
        let sm = artboard.state_machine("Locomotion").unwrap();

        let mut instance = sm.create_instance(&artboard);

        // Set running state
        instance.set_bool("isRunning", true);

        // Update until transition occurs
        for _ in 0..60 {
            instance.update(1.0 / 60.0);

            if instance.active_state_name() == "Running" {
                break;
            }
        }

        assert_eq!(instance.active_state_name(), "Running");
    }
}
```

### Visual Regression Tests

```rust
// Screenshot comparison testing

#[test]
fn test_rendering_output() {
    let file = load_test_file("test.riv");
    let artboard = file.artboard(0);

    let mut renderer = TestRenderer::new(800, 600);
    artboard.draw(&mut renderer);

    let screenshot = renderer.screenshot();

    // Compare against golden image
    let golden = include_bytes!("golden/test_render.png");
    let golden_image = Image::from_png(golden);

    let diff = image_diff(&screenshot, &golden_image);

    // Allow small differences due to floating point
    assert!(diff.percentage < 0.01, "Visual regression: {}% different", diff.percentage);
}
```

### Coverage Requirements

```toml
# Cargo.toml
[package.metadata.cov]
required-coverage = 80
excluded = ["tests/", "examples/", "benches/"]

# Run coverage:
# cargo tarpaulin --out Html --output-dir coverage
```

---

## CI/CD Pipeline

### GitHub Actions Workflow

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]

    runs-on: ${{ matrix.os }}

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: dtolnay/rust-action@stable

    - name: Cache dependencies
      uses: Swatinem/rust-cache@v2

    - name: Build
      run: cargo build --verbose

    - name: Run tests
      run: cargo test --verbose

    - name: Run clippy
      run: cargo clippy -- -D warnings

    - name: Check formatting
      run: cargo fmt --check

  wasm:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: dtolnay/rust-action@stable
      with:
        target: wasm32-unknown-unknown

    - name: Install wasm-pack
      run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

    - name: Build WASM
      run: wasm-pack build --target web

  coverage:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: dtolnay/rust-action@stable

    - name: Install cargo-tarpaulin
      run: cargo install cargo-tarpaulin

    - name: Generate coverage
      run: cargo tarpaulin --out Xml

    - name: Upload to codecov
      uses: codecov/codecov-action@v3
```

---

## Documentation

### API Documentation

```rust
/// A Rive file containing artboards and animations.
///
/// # Examples
///
/// ```
/// use rive_rs::File;
///
/// let data = std::fs::read("animation.riv").unwrap();
/// let file = File::load(&data).unwrap();
///
/// // Get the first artboard
/// let artboard = file.artboard(0).unwrap();
/// ```
pub struct File {
    // Fields hidden from public API
}

impl File {
    /// Load a Rive file from bytes.
    ///
    /// # Errors
    ///
    /// Returns `RiveError::InvalidFormat` if the data is not a valid .riv file.
    /// Returns `RiveError::UnsupportedVersion` if the file version is too new.
    ///
    /// # Examples
    ///
    /// ```
    /// let data = include_bytes!("test.riv");
    /// let file = File::load(data).expect("Failed to load file");
    /// ```
    pub fn load(data: &[u8]) -> Result<Self> {
        // Implementation
    }

    /// Get the number of artboards in this file.
    pub fn artboard_count(&self) -> usize {
        // Implementation
    }

    /// Get an artboard by index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn artboard(&self, index: usize) -> Option<ArtboardRef<'_>> {
        // Implementation
    }
}
```

### README Structure

```markdown
# rive-rs

A Rust runtime for Rive animations.

## Quick Start

```rust
use rive_rs::{File, Renderer};

let data = std::fs::read("animation.riv")?;
let file = File::load(&data)?;
let artboard = file.artboard(0)?;

// Create renderer and draw
let mut renderer = MyRenderer::new();
artboard.draw(&mut renderer);
```

## Features

- Full .riv file support
- State machines
- Linear animations
- Multiple renderer backends

## Installation

```toml
[dependencies]
rive-rs = "0.1"
```

## Examples

See the `examples/` directory for usage examples.

## License

MIT
```

---

## Platform Support Matrix

### Officially Supported Platforms

| Platform | Status | CI Testing | Notes |
|----------|--------|------------|-------|
| Windows x64 | ✅ | GitHub Actions | D3D11, Vulkan |
| macOS x64 | ✅ | GitHub Actions | Metal |
| macOS ARM64 | ✅ | GitHub Actions | Metal (M1/M2) |
| Linux x64 | ✅ | GitHub Actions | Vulkan, OpenGL |
| iOS | ✅ | Manual | Metal |
| Android | ✅ | Manual | Vulkan, OpenGL ES |
| Web (WASM) | ✅ | GitHub Actions | WebGL 2, Canvas |

### Feature Support Matrix

| Feature | Windows | macOS | Linux | iOS | Android | Web |
|---------|---------|-------|-------|-----|---------|-----|
| Vulkan | ✅ | ❌ | ✅ | ❌ | ✅ | ❌ |
| Metal | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ |
| D3D11 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| OpenGL | ✅ | ❌ | ✅ | ❌ | ✅ | ✅ |
| WebGL | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |

---

## Summary

Production-grade software requires:

1. **Comprehensive Testing**: Unit, integration, E2E, visual regression
2. **Performance Optimization**: Benchmarks, profiling, memory management
3. **Error Handling**: Graceful degradation, clear error messages
4. **Documentation**: API docs, examples, troubleshooting
5. **CI/CD**: Automated testing, coverage reports, multi-platform builds
6. **Platform Support**: Official support matrix with testing

For related topics:
- `rust-revision.md` - Rust implementation approach
- `cpp-core-architecture.md` - C++ reference implementation
- `storage-system-guide.md` - File format documentation
