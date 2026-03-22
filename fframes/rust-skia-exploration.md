---
name: Rust Skia
description: Skia graphics library bindings for GPU-accelerated 2D rendering in Rust
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/rust-skia/
---

# Rust Skia - GPU-Accelerated 2D Graphics

## Overview

Rust Skia provides **Rust bindings to the Skia graphics library**, enabling high-performance 2D rendering with GPU acceleration. Skia is the same graphics engine powering Chrome, Android, Flutter, and Electron, providing:

- **GPU acceleration** - Vulkan, Metal, Direct3D, OpenGL backends
- **Cross-platform** - Windows, macOS, Linux, iOS, Android
- **Advanced text rendering** - Complex script support, font shaping
- **Vector graphics** - Paths, gradients, transforms
- **Image processing** - Filters, blending, color spaces
- **PDF/SVG output** - Vector format export

## Directory Structure

```
rust-skia/
├── skia-bindings/           # Low-level FFI bindings
│   ├── src/
│   │   ├── bindings.rs      # Generated Skia bindings
│   │   ├── builder.rs       # Bindgen builder
│   │   └── lib.rs
│   ├── build.rs             # Skia source build
│   └── Cargo.toml
├── skia-safe/               # Safe Rust wrappers
│   ├── src/
│   │   ├── canvas.rs        # Drawing canvas
│   │   ├── paint.rs         # Paint styling
│   │   ├── path.rs          # Vector paths
│   │   ├── surface.rs       # Rendering surface
│   │   ├── image.rs         # Image handling
│   │   ├── font.rs          # Font handling
│   │   ├── gpu/             # GPU backend support
│   │   └── lib.rs
│   └── Cargo.toml
├── skia-svg-macros/         # SVG macro support
├── wasm-example/            # WebAssembly example
├── skia-org/                # Organization tools
├── Makefile                 # Build automation
├── README.md
└── flake.nix                # Nix development env
```

## Build System

### Skia Source Compilation

```rust
// skia-bindings/build.rs
fn main() {
    // Clone or locate Skia source
    let skia_dir = locate_or_clone_skia();

    // Configure Skia build
    let mut gn_args = vec![
        "is_official_build=true".into(),
        "skia_enable_gpu=true".into(),
        "skia_use_system_libs=false".into(),
        "skia_use_png=true".into(),
        "skia_use_jpeg=true".into(),
        "skia_use_webp=true".into(),
        "skia_use_icu=true".into(),
    ];

    // Platform-specific GPU backend
    #[cfg(target_os = "macos")]
    gn_args.push("skia_use_metal=true".into());

    #[cfg(target_os = "linux")]
    gn_args.push("skia_use_vulkan=true".into());

    #[cfg(target_os = "windows")]
    gn_args.push("skia_use_d3d=true".into());

    // Build Skia with ninja
    build_skia_with_gn(&skia_dir, &gn_args);

    // Generate FFI bindings
    generate_bindings(&skia_dir);
}
```

### Cargo Features

```toml
[features]
default = ["gpu", "textlayout", "svg"]
gpu = ["skia-safe/gpu"]
metal = ["skia-safe/metal"]      # macOS
vulkan = ["skia-safe/vulkan"]    # Linux/Windows
d3d = ["skia-safe/d3d"]          # Windows
gl = ["skia-safe/gl"]            # OpenGL
textlayout = ["skia-safe/textlayout"]
svg = ["skia-safe/svg"]
webp = ["skia-safe/webp"]
```

## Core API

### Canvas and Surface

```rust
use skia_safe::{
    Canvas, Surface, Color, ColorType, AlphaType,
    Paint, PaintStyle, Path, Matrix, Rect, Point,
};

// Create offscreen surface
let mut surface = Surface::new_raster_direct(
    &ImageInfo::new(
        (1920, 1080),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    ),
    None,
    None,
).unwrap();

// Get canvas for drawing
let canvas = surface.canvas();

// Clear to transparent
canvas.clear(Color::TRANSPARENT);

// Draw rectangle
let mut paint = Paint::default();
paint.set_color(Color::from_rgb(255, 0, 0));
paint.set_anti_alias(true);
canvas.draw_rect(Rect::new(100.0, 100.0, 200.0, 200.0), &paint);

// Draw circle
paint.set_color(Color::from_rgb(0, 255, 0));
canvas.draw_circle((400.0, 400.0), 100.0, &paint);

// Flush to ensure drawing completes
surface.flush();

// Get pixel data
let image = surface.image_snapshot();
let data = image.encode_to_data(EncodedImageFormat::PNG).unwrap();
```

### GPU Surfaces

```rust
use skia_safe::gpu::{
    vulkan::{BackendContext, GetProcOf},
    metal::{BackendContext as MetalBackend},
    DirectContext,
    SurfaceOrigin,
};
use skia_safe::{Surface, ColorType};

// Vulkan backend
fn create_vulkan_surface(
    get_proc: impl GetProcOf,
    width: i32,
    height: i32,
) -> Surface {
    // Create backend context
    let backend = BackendContext::new(get_proc);

    // Create direct context
    let mut context = DirectContext::new_vulkan(&backend, None).unwrap();

    // Create surface
    Surface::new_renderable(
        &mut context,
        (width, height),
        None,
        ColorType::RGBA8888,
        None,
        None,
        false,
    ).unwrap()
}

// Metal backend (macOS)
fn create_metal_surface(
    device: &metal::Device,
    queue: &metal::CommandQueue,
    width: i32,
    height: i32,
) -> Surface {
    let backend = MetalBackend::new(device, queue);
    let mut context = DirectContext::new_metal(&backend, None).unwrap();

    Surface::new_renderable(
        &mut context,
        (width, height),
        None,
        ColorType::RGBA8888,
        None,
        None,
        false,
    ).unwrap()
}
```

### Paths and Drawing

```rust
use skia_safe::{Path, Paint, Color, PathFillType};

// Create complex path
let mut path = Path::new();
path.move_to((100.0, 100.0));
path.line_to((200.0, 150.0));
path.quad_to((250.0, 250.0), (300.0, 100.0));
path.cubic_to((350.0, 50.0), (400.0, 150.0), (450.0, 100.0));
path.close();

// Set fill type
path.set_fill_type(PathFillType::EvenOdd);

// Create gradient paint
let mut paint = Paint::default();
paint.set_shader(skia_safe::gradient_shader::linear(
    (Point::new(0.0, 0.0), Point::new(100.0, 100.0)),
    &[Color::from_rgb(255, 0, 0), Color::from_rgb(0, 0, 255)],
    None,
    None,
    None,
));
paint.set_anti_alias(true);

// Draw path
canvas.draw_path(&path, &paint);
```

### Text Rendering

```rust
use skia_safe::{
    Font, Typeface, TypefaceFontProvider, TextBlob,
    textlayout::{FontCollection, ParagraphStyle, ParagraphBuilder},
};

// Load font
let typeface = Typeface::from_name("Helvetica", skia_safe::FontStyle::normal()).unwrap();

// Create font
let mut font = Font::new(Some(typeface), 72.0);
font.set_subpixel(true);
font.set_edging(skia_safe::font::Edging::AntiAlias);

// Draw text
let blob = TextBlob::from_str("Hello, Skia!", &font).unwrap();
canvas.draw_text_blob(&blob, (100.0, 200.0), &paint);

// Advanced text layout
let mut font_collection = FontCollection::new();
font_collection.set_default_font_manager(None);

let paragraph_style = ParagraphStyle::new();
let mut builder = ParagraphBuilder::new(&paragraph_style, font_collection);
builder.add_text("Complex text with ");
builder.push_style(skia_safe::textlayout::TextStyle::new().font_size(24.0));
builder.add_text("mixed styles");
builder.pop();

let paragraph = builder.build();
paragraph.layout(800.0);
paragraph.paint(canvas, (0.0, 0.0));
```

### Image Operations

```rust
use skia_safe::{Image, EncodedImageFormat, Matrix, SamplingOptions, FilterMode};

// Load image
let data = skia_safe::Data::new_copy(&std::fs::read("input.png").unwrap());
let image = Image::from_encoded(data).unwrap();

// Scale image
let scaled = image.scale_to_subset(
    &Rect::new(0.0, 0.0, 500.0, 500.0),
    SamplingOptions::new(FilterMode::Linear, FilterMode::Linear),
);

// Encode to PNG
let png_data = scaled
    .encode_to_data(EncodedImageFormat::PNG)
    .unwrap();

// Decode and draw
let decoded = Image::from_encoded(png_data).unwrap();
canvas.draw_image(&decoded, (0.0, 0.0), None);

// Apply image filter
let filter = skia_safe::image_filters::blur((5.0, 5.0), None);
let filtered = image
    .filter_image(
        filter,
        &image.subset().unwrap(),
        &Matrix::identity(),
        SamplingOptions::default(),
    )
    .unwrap();
```

### SVG Rendering

```rust
use skia_safe::svg::Canvas;
use skia_safe::{Rect, Size};
use std::fs::File;
use std::io::BufWriter;

// Create SVG canvas
let file = File::create("output.svg").unwrap();
let mut writer = BufWriter::new(file);

let svg_canvas = Canvas::new_in_box(
    Size::new(800.0, 600.0),
    &mut writer,
).unwrap();

// Draw to SVG
{
    let canvas = svg_canvas.as_canvas_mut();
    let mut paint = Paint::default();
    paint.set_color(skia_safe::Color::from_rgb(255, 0, 0));
    canvas.draw_circle((400.0, 300.0), 100.0, &paint);
}

// Finalize SVG
drop(svg_canvas);
writer.flush().unwrap();
```

## Integration with FFrames

```rust
// From fframes-skia-renderer
use skia_safe::{
    Surface, ColorType, AlphaType, ImageInfo,
    gpu::{DirectContext, SurfaceOrigin},
};

pub struct SkiaRenderer {
    surface: Surface,
    gpu_context: Option<DirectContext>,
    width: i32,
    height: i32,
}

impl SkiaRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        // Create raster surface for CPU rendering
        let surface = Surface::new_raster_direct(
            &ImageInfo::new(
                (width as i32, height as i32),
                ColorType::RGBA8888,
                AlphaType::Premul,
                None,
            ),
            None,
            None,
        ).unwrap();

        SkiaRenderer {
            surface,
            gpu_context: None,
            width: width as i32,
            height: height as i32,
        }
    }

    pub fn render_svg(&mut self, svg_tree: &resvg::Tree) -> Vec<u8> {
        let canvas = self.surface.canvas();
        canvas.clear(skia_safe::Color::TRANSPARENT);

        // Convert SVG to Skia picture
        let picture = svg_to_skia_picture(svg_tree);

        // Draw picture
        canvas.draw_picture(&picture, None, None);

        // Flush and read pixels
        self.surface.flush();

        let image_info = ImageInfo::new(
            (self.width, self.height),
            ColorType::RGBA8888,
            AlphaType::Premul,
            None,
        );

        let mut pixels = vec![0u8; image_info.min_row_bytes() * self.height as usize];
        self.surface.read_pixels(
            &image_info,
            &mut pixels,
            image_info.min_row_bytes(),
            0,
            0,
        );

        pixels
    }
}

fn svg_to_skia_picture(svg_tree: &resvg::Tree) -> skia_safe::Picture {
    use skia_safe::PictureRecorder;

    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(
        skia_safe::Rect::new(
            0.0,
            0.0,
            svg_tree.size.width(),
            svg_tree.size.height(),
        ),
        None,
    );

    // Draw SVG nodes to canvas
    for node in &svg_tree.root.children {
        draw_svg_node(canvas, node);
    }

    recorder.finish_recording_as_picture().unwrap()
}
```

## GPU Backend Selection

### Platform Detection

```rust
// skia-safe/src/gpu/mod.rs
pub fn get_default_backend() -> GpuBackend {
    #[cfg(target_os = "macos")]
    {
        GpuBackend::Metal
    }

    #[cfg(all(target_os = "windows", feature = "d3d"))]
    {
        GpuBackend::D3D
    }

    #[cfg(target_os = "linux")]
    {
        GpuBackend::Vulkan
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        GpuBackend::OpenGL
    }
}
```

### Vulkan Setup

```rust
use skia_safe::gpu::vulkan::{BackendContext, GetProcOf};
use ash::{Instance, Entry};

struct VulkanProc {
    instance: Instance,
    entry: Entry,
}

impl GetProcOf for VulkanProc {
    fn get_proc_of(&self, name: &str) -> Option<std::ffi::c_void> {
        self.entry.get_instance_proc_addr(
            self.instance.handle(),
            std::ffi::CString::new(name).unwrap().as_ptr(),
        )
    }
}

fn create_vulkan_backend() -> BackendContext {
    let entry = Entry::linked();
    let instance = create_vulkan_instance(&entry);
    let proc = VulkanProc { instance, entry };

    BackendContext::new(proc)
}
```

## WASM Support

```rust
// For WebAssembly builds
use skia_safe::Canvas;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmRenderer {
    surface: skia_safe::Surface,
}

#[wasm_bindgen]
impl WasmRenderer {
    pub fn new(width: i32, height: i32) -> WasmRenderer {
        let surface = skia_safe::Surface::new_raster_direct(
            &skia_safe::ImageInfo::new(
                (width, height),
                skia_safe::ColorType::RGBA8888,
                skia_safe::AlphaType::Premul,
                None,
            ),
            None,
            None,
        ).unwrap();

        WasmRenderer { surface }
    }

    pub fn draw_circle(&mut self, x: f32, y: f32, radius: f32, color: u32) {
        let canvas = self.surface.canvas();
        let mut paint = skia_safe::Paint::default();
        paint.set_color(skia_safe::Color::new(color));
        paint.set_anti_alias(true);
        canvas.draw_circle((x, y), radius, &paint);
    }

    pub fn get_pixels(&mut self) -> Vec<u8> {
        self.surface.flush();

        let image_info = skia_safe::ImageInfo::new(
            (self.surface.width(), self.surface.height()),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Premul,
            None,
        );

        let mut pixels = vec![0u8; image_info.min_row_bytes() * self.surface.height() as usize];
        self.surface.read_pixels(
            &image_info,
            &mut pixels,
            image_info.min_row_bytes(),
            0,
            0,
        );

        pixels
    }
}
```

## Performance Considerations

### Surface Reuse

```rust
// Bad: Creating surface every frame
fn render_frame_bad() -> Vec<u8> {
    let surface = Surface::new_raster(...); // Slow!
    // ... draw ...
    read_pixels(&surface)
}

// Good: Reuse surface
struct Renderer {
    surface: Surface,
}

impl Renderer {
    fn render_frame(&mut self) -> Vec<u8> {
        let canvas = self.surface.canvas();
        canvas.clear(Color::TRANSPARENT);
        // ... draw ...
        self.surface.flush();
        read_pixels(&self.surface)
    }
}
```

### Batch Drawing

```rust
// Group operations to reduce state changes
let mut paint = Paint::default();
paint.set_color(Color::RED);

// Draw all red objects first
for obj in &red_objects {
    canvas.draw_path(&obj.path, &paint);
}

// Then all blue
paint.set_color(Color::BLUE);
for obj in &blue_objects {
    canvas.draw_path(&obj.path, &paint);
}
```

## Related Documents

- [FFrames Renderer](./fframes-renderer-exploration.md) - GPU rendering backend
- [Resvg](./resvg-exploration.md) - SVG rendering

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.fframes/rust-skia/`
- Skia Documentation: https://skia.org/
- Rust Skia GitHub: https://github.com/rust-skia/rust-skia
