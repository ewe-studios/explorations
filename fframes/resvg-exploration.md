---
name: Resvg
description: Rust SVG renderer using tiny-skia for high-fidelity SVG to raster conversion
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/resvg/
---

# Resvg - Rust SVG Renderer

## Overview

Resvg is a **high-fidelity SVG renderer** written in Rust. It converts SVG documents to raster images using the tiny-skia library, providing accurate SVG 2.0 rendering with excellent performance. This library is used extensively in FFrames for converting SVG scene descriptions to pixel data.

Key features:
- **SVG 2.0 support** - Comprehensive SVG specification coverage
- **tiny-skia backend** - Fast CPU-based rasterization
- **No external dependencies** - Pure Rust, no FFI required
- **High fidelity** - Accurate rendering of complex SVG features
- **Text shaping** - Integration with fontdue for text rendering
- **Filter support** - SVG filters (blur, shadows, etc.)

## Directory Structure

```
resvg/
├── crates/
│   ├── resvg/                 # Main library
│   │   ├── src/
│   │   │   ├── render.rs      # Rendering logic
│   │   │   ├── tree.rs        # SVG tree representation
│   │   │   └── lib.rs
│   ├── tiny-skia/             # Skia-like 2D graphics (often separate)
│   └── usvg/                  # Universal SVG parser
├── docs/                      # Documentation
├── tools/                     # Utility tools
├── benches/                   # Benchmarks
├── fuzz/                      # Fuzzing infrastructure
├── Cargo.toml
├── CHANGELOG.md
└── README.md
```

## Core Components

### Main API

```rust
use resvg::tiny_skia;
use usvg::{Tree, TreeParsing, TreeTextToPath};

// Parse SVG string
let svg_data = r#"
<svg width="100" height="100" viewBox="0 0 100 100">
    <circle cx="50" cy="50" r="40" fill="red"/>
</svg>
"#;

// Parse options
let options = usvg::Options::default();
let tree = Tree::from_str(svg_data, &options).unwrap();

// Convert text to paths (if any)
let mut tree = tree;
tree.convert_text(&usvg::fontdb::Database::new());

// Render to pixmap
let pixmap_size = tree.size().to_int_size();
let mut pixmap = tiny_skia::Pixmap::new(
    pixmap_size.width() as u32,
    pixmap_size.height() as u32,
).unwrap();

// Render
resvg::render(
    &tree,
    tiny_skia::Transform::default(),
    &mut pixmap.as_mut(),
);

// Get pixel data
let rgba_data: &[u8] = pixmap.data();
```

### Tree Representation

```rust
// usvg tree structure
pub struct Tree {
    /// SVG size
    size: Size,

    /// Root layer
    root: Group,

    /// CSS stylesheet
    stylesheet: Option<StyleSheet>,

    /// Document metadata
    metadata: Metadata,
}

pub struct Group {
    /// Child elements
    children: Vec<Node>,

    /// Transform
    transform: Transform,

    /// Clip path
    clip_path: Option<Rc<Group>>,

    /// Mask
    mask: Option<Rc<Group>>,

    /// Opacity
    opacity: f32,
}

pub enum Node {
    Group(Group),
    Path(Path),
    Image(Image),
    Text(Text),
    Svg(Svg),
}

pub struct Path {
    /// Path geometry
    path: tiny_skia::Path,

    /// Fill style
    fill: Option<Fill>,

    /// Stroke style
    stroke: Option<Stroke>,

    /// Visibility
    visible: bool,
}

pub struct Fill {
    /// Fill rule
    rule: FillRule,

    /// Paint (color, gradient, pattern)
    paint: Paint,

    /// Opacity
    opacity: f32,
}

pub enum Paint {
    Color(Color),
    LinearGradient(Rc<LinearGradient>),
    RadialGradient(Rc<RadialGradient>),
    Pattern(Rc<Pattern>),
}
```

### Rendering Process

```rust
/// Main rendering function
pub fn render(
    tree: &usvg::Tree,
    transform: tiny_skia::Transform,
    target: &mut tiny_skia::PixmapMut,
) {
    // Clear target
    target.fill(tiny_skia::Color::TRANSPARENT);

    // Create render state
    let state = RenderState {
        transform,
        clip_rect: None,
        opacity: 1.0,
    };

    // Render root group
    render_group(&tree.root, &state, target);
}

fn render_group(
    group: &usvg::Group,
    state: &RenderState,
    target: &mut tiny_skia::PixmapMut,
) {
    // Apply group transform
    let new_transform = state.transform.pre_concat(group.transform);

    // Create new state for children
    let child_state = RenderState {
        transform: new_transform,
        opacity: state.opacity * group.opacity,
        ..*state
    };

    // Render children
    for child in &group.children {
        render_node(child, &child_state, target);
    }
}

fn render_node(
    node: &usvg::Node,
    state: &RenderState,
    target: &mut tiny_skia::PixmapMut,
) {
    match node {
        usvg::Node::Path(path) => render_path(path, state, target),
        usvg::Node::Group(group) => render_group(group, state, target),
        usvg::Node::Image(image) => render_image(image, state, target),
        usvg::Node::Text(text) => render_text(text, state, target),
        usvg::Node::Svg(svg) => render_embedded_svg(svg, state, target),
    }
}
```

### Path Rendering

```rust
fn render_path(
    path: &usvg::Path,
    state: &RenderState,
    target: &mut tiny_skia::PixmapMut,
) {
    // Convert usvg path to tiny-skia path
    let sk_path = convert_path(&path.path);

    // Create paint
    let mut paint = tiny_skia::Paint::default();

    // Apply fill
    if let Some(fill) = &path.fill {
        match &fill.paint {
            usvg::Paint::Color(color) => {
                paint.set_color_rgba8(
                    color.red,
                    color.green,
                    color.blue,
                    (color.alpha as f32 * fill.opacity) as u8,
                );
            }
            usvg::Paint::LinearGradient(gradient) => {
                paint.shader = create_linear_gradient(gradient, state);
            }
            usvg::Paint::RadialGradient(gradient) => {
                paint.shader = create_radial_gradient(gradient, state);
            }
            usvg::Paint::Pattern(pattern) => {
                paint.shader = create_pattern(pattern, state);
            }
        }
    }

    // Set fill rule
    paint.fill_rule = match path.fill.map(|f| f.rule) {
        Some(usvg::FillRule::EvenOdd) => tiny_skia::FillRule::EvenOdd,
        _ => tiny_skia::FillRule::Winding,
    };

    // Fill path
    target.fill_path(
        &sk_path,
        &paint,
        paint.fill_rule,
        tiny_skia::AntiAlias::Yes,
        state.transform,
    );

    // Apply stroke
    if let Some(stroke) = &path.stroke {
        render_stroke(path, stroke, state, target);
    }
}
```

### Gradient Support

```rust
fn create_linear_gradient(
    gradient: &usvg::LinearGradient,
    state: &RenderState,
) -> Option<tiny_skia::Shader> {
    // Convert stops
    let stops: Vec<tiny_skia::GradientStop> = gradient
        .stops
        .iter()
        .map(|stop| tiny_skia::GradientStop {
            offset: stop.offset.get() as f32,
            color: tiny_skia::Color::from_rgba8(
                stop.color.red,
                stop.color.green,
                stop.color.blue,
                stop.color.alpha,
            ),
        })
        .collect();

    // Create gradient
    tiny_skia::LinearGradient::new(
        tiny_skia::Point::from_xy(
            gradient.x1.get() as f32,
            gradient.y1.get() as f32,
        ),
        tiny_skia::Point::from_xy(
            gradient.x2.get() as f32,
            gradient.y2.get() as f32,
        ),
        tiny_skia::Vector::new(0.0, 0.0),
        stops,
        tiny_skia::GradientSpread::Pad,
        tiny_skia::Transform::default(),
    )
}
```

### Filter Support

```rust
fn apply_filter(
    source: &tiny_skia::Pixmap,
    filter: &usvg::Filter,
    target: &mut tiny_skia::PixmapMut,
) {
    for primitive in &filter.primitives {
        match primitive {
            usvg::FilterPrimitive::Blur(blur) => {
                apply_blur(source, target, blur.std_deviation);
            }
            usvg::FilterPrimitive::DropShadow(shadow) => {
                apply_drop_shadow(source, target, shadow);
            }
            usvg::FilterPrimitive::ColorMatrix(matrix) => {
                apply_color_matrix(source, target, &matrix.matrix);
            }
            usvg::FilterPrimitive::Merge(merge) => {
                apply_merge(source, target, &merge.inputs);
            }
            _ => {}
        }
    }
}

fn apply_blur(
    source: &tiny_skia::Pixmap,
    target: &mut tiny_skia::PixmapMut,
    std_dev: (f32, f32),
) {
    // Gaussian blur implementation
    tiny_skia::filter_blur(
        source,
        std_dev.0,
        std_dev.1,
        target,
        tiny_skia::FilterQuality::Medium,
    );
}
```

### Text Rendering

```rust
use usvg::{TreeTextToPath, fontdb};

fn render_text(
    text: &usvg::Text,
    state: &RenderState,
    target: &mut tiny_skia::PixmapMut,
) {
    // Convert text to paths
    let mut font_db = fontdb::Database::new();
    font_db.load_system_fonts();

    let mut tree_with_paths = text.clone();
    tree_with_paths.convert_text(&font_db);

    // Now render as paths
    for span in &tree_with_paths.spans {
        for glyph in &span.glyphs {
            let path = glyph.path.clone();
            render_path_glyph(&path, glyph.transform, state, target);
        }
    }
}
```

## Performance Optimization

### Bounding Box Culling

```rust
fn should_render(
    bbox: &tiny_skia::Rect,
    clip_rect: Option<tiny_skia::Rect>,
) -> bool {
    if let Some(clip) = clip_rect {
        bbox.intersect(clip).is_some()
    } else {
        true
    }
}

fn render_group_optimized(
    group: &usvg::Group,
    state: &RenderState,
    target: &mut tiny_skia::PixmapMut,
) {
    // Calculate group bounding box
    let bbox = group.calculate_bbox();

    // Skip if outside clip region
    if !should_render(&bbox, state.clip_rect) {
        return;
    }

    // Continue rendering...
}
```

### Layer Caching

```rust
use std::collections::HashMap;

struct RenderCache {
    layers: HashMap<usize, tiny_skia::Pixmap>,
}

impl RenderCache {
    fn get_or_render<F>(
        &mut self,
        id: usize,
        render_fn: F,
    ) -> &tiny_skia::Pixmap
    where
        F: FnOnce() -> tiny_skia::Pixmap,
    {
        self.layers.entry(id).or_insert_with(render_fn)
    }
}
```

## Integration with FFrames

```rust
// From fframes-renderer
use resvg::tiny_skia;
use usvg::{Tree, TreeParsing};

pub struct SvgRenderer {
    options: usvg::Options,
    font_db: usvg::fontdb::Database,
}

impl SvgRenderer {
    pub fn new() -> Self {
        let mut font_db = usvg::fontdb::Database::new();
        font_db.load_system_fonts();

        SvgRenderer {
            options: usvg::Options::default(),
            font_db,
        }
    }

    pub fn render_svg(
        &self,
        svg_str: &str,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        // Parse SVG
        let tree = Tree::from_str(svg_str, &self.options).unwrap();

        // Create pixmap
        let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();

        // Render
        resvg::render(
            &tree,
            tiny_skia::Transform::default(),
            &mut pixmap.as_mut(),
        );

        // Return RGBA pixels
        pixmap.data().to_vec()
    }
}
```

## CLI Tool

```bash
# Render SVG to PNG
resvg input.svg output.png

# With specific size
resvg --width 800 --height 600 input.svg output.png

# With background color
resvg --background-color "#ffffff" input.svg output.png

# Output format options
resvg input.svg output.png  # PNG
resvg input.svg output.pam  # PAM
resvg input.svg output.pnm  # PNM
```

## Related Documents

- [FFrames Renderer](./fframes-renderer-exploration.md) - SVG rendering integration
- [SVGTYPES](./svgtypes-exploration.md) - SVG type definitions

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/resvg/`
- Resvg GitHub: https://github.com/RazrFalcon/resvg
- tiny-skia: https://github.com/RazrFalcon/tiny-skia
