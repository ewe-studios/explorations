---
title: "Rockies Physics: Rust Revision"
subtitle: "Architecture patterns and improvements for the physics engine"
---

# Rust Revision - Architecture Patterns

## Overview

Since Rockies is already written in Rust, this document focuses on:
- Architecture improvements
- Ownership pattern analysis
- Performance optimizations
- Alternative designs
- Extension patterns

---

## Chapter 1: Current Architecture Analysis

### 1.1 Ownership Strategy

```rust
// Current: Rc<RefCell<T>> for shared mutable cells
pub type GridCellRef<T> = Rc<RefCell<T>>;

struct GridCell<T> {
    value: Vec<GridCellRef<T>>,      // Shared ownership
    neighbors: Vec<GridCellRef<T>>,  // Multiple references OK
}
```

**Why Rc<RefCell<T>>?**
- Multiple grid cells reference the same object (via neighbors)
- Interior mutability needed (cell properties change during simulation)
- Single-threaded (Rc is fine, no need for Arc)

### 1.2 Alternative: Index-Based Storage

```rust
// Alternative: Use indices instead of Rc<RefCell<T>>
pub type CellId = usize;

struct GridCell {
    value: Vec<CellId>,
    neighbors: Vec<CellId>,
}

struct CellStorage {
    cells: Vec<Cell>,  // Owned storage
}

impl CellStorage {
    fn get(&self, id: CellId) -> &Cell { &self.cells[id] }
    fn get_mut(&mut self, id: CellId) -> &mut Cell { &mut self.cells[id] }
}
```

**Benefits:**
- No Rc overhead
- Better cache locality
- Easier serialization

**Tradeoffs:**
- Need to handle invalid indices
- Can't have multiple independent storages easily

---

## Chapter 2: Grid Architecture Improvements

### 2.1 Current: Version-Based Clearing

```rust
struct GridCell<T> {
    version: usize,
    value: Vec<GridCellRef<T>>,
    neighbors: Vec<GridCellRef<T>>,
}

fn ensure_version(&mut self, version: usize) {
    if version != self.version {
        self.version = version;
        self.value.clear();
        self.neighbors.clear();
    }
}
```

### 2.2 Alternative: Arena Allocation

```rust
use typed_arena::Arena;

struct GridFrame<'a> {
    arena: Arena<GridCellData>,
    cells: HashMap<(usize, usize), &'a GridCellData>,
}

struct Grid {
    frames: Vec<GridFrame>,
    current_frame: usize,
}

impl Grid {
    fn new_frame(&mut self) {
        self.current_frame += 1;
        if self.current_frame >= self.frames.len() {
            self.frames.push(GridFrame {
                arena: Arena::new(),
                cells: HashMap::new(),
            });
        }
    }
}
```

**Benefits:**
- Bulk deallocation (drop entire frame)
- No per-cell version checks

### 2.3 Alternative: Double Buffering

```rust
struct Grid<T> {
    front: Vec<GridCell<T>>,
    back: Vec<GridCell<T>>,
    active: bool,
}

impl Grid<T> {
    fn swap(&mut self) {
        self.active = !self.active;
        // Clear the new back buffer
        let back = if self.active { &mut self.front } else { &mut self.back };
        for cell in back.iter_mut() {
            cell.value.clear();
            cell.neighbors.clear();
        }
    }
    
    fn get_active(&self) -> &Vec<GridCell<T>> {
        if self.active { &self.front } else { &self.back }
    }
}
```

---

## Chapter 3: Physics Pipeline Improvements

### 3.1 Current: Sequential Collision Resolution

```rust
fn calc_collisions(&mut self, dt: f64) {
    self.collect_collisions();
    
    for (cell1, cell2) in &self.collisions_list {
        // Process one at a time
        let (new1, new2) = Inertia::collide(cell1, cell2);
        // ...
    }
}
```

### 3.2 Improved: Iterative Solver

```rust
fn calc_collisions(&mut self, dt: f64, iterations: usize) {
    self.collect_collisions();
    
    // Multiple passes for better accuracy
    for _ in 0..iterations {
        for (cell1, cell2) in &self.collisions_list {
            let (new1, new2) = Inertia::collide(
                &cell1.borrow().inertia,
                &cell2.borrow().inertia
            );
            cell1.borrow_mut().inertia = new1;
            cell2.borrow_mut().inertia = new2;
        }
    }
}
```

**Why iterative?**
- Sequential processing introduces order-dependency
- Iterative solvers converge toward simultaneous solution
- Common in physics engines (Box2D uses 10 iterations)

### 3.3 Alternative: Island-Based Solving

```rust
// Group connected bodies into "islands"
fn build_islands(&self) -> Vec<Island> {
    // Use union-find to group colliding bodies
    // Solve each island independently
}

fn solve_islands(&mut self) {
    let islands = self.build_islands();
    
    // Could parallelize: islands.par_iter().for_each(...)
    for island in islands {
        island.solve();
    }
}
```

---

## Chapter 4: Spatial Partitioning Improvements

### 4.1 Current: Fixed-Size Grid Cells

```rust
const FACTOR: usize = 1;  // Cell size

fn grid_index(x: usize, y: usize, height: usize) -> usize {
    (x / FACTOR) * (height / FACTOR + 2) + (y / FACTOR)
}
```

### 4.2 Alternative: QuadTree

```rust
enum QuadNode<T> {
    Leaf(Vec<T>),
    Branch {
        bounds: Rectangle,
        children: [Box<QuadNode<T>>; 4],
    },
}

impl QuadNode<Cell> {
    fn query(&self, range: Rectangle) -> Vec<&Cell> {
        // Recursive spatial query
    }
    
    fn insert(&mut self, cell: Cell) {
        // Subdivide if too many items
    }
}
```

**Benefits:**
- Adapts to object density
- Better for sparse distributions

**Tradeoffs:**
- More complex
- Higher memory overhead

### 4.3 Alternative: Spatial Hash

```rust
use std::collections::HashMap;

struct SpatialHash<T> {
    cell_size: f64,
    cells: HashMap<(i32, i32), Vec<T>>,
}

impl<T> SpatialHash<T> {
    fn hash(&self, x: f64, y: f64) -> (i32, i32) {
        (
            (x / self.cell_size) as i32,
            (y / self.cell_size) as i32,
        )
    }
    
    fn insert(&mut self, x: f64, y: f64, item: T) {
        let key = self.hash(x, y);
        self.cells.entry(key).or_default().push(item);
    }
    
    fn query(&self, x: f64, y: f64, radius: f64) -> Vec<&T> {
        // Check all cells in range
    }
}
```

**Benefits:**
- Handles infinite space naturally
- No chunk boundaries

---

## Chapter 5: Entity Component System (ECS) Pattern

### 5.1 Current: Monolithic Structs

```rust
pub struct Cell {
    index: CellIndex,
    color: Color,
    inertia: Inertia,
}

pub struct Player {
    w: usize,
    h: usize,
    inertia: Inertia,
    frame: usize,
    direction: i32,
    life: u32,
}
```

### 5.2 ECS Alternative

```rust
use hecs::World;

struct Position { x: f64, y: f64 }
struct Velocity { x: f64, y: f64 }
struct Mass(i32)
struct Sprite { width: usize, height: usize, frame: usize }
struct PlayerTag;

struct PhysicsWorld {
    world: World,
}

impl PhysicsWorld {
    fn update(&mut self, dt: f64) {
        // System: Apply forces
        for (_, (pos, vel, mass)) in self.world.query_mut::<(&mut Position, &Velocity, &Mass)>() {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
        }
        
        // System: Player movement
        for (_, (pos, vel)) in self.world.query_mut::<(&mut Position, &mut Velocity, With<PlayerTag>)>() {
            // Player-specific logic
        }
    }
}
```

**Benefits:**
- Better separation of concerns
- Easy to add new entity types
- Cache-friendly iteration

---

## Chapter 6: Serialization Improvements

### 6.2 Current: JsValue Serialization

```rust
pub fn to_bytes(&self) -> Result<JsValue, Error> {
    let items = self.collect_items();
    serde_wasm_bindgen::to_value(&GridSerialData { items })
}
```

### 6.2 Alternative: Bincode

```rust
use bincode;

#[derive(Serialize, Deserialize)]
struct GridSnapshot {
    version: u64,
    cells: Vec<(u32, u32, Cell)>,  // (x, y, cell)
}

impl Grid {
    fn save(&self) -> Vec<u8> {
        let snapshot = GridSnapshot {
            version: self.version as u64,
            cells: self.collect_items(),
        };
        bincode::serialize(&snapshot).unwrap()
    }
    
    fn load(data: &[u8]) -> Self {
        let snapshot: GridSnapshot = bincode::deserialize(data).unwrap();
        // Reconstruct grid
    }
}
```

**Benefits:**
- Compact binary format
- Fast serialization
- Works across platforms

---

## Chapter 7: Parallelization Options

### 7.1 Current: Single-Threaded

```rust
fn tick(&mut self) {
    // Everything runs on main thread
    self.calc_forces();
    self.update_velocity();
    self.cells.calc_collisions(self.dt);
    // ...
}
```

### 7.2 Rayon Parallelization

```rust
use rayon::prelude::*;

fn calc_forces(&mut self, gravity: V2) {
    self.moving_cells.par_iter_mut().for_each(|cell_ref| {
        let mut cell = cell_ref.borrow_mut();
        if cell.inertia.mass > 0 {
            cell.inertia.force = gravity.cmul(cell.inertia.mass as f64);
        }
    });
}
```

**Note:** This doesn't work well with RefCell - would need restructuring.

### 7.3 Restructured for Parallelization

```rust
use rayon::prelude::*;
use std::sync::Mutex;

struct PhysicsState {
    positions: Vec<V2>,
    velocities: Vec<V2>,
    masses: Vec<i32>,
    forces: Mutex<Vec<V2>>,  // Thread-safe accumulation
}

impl PhysicsState {
    fn calc_forces(&mut self, gravity: V2) {
        self.forces.get_mut().unwrap().par_iter_mut().enumerate().for_each(|(i, force)| {
            *force = gravity.cmul(self.masses[i] as f64);
        });
    }
}
```

---

## Chapter 8: Testing Improvements

### 8.1 Add Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_collision_symmetric(
        x1 in 0.0..100.0, y1 in 0.0..100.0,
        x2 in 0.0..100.0, y2 in 0.0..100.0,
    ) {
        let i1 = Inertia { pos: V2::new(x1, y1), ..default() };
        let i2 = Inertia { pos: V2::new(x2, y2), ..default() };
        
        // Collision check should be symmetric
        assert_eq!(
            Inertia::is_collision(&i1, &i2),
            Inertia::is_collision(&i2, &i1)
        );
    }
    
    #[test]
    fn test_collision_response_momentum_conserved(
        v1x in -1.0..1.0, v1y in -1.0..1.0,
        v2x in -1.0..1.0, v2y in -1.0..1.0,
    ) {
        let i1 = Inertia { velocity: V2::new(v1x, v1y), mass: 1, ..default() };
        let i2 = Inertia { velocity: V2::new(v2x, v2y), mass: 1, ..default() };
        
        let (new1, new2) = Inertia::collide(&i1, &i2);
        
        // Total momentum should be conserved
        let before = i1.velocity.plus(i2.velocity);
        let after = new1.velocity.plus(new2.velocity);
        
        prop_assert!(before.minus(after).magnitude_sqr() < 0.0001);
    }
}
```

---

## Summary

Architecture improvement options:

1. **Index-based storage** - Replace Rc<RefCell<T>> with indices
2. **Iterative solver** - Multiple collision resolution passes
3. **Alternative spatial structures** - QuadTree, Spatial Hash
4. **ECS pattern** - Better separation of concerns
5. **Bincode serialization** - Compact binary format
6. **Parallelization** - Rayon for multi-core
7. **Property testing** - Proptest for physics invariants

---

## Next Steps

See [production-grade.md](./production-grade.md) for deployment optimization.
