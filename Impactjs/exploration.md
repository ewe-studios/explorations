---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs
repository: https://github.com/phoboslab (various individual repos)
explored_at: 2026-03-20T00:00:00Z
language: JavaScript, C, C++, Zig, WebGL, GLSL
---

# Project Exploration: Impact.js Gaming Ecosystem

## Overview

This directory contains a collection of gaming-related projects from **Dominic Szablewski** (PhobosLab), a renowned developer known for pushing the boundaries of what's possible with JavaScript and web technologies. The ecosystem spans game engines, media codecs, graphics libraries, and experimental projects - all unified by a focus on performance, simplicity, and practical utility.

PhobosLab's work is characterized by:
- Single-file, header-only libraries (C/C++)
- Minimal dependencies
- High performance through algorithmic innovation
- Clean, readable source code
- MIT/permissive licensing

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/
├── Impact/              # Impact Game Engine (HTML5/JavaScript)
├── Ejecta/              # iOS JavaScript runtime (Native iOS app)
├── jsmpeg/              # MPEG1 video decoder in JavaScript
├── jsmpeg-vnc/          # Screen sharing over jsmpeg
├── qoi/                 # Quite OK Image format (C header)
├── qoa/                 # Quite OK Audio format (C header)
├── qop/                 # Quite OK Protocol (experimental)
├── sokol/               # Cross-platform graphics library
├── high_impact/         # C game engine (successor to Impact)
├── z_impact/            # Zig port of high_impact
├── wipeout-rewrite/     # WipEout (1995 PSX) re-implementation
├── q1k3/                # JS13k competition entry (2021)
├── pl_json/             # Simple JSON parser
└── pl_synth/            # Audio synthesizer library
```

## Architecture

### High-Level Ecosystem Diagram

```
                              ┌─────────────────────────────────────┐
                              │        PhobosLab Gaming Stack       │
                              └─────────────────────────────────────┘
                                           │
           ┌───────────────────────────────┼───────────────────────────────┐
           │                               │                               │
    ┌──────▼──────┐                 ┌──────▼──────┐                 ┌──────▼──────┐
    │   Impact    │                 │  high_impact│                 │   Ejecta    │
    │  (JS 2011)  │                 │   (C 2024)  │                 │ (iOS 2015)  │
    └──────┬──────┘                 └──────┬──────┘                 └──────┬──────┘
           │                               │                               │
           │                               │                               │
    ┌──────▼───────────────────────────────▼───────────────────────────────▼──────┐
    │                         Media Formats & Libraries                           │
    │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐   │
    │  │   QOI   │  │   QOA   │  │ jsmpeg  │  │ sokol   │  │ pl_synth/pl_json│   │
    │  │ (Image) │  │ (Audio) │  │ (Video) │  │(Graphics)│  │   (Utilities)   │   │
    │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────────────┘   │
    └─────────────────────────────────────────────────────────────────────────────┘
```

## Impact Game Engine

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/Impact/`

Impact is an HTML5 game engine published in 2011, designed for creating 2D games with JavaScript. It was one of the first practical JavaScript game engines and helped establish the HTML5 gaming scene.

### Core Architecture

```javascript
// Main entry point
ig.main(canvasId, gameClass, fps, width, height, scale)
  ├── ig.System  - Canvas, timing, render loop
  ├── ig.Input   - Keyboard, mouse, touch handling
  ├── ig.SoundManager - Audio playback
  └── ig.Loader  - Resource loading
```

### Entity System

Impact uses a class-based entity system with collision detection:

```javascript
ig.Entity.extend({
    // Properties
    size: {x: 16, y: 16},      // Bounding box
    pos: {x: 0, y: 0},         // Position
    vel: {x: 0, y: 0},         // Velocity
    accel: {x: 0, y: 0},       // Acceleration
    friction: {x: 0, y: 0},    // Friction
    maxVel: {x: 100, y: 100},  // Max velocity
    health: 10,

    // Collision types
    collides: ig.Entity.COLLIDES.ACTIVE,  // ACTIVE, PASSIVE, LITE, FIXED, NEVER
    type: ig.Entity.TYPE.A,               // For entity checks
    checkAgainst: ig.Entity.TYPE.B,

    // Methods
    init(x, y, settings) {},
    update() {},           // Called each frame
    draw() {},             // Render entity
    check(other) {},       // Overlap check
    collideWith(other, axis) {},  // Collision response
    receiveDamage(amount, from) {},
    kill() {}
});
```

### Collision System

Impact features a sophisticated collision system:

1. **Collision Map** - Tile-based collision with slope support
2. **Entity vs Entity** - Spatial hash grid for efficient pair checking
3. **Collision Types:**
   - `NEVER` - No collision
   - `LITE` - Weak collision (other entity doesn't move)
   - `PASSIVE` - Normal collision
   - `ACTIVE` - Strong collision (both entities move)
   - `FIXED` - Immovable object

### Collision Map with Slopes

The collision map supports sloped tiles through line definitions:

```javascript
// Default tile definition format: [x1, y1, x2, y2, solid]
ig.CollisionMap.defaultTileDef = {
    /* 45 degree slope */  2: [0,1, 1,0, SOLID],
    /* 22 degree slope */  3: [0,1, 1,H, SOLID],  // H = 1/2
    /* Non-solid line */  12: [0,0, 1,0, NON_SOLID]
};
```

The `trace()` function breaks movement into sub-steps to handle high velocities:

```javascript
// Trace movement through collision map
var res = collisionMap.trace(x, y, vx, vy, width, height);
// Returns: {collision: {x, y, slope}, pos: {x, y}, tile: {x, y}}
```

### Game Loop

```javascript
ig.Game = ig.Class.extend({
    entities: [],
    collisionMap: null,
    backgroundMaps: [],
    gravity: 0,

    loadLevel(data) { /* Load entities and maps */ },
    update() {
        this.updateEntities();  // Update all entities
        this.checkEntities();   // Spatial hash collision checks
        this._deferredKill = []; // Remove killed entities
    },
    draw() {
        // Draw background maps (non-foreground)
        // Draw entities
        // Draw background maps (foreground)
    }
});
```

### Weltmeister Level Editor

Impact includes `weltmeister.html`, a browser-based level editor that:
- Edits tile layers
- Places entities with custom settings
- Exports JSON level files
- Supports tile collision editing

### Module System

Impact has a custom module loader:

```javascript
ig.module('impact.entity')
    .requires('impact.animation', 'impact.impact')
    .defines(function() {
        // Module code here
    });
```

---

## Ejecta Runtime

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/Ejecta/`

Ejecta is a fast, open-source JavaScript, Canvas, and Audio implementation for iOS (iPhone, iPad, Apple TV). Think of it as a browser that can only display a Canvas element.

### Key Features

- **JavaScriptCore Integration** - Uses iOS's built-in JSC engine
- **Canvas2D Implementation** - Native OpenGL ES rendering
- **WebGL Support** - Full WebGL implementation alongside Canvas2D
- **Audio** - CoreAudio-based audio playback
- **Input** - Touch, accelerometer support

### Architecture

```
┌─────────────────────────────────────┐
│         Ejecta (Native iOS)         │
├─────────────────────────────────────┤
│  Objective-C + JavaScriptCore       │
│  ├── Canvas Context (OpenGL ES)     │
│  ├── Audio (CoreAudio)              │
│  ├── Input (UIKit touches)          │
│  └── File System Access             │
├─────────────────────────────────────┤
│         User's JavaScript           │
│         (./App/index.js)            │
└─────────────────────────────────────┘
```

### Canvas2D Implementation

Ejecta provides a native Canvas2D context that maps to OpenGL ES:

```javascript
var canvas = document.getElementById('canvas');
var ctx = canvas.getContext('2d', {antialias: true, antialiasSamples: 4});

// All standard Canvas2D methods supported
ctx.fillRect();
ctx.beginPath();
ctx.bezierCurveTo();
ctx.drawImage();
ctx.getImageData();  // Note: Typed Array performance considerations
```

### Performance Considerations

- **Typed Arrays** - Read/write performance varies by iOS version
- **Retina Display** - Manual handling required (no automatic pixel doubling since 2015)
- **Orientation** - Automatic rotation with resize events

```javascript
// Handle orientation changes
window.addEventListener('resize', function() {
    console.log('new size:', window.innerWidth, window.innerHeight);
});
```

---

## QOI Image Format

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/qoi/`

QOI (Quite OK Image) is a fast, lossless image compression format that offers 20x-50x faster encoding and 3x-4x faster decoding than PNG/stb_image, with 20% better compression.

### Design Philosophy

QOI trades compression ratio for **extreme speed** while maintaining reasonable compression. The format is:
- **Lossless** - Perfect reconstruction
- **Simple** - ~400 lines of C code
- **Fast** - Stream encoding/decoding
- **Royalty-free** - Public domain / MIT

### File Format

```
Header (14 bytes):
├── magic[4]    = "qoif"
├── width[4]    = Big-endian uint32
├── height[4]   = Big-endian uint32
├── channels[1] = 3 (RGB) or 4 (RGBA)
└── colorspace[1] = 0 (sRGB) or 1 (linear)

Chunks (variable):
└── End marker: 7 x 0x00 + 1 x 0x01
```

### Chunk Types

```
QOI_OP_INDEX (0b00xxxxxx) - 6-bit index into seen-pixels array
    Index = (r*3 + g*5 + b*7 + a*11) % 64

QOI_OP_DIFF (0b01xxxxxx) - 2-bit differences per channel
    dr, dg, db each in range -2..1

QOI_OP_LUMA (0b10xxxxxx) - Green diff + red/blue offset
    6-bit green diff (-32..31)
    4-bit dr-dg, db-dg (-8..7)

QOI_OP_RUN (0b11xxxxxx) - Repeat previous pixel 1-62 times

QOI_OP_RGB (0b11111110) - Full RGB values
QOI_OP_RGBA (0b11111111) - Full RGBA values
```

### Encoder Core Algorithm

```c
qoi_rgba_t index[64];  // Hash table of seen pixels

for each pixel px:
    if (px == px_prev) {
        run++;
        if (run == 62) emit QOI_OP_RUN | 61;
    } else {
        if (run > 0) emit QOI_OP_RUN | (run-1);

        hash = QOI_COLOR_HASH(px) % 64;
        if (index[hash] == px) {
            emit QOI_OP_INDEX | hash;
        } else {
            index[hash] = px;
            if (px.a == px_prev.a) {
                dr = px.r - px_prev.r;  // Range -2..1?
                dg = px.g - px_prev.g;
                db = px.b - px_prev.b;

                if (small differences) {
                    emit QOI_OP_DIFF | dr+2 | dg+2 | db+2;
                } else if (luma-predictable) {
                    emit QOI_OP_LUMA | dg+32;
                    emit (dr-dg)+8 | (db-dg)+8;
                } else {
                    emit QOI_OP_RGB;
                    emit px.r, px.g, px.b;
                }
            } else {
                emit QOI_OP_RGBA;
                emit px.r, px.g, px.b, px.a;
            }
        }
    }
    px_prev = px;
```

### Why QOI Works

1. **Index Cache** - Reuses recently seen colors (great for gradients)
2. **Difference Encoding** - Small changes use fewer bits
3. **LUMA Prediction** - Green channel predicts red/blue changes
4. **Run Length** - Consecutive identical pixels compressed

### Adoption

QOI is now supported in:
- FFmpeg, ImageMagick, GIMP 3.0
- Godot Engine, Raylib
- Windows Explorer, macOS QuickLook
- Dozens of language bindings (Rust, Go, Python, etc.)

---

## QOA Audio Format

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/qoa/`

QOA (Quite OK Audio) is a lossy audio compression format achieving 5:1 compression (16-bit PCM to 3.2 bits/sample) with transparent quality for many audio types.

### Design Goals

- **Simple** - Single header, ~500 lines
- **Fast** - Real-time encoding/decoding
- **Lossy but Transparent** - Inaudible difference for most content
- **Low Latency** - Suitable for games

### File Format

```
File Header (8 bytes):
├── magic[4] = "qoaf"
└── samples[4] = Total samples per channel

Frames (variable):
├── Frame Header (8 bytes)
│   ├── channels[1]
│   ├── samplerate[3]
│   ├── frame_samples[2]
│   └── frame_size[2]
├── LMS State (16 bytes per channel)
│   ├── history[4] = Last 4 samples
│   └── weights[4] = LMS filter weights
└── Slices (256 per channel, 8 bytes each)
    ├── scalefactor[4 bits]
    └── residuals[20 x 3 bits]
```

### Slice Structure (64 bits = 20 samples)

```
┌────────────────────────────────────────────────────────┐
│ Byte[0] │ Byte[1] │ Byte[2] │ ... │ Byte[7]           │
├─────────┼─────────┼─────────┼─────┼───────────────────┤
│ sf_quant│  qr00   │  qr01   │ ... │  qr19 (partial)   │
│  (4bit) │ (3bits) │ (3bits) │     │  (3bits)          │
└─────────┴─────────┴─────────┴─────┴───────────────────┘
```

### LMS Prediction

QOA uses a Sign-Sign Least Mean Squares filter to predict each sample:

```c
// Predict next sample from 4 previous samples
int qoa_lms_predict(qoa_lms_t *lms) {
    int prediction = 0;
    for (int i = 0; i < 4; i++) {
        prediction += lms->weights[i] * lms->history[i];
    }
    return prediction >> 13;  // Fixed-point scaling
}

// Update weights based on residual
void qoa_lms_update(qoa_lms_t *lms, int sample, int residual) {
    int delta = residual >> 4;
    for (int i = 0; i < 4; i++) {
        lms->weights[i] += lms->history[i] < 0 ? -delta : delta;
    }
    // Shift history
    for (int i = 0; i < 3; i++) {
        lms->history[i] = lms->history[i+1];
    }
    lms->history[3] = sample;
}
```

### Encoding Process

1. For each slice, try all 16 scalefactors
2. For each scalefactor:
   - Predict each sample using LMS
   - Calculate residual = sample - predicted
   - Scale and quantize residual to 3 bits
   - Calculate total squared error
3. Choose scalefactor with lowest error (+ weight penalty)
4. Output 8-byte slice

### Quantization Table

```c
// Maps residuals -8..8 to 3-bit indices
static const int qoa_quant_tab[17] = {
    7, 7, 7, 5, 5, 3, 3, 1,  // -8..-1
    0,                        // 0
    0, 2, 2, 4, 4, 6, 6, 6   // 1..8
};

// Dequantization (scalefactor x quant_index -> value)
static const int qoa_dequant_tab[16][8] = {
    { 1, -1, 3, -3, 5, -5, 7, -7},  // sf=0
    { 5, -5, 18, -18, ...},          // sf=1
    // ... 16 scalefactors
};
```

### Adoption

- Godot Engine 4.3+ (built-in QOA support)
- Raylib (raudio module)
- SerenityOS (system-wide support)

---

## jsmpeg Video Decoder

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/jsmpeg/`

jsmpeg is an MPEG1 video and MP2 audio decoder written entirely in JavaScript. It can decode 720p at 30fps on an iPhone 5S in just 20KB gzipped.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    JSMpeg.Player                        │
├─────────────────────────────────────────────────────────┤
│  Source Layer                                           │
│  ├── AJAX (static files)                                │
│  ├── WebSocket (streaming)                              │
│  └── Fetch API                                          │
├─────────────────────────────────────────────────────────┤
│  Demuxer (TS - Transport Stream)                        │
├─────────────────────────────────────────────────────────┤
│  Decoders                                               │
│  ├── MPEG1 Video Decoder                                │
│  └── MP2 Audio Decoder                                  │
├─────────────────────────────────────────────────────────┤
│  Output                                                 │
│  ├── WebGL Renderer (GPU-accelerated)                   │
│  ├── Canvas2D Renderer (fallback)                       │
│  └── WebAudio Output                                    │
└─────────────────────────────────────────────────────────┘
```

### MPEG1 Video Decoding

The decoder handles:
- I-frames (intra-coded)
- P-frames (predicted from previous)
- B-frames (bi-directional, not fully supported)

Key components:
- **Variable Length Decode (VLD)** - Huffman decoding
- **Inverse DCT** - Frequency to spatial domain
- **Motion Compensation** - Apply motion vectors
- **Color Conversion** - YCrCb to RGB

### WebSocket Streaming

jsmpeg achieves ultra-low latency (~50ms) by:
1. Discarding timestamps (plays immediately)
2. Small buffers (512KB video, 128KB audio)
3. Immediate decoding on data arrival

```javascript
var player = new JSMpeg.Player('ws://server:8080', {
    video: true,
    audio: true,
    maxAudioLag: 0.5,
    onVideoDecode: function(decoder, time) { /* ... */ }
});
```

### FFmpeg Encoding

```bash
# Basic encoding
ffmpeg -i input.mp4 -f mpegts -codec:v mpeg1video -codec:a mp2 output.ts

# Low latency streaming
ffmpeg -i input \
    -f mpegts \
    -codec:v mpeg1video -s 960x540 -b:v 1500k -r 30 -bf 0 \
    -codec:a mp2 -ar 44100 -ac 1 -b:a 128k \
    -muxdelay 0.001 \
    output.ts
```

---

## Sokol Graphics Library

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/sokol/`

Sokol is a collection of cross-platform, single-file C libraries for graphics, audio, input, and application lifecycle management.

### Core Headers

| Header | Purpose | Size |
|--------|---------|------|
| `sokol_gfx.h` | 3D graphics abstraction (GL/Metal/D3D/WebGPU) | 968KB |
| `sokol_app.h` | App framework (window, context, input) | 488KB |
| `sokol_audio.h` | Audio streaming | 92KB |
| `sokol_time.h` | Time measurement | 11KB |
| `sokol_fetch.h` | Async file loading | 118KB |
| `sokol_args.h` | Unified argument parsing | 26KB |
| `sokol_log.h` | Logging callbacks | 12KB |

### Graphics Abstraction

```c
// sokol_gfx.h abstracts multiple backends:
// - OpenGL 3.3 (GLX/WGL)
// - GLES3 / WebGL2
// - Metal (macOS, iOS)
// - D3D11 (Windows)
// - WebGPU (emerging)

sg_setup(&(sg_desc){
    .environment = sglue_environment(),  // From sokol_glue.h
    .logger.func = slog_func
});
```

### Minimal Triangle Example

```c
#include "sokol_app.h"
#include "sokol_gfx.h"
#include "sokol_log.h"

static struct {
    sg_pipeline pip;
    sg_bindings bind;
} state;

void init(void) {
    sg_setup(&(sg_desc){.logger.func = slog_func});

    float vertices[] = {
         0.0f,  0.5f, 0.5f,  1.0f, 0.0f, 0.0f, 1.0f,
         0.5f, -0.5f, 0.5f,  0.0f, 1.0f, 0.0f, 1.0f,
        -0.5f, -0.5f, 0.5f,  0.0f, 0.0f, 1.0f, 1.0f
    };

    state.bind.vertex_buffers[0] = sg_make_buffer(&(sg_buffer_desc){
        .data = SG_RANGE(vertices)
    });

    state.pip = sg_make_pipeline(&(sg_pipeline_desc){
        .shader = sg_make_shader(my_shader_desc(sg_query_backend()))
    });
}

void frame(void) {
    sg_begin_pass(&(sg_pass){.action = {
        .colors[0] = {.load_action=SG_LOADACTION_CLEAR,
                      .clear_value={0.5f, 0.5f, 0.5f, 1.0f}}
    }});
    sg_apply_pipeline(state.pip);
    sg_apply_bindings(&state.bind);
    sg_draw(0, 3, 1);
    sg_end_pass();
    sg_commit();
}
```

### Language Bindings

Official bindings (auto-generated):
- Zig (`sokol-zig`)
- Odin (`sokol-odin`)
- Nim (`sokol-nim`)
- Rust (`sokol-rust`)
- D (`sokol-d`)
- Jai (`sokol-jai`)
- C3 (`sokol-c3`)

---

## high_impact Engine

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/high_impact/`

high_impact is a C game engine for 2D action games, representing a spiritual successor to the JavaScript Impact engine.

### Key Differences from Impact

| Aspect | Impact (JS) | high_impact (C) |
|--------|-------------|-----------------|
| Language | JavaScript | C99 |
| Platform | Web browsers | Native (SDL2/Sokol) + WASM |
| Asset Format | PNG | QOI |
| Audio Format | WAV/MP3 | QOA |
| Renderer | Canvas2D/WebGL | OpenGL, Software |
| Level Editor | Built-in (Weltmeister) | Included (weltmeister.html) |

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Game Code                            │
├─────────────────────────────────────────────────────────┤
│                    high_impact                          │
├───────────────┬───────────────┬─────────────────────────┤
│   Platform    │   Renderer    │    Core Engine          │
│   ├── SDL2    │   ├── OpenGL  │    ├── Entity System   │
│   └── Sokol   │   ├── Metal   │    ├── Collision       │
│               │   └── Software│    ├── Animation       │
│               │               │    └── World Loading   │
├───────────────┴───────────────┴─────────────────────────┤
│              Libraries (bundled)                        │
│  QOI, QOA, pl_json, stb_image, glad, sokol_*           │
└─────────────────────────────────────────────────────────┘
```

### Building

```bash
# SDL2 + OpenGL
make sdl

# Sokol (native platforms)
make sokol

# WASM
make wasm
```

### Example Games

- **Biolab Disaster** - Jump'n'gun platformer
- **Drop** - Minimal arcade game with procedural generation

---

## z_impact Engine

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/z_impact/`

z_impact is a Zig port of high_impact, bringing the engine to the Zig ecosystem.

### Features

- Direct port of high_impact architecture
- Sokol bindings via `sokol-zig`
- SDL bindings via `SDL.zig`
- Native QOI/QOA support

### Building

```bash
# SDL2 + OpenGL
zig build run

# Sokol (platform-native renderer)
zig build -Dplatform=sokol run

# WASM
zig build -Dtarget=wasm32-emscripten run
```

---

## wipeout-rewrite

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/wipeout-rewrite/`

A re-implementation of the 1995 PSX game wipEout, featuring:
- Original game logic recreated
- Modern rendering with OpenGL
- QOA audio format for music
- MPEG1 intro video (via jsmpeg)

### Architecture

```
┌────────────────────────────────────────────────────────┐
│                  wipeout-rewrite                       │
├──────────────┬─────────────────────────────────────────┤
│   Platform   │              Core Game                  │
│   ├── SDL2   │    ├── 3D Rendering (track, ships)     │
│   └── Sokol  │    ├── Physics (ship movement)         │
│              │    ├── AI (enemy ships)                │
│              │    ├── Audio (QOA music, SFX)          │
│              │    └── Menus                           │
└──────────────┴─────────────────────────────────────────┘
```

### Build Options

| Flag | Description | Options |
|------|-------------|---------|
| `PLATFORM` | Platform backend | SDL2, SOKOL |
| `RENDERER` | Graphics backend | GL, GLES2, SOFTWARE |
| `MINIMAL_BUNDLE` | Web build size | ON, OFF |

---

## q1k3 - JS13k Entry

**Location:** `/home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/q1k3/`

A first-person dungeon crawler created for the 2021 JS13k competition (13KB limit).

### Features

- 2 levels
- 5 enemy types
- 3 weapons
- 30 textures
- Dynamic lighting
- Raycasting engine (Wolfenstein 3D style)
- Sonant-X based music generation

### Technical Achievements

- **Map Compiler** - C program to compile TrenchBroom maps
- **Texture Generation** - Tiny Texture Tumbler (TTT)
- **Compression** - UglifyJS + Roadroller
- **Audio** - Modified Sonant-X synthesizer

---

## Key Insights

### 1. Simplicity Through Constraints

All PhobosLab projects embrace constraints:
- **Single-file** headers for C libraries
- **No external dependencies** where possible
- **Readable code** over clever optimizations

### 2. Performance via Algorithm Design

Rather than micro-optimizations, the focus is on algorithmic choices:
- **QOI**: Hash-based pixel caching instead of complex transforms
- **QOA**: Simple LMS prediction instead of FFT
- **jsmpeg**: Direct MPEG1 decoding without intermediate formats

### 3. Ecosystem Integration

The projects interconnect:
- high_impact uses QOI, QOA, sokol
- sokol provides graphics for high_impact, wipeout-rewrite
- jsmpeg handles video in wipeout-rewrite
- Ejecta brings Impact games to iOS

### 4. Evolution Path

```
Impact (2011, JS) → Ejecta (2015, iOS) → high_impact (2024, C) → z_impact (2024, Zig)
                              ↓
                    QOI (2021) → QOA (2023)
                              ↓
                        sokol (ongoing)
```

### 5. Practical Game Development

All tools are battle-tested in actual games:
- Impact: Multiple commercial games
- jsmpeg: Used for streaming and embedded video
- QOI/QOA: Adopted by major engines (Godot, raylib)
- wipeout-rewrite: Full commercial game recreation

---

## References

- **PhobosLab Blog**: https://phoboslab.org/log
- **QOI Format**: https://qoiformat.org
- **QOA Format**: https://qoaformat.org
- **jsmpeg**: https://jsmpeg.com
- **Impact Engine**: https://impactjs.com

---

## Summary Table

| Project | Language | Lines | Purpose | Year |
|---------|----------|-------|---------|------|
| Impact | JavaScript | ~3000 | 2D Game Engine | 2011 |
| Ejecta | Obj-C/JS | ~5000 | iOS JS Runtime | 2015 |
| jsmpeg | JavaScript | ~2500 | MPEG1 Decoder | 2015 |
| QOI | C (header) | ~650 | Image Format | 2021 |
| QOA | C (header) | ~750 | Audio Format | 2023 |
| sokol | C (header) | ~40000 | Graphics API | 2018+ |
| high_impact | C | ~4000 | 2D Game Engine | 2024 |
| z_impact | Zig | ~4000 | 2D Game Engine | 2024 |
| wipeout-rewrite | C | ~15000 | Game Recreation | 2023 |
| q1k3 | JavaScript | ~8000 | FPS Game (13KB) | 2021 |
