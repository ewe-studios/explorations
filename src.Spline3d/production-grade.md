# Production-Grade Spline 3D Implementation

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Spline3d/`

This document covers production considerations for building and deploying a Spline 3D-like application.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Performance Optimization](#performance-optimization)
3. [Asset Pipeline](#asset-pipeline)
4. [Security Considerations](#security-considerations)
5. [Testing Strategy](#testing-strategy)
6. [Deployment](#deployment)
7. [Monitoring and Analytics](#monitoring-and-analytics)
8. [Scalability](#scalability)

---

## Architecture Overview

### Production Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                  Production Architecture                         │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    CDN Edge Layer                          │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │   Static    │  │   WASM      │  │   Texture       │   │  │
│  │  │   Assets    │  │   Modules   │  │   Assets        │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   Application Layer                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │   React     │  │   Spline    │  │   State         │   │  │
│  │  │   Frontend  │  │   Runtime   │  │   Management    │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Backend Services                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │   Scene     │  │   Asset     │  │   User          │   │  │
│  │  │   Storage   │  │   Processing│  │   Management    │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   Data Layer                               │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │  │
│  │  │   Object    │  │   Scene     │  │   Analytics     │   │  │
│  │  │   Storage   │  │   Database  │  │   Database      │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Frontend | React, TypeScript | UI framework |
| 3D Runtime | Rust → WASM | Core engine |
| Rendering | WebGPU/WebGL | Graphics |
| CDN | Cloudflare, Fastly | Asset delivery |
| Backend | Node.js, Rust | API services |
| Storage | S3, R2 | Asset storage |
| Database | PostgreSQL, Redis | Data persistence |

---

## Performance Optimization

### WASM Bundle Optimization

```toml
# Cargo.toml - Release profile optimization
[profile.release]
opt-level = 3           # Maximum optimization
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit for better optimization
panic = "abort"         # Smaller binary, no unwind
strip = true            # Strip debug symbols

# Size optimization
[profile.release-small]
inherits = "release"
opt-level = "z"         # Optimize for size
lto = true
codegen-units = 1
```

```bash
# Post-build optimization with wasm-opt
wasm-opt -O4 spline3d.wasm -o spline3d.optimized.wasm

# Compression
gzip -9 spline3d.optimized.wasm
brotli -Z spline3d.optimized.wasm

# Typical results:
# Original:     2.5 MB
# wasm-opt:     1.8 MB
# gzip:         500 KB
# brotli:       400 KB
```

### Lazy Loading Strategy

```javascript
// Dynamic WASM loading
class WasmLoader {
  constructor() {
    this.module = null;
    this.instance = null;
  }

  async load(moduleUrl) {
    if (this.module) return this.instance;

    // Check for supported features
    const supportsWebGPU = !!navigator.gpu;
    const backend = supportsWebGPU ? 'webgpu' : 'webgl';

    // Load appropriate WASM module
    const response = await fetch(`${moduleUrl}/spline3d-${backend}.wasm`);
    const buffer = await response.arrayBuffer();

    // Compile and instantiate
    this.module = await WebAssembly.compile(buffer);
    this.instance = await WebAssembly.instantiate(this.module, {
      env: {
        memory: new WebAssembly.Memory({ initial: 256 }),
        table: new WebAssembly.Table({ initial: 0, element: 'anyfunc' }),
      }
    });

    return this.instance;
  }
}

// Usage with React.lazy
const SplineCanvas = React.lazy(() => import('./SplineCanvas'));

function App() {
  return (
    <Suspense fallback={<LoadingSpinner />}>
      <SplineCanvas sceneUrl={sceneUrl} />
    </Suspense>
  );
}
```

### Rendering Optimization

```rust
// Instanced rendering for repeated objects
pub struct InstancedRenderer {
    instances: Vec<InstanceData>,
    instance_buffer: wgpu::Buffer,
    max_instances: u32,
}

struct InstanceData {
    model_matrix: [[f32; 4]; 4],
    normal_matrix: [[f32; 4]; 4],
}

impl InstancedRenderer {
    pub fn draw(&mut self, render_pass: &mut wgpu::RenderPass, vertex_count: u32) {
        // Update instance buffer
        self.instance_buffer.write_data(&self.instances);

        // Single draw call for all instances
        render_pass.draw_instanced(0..vertex_count, 0..self.instances.len() as u32);
    }
}

// Frustum culling
pub fn cull_objects(
    objects: &[Object],
    frustum: &Frustum,
) -> Vec<ObjectId> {
    objects
        .iter()
        .filter(|obj| frustum.intersects(&obj.bounds))
        .map(|obj| obj.id)
        .collect()
}

// Level of Detail
pub enum LOD {
    High { mesh: Mesh, threshold: f32 },
    Medium { mesh: Mesh, threshold: f32 },
    Low { mesh: Mesh, threshold: f32 },
}

impl LOD {
    pub fn select(&self, distance: f32) -> &Mesh {
        if distance < self.High.threshold {
            &self.High.mesh
        } else if distance < self.Medium.threshold {
            &self.Medium.mesh
        } else {
            &self.Low.mesh
        }
    }
}
```

### Memory Management

```javascript
// Buffer pooling for WebGL/WASM
class BufferPool {
  constructor(size, initialCount = 10) {
    this.buffers = [];
    this.size = size;
    for (let i = 0; i < initialCount; i++) {
      this.buffers.push(new ArrayBuffer(size));
    }
  }

  acquire() {
    return this.buffers.pop() || new ArrayBuffer(this.size);
  }

  release(buffer) {
    // Clear buffer before returning to pool
    new Uint8Array(buffer).fill(0);
    this.buffers.push(buffer);
  }

  get available() {
    return this.buffers.length;
  }
}

// Texture atlasing
class TextureAtlas {
  constructor(size = 2048) {
    this.size = size;
    this.canvas = document.createElement('canvas');
    this.canvas.width = size;
    this.canvas.height = size;
    this.ctx = this.canvas.getContext('2d');
    this.regions = new Map();
    this.packer = new Packer(size, size);
  }

  addTexture(image, id) {
    const region = this.packer.pack(image.width, image.height);
    if (region) {
      this.ctx.drawImage(image, region.x, region.y);
      this.regions.set(id, {
        u: region.x / this.size,
        v: region.y / this.size,
        w: region.w / this.size,
        h: region.h / this.size,
      });
    }
    return this.regions.get(id);
  }
}
```

---

## Asset Pipeline

### Scene File Format

```
.splinecode (Binary Format)
├── Header (64 bytes)
│   ├── Magic: "SPLN" (4 bytes)
│   ├── Version: u32 (4 bytes)
│   ├── Flags: u32 (4 bytes)
│   ├── Scene Size: u64 (8 bytes)
│   ├── Object Count: u32 (4 bytes)
│   ├── Animation Count: u32 (4 bytes)
│   └── Reserved (32 bytes)
│
├── Object Table
│   ├── Object Count: u32
│   └── Objects[]
│       ├── ID: u64
│       ├── Parent ID: u64
│       ├── Name: string
│       ├── Type: u8
│       ├── Transform: f32[16]
│       └── Properties: binary
│
├── Geometry Section
│   ├── Mesh Count: u32
│   └── Meshes[]
│       ├── Vertex Count: u32
│       ├── Index Count: u32
│       ├── Vertices: f32[n][3]
│       ├── Normals: f32[n][3]
│       ├── UVs: f32[n][2]
│       └── Indices: u32[n]
│
├── Material Section
│   ├── Material Count: u32
│   └── Materials[]
│       ├── Albedo: rgba
│       ├── Roughness: f32
│       ├── Metalness: f32
│       ├── Normal Map: texture_id
│       └── Shader: shader_id
│
├── Texture Section
│   ├── Texture Count: u32
│   └── Textures[]
│       ├── Format: u8
│       ├── Width: u32
│       ├── Height: u32
│       ├── Mip Levels: u8
│       └── Data: bytes
│
└── Animation Section
    ├── Animation Count: u32
    └── Animations[]
        ├── Target ID: u64
        ├── Property: u8
        ├── Keyframe Count: u32
        └── Keyframes[]
            ├── Time: f32
            ├── Value: f32[4]
            └── Interpolation: u8
```

### Asset Processing Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                   Asset Processing Pipeline                      │
│                                                                  │
│  Designer Exports (.spline)                                      │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   Validator  │  Check scene integrity                        │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   Optimizer  │  Mesh decimation, texture compression         │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   Compiler   │  Convert to .splinecode binary                │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   Bundler    │  Package with dependencies                    │
│  └──────┬───────┘                                               │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐                                               │
│  │   CDN Upload │  Distribute to edge locations                 │
│  └──────────────┘                                               │
└─────────────────────────────────────────────────────────────────┘
```

### Texture Compression

```rust
// KTX2 texture compression
use ktx2::{compress, CompressOptions};

pub fn compress_texture(
    data: &[u8],
    width: u32,
    height: u32,
    format: TextureFormat,
) -> Vec<u8> {
    let options = CompressOptions {
        quality: ktx2::Quality::NORMAL,
        compression: ktx2::Compression::ZSTD,
        mipmap_generation: true,
        ..Default::default()
    };

    compress(data, width, height, format, options)
}

// Supported formats by platform
pub enum TextureCompression {
    ASTC,    // iOS, modern Android
    BC7,     // Desktop
    ETC2,    // Android
    PVRTC,   // Older iOS
}

impl TextureCompression {
    pub fn for_platform(platform: &str) -> Self {
        match platform {
            "ios" => TextureCompression::ASTC,
            "android" => TextureCompression::ETC2,
            "web" => TextureCompression::BC7,
            _ => TextureCompression::BC7,
        }
    }
}
```

### Mesh Optimization

```rust
// Mesh optimization with meshopt
use meshopt::{
    simplify,
    optimize_vertex_cache,
    optimize_overdraw,
    optimize_vertex_fetch,
};

pub fn optimize_mesh(
    vertices: &[[f32; 3]],
    indices: &[u32],
    target_count: usize,
) -> (Vec<[f32; 3]>, Vec<u32>) {
    // Simplify mesh
    let (simp_vertices, simp_indices) = simplify(
        vertices,
        indices,
        target_count,
        true, // lock borders
    );

    // Optimize vertex cache
    let cache_indices = optimize_vertex_cache(&simp_indices, simp_vertices.len());

    // Optimize overdraw
    let overdraw_indices = optimize_overdraw(
        &cache_indices,
        &simp_vertices,
        1.05, // threshold
    );

    // Optimize vertex fetch
    let (final_vertices, fetch_indices) = optimize_vertex_fetch(
        &simp_vertices,
        &overdraw_indices,
    );

    (final_vertices, fetch_indices)
}
```

---

## Security Considerations

### WASM Security

```rust
// Validate scene data before processing
pub fn validate_scene(data: &[u8]) -> Result<(), SceneError> {
    // Check header
    if data.len() < 64 {
        return Err(SceneError::InvalidHeader);
    }

    // Validate magic bytes
    if &data[0..4] != b"SPLN" {
        return Err(SceneError::InvalidMagic);
    }

    // Validate sizes
    let object_count = u32::from_le_bytes(data[16..20].try_into()?);
    if object_count > 100000 {
        return Err(SceneError::TooManyObjects);
    }

    // Validate offsets
    // ...

    Ok(())
}

// Resource limits
pub struct ResourceLimits {
    pub max_objects: usize,
    pub max_vertices: usize,
    pub max_textures: usize,
    pub max_texture_size: usize,
    pub max_animation_duration: f32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_objects: 10000,
            max_vertices: 1_000_000,
            max_textures: 1000,
            max_texture_size: 4096,
            max_animation_duration: 3600.0,
        }
    }
}
```

### Content Security

```javascript
// CSP headers for 3D content
const cspHeaders = {
  'Content-Security-Policy': [
    "default-src 'self'",
    "script-src 'self' 'wasm-unsafe-eval'",
    "worker-src 'self' blob:",
    "img-src 'self' blob: data: https://cdn.spline.design",
    "connect-src 'self' https://api.spline.design",
    "worker-src blob:",
  ].join('; '),
};

// Validate scene URLs
function validateSceneUrl(url) {
  try {
    const parsed = new URL(url);

    // Only allow trusted domains
    const trustedDomains = [
      'prod.spline.design',
      'cdn.spline.design',
      window.location.hostname,
    ];

    if (!trustedDomains.includes(parsed.hostname)) {
      throw new Error('Untrusted scene source');
    }

    // Validate protocol
    if (!['https:', 'blob:', 'data:'].includes(parsed.protocol)) {
      throw new Error('Invalid protocol');
    }

    return true;
  } catch (e) {
    console.error('Invalid scene URL:', e);
    return false;
  }
}
```

### Input Sanitization

```rust
// Sanitize object names
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(64)
        .collect()
}

// Validate transform values
pub fn validate_transform(transform: &Transform) -> bool {
    // Check for NaN/Infinity
    transform.translation.iter().all(|&v| v.is_finite())
        && transform.rotation.iter().all(|&v| v.is_finite())
        && transform.scale.iter().all(|&v| v.is_finite() && v > 0.0)
}
```

---

## Testing Strategy

### Unit Testing

```rust
// crates/spline3d-curves/tests/bezier_tests.rs
#[cfg(test)]
mod bezier_tests {
    use spline3d_curves::CubicBezier;
    use nalgebra::Vector3;

    #[test]
    fn test_bezier_start_end() {
        let bezier = CubicBezier::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(2.0, 1.0, 0.0),
            Vector3::new(3.0, 0.0, 0.0),
        );

        assert_eq!(bezier.evaluate(0.0), Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(bezier.evaluate(1.0), Vector3::new(3.0, 0.0, 0.0));
    }

    #[test]
    fn test_bezier_derivative() {
        let bezier = CubicBezier::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
        );

        let deriv = bezier.derivative(0.5);
        assert!(deriv.x > 0.0);
    }

    #[test]
    fn test_de_casteljau_split() {
        let bezier = CubicBezier::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(2.0, 1.0, 0.0),
            Vector3::new(3.0, 0.0, 0.0),
        );

        let (left, right) = bezier.split(0.5);

        // Left curve should end where right curve starts
        assert_eq!(left.evaluate(1.0), right.evaluate(0.0));
    }
}
```

### Integration Testing

```rust
// tests/scene_loading.rs
#[wasm_bindgen_test]
async fn test_scene_load() {
    let app = Application::new("canvas");

    let result = app.load("test-scene.splinecode").await;
    assert!(result.is_ok());

    let cube = app.find_object_by_name("Cube");
    assert!(cube.is_some());
}

#[wasm_bindgen_test]
async fn test_animation_playback() {
    let app = Application::new("canvas");
    app.load("animation-scene.splinecode").await.unwrap();

    let obj = app.find_object_by_name("AnimatedCube").unwrap();

    // Record initial position
    let initial_pos = obj.position();

    // Wait for animation frame
    sleep(100).await;

    // Position should have changed
    let new_pos = obj.position();
    assert_ne!(initial_pos, new_pos);
}
```

### E2E Testing

```javascript
// tests/e2e/scene-interaction.spec.js
import { test, expect } from '@playwright/test';

test('scene loads and interacts', async ({ page }) => {
  await page.goto('/scene');

  // Wait for scene to load
  await page.waitForSelector('canvas');
  await page.waitForTimeout(2000); // Wait for WASM

  // Take screenshot for visual regression
  await expect(page).toHaveScreenshot('initial.png');

  // Click on object
  await page.click('canvas', { position: { x: 100, y: 100 } });

  // Verify interaction
  await expect(page.locator('.object-info')).toBeVisible();

  // Take another screenshot
  await expect(page).toHaveScreenshot('clicked.png');
});

test('animation plays correctly', async ({ page }) => {
  await page.goto('/animated-scene');

  // Record animation frames
  const frames = [];
  for (let i = 0; i < 60; i++) {
    frames.push(await page.screenshot());
    await page.waitForTimeout(16); // ~60fps
  }

  // Verify animation (could use pixel comparison)
  expect(frames.length).toBe(60);
});
```

### Performance Testing

```javascript
// tests/performance/benchmark.js
import { Benchmark } from 'benchmark';

const suite = new Benchmark.Suite();

suite
  .add('Bezier evaluation', () => {
    for (let t = 0; t <= 1; t += 0.01) {
      bezier.evaluate(t);
    }
  })
  .add('Scene traversal', () => {
    scene.traverse(obj => {
      // Process object
    });
  })
  .add('Render frame', async () => {
    await renderer.render(scene);
  })
  .on('cycle', (event) => {
    console.log(String(event.target));
  })
  .run();
```

---

## Deployment

### CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI/CD

on:
  push:
    branches: [main, next]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Test
        run: cargo test --all

      - name: Build WASM
        run: wasm-pack build --release --target web

      - name: Run WASM tests
        run: wasm-pack test --headless --firefox

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build
        run: |
          cargo build --release
          wasm-pack build --release

      - name: Optimize WASM
        run: |
          curl -LO https://github.com/WebAssembly/binaryen/releases/download/version_117/binaryen-version_117-x86_64-linux.tar.gz
          tar -xzf binaryen-version_117-x86_64-linux.tar.gz
          ./binaryen-version_117/bin/wasm-opt -O4 pkg/spline3d_wasm.wasm -o pkg/spline3d_wasm.opt.wasm

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: wasm-bundle
          path: pkg/

  deploy:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          name: wasm-bundle
          path: dist/

      - name: Deploy to CDN
        run: |
          # Deploy to Cloudflare R2 / AWS S3
          aws s3 sync dist/ s3://cdn.spline.design/runtime/
          # Invalidate cache
          aws cloudfront create-invalidation --distribution-id XXX --paths "/*"
```

### Docker Configuration

```dockerfile
# Dockerfile for build environment
FROM rust:1.75 as builder

RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
COPY . .

RUN wasm-pack build --release --target web

# Production image
FROM nginx:alpine
COPY --from=builder /app/pkg /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

### CDN Configuration

```javascript
// Cloudflare Workers for edge optimization
export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // Serve WASM with correct headers
    if (url.pathname.endsWith('.wasm')) {
      const response = await env.ASSETS.fetch(request);
      return new Response(response.body, {
        headers: {
          'Content-Type': 'application/wasm',
          'Cache-Control': 'public, max-age=31536000',
          'Content-Encoding': 'br', // Brotli
        },
      });
    }

    // Serve textures with compression
    if (url.pathname.match(/\.(ktx2|basis)$/)) {
      const response = await env.ASSETS.fetch(request);
      return new Response(response.body, {
        headers: {
          'Content-Type': 'application/octet-stream',
          'Cache-Control': 'public, max-age=604800',
        },
      });
    }

    return env.ASSETS.fetch(request);
  }
};
```

---

## Monitoring and Analytics

### Performance Monitoring

```javascript
// Web Vitals for 3D content
import { onCLS, onFID, onFCP, onLCP, onTTFB } from 'web-vitals';

// Custom metrics
function reportWebVitals() {
  onCLS(console.log);
  onFID(console.log);
  onFCP(console.log);
  onLCP(console.log);
  onTTFB(console.log);

  // Custom 3D metrics
  observe3DMetrics();
}

function observe3DMetrics() {
  // Time to first render
  const observer = new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) {
      if (entry.name === 'first-scene-render') {
        sendMetric('3D First Render', entry.startTime);
      }
    }
  });
  observer.observe({ entryTypes: ['measure'] });

  // Frame rate monitoring
  let frameCount = 0;
  let lastTime = performance.now();

  function measureFrame() {
    frameCount++;
    const now = performance.now();
    if (now - lastTime >= 1000) {
      const fps = frameCount;
      sendMetric('3D FPS', fps);
      frameCount = 0;
      lastTime = now;
    }
    requestAnimationFrame(measureFrame);
  }
  measureFrame();
}

function sendMetric(name, value) {
  navigator.sendBeacon('/metrics', JSON.stringify({
    name,
    value,
    timestamp: Date.now(),
    url: window.location.href,
  }));
}
```

### Error Tracking

```javascript
// Error boundary for 3D content
class SplineErrorBoundary extends React.Component {
  state = { hasError: false, error: null };

  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }

  componentDidCatch(error, errorInfo) {
    // Send to error tracking
    sendError({
      error: error.toString(),
      stack: error.stack,
      componentStack: errorInfo.componentStack,
      scene: this.props.sceneUrl,
    });
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="error-fallback">
          <h2>Something went wrong</h2>
          <button onClick={() => window.location.reload()}>
            Reload
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}

// WASM error handling
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();

    // Custom panic handler
    std::panic::set_hook(Box::new(|info| {
        let message = format!("{}\n{}", info.message(), info.location().unwrap());
        web_sys::console::error_1(&JsValue::from_str(&message));

        // Send to error tracking
        send_panic_to_server(&message);
    }));
}
```

### Usage Analytics

```javascript
// Track scene interactions
function trackSceneEvents(scene) {
  scene.addEventListener('objectClick', (e) => {
    analytics.track('Scene Object Clicked', {
      objectName: e.target.name,
      objectId: e.target.id,
      scene: scene.url,
      timestamp: Date.now(),
    });
  });

  scene.addEventListener('animationStart', (e) => {
    analytics.track('Animation Started', {
      animationName: e.animation.name,
      scene: scene.url,
    });
  });
}

// Heatmap for object interactions
class InteractionHeatmap {
  constructor(scene) {
    this.interactions = new Map();
    this.setupListeners(scene);
  }

  setupListeners(scene) {
    scene.addEventListener('click', (e) => {
      const key = e.target.name;
      this.interactions.set(key, (this.interactions.get(key) || 0) + 1);
    });
  }

  getHotspots() {
    return Array.from(this.interactions.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10);
  }
}
```

---

## Scalability

### Horizontal Scaling

```
┌─────────────────────────────────────────────────────────────────┐
│                   Scalable Architecture                          │
│                                                                  │
│                    ┌─────────────┐                              │
│                    │   Load      │                              │
│                    │   Balancer  │                              │
│                    └──────┬──────┘                              │
│                           │                                      │
│         ┌─────────────────┼─────────────────┐                   │
│         │                 │                 │                   │
│    ┌────▼────┐      ┌────▼────┐      ┌────▼────┐              │
│    │ Server  │      │ Server  │      │ Server  │              │
│    │    1    │      │    2    │      │    N    │              │
│    └────┬────┘      └────┬────┘      └────┬────┘              │
│         │                 │                 │                   │
│         └─────────────────┼─────────────────┘                   │
│                           │                                      │
│                    ┌──────▼──────┐                              │
│                    │   Database  │                              │
│                    │   Cluster   │                              │
│                    └─────────────┘                              │
└─────────────────────────────────────────────────────────────────┘
```

### CDN Caching Strategy

```
Cache Headers by Content Type:

WASM Modules:
  Cache-Control: public, max-age=31536000, immutable
  (Version in filename enables long caching)

Textures:
  Cache-Control: public, max-age=604800
  (Shorter cache, may update)

Scene Files:
  Cache-Control: private, max-age=3600
  (User-specific, frequent updates)

API Responses:
  Cache-Control: private, no-cache
  (Always fresh)
```

### Database Schema

```sql
-- Scenes table
CREATE TABLE scenes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    published BOOLEAN DEFAULT FALSE,
    version INTEGER DEFAULT 1
);

-- Scene data (stored in object storage, referenced here)
CREATE TABLE scene_data (
    scene_id UUID REFERENCES scenes(id),
    version INTEGER,
    data_url TEXT NOT NULL,
    thumbnail_url TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (scene_id, version)
);

-- Objects index for search
CREATE TABLE scene_objects (
    id UUID PRIMARY KEY,
    scene_id UUID REFERENCES scenes(id),
    name VARCHAR(255),
    type VARCHAR(50),
    parent_id UUID,
    path TEXT[], -- Hierarchical path for quick lookup
    metadata JSONB
);

-- Analytics
CREATE TABLE scene_analytics (
    scene_id UUID REFERENCES scenes(id),
    date DATE,
    views INTEGER DEFAULT 0,
    interactions INTEGER DEFAULT 0,
    avg_load_time FLOAT,
    avg_fps FLOAT,
    PRIMARY KEY (scene_id, date)
);

-- Indexes
CREATE INDEX idx_scenes_user ON scenes(user_id);
CREATE INDEX idx_scenes_published ON scenes(published);
CREATE INDEX idx_objects_scene ON scene_objects(scene_id);
CREATE INDEX idx_objects_path ON scene_objects USING GIN(path);
```

---

## References

1. **WebAssembly Best Practices** - https://developer.mozilla.org/en-US/docs/WebAssembly
2. **wgpu Documentation** - https://wgpu.rs/
3. **Cloudflare Workers** - https://developers.cloudflare.com/workers/
4. **Web Vitals** - https://web.dev/vitals/
