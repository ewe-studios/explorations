---
title: "Grid System Deep Dive"
subtitle: "MultiGrid spatial hashing for O(n) collision detection"
---

# Grid System Deep Dive

## Overview

The MultiGrid system is the foundation of Rockies' performance. It reduces collision detection from O(n²) to O(n) through spatial partitioning.

```
┌─────────────────────────────────────────────────────────────────┐
│                    MultiGrid Hierarchy                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    MultiGrid<Cell>                          │ │
│  │  - grids: FnvHashMap<GridIndex, UniverseGrid<Cell>>        │ │
│  │  - grid_width: 128                                          │ │
│  │  - grid_height: 128                                         │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              │ GridIndex::from_pos()            │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                  UniverseGrid<Cell>                         │ │
│  │  - offset: V2i (world position of chunk origin)            │ │
│  │  - width: 128, height: 128                                  │ │
│  │  - grid: Grid<Cell>                                         │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              │ Local coordinates                 │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                      Grid<Cell>                             │ │
│  │  - grid: Vec<GridCell<Cell>>                               │ │
│  │  - version: usize (for O(1) clearing)                      │ │
│  │  - FACTOR: 1 (cell size in grid units)                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              │ Per-cell storage                  │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    GridCell<Cell>                           │ │
│  │  - version: usize                                           │ │
│  │  - value: Vec<GridCellRef<Cell>> (items at this position)  │ │
│  │  - neighbors: Vec<GridCellRef<Cell>> (items in 3x3 area)   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Chapter 1: GridIndex - Chunk Identification

### 1.1 The GridIndex Struct

```rust
#[derive(Hash, Eq, Clone, Copy, Debug, PartialEq)]
pub struct GridIndex {
    pub grid_offset: V2i,  // (chunk_x, chunk_y)
}
```

A `GridIndex` identifies a 128x128 chunk by its chunk coordinates.

### 1.2 Position to GridIndex

```rust
impl GridIndex {
    pub fn from_pos(pos: V2i, width: usize, height: usize) -> GridIndex {
        GridIndex {
            grid_offset: V2i::new(
                pos.x.div_euclid(width as i32),   // Which chunk in X
                pos.y.div_euclid(height as i32),  // Which chunk in Y
            ),
        }
    }
}
```

**Example:**
```
World position: (150, 200)
Chunk size: 128x128

GridIndex::from_pos(V2i::new(150, 200), 128, 128)
  = GridIndex { grid_offset: V2i::new(1, 1) }
```

### 1.3 GridIndex to Position

```rust
impl GridIndex {
    pub fn to_pos(&self, width: usize, height: usize) -> V2i {
        V2i::new(
            self.grid_offset.x * width as i32,
            self.grid_offset.y * height as i32,
        )
    }
}
```

**Example:**
```
GridIndex { grid_offset: V2i::new(1, 1) }
  .to_pos(128, 128)
  = V2i::new(128, 128)  // World position of chunk origin
```

### 1.4 Tests

```rust
#[test]
fn test_grid_index_from_pos() {
    // Position (0,0) to (127,127) are in chunk (0,0)
    assert_eq!(
        GridIndex::from_pos(V2i::new(0, 0), 128, 128),
        GridIndex { grid_offset: V2i::new(0, 0) }
    );
    assert_eq!(
        GridIndex::from_pos(V2i::new(127, 127), 128, 128),
        GridIndex { grid_offset: V2i::new(0, 0) }
    );
    
    // Position (128,128) to (255,255) are in chunk (1,1)
    assert_eq!(
        GridIndex::from_pos(V2i::new(128, 128), 128, 128),
        GridIndex { grid_offset: V2i::new(1, 1) }
    );
    
    // Negative positions work too
    assert_eq!(
        GridIndex::from_pos(V2i::new(-1, -1), 128, 128),
        GridIndex { grid_offset: V2i::new(-1, -1) }
    );
}
```

---

## Chapter 2: MultiGrid - Chunk Management

### 2.1 The MultiGrid Struct

```rust
pub struct MultiGrid<T> {
    grids: FnvHashMap<GridIndex, UniverseGrid<T>>,
    pub grid_width: usize,
    pub grid_height: usize,
}
```

**Design decisions:**
- `FnvHashMap` for fast hashing (faster than standard HashMap)
- Generic over type `T` (works with `Cell` or any type)
- Stores chunk dimensions for index calculations

### 2.2 Creating a MultiGrid

```rust
impl MultiGrid<T> {
    pub fn new(width: usize, height: usize) -> MultiGrid<T> {
        MultiGrid {
            grids: FnvHashMap::default(),
            grid_width: width,
            grid_height: height,
        }
    }
}
```

### 2.3 Lazy Loading with `or_insert_with`

```rust
pub fn or_insert_with(
    &mut self,
    index: GridIndex,
    f: impl Fn() -> UniverseGrid<T>,
) -> (bool, &mut UniverseGrid<T>) {
    let is_new = !self.grids.contains_key(&index);
    let res = self.grids.entry(index).or_insert_with(f);
    (is_new, res)
}
```

**Usage:**
```rust
let (is_new, grid) = multigrid.or_insert_with(
    grid_index,
    || UniverseGrid::new(grid_index, width, height)
);

if is_new {
    // Generate terrain for this new chunk
    generator.generate_pristine_grid(grid, grid_index, width, height);
}
```

### 2.4 Finding Grids to Load/Unload

```rust
impl MultiGrid<T> {
    // Grids around player that need loading
    pub fn get_dropped_grids(&self, center: V2i, drop_radius: usize) -> Vec<GridIndex> {
        let r = drop_radius as i32;
        let center_grid = GridIndex::from_pos(center, self.grid_width, self.grid_height);
        let mut res = Vec::new();
        
        for x in -r..r {
            for y in -r..r {
                let grid_index = GridIndex {
                    grid_offset: V2i::new(
                        center_grid.grid_offset.x + x,
                        center_grid.grid_offset.y + y,
                    ),
                };
                if !self.grids.contains_key(&grid_index) {
                    res.push(grid_index);  // Missing - needs loading
                }
            }
        }
        res
    }
    
    // Grids far from player that can be unloaded
    pub fn get_far_grids(&self, center: V2i, drop_radius: usize) -> Vec<GridIndex> {
        let r = drop_radius as i32;
        let center_grid = GridIndex::from_pos(center, self.grid_width, self.grid_height);
        
        self.grids
            .iter()
            .map(|(grid_index, _)| *grid_index)
            .filter(|grid_index| {
                let dx = grid_index.grid_offset.x - center_grid.grid_offset.x;
                let dy = grid_index.grid_offset.y - center_grid.grid_offset.y;
                dx.abs() > r || dy.abs() > r  // Outside radius
            })
            .collect()
    }
}
```

### 2.5 Cell Position Updates

When a cell moves, it may cross chunk boundaries:

```rust
pub fn update_cell_pos(
    &mut self,
    cell_idx: &GridCellRef<T>,
    old_pos: V2i,
    new_pos: V2i,
) {
    if old_pos != new_pos {
        // Remove from old chunk
        if let Some(old_grid) = self.get_mut(self.pos_to_index(old_pos)) {
            old_grid.remove(old_pos, cell_idx);
        }
        
        // Add to new chunk
        if let Some(new_grid) = self.get_mut(self.pos_to_index(new_pos)) {
            new_grid.put(new_pos, cell_idx.clone());
        }
    }
}
```

---

## Chapter 3: UniverseGrid - Chunk Implementation

### 3.1 UniverseGrid Structure

```rust
pub struct UniverseGrid<T> {
    pub width: usize,
    pub height: usize,
    offset: V2i,       // World position of chunk origin
    grid: Grid<T>,     // Local grid (128x128)
}
```

### 3.2 Creating a UniverseGrid

```rust
impl UniverseGrid<T> {
    pub fn new(grid_index: GridIndex, grid_width: usize, grid_height: usize) -> Self {
        UniverseGrid {
            grid: Grid::new(grid_width, grid_height),
            width: grid_width,
            height: grid_height,
            offset: grid_index.to_pos(grid_width, grid_height),
        }
    }
}
```

### 3.3 Bounds Checking

```rust
pub fn is_in_bounds(&self, pos: V2i) -> bool {
    let relative_pos = pos.minus(self.offset);
    relative_pos.x >= 0
        && relative_pos.y >= 0
        && relative_pos.x < self.width as i32
        && relative_pos.y < self.height as i32
}
```

**Example:**
```
UniverseGrid at offset (128, 128), size 128x128

Position (150, 150):
  relative = (150-128, 150-128) = (22, 22)
  0 <= 22 < 128 ✓ IN BOUNDS

Position (100, 100):
  relative = (100-128, 100-128) = (-28, -28)
  -28 < 0 ✗ OUT OF BOUNDS
```

### 3.4 Local Coordinate Conversion

```rust
pub fn put(&mut self, pos: V2i, cell_idx: GridCellRef<T>) {
    assert!(self.is_in_bounds(pos));
    
    // Convert world position to local grid coordinates
    let rpos = pos.minus(self.offset);
    
    self.grid.put(
        usize::try_from(rpos.x).unwrap(),
        usize::try_from(rpos.y).unwrap(),
        cell_idx,
    )
}
```

### 3.5 Serialization

```rust
impl UniverseGrid<T> 
where
    T: serde::Serialize + Clone,
{
    pub fn to_bytes(&self) -> Result<JsValue, Error> {
        self.grid.to_bytes()
    }
}

impl UniverseGrid<T>
where
    T: serde::de::DeserializeOwned + Clone,
{
    pub fn from_bytes(
        bytes: JsValue,
        grid_index: GridIndex,
        grid_width: usize,
        grid_height: usize,
    ) -> Result<Self, Error> {
        let grid = Grid::from_bytes(bytes)?;
        Ok(UniverseGrid {
            grid,
            width: grid_width,
            height: grid_height,
            offset: grid_index.to_pos(grid_width, grid_height),
        })
    }
}
```

---

## Chapter 4: Grid - Spatial Hashing

### 4.1 Grid Structure

```rust
pub struct Grid<T> {
    width: usize,
    height: usize,
    grid: Vec<GridCell<T>>,
    version: usize,
}
```

### 4.2 Index Calculation

```rust
const FACTOR: usize = 1;  // Cell size in grid units

fn grid_index(x: usize, y: usize, height: usize) -> usize {
    (x / FACTOR) * (height / FACTOR + 2) + (y / FACTOR)
}
```

**Why `+ 2`?** Extra padding for boundary handling.

**Example:**
```
Grid 128x128, FACTOR=1
Position (5, 10):
  grid_index(5, 10, 128) = 5 * (128 + 2) + 10 = 5 * 130 + 10 = 660
```

### 4.3 Creating a Grid

```rust
impl Grid<T> {
    pub fn new(width: usize, height: usize) -> Grid<T> {
        let mut grid: Vec<GridCell<T>> =
            Vec::with_capacity(((width / FACTOR + 2) * (height / FACTOR + 2)) as usize);
        
        for _ in 0..((width / FACTOR + 2) * (height / FACTOR + 2)) {
            grid.push(GridCell::new());
        }
        
        Grid {
            width,
            height,
            grid,
            version: 0,
        }
    }
}
```

### 4.4 Version-Based Clearing

```rust
impl Grid<T> {
    pub fn put(&mut self, x: usize, y: usize, value: GridCellRef<T>) {
        // This increments version, causing cells to clear on first access
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
    
    pub fn get(&self, x: usize, y: usize) -> GetResult<T> {
        self.grid[grid_index(x + 1, y + 1, self.height)].get(self.version)
    }
}

impl GridCell<T> {
    fn ensure_version(&mut self, version: usize) {
        if version != self.version {
            self.version = version;
            self.value.clear();      // Reuse allocated memory
            self.neighbors.clear();
        }
    }
    
    pub fn set_value(&mut self, version: usize, value: GridCellRef<T>) {
        self.ensure_version(version);
        self.value.push(value);
    }
    
    pub fn add_neighbor(&mut self, version: usize, neighbor: GridCellRef<T>) {
        self.ensure_version(version);
        self.neighbors.push(neighbor);
    }
}
```

**How it works:**
1. Each frame, `grid.version` increments
2. When accessing a cell, check if `cell.version == grid.version`
3. If not, clear the cell and update its version
4. This is O(1) per accessed cell, not O(n) for all cells

---

## Chapter 5: Neighbor Pre-Calculation

### 5.1 The Key Insight

When placing a cell at position (x, y), register it as a neighbor in all 9 cells of the 3x3 area centered on (x, y):

```
┌─────┬─────┬─────┐
│  1  │  2  │  3  │
├─────┼─────┼─────┤
│  4  │  X  │  5  │  X = position (x, y)
├─────┼─────┼─────┤       1-8 = cells that get X as neighbor
│  6  │  7  │  8  │
└─────┴─────┴─────┘
```

### 5.2 Implementation

```rust
pub fn put(&mut self, x: usize, y: usize, value: GridCellRef<T>) {
    // Place in primary cell
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

### 5.3 Why This Works

After placing a cell at (x, y), calling `get(x+dx, y+dy).neighbors` for any `dx, dy ∈ {-1, 0, 1}` will include that cell.

**Example:**
```rust
// Place cell at (5, 5)
grid.put(5, 5, cell.clone());

// Get neighbors at (6, 6)
let result = grid.get(6, 6);
assert!(result.neighbors.contains(&cell));  // True!
```

This means collision detection only needs to check `result.neighbors` - all potential collisions are already collected.

---

## Chapter 6: Grid Operations

### 6.1 Put Operation

```rust
pub fn put(&mut self, x: usize, y: usize, value: GridCellRef<T>) {
    assert!(x < self.width);
    assert!(y < self.height);
    
    self.grid[grid_index(x + 1, y + 1, self.height)]
        .set_value(self.version, value.clone());
    
    for px in 0..3 {
        for py in 0..3 {
            self.grid[grid_index(x + px, y + py, self.height)]
                .add_neighbor(self.version, value.clone());
        }
    }
}
```

### 6.2 Get Operation

```rust
pub fn get(&self, x: usize, y: usize) -> GetResult<T> {
    assert!(x < self.width);
    assert!(y < self.height);
    
    self.grid[grid_index(x + 1, y + 1, self.height)].get(self.version)
}
```

### 6.3 Remove Operation

```rust
pub fn remove(&mut self, x: usize, y: usize, value: &GridCellRef<T>) {
    assert!(x < self.width);
    assert!(y < self.height);
    
    // Remove from primary cell
    self.grid[grid_index(x + 1, y + 1, self.height)]
        .remove_value(self.version, value);
    
    // Remove from neighbor lists
    for px in 0..3 {
        for py in 0..3 {
            self.grid[grid_index(x + px, y + py, self.height)]
                .remove_neighbor(self.version, value);
        }
    }
}

impl GridCell<T> {
    pub fn remove_value(&mut self, version: usize, value: &GridCellRef<T>) {
        if version != self.version {
            return;  // Already cleared
        }
        self.value.retain(|x| !Rc::ptr_eq(x, value));
    }
    
    pub fn remove_neighbor(&mut self, version: usize, neighbor: &GridCellRef<T>) {
        if version != self.version {
            return;  // Already cleared
        }
        self.neighbors.retain(|x| !Rc::ptr_eq(x, neighbor));
    }
}
```

---

## Chapter 7: Tests

### 7.1 Basic Grid Test

```rust
#[test]
fn test_grid_one() {
    let mut grid: Grid<char> = Grid::new(1, 1);
    let a = Rc::new(RefCell::new('a'));
    
    // Put item
    grid.put(0, 0, a.clone());
    
    // Check value and neighbors
    let res = grid.get(0, 0);
    assert_eq!(res.neighbors.len(), 1);
    assert_eq!(res.value, &[a.clone()]);
    assert_eq!(res.neighbors, &[a.clone()]);
    
    // Remove item
    grid.remove(0, 0, &a);
    let res = grid.get(0, 0);
    assert_eq!(res.neighbors.len(), 0);
    assert_eq!(res.value, &[]);
}
```

### 7.2 Two Adjacent Items

```rust
#[test]
fn test_grid_two() {
    let mut grid: Grid<char> = Grid::new(2, 1);
    let a = Rc::new(RefCell::new('a'));
    let b = Rc::new(RefCell::new('b'));
    
    grid.put(0, 0, a.clone());
    grid.put(1, 0, b.clone());
    
    // From position (0, 0), neighbors include both a and b
    let res = grid.get(0, 0);
    assert_eq!(res.neighbors.len(), 2);
    assert_eq!(res.value, &[a.clone()]);
    assert_eq!(res.neighbors, &[a.clone(), b.clone()]);
    
    // Remove a
    grid.remove(0, 0, &a);
    let res = grid.get(0, 0);
    assert_eq!(res.neighbors.len(), 1);
    assert_eq!(res.value, &[]);
    assert_eq!(res.neighbors, &[b.clone()]);
}
```

### 7.3 Distant Items Don't Interfere

```rust
#[test]
fn test_grid_two_apart() {
    let mut grid: Grid<char> = Grid::new(6, 2);
    let a = Rc::new(RefCell::new('a'));
    let b = Rc::new(RefCell::new('b'));
    
    grid.put(0, 0, a.clone());
    grid.put(4, 0, b.clone());  // Far away
    
    // From (0, 0), only see a
    let res = grid.get(0, 0);
    assert_eq!(res.neighbors.len(), 1);
    assert_eq!(res.value, &[a.clone()]);
    assert_eq!(res.neighbors, &[a.clone()]);
    
    // From (4, 0), only see b
    let res = grid.get(4, 0);
    assert_eq!(res.neighbors.len(), 1);
    assert_eq!(res.value, &[b.clone()]);
    assert_eq!(res.neighbors, &[b.clone()]);
}
```

---

## Summary

The MultiGrid system achieves O(n) collision detection through:

1. **Chunked storage** - MultiGrid manages 128x128 UniverseGrid chunks
2. **Lazy loading** - Chunks loaded/unloaded based on player position
3. **Spatial hashing** - Grid maps positions to cells
4. **Neighbor pre-calculation** - 3x3 registration for O(1) neighbor lookup
5. **Version-based clearing** - O(1) per accessed cell, not O(n) total

---

## Next Steps

See [02-physics-collision-deep-dive.md](./02-physics-collision-deep-dive.md) for collision detection and response.
