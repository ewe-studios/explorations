---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad
repository: https://github.com/makepad/makepad, https://github.com/project-robius
explored_at: 2026-03-22
language: Rust, JavaScript, Kotlin, Swift, HLSL/GLSL
---

# Project Exploration: Makepad & Project Robius

## Overview

**Makepad** is a Rust-based UI toolkit for building cross-platform applications that run on desktop (Windows, macOS, Linux), mobile (iOS, Android), and web (WASM) from a single codebase. It features a custom immediate-mode rendering engine, a DSL for styling and layout, and a growing widget set.

**Project Robius** is a community-driven initiative building on Makepad to create a complete application development framework, including **Robrix** (a Matrix chat client), **Moly** (an AI LLM client), and supporting libraries like **eyeball** (observability).

### Key Value Proposition

- **True cross-platform** - Single Rust codebase for desktop, mobile, and web
- **Custom rendering engine** - GPU-accelerated 2D/3D rendering without native dependencies
- **Hot reloading** - Live editing of styles, layout, and logic
- **DSL for UI** - Makepad Style Language (MPSL) for declarative UI
- **No webview** - Native rendering, not Electron-style web wrapping
- **SIMD & GPU acceleration** - Optimized for performance-critical applications
- **AI/ML integration** - Built-in support for LLM inference (Moxin), VLM, TTS

### Example Usage

```rust
// Basic Makepad application
use makepad_widgets::*;

live_design! {
    import makepad::draw::*;
    import makepad::widgets::*;

    App = {{App}} {
        ui: Window = {
            body: View = {
                label = {
                    text: "Hello, Makepad!"
                }
                button = {
                    text: "Click me"
                    click: void
                }
            }
        }
    }
}

struct App {
    ui: WidgetRef,
    counter: u32,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        live_design!(cx, include_str!("app.rs"));
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Click = event {
            self.counter += 1;
            self.ui.set_text(&format!("Clicked: {}", self.counter));
            self.ui.redraw(cx);
        }
    }
}

fn main() {
    makepad_widgets::run_new(App::default()).unwrap();
}
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.Makerpad/
│
├── makepad/                           # Main Makepad UI toolkit
│   ├── audio_graph/                   # Audio synthesis & processing
│   │   ├── src/
│   │   │   ├── audio_graph.rs         # Audio node graph
│   │   │   ├── audio_stream.rs        # Real-time audio stream
│   │   │   ├── audio_traits.rs        # Audio trait definitions
│   │   │   ├── audio_unit_effect.rs   # Audio effects
│   │   │   ├── audio_unit_instrument.rs # Instruments
│   │   │   ├── instrument.rs          # Instrument trait
│   │   │   ├── mixer.rs               # Audio mixer
│   │   │   └── test_synth.rs          # Test synthesizer
│   │   └── audio_widgets/             # Audio UI widgets
│   │
│   ├── code_editor/                   # Code editor component
│   │   ├── src/
│   │   │   ├── code_editor.rs         # Main editor widget
│   │   │   ├── document.rs            # Document model
│   │   │   ├── tokenizer.rs           # Syntax highlighting
│   │   │   ├── layout.rs              # Text layout engine
│   │   │   └── widgets.rs             # Editor widgets
│   │
│   ├── draw/                          # 2D/3D drawing engine
│   │   ├── src/
│   │   │   ├── shader/                # GPU shaders
│   │   │   │   ├── draw_color.rs      # Color fills
│   │   │   │   ├── draw_line.rs       # Line drawing
│   │   │   │   ├── draw_text.rs       # Text rendering
│   │   │   │   ├── draw_quad.rs       # Quad rendering
│   │   │   │   └── draw_trapezoid.rs  # Trapezoid fills
│   │   │   ├── text/                  # Text engine
│   │   │   │   ├── font.rs            # Font handling
│   │   │   │   ├── font_atlas.rs      # Font atlases
│   │   │   │   └── font_face.rs       # Font faces
│   │   │   └── geometry/              # Geometry generation
│   │   └── vector/                    # Vector graphics (forked libs)
│   │       ├── bender/                # Path tessellation
│   │       │   ├── clipper/           # Path clipping
│   │       │   ├── filler/            # Path filling
│   │       │   ├── offsetter/         # Path offsetting
│   │       │   └── stroker/           # Path stroking
│   │
│   ├── examples/                      # Example applications
│   │   ├── chatgpt/                   # ChatGPT-like UI
│   │   ├── ironfish/                  # Synthesizer application
│   │   ├── fractal_zoom/              # GPU fractal renderer
│   │   ├── news_feed/                 # Social feed UI
│   │   ├── snake/                     # Snake game
│   │   ├── slides/                    # Presentation software
│   │   ├── teamtalk/                  # Video conferencing
│   │   ├── text_flow/                 # Text layout demo
│   │   ├── ui_zoo/                    # Widget showcase
│   │   ├── web_cam/                   # Webcam capture
│   │   └── websocket_image/           # WebSocket streaming
│   │
│   ├── libs/                          # vendored libraries
│   │   ├── html/                      # HTML parsing
│   │   ├── rustybuzz/                 # Text shaping (fork)
│   │   ├── ttf-parser/                # Font parsing (fork)
│   │   ├── ab_glyph_rasterizer/       # Glyph rasterization
│   │   ├── sdfer/                     # Signed distance field rendering
│   │   ├── zune-*                     # Image decoding
│   │   │   ├── zune-core/
│   │   │   ├── zune-inflate/
│   │   │   ├── zune-jpeg/
│   │   │   └── zune-png/
│   │   ├── stitch/                    # Hot reloading
│   │   └── wasm_bridge/               # WASM interop
│   │
│   ├── platform/                      # Platform abstraction
│   │   ├── src/
│   │   │   ├── android/               # Android backend
│   │   │   ├── apple/                 # iOS/macOS backend
│   │   │   ├── web/                   # Web/WASM backend
│   │   │   ├── windows/               # Windows backend
│   │   │   ├── linux/                 # Linux backend
│   │   │   └── turbo/                 # High-performance primitives
│   │
│   ├── studio/                        # Makepad Studio IDE
│   │   └── src/
│   │
│   ├── tools/
│   │   ├── cargo_makepad/             # Cargo subcommand for cross-compile
│   │   ├── web_server/                # Development server
│   │   ├── wasm_strip/                # WASM size optimization
│   │   └── shader-compiler/           # Shader compilation
│   │
│   └── widgets/                       # Widget library
│       ├── src/
│       │   ├── button.rs              # Button widget
│       │   ├── label.rs               # Label widget
│       │   ├── text_input.rs          # Text input
│       │   ├── scroll_view.rs         # Scrollable view
│       │   ├── tab_bar.rs             # Tab bar
│       │   └── ...
│
├── eyeball/                           # Observable types library
│   ├── eyeball/                       # Core Observable type
│   │   ├── src/
│   │   │   ├── lib.rs                 # Observable trait
│   │   │   ├── subscriber.rs          # Subscriber handling
│   │   │   └── read_guard.rs          # Read guards
│   ├── eyeball-im/                    # Observable collections
│   │   └── src/
│   │       ├── vector.rs              # ObservableVector
│   │       └── batch.rs               # Batch updates
│   └── eyeball-im-util/               # Utilities for eyeball-im
│
├── src.Moxin-Org/                     # Moxin AI models & tools
│   ├── Moxin-LLM/                     # LLM family (7B parameters)
│   │   ├── scripts/
│   │   └── README.md
│   ├── Moxin-VLM/                     # Vision-Language Model
│   ├── Moxin-TTS/                     # Text-to-Speech
│   ├── Moxin-XD/                      # Cross-modal models
│   ├── Ominix-SD.cpp/                 # Stable Diffusion (ggml)
│   │   └── ggml/
│   │       └── examples/
│   │           ├── whisper/           # Speech recognition
│   │           ├── gpt-2/             # Text generation
│   │           ├── mnist/             # MNIST inference
│   │           └── sam/               # Segmentation
│   └── mofa-docker-stack/             # Docker deployment
│
├── robrix/                            # Matrix chat client (Project Robius)
│   ├── src/
│   │   ├── room_list_service.rs       # Room listing
│   │   ├── timeline/                  # Timeline view
│   │   ├── auth/                      # Authentication
│   │   └── app.rs                     # Main application
│
├── glui/                              # Legacy UI system (predecessor to Makepad)
│   ├── shader_ast/                    # Shader AST
│   └── src/
│
├── image_viewer/                      # Image viewer tutorial series
│   ├── step_1/ through step_17/       # Step-by-step tutorial
│
├── experiments/                       # Experimental projects
│   ├── embedded/                      # Embedded Rust projects
│   │   ├── esp-term/                  # ESP32 terminal
│   │   ├── esp_chat/                  # ESP32 chat
│   │   ├── esp_car/                   # ESP32 RC car
│   │   └── makepad-logothing/         # Logo display device
│   ├── ai_mr/                         # AI/MR experiments
│   ├── xr_net/                        # XR networking
│   ├── vulkan/                        # Vulkan experiments
│   └── floating_elements/             # UI experiments
│
├── jsast/                             # JavaScript AST parser
│   └── src/
│
├── microserde/                        # Minimal serde implementation
│   └── src/
│
├── robius-authentication/             # Robius auth module
│   └── src/
│
├── robius-keychain/                   # Secure key storage
│   └── src/
│
├── robius-open/                       # URL/file opening
│   └── src/
│
├── robius-url-handler/                # URL scheme handling
│   └── src/
│
├── rustquest/                         # Learning/experimental project
│   └── src/
│
├── stitch/                            # Hot reload runtime
│   └── src/
│
├── uX/                                # Microcontroller framework
│   └── src/
│
├── wasm-index/                        # WASM indexing tool
│   └── src/
│
├── hello_quest/                       # VR/AR experiments
│   └── src/
│
├── html_experiment/                   # HTML rendering experiments
│   └── src/
│
├── irq_safety/                        # Interrupt-safe primitives
│   └── src/
│
├── android-build/                     # Android build configuration
│   └── src/
│
├── boiler/                            # Template/boilerplate project
│   └── src/
│
└── book/                              # Robius Book (documentation)
    └── src/
```

## Core Architecture

### Makepad Rendering Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                    Makepad Rendering Architecture                │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Widget Layer                               │ │
│  │  Buttons, Labels, TextInputs, ScrollViews, Tabs, etc.        │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                              │                                    │
│                              │ Live Design (MPSL DSL)            │
│                              ▼                                    │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Draw Layer                                 │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │ │
│  │  │   Shaders   │  │    Text     │  │     Geometry        │  │ │
│  │  │  (draw_*)   │  │  (fonts,    │  │  (paths, shapes)    │  │ │
│  │  │             │  │  shaping)   │  │                     │  │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                              │                                    │
│                              │ Platform Abstraction              │
│                              ▼                                    │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                   Platform Layer                              │ │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌─────────┐   │ │
│  │  │Windows │ │ macOS  │ │ Linux  │ │Android │ │  WASM   │   │ │
│  │  │(Win32) │ │(Metal) │ │ (GL)   │ │ (GL)   │ │(WebGL)  │   │ │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └─────────┘   │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Live Design System (Hot Reloading)

```rust
// Live design allows runtime UI changes
live_design! {
    import makepad::draw::*;
    import makepad::widgets::*;

    // Define styles inline with DSL
    Theme = {
        button = {
            default = {
                draw_bg: {
                    color: #3498db
                }
                draw_text: {
                    color: #ffffff
                }
            }
            hover = {
                draw_bg: {
                    color: #2980b9
                }
            }
        }
    }
}

// Changes to live_design blocks are picked up at runtime
// No recompilation needed for style/layout changes
```

### Eyeball Observability

```rust
use eyeball::{Observable, Subscriber};

// Create an observable value
let mut observable = Observable::new(initial_value);

// Subscribe to changes
let mut subscriber = observable.subscribe();

// Modify the value
observable.set(new_value);

// Receive updates (async polling)
while let Some(value) = subscriber.next().await {
    println!("Value changed: {:?}", value);
}

// Observable collections (eyeball-im)
use eyeball_im::ObservableVector;

let mut vector = ObservableVector::new();
let mut sub = vector.subscribe();

vector.push_back("item1");
vector.push_back("item2");

// Subscriber receives batch updates
if let Some(update) = sub.next().await {
    for op in update {
        match op {
            VectorDiff::Append { values } => { /* ... */ }
            VectorDiff::Insert { index, value } => { /* ... */ }
            VectorDiff::Remove { index } => { /* ... */ }
            // ... more operations
        }
    }
}
```

## Key Projects

### 1. Robrix - Matrix Chat Client

Robrix is a production-ready Matrix chat client demonstrating Makepad's capabilities:

**Features:**
- Room list with sliding sync
- Timeline view with message rendering
- User profiles and avatars
- Message sending/receiving
- Encryption support (via matrix-rust-sdk)
- Cross-platform (desktop, mobile)

**Architecture:**

```rust
// Robrix uses matrix-rust-sdk underneath
use matrix_sdk::{Client, ClientBuilder};
use eyeball_im::ObservableVector;

// Room list service
pub struct RoomListService {
    rooms: ObservableVector<RoomListItem>,
    client: Client,
}

impl RoomListService {
    pub async fn sync(&mut self) -> Result<()> {
        // Sliding sync with Matrix homeserver
        let response = self.client.sliding_sync().await?;

        // Update observable vector (triggers UI update)
        self.rooms.set(0, new_room);

        Ok(())
    }
}

// Timeline rendering
pub struct TimelineView {
    timeline: ObservableVector<TimelineItem>,
}

impl TimelineView {
    pub fn add_message(&mut self, message: Message) {
        self.timeline.push_back(TimelineItem::Message(message));
        // UI automatically updates via eyeball subscription
    }
}
```

### 2. Moly - AI LLM Client

Moly is an AI LLM client built with Makepad, showcasing AI/ML integration:

**Features:**
- Local LLM inference (Moxin models)
- Chat interface
- Model management
- Streaming responses
- Context management

**Moxin Models:**

| Model | Parameters | Purpose |
|-------|------------|---------|
| Moxin-7B-Base | 7B | Base language model |
| Moxin-7B-Chat | 7B | Chat-optimized (DPO) |
| Moxin-7B-Instruct | 7B | Instruction-tuned |
| Moxin-7B-Reasoning | 7B | Math/reasoning focused |
| Moxin-7B-VLM | 7B | Vision-language model |

**Performance benchmarks (Moxin-7B-Base):**

| Benchmark | Score |
|-----------|-------|
| ARC-C | 59.47 |
| HellaSwag | 83.08 |
| MMLU (5-shot) | 60.97 |
| Winogrande | 78.69 |
| Average | 70.55 |

### 3. IronFish - Synthesizer Application

IronFish is a feature-rich synthesizer demonstrating Makepad's audio capabilities:

```rust
use makepad_audio_graph::*;

// Audio graph nodes
struct SynthGraph {
    osc1: OscillatorNode,
    osc2: OscillatorNode,
    filter: FilterNode,
    envelope: EnvelopeNode,
    output: OutputNode,
}

impl AudioGraph for SynthGraph {
    fn process(&mut self, buffer: &mut AudioBuffer) {
        // Generate audio in real-time
        let osc1_signal = self.osc1.generate();
        let osc2_signal = self.osc2.generate();

        // Mix and filter
        let mixed = osc1_signal + osc2_signal;
        let filtered = self.filter.process(mixed);

        // Apply envelope
        let enveloped = self.envelope.apply(filtered);

        // Output
        *buffer = enveloped;
    }
}
```

### 4. Fractal Zoom - GPU Renderer

Demonstrates SIMD/WebWorker rendering for compute-intensive tasks:

```rust
// Tiled rendering with WebWorkers for WASM
use makepad_platform::web_worker::*;

struct FractalRenderer {
    workers: Vec<WebWorker>,
    tiles: Vec<TileBuffer>,
}

impl FractalRenderer {
    fn render_frame(&mut self, zoom: f64, offset: Vec2) {
        // Distribute tiles across workers
        for (i, worker) in self.workers.iter().enumerate() {
            worker.send(RenderCommand {
                tile_index: i,
                zoom,
                offset,
            });
        }

        // Collect results
        for worker in &self.workers {
            let tile = worker.recv::<Tile>();
            self.tiles.push(tile);
        }
    }
}
```

## Build System

### Cargo-makepad

Makepad provides a custom cargo subcommand for cross-platform builds:

```bash
# Install cargo-makepad
cargo install --path=./tools/cargo_makepad

# Install toolchains
cargo makepad wasm install-toolchain
cargo makepad apple ios install-toolchain
cargo makepad apple tvos install-toolchain
cargo makepad android --abi=all install-toolchain

# Build for different platforms
cargo makepad wasm run -p example --release
cargo makepad android run -p robrix --release
cargo makepad apple ios run-sim -p robrix --release
```

### Build Profiles

```toml
# Optimized for small WASM size
[profile.small]
inherits = "release"
opt-level = 'z'       # Optimize for size
lto = true            # Link-time optimization
codegen-units = 1     # Single codegen unit
panic = 'abort'       # Abort on panic
strip = true          # Strip symbols
```

### Hot Reloading

```rust
// Enable hot reloading in development
#[cfg(debug_assertions)]
use makepad_stitch::hot_reload;

fn main() {
    #[cfg(debug_assertions)]
    hot_reload::init();

    // Application code...
    // Changes to live_design! blocks are detected and reloaded at runtime
}
```

## Makepad Style Language (MPSL)

MPSL is a DSL for styling and layout:

```
// MPSL syntax example
Button = {
    draw_bg: {
        color: #3498db
        border_radius: 4.0
    }

    draw_text: {
        color: #ffffff
        font_size: 14.0
    }

    layout: {
        padding: {left: 16, right: 16, top: 8, bottom: 8}
    }

    text: "Click me"
    click: void
}

// Inheritance
PrimaryButton = <Button> {
    draw_bg: {
        color: #27ae60
    }
}
```

## Platform Support Matrix

| Host OS | Target Platform | Build Command | Status |
|---------|-----------------|---------------|--------|
| macOS | macOS | `cargo run` | ✅ Stable |
| macOS | iOS | `cargo makepad apple ios` | ✅ Stable |
| macOS | Android | `cargo makepad android` | ✅ Stable |
| macOS | WASM | `cargo makepad wasm` | ✅ Stable |
| Linux | Linux | `cargo run` | ✅ Stable |
| Linux | Android | `cargo makepad android` | ✅ Stable |
| Linux | WASM | `cargo makepad wasm` | ✅ Stable |
| Windows | Windows | `cargo run` | ✅ Stable |
| Windows | Android | `cargo makepad android` | ✅ Stable |
| Windows | WASM | `cargo makepad wasm` | ✅ Stable |

## Performance Characteristics

### WASM Size Optimization

```bash
# Typical WASM sizes (optimized)
makepad-studio:     ~2.5 MB (raw)
                   ~800 KB (gzipped)
                   ~600 KB (with small profile)

example-simple:     ~1.2 MB (raw)
                   ~400 KB (gzipped)
```

### Rendering Performance

- **Immediate mode** - No retained state overhead
- **GPU batched** - Minimal draw calls
- **SIMD text layout** - Parallel text shaping
- **Lazy evaluation** - Only render visible content

## Comparison with Alternatives

| Framework | Language | Platforms | Rendering | Bundle Size | Performance |
|-----------|----------|-----------|-----------|-------------|-------------|
| Makepad | Rust | All | Custom GPU | ~600KB | Excellent |
| Tauri | Rust + Web | Desktop | WebView | ~2MB+ | Good |
| Electron | JS | Desktop | WebView | ~100MB+ | Moderate |
| Flutter | Dart | All | Skia | ~4MB+ | Good |
| Iced | Rust | Desktop | GPU (wgpu) | ~1MB+ | Good |
| egui | Rust | All | GPU | ~500KB | Excellent |

## Trade-offs

| Design Choice | Benefit | Cost |
|---------------|---------|------|
| Immediate mode | Simple state management | Redraw overhead |
| Custom rendering | No native dependencies | More maintenance |
| MPSL DSL | Hot reloading | Learning curve |
| Nightly Rust | SIMD optimizations | Toolchain stability |
| Vendored libs | Consistent behavior | Larger repo, update lag |
| GPU-first | High performance | Fallback complexity |

## Related Projects

### In this Repository

- **eyeball** - Observable types for reactive UI
- **Moxin** - Open-source LLM family
- **glui** - Predecessor UI system
- **stitch** - Hot reload runtime
- **jsast** - JavaScript AST parser
- **microserde** - Minimal serialization

### External

- **matrix-rust-sdk** - Matrix protocol SDK (used by Robrix)
- **wasm-bindgen** - Rust/WASM interop
- **wgpu** - GPU abstraction (alternative rendering backend)
- **sliding-sync** - Matrix protocol extension

## References

- [Makepad GitHub](https://github.com/makepad/makepad)
- [Project Robius GitHub](https://github.com/project-robius)
- [Robius Book (Documentation)](https://project-robius.github.io/book/)
- [Makepad Discord](https://discord.gg/adqBRq7Ece)
- [Moxin LLM Technical Report](https://arxiv.org/abs/2412.06845)
- [Robrix GOSIM 2024 Talk](https://www.youtube.com/watch?v=DO5C7aITVyU)
