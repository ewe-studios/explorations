# Rockies Exploration

## Overview

**Rockies** is a 2D pixel-based sandbox physics game written in Rust and WebAssembly, designed to run in web browsers. It simulates a world with gravity, collisions, and user interaction.

- **License**: GPL-2.0-only
- **Author**: Noam Lewis
- **WASM Support**: Full production-ready WASM build
- **Repository**: https://github.com/sinelaw/rockies

---

## Features

### Core Gameplay

- **Basic Physics**: Collision detection, gravity, inertia, mass
- **User Interaction**:
  - Click and drag objects
  - Keyboard controls for player character
  - Click to add new cells
- **Procedural Generation**: Perlin noise terrain
- **Multi-grid Universe**: Efficient cell management with grid loading/unloading

### Visual Features

- **Pixel-based Rendering**: Direct pixel buffer manipulation
- **HSV Color System**: Full color wheel support
- **Background Generation**: Procedural sky and underground
- **Cloud Rendering**: Noise-based cloud layers

---

## Architecture

### Directory Structure

```
rockies/
├── Cargo.toml           # Main crate configuration
├── build.rs             # Build script (asset generation)
├── src/
│   ├── lib.rs           # WASM bindings (Game struct)
│   ├── main.rs          # Terminal binary (SDL2 backend)
│   ├── universe.rs      # Game universe management
│   ├── grid.rs          # Single grid data structure
│   ├── multigrid.rs     # Multi-grid coordinate system
│   ├── color.rs         # HSV color handling
│   ├── v2.rs            # 2D vector types (V2, V2i)
│   ├── inertia.rs       # Physics inertia/mass/velocity
│   ├── generator.rs     # Procedural generation
│   ├── assets.rs        # Asset loading/generation
│   ├── console.rs       # Console/debug output
│   ├── log.rs           # Logging utilities
│   └── utils.rs         # Utility functions
├── www/                 # Web frontend
│   ├── index.html
│   ├── package.json
│   └── src/
│       └── index.js     # JS bootstrap code
└── schemas/             # Data schemas
    ├── languages.json
    ├── plugins.json
    ├── themes.json
    └── blocklist.json
```

---

## Core Types

### Vector Types (v2.rs)

```rust
/// Floating-point 2D vector
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

/// Integer 2D vector (WASM-exposed)
#[wasm_bindgen]
#[derive(Clone, Hash, Copy, Debug, PartialEq, Eq)]
pub struct V2i {
    pub x: i32,
    pub y: i32,
}

impl V2i {
    pub fn new(x: i32, y: i32) -> V2i;
    pub fn plus(&self, offset: V2i) -> V2i;
    pub fn minus(&self, offset: V2i) -> V2i;
    pub fn to_v2(&self) -> V2;
}
```

### Color System (color.rs)

```rust
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub h: f64,  // Hue: 0-360
    pub s: f64,  // Saturation: 0-1
    pub v: f64,  // Value: 0-1
}

impl Color {
    pub fn hsv(h: f64, s: f64, v: f64) -> Color;
    pub fn to_u32(&self) -> u32;  // ARGB format
    pub fn lerp(&self, other: &Color, t: f64) -> Color;
}
```

### Physics (inertia.rs)

```rust
#[derive(Clone, Copy, Debug)]
pub struct Inertia {
    pub velocity: V2,      // Current velocity
    pub force: V2,         // Accumulated force
    pub pos: V2,           // Position
    pub mass: u32,         // Mass units
    pub elasticity: f64,   // Bounciness (0-1)
    pub collision_stats: u32,  // Collision counter
}

impl Inertia {
    pub fn tick(&mut self, dt: f64);
    pub fn apply_force(&mut self, force: V2);
    pub fn add_velocity(&mut self, vel: V2);
}
```

---

## Game Struct (WASM API)

### Main Game Loop

```rust
#[wasm_bindgen]
pub struct Game {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
    universe: Universe,
    keys: HashSet<String>,
    shoot_color: Color,
    hasher: PermutationTable,  // For noise
}

#[wasm_bindgen]
impl Game {
    /// Create new game instance
    pub fn new(width: usize, height: usize) -> Self;

    /// Get pixel buffer pointer (for canvas rendering)
    pub fn pixels(&self) -> *const u32;

    /// Game tick - update and render
    pub fn tick(&mut self);

    /// Input handlers
    pub fn key_down(&mut self, key: String);
    pub fn key_up(&mut self, key: String);
    pub fn unfocus(&mut self);
    pub fn click(&mut self, x: i32, y: i32);

    /// Grid management (persistence)
    pub fn get_missing_grids(&self) -> Vec<GridIndex>;
    pub fn get_loaded_grids(&self) -> Vec<GridIndex>;
    pub fn get_droppable_grids(&self) -> Vec<GridIndex>;
    pub fn load_grid(&mut self, grid_index: &GridIndex, bytes: JsValue);
    pub fn generate_grid(&mut self, grid_index: &GridIndex);
    pub fn save_grid(&mut self, grid_index: &GridIndex) -> JsValue;
    pub fn drop_grid(&mut self, grid_index: &GridIndex);

    /// Debug/info
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
    pub fn stats(&mut self) -> Stats;
}
```

### JavaScript Usage

```javascript
// Initialize game
const game = Game.new(800, 600);
const pixels = game.pixels();
const pixelArray = new Uint32Array(
    wasm.memory.buffer,
    pixels,
    game.width() * game.height()
);

// Game loop
function render() {
    game.tick();

    // Copy pixel buffer to canvas
    ctx.putImageData(imageData, 0, 0);

    requestAnimationFrame(render);
}
render();

// Input handling
window.addEventListener('keydown', (e) => {
    game.key_down(e.key);
});
window.addEventListener('keyup', (e) => {
    game.key_up(e.key);
});
canvas.addEventListener('click', (e) => {
    game.click(e.clientX, e.clientY);
});
```

---

## Universe System

### Multi-Grid Architecture

The universe is divided into 128x128 grids for efficient management:

```rust
pub const GRID_SIZE: usize = 128;

pub struct Universe {
    pub cells: MultiGrid<Cell>,
    pub player: Player,
    // ...
}

pub struct MultiGrid<T> {
    grids: HashMap<GridIndex, Grid<T>>,
    // ...
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridIndex {
    pub x: i32,
    pub y: i32,
}
```

### Cell Management

```rust
pub struct Cell {
    pub index: CellIndex,
    pub color: Color,
    pub inertia: Inertia,
}

impl Cell {
    pub fn update(&mut self);
    pub fn collide(&mut self, other: &mut Cell);
}
```

### Grid Persistence

```rust
impl Universe {
    /// Get grids that need to be loaded from storage
    pub fn get_missing_grids(&self) -> Vec<GridIndex>;

    /// Load a grid from storage (IndexedDB, etc.)
    pub fn load_from_storage(&mut self, index: GridIndex, bytes: JsValue);

    /// Save a grid to storage
    pub fn save_grid(&mut self, index: GridIndex) -> Option<JsValue>;

    /// Drop a grid from memory (free memory)
    pub fn drop_grid(&mut self, index: GridIndex);
}
```

---

## Procedural Generation

### Terrain Generation

```rust
// generator.rs
use noise::{perlin_2d, PermutationTable};

pub fn generate_terrain(
    pos: V2i,
    hasher: &PermutationTable
) -> TerrainType {
    let posv = pos.to_v2().cmul(0.01);

    // Multi-octave noise
    let noise1 = perlin_2d(posv, hasher);
    let noise2 = perlin_2d(posv.cmul(2.0), hasher) * 0.5;
    let noise3 = perlin_2d(posv.cmul(4.0), hasher) * 0.25;

    let combined = (noise1 + noise2 + noise3) / 1.75;

    match combined {
        n if n > 0.7 => TerrainType::Stone,
        n if n > 0.4 => TerrainType::Dirt,
        n if n > 0.2 => TerrainType::Grass,
        _ => TerrainType::Air,
    }
}
```

### Background Rendering

```rust
fn render_background(&self, pos: V2i) -> u32 {
    let depth = pos.y - (self.height as i32);

    if depth >= self.height as i32 {
        // Underground - darker with depth
        let value = (255.0 / ((depth + 2) as f64).powf(0.5)) as u32;
        value + (value << 8) + (value << 16)  // Gray
    } else {
        // Sky with clouds
        let altitude = -depth as f64 + self.height as f64;
        let posv = pos.to_v2().plus(V2::new(0.5, 0.7)).cmul(0.01);

        let noise2 = perlin_2d(Vector2::new(posv.y * 10.0, posv.x * 10.0), self.hasher);
        let noise = perlin_2d(Vector2::new(posv.x, posv.y), self.hasher);

        if (0.2 + 0.9 / (altitude / 10.0)) < noise2 * noise {
            0xFFFFFF  // Cloud (white)
        } else {
            0xCCCCFF  // Sky (blue)
        }
    }
}
```

---

## Build System

### Cargo Configuration

```toml
[package]
name = "rockies"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]  # cdylib for WASM, rlib for testing

[[bin]]
name = "rockies"
path = "src/main.rs"
required-features = ["terminal"]

[features]
default = ["terminal"]
wasm = ["console_error_panic_hook"]
terminal = ["ansi-control-codes", "libc", "sdl2"]
wasm_js = ["console_error_panic_hook"]

[dependencies]
wasm-bindgen = "0.2.100"
serde-wasm-bindgen = "0.5"
console_error_panic_hook = { version = "0.1.7", optional = true }
noise = "0.9"
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
chrono = "0.4"

# Terminal rendering (optional)
sdl2 = { version = "0.37.0", optional = true }
ansi_term = "0.12.1"

[dev-dependencies]
wasm-bindgen-test = "0.3.34"

[profile.release]
debug = true  # Keep debug info for profiling

[package.metadata.wasm-pack.profile.profiling]
wasm-opt = ['-Os']
```

### Build Commands

```bash
# WASM build (recommended)
wasm-pack build --target web

# WASM with profiling
wasm-pack build --profile profiling --target web

# Native terminal build
cargo build --release --features terminal

# Test
cargo test

# WASM tests
wasm-pack test --headless --firefox
```

---

## Web Integration

### HTML Bootstrap

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Rockies</title>
</head>
<body>
    <canvas id="game" width="800" height="600"></canvas>
    <script type="module">
        import init, { Game } from './pkg/rockies.js';

        async function run() {
            await init();
            const game = Game.new(800, 600);

            const canvas = document.getElementById('game');
            const ctx = canvas.getContext('2d');
            const imageData = ctx.createImageData(800, 600);

            function render() {
                game.tick();

                const pixels = game.pixels();
                const memory = new Uint32Array(
                    wasm.memory.buffer,
                    pixels,
                    800 * 600
                );

                imageData.data.set(
                    new Uint8ClampedArray(
                        memory.buffer,
                        0,
                        800 * 600 * 4
                    )
                );
                ctx.putImageData(imageData, 0, 0);

                requestAnimationFrame(render);
            }
            render();
        }
        run();
    </script>
</body>
</html>
```

---

## Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| WASM Size | ~500KB | Optimized release |
| Frame Rate | 60 FPS | Typical desktop |
| Cell Count | 10,000+ | Depends on grid loading |
| Memory | ~50MB | With grid caching |
| Load Time | <1s | Initial WASM download |

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v2_addition() {
        let a = V2::new(1.0, 2.0);
        let b = V2::new(3.0, 4.0);
        assert_eq!(a.plus(b), V2::new(4.0, 6.0));
    }

    #[test]
    fn test_color_conversion() {
        let red = Color::hsv(0.0, 1.0, 1.0);
        assert_eq!(red.to_u32(), 0xFFFF0000);
    }
}
```

### WASM Tests

```rust
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
fn test_game_creation() {
    let game = Game::new(100, 100);
    assert_eq!(game.width(), 100);
    assert_eq!(game.height(), 100);
}
```

---

## Future Enhancements

### Potential Features

1. **Multiplayer**: WebRTC-based peer-to-peer
2. **Save System**: IndexedDB persistence
3. **More Cell Types**: Different physics properties
4. **Crafting System**: Combine cells
5. **Fluid Simulation**: Water/lava physics
6. **Electricity**: Logic gates and circuits

### WASM Optimizations

1. **SIMD**: Use WASM SIMD for pixel operations
2. **Multi-threading**: Web Workers for physics
3. **SharedArrayBuffer**: Shared memory for rendering
4. **WebGPU**: Hardware-accelerated rendering

---

## Related Documents

- [WASM Analysis](wasm-web-editor-analysis.md) - Web editor feasibility
- [Fresh Editor](fresh-exploration.md) - Terminal editor with WASM support
- [Rust Revision](rust-revision.md) - Rust reproduction guide
