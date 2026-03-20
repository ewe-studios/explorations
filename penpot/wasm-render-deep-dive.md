---
location: /home/darkvoid/Boxxed/@formulas/src.AppOSS/src.penpot/penpot/frontend/app/src/wasm-render/
repository: git@github.com:penpot/penpot
explored_at: 2026-03-17
language: Rust, C (Skia)
parent: exploration.md
---

# Penpot Wasm Render Engine - Deep Dive

## Overview

The Wasm render engine is a Rust-based 2D graphics rendering system that powers Penpot's canvas. It uses Skia (the same graphics library behind Chrome and Android) compiled to WebAssembly via Emscripten, providing hardware-accelerated rendering through WebGL.

## Build System

### Compilation Target
- **Target:** `wasm32-unknown-emscripten`
- **Build Script:** `render-wasm/build`
- **Watch Mode:** `render-wasm/watch`

### Dependencies (Cargo.toml)
```toml
skia-safe = { version = "0.87.0", features = ["gl", "svg", "textlayout", "binary-cache", "webp"] }
glam = "0.24.2"           # Math library (vectors, matrices)
bezier-rs = "0.4.0"       # Bezier curve operations
gl = "0.14.0"             # OpenGL bindings
uuid = { version = "1.11.0", features = ["v4", "js"] }
```

### Build Output
The Emscripten build produces:
1. `render_wasm.wasm` - The WebAssembly binary
2. `render_wasm.js` - JavaScript glue code for loading/initializing the Wasm module
3. Output copied to `frontend/resources/public/js/render-wasm/`

## Architecture

### Module Structure

```
render-wasm/src/
├── main.rs           # Wasm exports, FFI boundary, global state
├── emscripten.rs     # Emscripten-specific FFI bindings
├── state.rs          # Global application state (ShapesPool, modifiers, structure)
├── render.rs         # Core rendering logic, tile orchestration
├── tiles.rs          # Tile grid, hash maps, pending tile queues
├── view.rs           # Viewbox/viewport management
├── shapes.rs         # Shape definitions and rendering
├── math.rs           # Math primitives (Bounds, Matrix)
├── mem.rs            # Memory management for JS ↔ Wasm serialization
└── render/           # Rendering submodules
    ├── surfaces.rs   # Skia surface management (layers)
    ├── fills.rs      # Fill rendering (solid, gradient, image)
    ├── strokes.rs    # Stroke rendering
    ├── shadows.rs    # Drop/inner shadows
    ├── text.rs       # Text layout and rendering
    ├── images.rs     # Image decoding and caching
    ├── fonts.rs      # Font collection and shaping
    └── gpu_state.rs  # WebGL context management
```

### Global State Design

The Wasm module maintains a single global state (`STATE`) accessible across FFI calls:

```rust
pub(crate) static mut STATE: Option<Box<State>> = None;

pub struct State {
    pub render_state: RenderState,      // Rendering context
    pub current_id: Option<Uuid>,       // Currently selected shape
    pub shapes: ShapesPool,             // Pre-allocated shape storage
    pub modifiers: HashMap<Uuid, Matrix>,  // Transform overrides
    pub scale_content: HashMap<Uuid, f32>, // Scale content flags
    pub structure: HashMap<Uuid, Vec<StructureEntry>>, // Parent-child relationships
}
```

**Design Rationale:**
- Uses `static mut` with `Box` to own the state in Wasm linear memory
- State persists between JS calls without serialization overhead
- Not thread-safe by design (single-threaded Wasm execution)

## ShapesPool - Pre-allocated Shape Storage

```rust
pub struct ShapesPool {
    inner: HashMap<Uuid, Shape>,
    capacity: usize,
}
```

**Why Pre-allocate?**
- Avoids GC pressure during animation
- Predictable memory usage
- JS doesn't need to manage Wasm memory layout

**Initialization:**
```javascript
// From JS: Initialize pool with 10,000 shape capacity
wasmModule.init_shapes_pool(10000);
```

## Serialization Protocol

### Shape Data Layout

Shapes are built incrementally via FFI calls:

```javascript
// 1. Start new shape
wasmModule.use_shape(uuid_a, uuid_b, uuid_c, uuid_d);

// 2. Set parent relationship
wasmModule.set_parent(parent_a, parent_b, parent_c, parent_d);

// 3. Set transform (6 values for 2D affine matrix)
wasmModule.set_shape_transform(a, b, c, d, e, f);

// 4. Set selection rect
wasmModule.set_shape_selrect(left, top, right, bottom);

// 5. Build children list
wasmModule.add_shape_child(child_a, child_b, child_c, child_d);
wasmModule.set_children(); // Commits from shared memory buffer
```

### Memory Buffer Protocol

Complex data is serialized via a shared byte buffer:

```rust
// Rust side - Reading from shared buffer
let bytes = mem::bytes();
let entries: Vec<Uuid> = bytes
    .chunks(size_of::<Uuid::BytesType>())
    .map(|data| Uuid::from_bytes(data.try_into().unwrap()))
    .collect();
mem::free_bytes(); // Explicit free after use
```

```javascript
// JS side - Writing to shared buffer
const buffer = wasmModule.get_memory_buffer();
const view = new Uint8Array(buffer, offset, size);
view.set(data);
wasmModule.function_call();
```

### Type Serialization

| Type | Format | Size |
|------|--------|------|
| Shape Type | u8 enum | 1 byte |
| Transform | 6× f32 | 24 bytes |
| UUID | 4× u32 quartet | 16 bytes |
| Path Segment | command(2) + flags(2) + c1(8) + c2(8) + p(8) | 28 bytes |
| Fill | type(1) + reserved(3) + data(156) | 160 bytes |
| Color | u32 ARGB | 4 bytes |

## Tile Rendering System

### Tile Structure

```rust
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Tile(pub i32, pub i32);  // (x, y) grid coordinates

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct TileRect(pub i32, pub i32, pub i32, pub i32);  // (sx, sy, ex, ey)
```

### Constants
```rust
pub const TILE_SIZE: f32 = 512.;  // 512×512 logical pixels
const VIEWPORT_DEFAULT_CAPACITY: usize = 24 * 12;  // ~288 tiles
const MAX_BLOCKING_TIME_MS: i32 = 32;  // Max frame time
const NODE_BATCH_THRESHOLD: i32 = 10;  // Shapes per batch
```

### Tile Viewbox

```rust
pub struct TileViewbox {
    pub visible_rect: TileRect,     // Currently visible tiles
    pub interest_rect: TileRect,    // Visible + margin for preloading
    pub interest: i32,              // Margin size (default: 1)
    pub center: Tile,               // Center tile for sorting
}
```

**Viewbox Update Flow:**
```rust
// 1. User zooms/pans → Viewbox changes
viewbox.update(zoom, pan_x, pan_y);

// 2. Calculate visible tiles
let tile_rect = get_tiles_for_viewbox(viewbox, scale);

// 3. Sort by Manhattan distance from center
let mut pending = PendingTiles::new_empty();
pending.update(&tile_viewbox);  // Spiral from center outward

// 4. Render in priority order
while let Some(tile) = pending.pop() {
    render_tile(tile);
}
```

### TileHashMap - Spatial Index

```rust
pub struct TileHashMap {
    grid: HashMap<Tile, IndexSet<Uuid>>,   // Tile → Shapes
    index: HashMap<Uuid, HashSet<Tile>>,   // Shape → Tiles
}
```

**Dual-index design enables:**
- Fast tile queries: "What shapes are in this tile?"
- Fast shape queries: "What tiles does this shape occupy?"

### Tile Cache Strategy

```rust
pub struct TileTextureCache {
    capacity: usize,
    textures: HashMap<Tile, TileTexture>,
}

pub struct TileTexture {
    image: skia::Image,
    last_used: u32,  // Frame counter for LRU eviction
}
```

**Cache Flow:**
1. Tile needed → Check cache
2. Cache hit → Blit to target surface
3. Cache miss → Render shape layers → Capture as Image → Store in cache
4. Cache full → Evict LRU tiles outside interest rect

### Rendering Pipeline

```rust
pub fn process_animation_frame(&mut self, timestamp: i32) -> Result<(), String> {
    self.render_state.process_animation_frame(
        &self.shapes,
        &self.modifiers,
        &self.structure,
        &self.scale_content,
        timestamp,
    )?;
    Ok(())
}
```

**Per-Frame Steps:**
1. Update tile viewbox from current zoom/pan
2. Identify new/changed tiles
3. Sort tiles by distance from viewport center
4. Render batches of 10 shapes per tile
5. Check time budget (32ms max)
6. Yield to browser if budget exceeded
7. Resume next frame

## Layer Surfaces

The renderer uses multiple Skia surfaces per tile:

```rust
pub enum SurfaceId {
    Fills,      // Shape fills
    Strokes,    // Shape strokes
    Shadows,    // Drop/inner shadows
    Current,    // Composite layer
    Target,     // Final display surface
}

pub struct Surfaces {
    surfaces: HashMap<SurfaceId, skia::Surface>,
}
```

**Render Order:**
1. Clear `Fills` surface
2. Draw all fills
3. Clear `Strokes` surface
4. Draw all strokes
5. Clear `Shadows` surface
6. Draw all shadows
7. Composite all surfaces → `Current`
8. Blit `Current` → `Target`

## Focus Mode

Focus mode isolates specific shapes for rendering:

```rust
pub struct FocusMode {
    shapes: Vec<Uuid>,
    active: bool,
}
```

**Usage:**
```rust
// Enter focus mode (e.g., editing a component)
wasmModule.set_focus_mode();  // Reads shape UUIDs from shared buffer

// Exit focus mode
wasmModule.clear_focus_mode();
```

## Performance Optimizations

### 1. Shallow Tile Rebuild
```rust
pub fn rebuild_tiles_shallow(&mut self) {
    // Only processes first-level children
    // Deep children resolved during render
}
```
Reduces upfront computation for complex hierarchies.

### 2. Modifier Propagation
Transform modifiers propagate through the tree efficiently:
```rust
pub fn propagate_modifiers(
    state: &State,
    entries: &[TransformEntry],
    pixel_precision: bool
) -> Vec<u8> {
    // Returns serialized transformed bounds
}
```

### 3. Animation Frame Throttling
```rust
// Check time budget every NODE_BATCH_THRESHOLD shapes
if shapes_processed % NODE_BATCH_THRESHOLD == 0 {
    let elapsed = get_time() - frame_start;
    if elapsed > MAX_BLOCKING_TIME_MS {
        return ContinueRender::Yield;  // Resume next frame
    }
}
```

### 4. Pixel-Aligned Rendering
```rust
pub fn process_animation_frame(
    &mut self,
    // ...
    pixel_precision: bool,  // Snap to pixel grid
) {
    // Reduces antialiasing work for crisp UI rendering
}
```

## WebGL Integration

### Surface Creation
```rust
let mut context = skia::gpu::gl::Context::new(
    gl_interface,
    skia::gpu::gl::ContextOptions::new(),
)?;

let mut surface = skia::surface::new_wrapped_gl(
    &mut context,
    budgeted,
    (width, height),
    color_type,
    None,
)?;
```

### Canvas Resize
```rust
#[no_mangle]
pub extern "C" fn resize_viewbox(width: i32, height: i32) {
    with_state_mut!(state, {
        state.resize(width, height);
    });
}
```

## Debug Features

### Debug Flags
```rust
pub fn set_render_options(debug: u32, dpr: f32) {
    // debug flags:
    // 0x01 - Show tile boundaries
    // 0x02 - Show shape bounds
    // 0x04 - Show focus mode highlight
    // 0x08 - Profile tile rebuild
}
```

### Profile Mode
```rust
if render_state.options.is_profile_rebuild_tiles() {
    state.rebuild_tiles();  // Full rebuild with timing
} else {
    state.rebuild_tiles_shallow();  // Optimized
}
```

## Error Handling

### Panic Boundaries
```javascript
// JS wrapper
try {
    wasmModule.process_animation_frame(timestamp);
} catch (e) {
    console.error('Wasm render error:', e);
    // Fallback to 2D canvas renderer
}
```

### Rust Panic Handler
```rust
let result = std::panic::catch_unwind(|| {
    with_state_mut!(state, {
        state.process_animation_frame(timestamp)
            .expect("Error rendering");
    });
});

match result {
    Ok(_) => {}
    Err(err) => {
        println!("Render error: {:?}", err);
        std::panic::resume_unwind(err);
    }
}
```

## Integration Points

### Frontend Bridge (ClojureScript)
```clojure
(ns app.render-wasm
  (:require
   [app.render-wasm.api :as api]
   [app.render-wasm.shape :as wasm.shape]))

(defn initialize [enabled?]
  (if enabled?
    (set! app.common.types.path/wasm:calc-bool-content api/calculate-bool)
    (set! app.common.types.path/wasm:calc-bool-content nil))
  (set! app.common.types.shape/wasm-enabled? enabled?)
  (set! app.common.types.shape/wasm-create-shape wasm.shape/create-shape))
```

### Feature Flags
- `enable-feature-render-wasm` - Enable Wasm renderer
- `enable-render-wasm-dpr` - Use device pixel ratio

## Future Improvements

1. **WebGPU Backend** - Migrate from WebGL to WebGPU for better performance
2. **Multi-threaded Rendering** - Use Wasm workers for parallel tile rendering
3. **Incremental GC** - Better memory management for long sessions
4. **SIMD Optimization** - Leverage Wasm SIMD for math operations
