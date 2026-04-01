---
title: "Zero to Physics Engineer"
subtitle: "Understanding 2D granular physics simulation from first principles"
level: "Beginner to Intermediate - No prior physics simulation knowledge assumed"
---

# Zero to Physics Engineer

## Introduction

This guide takes you from zero knowledge to understanding how to build real-time 2D physics simulations like Rockies.

---

## Chapter 1: What is a Physics Simulation?

### 1.1 The Basic Idea

A physics simulation updates the state of objects over time according to physical laws:

```
Position + Velocity * Time = New Position
Force / Mass = Acceleration
```

### 1.2 The Simulation Loop

Every physics engine runs a loop like this:

```
┌─────────────────────────────────────────┐
│           Physics Loop                   │
│                                          │
│  1. Apply Forces (gravity, input)       │
│       ▼                                  │
│  2. Integrate (update velocities)        │
│       ▼                                  │
│  3. Detect Collisions                    │
│       ▼                                  │
│  4. Resolve Collisions                   │
│       ▼                                  │
│  5. Update Positions                     │
│       ▼                                  │
│  (repeat at fixed timestep)              │
└─────────────────────────────────────────┘
```

### 1.3 Why Fixed Timestep?

```rust
// BAD: Variable timestep
fn update(&mut self, delta_time: f64) {
    // Physics behaves differently at different framerates!
    self.velocity += self.force * delta_time;
}

// GOOD: Fixed timestep
fn update(&mut self) {
    const DT: f64 = 0.01;  // Always 100Hz
    self.velocity += self.force * DT;
}
```

**Fixed timestep ensures:**
- Deterministic behavior
- Stable collisions
- Consistent gameplay

---

## Chapter 2: Vectors - The Language of Physics

### 2.1 What is a Vector?

A 2D vector has two components: x and y.

```rust
#[derive(Clone, Copy, Debug)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}
```

### 2.2 Vector Operations

```rust
impl V2 {
    // Addition: combine vectors
    pub fn plus(&self, other: V2) -> V2 {
        V2 { x: self.x + other.x, y: self.y + other.y }
    }
    
    // Subtraction: difference between vectors
    pub fn minus(&self, other: V2) -> V2 {
        V2 { x: self.x - other.x, y: self.y - other.y }
    }
    
    // Scalar multiplication: scale vector
    pub fn cmul(&self, scalar: f64) -> V2 {
        V2 { x: self.x * scalar, y: self.y * scalar }
    }
    
    // Dot product: projection
    pub fn dot(&self, other: V2) -> f64 {
        self.x * other.x + self.y * other.y
    }
    
    // Magnitude: length of vector
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    
    // Normalize: unit vector (length = 1)
    pub fn normalize(&self) -> V2 {
        let mag = self.magnitude();
        V2 { x: self.x / mag, y: self.y / mag }
    }
}
```

### 2.3 Visualizing Vectors

```
Position Vector:
    (0,0) ──────▶ (3, 2)
    Position = V2 { x: 3.0, y: 2.0 }

Velocity Vector:
    Object at (5, 5)
    Velocity = V2 { x: 1.0, y: -0.5 }
    Moving: right 1, up 0.5 per frame

Force Vector:
    Gravity = V2 { x: 0.0, y: 0.1 }
    Pulling objects downward
```

---

## Chapter 3: Inertia and Motion

### 3.1 What is Inertia?

Inertia describes an object's resistance to changes in motion. In Rockies, we track:

```rust
pub struct Inertia {
    pub velocity: V2,    // How fast and in what direction
    pub force: V2,       // Current forces applied
    pub pos: V2,         // Where it is (sub-pixel accurate)
    pub mass: i32,       // How heavy (0 = immovable)
    pub elasticity: f64, // How bouncy (0..1)
}
```

### 3.2 Newton's Second Law

```
F = m * a
Force = Mass * Acceleration
```

Rearranged for our simulation:

```rust
// Acceleration from force
a = F / m

// Update velocity
v_new = v_old + a * dt
v_new = v_old + (F / m) * dt
```

### 3.3 Integration (Euler Method)

```rust
fn update_velocity(&mut self, dt: f64) {
    // Semi-implicit Euler integration
    let acceleration = self.force / self.mass;
    self.velocity = self.velocity + acceleration * dt;
}

fn update_position(&mut self, dt: f64) {
    self.pos = self.pos + self.velocity * dt;
}
```

**Semi-implicit Euler** updates velocity first, then uses new velocity for position. This is more stable than explicit Euler.

### 3.4 Gravity

```rust
fn apply_gravity(&mut self, gravity: V2) {
    // F = m * g
    self.force = self.force + gravity * self.mass as f64;
}
```

---

## Chapter 4: Collision Detection

### 4.1 The Naive Approach (O(n²))

```rust
// Check every pair - SLOW!
for i in 0..n {
    for j in (i+1)..n {
        if colliding(&objects[i], &objects[j]) {
            resolve(&mut objects[i], &mut objects[j]);
        }
    }
}
```

With 1000 objects: 1000 * 999 / 2 = 499,500 checks per frame!

### 4.2 Spatial Partitioning

The key insight: **Objects can only collide if they're near each other.**

```
┌─────────────────────────────────────────────┐
│              Game World                      │
│  ┌─────┬─────┬─────┬─────┐                  │
│  │ A   │ B   │     │     │                  │
│  │     │     │     │     │                  │
│  ├─────┼─────┼─────┼─────┤                  │
│  │     │  C  │  D  │     │  Only check:    │
│  │     │     │     │     │  A-B, C-D       │
│  ├─────┼─────┼─────┼─────┤  (neighbors)    │
│  │     │     │  E  │  F  │                  │
│  │     │     │     │     │                  │
│  └─────┴─────┴─────┴─────┘                  │
└─────────────────────────────────────────────┘
```

### 4.3 Grid-Based Partitioning

```rust
// Divide world into cells
const CELL_SIZE: usize = 128;

struct Grid {
    cells: Vec<Vec<GameObject>>,
}

fn get_cell(x: usize, y: usize) -> usize {
    x / CELL_SIZE + (y / CELL_SIZE) * GRID_WIDTH
}

// Objects in adjacent cells are neighbors
fn get_neighbors(x: usize, y: usize) -> Vec<&GameObject> {
    let mut neighbors = Vec::new();
    for dx in -1..=1 {
        for dy in -1..=1 {
            neighbors.extend(grid[x+dx][y+dy].iter());
        }
    }
    neighbors
}
```

### 4.4 Rockies' MultiGrid

Rockies uses a hierarchical approach:

```
World → MultiGrid → UniverseGrid → Grid → GridCell

MultiGrid: HashMap of chunks (lazy loaded)
UniverseGrid: 128x128 chunk with offset
Grid: Local cell storage
GridCell: Items at position + neighbors
```

### 4.5 Distance-Based Collision Test

```rust
fn is_collision(obj1: &Inertia, obj2: &Inertia) -> bool {
    // Vector from obj1 to obj2
    let delta = obj2.pos - obj1.pos;
    
    // Distance squared (avoid sqrt for performance)
    let dist_sq = delta.x * delta.x + delta.y * delta.y;
    
    // Objects have radius 1.0
    if dist_sq > 1.0 * 1.0 {
        return false;  // Too far apart
    }
    
    // Check if moving toward each other
    let rel_vel = obj1.velocity - obj2.velocity;
    let dot = rel_vel.dot(delta);
    
    if dot >= 0.0 {
        return false;  // Moving apart
    }
    
    true  // Collision!
}
```

---

## Chapter 5: Collision Response

### 5.1 What Happens on Collision?

When two objects collide:
1. They exert equal and opposite forces (Newton's 3rd law)
2. They bounce based on elasticity
3. They shouldn't overlap (position correction)

### 5.2 Impulse-Based Response

An **impulse** is an instantaneous change in momentum.

```
Impulse = j * normal

Where:
j = (m1 * m2) / (m1 + m2) * (1 + e) * v_rel

e = elasticity (0 = no bounce, 1 = perfect bounce)
v_rel = relative velocity along collision normal
```

### 5.3 Collision Response Implementation

```rust
fn collide(a: &Inertia, b: &Inertia) -> (Inertia, Inertia) {
    let m1 = fixup_mass(a.mass) as f64;
    let m2 = fixup_mass(b.mass) as f64;
    let v1 = a.velocity;
    let v2 = b.velocity;
    let x1 = a.pos;
    let x2 = b.pos;
    
    // Collision normal (direction from a to b)
    let dist = (x2 - x1).magnitude();
    let normal = (x2 - x1) / dist;
    
    // Relative velocity
    let v_rel = (v2 - v1).dot(normal);
    
    // If moving apart, no response needed
    if v_rel > 0.0 {
        return (a, b);
    }
    
    // Combined elasticity
    let e = a.elasticity.min(b.elasticity);
    
    // Impulse magnitude
    let j = (m1 * m2) / (m1 + m2) * (1.0 + e) * v_rel;
    
    // Apply impulse to velocities
    let u1 = normal * (j / m1) + v1;
    let u2 = normal * (-j / m2) + v2;
    
    // Position correction (prevent sinking)
    let penetration = 1.0 - dist;
    let slop = 0.02;  // Tolerance
    let correction = if penetration > slop {
        normal * ((penetration - slop) / (1/m1 + 1/m2)) * 0.1
    } else {
        V2::zero()
    };
    
    // Return new states
    (
        Inertia { pos: x1 - correction/m1, velocity: u1, ..a },
        Inertia { pos: x2 + correction/m2, velocity: u2, ..b }
    )
}
```

### 5.4 Static vs Dynamic Objects

Objects with mass = 0 are "static" (immovable):

```rust
fn fixup_mass(mass: i32) -> i32 {
    if mass < 0 {
        panic!("Mass cannot be negative");
    }
    if mass == 0 {
        return 10_000_000;  // "Infinite" mass
    }
    mass
}
```

When a dynamic object hits a static object:
- Static object doesn't move (infinite inertia)
- Dynamic object bounces back

---

## Chapter 6: The Physics Pipeline (Deep Dive)

### 6.1 Complete Frame Breakdown

```rust
pub fn tick(&mut self) {
    // Multiple substeps for stability
    for _ in 0..((1.0 / self.dt) as usize) {
        // PHASE 1: Forces
        self.calc_forces();  // Apply gravity to all objects
        
        // PHASE 2: Integration
        self.update_velocity();  // v = v + F/m * dt
        
        // PHASE 3: Collision Detection
        self.cells.calc_collisions(self.dt);
        
        // PHASE 4: Player Update
        self.player.update_pos(&self.cells, self.dt);
        
        // PHASE 5: Position Update
        self.cells.update_pos(self.dt);
        
        // PHASE 6: Reset
        self.zero_forces();  // Clear forces for next frame
    }
}
```

### 6.2 Why Multiple Substeps?

```
Single step at dt=0.1:
  Object moves 10 pixels, might tunnel through wall

10 substeps at dt=0.01:
  Object moves 1 pixel per substep, collision caught early
```

### 6.3 Force Accumulation

```rust
fn calc_forces(&mut self) {
    // Reset all forces first
    self.zero_forces();
    
    // Apply gravity to each object
    for cell in &mut self.cells.moving_cells {
        if cell.mass > 0 {
            cell.force = cell.force + gravity * cell.mass as f64;
        }
    }
}
```

### 6.4 Collision Pipeline

```rust
fn calc_collisions(&mut self, dt: f64) {
    // Step 1: Collect all potential collisions
    self.collect_collisions();
    
    // Step 2: Resolve each collision
    for (cell1, cell2) in &self.collisions_list {
        let (new_inertia1, new_inertia2) = 
            Inertia::collide(&cell1.inertia, &cell2.inertia);
        
        // Update grid positions
        self.grids.update_cell_pos(
            cell1, 
            cell1.inertia.pos.round(), 
            new_inertia1.pos.round()
        );
        
        // Update cell state
        cell1.inertia = new_inertia1;
        cell2.inertia = new_inertia2;
    }
}
```

---

## Chapter 7: Player Physics

### 7.1 Player as a Physics Object

The player is special:
- Has sprite graphics (unlike simple cells)
- Controlled by keyboard input
- Must collide with world cells

```rust
pub struct Player {
    pub w: usize,              // Sprite width
    pub h: usize,              // Sprite height
    pub inertia: Inertia,      // Physics state
    pub frame: usize,          // Animation frame
    pub direction: i32,        // Facing direction
    pub life: u32,             // Health
}
```

### 7.2 Input Handling

```rust
pub fn move_left(&mut self) {
    self.inertia.velocity.x = -0.5;
    self.direction = -1;
    self.frame += 1;  // Animate
}

pub fn move_right(&mut self) {
    self.inertia.velocity.x = 0.5;
    self.direction = 1;
    self.frame += 1;
}

pub fn move_up(&mut self) {
    self.inertia.velocity.y = -0.5;
    self.frame += 1;
}
```

### 7.3 Player Collision Detection

```rust
fn get_next_player_inertia(&self, cells: &UniverseCells, dt: f64) -> Inertia {
    // Where we WANT to be
    let new_pos = self.inertia.pos + self.inertia.velocity * dt;
    
    // Check each pixel of our sprite
    for x in 0..self.w {
        for y in 0..self.h {
            let pixel_pos = V2 {
                x: new_pos.x + x as f64,
                y: new_pos.y + y as f64,
            };
            
            // Get cells near this pixel
            let grid = cells.grids.get(grid_index(pixel_pos.round())).unwrap();
            
            for cell in grid.get(pixel_pos.round()).neighbors {
                if Inertia::is_collision(&player_pixel, &cell.inertia) {
                    // Collision! Don't move.
                    return Inertia {
                        velocity: V2::zero(),
                        pos: self.inertia.pos.round().to_v2(),
                        ..self.inertia
                    };
                }
            }
        }
    }
    
    // No collision - move to new position
    Inertia { pos: new_pos, ..self.inertia }
}
```

---

## Chapter 8: Procedural Generation

### 8.1 What is Procedural Generation?

Creating content algorithmically instead of by hand.

### 8.2 Perlin Noise Basics

Perlin noise generates smooth, natural-looking randomness:

```
Perlin Noise at different points:
(0, 0)   → 0.23
(10, 0)  → 0.45
(20, 0)  → 0.67
(30, 0)  → 0.34

Notice: Nearby points have similar values (smooth)
```

### 8.3 Layered Noise

```rust
fn generated_point(&self, pos: V2i) -> f64 {
    let posv = pos.to_v2() * 0.01;  // Scale for frequency
    
    // Layer 1: Base terrain
    let n1 = perlin_2d(Vector2::new(posv.x, posv.y), &self.hasher);
    
    // Layer 2: Detail (different frequency)
    let n2 = perlin_2d(Vector2::new(posv.y * 0.3, posv.x * 0.4), &self.hasher);
    
    // Combine
    n1.abs() * n2.abs()
}
```

### 8.4 Terrain Rules

```rust
// Above ground: mountains
let noise_value = generated_point(x, 0);
if noise_value * 100.0 > altitude {
    // Place mountain block
}

// Underground: caverns
let noise_value = generated_point(x, y);
let depth = -altitude;
if noise_value < 0.02 + 0.5 / (depth * 0.1) {
    // Place cavern wall
}
```

---

## Chapter 9: Performance Optimization

### 9.1 Why MultiGrid?

Naive collision detection: O(n²)
- 1000 objects = 499,500 checks
- 10000 objects = 49,995,000 checks

MultiGrid: O(n)
- 1000 objects = ~1000 checks (only neighbors)
- 10000 objects = ~10000 checks

### 9.2 Version-Based Clearing

Instead of clearing every cell each frame:

```rust
// BAD: Clear all cells
fn clear(&mut self) {
    for cell in &mut self.grid {
        cell.value.clear();
        cell.neighbors.clear();
    }
}

// GOOD: Version tracking
pub fn put(&mut self, ...) {
    if version != cell.version {
        cell.version = version;
        cell.value.clear();  // Only clear when first accessed
        cell.neighbors.clear();
    }
    cell.value.push(item);
}

fn new_frame(&mut self) {
    self.version += 1;  // O(1) "clear"
}
```

### 9.3 Neighbor Pre-Calculation

```rust
// When placing an item, pre-calculate which cells need it as neighbor
for dx in -1..=1 {
    for dy in -1..=1 {
        grid[x + dx][y + dy].neighbors.push(item);
    }
}

// Later: O(1) neighbor lookup
let neighbors = grid[x][y].neighbors;
```

---

## Summary

You now understand:

1. **Vectors** - Position, velocity, force as V2
2. **Integration** - Euler method for motion
3. **Collision Detection** - Spatial partitioning with MultiGrid
4. **Collision Response** - Impulse-based physics with elasticity
5. **Player Controller** - Input-driven movement with collision
6. **Procedural Generation** - Perlin noise terrain
7. **Performance** - O(n) collision, version-based clearing

---

## Next Steps

See [01-grid-system-deep-dive.md](./01-grid-system-deep-dive.md) for MultiGrid implementation.
See [02-physics-collision-deep-dive.md](./02-physics-collision-deep-dive.md) for collision response details.
