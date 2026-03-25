# Rive WASM Web Rendering

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/rive-wasm/`

---

## Table of Contents

1. [Overview](#overview)
2. [WASM Architecture](#wasm-architecture)
3. [Memory Management](#memory-management)
4. [JavaScript Binding Layer](#javascript-binding-layer)
5. [Renderer Backends](#renderer-backends)
6. [Performance Optimizations](#performance-optimizations)
7. [Usage Examples](#usage-examples)

---

## Overview

Rive's WebAssembly (WASM) runtime enables high-performance vector animation playback in web browsers. The WASM module contains the C++ core runtime compiled to WASM, with JavaScript bindings for web integration.

### Key Files

| File | Purpose | Lines |
|------|---------|-------|
| `wasm/src/bindings.cpp` | Main WASM bindings | ~1,600 |
| `wasm/src/bindings_c2d.cpp` | Canvas 2D bindings | ~500 |
| `wasm/src/bindings_webgl2.cpp` | WebGL2 bindings | ~480 |
| `wasm/src/bindings_skia.cpp` | Skia bindings | ~180 |
| `js/src/rive.ts` | TypeScript API layer | ~600 |
| `CHANGELOG.md` | Version history | ~7,000 |

### Package Structure

```
@rive-app/canvas              # High-level Canvas 2D API (recommended)
@rive-app/webgl               # High-level WebGL API
@rive-app/canvas-advanced     # Low-level Canvas 2D API
@rive-app/webgl-advanced      # Low-level WebGL API
@rive-app/canvas-single       # Single-file bundle (WASM embedded)
@rive-app/webgl-single        # Single-file bundle (WASM embedded)
```

---

## WASM Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Web Application                              │
│                    (JavaScript/TypeScript)                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  JavaScript API Layer                            │
│  - Rive class                                                   │
│  - StateMachineController                                       │
│  - Event listeners                                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (WASM imports/exports)
┌─────────────────────────────────────────────────────────────────┐
│                    WASM Module                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              C++ Runtime (Compiled)                       │   │
│  │  - File loading & parsing                                 │   │
│  │  - Artboard hierarchy                                     │   │
│  │  - Animation system                                       │   │
│  │  - Renderer abstraction                                   │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              WebGL/Canvas Bindings                        │   │
│  │  - GL bindings (bindings_webgl2.cpp)                      │   │
│  │  - Canvas 2D bindings (bindings_c2d.cpp)                  │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Browser APIs                                  │
│  - WebGLRenderingContext                                        │
│  - CanvasRenderingContext2D                                     │
│  - OffscreenCanvas                                              │
└─────────────────────────────────────────────────────────────────┘
```

### Build Process

```bash
# Build WASM module
cd rive-wasm/wasm
./build_wasm.sh release

# Uses Emscripten toolchain:
# C++ → LLVM IR → WASM (.wasm) + JS glue (.js)
```

### WASM Module Exports

```cpp
// From bindings.cpp - WASM exports

// File operations
EMSCRIPTEN_KEEPALIVE
void* rive_load_file(const uint8_t* data, uint32_t length);

EMSCRIPTEN_KEEPALIVE
void rive_file_delete(void* file);

// Artboard operations
EMSCRIPTEN_KEEPALIVE
void* rive_file_artboard_by_name(void* file, const char* name);

EMSCRIPTEN_KEEPALIVE
void rive_artboard_draw(void* artboard, void* renderer);

// Animation operations
EMSCRIPTEN_KEEPALIVE
void* rive_artboard_animation_by_name(void* artboard, const char* name);

EMSCRIPTEN_KEEPALIVE
void* rive_animation_create_instance(void* animation);

EMSCRIPTEN_KEEPALIVE
void rive_animation_instance_update(void* instance, float elapsedSeconds);

// State machine operations
EMSCRIPTEN_KEEPALIVE
void* rive_artboard_state_machine_by_name(void* artboard, const char* name);

EMSCRIPTEN_KEEPALIVE
void rive_state_machine_set_bool_input(void* sm, const char* name, bool value);
```

---

## Memory Management

### WASM Linear Memory

WASM uses a single linear memory array that both C++ and JavaScript can access:

```
WASM Memory Layout:

┌─────────────────────────────────────────────────────────────┐
│  0x0000  │  Stack (grows down)                              │
│          │  ...                                             │
│          ├──────────────────────────────────────────────────│
│          │  Heap (dynamic allocations)                      │
│          │  ...                                             │
│          ├──────────────────────────────────────────────────│
│  0x10000 │  Static data (globals, strings)                  │
│          │  ...                                             │
│          ├──────────────────────────────────────────────────│
│          │  Imports table                                   │
└─────────────────────────────────────────────────────────────┘
```

### Memory Transfer Between JS and WASM

```javascript
// JavaScript side memory access
const wasmMemory = wasmModule.exports.memory;
const heap = new Uint8Array(wasmMemory.buffer);

// Write data to WASM memory
function writeString(str) {
    const ptr = wasmModule.exports.malloc(str.length + 1);
    for (let i = 0; i < str.length; i++) {
        heap[ptr + i] = str.charCodeAt(i);
    }
    heap[ptr + str.length] = 0; // Null terminator
    return ptr;
}

// Read data from WASM memory
function readString(ptr, length) {
    const bytes = heap.subarray(ptr, ptr + length);
    return new TextDecoder().decode(bytes);
}
```

### Automatic Memory Management

```cpp
// From bindings.cpp - Reference counting

class WASMWrapper {
    void* m_Ptr;

public:
    WASMWrapper(void* ptr) : m_Ptr(ptr) {}

    ~WASMWrapper() {
        // Automatic cleanup when JS garbage collects
        if (m_Ptr) {
            delete m_Ptr;
        }
    }

    void* get() const { return m_Ptr; }
};

// Finalizer for JS garbage collection
EMSCRIPTEN_KEEPALIVE
void rive_finalize(void* ptr) {
    delete ptr;
}
```

### JavaScript FinalizationRegistry

```javascript
// From js/src/rive.ts

const registry = new FinalizationRegistry((ptr) => {
    // Called when JS object is garbage collected
    wasmModule.exports.rive_finalize(ptr);
});

class Rive {
    constructor(ptr) {
        this.ptr = ptr;
        registry.register(this, ptr);
    }

    delete() {
        // Explicit cleanup
        wasmModule.exports.rive_finalize(this.ptr);
        registry.unregister(this.ptr);
        this.ptr = null;
    }
}
```

---

## JavaScript Binding Layer

### High-Level API

```javascript
// From js/src/rive.ts - Simplified

class Rive {
    constructor(options) {
        this.canvas = options.canvas;
        this.src = options.src;
        this.onLoad = options.onLoad;
        this.onStateChange = options.onStateChange;

        this.load();
    }

    async load() {
        // Fetch .riv file
        const response = await fetch(this.src);
        const buffer = await response.arrayBuffer();

        // Load into WASM
        const ptr = wasmModule.exports.rive_load_file(
            new Uint8Array(buffer),
            buffer.byteLength
        );

        this.file = new File(ptr);
        this.artboard = this.file.artboard();

        // Create renderer
        this.renderer = this.createRenderer();

        // Start render loop
        this.startRendering();

        this.onLoad?.({ file: this.file });
    }

    createRenderer() {
        if (this.useWebGL) {
            return new WebGLRenderer(this.canvas);
        }
        return new Canvas2DRenderer(this.canvas);
    }

    startRendering() {
        const render = (time) => {
            const elapsed = (time - this.lastTime) / 1000;
            this.lastTime = time;

            // Update animations
            this.artboard.advance(elapsed);

            // Render
            this.renderer.save();
            this.artboard.draw(this.renderer);
            this.renderer.restore();

            requestAnimationFrame(render);
        };

        requestAnimationFrame(render);
    }
}
```

### State Machine Controller

```javascript
// From js/src/rive.ts

class StateMachineController {
    constructor(rive, artboard, stateMachineName) {
        this.rive = rive;
        this.artboard = artboard;

        // Get state machine from artboard
        this.stateMachine = artboard.stateMachine(stateMachineName);

        // Cache inputs
        this.inputs = {
            bool: {},
            number: {},
            trigger: {}
        };

        this.cacheInputs();
    }

    cacheInputs() {
        const inputCount = this.stateMachine.inputCount();

        for (let i = 0; i < inputCount; i++) {
            const input = this.stateMachine.input(i);
            switch (input.type) {
                case InputType.Bool:
                    this.inputs.bool[input.name] = input;
                    break;
                case InputType.Number:
                    this.inputs.number[input.name] = input;
                    break;
                case InputType.Trigger:
                    this.inputs.trigger[input.name] = input;
                    break;
            }
        }
    }

    setInput(name, value) {
        // Set boolean input
        if (name in this.inputs.bool) {
            this.inputs.bool[name].value = value;
            return;
        }

        // Set number input
        if (name in this.inputs.number) {
            this.inputs.number[name].value = value;
            return;
        }
    }

    fire(name) {
        // Fire trigger
        if (name in this.inputs.trigger) {
            this.inputs.trigger[name].fire();
        }
    }

    advance(elapsed) {
        this.stateMachine.advance(elapsed);
    }
}
```

### Event System

```javascript
// Event listener registration

class Rive {
    constructor(options) {
        // Register event listeners
        this.eventListeners = new Map();

        // Create WASM listener callbacks
        this.listenerPtr = wasmModule.exports.create_listener(
            (eventId, eventData) => {
                this.handleEvent(eventId, eventData);
            }
        );
    }

    on(eventName, callback) {
        if (!this.eventListeners.has(eventName)) {
            this.eventListeners.set(eventName, []);
        }
        this.eventListeners.get(eventName).push(callback);
    }

    handleEvent(eventId, eventData) {
        const eventName = this.getEventName(eventId);
        const listeners = this.eventListeners.get(eventName) || [];

        for (const callback of listeners) {
            callback({ type: eventName, data: eventData });
        }
    }
}
```

---

## Renderer Backends

### Canvas 2D Renderer

```javascript
// From js/src/renderer_2d.ts

class Canvas2DRenderer {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
    }

    // Implement WASM renderer interface
    save() {
        this.ctx.save();
    }

    restore() {
        this.ctx.restore();
    }

    transform(matrix) {
        // Apply 3x3 matrix to canvas context
        this.ctx.transform(
            matrix[0], matrix[1],
            matrix[2], matrix[3],
            matrix[4], matrix[5]
        );
    }

    drawPath(pathPtr, paintPtr) {
        // Read path commands from WASM memory
        const commands = readPathCommands(pathPtr);

        this.ctx.beginPath();
        for (const cmd of commands) {
            switch (cmd.type) {
                case 'move':
                    this.ctx.moveTo(cmd.x, cmd.y);
                    break;
                case 'line':
                    this.ctx.lineTo(cmd.x, cmd.y);
                    break;
                case 'cubic':
                    this.ctx.bezierCurveTo(
                        cmd.c1x, cmd.c1y,
                        cmd.c2x, cmd.c2y,
                        cmd.x, cmd.y
                    );
                    break;
                case 'close':
                    this.ctx.closePath();
                    break;
            }
        }

        // Apply paint
        if (paintPtr.style === 'fill') {
            this.ctx.fillStyle = paintPtr.color;
            this.ctx.fill();
        } else {
            this.ctx.strokeStyle = paintPtr.color;
            this.ctx.lineWidth = paintPtr.strokeWidth;
            this.ctx.stroke();
        }
    }

    clipPath(pathPtr) {
        const commands = readPathCommands(pathPtr);
        this.ctx.beginPath();
        // ... rebuild path ...
        this.ctx.clip();
    }
}
```

### WebGL Renderer

```javascript
// From js/src/renderer_webgl.ts

class WebGLRenderer {
    constructor(canvas) {
        this.canvas = canvas;
        this.gl = canvas.getContext('webgl2');

        // Initialize shaders
        this.initShaders();

        // Create vertex array object
        this.vao = this.gl.createVertexArray();
        this.gl.bindVertexArray(this.vao);

        // Create vertex buffer
        this.vertexBuffer = this.gl.createBuffer();

        // Create index buffer
        this.indexBuffer = this.gl.createBuffer();
    }

    initShaders() {
        // Vertex shader
        const vsSource = `
            #version 300 es
            in vec2 a_position;
            in vec4 a_color;
            out vec4 v_color;

            uniform mat3 u_transform;

            void main() {
                vec3 transformed = u_transform * vec3(a_position, 1.0);
                gl_Position = vec4(transformed.xy, 0.0, 1.0);
                v_color = a_color;
            }
        `;

        // Fragment shader
        const fsSource = `
            #version 300 es
            precision highp float;
            in vec4 v_color;
            out vec4 fragColor;

            void main() {
                fragColor = v_color;
            }
        `;

        this.program = this.createProgram(vsSource, fsSource);
    }

    drawPath(pathPtr, paintPtr) {
        // Read triangulated path from WASM
        const { vertices, indices } = readPathTriangles(pathPtr);

        // Upload vertices
        this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.vertexBuffer);
        this.gl.bufferData(
            this.gl.ARRAY_BUFFER,
            vertices,
            this.gl.DYNAMIC_DRAW
        );

        // Upload indices
        this.gl.bindBuffer(this.gl.ELEMENT_ARRAY_BUFFER, this.indexBuffer);
        this.gl.bufferData(
            this.gl.ELEMENT_ARRAY_BUFFER,
            indices,
            this.gl.DYNAMIC_DRAW
        );

        // Set vertex attributes
        const positionLoc = this.gl.getAttribLocation(this.program, 'a_position');
        const colorLoc = this.gl.getAttribLocation(this.program, 'a_color');

        this.gl.enableVertexAttribArray(positionLoc);
        this.gl.vertexAttribPointer(positionLoc, 2, this.gl.FLOAT, false, 24, 0);

        this.gl.enableVertexAttribArray(colorLoc);
        this.gl.vertexAttribPointer(colorLoc, 4, this.gl.UNSIGNED_BYTE, true, 24, 8);

        // Set transform uniform
        const transformLoc = this.gl.getUniformLocation(this.program, 'u_transform');
        this.gl.uniformMatrix3fv(transformLoc, false, paintPtr.transform);

        // Draw
        this.gl.drawElements(
            this.gl.TRIANGLES,
            indices.length,
            this.gl.UNSIGNED_SHORT,
            0
        );
    }

    clipPath(pathPtr) {
        // Use stencil buffer for clipping
        this.gl.enable(this.gl.STENCIL_TEST);
        this.gl.stencilFunc(this.gl.ALWAYS, 1, 0xFF);
        this.gl.stencilOp(this.gl.KEEP, this.gl.KEEP, this.gl.REPLACE);
        this.gl.colorMask(false, false, false, false);

        // Draw path to stencil
        this.drawPathToStencil(pathPtr);

        this.gl.colorMask(true, true, true, true);
        this.gl.stencilFunc(this.gl.EQUAL, 1, 0xFF);
        this.gl.stencilOp(this.gl.KEEP, this.gl.KEEP, this.gl.KEEP);
    }
}
```

### OffscreenCanvas Renderer

For better performance in Web Workers:

```javascript
// OffscreenCanvas support

class OffscreenRenderer {
    constructor(offscreen) {
        this.canvas = offscreen;
        this.ctx = offscreen.getContext('2d', { alpha: true });
    }

    // Same interface as Canvas2DRenderer
    // Can be used in Web Worker for parallel rendering
}

// Worker usage:
// Main thread: Handle input, game logic
// Worker thread: Rive rendering
```

---

## Performance Optimizations

### 1. WASM Module Caching

```javascript
// Cache WASM module across page loads
const wasmCache = {
    module: null,
    instance: null
};

async function loadWasm() {
    if (wasmCache.module) {
        return WebAssembly.instantiate(wasmCache.module, imports);
    }

    const response = await fetch('rive.wasm');
    const buffer = await response.arrayBuffer();
    wasmCache.module = buffer;

    return WebAssembly.instantiate(buffer, imports);
}
```

### 2. Single-File Bundles

```javascript
// Single-file bundles embed WASM as base64
// No network request needed for WASM file

// @rive-app/canvas-single
const wasmBase64 = 'AGFzbQEAAAAB...'; // Embedded WASM
const wasmBinary = Uint8Array.from(atob(wasmBase64), c => c.charCodeAt(0));

// Pros: Faster initial load, simpler deployment
// Cons: Larger JS bundle, no caching benefit
```

### 3. Renderer Selection

```javascript
// Automatic renderer selection
function selectRenderer(canvas) {
    // Try WebGL first (fastest)
    if (canvas.getContext('webgl2')) {
        return 'webgl';
    }
    if (canvas.getContext('webgl')) {
        return 'webgl';
    }

    // Fall back to Canvas 2D
    if (canvas.getContext('2d')) {
        return '2d';
    }

    throw new Error('No supported rendering context');
}
```

### 4. Animation Frame Throttling

```javascript
// Throttle updates when tab is not visible
class Rive {
    constructor() {
        this.targetFps = 60;
        this.frameInterval = 1000 / this.targetFps;

        document.addEventListener('visibilitychange', () => {
            if (document.hidden) {
                this.targetFps = 1; // Minimize CPU when hidden
            } else {
                this.targetFps = 60;
            }
        });
    }

    render(time) {
        const elapsed = time - this.lastFrameTime;

        if (elapsed >= this.frameInterval) {
            this.update(elapsed / 1000);
            this.lastFrameTime = time;
        }

        requestAnimationFrame((t) => this.render(t));
    }
}
```

### 5. Path Tessellation Caching

```cpp
// From bindings.cpp - Cache tessellated paths

class PathCache {
    std::unordered_map<PathId, CachedPath> m_Cache;

    CachedPath* getOrTessellate(Path* path) {
        auto it = m_Cache.find(path->id());
        if (it != m_Cache.end()) {
            return &it->second;
        }

        // Tessellate and cache
        CachedPath cached;
        tessellate(path, &cached);
        m_Cache[path->id()] = cached;
        return &m_Cache[path->id()];
    }
};
```

### 6. Transferable Objects

```javascript
// Transfer ArrayBuffers to WASM without copying
const buffer = new ArrayBuffer(size);
const view = new Uint8Array(buffer);

// Fill with data
// ...

// Transfer to WASM (zero-copy)
wasmModule.exports.process_data(
    buffer,
    { transfer: [buffer] }  // Transfer ownership
);

// buffer is now inaccessible from JS (transferred)
```

---

## Usage Examples

### Basic Animation Display

```html
<!DOCTYPE html>
<html>
<head>
    <script src="https://cdn.rive.app/canvas/4.0.0/rive.js"></script>
</head>
<body>
    <canvas id="canvas"></canvas>
    <script>
        const r = new rive.Rive({
            src: 'animation.riv',
            canvas: document.getElementById('canvas'),
            autoplay: true,
            onLoad: () => {
                console.log('Rive loaded!');
            }
        });
    </script>
</body>
</html>
```

### State Machine Interaction

```javascript
const r = new rive.Rive({
    src: 'character.riv',
    canvas: document.getElementById('canvas'),
    stateMachines: 'Player',
    onLoad: ({ file }) => {
        const sm = r.stateMachineCtrl('Player');

        // Handle keyboard input
        document.addEventListener('keydown', (e) => {
            if (e.code === 'Space') {
                sm.fire('jump');
            }
            if (e.code === 'ShiftLeft') {
                sm.setInput('isRunning', true);
            }
        });

        document.addEventListener('keyup', (e) => {
            if (e.code === 'ShiftLeft') {
                sm.setInput('isRunning', false);
            }
        });
    }
});
```

### Multiple Artboards

```javascript
// Low-level API for multiple artboards
const { rive, canvaskit } = await rive.loadRive();

const buffer = await fetch('scene.riv').then(r => r.arrayBuffer());
const file = rive.loadFile(new Uint8Array(buffer));

const artboard1 = file.artboardByName('Player');
const artboard2 = file.artboardByName('Enemy');

const animation1 = artboard1.animationByName('Walk');
const animation2 = artboard2.animationByName('Idle');

const instance1 = animation1.createInstance();
const instance2 = animation2.createInstance();

function render(time) {
    const elapsed = time / 1000;

    // Update both animations
    instance1.update(elapsed);
    instance2.update(elapsed);

    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw first artboard
    ctx.save();
    ctx.translate(100, 300);
    instance1.apply(artboard1);
    artboard1.draw(renderer);
    ctx.restore();

    // Draw second artboard
    ctx.save();
    ctx.translate(500, 300);
    instance2.apply(artboard2);
    artboard2.draw(renderer);
    ctx.restore();

    requestAnimationFrame(render);
}

requestAnimationFrame(render);
```

---

## Summary

Rive's WASM runtime provides:

1. **High Performance**: C++ core compiled to WASM
2. **Multiple Backends**: Canvas 2D and WebGL support
3. **Memory Efficiency**: Zero-copy transfers where possible
4. **Easy Integration**: High-level JavaScript API
5. **Flexibility**: Low-level API for advanced use cases

For related topics:
- `rendering-engine-deep-dive.md` - GPU rendering details
- `vector-graphics-algorithms.md` - Path rendering algorithms
- `cpp-core-architecture.md` - C++ runtime architecture
