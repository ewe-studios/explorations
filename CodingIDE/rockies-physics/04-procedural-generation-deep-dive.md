---
title: "Procedural Generation Deep Dive"
subtitle: "Perlin noise terrain generation for infinite worlds"
---

# Procedural Generation Deep Dive

## Overview

Rockies uses Perlin noise to generate infinite, natural-looking terrain:
- Mountains above ground
- Caverns below ground
- Seamless chunk boundaries
- Deterministic from seed

```
┌─────────────────────────────────────────────────────────────────┐
│                 Procedural Generation Pipeline                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Generator (seeded)                                             │
│       │                                                          │
│       ▼                                                          │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Perlin Noise Function                          │ │
│  │  - Input: (x, y) world coordinates                         │ │
│  │  - Output: smooth random value (-1 to 1)                   │ │
│  └────────────────────────────────────────────────────────────┘ │
│       │                                                          │
│       ▼                                                          │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Terrain Rules                                  │ │
│  │  - Above ground: mountains if noise > threshold            │ │
│  │  - Below ground: caverns if noise < threshold              │ │
│  └────────────────────────────────────────────────────────────┘ │
│       │                                                          │
│       ▼                                                          │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Grid Population                                │ │
│  │  - Place static wall cells at generated positions          │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Chapter 1: Perlin Noise Basics

### 1.1 What is Perlin Noise?

Perlin noise generates **smooth randomness**:

```
Pure Random:          Perlin Noise:
0.83  0.12  0.95      0.45  0.52  0.58
0.07  0.64  0.21      0.48  0.55  0.61
0.91  0.33  0.47      0.51  0.58  0.65

Notice: Perlin noise has smooth transitions
```

### 1.2 How Perlin Noise Works

```
1. Create a grid of random gradient vectors
2. For any point (x, y):
   a. Find the 4 surrounding grid points
   b. Calculate dot product with gradients
   c. Interpolate between the 4 values
3. Result: smooth, continuous randomness
```

### 1.3 Using the noise Crate

```rust
use noise::{Vector2, core::perlin::perlin_2d, permutationtable::PermutationTable};

pub struct Generator {
    hasher: PermutationTable,  // Seeded random gradients
}

impl Generator {
    pub fn new(seed: u32) -> Self {
        Self {
            hasher: PermutationTable::new(seed),
        }
    }
    
    fn generated_point(&self, pos: V2i) -> f64 {
        // Scale coordinates for appropriate frequency
        let posv = pos.to_v2().cmul(0.01);
        
        // Get Perlin noise value
        let noise = perlin_2d(Vector2::new(posv.x, posv.y), &self.hasher);
        
        // Returns -1 to 1, we use absolute value
        noise.abs()
    }
}
```

---

## Chapter 2: Layered Noise

### 2.1 Single vs Layered Noise

**Single layer:**
```rust
let noise = perlin_2d(Vector2::new(x, y), &hasher);
```

**Layered (fractal-like):**
```rust
let n1 = perlin_2d(Vector2::new(posv.x, posv.y), &hasher);
let n2 = perlin_2d(Vector2::new(posv.y * 0.3, posv.x * 0.4), &hasher);
let combined = n1.abs() * n2.abs();
```

**Why layer?**
- Layer 1: Base terrain shape
- Layer 2: Fine details and variation
- Multiplication creates more interesting patterns

### 2.2 Frequency and Octaves

```rust
fn generated_point(&self, pos: V2i) -> f64 {
    let posv = pos.to_v2().cmul(0.01);  // Base frequency
    
    // Layer 1: Base frequency
    let n1 = perlin_2d(Vector2::new(posv.x, posv.y), &self.hasher);
    
    // Layer 2: Different frequency (swapped x/y for variation)
    let n2 = perlin_2d(Vector2::new(posv.y * 0.3, posv.x * 0.4), &self.hasher);
    
    // Combine
    n1.abs() * n2.abs()
}
```

---

## Chapter 3: Terrain Generation Rules

### 3.1 High-Level Rules

```
World is divided into two regions:

1. ABOVE GROUND (altitude > 0)
   - Generate mountains
   - Noise threshold vs altitude
   - Higher altitude = harder to place blocks

2. BELOW GROUND (altitude <= 0)
   - Generate caverns
   - Noise threshold increases with depth
   - Creates natural cave systems
```

### 3.2 Mountain Generation

```rust
fn generate_pristine_grid(&mut self, grid: &mut UniverseGrid<Cell>, ...) {
    let base_pos = grid_index.to_pos(width, height);
    
    for x in 0..width {
        for y in 0..height {
            let pos = V2i::new(x as i32, y as i32).plus(base_pos);
            let altitude = height as i32 - pos.y;
            
            if altitude > 0 {
                // ABOVE GROUND - mountains
                let val = self.generated_point(V2i::new(pos.x, 0));
                
                // Higher peaks are rarer
                if val * 100.0 > altitude as f64 {
                    let cell = Self::wall_cell(
                        pos,
                        Color::hsv(30.0, 1.0, 0.5)  // Brown mountain
                    );
                    grid.put(pos, Rc::new(RefCell::new(cell)));
                }
            }
        }
    }
}
```

**How it works:**
```
Altitude: 50
Noise value: 0.6
Threshold: 0.6 * 100 = 60
60 > 50? Yes → Place block

Altitude: 80
Noise value: 0.6
Threshold: 0.6 * 100 = 60
60 > 80? No → No block (too high)
```

### 3.3 Cavern Generation

```rust
if altitude <= 0 {
    // BELOW GROUND - caverns
    let val = self.generated_point(pos);
    let depth = -altitude as f64;
    
    // Threshold increases with depth (smaller caves deeper)
    let threshold = 0.02 + 0.5 / (depth * 0.1);
    
    if val < threshold {
        let cell = Self::wall_cell(
            pos,
            Color::hsv(30.0, 1.0, (1.0 - val) * 0.5)  // Darker deeper
        );
        grid.put(pos, Rc::new(RefCell::new(cell)));
    }
}
```

**How it works:**
```
Depth: 10
Noise: 0.03
Threshold: 0.02 + 0.5 / (10 * 0.1) = 0.02 + 0.5 = 0.52
0.03 < 0.52? Yes → Place wall (cavern opening)

Depth: 100
Noise: 0.03
Threshold: 0.02 + 0.5 / (100 * 0.1) = 0.02 + 0.05 = 0.07
0.03 < 0.07? Yes → Place wall (smaller caves)
```

---

## Chapter 4: The Generator Struct

### 4.1 Structure

```rust
pub struct Generator {
    hasher: PermutationTable,
}
```

### 4.2 Seeded Random

```rust
impl Generator {
    pub fn new(seed: u32) -> Self {
        Self {
            hasher: PermutationTable::new(seed),
        }
    }
}
```

**Same seed = same world:**
```rust
let gen1 = Generator::new(42);
let gen2 = Generator::new(42);
// Both generate identical terrain!

let gen3 = Generator::new(123);
// Different world
```

### 4.3 Generating a Grid

```rust
impl Generator {
    pub fn generate_pristine_grid(
        &mut self,
        grid: &mut UniverseGrid<Cell>,
        grid_index: GridIndex,
        width: usize,
        height: usize,
    ) {
        let base_pos = grid_index.to_pos(width, height);
        
        for x in 0..width {
            for y in 0..height {
                let pos = V2i::new(x as i32, y as i32).plus(base_pos);
                let altitude = height as i32 - pos.y;
                let above_ground = altitude > 0;
                
                if above_ground {
                    // Mountains
                    let val = self.generated_point(V2i::new(pos.x, 0));
                    if val * 100.0 > altitude as f64 {
                        let cell = Self::wall_cell(pos, Color::hsv(30.0, 1.0, 0.5));
                        grid.put(pos, Rc::new(RefCell::new(cell)));
                    }
                } else {
                    // Caverns
                    let val = self.generated_point(pos);
                    let depth = -altitude as f64;
                    if val < 0.02 + 0.5 / (depth * 0.1) {
                        let cell = Self::wall_cell(
                            pos,
                            Color::hsv(30.0, 1.0, (1.0 - val) * 0.5)
                        );
                        grid.put(pos, Rc::new(RefCell::new(cell)));
                    }
                }
            }
        }
    }
}
```

---

## Chapter 5: Wall Cells

### 5.1 Static Cell Creation

```rust
impl Generator {
    fn wall_cell(pos: V2i, color: Color) -> Cell {
        Cell {
            index: CellIndex::default(),
            color: color,
            inertia: Inertia {
                velocity: V2::zero(),
                force: V2::zero(),
                pos: pos.to_v2(),
                mass: 0,  // Static!
                elasticity: 1.0,
                collision_stats: 0,
            },
        }
    }
}
```

**Key property:** `mass = 0` means the cell never moves (part of the terrain).

### 5.2 Color Variation

```rust
// Mountain color (light brown)
Color::hsv(30.0, 1.0, 0.5)

// Cavern color (darker based on noise)
Color::hsv(30.0, 1.0, (1.0 - val) * 0.5)
```

---

## Chapter 6: Chunk Integration

### 6.1 Loading New Chunks

```rust
impl UniverseCells {
    pub fn ensure_grid(&mut self, grid_index: GridIndex) {
        let width = self.grids.grid_width;
        let height = self.grids.grid_height;
        let generator = &mut self.generator;
        
        let (is_new, grid) = self.grids.or_insert_with(
            grid_index,
            || UniverseGrid::new(grid_index, width, height)
        );
        
        if is_new {
            // Generate terrain for this new chunk
            generator.generate_pristine_grid(grid, grid_index, width, height);
        }
    }
}
```

### 6.2 Seamless Boundaries

Because Perlin noise is **continuous**, adjacent chunks have matching terrain:

```
Chunk A (0, 0) to (128, 128)
Chunk B (128, 0) to (256, 128)

At x=127 (Chunk A edge): noise(127, y)
At x=128 (Chunk B edge): noise(128, y)

These values are continuous - no visible seam!
```

---

## Chapter 7: Color Generation

### 7.1 HSV to RGB

```rust
impl Color {
    pub fn hsv(h: f64, s: f64, v: f64) -> Color {
        let c = v * s;
        let h_prime = h / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
        
        let (r, g, b) = match h_prime.floor() as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            5 => (c, 0.0, x),
            _ => (0.0, 0.0, 0.0),
        };
        
        let m = v - c;
        
        Color {
            r: ((r + m) * 255.0) as u8,
            g: ((g + m) * 255.0) as u8,
            b: ((b + m) * 255.0) as u8,
        }
    }
}
```

### 7.2 Using HSV

```rust
// Brown (hue=30, saturated, medium value)
Color::hsv(30.0, 1.0, 0.5)

// Lighter brown (higher value)
Color::hsv(30.0, 1.0, 0.8)

// Darker brown (lower value)
Color::hsv(30.0, 1.0, 0.3)
```

---

## Chapter 8: Practical Examples

### 8.1 Generating a Specific Feature

**Want to add underground lakes?**

```rust
if altitude < -50 {
    // Deep underground
    let water_level = -60;
    if pos.y > water_level {
        // Place water cell (blue, static)
        let cell = Self::wall_cell(pos, Color::hsv(200.0, 0.8, 0.5));
        grid.put(pos, Rc::new(RefCell::new(cell)));
    }
}
```

### 8.2 Adding Ore Veins

```rust
if altitude < -20 && altitude > -80 {
    let ore_noise = self.generated_point(pos);
    if ore_noise > 0.7 {
        // Rare ore deposit
        let cell = Self::wall_cell(pos, Color::hsv(50.0, 1.0, 0.7));  // Gold
        grid.put(pos, Rc::new(RefCell::new(cell)));
    }
}
```

### 8.3 Surface Vegetation

```rust
if altitude > 0 && altitude < 10 {
    let grass_noise = self.generated_point(V2i::new(pos.x * 2, pos.y * 2));
    if grass_noise > 0.6 {
        // Place grass/decoration
        let cell = Self::wall_cell(pos, Color::hsv(120.0, 0.8, 0.4));  // Green
        grid.put(pos, Rc::new(RefCell::new(cell)));
    }
}
```

---

## Summary

Procedural generation in Rockies:

1. **Perlin noise** - Smooth, continuous randomness
2. **Layered noise** - Multiple frequencies for detail
3. **Seeded generation** - Reproducible worlds
4. **Terrain rules** - Mountains above, caverns below
5. **Seamless chunks** - Continuous noise = no seams
6. **Color variation** - HSV for easy color control

---

## Next Steps

See [rust-revision.md](./rust-revision.md) for architecture patterns.
See [production-grade.md](./production-grade.md) for optimization strategies.
