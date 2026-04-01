---
title: "Physics & Collision Deep Dive"
subtitle: "Impulse-based collision response and inertia physics"
---

# Physics & Collision Deep Dive

## Overview

This document covers the complete physics implementation in Rockies:
- Inertia and motion
- Collision detection
- Impulse-based response
- Position correction
- Static vs dynamic objects

```
┌─────────────────────────────────────────────────────────────────┐
│                    Collision Pipeline                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Collect Collisions                                          │
│     ┌─────────────────────────────────────────────────────┐    │
│     │ For each moving cell:                                │    │
│     │   - Get neighboring cells from grid                  │    │
│     │   - Check is_collision() for each neighbor           │    │
│     │   - Add to collisions_list if colliding              │    │
│     └─────────────────────────────────────────────────────┘    │
│                            ▼                                     │
│  2. Resolve Collisions                                          │
│     ┌─────────────────────────────────────────────────────┐    │
│     │ For each collision pair:                             │    │
│     │   - Calculate impulse magnitude j                    │    │
│     │   - Apply impulse to velocities                      │    │
│     │   - Calculate position correction                    │    │
│     │   - Update grid positions                            │    │
│     └─────────────────────────────────────────────────────┘    │
│                            ▼                                     │
│  3. Post-Resolution                                             │
│     ┌─────────────────────────────────────────────────────┐    │
│     │ - Check for highly colliding cells                   │    │
│     │ - Apply damping if needed                            │    │
│     │ - Filter out static cells from moving_cells          │    │
│     └─────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Chapter 1: Inertia - The Physics State

### 1.1 The Inertia Struct

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inertia {
    pub velocity: V2,           // Pixels per frame
    pub force: V2,              // Accumulated force
    pub pos: V2,                // Sub-pixel position
    pub mass: i32,              // 0 = static (infinite)
    pub elasticity: f64,        // 0..1 (bounciness)
    pub collision_stats: usize, // Collision counter
}
```

### 1.2 Mass and Static Objects

```rust
// Mass = 0 means static (immovable)
fn set_static(&mut self) {
    self.inertia.velocity = V2::zero();
    self.inertia.pos = self.inertia.pos.round();
    self.inertia.mass = 0;
}

// Mass > 0 means dynamic (responds to forces)
fn unset_static(&mut self) {
    self.inertia.mass = 1;
    self.inertia.elasticity = ELASTICITY;  // 0.2
}
```

### 1.3 Force Accumulation

```rust
fn calc_forces(&mut self, gravity: V2) {
    for (_cell_idx, cell_ref) in self.moving_cells.iter() {
        let mut cell = cell_ref.borrow_mut();
        if cell.inertia.mass > 0 {
            // F = m * g
            cell.inertia.force = gravity.cmul(cell.inertia.mass as f64);
        }
    }
}

fn zero_forces(&mut self) {
    for (_cell_idx, cell_ref) in self.moving_cells.iter() {
        let mut cell = cell_ref.borrow_mut();
        cell.inertia.force = V2::zero();
    }
}
```

### 1.4 Velocity Integration

```rust
fn update_velocity(&mut self, dt: f64) {
    for (_cell_idx, cell_ref) in self.moving_cells.iter() {
        let mut cell = cell_ref.borrow_mut();
        if cell.inertia.mass > 0 {
            // v = v + F/m * dt
            cell.inertia.velocity = clamp_velocity(
                cell.inertia.velocity.plus(
                    cell.inertia.force
                        .cdiv(cell.inertia.mass as f64)
                        .cmul(dt)
                )
            );
        }
    }
}

// Prevent runaway velocities
fn clamp_velocity(v: V2) -> V2 {
    let max = V2 { x: 1.0, y: 1.0 };
    let min = V2 { x: -1.0, y: -1.0 };
    v.min(max).max(min)
}
```

---

## Chapter 2: Collision Detection

### 2.1 The Distance Test

```rust
impl Inertia {
    pub fn is_collision(inertia1: &Inertia, inertia2: &Inertia) -> bool {
        // Infinite masses don't collide with each other
        if (inertia1.mass == 0) && (inertia2.mass == 0) {
            return false;
        }
        
        // Distance check (radius = 1.0)
        let normal = inertia1.pos.minus(inertia2.pos);
        if normal.magnitude_sqr() > 1.0 * 1.0 {
            return false;  // Too far apart
        }
        
        // Check if moving toward each other
        let rel_velocity = inertia1.velocity.minus(inertia2.velocity);
        let dot = rel_velocity.dot(normal);
        
        if dot >= 0.0 {
            return false;  // Moving apart
        }
        
        if dot * dot < 0.00001 {
            return false;  // Negligible velocity
        }
        
        true  // Collision!
    }
}
```

### 2.2 Understanding the Test

**Step 1: Distance Check**
```
Object A at (5, 5)
Object B at (7, 5)

delta = B - A = (2, 0)
distance² = 2² + 0² = 4

If distance² > 1.0: No collision (too far)
```

**Step 2: Approach Check**
```
Velocity A: (0.5, 0)  → moving right
Velocity B: (-0.5, 0) ← moving left

relative_velocity = A.vel - B.vel = (1.0, 0)
normal = A.pos - B.pos = (-2, 0)

dot = 1.0 * (-2) + 0 * 0 = -2

If dot < 0: Moving toward each other = collision!
```

### 2.3 Collecting Collisions

```rust
fn collect_collisions(&mut self) {
    self.collisions_map.clear();
    self.collisions_list.clear();
    
    for (_cell1_idx, cell1_ref) in self.moving_cells.iter() {
        let cell1 = cell1_ref.borrow();
        let grid_index = self.grids.pos_to_index(cell1.inertia.pos.round());
        
        // Get the grid containing this cell
        if self.grids.get(grid_index).is_none() {
            continue;  // Grid not loaded
        }
        
        // Get all neighbors from grid
        let get_res = self.grids.get(grid_index).unwrap()
            .get(cell1.inertia.pos.round());
        
        for cell2_ref in get_res.neighbors {
            // Skip self
            if Rc::ptr_eq(cell1_ref, cell2_ref) {
                continue;
            }
            
            let cell2 = cell2_ref.borrow();
            
            // Skip already-processed pairs
            if !self.collisions_map.insert((cell1.index, cell2.index)) {
                continue;
            }
            
            self.stats.collision_pairs_tested += 1;
            
            // Check if actually colliding
            if Inertia::is_collision(&cell1.inertia, &cell2.inertia) {
                self.collisions_list.push((cell1_ref.clone(), cell2_ref.clone()));
            }
        }
    }
}
```

---

## Chapter 3: Collision Response

### 3.1 The Physics of Collision

When two objects collide:
1. They exert equal and opposite impulses (Newton's 3rd law)
2. The impulse magnitude depends on mass, elasticity, and relative velocity
3. Position correction prevents objects from sinking into each other

### 3.2 Impulse Calculation

```rust
impl Inertia {
    pub fn collide(inertia1: &Inertia, inertia2: &Inertia) -> (Inertia, Inertia) {
        let m1 = fixup_mass(inertia1.mass) as f64;
        let m2 = fixup_mass(inertia2.mass) as f64;
        
        let v1 = inertia1.velocity;
        let v2 = inertia2.velocity;
        
        let x1 = inertia1.pos;
        let x2 = inertia2.pos;
        
        // Collision normal (direction from 1 to 2)
        let distance = x2.minus(x1).magnitude();
        let normal = x2.minus(x1).cdiv(distance);
        
        // Relative velocity along normal
        let v_rel = v2.minus(v1).dot(normal);
        
        // If moving apart, no response needed
        if v_rel > 0.0 {
            return (*inertia1, *inertia2);
        }
        
        // Combined elasticity
        let e = inertia1.elasticity.min(inertia2.elasticity);
        
        // Impulse magnitude
        // j = (m1 * m2) / (m1 + m2) * (1 + e) * v_rel
        let j = (m1 * m2) / (m1 + m2) * (1.0 + e) * v_rel;
        
        // Apply impulse to velocities
        let u1 = normal.cmul(j / m1).plus(v1);
        let u2 = normal.cmul(-j / m2).plus(v2);
        
        // Position correction
        let im1 = inverse_mass(m1);
        let im2 = inverse_mass(m2);
        
        let penetration = 1.0 - distance;
        let slop = 0.02;  // Tolerance
        
        let pos_correct = if penetration > slop {
            normal.cmul((penetration - slop) / (im1 + im2)).cmul(0.1)
        } else {
            V2::zero()
        };
        
        // Static objects don't move
        let uf1 = if inertia1.mass == 0 { v1 } else { u1 };
        let uf2 = if inertia2.mass == 0 { v2 } else { u2 };
        
        let p1 = if inertia1.mass == 0 {
            x1
        } else {
            x1.minus(pos_correct.cmul(im1))
        };
        let p2 = if inertia2.mass == 0 {
            x2
        } else {
            x2.plus(pos_correct.cmul(im2))
        };
        
        (
            Inertia { pos: p1, velocity: uf1, ..*inertia1 },
            Inertia { pos: p2, velocity: uf2, ..*inertia2 }
        )
    }
}
```

### 3.3 Understanding the Math

**Impulse Magnitude:**
```
j = (m1 * m2) / (m1 + m2) * (1 + e) * v_rel

Where:
- (m1 * m2) / (m1 + m2) = reduced mass
- (1 + e) = elasticity factor (1 = no bounce, 2 = perfect bounce)
- v_rel = relative velocity along collision normal

Example:
m1 = 1, m2 = 1, e = 0.2, v_rel = -1.0
j = (1 * 1) / (1 + 1) * (1 + 0.2) * (-1.0)
j = 0.5 * 1.2 * (-1.0) = -0.6
```

**Velocity Change:**
```
u1 = normal * (j / m1) + v1
u2 = normal * (-j / m2) + v2

Example:
j = -0.6, m1 = 1, m2 = 1, normal = (1, 0)
v1 = (0.5, 0), v2 = (-0.5, 0)

u1 = (1, 0) * (-0.6 / 1) + (0.5, 0) = (-0.6, 0) + (0.5, 0) = (-0.1, 0)
u2 = (1, 0) * (0.6 / 1) + (-0.5, 0) = (0.6, 0) + (-0.5, 0) = (0.1, 0)

Result: Objects bounce apart!
```

**Position Correction:**
```
penetration = 1.0 - distance  // How much they overlap
slop = 0.02                   // Tolerance

correction = normal * ((penetration - slop) / (im1 + im2)) * 0.1

The 0.1 factor is "positional correction factor" (Baumgarte stabilization)
Prevents explosive corrections while gradually fixing overlaps
```

### 3.4 Mass Handling

```rust
fn fixup_mass(mass: i32) -> i32 {
    if mass < 0 {
        panic!("Mass cannot be negative: {mass}");
    }
    if mass == 0 {
        return 10_000_000;  // "Infinite" mass for static objects
    }
    return mass;
}

fn inverse_mass(mass: f64) -> f64 {
    if mass.abs() < 0.000001 {
        return 0.0;  // Zero inverse mass = infinite mass
    }
    return 1.0 / mass;
}
```

---

## Chapter 4: Collision Resolution Pipeline

### 4.1 Full Resolution

```rust
fn calc_collisions(&mut self, dt: f64) {
    // Step 1: Collect all collisions
    self.collect_collisions();
    
    self.stats.collisions_count += self.collisions_list.len();
    
    // Step 2: Resolve each collision
    for (cell1_idx, cell2_idx) in self.collisions_list.iter() {
        let mut cell1 = cell1_idx.borrow_mut();
        let mut cell2 = cell2_idx.borrow_mut();
        
        let inertia1 = &cell1.inertia;
        let inertia2 = &cell2.inertia;
        
        // Check for static object involvement
        if ((inertia1.mass == 0) || (inertia2.mass == 0))
            && low_velocity_collision(inertia1, inertia2, dt)
        {
            // Both become static
            if inertia1.mass > 0 {
                cell1.set_static();
            }
            if inertia2.mass > 0 {
                cell2.set_static();
            }
            continue;
        }
        
        // Calculate collision response
        let (new_inertia1, new_inertia2) = Inertia::collide(inertia1, inertia2);
        
        // Update grid positions
        self.grids.update_cell_pos(
            cell1_idx,
            inertia1.pos.round(),
            new_inertia1.pos.round()
        );
        self.grids.update_cell_pos(
            cell2_idx,
            inertia2.pos.round(),
            new_inertia2.pos.round()
        );
        
        // Apply new inertias
        cell1.inertia = new_inertia1;
        cell2.inertia = new_inertia2;
    }
}

fn low_velocity_collision(inertia1: &Inertia, inertia2: &Inertia, dt: f64) -> bool {
    let threshold = dt / 2.0;
    inertia1.velocity.magnitude_sqr() < threshold
        && inertia2.velocity.magnitude_sqr() < threshold
}
```

### 4.2 Collision Damping

```rust
fn update_cell_collision(cell: &mut Cell, new_inertia: Inertia) {
    cell.inertia = new_inertia;
    cell.inertia.collision_stats += 1;
    
    // Dampen highly colliding cells
    if cell.inertia.collision_stats > 1000 {
        cell.inertia.velocity = V2::zero();
    }
}
```

### 4.3 Position Updates

```rust
fn update_pos(&mut self, dt: f64) {
    let mut grids_to_update = Vec::new();
    
    for (_cell_index, cell_ref) in &self.moving_cells {
        let mut cell = cell_ref.borrow_mut();
        let old_pos = cell.inertia.pos;
        let new_pos = cell.inertia.pos.plus(cell.inertia.velocity.cmul(dt));
        
        let new_pos_i = new_pos.round();
        
        // Update grid
        self.grids.update_cell_pos(&cell_ref, old_pos.round(), new_pos_i);
        
        // Update position
        cell.inertia.pos = new_pos;
        
        grids_to_update.push((self.grids.pos_to_index(new_pos_i), new_pos_i));
    }
    
    // Filter out cells that became static
    self.moving_cells.retain(|_, cell_ref| {
        let cell = cell_ref.borrow();
        cell.inertia.mass > 0
    });
}
```

---

## Chapter 5: Special Cases

### 5.1 Static-Dynamic Collision

When a dynamic object hits a static object (mass = 0):

```rust
// In collide():
let uf1 = if inertia1.mass == 0 { v1 } else { u1 };
let uf2 = if inertia2.mass == 0 { v2 } else { u2 };

let p1 = if inertia1.mass == 0 { x1 } else { x1.minus(pos_correct.cmul(im1)) };
let p2 = if inertia2.mass == 0 { x2 } else { x2.plus(pos_correct.cmul(im2)) };
```

**Static object:**
- Keeps its velocity (usually zero)
- Doesn't receive position correction

**Dynamic object:**
- Bounces based on elasticity
- Receives full position correction

### 5.2 Low Velocity Collisions

When objects are moving slowly, they "stack" instead of bouncing:

```rust
if ((inertia1.mass == 0) || (inertia2.mass == 0))
    && low_velocity_collision(inertia1, inertia2, dt)
{
    // Both become static - forms a stable pile
    if inertia1.mass > 0 {
        cell1.set_static();
    }
    if inertia2.mass > 0 {
        cell2.set_static();
    }
    continue;
}
```

### 5.3 Multiple Simultaneous Collisions

The collision list is processed sequentially. This is an approximation - for perfect physics, you'd solve all contacts simultaneously with a constraint solver.

**Rockies' approach:**
```rust
for (cell1, cell2) in &self.collisions_list {
    // Process one collision at a time
    let (new1, new2) = Inertia::collide(cell1.inertia, cell2.inertia);
    // ...
}
```

**Iterative approach (better but slower):**
```rust
for _ in 0..10 {  // Multiple iterations
    for (cell1, cell2) in &self.collisions_list {
        // Process all collisions multiple times
    }
}
```

---

## Chapter 6: Player Collision

### 6.1 Player-Specific Handling

```rust
fn get_next_player_inertia(&self, cells: &UniverseCells, dt: f64) -> Inertia {
    let new_pos = self.inertia.pos.plus(self.inertia.velocity.cmul(dt));
    
    // Check each pixel of player sprite
    for x in 0..self.w {
        for y in 0..self.h {
            let pos = V2 {
                x: new_pos.x + x as f64,
                y: new_pos.y + y as f64,
            };
            
            let grid = cells.grids.get(cells.grids.pos_to_index(pos.round())).unwrap();
            
            for cell_idx in grid.get(pos.round()).neighbors {
                let cell = cell_idx.borrow();
                
                // Create player-part inertia for comparison
                let player_part = Inertia {
                    pos: pos,
                    ..self.inertia
                };
                
                if Inertia::is_collision(&player_part, &cell.inertia) {
                    // Collision - stop and return current position
                    return Inertia {
                        velocity: V2::zero(),
                        pos: self.inertia.pos.round().to_v2(),
                        ..self.inertia
                    };
                }
            }
        }
    }
    
    // No collision - apply movement
    Inertia { pos: new_pos, ..self.inertia }
}
```

### 6.2 Why Per-Pixel Checking?

The player sprite is W×H pixels. Each pixel could potentially collide:

```
Player sprite (3x3):
┌─────┬─────┬─────┐
│  1  │  2  │  3  │
├─────┼─────┼─────┤
│  4  │  5  │  6  │
├─────┼─────┼─────┤
│  7  │  8  │  9  │
└─────┴─────┴─────┘

Check all 9 pixels against world cells
First collision = player stops
```

---

## Summary

Collision physics in Rockies:

1. **Detection** - Grid neighbors + distance + approach test
2. **Response** - Impulse-based with elasticity
3. **Correction** - Position correction to prevent overlap
4. **Static handling** - Mass = 0 means immovable
5. **Damping** - Highly colliding cells get zeroed velocity
6. **Player** - Per-pixel collision checking

---

## Next Steps

See [03-wasm-integration-deep-dive.md](./03-wasm-integration-deep-dive.md) for WASM deployment.
