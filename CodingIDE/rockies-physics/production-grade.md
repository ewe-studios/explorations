---
title: "Production-Grade Physics Engine"
subtitle: "Performance optimization, profiling, and deployment strategies"
---

# Production-Grade Physics Engine

## Overview

This document covers making Rockies production-ready:
- Performance optimization
- Memory management
- Profiling and debugging
- Deployment strategies
- Quality of life improvements

---

## Chapter 1: Performance Optimization

### 1.1 Hot Path Analysis

The physics tick is the hot path:

```rust
pub fn tick(&mut self) {
    for _ in 0..((1.0 / self.dt) as usize) {  // 10 substeps
        self.calc_forces();        // ~10% of time
        self.update_velocity();    // ~10% of time
        self.cells.calc_collisions(self.dt);  // ~60% of time (HOT)
        self.cells.update_pos(self.dt);       // ~15% of time
        self.zero_forces();        // ~5% of time
    }
}
```

**Focus optimization on `calc_collisions()` - it's 60% of frame time.**

### 1.2 Collision Detection Optimization

**Current:**
```rust
fn collect_collisions(&mut self) {
    for (_cell1_idx, cell1_ref) in self.moving_cells.iter() {
        let cell1 = cell1_ref.borrow();  // Borrow each iteration
        // ...
        for cell2_ref in get_res.neighbors {
            let cell2 = cell2_ref.borrow();  // Another borrow
            if Inertia::is_collision(&cell1.inertia, &cell2.inertia) {
                // ...
            }
        }
    }
}
```

**Optimized: Reduce borrow overhead**
```rust
fn collect_collisions(&mut self) {
    // Pre-collect inertias to avoid repeated borrows
    let mut cell_inertias: Vec<(CellIndex, Inertia, GridCellRef<Cell>)> = 
        self.moving_cells
            .iter()
            .map(|(idx, cell_ref)| (*idx, cell_ref.borrow().inertia, cell_ref.clone()))
            .collect();
    
    for (idx1, inertia1, cell1_ref) in &cell_inertias {
        // Single borrow per cell
        for cell2_ref in get_neighbors(*idx1) {
            // Direct comparison without extra borrows
        }
    }
}
```

### 1.3 Cache-Friendly Data Layout

**Current:**
```rust
struct Inertia {
    velocity: V2,      // 16 bytes
    force: V2,         // 16 bytes
    pos: V2,           // 16 bytes
    mass: i32,         // 4 bytes
    elasticity: f64,   // 8 bytes
    collision_stats: usize,  // 8 bytes
}  // ~68 bytes total, poor cache utilization
```

**Optimized (SoA - Structure of Arrays):**
```rust
struct PhysicsArrays {
    velocities: Vec<V2>,    // Contiguous
    forces: Vec<V2>,        // Contiguous
    positions: Vec<V2>,     // Contiguous
    masses: Vec<i32>,       // Contiguous
}

// Better cache utilization - iterate only what you need
fn calc_forces(&mut self, gravity: V2) {
    for i in 0..self.velocities.len() {
        if self.masses[i] > 0 {
            self.forces[i] = gravity.cmul(self.masses[i] as f64);
        }
    }
}
```

### 1.4 Avoiding Allocations in Hot Path

**Current:**
```rust
fn collect_collisions(&mut self) {
    self.collisions_list.clear();  // Clears but keeps capacity ✓
    self.collisions_map.clear();   // ✓
    
    // Good: Reuses allocations
}
```

**Potential issue:**
```rust
fn get_neighbors(&self, pos: V2i) -> Vec<GridCellRef<Cell>> {
    let mut res = Vec::new();  // New allocation each call!
    // ...
    res
}
```

**Optimized:**
```rust
// Use a reusable buffer
fn get_neighbors(&self, pos: V2i, buffer: &mut Vec<GridCellRef<Cell>>) {
    buffer.clear();  // Reuse capacity
    // ... fill buffer
}

// Usage in hot path:
let mut neighbor_buffer = Vec::with_capacity(16);
for cell in &self.moving_cells {
    self.get_neighbors(cell.pos, &mut neighbor_buffer);
    // Process neighbors
}
```

---

## Chapter 2: Memory Management

### 2.1 Object Pool for Cells

```rust
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct CellKey; }

struct CellPool {
    cells: SlotMap<CellKey, Cell>,
    free_list: Vec<CellKey>,
}

impl CellPool {
    fn new() -> Self {
        Self {
            cells: SlotMap::with_key(),
            free_list: Vec::new(),
        }
    }
    
    fn allocate(&mut self, cell: Cell) -> CellKey {
        if let Some(key) = self.free_list.pop() {
            self.cells[key] = cell;
            key
        } else {
            self.cells.insert(cell)
        }
    }
    
    fn deallocate(&mut self, key: CellKey) {
        self.free_list.push(key);
        // Cell data stays in slotmap for reuse
    }
}
```

### 2.2 Grid Memory Budget

```rust
struct GridConfig {
    max_loaded_chunks: usize,  // e.g., 25 chunks = ~4MB
    chunk unload_distance: usize,  // Unload chunks beyond this
    chunk load_distance: usize,    // Load chunks within this
}

impl GridConfig {
    fn estimate_memory(&self) -> usize {
        // Each chunk: 128x128 grid + cells
        let bytes_per_chunk = 128 * 128 * std::mem::size_of::<GridCell<Cell>>();
        self.max_loaded_chunks * bytes_per_chunk
    }
}
```

### 2.3 GC for Old Chunks

```rust
fn cleanup_old_chunks(&mut self, player_pos: V2i) {
    let far_chunks = self.grids.get_far_grids(player_pos, UNLOAD_DISTANCE);
    
    for chunk_index in far_chunks {
        // Save to storage first if dirty
        if self.is_dirty(chunk_index) {
            self.save_chunk(chunk_index);
        }
        
        // Then unload
        self.grids.drop_grid(chunk_index);
    }
}
```

---

## Chapter 3: Profiling

### 3.1 Built-in Stats Tracking

```rust
#[wasm_bindgen]
impl Stats {
    pub fn ticks(&self) -> usize { self.ticks }
    pub fn cells_count(&self) -> usize { self.cells_count }
    pub fn collisions_count(&self) -> usize { self.collisions_count }
    pub fn collision_pairs_tested(&self) -> usize { self.collision_pairs_tested }
}

// Expose to JavaScript for monitoring
const stats = game.stats();
console.log(`Collisions: ${stats.collisions_count}, Tested: ${stats.collision_pairs_tested}`);
```

### 3.2 Timing Individual Phases

```rust
use web_sys::console;
use wasm_bindgen::JsCast;

fn tick(&mut self) {
    let start = performance_now();
    
    let forces_start = performance_now();
    self.calc_forces();
    console::log(&format!("calc_forces: {}ms", performance_now() - forces_start));
    
    let velocity_start = performance_now();
    self.update_velocity();
    console::log(&format!("update_velocity: {}ms", performance_now() - velocity_start));
    
    let collision_start = performance_now();
    self.cells.calc_collisions(self.dt);
    console::log(&format!("calc_collisions: {}ms", performance_now() - collision_start));
    
    console::log(&format!("Total tick: {}ms", performance_now() - start));
}
```

### 3.3 Using perf (Native)

For terminal mode profiling:

```bash
# Build with debug symbols
cargo build --release --features terminal

# Profile with perf
perf record --call-graph dwarf ./target/release/rockies
perf report --stdio
```

---

## Chapter 4: Error Handling

### 4.1 Panic Hook for WASM

```rust
// In lib.rs
#[cfg(feature = "console_error_panic_hook")]
console_error_panic_hook::set_once();
```

This converts Rust panics to readable JavaScript errors.

### 4.2 Graceful Degradation

```rust
fn tick(&mut self) {
    // Catch panics in physics step
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        self.physics_step();
    }));
    
    if result.is_err() {
        // Log error, reset physics state
        console_error!("Physics panic! Resetting...");
        self.reset_physics();
    }
}
```

### 4.3 Save on Crash

```rust
use std::panic;

panic::set_hook(Box::new(|info| {
    // Try to save game state before crashing
    if let Some(game_state) = get_game_state() {
        let _ = game_state.save();
    }
    
    // Then panic normally
    console_error_panic_hook::hook(info);
}));
```

---

## Chapter 5: Deployment

### 5.1 CDN Deployment

```bash
# Build optimized WASM
wasm-pack build --target web --out-dir pkg --release

# Upload to CDN
aws s3 sync pkg s3://my-cdn/rockies/pkg/ \
    --cache-control "public,max-age=31536000,immutable"
```

### 5.2 Compression

```bash
# Brotli compression (best for WASM)
brotli -q 11 pkg/rockies_bg.wasm  # Creates .br file

# Configure server to serve .br with correct headers
# Content-Encoding: br
# Content-Type: application/wasm
```

### 5.3 Lazy Loading

```javascript
// Don't load WASM until needed
async function initGame() {
    const { Game } = await import('./pkg/rockies.js');
    const game = new Game(800, 600);
    return game;
}

// Start game when user clicks "Play"
document.getElementById('play-btn').addEventListener('click', async () => {
    const game = await initGame();
    startGameLoop(game);
});
```

---

## Chapter 6: Quality of Life

### 6.1 Debug Overlay

```rust
#[wasm_bindgen]
impl Game {
    #[wasm_bindgen]
    pub fn toggle_debug(&mut self) {
        self.show_debug = !self.show_debug;
    }
    
    fn render_debug(&mut self) {
        // Draw collision bounds
        // Draw grid lines
        // Show velocity vectors
    }
}
```

### 6.2 Speed Control

```rust
#[wasm_bindgen]
impl Game {
    #[wasm_bindgen]
    pub fn set_time_scale(&mut self, scale: f64) {
        self.time_scale = scale.clamp(0.0, 2.0);
    }
    
    fn tick(&mut self) {
        let steps = ((1.0 / self.dt) as f64 * self.time_scale) as usize;
        for _ in 0..steps {
            self.physics_step();
        }
    }
}
```

### 6.3 Save/Load State

```rust
#[wasm_bindgen]
impl Game {
    #[wasm_bindgen]
    pub fn save_state(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&GameState {
            player: self.universe.player,
            loaded_grids: self.universe.cells.get_loaded_grids(),
        }).unwrap()
    }
    
    #[wasm_bindgen]
    pub fn load_state(&mut self, state: JsValue) {
        let game_state: GameState = serde_wasm_bindgen::from_value(state).unwrap();
        self.universe.player = game_state.player;
        // Restore grids...
    }
}
```

---

## Chapter 7: Testing in Production

### 7.1 Telemetry

```rust
fn tick(&mut self) {
    let start = performance_now();
    
    // ... physics step ...
    
    let frame_time = performance_now() - start;
    
    // Send telemetry every 60 frames
    self.frame_count += 1;
    if self.frame_count >= 60 {
        send_telemetry("frame_time", frame_time);
        self.frame_count = 0;
    }
}
```

### 7.2 A/B Testing Optimizations

```rust
// Different collision strategies for different users
let strategy = match user_id % 3 {
    0 => CollisionStrategy::Sequential,
    1 => CollisionStrategy::Iterative(5),
    2 => CollisionStrategy::Iterative(10),
    _ => unreachable!(),
};

self.cells.calc_collisions(self.dt, strategy);
```

---

## Summary

Production considerations:

1. **Performance** - Optimize hot paths, cache-friendly layouts
2. **Memory** - Object pools, chunk budgets, GC
3. **Profiling** - Built-in stats, timing, perf
4. **Error handling** - Panic hooks, graceful degradation
5. **Deployment** - CDN, compression, lazy loading
6. **Quality of life** - Debug overlay, speed control, save/load
7. **Telemetry** - Frame time tracking, A/B testing

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-28 | Initial production guide created |

*This is a living document. Update as new optimizations are discovered.*
