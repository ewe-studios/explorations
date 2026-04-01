---
title: "WASM Integration Deep Dive"
subtitle: "Building and deploying Rockies to the web with wasm-bindgen"
---

# WASM Integration Deep Dive

## Overview

Rockies compiles to WebAssembly for browser deployment. This document covers:
- Build configuration
- wasm-bindgen bindings
- Asset pipeline
- JavaScript interop
- Performance considerations

```
┌─────────────────────────────────────────────────────────────────┐
│                    WASM Build Pipeline                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Rust Source Code                                               │
│  (lib.rs, universe.rs, grid.rs, etc.)                          │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              cargo build --target wasm32-unknown-unknown  │  │
│  └──────────────────────────────────────────────────────────┘  │
│         │                                                        │
│         ▼                                                        │
│  WASM Binary (.wasm)                                            │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              wasm-bindgen --out-dir www/pkg               │  │
│  └──────────────────────────────────────────────────────────┘  │
│         │                                                        │
│         ▼                                                        │
│  Generated Files:                                               │
│  - rockies_bg.wasm (compiled binary)                           │
│  - rockies.js (JavaScript bindings)                            │
│  - rockies.d.ts (TypeScript types)                             │
│         │                                                        │
│         ▼                                                        │
│  Web Bundle (webpack/vite)                                      │
│         │                                                        │
│         ▼                                                        │
│  Browser (Chrome, Firefox, Safari)                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Chapter 1: Cargo Configuration

### 1.1 Library Types

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]
```

**crate-type explanation:**
- `cdylib` - C-compatible dynamic library (needed for WASM)
- `rlib` - Rust library (for testing and terminal mode)

### 1.2 Features

```toml
[features]
default = ["console_error_panic_hook"]
wasm = ["wasm-bindgen", "serde-wasm-bindgen"]
terminal = ["sdl2"]
wasm_js = ["wasm-bindgen", "serde-wasm-bindgen"]

[dependencies]
# Core dependencies (always included)
noise = "0.8"
rand = "0.8"
fnv = "1.0"
serde = { version = "1.0", features = ["derive"] }

# WASM-specific
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.4", optional = true }
console_error_panic_hook = { version = "0.1", optional = true }

# Terminal-specific
sdl2 = { version = "0.35", optional = true, features = ["gfx"] }
```

### 1.3 Build Target

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Build for WASM
cargo build --target wasm32-unknown-unknown --features wasm

# Or use wasm-pack (recommended)
wasm-pack build --target web --out-dir www/pkg
```

---

## Chapter 2: wasm-bindgen Bindings

### 2.1 The Game Struct

```rust
use wasm_bindgen::prelude::*;
use std::collections::HashSet;
use crate::color::Color;
use crate::universe::Universe;

#[wasm_bindgen]
pub struct Game {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
    universe: Universe,
    keys: HashSet<Key>,
    shoot_color: Color,
}
```

### 2.2 Constructor

```rust
#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(width: usize, height: usize) -> Game {
        // Set up panic hook for better error messages
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        
        Game {
            width,
            height,
            pixels: vec![0; width * height],
            universe: Universe::new(width, height),
            keys: HashSet::new(),
            shoot_color: Color::hsv(0.0, 1.0, 1.0),
        }
    }
}
```

### 2.3 Game Loop Methods

```rust
#[wasm_bindgen]
impl Game {
    /// Advance simulation by one frame
    #[wasm_bindgen]
    pub fn tick(&mut self) {
        self.universe.tick();
    }
    
    /// Get pixel buffer for rendering
    #[wasm_bindgen]
    pub fn pixels(&self) -> Clamped<&[u8]> {
        // Convert Vec<u32> to Vec<u8> (RGBA)
        let bytes: Vec<u8> = self.pixels
            .iter()
            .flat_map(|p| {
                vec![
                    ((p >> 16) & 0xFF) as u8,
                    ((p >> 8) & 0xFF) as u8,
                    (p & 0xFF) as u8,
                    0xFF,  // Alpha
                ]
            })
            .collect();
        
        Clamped::new(&bytes)
    }
    
    /// Get simulation statistics
    #[wasm_bindgen]
    pub fn stats(&mut self) -> Stats {
        self.universe.stats()
    }
}
```

### 2.4 Input Handling

```rust
#[wasm_bindgen]
impl Game {
    #[wasm_bindgen]
    pub fn key_down(&mut self, key: &str) {
        match key {
            "a" | "A" | "ArrowLeft" => self.keys.insert(Key::Left),
            "d" | "D" | "ArrowRight" => self.keys.insert(Key::Right),
            "w" | "W" | "ArrowUp" => self.keys.insert(Key::Up),
            "s" | "S" | "ArrowDown" => self.keys.insert(Key::Down),
            " " => self.keys.insert(Key::Space),
            _ => false,
        };
        
        self.process_keys();
    }
    
    #[wasm_bindgen]
    pub fn key_up(&mut self, key: &str) {
        match key {
            "a" | "A" | "ArrowLeft" => self.keys.remove(&Key::Left),
            "d" | "D" | "ArrowRight" => self.keys.remove(&Key::Right),
            "w" | "W" | "ArrowUp" => self.keys.remove(&Key::Up),
            "s" | "S" | "ArrowDown" => self.keys.remove(&Key::Down),
            " " => self.keys.remove(&Key::Space),
            _ => {}
        }
    }
    
    #[wasm_bindgen]
    pub fn click(&mut self, x: usize, y: usize) {
        // Handle mouse click for interaction
        self.universe.click(x, y);
    }
}
```

### 2.5 Stats Export

```rust
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    ticks: usize,
    cells_count: usize,
    collisions_count: usize,
    collision_pairs_tested: usize,
}

#[wasm_bindgen]
impl Stats {
    #[wasm_bindgen]
    pub fn get_and_reset(&mut self) -> Stats {
        let res = self.clone();
        *self = Stats::zero();
        res
    }
    
    #[wasm_bindgen(getter)]
    pub fn ticks(&self) -> usize { self.ticks }
    
    #[wasm_bindgen(getter)]
    pub fn cells_count(&self) -> usize { self.cells_count }
    
    #[wasm_bindgen(getter)]
    pub fn collisions_count(&self) -> usize { self.collisions_count }
    
    #[wasm_bindgen(getter)]
    pub fn collision_pairs_tested(&self) -> usize { self.collision_pairs_tested }
}
```

---

## Chapter 3: Asset Pipeline

### 3.1 Build Script

```rust
// build.rs
use image::GenericImageView;
use itertools::Itertools;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("png.rs");
    
    let mut output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest_path)?;
    
    // Write header
    output_file.write_all(b"use crate::color::Color;\n")?;
    
    // Process all PNGs in pngs/ directory
    let read_dir = fs::read_dir("pngs")?;
    let sorted_entries = read_dir
        .map(|x| x.unwrap())
        .sorted_by(|a, b| a.file_name().cmp(&b.file_name()));
    
    for file in sorted_entries {
        let img = image::open(file.path())?;
        let (width, height) = img.dimensions();
        
        let name = file.file_name();
        let name = name.to_str().unwrap();
        let name = name.replace(".png", "").to_uppercase();
        
        let count = width * height;
        
        // Write constant array
        output_file.write_all(
            format!("const {name}_IMAGE: [Color; {count}] = [").as_bytes()
        )?;
        
        // Write pixel data
        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y);
                output_file.write_all(
                    format!(
                        "Color {{ r: {}, g: {}, b: {} }},",
                        pixel[0], pixel[1], pixel[2]
                    ).as_bytes()
                )?;
            }
        }
        output_file.write_all(b"];\n")?;
        
        // Write public constant
        output_file.write_all(
            format!(
                "pub const {name}: (usize, usize, &[Color]) = \
                 ({width}, {height}, &{name}_IMAGE);\n"
            ).as_bytes()
        )?;
    }
    
    Ok(())
}
```

### 3.2 Generated Code

After build, `OUT_DIR/png.rs` contains:

```rust
use crate::color::Color;

const HAMMY_0_IMAGE: [Color; 256] = [
    Color { r: 0, g: 0, b: 0 },
    Color { r: 255, g: 128, b: 0 },
    // ... more pixels
];

pub const HAMMY_0: (usize, usize, &[Color]) = (16, 16, &HAMMY_0_IMAGE);
pub const HAMMY_1: (usize, usize, &[Color]) = (16, 16, &HAMMY_1_IMAGE);
pub const HAMMY_2: (usize, usize, &[Color]) = (16, 16, &HAMMY_2_IMAGE);
```

### 3.3 Using Assets

```rust
// In universe.rs
use crate::assets;

pub struct Player {
    pub w: usize,
    pub h: usize,
    // ...
}

impl Player {
    fn new(x: usize, y: usize) -> Self {
        let (w, h, _): (usize, usize, &[Color]) = assets::HAMMY_0;
        
        Player {
            w,
            h,
            // ...
        }
    }
    
    pub fn render(&self, pixels: &mut Vec<u32>, ...) {
        let hammies = [assets::HAMMY_0, assets::HAMMY_1, assets::HAMMY_2];
        let (w, h, colors) = hammies[self.frame % 3];
        
        // Render sprite pixels
        for x in 0..w {
            for y in 0..h {
                let c = colors[x + y * w];
                // ... draw pixel
            }
        }
    }
}
```

---

## Chapter 4: JavaScript Integration

### 4.1 HTML Entry Point

```html
<!-- www/index.html -->
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Rockies Physics Simulation</title>
</head>
<body>
    <canvas id="game" width="800" height="600"></canvas>
    <div id="stats"></div>
    <script type="module" src="index.js"></script>
</body>
</html>
```

### 4.2 JavaScript Bootstrap

```javascript
// www/index.js
import init, { Game } from './pkg/rockies.js';

async function main() {
    // Initialize WASM
    await init();
    
    // Create game instance
    const game = new Game(800, 600);
    
    // Get canvas
    const canvas = document.getElementById('game');
    const ctx = canvas.getContext('2d');
    
    // Game loop
    function frame() {
        // Update
        game.tick();
        
        // Render
        const pixels = game.pixels();
        const image_data = new ImageData(
            new Uint8ClampedArray(pixels),
            800,
            600
        );
        ctx.putImageData(image_data, 0, 0);
        
        // Update stats
        const stats = game.stats();
        document.getElementById('stats').textContent = 
            `Cells: ${stats.cells_count}, Collisions: ${stats.collisions_count}`;
        
        requestAnimationFrame(frame);
    }
    
    frame();
    
    // Keyboard input
    document.addEventListener('keydown', (e) => {
        game.key_down(e.key);
    });
    
    document.addEventListener('keyup', (e) => {
        game.key_up(e.key);
    });
    
    // Mouse input
    canvas.addEventListener('click', (e) => {
        const rect = canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        game.click(x, y);
    });
}

main();
```

### 4.3 TypeScript Types (Auto-generated)

```typescript
// pkg/rockies.d.ts
export class Game {
  constructor(width: number, height: number);
  tick(): void;
  pixels(): Clamped<Array<number>>;
  stats(): Stats;
  key_down(key: string): void;
  key_up(key: string): void;
  click(x: number, y: number): void;
}

export class Stats {
  ticks: number;
  cells_count: number;
  collisions_count: number;
  collision_pairs_tested: number;
  get_and_reset(): Stats;
}
```

---

## Chapter 5: Performance Considerations

### 5.1 Memory Layout

```rust
// Good: Contiguous pixel buffer
pub struct Game {
    pixels: Vec<u32>,  // 4 bytes per pixel, contiguous
    // ...
}

// Pixel format: 0x00RRGGBB
fn render(&mut self) {
    for y in 0..self.height {
        for x in 0..self.width {
            let color = self.get_color(x, y);
            self.pixels[y * self.width + x] = color.to_u32();
        }
    }
}
```

### 5.2 Minimizing JS ↔ WASM Crossings

```rust
// BAD: Many crossings
#[wasm_bindgen]
impl Game {
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 { ... }
}

// JavaScript:
for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
        const pixel = game.get_pixel(x, y);  // SLOW!
    }
}

// GOOD: Single crossing
#[wasm_bindgen]
impl Game {
    pub fn pixels(&self) -> Clamped<&[u8]> {
        &self.pixels_bytes  // Return entire buffer
    }
}

// JavaScript:
const pixels = game.pixels();  // One call, copy all
```

### 5.3 Clamped for Typed Arrays

```rust
use wasm_bindgen::Clamped;

#[wasm_bindgen]
impl Game {
    pub fn pixels(&self) -> Clamped<&[u8]> {
        Clamped(self.pixels_bytes.as_slice())
    }
}
```

`Clamped<T>` tells JavaScript to use `Uint8ClampedArray` which clamps values to 0-255.

---

## Chapter 6: Build Script

### 6.1 Shell Script

```bash
#!/bin/bash
# wasm-build.sh

set -e

echo "Building Rockies for WASM..."

# Build with wasm-pack
wasm-pack build --target web --out-dir www/pkg --release

echo "Build complete!"
echo "Serving from www/..."

# Optional: serve locally
# cd www && python3 -m http.server 8080
```

### 6.2 GitHub Actions

```yaml
# .github/workflows/wasm.yml
name: WASM Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v2
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        target: wasm32-unknown-unknown
    
    - name: Install wasm-pack
      run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    
    - name: Build
      run: ./wasm-build.sh
    
    - name: Deploy
      uses: peaceiris/actions-gh-pages@v3
      with:
        github_token: ${{ secrets.GITHUB_TOKEN }}
        publish_dir: ./www
```

---

## Summary

WASM integration for Rockies:

1. **Cargo config** - cdylib + rlib, feature flags
2. **wasm-bindgen** - #[wasm_bindgen] exports
3. **Asset pipeline** - build.rs compiles PNGs to Rust
4. **JavaScript** - init(), Game class, game loop
5. **Performance** - minimize crossings, contiguous buffers

---

## Next Steps

See [04-procedural-generation-deep-dive.md](./04-procedural-generation-deep-dive.md) for terrain generation.
