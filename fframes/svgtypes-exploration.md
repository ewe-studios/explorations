---
name: SVGTYPES
description: Low-level SVG type definitions and parsing primitives for Rust SVG processing
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/svgtypes/
---

# SVGTYPES - SVG Type Definitions

## Overview

SVGTYPES is a **low-level crate providing fundamental SVG type definitions and parsers**. It offers reusable types for SVG values like colors, lengths, coordinates, transforms, and path data that are used by higher-level SVG libraries like resvg, usvg, and resvg.

Key features:
- **SVG value types** - Color, Length, Coordinate, Transform
- **Path data parsing** - SVG path command parsing
- **Color parsing** - CSS/SVG color format support
- **Length parsing** - SVG length units (px, em, %, etc.)
- **Transform parsing** - SVG transform syntax
- **No dependencies** - Minimal, pure Rust implementation

## Directory Structure

```
svgtypes/
├── src/
│   ├── lib.rs              # Main module
│   ├── color.rs            # Color types and parsing
│   ├── length.rs           # Length type with units
│   ├── coordinate.rs       # Coordinate system types
│   ├── transform.rs        # Transform matrix
│   ├── path.rs             # Path data parsing
│   ├── rect.rs             # Rectangle types
│   ├── size.rs             # Size dimensions
│   └── aspect_ratio.rs     # Aspect ratio handling
├── benches/                # Benchmarks
├── fuzz/                   # Fuzzing tests
├── Cargo.toml
├── CHANGELOG.md
└── README.md
```

## Core Types

### Color

```rust
use svgtypes::Color;

// Create colors
let color = Color::new(255, 0, 0, 255);  // RGBA
let red = Color::new_rgb(255, 0, 0);
let transparent = Color::new_rgba(255, 0, 0, 128);

// Parse from string
let color = Color::parse("#ff0000").unwrap();
let color = Color::parse("rgb(255, 0, 0)").unwrap();
let color = Color::parse("rgba(255, 0, 0, 0.5)").unwrap();
let color = Color::parse("red").unwrap();  // Named colors
let color = Color::parse("transparent").unwrap();

// Named color constants
let black = Color::BLACK;
let white = Color::WHITE;
let red = Color::RED;
let green = Color::GREEN;
let blue = Color::BLUE;

// Operations
let color = Color::new(100, 100, 100, 255);
let darker = color.darken(0.2);  // 20% darker
let lighter = color.lighten(0.2); // 20% lighter
```

### Length

```rust
use svgtypes::Length;

// Create lengths
let px = Length::new(100.0, LengthUnit::Pixels);
let em = Length::new(1.5, LengthUnit::Em);
let percent = Length::new(50.0, LengthUnit::Percent);
let vw = Length::new(10.0, LengthUnit::ViewBoxWidth);
let vh = Length::new(20.0, LengthUnit::ViewBoxHeight);

// Parse from string
let length = Length::parse("100px").unwrap();
let length = Length::parse("1.5em").unwrap();
let length = Length::parse("50%").unwrap();
let length = Length::parse("10vw").unwrap();

// Convert to pixels (requires context)
let length = Length::new(50.0, LengthUnit::Percent);
let px_value = length.to_pixels(
    100.0,  // viewport size
    16.0,   // font size for em
);

// Operations
let a = Length::new(100.0, LengthUnit::Pixels);
let b = Length::new(50.0, LengthUnit::Pixels);
let sum = a + b;
```

### Transform

```rust
use svgtypes::Transform;

// Identity transform
let identity = Transform::identity();

// Create transforms
let translate = Transform::new_translate(100.0, 50.0);
let scale = Transform::new_scale(2.0, 2.0);
let rotate = Transform::new_rotate(45.0);  // degrees
let rotate_at = Transform::new_rotate_at(45.0, 100.0, 100.0);
let skew_x = Transform::new_skew_x(30.0);
let skew_y = Transform::new_skew_y(30.0);

// Matrix representation
let matrix = Transform {
    a: 1.0, b: 0.0,
    c: 0.0, d: 1.0,
    e: 0.0, f: 0.0,
};

// Combine transforms
let combined = translate.post_concat(scale);
let combined = rotate.pre_concat(translate);

// Inverse
if let Some(inverse) = transform.inverse() {
    // Use inverse
}

// Transform points
let point = transform.transform_point(100.0, 50.0);
```

### Path Data

```rust
use svgtypes::{PathData, PathSegment, PathParser};

// Parse path data
let path_str = "M 10 10 L 100 100 L 100 10 Z";
let path_data = PathData::parse(path_str).unwrap();

// Iterate over segments
for segment in path_data.iter() {
    match segment {
        PathSegment::MoveTo { abs, x, y } => {
            println!("Move to ({}, {})", x, y);
        }
        PathSegment::LineTo { abs, x, y } => {
            println!("Line to ({}, {})", x, y);
        }
        PathSegment::CubicTo { abs, x1, y1, x2, y2, x, y } => {
            println!("Cubic to ({}, {}) via ({}, {}) and ({}, {})", x, y, x1, y1, x2, y2);
        }
        PathSegment::QuadTo { abs, x1, y1, x, y } => {
            println!("Quad to ({}, {}) via ({}, {})", x, y, x1, y1);
        }
        PathSegment::ArcTo { abs, rx, ry, x_axis_rotation, large_arc, sweep, x, y } => {
            println!("Arc to ({}, {})", x, y);
        }
        PathSegment::ClosePath { abs } => {
            println!("Close path");
        }
        _ => {}
    }
}

// Build path programmatically
let mut path_data = PathData::new();
path_data.push_move_to(10.0, 10.0, true);
path_data.push_line_to(100.0, 100.0, true);
path_data.push_line_to(100.0, 10.0, true);
path_data.push_close_path(true);
```

### Path Segment Types

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathSegment {
    /// Move to point (M/m)
    MoveTo { abs: bool, x: f64, y: f64 },

    /// Line to point (L/l)
    LineTo { abs: bool, x: f64, y: f64 },

    /// Line to horizontal (H/h)
    LineToHorizontal { abs: bool, x: f64 },

    /// Line to vertical (V/v)
    LineToVertical { abs: bool, y: f64 },

    /// Cubic Bezier curve (C/c)
    CubicTo {
        abs: bool,
        x1: f64, y1: f64,
        x2: f64, y2: f64,
        x: f64, y: f64,
    },

    /// Smooth cubic Bezier (S/s)
    SmoothCubicTo {
        abs: bool,
        x2: f64, y2: f64,
        x: f64, y: f64,
    },

    /// Quadratic Bezier curve (Q/q)
    QuadTo {
        abs: bool,
        x1: f64, y1: f64,
        x: f64, y: f64,
    },

    /// Smooth quadratic Bezier (T/t)
    SmoothQuadTo {
        abs: bool,
        x: f64, y: f64,
    },

    /// Elliptical arc (A/a)
    ArcTo {
        abs: bool,
        rx: f64, ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        x: f64, y: f64,
    },

    /// Close path (Z/z)
    ClosePath { abs: bool },
}
```

### Rectangle

```rust
use svgtypes::Rect;

// Create rect
let rect = Rect::new(10.0, 10.0, 100.0, 100.0);  // x, y, width, height
let rect = Rect::from_ltrb(10.0, 10.0, 110.0, 110.0);  // left, top, right, bottom

// Properties
let x = rect.x();
let y = rect.y();
let width = rect.width();
let height = rect.height();
let left = rect.left();
let right = rect.right();
let top = rect.top();
let bottom = rect.bottom();
let center = rect.center();

// Operations
if rect.contains_point(50.0, 50.0) {
    println!("Point is inside");
}

if let Some(intersection) = rect1.intersect(&rect2) {
    println!("Intersection: {:?}", intersection);
}

let inflated = rect.inflate(10.0, 10.0);  // Grow by 10 in each direction
```

### Aspect Ratio

```rust
use svgtypes::{Align, AspectRatio, MeetOrSlice};

// Parse viewBox preserveAspectRatio
let aspect = AspectRatio {
    align: Align::XMidYMid,
    meet_or_slice: MeetOrSlice::Meet,
};

// Parse from string
let aspect = AspectRatio::parse("xMidYMid meet").unwrap();
let aspect = AspectRatio::parse("xMaxYMax slice").unwrap();

// Apply to rect
let source = Rect::new(0.0, 0.0, 100.0, 100.0);
let target = Rect::new(0.0, 0.0, 200.0, 150.0);
let transform = aspect.get_transform(source, target);
```

## Path Normalization

```rust
impl PathData {
    /// Convert relative commands to absolute
    pub fn to_absolute(&self) -> PathData {
        let mut result = PathData::new();
        let mut current_x = 0.0;
        let mut current_y = 0.0;

        for seg in self.iter() {
            match seg {
                PathSegment::MoveTo { abs, x, y } => {
                    let (x, y) = if *abs {
                        (*x, *y)
                    } else {
                        (current_x + x, current_y + y)
                    };
                    result.push_move_to(x, y, true);
                    current_x = x;
                    current_y = y;
                }
                PathSegment::LineTo { abs, x, y } => {
                    let (x, y) = if *abs {
                        (*x, *y)
                    } else {
                        (current_x + x, current_y + y)
                    };
                    result.push_line_to(x, y, true);
                    current_x = x;
                    current_y = y;
                }
                // ... handle other segments
                _ => {}
            }
        }

        result
    }

    /// Convert curves to lines (flatten)
    pub fn flatten(&self, tolerance: f64) -> PathData {
        let mut result = PathData::new();

        for seg in self.iter() {
            match seg {
                PathSegment::CubicTo { x1, y1, x2, y2, x, y, .. } => {
                    // Convert cubic bezier to line segments
                    let lines = cubic_to_lines(*x1, *y1, *x2, *y2, *x, *y, tolerance);
                    for (lx, ly) in lines {
                        result.push_line_to(lx, ly, true);
                    }
                }
                _ => result.push(*seg),
            }
        }

        result
    }
}
```

## Color Parsing Implementation

```rust
impl Color {
    pub fn parse(s: &str) -> Option<Color> {
        let s = s.trim();

        // Named color
        if let Some(color) = Self::parse_named(s) {
            return Some(color);
        }

        // Hex color
        if s.starts_with('#') {
            return Self::parse_hex(&s[1..]);
        }

        // rgb/rgba
        if s.starts_with("rgb") {
            return Self::parse_rgb(s);
        }

        // hsl/hsla
        if s.starts_with("hsl") {
            return Self::parse_hsl(s);
        }

        None
    }

    fn parse_hex(s: &str) -> Option<Color> {
        match s.len() {
            3 => {
                // #RGB
                let r = u8::from_str_radix(&s[0..1], 16).ok()?;
                let g = u8::from_str_radix(&s[1..2], 16).ok()?;
                let b = u8::from_str_radix(&s[2..3], 16).ok()?;
                Some(Color::new(r * 17, g * 17, b * 17, 255))
            }
            6 => {
                // #RRGGBB
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Color::new(r, g, b, 255))
            }
            8 => {
                // #RRGGBBAA
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = u8::from_str_radix(&s[6..8], 16).ok()?;
                Some(Color::new(r, g, b, a))
            }
            _ => None,
        }
    }

    fn parse_named(s: &str) -> Option<Color> {
        match s.to_lowercase().as_str() {
            "black" => Some(Color::BLACK),
            "white" => Some(Color::WHITE),
            "red" => Some(Color::RED),
            "green" => Some(Color::GREEN),
            "blue" => Some(Color::BLUE),
            "transparent" => Some(Color::TRANSPARENT),
            // ... more named colors
            _ => None,
        }
    }
}
```

## Usage in Resvg/USvg

```rust
// usvg uses svgtypes for parsing
use svgtypes::{Color, Length, Transform, PathData};

pub struct SvgElement {
    fill: Option<Color>,
    stroke: Option<Color>,
    stroke_width: Length,
    transform: Transform,
    path_data: PathData,
}

impl SvgElement {
    pub fn parse(attrs: &Attributes) -> Result<Self> {
        let fill = attrs.get("fill").map(Color::parse);
        let stroke = attrs.get("stroke").map(Color::parse);
        let stroke_width = Length::parse(attrs.get("stroke-width").unwrap_or("1"))?;
        let transform = Transform::parse(attrs.get("transform").unwrap_or(""))?;
        let path_data = PathData::parse(attrs.get("d").unwrap_or(""))?;

        Ok(SvgElement {
            fill,
            stroke,
            stroke_width,
            transform,
            path_data,
        })
    }
}
```

## Related Documents

- [Resvg](./resvg-exploration.md) - SVG renderer using these types
- [FFrames Core](./fframes-core-exploration.md) - SVG handling

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/svgtypes/`
- SVGTYPES GitHub: https://github.com/RazrFalcon/svgtypes
- SVG Specification: https://www.w3.org/TR/SVG/
