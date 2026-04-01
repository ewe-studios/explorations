---
title: "Rockies Physics Engine Exploration"
subtitle: "2D granular physics simulation with WebAssembly and spatial partitioning"
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/rockies
language: Rust + WebAssembly
---

# Rockies Physics Engine Exploration

## Overview

Rockies is a 2D granular physics sandbox simulation that demonstrates:

- **Cell-based physics** - Individual particle simulation with mass, velocity, elasticity
- **Impulse collision response** - Real-time collision detection and resolution
- **MultiGrid spatial partitioning** - O(n) collision detection via spatial hashing
- **Procedural terrain generation** - Perlin noise-based world generation
- **WebAssembly deployment** - Browser-native performance via wasm-bindgen
- **Chunked world management** - Dynamic loading/unloading of world regions

```
┌─────────────────────────────────────────────────────────────────┐
│                     Rockies Physics Pipeline                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────────┐   │
│  │   Input     │     │   Physics   │     │    Rendering    │   │
│  │  (Keys/     │────▶│   Ticks     │────▶│   (Pixel        │   │
│  │   Mouse)    │     │  (100Hz)    │     │    Buffer)      │   │
│  └─────────────┘     └──────┬──────┘     └─────────────────┘   │
│                             │                                   │
│              ┌──────────────┼──────────────┐                   │
│              ▼              ▼              ▼                   │
│     ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│     │   Forces    │ │ Collisions  │ │   Position  │           │
│     │  (Gravity)  │ │  (Impulse)  │ │   Updates   │           │
│     └─────────────┘ └─────────────┘ └─────────────┘           │
│                             │                                   │
│              ┌──────────────┴──────────────┐                   │
│              ▼                              ▼                   │
│     ┌─────────────────┐          ┌─────────────────┐          │
│     │   MultiGrid     │          │   Generator     │          │
│     │  (Spatial Hash) │          │ (Perlin Noise)  │          │
│     └─────────────────┘          └─────────────────┘          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Statistics

| Metric | Value |
|--------|-------|
| Total Lines of Code | ~1,800 |
| Core Physics | ~500 lines (universe.rs, inertia.rs) |
| Spatial System | ~600 lines (multigrid.rs, grid.rs) |
| WASM Integration | ~200 lines (lib.rs) |
| Procedural Gen | ~100 lines (generator.rs) |
| Math Library | ~180 lines (v2.rs, color.rs) |
| Simulation Frequency | 100Hz (dt = 0.01) |
| Grid Chunk Size | 128x128 cells |

---

## Table of Contents

This exploration consists of multiple deep-dive documents:

### Core Documents

1. **[exploration.md](./exploration.md)** (this file) - Architecture overview
2. **[00-zero-to-physics-engineer.md](./00-zero-to-physics-engineer.md)** - Physics fundamentals textbook
3. **[01-grid-system-deep-dive.md](./01-grid-system-deep-dive.md)** - MultiGrid spatial hashing
4. **[02-physics-collision-deep-dive.md](./02-physics-collision-deep-dive.md)** - Impulse-based collision response
5. **[03-wasm-integration-deep-dive.md](./03-wasm-integration-deep-dive.md)** - WebAssembly deployment
6. **[04-procedural-generation-deep-dive.md](./04-procedural-generation-deep-dive.md)** - Perlin noise terrain

### Implementation Documents

7. **[rust-revision.md](./rust-revision.md)** - Architecture improvements (already Rust)
8. **[production-grade.md](./production-grade.md)** - Performance optimization guide

---

## Chapter 1: The Physics Pipeline

### Fixed Timestep Loop

```rust
pub fn tick(&mut self) {
    // Multiple substeps for stability
    for _ in 0..((1.0 / self.dt) as usize) {
        self.calc_forces();           // Apply gravity
        self.update_velocity();       // Semi-implicit Euler
        self.cells.calc_collisions(self.dt);  // Resolve contacts
        self.player.update_pos(&self.cells, self.dt);
        self.cells.update_pos(self.dt);
        self.zero_forces();           // Clear for next frame
    }
    
    // Player health decay over time
    self.player.life = self.player.life.saturating_sub(10000);
}
```

**Why substepping?** Physics stability requires small time steps. Running at 100Hz (dt=0.01) prevents tunneling and energy explosion.

### Force Integration Pipeline

```mermaid
flowchart LR
    A[Start of Frame] --> B[calc_forces]
    B --> C[Apply Gravity: F = m * g]
    C --> D[update_velocity]
    D --> E[v = v + F/m * dt]
    E --> F[calc_collisions]
    F --> G[Detect contacts]
    G --> H[Apply impulse response]
    H --> I[Position correction]
    I --> J[update_pos]
    J --> K[zero_forces]
    K --> L[End of Frame]
```

---

## Chapter 2: Spatial Partitioning Architecture

### The MultiGrid Hierarchy

```
World Space (infinite)
    │
    ▼
┌─────────────────────────────────────┐
│         MultiGrid<Cell>              │
│  - grids: HashMap<GridIndex, Grid>  │
│  - grid_width: 128                  │
│  - grid_height: 128                 │
└─────────────────────────────────────┘
    │
    │ GridIndex::from_pos(x, y, 128, 128)
    ▼
┌─────────────────────────────────────┐
│      UniverseGrid<Cell>              │
│  - offset: V2i (world position)     │
│  - width: 128, height: 128          │
│  - grid: Grid<Cell>                 │
└─────────────────────────────────────┘
    │
    │ Local coordinates (0..128, 0..128)
    ▼
┌─────────────────────────────────────┐
│         Grid<Cell>                   │
│  - cells: Vec<GridCell<Cell>>       │
│  - version: usize (for clearing)    │
│  - FACTOR: 1 (cell size)            │
└─────────────────────────────────────┘
    │
    │ Per-cell storage
    ▼
┌─────────────────────────────────────┐
│       GridCell<Cell>                 │
│  - value: Vec<Rc<RefCell<Cell>>>    │
│  - neighbors: Vec<Rc<RefCell<Cell>>>│
│  - version: usize                   │
└─────────────────────────────────────┘
```

### Neighbor Pre-Calculation

The key insight: when placing a cell at (x, y), register it as a neighbor in all 9 adjacent cells:

```rust
pub fn put(&mut self, x: usize, y: usize, value: GridCellRef<T>) {
    // Primary cell
    self.grid[grid_index(x + 1, y + 1, self.height)]
        .set_value(self.version, value.clone());
    
    // Register as neighbor in 3x3 area
    for px in 0..3 {
        for py in 0..3 {
            self.grid[grid_index(x + px, y + py, self.height)]
                .add_neighbor(self.version, value.clone());
        }
    }
}
```

**Result:** `get(x, y).neighbors` returns ALL cells within collision range in O(1).

### Version-Based Clearing

```rust
struct GridCell<T> {
    version: usize,
    value: Vec<GridCellRef<T>>,
    neighbors: Vec<GridCellRef<T>>,
}

impl GridCell<T> {
    fn ensure_version(&mut self, version: usize) {
        if version != self.version {
            self.version = version;
            self.value.clear();      // Reuse allocation
            self.neighbors.clear();
        }
    }
}
```

Instead of clearing every cell each frame, increment the global version. Cells only clear when accessed with a new version.

---

## Chapter 3: Inertia-Based Physics

### The Inertia Struct

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inertia {
    pub velocity: V2,           // Current velocity (pixels/frame)
    pub force: V2,              // Accumulated force
    pub pos: V2,                // Sub-pixel position
    pub mass: i32,              // 0 = static (infinite)
    pub elasticity: f64,        // 0..1 (0.2 = 20% bounce)
    pub collision_stats: usize, // Collision counter for damping
}
```

### Static vs Dynamic Objects

```rust
// Static objects (mass = 0) behave as infinite mass
fn set_static(&mut self) {
    self.inertia.velocity = V2::zero();
    self.inertia.pos = self.inertia.pos.round();
    self.inertia.mass = 0;  // Zero mass = immovable
}

// Dynamic objects (mass > 0) respond to forces
fn unset_static(&mut self) {
    self.inertia.mass = 1;
    self.inertia.elasticity = ELASTICITY;  // 0.2
}
```

---

## Chapter 4: Collision Response

### Impulse-Based Resolution

```rust
pub fn collide(inertia1: &Inertia, inertia2: &Inertia) -> (Inertia, Inertia) {
    let m1 = fixup_mass(inertia1.mass) as f64;
    let m2 = fixup_mass(inertia2.mass) as f64;
    let v1 = inertia1.velocity;
    let v2 = inertia2.velocity;
    let x1 = inertia1.pos;
    let x2 = inertia2.pos;
    
    // Collision normal
    let distance = x2.minus(x1).magnitude();
    let normal = x2.minus(x1).cdiv(distance);
    
    // Relative velocity along normal
    let v_rel = v2.minus(v1).dot(normal);
    
    if v_rel > 0.0 {
        // Moving apart - no response needed
        return (*inertia1, *inertia2);
    }
    
    // Impulse with elasticity
    let e = inertia1.elasticity.min(inertia2.elasticity);
    let j = (m1 * m2) / (m1 + m2) * (1.0 + e) * (v_rel);
    
    // Apply impulse
    let u1 = normal.cmul(j / m1).plus(v1);
    let u2 = normal.cmul(-j / m2).plus(v2);
    
    // Position correction (prevent sinking)
    let penetration = 1.0 - distance;
    let slop = 0.02;  // Tolerance
    let pos_correct = if penetration > slop {
        normal.cmul((penetration - slop) / (im1 + im2)).cmul(0.1)
    } else {
        V2::zero()
    };
    
    // Return corrected inertias
    (inertia1_new, inertia2_new)
}
```

### Collision Detection

```rust
pub fn is_collision(inertia1: &Inertia, inertia2: &Inertia) -> bool {
    // Infinite masses don't collide with each other
    if (inertia1.mass == 0) && (inertia2.mass == 0) {
        return false;
    }
    
    // Distance check (radius = 1.0)
    let normal = inertia1.pos.minus(inertia2.pos);
    if normal.magnitude_sqr() > 1.0 {
        return false;  // Too far apart
    }
    
    // Check if moving toward each other
    let rel_velocity = inertia1.velocity.minus(inertia2.velocity);
    let dot = rel_velocity.dot(normal);
    
    if dot >= 0.0 {
        return false;  // Moving apart
    }
    
    return true;  // Collision!
}
```

---

## Chapter 5: Player Character

### Player Structure

```rust
pub struct Player {
    pub w: usize,              // Sprite dimensions
    pub h: usize,
    pub inertia: Inertia,      // Physics state
    pub frame: usize,          // Animation frame (0-2)
    pub direction: i32,        // -1 = left, 1 = right
    pub life: u32,             // Health (decreases over time)
}
```

### Movement with Collision

```rust
pub fn move_right(&mut self) {
    self.inertia.velocity.x = 0.5;
    self.direction = 1;
    self.next_frame();  // Animate
}

fn get_next_player_inertia(&self, cells: &UniverseCells, dt: f64) -> Inertia {
    let new_pos = self.inertia.pos.plus(self.inertia.velocity.cmul(dt));
    
    // Check each sprite pixel
    for x in 0..self.w {
        for y in 0..self.h {
            let pos = V2 { x: new_pos.x + x as f64, y: new_pos.y + y as f64 };
            let grid = cells.grids.get(grid_index(pos.round())).unwrap();
            
            for cell in grid.get(pos.round()).neighbors {
                if Inertia::is_collision(&player_part, &cell.borrow().inertia) {
                    // Collision - stop at boundary
                    return Inertia {
                        velocity: V2::zero(),
                        pos: self.inertia.pos.round().to_v2(),
                        ..self.inertia
                    };
                }
            }
        }
    }
    
    Inertia { pos: new_pos, ..self.inertia }
}
```

---

## Chapter 6: Procedural Generation

### Perlin Noise Terrain

```rust
pub struct Generator {
    hasher: PermutationTable,  // Seeded with u32
}

fn generated_point(&self, pos: V2i) -> f64 {
    let posv = pos.to_v2().cmul(0.01);  // Scale for frequency
    
    // Layered noise (fractal-like)
    perlin_2d(Vector2::new(posv.x, posv.y), &self.hasher).abs()
        * perlin_2d(Vector2::new(posv.y * 0.3, posv.x * 0.4), &self.hasher).abs()
}
```

### Terrain Rules

```rust
pub fn generate_pristine_grid(&mut self, grid: &mut UniverseGrid<Cell>, ...) {
    for x in 0..width {
        for y in 0..height {
            let pos = V2i::new(x as i32, y as i32).plus(base_pos);
            let altitude = height as i32 - pos.y;
            
            if altitude > 0 {
                // Above ground - mountains
                let val = self.generated_point(V2i::new(pos.x, 0));
                if val * 100.0 > altitude as f64 {
                    // Mountain peak
                    grid.put(pos, wall_cell(pos, Color::hsv(30.0, 1.0, 0.5)));
                }
            } else {
                // Underground - caverns
                let val = self.generated_point(pos);
                let depth = -altitude as f64;
                if val < 0.02 + 0.5 / (depth * 0.1) {
                    // Cave wall
                    grid.put(pos, wall_cell(pos, Color::hsv(30.0, 1.0, (1.0 - val) * 0.5)));
                }
            }
        }
    }
}
```

---

## Chapter 7: WASM Integration

### Build Configuration

```toml
[lib]
crate-type = ["cdylib", "rlib"]  # WASM + Rust library

[features]
wasm = ["wasm-bindgen", "serde-wasm-bindgen"]
terminal = ["sdl2"]

[dependencies]
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.4"
noise = "0.8"
fnv = "1.0"
rand = "0.8"
```

### JavaScript Bindings

```rust
#[wasm_bindgen]
pub struct Game {
    width: usize,
    height: usize,
    pixels: Vec<u32>,       // RGBA framebuffer
    universe: Universe,
    keys: HashSet<Key>,
    shoot_color: Color,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(width: usize, height: usize) -> Game { ... }
    
    #[wasm_bindgen]
    pub fn tick(&mut self) { ... }
    
    #[wasm_bindgen]
    pub fn pixels(&self) -> Clamped<&[u8]> { ... }
    
    #[wasm_bindgen]
    pub fn key_down(&mut self, key: &str) { ... }
    
    #[wasm_bindgen]
    pub fn click(&mut self, x: usize, y: usize) { ... }
}
```

### Asset Pipeline (build.rs)

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for file in fs::read_dir("pngs")? {
        let img = image::open(file.path())?;
        let (w, h) = img.dimensions();
        
        // Generate const array
        output_file.write_all(
            format!("const {name}_IMAGE: [Color; {}] = [", w * h).as_bytes()
        )?;
        
        for pixel in img.pixels() {
            output_file.write_all(
                format!("Color {{ r: {}, g: {}, b: {} }},", r, g, b).as_bytes()
            )?;
        }
    }
    Ok(())
}
```

---

## Chapter 8: Chunked World Management

### Dynamic Loading

```rust
impl UniverseCells {
    // Grids that need to be loaded from storage
    pub fn get_missing_grids(&self, center: V2) -> Vec<GridIndex> {
        self.grids.get_dropped_grids(center.round(), drop_radius = 2)
    }
    
    // Grids that can be unloaded (far from player)
    pub fn get_droppable_grids(&self, center: V2) -> Vec<GridIndex> {
        self.grids.get_far_grids(center.round(), drop_radius = 2)
    }
    
    // Save grid before unloading
    pub fn save_grid(&mut self, grid_index: GridIndex) -> Option<JsValue> {
        self.grids.get(grid_index).map(|g| g.to_bytes().unwrap())
    }
    
    // Unload grid from memory
    pub fn drop_grid(&mut self, grid_index: GridIndex) {
        self.grids.drop_grid(grid_index)
    }
}
```

### Grid Serialization

```rust
pub fn to_bytes(&self) -> Result<JsValue, Error> {
    let mut items = Vec::new();
    
    for x in 0..self.width {
        for y in 0..self.height {
            for item_ref in self.get(x, y).value {
                items.push((x, y, item_ref.borrow().clone()));
            }
        }
    }
    
    serde_wasm_bindgen::to_value(&GridSerialData {
        width: self.width,
        height: self.height,
        version: self.version,
        items,
    })
}

pub fn from_bytes(bytes: JsValue) -> Result<Self, Error> {
    let data: GridSerialData<T> = serde_wasm_bindgen::from_value(bytes)?;
    
    let mut grid = Grid::new(data.width, data.height);
    grid.version = data.version;
    
    for (x, y, item) in data.items {
        let item_ref = Rc::new(RefCell::new(item));
        grid.put(x, y, item_ref);
    }
    
    Ok(grid)
}
```

---

## Summary

Rockies demonstrates a complete 2D physics simulation:

1. **Fixed timestep loop** - 100Hz for stability
2. **MultiGrid spatial hashing** - O(n) collision detection
3. **Impulse-based response** - Elasticity, mass, position correction
4. **Versioned grids** - Efficient clearing without reallocation
5. **Procedural terrain** - Perlin noise mountains and caverns
6. **WASM deployment** - Browser-native performance
7. **Chunked worlds** - Dynamic load/unload based on player position
8. **Player controller** - Inertia-based movement with sprite animation

---

## Next Steps

See [00-zero-to-physics-engineer.md](./00-zero-to-physics-engineer.md) for physics fundamentals.
See [01-grid-system-deep-dive.md](./01-grid-system-deep-dive.md) for MultiGrid implementation details.
