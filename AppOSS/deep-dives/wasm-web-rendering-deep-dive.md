---
source: /home/darkvoid/Boxxed/@formulas/src.AppOSS
projects: Penpot (render-wasm), OpenPencil (CanvasKit), Rive (wasm runtime)
created_at: 2026-04-02
tags: wasm, webassembly, emscripten, rust, skia, rendering
---

# WASM Web Rendering Deep Dive

## Overview

WebAssembly (WASM) enables high-performance graphics in the browser by running compiled C++, Rust, and other languages at near-native speed. This document explores how AppOSS projects use WASM for rendering.

### Projects Covered

| Project | WASM Usage | Technology |
|---------|-----------|------------|
| Penpot | Canvas-based rendering | Rust + Skia + Emscripten |
| OpenPencil | Design editor | CanvasKit (Skia WASM) |
| Rive | Animation runtime | C++ + Emscripten |
| Skia | Graphics library | CanvasKit for Web |

---

## Part 1: WebAssembly Fundamentals

### 1.1 What is WebAssembly?

WebAssembly is a binary instruction format that runs in browsers and other environments:

```
High-level code → Compiler → WASM binary → Browser/Runtime
   (C++, Rust)    (clang,    (.wasm file)   (V8, SpiderMonkey,
                    rustc)                      Wasmtime)
```

**Key Properties:**
- Fast: Near-native performance
- Safe: Sandboxed execution
- Portable: Runs everywhere
- Interoperable: JavaScript FFI

### 1.2 WASM Memory Model

```
┌─────────────────────────────────────────────────────────┐
│                  Linear Memory                          │
│  (Contiguous byte array, accessed via indices)          │
│                                                         │
│  0          Stack       Heap                Memory Size │
│  │──────────│──────────│────────────────────│           │
│  │          │          │                    │           │
│  └──────────┴──────────┴────────────────────┘           │
│      ↑          ↑              ↑                        │
│   Static     Stack         Heap                        │
│   Data       Pointer      Pointer                      │
└─────────────────────────────────────────────────────────┘
```

```rust
// Rust WASM memory
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Buffer {
    data: Vec<u8>,
}

#[wasm_bindgen]
impl Buffer {
    pub fn new(size: usize) -> Buffer {
        Buffer { data: vec![0; size] }
    }
    
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }
    
    pub fn len(&self) -> usize {
        self.data.len()
    }
}
```

### 1.3 JavaScript Interop

```rust
// wasm-bindgen bindings
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    log(&format!("Hello, {}!", name));
}
```

```javascript
// JavaScript usage
import('./pkg/index.js').then(({ greet }) => {
    greet('World');
});
```

---

## Part 2: Building WASM for Graphics

### 2.1 Toolchains

#### Emscripten (C/C++ → WASM)

```bash
# Install
emsdk install latest
emsdk activate latest

# Compile
emcc source.cpp -o output.js \
  -s WASM=1 \
  -s EXPORTED_FUNCTIONS='["_render","_init"]' \
  -s EXPORTED_RUNTIME_METHODS='["ccall","cwrap"]' \
  -s TOTAL_MEMORY=67108864
```

#### wasm-pack (Rust → WASM)

```bash
# Install
cargo install wasm-pack

# Build
wasm-pack build --target web

# Output:
# - pkg/package.json
# - pkg/index.js (glue code)
# - pkg/index_bg.wasm (WASM binary)
# - pkg/index.d.ts (TypeScript types)
```

### 2.2 Penpot WASM Architecture

```
┌─────────────────────────────────────────────────────────┐
│              ClojureScript Frontend                     │
│           (UI, state, user interaction)                 │
└─────────────────────────────────────────────────────────┘
                         │
                         │ JS bindings
                         ▼
┌─────────────────────────────────────────────────────────┐
│         render-wasm (Rust + Skia + Emscripten)          │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │
│  │  main.rs    │  │  render.rs  │  │   shapes.rs   │  │
│  │  (entry)    │  │ (rasterize) │  │  (paths,text) │  │
│  └─────────────┘  └─────────────┘  └───────────────┘  │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │
│  │  Skia       │  │  Emscripten │  │   Memory      │  │
│  │  bindings   │  │  glue       │  │   management  │  │
│  └─────────────┘  └─────────────┘  └───────────────┘  │
└─────────────────────────────────────────────────────────┘
                         │
                         │ Canvas API
                         ▼
┌─────────────────────────────────────────────────────────┐
│                  HTML5 Canvas                           │
│               (2D rendering context)                    │
└─────────────────────────────────────────────────────────┘
```

### 2.3 Penpot WASM Structure

```rust
// src/main.rs - Entry point
use wasm_bindgen::prelude::*;
use crate::render::Renderer;

#[wasm_bindgen]
pub struct WasmRenderer {
    renderer: Renderer,
}

#[wasm_bindgen]
impl WasmRenderer {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> WasmRenderer {
        WasmRenderer {
            renderer: Renderer::new(width, height),
        }
    }
    
    pub fn render(&mut self, scene: &[u8]) {
        // Deserialize scene
        // Render to canvas
        // Flush to JS
    }
    
    pub fn get_image_data(&self) -> Vec<u8> {
        self.renderer.get_pixels()
    }
}

// Export initialization
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}
```

### 2.4 Rendering Flow

```rust
// src/render.rs
use skia_safe::{Canvas, Paint, Color, Surface, raster_n32_premul};

pub struct Renderer {
    surface: Surface,
    canvas: Canvas,
    paint: Paint,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        let surface = raster_n32_premul((width as i32, height as i32))
            .expect("Failed to create surface");
        let canvas = surface.canvas();
        
        Renderer {
            surface,
            canvas,
            paint: Paint::default(),
        }
    }
    
    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.paint.set_color(color);
        self.canvas.draw_rect(
            skia_safe::Rect::new(x, y, x + w, y + h),
            &self.paint
        );
    }
    
    pub fn flush(&mut self) -> Vec<u8> {
        self.surface.flush();
        let image_info = self.surface.image_info();
        let bytes_per_row = image_info.min_row_bytes();
        let total_bytes = bytes_per_row * image_info.height();
        
        // Get pixel data
        let pixels = self.surface.peek_pixels().unwrap();
        pixels.read_pixels().unwrap()
    }
}
```

### 2.5 Canvas Integration

```javascript
// JavaScript glue code (auto-generated by Emscripten/wasm-pack)
import init, { WasmRenderer } from './render_wasm.js';

async function start() {
    await init();
    
    const canvas = document.getElementById('canvas');
    const renderer = new WasmRenderer(800, 600);
    
    // Render scene
    const sceneData = encodeScene(myScene);
    renderer.render(sceneData);
    
    // Get image data and draw to canvas
    const imageData = renderer.getImageData();
    const ctx = canvas.getContext('2d');
    const imgData = new ImageData(
        new Uint8ClampedArray(imageData),
        800,
        600
    );
    ctx.putImageData(imgData, 0, 0);
}

start();
```

---

## Part 3: CanvasKit (Skia for Web)

### 3.1 What is CanvasKit?

CanvasKit is Skia compiled to WASM with a JavaScript API:

```javascript
// Load CanvasKit
const CanvasKit = await createCanvasKit({
    locateFile: (file) => `https://unpkg.com/canvaskit-wasm@latest/bin/${file}`
});

// Create surface
const surface = CanvasKit.MakeSurface(800, 600);
const canvas = surface.getCanvas();

// Draw
const paint = new CanvasKit.Paint();
paint.setColor(CanvasKit.Color4f(1, 0, 0, 1)); // Red
canvas.drawCircle(100, 100, 50, paint);

// Flush
surface.flush();
surface.draw(canvas);
```

### 3.2 OpenPencil Usage

```typescript
// OpenPencil rendering
import { CanvasKitInit } from 'canvaskit';

class Renderer {
    private skia: CanvasKit;
    private surface: Surface;
    private canvas: Canvas;
    
    async init() {
        this.skia = await CanvasKitInit();
        this.surface = this.skia.MakeSurface(800, 600);
        this.canvas = this.surface.getCanvas();
    }
    
    render(shape: Shape) {
        const paint = new this.skia.Paint();
        paint.setColor(this.skia.Color4f(...shape.fill));
        
        if (shape.type === 'rect') {
            const rect = this.skia.Rect(
                shape.x, shape.y,
                shape.x + shape.width,
                shape.y + shape.height
            );
            this.canvas.drawRect(rect, paint);
        }
    }
}
```

### 3.3 Skia Types Mapping

| JavaScript | C++ | Rust |
|-----------|-----|------|
| `Canvas` | `SkCanvas` | `Canvas` |
| `Paint` | `SkPaint` | `Paint` |
| `Path` | `SkPath` | `Path` |
| `Image` | `SkImage` | `Image` |
| `Font` | `SkFont` | `Font` |
| `Surface` | `SkSurface` | `Surface` |

---

## Part 4: Rive WASM Runtime

### 4.1 Architecture

```
┌─────────────────────────────────────────────────────────┐
│              JavaScript/TypeScript API                  │
│            (State machine, animations)                  │
└─────────────────────────────────────────────────────────┘
                         │
                         │ Emscripten bindings
                         ▼
┌─────────────────────────────────────────────────────────┐
│            Rive C++ Runtime (WASM)                      │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────┐  │
│  │   File      │  │ Animation   │  │   Renderer    │  │
│  │   Loader    │  │   System    │  │   (GPU/CPU)   │  │
│  └─────────────┘  └─────────────┘  └───────────────┘  │
└─────────────────────────────────────────────────────────┘
                         │
                         │ Canvas/WebGL
                         ▼
┌─────────────────────────────────────────────────────────┐
│                   Output                                │
│            (Canvas 2D / WebGL)                          │
└─────────────────────────────────────────────────────────┘
```

### 4.2 Usage Example

```typescript
import { CanvasRenderer, File } from '@rive-app/canvas';

// Load animation
const file = await File.load({
    src: 'animation.riv',
    onLoad: () => {
        const artboard = file.defaultArtboard();
        const animation = artboard.defaultAnimation();
        
        // Create renderer
        const renderer = new CanvasRenderer();
        renderer.setArtboard(artboard);
        
        // Animation loop
        function render() {
            animation.advance(deltaTime);
            animation.apply(1.0);
            artboard.advance(deltaTime, false);
            renderer.render();
            requestAnimationFrame(render);
        }
        render();
    }
});
```

### 4.3 State Machine Control

```typescript
// Interactive animation via state machines
const file = await File.load('interactive.riv');
const artboard = file.defaultArtboard();
const sm = artboard.stateMachine('State Machine 1');

// Set input values
sm.input('hover') = true;
sm.input('click') = false;

// Listen for events
sm.on('eventName', (event) => {
    console.log('Animation event:', event);
});
```

---

## Part 5: Performance Optimization

### 5.1 Memory Management

```rust
// Efficient WASM memory usage
use wasm_bindgen::prelude::*;

pub struct LargeBuffer {
    // Use Box for heap allocation
    data: Box<[u8]>,
}

impl LargeBuffer {
    pub fn new(size: usize) -> Self {
        LargeBuffer {
            data: vec![0; size].into_boxed_slice(),
        }
    }
    
    // Provide direct memory access to JS
    pub fn data_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }
    
    pub fn data_len(&self) -> usize {
        self.data.len()
    }
}
```

```javascript
// Zero-copy access from JS
const wasmModule = await init();
const buffer = wasmModule.LargeBuffer.new(1024 * 1024);

// Get direct memory access
const ptr = buffer.data_ptr();
const len = buffer.data_len();
const memory = wasmModule.memory.buffer;

// Create view without copying
const view = new Uint8Array(memory, ptr, len);
view[0] = 42; // Direct modification
```

### 5.2 Batching Draw Calls

```rust
// Batch similar operations
pub struct BatchedRenderer {
    commands: Vec<DrawCommand>,
}

enum DrawCommand {
    FillRect { x: f32, y: f32, w: f32, h: f32, color: u32 },
    StrokePath { path_id: u32, stroke_width: f32, color: u32 },
    DrawImage { image_id: u32, x: f32, y: f32 },
}

impl BatchedRenderer {
    pub fn push_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: u32) {
        self.commands.push(DrawCommand::FillRect { x, y, w, h, color });
    }
    
    pub fn flush(&mut self, canvas: &Canvas) {
        // Sort by color to minimize state changes
        self.commands.sort_by_key(|c| match c {
            DrawCommand::FillRect { color, .. } => *color,
            _ => 0,
        });
        
        let mut paint = Paint::default();
        for cmd in &self.commands {
            match cmd {
                DrawCommand::FillRect { x, y, w, h, color } => {
                    paint.set_color(*color);
                    canvas.draw_rect(Rect::new(*x, *y, *x + *w, *y + *h), &paint);
                }
                // ... handle other commands
            }
        }
        
        self.commands.clear();
    }
}
```

### 5.3 Web Workers for Off-Main-Thread Rendering

```javascript
// main.js
const worker = new Worker('render-worker.js');

// Send scene to worker
worker.postMessage({
    type: 'render',
    scene: sceneData,
    canvas: offscreenCanvas
}, [offscreenCanvas]);

// render-worker.js
self.onmessage = async (e) => {
    const { type, scene, canvas } = e.data;
    
    if (type === 'render') {
        const renderer = await initRenderer(canvas);
        renderer.render(scene);
    }
};
```

### 5.4 Tiled Rendering for Large Canvases

```rust
// Split large render into tiles
const TILE_SIZE = 256;

pub fn render_tiled(
    scene: &Scene,
    width: u32,
    height: u32,
) -> Vec<TileResult> {
    let mut tiles = Vec::new();
    
    for ty in (0..height).step_by(TILE_SIZE) {
        for tx in (0..width).step_by(TILE_SIZE) {
            let tw = TILE_SIZE.min(width - tx);
            let th = TILE_SIZE.min(height - ty);
            
            let tile = render_tile(scene, tx, ty, tw, th);
            tiles.push(TileResult { x: tx, y: ty, data: tile });
        }
    }
    
    tiles
}
```

---

## Part 6: Debugging WASM

### 6.1 Console Logging

```rust
// Use console_error_panic_hook for better error messages
use console_error_panic_hook;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    // ... rest of init
}
```

```rust
// Logging from Rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

pub fn debug_log(msg: &str) {
    log(msg);
}
```

### 6.2 Chrome DevTools

1. Open DevTools → Sources
2. Find WASM module under "Sources" → "wasm"
3. Set breakpoints in Rust code (with source maps)
4. Use Console for logging

### 6.3 Profiling

```javascript
// Performance measurement
const start = performance.now();
renderer.render(scene);
const end = performance.now();
console.log(`Render took ${end - start}ms`);
```

```rust
// Rust-side profiling
use web_time::Instant;

fn profile_render() {
    let start = Instant::now();
    
    // ... rendering code ...
    
    let elapsed = start.elapsed();
    log(&format!("Render took {:?}", elapsed));
}
```

---

## Part 7: Rust WASM Best Practices

### 7.1 Crate Structure

```toml
# Cargo.toml
[package]
name = "render-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
console_error_panic_hook = "0.1"
skia-safe = { version = "0.70", features = ["gl", "webgl"] }
web-sys = { version = "0.3", features = ["CanvasRenderingContext2d", "ImageData"] }

[profile.release]
opt-level = "s"  # Optimize for size
lto = true
```

### 7.2 Size Optimization

```toml
# .cargo/config.toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "opt-level=s"]

# Build with wasm-opt
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz", "--enable-mutable-globals"]
```

```bash
# Check size
wasm-pack build --release
ls -la pkg/*.wasm

# Typically 500KB - 2MB for graphics apps
```

### 7.3 Async Operations

```rust
use wasm_bindgen_futures::spawn_local;
use js_sys::Promise;

#[wasm_bindgen]
pub fn load_image(src: String) -> Promise {
    let future = async move {
        // Load image asynchronously
        let image_data = fetch_image(&src).await?;
        Ok(JsValue::from(image_data))
    };
    
    wasm_bindgen_futures::future_to_promise(future)
}
```

---

## Summary

Key takeaways for WASM web rendering:

1. **Toolchains**: Emscripten (C++), wasm-pack (Rust)
2. **Graphics Libraries**: Skia/CanvasKit, custom renderers
3. **Interop**: wasm-bindgen for Rust, Emscripten glue for C++
4. **Performance**: Batch draw calls, use workers, tile large renders
5. **Debugging**: Console logging, DevTools, source maps
6. **Size**: Optimize with -Oz, wasm-opt, tree shaking

For building similar systems:
1. Start with existing libraries (CanvasKit, skia-safe)
2. Use wasm-bindgen for Rust JS interop
3. Profile early and often
4. Consider off-main-thread rendering for complex scenes
