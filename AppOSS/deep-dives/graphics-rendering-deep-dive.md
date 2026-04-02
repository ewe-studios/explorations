---
source: /home/darkvoid/Boxxed/@formulas/src.AppOSS
projects: Skia, Penpot, OpenPencil, Rive
created_at: 2026-04-02
tags: graphics, rendering, vector, raster, gpu, webgl, webgpu
---

# Graphics and Rendering Deep Dive

## Overview

This document explores the graphics and rendering systems used in AppOSS projects, focusing on:
- **Skia**: C++ 2D graphics library (Chrome, Android, Flutter)
- **Penpot**: Open-source design tool with WASM rendering
- **OpenPencil**: Figma-compatible editor using CanvasKit
- **Rive**: Interactive animation runtime

We'll cover fundamental algorithms, how they work, and how to replicate them in Rust.

---

## Part 1: Graphics Fundamentals

### 1.1 What is Computer Graphics?

Computer graphics is about generating images programmatically. There are two main approaches:

#### Vector Graphics
- Mathematical descriptions of shapes
- Infinitely scalable
- Used by: SVG, PDF, design tools

```
Circle: center=(100,100), radius=50
Line: from=(0,0), to=(100,100)
Bezier: P0, P1, P2, P3 control points
```

#### Raster Graphics
- Pixel-based images
- Fixed resolution
- Used by: Photos, canvas rendering

```
Bitmap: width x height grid of colors
Each pixel: RGBA values (0-255)
```

### 1.2 Coordinate Systems

```
Screen Coordinates:
  (0,0) ──────────────────► x
    │
    │
    │    (x,y)
    │      ●
    │
    ▼
    y

Y-axis points DOWN in most graphics systems!
```

### 1.3 Transformations

Transformations change how shapes are positioned/scaled:

#### Translation (Moving)
```
new_x = x + dx
new_y = y + dy

Matrix form:
[1 0 dx]   [x]   [x + dx]
[0 1 dy] * [y] = [y + dy]
[0 0 1 ]   [1]   [   1  ]
```

#### Scaling
```
new_x = x * sx
new_y = y * sy

Matrix:
[sx 0  0]
[0  sy 0]
[0  0  1]
```

#### Rotation
```
new_x = x * cos(θ) - y * sin(θ)
new_y = x * sin(θ) + y * cos(θ)

Matrix:
[cos(θ) -sin(θ) 0]
[sin(θ)  cos(θ) 0]
[0       0      1]
```

#### Combined Transform

Transformations combine via matrix multiplication:

```rust
// Rust example with nalgebra
use nalgebra::{Matrix3, Vector2};

fn transform_point(point: Vector2<f32>, transform: Matrix3<f32>) -> Vector2<f32> {
    let homogeneous = Vector2::new(point.x, point.y);
    let result = transform * Vector2::new(homogeneous.x, homogeneous.y, 1.0);
    Vector2::new(result.x / result.z, result.y / result.z)
}
```

---

## Part 2: Path Rendering

### 2.1 Path Representation

A path is a sequence of drawing commands:

```rust
enum PathCommand {
    MoveTo { x: f32, y: f32 },
    LineTo { x: f32, y: f32 },
    QuadTo { cp1x: f32, cp1y: f32, x: f32, y: f32 },
    CubicTo { cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32 },
    ArcTo { rx: f32, ry: f32, rotation: f32, large: bool, sweep: bool, x: f32, y: f32 },
    Close,
}

struct Path {
    commands: Vec<PathCommand>,
    points: Vec<[f32; 2]>,
}
```

### 2.2 Bezier Curves

#### Quadratic Bezier (2 control points)

```
B(t) = (1-t)²P₀ + 2(1-t)tP₁ + t²P₂

where t ∈ [0, 1]
```

```rust
fn quadratic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let one_minus_t = 1.0 - t;
    p0 * one_minus_t * one_minus_t +
    p1 * 2.0 * one_minus_t * t +
    p2 * t * t
}
```

#### Cubic Bezier (4 control points)

```
B(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃
```

```rust
fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    
    p0 * mt3 +
    p1 * 3.0 * mt2 * t +
    p2 * 3.0 * mt * t2 +
    p3 * t3
}
```

### 2.3 Path Flattening (Tessellation)

Convert curves to line segments for rendering:

```rust
fn flatten_curve(
    p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2,
    tolerance: f32,
    output: &mut Vec<Vec2>
) {
    // De Casteljau subdivision
    let mut stack = vec![(p0, p1, p2, p3, 0.0f32, 1.0f32)];
    
    while let Some((p0, p1, p2, p3, t0, t1)) = stack.pop() {
        // Check if curve is flat enough
        if is_flat_enough(p0, p1, p2, p3, tolerance) {
            output.push(evaluate(p0, p1, p2, p3, t1));
        } else {
            // Subdivide
            let (left, right) = subdivide(p0, p1, p2, p3);
            let tm = (t0 + t1) / 2.0;
            stack.push((left.0, left.1, left.2, left.3, t0, tm));
            stack.push((right.0, right.1, right.2, right.3, tm, t1));
        }
    }
}

fn is_flat_enough(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, tolerance: f32) -> bool {
    // Distance from control points to line p0-p3
    let line_len = (p3 - p0).length();
    let d1 = distance_point_to_line(p1, p0, p3);
    let d2 = distance_point_to_line(p2, p0, p3);
    d1 * d1 / line_len < tolerance && d2 * d2 / line_len < tolerance
}
```

---

## Part 3: Rasterization

### 3.1 What is Rasterization?

Rasterization converts vector shapes to pixels:

```
Vector Path          Tessellation         Rasterization
    ●────●              ●──●──●              ████████
   /      \            /      \             ██      ██
  ●        ●    →     ●        ●     →     ██      ██
   \      /            \      /             ██      ██
    ●────●              ●──●──●              ████████
```

### 3.2 Scanline Rasterization

Fill polygons by scanning horizontal lines:

```rust
struct Edge {
    x: f32,
    y_min: i32,
    y_max: i32,
    dx_dy: f32, // 1/slope
}

fn rasterize_polygon(vertices: &[Vec2], framebuffer: &mut Framebuffer, color: Color) {
    let mut edges: Vec<Edge> = Vec::new();
    
    // Build edge table
    for i in 0..vertices.len() {
        let v1 = vertices[i];
        let v2 = vertices[(i + 1) % vertices.len()];
        
        let (top, bot) = if v1.y < v2.y { (v1, v2) } else { (v2, v1) };
        
        if top.y != bot.y {
            edges.push(Edge {
                x: top.x,
                y_min: top.y.ceil() as i32,
                y_max: bot.y.ceil() as i32,
                dx_dy: (bot.x - top.x) / (bot.y - top.y),
            });
        }
    }
    
    // Sort edges by y_min
    edges.sort_by_key(|e| e.y_min);
    
    // Process scanlines
    let mut active_edges: Vec<Edge> = Vec::new();
    let mut edge_idx = 0;
    
    for y in framebuffer.y_range() {
        // Add new edges
        while edge_idx < edges.len() && edges[edge_idx].y_min <= y {
            active_edges.push(edges[edge_idx]);
            edge_idx += 1;
        }
        
        // Remove finished edges
        active_edges.retain(|e| e.y_max > y);
        
        // Sort by x
        active_edges.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
        
        // Fill spans
        for i in (0..active_edges.len()).step_by(2) {
            let x_start = active_edges[i].x.ceil() as i32;
            let x_end = active_edges[i + 1].x.ceil() as i32;
            
            for x in x_start..x_end {
                framebuffer.set_pixel(x, y, color);
            }
        }
        
        // Update x for next scanline
        for edge in &mut active_edges {
            edge.x += edge.dx_dy;
        }
    }
}
```

### 3.3 Triangle Rasterization

GPU-friendly triangle filling:

```rust
fn rasterize_triangle(
    v0: Vec2, v1: Vec2, v2: Vec2,
    framebuffer: &mut Framebuffer,
    color: Color
) {
    // Bounding box
    let min_x = v0.x.min(v1.x).min(v2.x) as i32;
    let max_x = v0.x.max(v1.x).max(v2.x) as i32;
    let min_y = v0.y.min(v1.y).min(v2.y) as i32;
    let max_y = v0.y.max(v1.y).max(v2.y) as i32;
    
    // Edge function (cross product)
    fn edge(x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        (x1 - x0) * (y2 - y0) - (y1 - y0) * (x2 - x0)
    }
    
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            
            // Inside test (all edges same sign)
            let e0 = edge(v0.x, v0.y, v1.x, v1.y, px, py);
            let e1 = edge(v1.x, v1.y, v2.x, v2.y, px, py);
            let e2 = edge(v2.x, v2.y, v0.x, v0.y, px, py);
            
            if (e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0) ||
               (e0 <= 0.0 && e1 <= 0.0 && e2 <= 0.0) {
                framebuffer.set_pixel(x, y, color);
            }
        }
    }
}
```

### 3.4 Anti-Aliasing

#### MSAA (Multisample Anti-Aliasing)

Sample multiple points per pixel:

```rust
const SAMPLES: [(f32, f32); 4] = [
    (0.25, 0.25),
    (0.75, 0.25),
    (0.25, 0.75),
    (0.75, 0.75),
];

fn msaa_rasterize(
    v0: Vec2, v1: Vec2, v2: Vec2,
    framebuffer: &mut Framebuffer,
    color: Color
) {
    // Same bounding box...
    
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let mut samples_inside = 0;
            
            for (dx, dy) in SAMPLES {
                let px = x as f32 + dx;
                let py = y as f32 + dy;
                
                if point_in_triangle(px, py, v0, v1, v2) {
                    samples_inside += 1;
                }
            }
            
            // Blend based on coverage
            let coverage = samples_inside as f32 / SAMPLES.len() as f32;
            let blended = color * coverage + framebuffer.get_pixel(x, y) * (1.0 - coverage);
            framebuffer.set_pixel(x, y, blended);
        }
    }
}
```

#### FXAA (Fast Approximate Anti-Aliasing)

Post-process edge smoothing:

```rust
fn fxaa_pixel(x: i32, y: i32, framebuffer: &Framebuffer) -> Color {
    // Sample neighborhood
    let center = framebuffer.get_pixel(x, y);
    let left = framebuffer.get_pixel(x - 1, y);
    let right = framebuffer.get_pixel(x + 1, y);
    let top = framebuffer.get_pixel(x, y - 1);
    let bottom = framebuffer.get_pixel(x, y + 1);
    
    // Calculate contrast
    let max_luma = center.luma().max(left.luma()).max(right.luma())
        .max(top.luma()).max(bottom.luma());
    let min_luma = center.luma().min(left.luma()).min(right.luma())
        .min(top.luma()).min(bottom.luma());
    
    let contrast = max_luma - min_luma;
    
    if contrast > THRESHOLD {
        // Edge detected - blend
        let avg = (left + right + top + bottom) / 4.0;
        return center * 0.5 + avg * 0.5;
    }
    
    center
}
```

---

## Part 4: GPU Rendering

### 4.1 Graphics Pipeline

```
┌─────────────────────────────────────────────────────────┐
│  1. Vertex Shader (per-vertex)                          │
│     - Transform vertices                                │
│     - Calculate positions                               │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  2. Primitive Assembly                                  │
│     - Form triangles from vertices                      │
│     - Clipping                                          │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  3. Rasterization                                       │
│     - Triangles → fragments (pixels)                    │
│     - Interpolation                                     │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  4. Fragment Shader (per-pixel)                         │
│     - Calculate color                                   │
│     - Apply textures, lighting                          │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│  5. Output Merger                                       │
│     - Blending, depth test                              │
│     - Write to framebuffer                              │
└─────────────────────────────────────────────────────────┘
```

### 4.2 WebGL Example

```javascript
// Vertex shader
const vsSource = `
    attribute vec4 aPosition;
    attribute vec4 aColor;
    varying lowp vec4 vColor;
    
    void main() {
        gl_Position = aPosition;
        vColor = aColor;
    }
`;

// Fragment shader
const fsSource = `
    varying lowp vec4 vColor;
    
    void main() {
        gl_FragColor = vColor;
    }
`;

// Initialize WebGL
const gl = canvas.getContext('webgl');

// Compile shader
function createShader(gl, type, source) {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    return shader;
}

// Create program
const vs = createShader(gl, gl.VERTEX_SHADER, vsSource);
const fs = createShader(gl, gl.FRAGMENT_SHADER, fsSource);
const program = gl.createProgram();
gl.attachShader(program, vs);
gl.attachShader(program, fs);
gl.linkProgram(program);

// Triangle vertices
const vertices = new Float32Array([
    // Position      Color
     0.0,  0.5,      1.0, 0.0, 0.0,
    -0.5, -0.5,      0.0, 1.0, 0.0,
     0.5, -0.5,      0.0, 0.0, 1.0,
]);

// Buffer setup
const buffer = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);

// Attribute pointers
const positionLoc = gl.getAttribLocation(program, 'aPosition');
const colorLoc = gl.getAttribLocation(program, 'aColor');

gl.enableVertexAttribArray(positionLoc);
gl.vertexAttribPointer(positionLoc, 2, gl.FLOAT, false, 20, 0);

gl.enableVertexAttribArray(colorLoc);
gl.vertexAttribPointer(colorLoc, 3, gl.FLOAT, false, 20, 8);

// Draw
gl.useProgram(program);
gl.drawArrays(gl.TRIANGLES, 0, 3);
```

### 4.3 WebGPU (Modern GPU API)

```rust
// wgpu (Rust WebGPU) example
use wgpu::*;

async fn render(
    device: &Device,
    queue: &Queue,
    surface: &Surface,
) {
    // Pipeline setup
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shader.wgsl"))),
    });
    
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[VertexBufferLayout {
                array_stride: 8,
                attributes: &[
                    VertexAttribute::new(Float32x2, 0..8),
                ],
            }],
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[ColorTargetState {
                format: TextureFormat::Bgra8UnormSrgb,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            }],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    });
    
    // Render
    let frame = surface.get_current_texture().unwrap();
    let view = frame.texture.create_view(&TextureViewDescriptor::default());
    
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });
    
    {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations { load: LoadOp::Clear(Color::BLUE), store: true },
            }],
            depth_stencil_attachment: None,
        });
        
        render_pass.set_pipeline(&pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
    }
    
    queue.submit(Some(encoder.finish()));
    frame.present();
}
```

---

## Part 5: Text Rendering

### 5.1 Font Basics

Fonts contain:
- **Glyphs**: Shape definitions for characters
- **Metrics**: Spacing, ascent, descent information
- **Kerning**: Pair-specific spacing adjustments

### 5.2 Font Loading

```rust
use rusttype::{Font, Scale};

// Load font
let font_data = include_bytes!("fonts/Roboto-Regular.ttf");
let font = Font::try_from_bytes(font_data).expect("invalid font");

// Scale font
let scale = Scale::uniform(24.0);

// Get glyph
let glyph_id = font.glyph_id('A').expect("glyph not found");
let scaled_glyph = font.glyph(glyph_id).scaled(scale);
```

### 5.3 Text Layout

```rust
use ab_glyph::{Font, FontRef, ScaleFont};

fn layout_text(
    font: &impl Font,
    text: &str,
    scale: f32,
) -> Vec<PositionedGlyph> {
    let scaled_font = font.clone().into_scaled(Scale::uniform(scale));
    
    let mut glyphs = Vec::new();
    let mut caret = point(0.0, scale);
    
    for c in text.chars() {
        if c == '\n' {
            caret.x = 0.0;
            caret.y += scaled_font.line_gap() + scaled_font.ascent() - scaled_font.descent();
            continue;
        }
        
        let mut glyph = scaled_font.scaled_glyph(c);
        glyph.position = caret;
        
        caret.x += scaled_font.h_advance(glyph.id);
        glyphs.push(glyph);
    }
    
    glyphs
}
```

### 5.4 Skia Text Rendering

```cpp
// C++ Skia
SkFont font(typeface, 24.0);
font.setSubpixel(true);
font.setEdging(SkFont::Edging::kAntiAlias);

SkPaint paint;
paint.setColor(SK_ColorBLACK);
paint.setStyle(SkPaint::kFill_Style);

// Draw text
canvas->drawString("Hello, World!", 100, 100, font, paint);

// Draw text on path
SkPath path;
path.moveTo(50, 50);
path.quadTo(150, 10, 250, 50);

SkPathMeasure measurer(path, false);
canvas->drawTextOnPath("Curved Text", path, 0, 0, font, paint);
```

---

## Part 6: Image Processing

### 6.1 Basic Operations

```rust
// Image scaling (bilinear interpolation)
fn scale_bilinear(src: &Image, dst_w: u32, dst_h: u32) -> Image {
    let mut dst = Image::new(dst_w, dst_h);
    
    let scale_x = src.width as f32 / dst_w as f32;
    let scale_y = src.height as f32 / dst_h as f32;
    
    for y in 0..dst_h {
        for x in 0..dst_w {
            let src_x = (x as f32) * scale_x;
            let src_y = (y as f32) * scale_y;
            
            let x0 = src_x.floor() as u32;
            let y0 = src_y.floor() as u32;
            let x1 = (x0 + 1).min(src.width - 1);
            let y1 = (y0 + 1).min(src.height - 1);
            
            let fx = src_x - x0 as f32;
            let fy = src_y - y0 as f32;
            
            // Bilinear blend
            let c00 = src.get_pixel(x0, y0);
            let c01 = src.get_pixel(x0, y1);
            let c10 = src.get_pixel(x1, y0);
            let c11 = src.get_pixel(x1, y1);
            
            let color = c00 * (1.0 - fx) * (1.0 - fy) +
                       c01 * (1.0 - fx) * fy +
                       c10 * fx * (1.0 - fy) +
                       c11 * fx * fy;
            
            dst.set_pixel(x, y, color);
        }
    }
    
    dst
}
```

### 6.2 Blur (Gaussian)

```rust
fn gaussian_blur(image: &Image, sigma: f32) -> Image {
    // Create kernel
    let kernel_size = (sigma * 6.0).ceil() as usize | 1; // Odd
    let kernel = gaussian_kernel(kernel_size, sigma);
    
    // Horizontal pass
    let mut temp = Image::new(image.width, image.height);
    for y in 0..image.height {
        for x in 0..image.width {
            temp.set_pixel(x, y, convolve_row(image, x, y, &kernel));
        }
    }
    
    // Vertical pass
    let mut result = Image::new(image.width, image.height);
    for y in 0..image.height {
        for x in 0..image.width {
            result.set_pixel(x, y, convolve_col(&temp, x, y, &kernel));
        }
    }
    
    result
}

fn gaussian_kernel(size: usize, sigma: f32) -> Vec<f32> {
    let center = size / 2;
    let mut kernel = Vec::with_capacity(size);
    let mut sum = 0.0;
    
    for i in 0..size {
        let x = (i as i32 - center as i32) as f32;
        let weight = (-x * x / (2.0 * sigma * sigma)).exp();
        kernel.push(weight);
        sum += weight;
    }
    
    // Normalize
    for w in &mut kernel {
        *w /= sum;
    }
    
    kernel
}
```

### 6.3 Blending Modes

```rust
enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    HardLight,
}

fn blend(src: Color, dst: Color, mode: BlendMode) -> Color {
    match mode {
        BlendMode::Normal => src,
        
        BlendMode::Multiply => Color {
            r: src.r * dst.r,
            g: src.g * dst.g,
            b: src.b * dst.b,
            a: src.a + dst.a * (1.0 - src.a),
        },
        
        BlendMode::Screen => Color {
            r: 1.0 - (1.0 - src.r) * (1.0 - dst.r),
            g: 1.0 - (1.0 - src.g) * (1.0 - dst.g),
            b: 1.0 - (1.0 - src.b) * (1.0 - dst.b),
            a: src.a + dst.a * (1.0 - src.a),
        },
        
        BlendMode::Overlay => {
            if dst.r < 0.5 {
                Color { r: 2.0 * src.r * dst.r, .. }
            } else {
                Color { r: 1.0 - 2.0 * (1.0 - src.r) * (1.0 - dst.r), .. }
            }
        }
    }
}
```

---

## Part 7: Animation

### 7.1 Interpolation

```rust
// Linear interpolation
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// Cubic bezier easing
fn ease_bezier(t: f32, cp1: f32, cp2: f32) -> f32 {
    let t2 = t * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    
    3.0 * mt2 * t * cp1 + 3.0 * mt * t2 * cp2 + t * t * t
}

// Common easings
fn ease_in_quad(t: f32) -> f32 { t * t }
fn ease_out_quad(t: f32) { t * (2.0 - t) }
fn ease_in_out_quad(t: f32) {
    if t < 0.5 { 2.0 * t * t } else { -1.0 + (4.0 - 2.0 * t) * t }
}
```

### 7.2 Keyframe Animation

```rust
struct Keyframe<T> {
    time: f32,
    value: T,
}

fn interpolate_keyframes<T>(
    keyframes: &[Keyframe<T>],
    time: f32,
) -> T
where
    T: Copy + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    if keyframes.is_empty() {
        panic!("No keyframes");
    }
    
    if time <= keyframes[0].time {
        return keyframes[0].value;
    }
    
    if time >= keyframes.last().unwrap().time {
        return keyframes.last().unwrap().value;
    }
    
    // Find surrounding keyframes
    let mut i = 0;
    while i < keyframes.len() - 1 && keyframes[i + 1].time <= time {
        i += 1;
    }
    
    let k0 = &keyframes[i];
    let k1 = &keyframes[i + 1];
    
    // Normalize time
    let t = (time - k0.time) / (k1.time - k0.time);
    
    // Lerp
    lerp_value(k0.value, k1.value, t)
}
```

---

## Part 8: Performance Optimization

### 8.1 Tiled Rendering

Divide screen into tiles for better cache utilization:

```rust
const TILE_SIZE: u32 = 16;

fn render_tiled(scene: &Scene, framebuffer: &mut Framebuffer) {
    for tile_y in (0..framebuffer.height).step_by(TILE_SIZE as usize) {
        for tile_x in (0..framebuffer.width).step_by(TILE_SIZE as usize) {
            render_tile(
                scene,
                framebuffer,
                tile_x,
                tile_y,
                TILE_SIZE.min(framebuffer.width - tile_x),
                TILE_SIZE.min(framebuffer.height - tile_y),
            );
        }
    }
}
```

### 8.2 Batching

Group similar draw calls:

```rust
struct DrawCall {
    shape: ShapeType,
    fill: FillStyle,
    path: Path,
}

fn batch_draw_calls(draw_calls: &[DrawCall]) -> Vec<BatchedDraw> {
    let mut batches: HashMap<(ShapeType, FillStyle), Vec<Path>> = HashMap::new();
    
    for call in draw_calls {
        batches
            .entry((call.shape, call.fill))
            .or_insert_with(Vec::new)
            .push(call.path.clone());
    }
    
    batches
        .into_iter()
        .map(|((shape, fill), paths)| BatchedDraw { shape, fill, paths })
        .collect()
}
```

### 8.3 Dirty Rectangles

Only redraw changed areas:

```rust
fn render_with_dirty_rects(
    scene: &Scene,
    prev_scene: &Scene,
    framebuffer: &mut Framebuffer,
) {
    let dirty_regions = calculate_dirty_regions(scene, prev_scene);
    
    for region in dirty_regions {
        // Save area around dirty region
        let backup = save_region(framebuffer, &region);
        
        // Render only this region
        render_scene_clipped(scene, framebuffer, &region);
        
        // Restore backup (for transparency)
        restore_region(framebuffer, &region, &backup);
    }
}
```

---

## Part 9: File Formats

### 9.1 SVG

```xml
<svg width="200" height="200" viewBox="0 0 200 200">
  <defs>
    <linearGradient id="grad1">
      <stop offset="0%" style="stop-color:red"/>
      <stop offset="100%" style="stop-color:blue"/>
    </linearGradient>
  </defs>
  
  <rect x="10" y="10" width="80" height="80" fill="url(#grad1)"/>
  <circle cx="150" cy="50" r="40" fill="green" opacity="0.5"/>
  <path d="M 10 150 Q 100 100 190 150" stroke="black" fill="none"/>
</svg>
```

### 9.2 PNG

PNG structure:
- Signature (8 bytes)
- Chunks (length, type, data, CRC)

```rust
struct PngChunk {
    length: u32,
    chunk_type: [u8; 4],
    data: Vec<u8>,
    crc: u32,
}

fn parse_png(data: &[u8]) -> Result<Image> {
    // Check signature
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    assert_eq!(&data[0..8], &PNG_SIGNATURE);
    
    let mut pos = 8;
    while pos < data.len() {
        let length = u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]);
        let chunk_type = [data[pos+4], data[pos+5], data[pos+6], data[pos+7]];
        let chunk_data = &data[pos+8..pos+8+length as usize];
        
        match &chunk_type {
            b"IHDR" => { /* Header */ }
            b"IDAT" => { /* Image data (compressed) */ }
            b"IEND" => { break; }
            _ => {}
        }
        
        pos += 12 + length as usize;
    }
    
    Ok(Image { /* ... */ })
}
```

---

## Summary

This deep dive covered:

1. **Fundamentals**: Coordinate systems, transformations, Bezier curves
2. **Path Rendering**: Flattening, tessellation
3. **Rasterization**: Scanline, triangle, anti-aliasing
4. **GPU Rendering**: Pipeline, WebGL, WebGPU
5. **Text**: Font loading, layout, rendering
6. **Image Processing**: Scaling, blur, blending
7. **Animation**: Interpolation, keyframes
8. **Optimization**: Tiling, batching, dirty rectangles
9. **Formats**: SVG, PNG

For Rust implementation, key crates include:
- `skia-safe`: Skia bindings
- `rusttype`/`ab_glyph`: Font rendering
- `wgpu`: WebGPU
- `image`: Image processing
- `vello`: GPU rendering
