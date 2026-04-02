---
source: /home/darkvoid/Boxxed/@formulas/src.AppOSS
related_projects: Skia, Penpot, OpenPencil, Rive, tldraw
created_at: 2026-04-02
tags: vector, svg, path, rendering, examples
---

# Vector Graphics Examples

## Overview

This document provides practical examples of vector graphics operations as used in design tools like Figma, Penpot, and OpenPencil.

---

## Example 1: Drawing Basic Shapes

### Rectangle with Rounded Corners

```rust
use kurbo::{Rect, RoundedRect, Shape};

fn create_rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> RoundedRect {
    RoundedRect::new(x, y, x + width, y + height, radius)
}

// Render with Vello
use vello::{peniko::{Brush, Color, Fill}, Scene};

fn render_rounded_rect(scene: &mut Scene, rect: &RoundedRect, color: Color) {
    let brush = Brush::Solid(color);
    scene.fill(Fill::NonZero, &rect.to_kurbo(), &brush, None, None);
}

// Usage
let mut scene = Scene::new();
let rect = create_rounded_rect(10.0, 10.0, 100.0, 50.0, 8.0);
render_rounded_rect(&mut scene, &rect, Color::rgb8(255, 0, 0));
```

### Circle/Ellipse

```rust
use kurbo::{Circle, Ellipse, Shape};

fn create_circle(cx: f32, cy: f32, radius: f32) -> Circle {
    Circle::new((cx, cy), radius)
}

fn create_ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Ellipse {
    Ellipse::new((cx, cy), (rx, ry), 0.0)
}

fn render_circle(scene: &mut Scene, circle: &Circle, color: Color) {
    let brush = Brush::Solid(color);
    scene.fill(Fill::NonZero, &circle.to_kurbo(), &brush, None, None);
}
```

### Line with Stroke

```rust
use kurbo::{Line, Shape, Stroke};

fn create_line(x0: f32, y0: f32, x1: f32, y1: f32) -> Line {
    Line::new((x0, y0), (x1, y1))
}

fn render_line(scene: &mut Scene, line: &Line, color: Color, width: f32) {
    let brush = Brush::Solid(color);
    let stroke = Stroke::new(width);
    scene.stroke(&stroke, &line.to_kurbo(), &brush, None, None);
}
```

---

## Example 2: Custom Paths

### Creating a Star Shape

```rust
use kurbo::{PathEl, BezPath, Point};
use std::f32::consts::PI;

fn create_star(cx: f32, cy: f32, inner_radius: f32, outer_radius: f32, points: u32) -> BezPath {
    let mut path = BezPath::new();
    let mut first = true;
    
    for i in 0..(points * 2) {
        let angle = (i as f32) * PI / (points as f32) - PI / 2.0;
        let radius = if i % 2 == 0 { outer_radius } else { inner_radius };
        
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();
        
        if first {
            path.move_to((x, y));
            first = false;
        } else {
            path.line_to((x, y));
        }
    }
    
    path.close_path();
    path
}

// Usage
let mut scene = Scene::new();
let star = create_star(100.0, 100.0, 30.0, 60.0, 5);
render_path(&mut scene, &star, Color::rgb8(255, 215, 0));
```

### Bezier Curve Path

```rust
fn create_s_curve() -> BezPath {
    let mut path = BezPath::new();
    path.move_to((50.0, 100.0));
    path.curve_to((50.0, 50.0), (150.0, 50.0), (150.0, 100.0));
    path.curve_to((150.0, 150.0), (250.0, 150.0), (250.0, 100.0));
    path
}

fn render_path(scene: &mut Scene, path: &BezPath, color: Color) {
    let brush = Brush::Solid(color);
    scene.fill(Fill::NonZero, path, &brush, None, None);
}
```

---

## Example 3: Gradients

### Linear Gradient

```rust
use vello::peniko::{Brush, Color, Fill, Gradient, GradientKind};

fn create_linear_gradient(start: Point, end: Point, colors: Vec<(Color, f32)>) -> Gradient {
    let mut gradient = Gradient::new(GradientKind::Linear { start, end });
    
    for (color, stop) in colors {
        gradient = gradient.with_stop(stop, color);
    }
    
    gradient
}

fn render_with_gradient(scene: &mut Scene, shape: &impl Shape, gradient: Gradient) {
    let brush = Brush::Gradient(gradient);
    scene.fill(Fill::NonZero, &shape.to_kurbo(), &brush, None, None);
}

// Usage
let rect = Rect::new(0.0, 0.0, 200.0, 100.0);
let gradient = create_linear_gradient(
    Point::new(0.0, 0.0),
    Point::new(200.0, 0.0),
    vec![
        (Color::rgb8(255, 0, 0), 0.0),
        (Color::rgb8(0, 255, 0), 0.5),
        (Color::rgb8(0, 0, 255), 1.0),
    ],
);
render_with_gradient(&mut scene, &rect, gradient);
```

### Radial Gradient

```rust
fn create_radial_gradient(center: Point, radius: f32, colors: Vec<(Color, f32)>) -> Gradient {
    let mut gradient = Gradient::new(GradientKind::Radial { center, radius });
    
    for (color, stop) in colors {
        gradient = gradient.with_stop(stop, color);
    }
    
    gradient
}

// Usage - Sun effect
let circle = Circle::new((150.0, 150.0), 50.0);
let gradient = create_radial_gradient(
    Point::new(150.0, 150.0),
    50.0,
    vec![
        (Color::rgb8(255, 255, 200), 0.0),
        (Color::rgb8(255, 200, 0), 0.5),
        (Color::rgb8(255, 100, 0), 1.0),
    ],
);
```

---

## Example 4: Transforms

### Applying Transformations

```rust
use kurbo::{Affine, Rect, Shape};

fn apply_transform(shape: &impl Shape, transform: Affine) -> BezPath {
    transform * shape.to_kurbo()
}

// Translation
let rect = Rect::new(0.0, 0.0, 50.0, 50.0);
let translated = apply_transform(&rect, Affine::translate((100.0, 100.0)));

// Rotation around center
let rotated = apply_transform(&rect, Affine::rotate_about(
    std::f32::consts::PI / 4.0,
    (25.0, 25.0),
));

// Scale
let scaled = apply_transform(&rect, Affine::scale(2.0));

// Combined transform
let combined = Affine::translate((100.0, 100.0))
    * Affine::rotate(std::f32::consts::PI / 4.0)
    * Affine::scale(1.5);
let transformed = apply_transform(&rect, combined);
```

---

## Example 5: Boolean Operations

### Using lyon for path operations

```rust
use lyon_algorithms::{boolean::*, path::Path};

fn union_shapes(path1: &Path, path2: &Path) -> Path {
    boolean_op(path1, path2, BooleanOp::Union)
}

fn intersect_shapes(path1: &Path, path2: &Path) -> Path {
    boolean_op(path1, path2, BooleanOp::Intersection)
}

fn subtract_shapes(path1: &Path, path2: &Path) -> Path {
    boolean_op(path1, path2, BooleanOp::Difference)
}

fn xor_shapes(path1: &Path, path2: &Path) -> Path {
    boolean_op(path1, path2, BooleanOp::Xor)
}

// Usage - Create a ring
let outer_circle = create_circle_path(100.0, 100.0, 50.0);
let inner_circle = create_circle_path(100.0, 100.0, 30.0);
let ring = subtract_shapes(&outer_circle, &inner_circle);
```

---

## Example 6: Path Effects

### Dashed Stroke

```rust
use kurbo::{Stroke, Shape, BezPath};

fn create_dashed_stroke() -> Stroke {
    Stroke::new(2.0)
        .with_dashes(0.0, [10.0, 5.0].iter().copied())
}

// Usage
let path = create_s_curve();
scene.stroke(&create_dashed_stroke(), &path, &brush, None, None);
```

### Stroke Caps and Joins

```rust
use kurbo::{Cap, Join, Stroke};

fn create_round_stroke(width: f32) -> Stroke {
    Stroke::new(width)
        .with_capt(Cap::Round)
        .with_joints(Join::Round)
}

fn create_square_stroke(width: f32) -> Stroke {
    Stroke::new(width)
        .with_capt(Cap::Square)
        .with_joints(Join::Miter)
}
```

---

## Example 7: Text Rendering

### Using svgtypes for text path

```rust
// Simple text on path (conceptual - full text layout is complex)
fn create_text_along_path(text: &str, path: &BezPath) -> Vec<GlyphPosition> {
    let mut positions = Vec::new();
    let mut offset = 0.0;
    
    for ch in text.chars() {
        // Get glyph width (simplified)
        let glyph_width = 10.0;
        
        // Find position along path
        let point = path.evaluate(offset);
        let tangent = path.tangent(offset);
        let angle = tangent.y.atan2(tangent.x);
        
        positions.push(GlyphPosition {
            ch,
            position: point,
            angle,
        });
        
        offset += glyph_width;
    }
    
    positions
}
```

---

## Example 8: SVG Import/Export

### Parsing SVG

```rust
use usvg::{Tree, Options};

fn parse_svg(svg_data: &str) -> Result<Tree, Error> {
    let opt = Options::default();
    let tree = Tree::from_str(svg_data, &opt)?;
    Ok(tree)
}

// Convert SVG to BezPath
fn svg_to_bez_path(svg: &Tree) -> Vec<BezPath> {
    let mut paths = Vec::new();
    
    fn process_node(node: &usvg::Node, paths: &mut Vec<BezPath>) {
        match node {
            usvg::Node::Path(p) => {
                let bez_path = usvg_path_to_bez(p);
                paths.push(bez_path);
            }
            uscfg::Node::Group(g) => {
                for child in &g.children {
                    process_node(child, paths);
                }
            }
            _ => {}
        }
    }
    
    process_node(svg.root(), &mut paths);
    paths
}
```

### Exporting to SVG

```rust
fn path_to_svg_data(path: &BezPath, width: f32, height: f32) -> String {
    let mut svg = format!(
        r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"#,
        width, height, width, height
    );
    
    svg.push_str("<path d=\"");
    
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => svg.push_str(&format!("M {} {} ", p.x, p.y)),
            PathEl::LineTo(p) => svg.push_str(&format!("L {} {} ", p.x, p.y)),
            PathEl::QuadTo(p1, p2) => svg.push_str(&format!("Q {} {} {} {} ", p1.x, p1.y, p2.x, p2.y)),
            PathEl::CurveTo(p1, p2, p3) => svg.push_str(&format!("C {} {} {} {} {} {} ", p1.x, p1.y, p2.x, p2.y, p3.x, p3.y)),
            PathEl::ClosePath => svg.push_str("Z "),
        }
    }
    
    svg.push_str("\"/>");
    svg.push_str("</svg>");
    svg
}
```

---

## Example 9: Hit Testing

### Point in Path Test

```rust
use kurbo::{Shape, BezPath};

fn point_in_path(point: Point, path: &BezPath) -> bool {
    let path_bbox = path.bounding_box();
    
    // Quick reject
    if !path_bbox.contains(point) {
        return false;
    }
    
    // Ray casting algorithm
    let mut crossings = 0;
    let ray = kurbo::Line::new(point, Point::new(f32::MAX, point.y));
    
    for el in path.elements() {
        if let PathEl::LineTo(p) = el {
            // Check intersection
            // (simplified - full implementation needs more cases)
        }
    }
    
    crossings % 2 == 1
}
```

---

## Example 10: Animation

### Animating a Shape

```rust
use std::time::{Duration, Instant};

struct AnimatedCircle {
    center: Point,
    base_radius: f32,
    amplitude: f32,
    frequency: f32,
    start_time: Instant,
}

impl AnimatedCircle {
    fn new(center: Point, radius: f32) -> Self {
        Self {
            center,
            base_radius: radius,
            amplitude: 10.0,
            frequency: 2.0,
            start_time: Instant::now(),
        }
    }
    
    fn current_radius(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        self.base_radius + self.amplitude * (elapsed * self.frequency).sin()
    }
    
    fn render(&self, scene: &mut Scene) {
        let circle = Circle::new(self.center, self.current_radius());
        let brush = Brush::Solid(Color::rgb8(255, 100, 100));
        scene.fill(Fill::NonZero, &circle.to_kurbo(), &brush, None, None);
    }
}
```

---

## Summary

These examples demonstrate common vector graphics operations:

1. **Basic shapes**: Rectangles, circles, lines
2. **Custom paths**: Stars, bezier curves
3. **Gradients**: Linear and radial
4. **Transforms**: Translate, rotate, scale
5. **Boolean ops**: Union, intersect, subtract
6. **Path effects**: Dashed strokes, caps, joins
7. **Text**: Basic text on path
8. **SVG**: Import and export
9. **Hit testing**: Point in path
10. **Animation**: Time-based rendering

Key crates for vector graphics in Rust:
- `kurbo`: 2D geometry
- `vello`: GPU rendering
- `lyon`: Path operations
- `usvg`: SVG parsing
- `resvg`: SVG rendering
