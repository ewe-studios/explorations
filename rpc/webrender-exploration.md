# WebRender Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC/webrender
repository: https://github.com/servo/webrender
explored_at: 2026-03-23

## Overview

WebRender is a GPU-based 2D rendering engine written in Rust. While not primarily an RPC framework, it uses IPC extensively for communication between the browser's content process and the rendering process. This exploration focuses on its IPC mechanisms.

## Project Structure

```
webrender/
├── webrender/             # Main renderer
│   ├── src/
│   │   ├── lib.rs
│   │   ├── renderer.rs    # GPU renderer
│   │   ├── scene.rs       # Scene building
│   │   ├── batching.rs    # Render batching
│   │   └── ...
│   └── Cargo.toml
├── webrender_api/         # IPC API definitions
│   ├── src/
│   │   ├── lib.rs
│   │   ├── messages.rs    # IPC messages
│   │   ├── display_list.rs # Display list
│   │   └── resources.rs   # Resource handling
│   └── Cargo.toml
├── webrender_build/       # Build utilities
├── example-compositor/    # Example compositor implementations
│   ├── compositor/
│   ├── compositor-wayland/
│   └── compositor-windows/
├── examples/              # Example applications
├── wr_glyph_rasterizer/   # Glyph rasterization
├── wr_malloc_size_of/     # Memory measurement
├── peek-poke/             # Serialization helpers
├── swgl/                  # Software GL
├── fog/                   # Firefox Graphics instrumentation
└── wrench/                # Testing tool
```

## IPC Architecture

### Process Model

```
┌─────────────────┐         ┌─────────────────┐
│  Content        │         │   GPU           │
│  Process        │         │   Process       │
│                 │         │                 │
│  ┌───────────┐  │  IPC    │  ┌───────────┐  │
│  │ Web       │  │ ──────> │  │ WebRender │  │
│  │ Content   │  │         │  │ Renderer  │  │
│  └───────────┘  │         │  └───────────┘  │
└─────────────────┘         └─────────────────┘
```

### webrender_api Crate

```toml
[package]
name = "webrender_api"
version = "0.1.0"
edition = "2021"

[dependencies]
bincode = "1"
serde = { version = "1", features = ["derive"] }
ipc-channel = "0.21"
```

## IPC Messages

### Message Types

```rust
// webrender_api/src/messages.rs

#[derive(Serialize, Deserialize)]
pub enum ApiMsg {
    /// Add new display list
    UpdateResources(Vec<ResourceUpdate>),

    /// Build and render new scene
    SetDisplayList(DisplayList),

    /// Generate frame
    GenerateFrame,

    /// Scroll the page
    Scroll(ScrollLocation, WebRenderScrollState),

    /// Hit testing
    HitTest(HitTestRequest),
}
```

### Resource Updates

```rust
#[derive(Serialize, Deserialize, Clone)]
pub enum ResourceUpdate {
    /// Add new font
    AddFont(AddFont),

    /// Add image data
    AddImage(ImageKey, ImageDescriptor, ImageData),

    /// Update existing image
    UpdateImage(ImageKey, ImageDescriptor, ImageData),

    /// Delete image
    DeleteImage(ImageKey),
}
```

### Display List

```rust
#[derive(Serialize, Deserialize)]
pub struct DisplayList {
    /// Pipeline identifier
    pub pipeline_id: PipelineId,

    /// Display list data (bincode encoded)
    pub display_list_data: Vec<u8>,

    /// Space definitions
    pub spatial_trees: SpatialTree,
}
```

## Serialization

### Peek-Poke

```rust
// peek-poke library for efficient serialization

pub trait Peek: Sized {
    fn peek(input: &[u8]) -> Option<(Self, &[u8])>;
}

pub trait Poke {
    fn poke_into(&self, output: &mut Vec<u8>);
}

// Display list items use peek-poke for efficiency
#[repr(C)]
pub struct DisplayItem {
    pub item_type: DisplayItemKind,
    pub rect: LayoutRect,
    // ...
}
```

### Bincode Integration

```rust
use bincode::{serialize, deserialize};

// Serialize display list
let data = bincode::serialize(&display_list)?;

// Deserialize on renderer side
let display_list: DisplayList = bincode::deserialize(&data)?;
```

## Transaction API

### Building Transactions

```rust
// Content process
let mut txn = Transaction::new();

// Update resources
txn.add_image(
    image_key,
    ImageDescriptor::new(100, 100, ImageFormat::RGBA8),
    ImageData::new(bytes),
);

// Set display list
txn.set_display_list(
    DocumentId(1),
    None,  // Epoch
    LayoutSize::new(800.0, 600.0),
    display_list,
);

// Request render
txn.generate_frame();

// Send to renderer
api.send_transaction(document_id, txn);
```

### Transaction Structure

```rust
pub struct Transaction {
    /// Display list to set
    display_list: Option<(PipelineId, BuiltDisplayList)>,

    /// Resource updates
    resource_updates: Vec<ResourceUpdate>,

    /// Frame generation request
    generate_frame: bool,

    /// Scroll updates
    scrolls: Vec<(ExternalScrollId, ScrollAmount)>,
}
```

## Renderer Communication

### Renderer Thread

```rust
// Renderer runs on dedicated thread
pub struct Renderer {
    /// GL device
    device: Device,

    /// Texture cache
    texture_cache: TextureCache,

    /// GPU batches
    batches: Vec<Batch>,
}

impl Renderer {
    pub fn new(options: Options) -> Result<Self, Error> {
        // Initialize GPU
    }

    pub fn update(&mut self) {
        // Process pending messages
        // Build scene
        // Render frame
    }

    pub fn render(&mut self) {
        // Execute GPU commands
    }
}
```

### Message Channel

```rust
use ipc_channel::ipc::{IpcSender, IpcReceiver};

pub struct RenderApi {
    /// Channel to renderer
    api_tx: IpcSender<ApiMsg>,

    /// Channel from renderer
    result_rx: IpcReceiver<RendererMsg>,
}

impl RenderApi {
    pub fn send_message(&self, msg: ApiMsg) {
        self.api_tx.send(msg).unwrap();
    }

    pub fn recv_result(&self) -> RendererMsg {
        self.result_rx.recv().unwrap()
    }
}
```

### Renderer Messages

```rust
#[derive(Serialize, Deserialize)]
pub enum RendererMsg {
    /// Frame rendered successfully
    FrameReady(FrameId, RenderedDocument),

    /// Hit test results
    HitTestReady(HitTestResult),

    /// Error occurred
    Error(String),
}
```

## Compositor Integration

### Native Compositor

```rust
// example-compositor/compositor/src/main.rs

use webrender::{Compositor, CompositorCapabilities};

pub struct NativeCompositor {
    // Platform-specific compositor state
}

impl Compositor for NativeCompositor {
    fn create_surface(
        &mut self,
        size: DeviceIntSize,
    ) -> CompositorSurface {
        // Create native surface
    }

    fn present(&mut self) {
        // Present frame to screen
    }
}
```

### Wayland Compositor

```rust
// compositor-wayland/src/lib.rs

use wayland_client::{protocol::wl_shm, Connection};

pub struct WaylandCompositor {
    connection: Connection,
    surface: wl_surface::WlSurface,
    // ...
}
```

## Performance Optimizations

### Display List Reuse

```rust
// Cache display lists for reuse
struct DisplayListCache {
    cache: HashMap<PipelineId, BuiltDisplayList>,
}

impl DisplayListCache {
    fn get_or_build(&mut self, pipeline_id: PipelineId) -> &BuiltDisplayList {
        // Reuse if unchanged
    }
}
```

### Resource Sharing

```rust
// Share resources between processes
struct SharedResources {
    /// Shared memory for images
    image_shmem: SharedMemory,

    /// Font data (read-only, shared)
    font_data: Arc<FontDb>,
}
```

### Batching

```rust
// Group draw calls by texture/state
struct Batch {
    /// Texture bound for this batch
    texture_id: TextureId,

    /// Draw commands
    commands: Vec<DrawCommand>,
}

// Reduces state changes
```

## Testing with Wrench

### Wrench Tool

```rust
// wrench/src/main.rs

// Run reftests
wrench reftest tests/

// Benchmark
wrench perf bench.yaml

// Debug rendering
wrench show frame.bin
```

### Reftest Format

```yaml
# tests/basic.yaml
display_list: basic.yaml
reftests:
  - test: basic-1.yaml
    reference: basic-1-ref.yaml
```

## Firefox Integration

### Layers IPC

```rust
// Firefox uses additional IPC layers
enum LayersMsg {
    /// Update CompositableHost
    UpdateCompositable(CompositableUpdate),

    /// Video frame update
    VideoFrameUpdate(VideoFrame),
}
```

### GPU Process

```rust
// Firefox GPU process隔离
#[cfg(feature = "gpu_process")]
mod gpu_process {
    // Separate process for GPU rendering
}
```

## Security Considerations

### Content Process Isolation

```rust
// Untrusted content in separate process
// IPC provides security boundary

// Validate all IPC messages
fn validate_message(msg: &ApiMsg) -> Result<(), Error> {
    // Check sizes, limits, etc.
}
```

### Resource Limits

```rust
struct ResourceLimits {
    max_texture_size: i32,
    max_image_bytes: usize,
    max_display_list_size: usize,
}
```

## Comparison with Other Approaches

| Approach | IPC Method | Zero-Copy | Platform Support |
|----------|-----------|-----------|------------------|
| WebRender | bincode + ipc-channel | Partial | All |
| Skia | Shared memory | Yes | Limited |
| Direct2D | COM | Yes | Windows only |

## Resources

- [WebRender Wiki](https://github.com/servo/webrender/wiki)
- [Mozilla Graphics Documentation](https://firefox-source-docs.mozilla.org/gfx/index.html)
- [Servo Project](https://github.com/servo/servo)
