---
source: /home/darkvoid/Boxxed/@formulas/src.AppOSS
projects: Skia, Penpot, OpenPencil, tldraw, svgo
created_at: 2026-04-02
tags: vector, svg, paths, curves, tessellation, geometry
---

# Vector Graphics and SVG Algorithms

## Overview

This document covers the fundamental algorithms behind vector graphics as used in design tools like Figma, Penpot, and OpenPencil. We explore path representations, curve algorithms, tessellation, and SVG processing.

---

## Part 1: Mathematical Foundations

### 1.1 Points and Vectors

```rust
#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self { Vec2 { x, y } }
    
    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }
    
    pub fn cross(self, other: Vec2) -> f32 {
        self.x * other.y - self.y * other.x
    }
    
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    
    pub fn normalize(self) -> Vec2 {
        let len = self.length();
        Vec2::new(self.x / len, self.y / len)
    }
    
    pub fn lerp(self, other: Vec2, t: f32) -> Vec2 {
        Vec2::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, scalar: f32) -> Vec2 {
        Vec2::new(self.x * scalar, self.y * scalar)
    }
}
```

### 1.2 Matrices for Transformations

```rust
#[derive(Clone, Copy, Debug)]
pub struct Mat3 {
    pub data: [[f32; 3]; 3],
}

impl Mat3 {
    pub fn identity() -> Self {
        Mat3 {
            data: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }
    
    pub fn translate(x: f32, y: f32) -> Self {
        Mat3 {
            data: [
                [1.0, 0.0, x],
                [0.0, 1.0, y],
                [0.0, 0.0, 1.0],
            ],
        }
    }
    
    pub fn scale(sx: f32, sy: f32) -> Self {
        Mat3 {
            data: [
                [sx, 0.0, 0.0],
                [0.0, sy, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }
    
    pub fn rotate(angle_rad: f32) -> Self {
        let c = angle_rad.cos();
        let s = angle_rad.sin();
        Mat3 {
            data: [
                [c, -s, 0.0],
                [s, c, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }
    
    pub fn transform_point(self, p: Vec2) -> Vec2 {
        let x = self.data[0][0] * p.x + self.data[0][1] * p.y + self.data[0][2];
        let y = self.data[1][0] * p.x + self.data[1][1] * p.y + self.data[1][2];
        Vec2::new(x, y)
    }
    
    pub fn multiply(self, other: Mat3) -> Mat3 {
        let mut result = Mat3::identity();
        for i in 0..3 {
            for j in 0..3 {
                result.data[i][j] = self.data[i][0] * other.data[0][j]
                    + self.data[i][1] * other.data[1][j]
                    + self.data[i][2] * other.data[2][j];
            }
        }
        result
    }
}
```

---

## Part 2: Path Representation

### 2.1 Path Commands

```rust
#[derive(Clone, Debug)]
pub enum PathCommand {
    MoveTo { x: f32, y: f32 },
    LineTo { x: f32, y: f32 },
    HorizontalTo { x: f32 },
    VerticalTo { y: f32 },
    QuadTo { cp1x: f32, cp1y: f32, x: f32, y: f32 },
    CubicTo { cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32 },
    ArcTo { rx: f32, ry: f32, x_axis_rotation: f32, 
            large_arc: bool, sweep: bool, x: f32, y: f32 },
    Close,
}

#[derive(Clone, Default, Debug)]
pub struct Path {
    commands: Vec<PathCommand>,
    points: Vec<Vec2>,
    start_points: Vec<Vec2>,
    current_point: Option<Vec2>,
}

impl Path {
    pub fn move_to(&mut self, x: f32, y: f32) {
        let p = Vec2::new(x, y);
        self.commands.push(PathCommand::MoveTo { x, y });
        self.points.push(p);
        self.start_points.push(p);
        self.current_point = Some(p);
    }
    
    pub fn line_to(&mut self, x: f32, y: f32) {
        if self.current_point.is_none() {
            self.move_to(0.0, 0.0);
        }
        let p = Vec2::new(x, y);
        self.commands.push(PathCommand::LineTo { x, y });
        self.points.push(p);
        self.current_point = Some(p);
    }
    
    pub fn close(&mut self) {
        if let Some(start) = self.start_points.last().copied() {
            self.commands.push(PathCommand::Close);
            self.current_point = Some(start);
        }
    }
}
```

### 2.2 SVG Path Parsing

```rust
pub fn parse_svg_path(d: &str) -> Result<Path, ParseError> {
    let mut path = Path::default();
    let mut chars = d.chars().peekable();
    
    while let Some(cmd) = next_command(&mut chars)? {
        match cmd {
            ('M', [x, y]) => path.move_to(x, y),
            ('L', [x, y]) => path.line_to(x, y),
            ('C', [cp1x, cp1y, cp2x, cp2y, x, y]) => {
                path.cubic_to(cp1x, cp1y, cp2x, cp2y, x, y);
            }
            ('Z' | 'z', _) => path.close(),
            _ => {}
        }
    }
    
    Ok(path)
}

fn next_command(chars: &mut std::iter::Peekable<Chars>) -> Result<(char, [f32; 6]), ParseError> {
    // Skip whitespace
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
    
    // Read command
    let cmd = chars.next().ok_or(ParseError::UnexpectedEnd)?;
    
    // Read parameters based on command
    let mut params = [0.0; 6];
    let param_count = match cmd {
        'M' | 'L' => 2,
        'H' => 1,
        'V' => 1,
        'Q' => 4,
        'C' => 6,
        'A' => 7,
        'Z' | 'z' => 0,
        _ => return Err(ParseError::UnknownCommand(cmd)),
    };
    
    for i in 0..param_count {
        params[i] = parse_number(chars)?;
    }
    
    Ok((cmd, params))
}
```

---

## Part 3: Bezier Curves

### 3.1 Quadratic Bezier

```rust
/// Quadratic Bezier curve evaluation
/// B(t) = (1-t)²P₀ + 2(1-t)tP₁ + t²P₂
pub fn quadratic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let t2 = t * t;
    
    p0 * mt2 + p1 * (2.0 * mt * t) + p2 * t2
}

/// Get tangent at parameter t
pub fn quadratic_tangent(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let mt = 1.0 - t;
    let tangent = (p1 - p0) * (2.0 * mt) + (p2 - p1) * (2.0 * t);
    tangent.normalize()
}

/// Split curve at parameter t (De Casteljau algorithm)
pub fn split_quadratic(
    p0: Vec2, p1: Vec2, p2: Vec2,
    t: f32,
) -> ((Vec2, Vec2, Vec2), (Vec2, Vec2, Vec2)) {
    let mt = 1.0 - t;
    
    let q0 = p0.lerp(p1, t);
    let q1 = p1.lerp(p2, t);
    let r0 = q0.lerp(q1, t);
    
    // Left curve: P0, Q0, R0
    // Right curve: R0, Q1, P2
    ((p0, q0, r0), (r0, q1, p2))
}
```

### 3.2 Cubic Bezier

```rust
/// Cubic Bezier curve evaluation
/// B(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃
pub fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    let t2 = t * t;
    let t3 = t2 * t;
    
    p0 * mt3 + p1 * (3.0 * mt2 * t) + p2 * (3.0 * mt * t2) + p3 * t3
}

/// Cubic Bezier tangent
pub fn cubic_tangent(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let t2 = t * t;
    
    let tangent = (p1 - p0) * (3.0 * mt2) 
                + (p2 - p1) * (6.0 * mt * t)
                + (p3 - p2) * (3.0 * t2);
    
    tangent.normalize()
}

/// Split cubic at parameter t
pub fn split_cubic(
    p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2,
    t: f32,
) -> ((Vec2, Vec2, Vec2, Vec2), (Vec2, Vec2, Vec2, Vec2)) {
    let mt = 1.0 - t;
    
    let q0 = p0.lerp(p1, t);
    let q1 = p1.lerp(p2, t);
    let q2 = p2.lerp(p3, t);
    
    let r0 = q0.lerp(q1, t);
    let r1 = q1.lerp(q2, t);
    
    let s0 = r0.lerp(r1, t);
    
    ((p0, q0, r0, s0), (s0, r1, q2, p3))
}
```

### 3.3 Curve Flattening

Convert curves to line segments for rendering:

```rust
/// Flatten a cubic Bezier curve into line segments
pub fn flatten_cubic(
    p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2,
    tolerance: f32,
    output: &mut Vec<Vec2>,
) {
    fn recurse(
        p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2,
        tolerance: f32,
        output: &mut Vec<Vec2>,
        depth: u32,
    ) {
        if depth > 20 {
            output.push(p3);
            return;
        }
        
        // Check if curve is flat enough
        if is_cubic_flat(p0, p1, p2, p3, tolerance) {
            output.push(p3);
            return;
        }
        
        // Subdivide
        let (left, right) = split_cubic(p0, p1, p2, p3, 0.5);
        
        recurse(left.0, left.1, left.2, left.3, tolerance, output, depth + 1);
        recurse(right.0, right.1, right.2, right.3, tolerance, output, depth + 1);
    }
    
    output.push(p0);
    recurse(p0, p1, p2, p3, tolerance, output, 0);
}

/// Check if cubic Bezier is flat enough to approximate as line
fn is_cubic_flat(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, tolerance: f32) -> bool {
    // Distance from control points to line P0-P3
    let line = p3 - p0;
    let line_len_sq = line.dot(line);
    
    if line_len_sq < 1e-6 {
        return true;
    }
    
    let d1 = distance_point_to_line_squared(p1, p0, p3, line_len_sq);
    let d2 = distance_point_to_line_squared(p2, p0, p3, line_len_sq);
    
    d1 <= tolerance * tolerance && d2 <= tolerance * tolerance
}

fn distance_point_to_line_squared(p: Vec2, a: Vec2, b: Vec2, len_sq: f32) -> f32 {
    let t = ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / len_sq;
    let t = t.max(0.0).min(1.0);
    let proj = Vec2::new(
        a.x + t * (b.x - a.x),
        a.y + t * (b.y - a.y),
    );
    let dist = p - proj;
    dist.dot(dist)
}
```

---

## Part 4: Arcs

### 4.1 SVG Arc Parameterization

SVG uses endpoint parameterization:

```
ArcTo rx ry x-axis-rotation large-arc-flag sweep-flag x y
```

### 4.2 Arc to Center Conversion

```rust
/// Convert SVG endpoint arc to center parameterization
pub fn svg_arc_to_center(
    x1: f32, y1: f32,
    rx: f32, ry: f32,
    x_axis_rotation: f32,
    large_arc: bool,
    sweep: bool,
    x2: f32, y2: f32,
) -> Option<ArcCenter> {
    // Step 1: Compute (x1', y1')
    let cos_phi = x_axis_rotation.to_radians().cos();
    let sin_phi = x_axis_rotation.to_radians().sin();
    
    let dx = (x1 - x2) / 2.0;
    let dy = (y1 - y2) / 2.0;
    
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;
    
    // Step 2: Compute (cx', cy')
    let rx_sq = rx * rx;
    let ry_sq = ry * ry;
    let x1p_sq = x1p * x1p;
    let y1p_sq = y1p * y1p;
    
    let lambda = (x1p_sq / rx_sq + y1p_sq / ry_sq).max(0.0);
    
    if lambda > 1.0 {
        // Scale up radii
        let scale = lambda.sqrt();
        return svg_arc_to_center(
            x1, y1, rx * scale, ry * scale,
            x_axis_rotation, large_arc, sweep, x2, y2,
        );
    }
    
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    
    let co = ((rx_sq * ry_sq - rx_sq * y1p_sq - ry_sq * x1p_sq) 
              / (rx_sq * y1p_sq + ry_sq * x1p_sq)).max(0.0).sqrt();
    
    let cxp = sign * co * (rx * y1p / ry);
    let cyp = sign * co * (-ry * x1p / rx);
    
    // Step 3: Compute (cx, cy)
    let cx = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;
    
    // Step 4: Compute theta and delta
    let theta = angle_between(
        Vec2::new((x1p - cxp) / rx, (y1p - cyp) / ry),
        Vec2::new(1.0, 0.0),
    );
    
    let delta = angle_between(
        Vec2::new((x1p - cxp) / rx, (y1p - cyp) / ry),
        Vec2::new((-x1p - cxp) / rx, (-y1p - cyp) / ry),
    );
    
    Some(ArcCenter {
        cx, cy, rx, ry,
        x_axis_rotation,
        start_angle: theta,
        sweep_angle: delta,
    })
}
```

### 4.3 Arc Flattening

```rust
/// Flatten arc to line segments
pub fn flatten_arc(
    cx: f32, cy: f32,
    rx: f32, ry: f32,
    rotation: f32,
    start_angle: f32,
    sweep: f32,
    tolerance: f32,
    output: &mut Vec<Vec2>,
) {
    let cos_rot = rotation.to_radians().cos();
    let sin_rot = rotation.to_radians().sin();
    
    // Determine number of segments
    let sweep_abs = sweep.abs();
    let num_segments = ((sweep_abs / std::f32::consts::PI * 2.0).ceil() as usize).max(1);
    
    for i in 0..=num_segments {
        let t = start_angle + sweep * (i as f32 / num_segments as f32);
        let x = cx + rx * t.cos();
        let y = cy + ry * t.sin();
        
        // Apply rotation
        let rx_rot = cos_rot * x - sin_rot * y;
        let ry_rot = sin_rot * x + cos_rot * y;
        
        output.push(Vec2::new(rx_rot, ry_rot));
    }
}
```

---

## Part 5: Path Operations (Boolean Operations)

### 5.1 Winding Numbers

```rust
/// Determine if point is inside path using winding number
pub fn point_in_path_winding(point: Vec2, path: &Path) -> bool {
    let mut winding = 0;
    
    for i in 0..path.points.len() {
        let p1 = path.points[i];
        let p2 = path.points[(i + 1) % path.points.len()];
        
        if p1.y <= point.y {
            if p2.y > point.y {
                if is_left(p1, p2, point) > 0.0 {
                    winding += 1;
                }
            }
        } else {
            if p2.y <= point.y {
                if is_left(p1, p2, point) < 0.0 {
                    winding -= 1;
                }
            }
        }
    }
    
    winding != 0
}

fn is_left(p1: Vec2, p2: Vec2, p: Vec2) -> f32 {
    (p2.x - p1.x) * (p.y - p1.y) - (p.x - p1.x) * (p2.y - p1.y)
}
```

### 5.2 Path Intersection

```rust
/// Find intersections between two paths
pub fn intersect_paths(path1: &Path, path2: &Path) -> Vec<Intersection> {
    let mut intersections = Vec::new();
    
    for edge1 in path1.edges() {
        for edge2 in path2.edges() {
            if let Some((t, u)) = line_intersect(edge1.p1, edge1.p2, edge2.p1, edge2.p2) {
                if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
                    let point = edge1.p1.lerp(edge1.p2, t);
                    intersections.push(Intersection {
                        point,
                        t,
                        u,
                        edge1,
                        edge2,
                    });
                }
            }
        }
    }
    
    intersections
}

fn line_intersect(
    p1: Vec2, p2: Vec2,
    p3: Vec2, p4: Vec2,
) -> Option<(f32, f32)> {
    let denom = (p1.x - p2.x) * (p3.y - p4.y) - (p1.y - p2.y) * (p3.x - p4.x);
    
    if denom.abs() < 1e-10 {
        return None;
    }
    
    let t = ((p1.x - p3.x) * (p3.y - p4.y) - (p1.y - p3.y) * (p3.x - p4.x)) / denom;
    let u = -((p1.x - p2.x) * (p1.y - p3.y) - (p1.y - p2.y) * (p1.x - p3.x)) / denom;
    
    Some((t, u))
}
```

---

## Part 6: Tessellation

### 6.1 Polygon Triangulation (Ear Clipping)

```rust
/// Triangulate a simple polygon using ear clipping
pub fn triangulate_polygon(vertices: &[Vec2]) -> Vec<[u32; 3]> {
    let mut triangles = Vec::new();
    let mut indices: Vec<usize> = (0..vertices.len()).collect();
    
    while indices.len() > 3 {
        let mut ear_found = false;
        
        for i in 0..indices.len() {
            let prev = indices[(i + indices.len() - 1) % indices.len()];
            let curr = indices[i];
            let next = indices[(i + 1) % indices.len()];
            
            if is_ear(vertices, prev, curr, next, &indices) {
                triangles.push([prev as u32, curr as u32, next as u32]);
                indices.remove(i);
                ear_found = true;
                break;
            }
        }
        
        if !ear_found {
            // Handle non-simple polygons (shouldn't happen with valid input)
            break;
        }
    }
    
    // Final triangle
    if indices.len() == 3 {
        triangles.push([
            indices[0] as u32,
            indices[1] as u32,
            indices[2] as u32,
        ]);
    }
    
    triangles
}

fn is_ear(
    vertices: &[Vec2],
    prev: usize, curr: usize, next: usize,
    remaining: &[usize],
) -> bool {
    let p0 = vertices[prev];
    let p1 = vertices[curr];
    let p2 = vertices[next];
    
    // Check if convex
    if cross_product(p0, p1, p2) <= 0.0 {
        return false;
    }
    
    // Check if any point is inside the triangle
    for &idx in remaining {
        if idx == prev || idx == curr || idx == next {
            continue;
        }
        
        if point_in_triangle(vertices[idx], p0, p1, p2) {
            return false;
        }
    }
    
    true
}

fn cross_product(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    
    !(has_neg && has_pos)
}

fn sign(p1: Vec2, p2: Vec2, p3: Vec2) -> f32 {
    (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
}
```

### 6.2 Path to Triangle Mesh

```rust
pub struct Tessellator {
    tolerance: f32,
}

impl Tessellator {
    pub fn new(tolerance: f32) -> Self {
        Tessellator { tolerance }
    }
    
    pub fn tessellate_path(&self, path: &Path) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // Step 1: Flatten curves
        let flattened = self.flatten_path(path);
        
        // Step 2: Build edge list
        let edges: Vec<Edge> = flattened
            .windows(2)
            .map(|w| Edge { p1: w[0], p2: w[1] })
            .collect();
        
        // Step 3: Find self-intersections
        let intersections = self.find_intersections(&edges);
        
        // Step 4: Build monotone polygons
        let polygons = self.build_monotone_polygons(&flattened, &intersections);
        
        // Step 5: Triangulate
        for polygon in polygons {
            let base_idx = vertices.len() as u32;
            vertices.extend(polygon.vertices);
            
            let triangles = triangulate_polygon(&polygon.vertices);
            for tri in triangles {
                indices.push([
                    base_idx + tri[0],
                    base_idx + tri[1],
                    base_idx + tri[2],
                ]);
            }
        }
        
        Mesh { vertices, indices }
    }
    
    fn flatten_path(&self, path: &Path) -> Vec<Vec2> {
        let mut points = Vec::new();
        
        for cmd in &path.commands {
            match cmd {
                PathCommand::MoveTo { x, y } => {
                    points.push(Vec2::new(*x, *y));
                }
                PathCommand::LineTo { x, y } => {
                    points.push(Vec2::new(*x, *y));
                }
                PathCommand::QuadTo { cp1x, cp1y, x, y } => {
                    let p0 = points.last().copied().unwrap_or_default();
                    let p1 = Vec2::new(*cp1x, *cp1y);
                    let p2 = Vec2::new(*x, *y);
                    flatten_quadratic(p0, p1, p2, self.tolerance, &mut points);
                }
                PathCommand::CubicTo { cp1x, cp1y, cp2x, cp2y, x, y } => {
                    let p0 = points.last().copied().unwrap_or_default();
                    let p1 = Vec2::new(*cp1x, *cp1y);
                    let p2 = Vec2::new(*cp2x, *cp2y);
                    let p3 = Vec2::new(*x, *y);
                    flatten_cubic(p0, p1, p2, p3, self.tolerance, &mut points);
                }
                PathCommand::ArcTo { .. } => {
                    // Convert and flatten arc
                }
                PathCommand::Close => {}
            }
        }
        
        points
    }
}

#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<Vec2>,
    pub indices: Vec<[u32; 3]>,
}
```

---

## Part 7: SVG Processing (SVGO-style)

### 7.1 SVG Optimization

```rust
pub fn optimize_svg(input: &str) -> Result<String, Error> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    
    let mut reader = Reader::from_str(input);
    reader.trim_text(true);
    
    let mut output = String::new();
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                write!(output, "<{}", String::from_utf8_lossy(name));
                
                for attr in e.attributes() {
                    let attr = attr.unwrap();
                    let key = String::from_utf8_lossy(attr.key);
                    
                    // Skip default/redundant attributes
                    if should_skip_attr(&key, &attr.value) {
                        continue;
                    }
                    
                    write!(output, " {}=\"{}\"", key, 
                           String::from_utf8_lossy(&attr.value));
                }
                
                output.push('>');
            }
            Ok(Event::End(ref e)) => {
                write!(output, "</{}>", String::from_utf8_lossy(e.name()));
            }
            Ok(Event::Empty(ref e)) => {
                write!(output, "<{}/>", String::from_utf8_lossy(e.name()));
            }
            Ok(Event::Text(ref e)) => {
                let text = String::from_utf8_lossy(&e).trim();
                if !text.is_empty() {
                    output.push_str(text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(e)),
            _ => {}
        }
        
        buf.clear();
    }
    
    Ok(output)
}

fn should_skip_attr(key: &str, value: &[u8]) -> bool {
    match key {
        "version" if value == b"1.1" => true,
        "xmlns" if value == b"http://www.w3.org/2000/svg" => true,
        "fill" if value == b"black" => true,
        "stroke" if value == b"none" => true,
        _ => false,
    }
}
```

### 7.2 Path Simplification

```rust
/// Simplify path by removing redundant points
pub fn simplify_path(points: &[Vec2], tolerance: f32) -> Vec<Vec2> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    
    // Ramer-Douglas-Peucker algorithm
    rdp_simplify(points, tolerance)
}

fn rdp_simplify(points: &[Vec2], epsilon: f32) -> Vec<Vec2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    
    let dmax = points.iter().skip(1).take(points.len() - 2)
        .enumerate()
        .map(|(i, &p)| (i, perpendicular_distance(p, points[0], points[points.len() - 1])))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap_or((0, 0.0));
    
    if dmax.1 > epsilon {
        let left = rdp_simplify(&points[..=dmax.0], epsilon);
        let right = rdp_simplify(&points[dmax.0..], epsilon);
        
        left.into_iter()
            .chain(right.into_iter().skip(1))
            .collect()
    } else {
        vec![points[0], points[points.len() - 1]]
    }
}

fn perpendicular_distance(point: Vec2, line_start: Vec2, line_end: Vec2) -> f32 {
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    
    if dx == 0.0 && dy == 0.0 {
        return ((point.x - line_start.x).powi(2) + (point.y - line_start.y).powi(2)).sqrt();
    }
    
    ((dy * point.x - dx * point.y + line_end.x * line_start.y - line_end.y * line_start.x).abs()) 
        / (dy.powi(2) + dx.powi(2)).sqrt()
}
```

---

## Summary

This document covered:

1. **Mathematical foundations**: Vectors, matrices, transformations
2. **Path representation**: Commands, SVG parsing
3. **Bezier curves**: Quadratic/cubic evaluation, flattening
4. **Arcs**: SVG arc conversion and flattening
5. **Boolean operations**: Winding numbers, intersections
6. **Tessellation**: Polygon triangulation, mesh generation
7. **SVG optimization**: Path simplification, attribute removal

For Rust implementation:
- `lyon`: Path tessellation library
- `svgtypes`: SVG path parsing
- `kurbo`: 2D geometry library
- `quick-xml`: XML parsing
