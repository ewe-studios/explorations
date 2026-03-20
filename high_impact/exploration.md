---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/high_impact
repository: https://github.com/phoboslab/high_impact
explored_at: 2026-03-20
language: C
license: MIT
---

# High Impact Game Engine - Comprehensive Technical Exploration

**Author:** phoboslab (Dominique Walter)
**License:** MIT (engine code)
**Primary Use:** 2D Pixel Art Game Engine in C
**Platforms:** Linux, macOS, Windows, Web (WASM)

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Engine Core](#2-engine-core)
3. [Entity System](#3-entity-system)
4. [Rendering System](#4-rendering-system)
5. [Map/Tilemap System](#5-map-tilemap-system)
6. [Input System](#6-input-system)
7. [Sound System](#7-sound-system)
8. [Animation System](#8-animation-system)
9. [Image System](#9-image-system)
10. [Physics/Collision (Trace)](#10-physicscollision-trace)
11. [Platform Abstraction](#11-platform-abstraction)
12. [Memory Management](#12-memory-management)
13. [Mathematical Foundations (Types)](#13-mathematical-foundations-types)
14. [Complete Game Loop](#14-complete-game-loop)
15. [Comparison with Impact.js](#15-comparison-with-impactjs)

---

## 1. Architecture Overview

### 1.1 Core Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         HIGH IMPACT ENGINE                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │   Platform   │───▶│    Engine    │◀───│    Input     │              │
│  │  (SDL/Sokol) │    │   (Main)     │    │   (Binding)  │              │
│  └──────────────┘    └──────┬───────┘    └──────────────┘              │
│                             │                                            │
│         ┌───────────────────┼───────────────────┐                       │
│         │                   │                   │                       │
│         ▼                   ▼                   ▼                       │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │   Render     │    │   Entities   │    │    Maps      │              │
│  │  (GL/Metal/  │    │  (Spawn/     │    │  (Tilemap/   │              │
│  │   Software)  │    │   Update)    │    │  Collision)  │              │
│  └──────────────┘    └──────────────┘    └──────────────┘              │
│         │                   │                   │                       │
│         ▼                   ▼                   ▼                       │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │    Image     │    │  Animation   │    │    Sound     │              │
│  │   (QOI)      │    │   (Sheets)   │    │   (QOA)      │              │
│  └──────────────┘    └──────────────┘    └──────────────┘              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Component Responsibility Matrix

| Component | Responsibility | Key Files |
|-----------|---------------|-----------|
| `engine` | Main game loop, scene management, timekeeping | `engine.h/c` |
| `entity` | Dynamic game objects with physics | `entity.h/c` |
| `render` | Drawing primitives, transform stack | `render.h/c`, `render_gl.c`, `render_metal.m` |
| `map` | Tilemap storage, drawing, collision data | `map.h/c` |
| `trace` | AABB collision detection vs. maps | `trace.h/c` |
| `input` | Keyboard, mouse, gamepad abstraction | `input.h/c` |
| `sound` | Audio playback, mixing, synthesis | `sound.h/c` |
| `animation` | Sprite sheet animation | `animation.h/c` |
| `image` | QOI image loading and drawing | `image.h/c` |
| `platform` | SDL/Sokol abstraction layer | `platform.h`, `platform_sdl.c`, `platform_sokol.c` |
| `types` | Math types (vec2, mat3, rgba) | `types.h` |
| `alloc` | Temp memory arena allocation | `alloc.h/c` |

### 1.3 Initialization Flow

```
Platform startup (SDL_Init / sokol_main)
        │
        ▼
┌───────────────┐
│ platform_init │
│ - Create window│
│ - Setup input  │
│ - Init audio   │
└───────┬───────┘
        │
        ▼
┌──────────────┐
│ engine_init  │
│ - Init render│
│ - Init sound │
│ - Init input │
│ - Init entities│
└───────┬───────┘
        │
        ▼
┌──────────────┐
│ main_init()  │ ◀── User code entry point
│ (scene init) │
└───────┬───────┘
        │
        ▼
┌──────────────┐
│ engine.update│ ◀── Main loop (60 FPS target)
│ - scene.update│
│ - entities.update│
│ - render.draw │
└──────────────┘
```

---

## 2. Engine Core

### 2.1 Engine State Structure

The engine is a **single global instance** (`engine_t engine`) that holds all game state:

```c
typedef struct {
    double time_real;     // Real time since program start
    double time;          // Game time since scene start
    double time_scale;    // Global time multiplier (default: 1.0)
    double tick;          // Delta time from last frame (~0.01666 for 60hz)
    uint64_t frame;       // Frame counter

    map_t *collision_map; // Current collision map
    map_t *background_maps[ENGINE_MAX_BACKGROUND_MAPS];
    uint32_t background_maps_len;

    float gravity;        // Global gravity multiplier (default: 1.0)
    vec2_t viewport;      // Camera offset

    struct {
        int entities;
        int checks;
        int draw_calls;
        float update, draw, total;
    } perf;
} engine_t;
```

### 2.2 Scene System

Games are structured as **scenes**, each providing four callbacks:

```c
typedef struct {
    void (*init)(void);    // Called once when scene starts
    void (*update)(void);  // Called every frame
    void (*draw)(void);    // Called every frame for rendering
    void (*cleanup)(void); // Called when scene ends
} scene_t;
```

**Scene Lifecycle:**
```
engine_set_scene(new_scene)
        │
        ▼
┌─────────────────────┐
│ Swap at frame start │
└─────────┬───────────┘
          │
    ┌─────┴─────┐
    ▼           ▼
┌───────┐   ┌─────────┐
│cleanup│   │  init() │
│ old   │   │  new    │
└───────┘   └────┬────┘
                 │
          ┌──────┴──────┐
          │  Per Frame  │
          │  update()   │
          │  draw()     │
          └─────────────┘
```

### 2.3 Key Functions

| Function | Purpose |
|----------|---------|
| `engine_set_scene(scene_t*)` | Switch to a new scene |
| `engine_load_level(json_path)` | Load a level from Weltmeister JSON |
| `engine_add_background_map(map_t*)` | Add a parallax background |
| `engine_set_collision_map(map_t*)` | Set the collision map |
| `scene_base_update()` | Update all entities (call from scene->update) |
| `scene_base_draw()` | Draw all background maps and entities |

---

## 3. Entity System

### 3.1 Entity Architecture

Every dynamic game object is an **entity**. The system uses a **data-oriented design** with:

- **Static entity array** (`ENTITIES_MAX` = 1024 by default)
- **Entity references** (stable handles, not pointers)
- **Component-style virtual table** per entity type

### 3.2 Entity Definition Pattern

Entities are defined using an X-Macro pattern:

```c
// 1. Define entity types enum
#define ENTITY_TYPES() \
    X(PLAYER)          \
    X(ENEMY)           \
    X(BULLET)          \
    X(PARTICLE)

// 2. Define entity struct with ENTITY_DEFINE()
ENTITY_DEFINE(
    vec2_t pos;
    vec2_t vel;
    float health;
    // ... custom fields
)
```

### 3.3 Entity Virtual Table

Each entity type can override these callbacks:

```c
typedef struct {
    void (*load)(void);           // One-time init for type
    void (*init)(entity_t *self); // Per-entity init
    void (*settings)(entity_t*, json_t*); // From level JSON
    void (*update)(entity_t *self);
    void (*draw)(entity_t *self, vec2_t viewport);
    void (*kill)(entity_t *self);
    void (*touch)(entity_t *self, entity_t *other);
    void (*collide)(entity_t*, vec2_t normal, trace_t*);
    void (*damage)(entity_t*, entity_t *other, float damage);
    void (*trigger)(entity_t *self, entity_t *other);
    void (*message)(entity_t*, entity_message_t, void*);
} entity_vtab_t;
```

### 3.4 Entity Lifecycle

```
entity_spawn(PLAYER, vec2(100, 100))
        │
        ▼
┌─────────────────────┐
│ Allocate from pool  │
│ Set type, pos       │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ vtab->init(self)    │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│ vtab->settings(def) │ ◀── If spawned from level JSON
└─────────┬───────────┘
          │
    ┌─────┴─────┐
    │Per Frame: │
    ▼           ▼
┌───────┐   ┌───────┐
│update │   │ draw  │
└───────┘   └───────┘
```

### 3.5 Entity Queries

| Function | Returns |
|----------|---------|
| `entities_by_proximity(ent, radius, type)` | Entities near another |
| `entities_by_location(pos, radius, type, exclude)` | Entities near point |
| `entities_by_type(type)` | All entities of type |
| `entity_by_name(name)` | Entity by name from JSON |
| `entity_by_ref(ref)` | Entity from reference handle |

---

## 4. Rendering System

### 4.1 Renderer Abstraction

High Impact supports **multiple render backends**:

| Backend | File | Platform |
|---------|------|----------|
| OpenGL | `render_gl.c` | Linux, Windows |
| Metal | `render_metal.m` | macOS, iOS |
| Software | `render_software.c` | All (fallback) |

### 4.2 Logical vs. Real Pixels

The renderer uses a **logical size** that may differ from the window size:

```c
#define RENDER_WIDTH 1280
#define RENDER_HEIGHT 720

#define RENDER_SCALE_MODE RENDER_SCALE_DISCRETE  // Integer scaling
#define RENDER_RESIZE_MODE RENDER_RESIZE_ANY     // Fill window
```

**Scale Modes:**
- `RENDER_SCALE_NONE` - No scaling
- `RENDER_SCALE_DISCRETE` - Integer steps (perfect pixel art)
- `RENDER_SCALE_EXACT` - Stretch to window

### 4.3 Transform Stack

```c
render_push();           // Save current transform
render_translate(pos);   // Apply translation
render_rotate(angle);    // Apply rotation
render_scale(scale);     // Apply scale
// ... draw calls ...
render_pop();            // Restore transform
```

**Stack limit:** `RENDER_TRANSFORM_STACK_SIZE` (16 by default)

### 4.4 Draw Calls

The backend must implement:
- `render_draw()` - Draw textured quad
- `render_frame_prepare()` - Start of frame
- `render_frame_end()` - Present
- `texture_create()` / `texture_replace_pixels()`

---

## 5. Map/Tilemap System

### 5.1 Map Structure

```c
typedef struct {
    vec2i_t size;         // Size in tiles
    uint16_t tile_size;   // Tile size in pixels
    char name[16];        // "collision" or layer name
    float distance;       // Parallax factor
    bool repeat;          // Tile infinitely
    bool foreground;      // Draw in front of entities
    image_t *tileset;     // Tile image
    uint16_t *data;       // Tile indices (bias +1)
} map_t;
```

### 5.2 Tile Index Bias

**Important:** Tile indices have a **+1 bias**:
- Index `0` = Empty/blank tile
- Index `1` = First tile from tileset
- Index `2` = Second tile, etc.

### 5.3 Map Functions

| Function | Purpose |
|----------|---------|
| `map_from_json(json_t*)` | Load from Weltmeister JSON |
| `map_draw(map, offset)` | Draw with parallax |
| `map_tile_at(map, tile_pos)` | Get tile at tile coordinate |
| `map_tile_at_px(map, px_pos)` | Get tile at pixel coordinate |
| `map_set_anim(tile, time, ...)` | Animate a tile |

---

## 6. Input System

### 6.1 Action Binding Pattern

Input uses an **action-based binding** system:

```c
// Define actions
enum {
    ACTION_JUMP,
    ACTION_LEFT,
    ACTION_RIGHT,
    ACTION_SHOOT,
};

// Bind keys/buttons to actions
input_bind(INPUT_KEY_SPACE, ACTION_JUMP);
input_bind(INPUT_GAMEPAD_A, ACTION_JUMP);
input_bind(INPUT_KEY_LEFT, ACTION_LEFT);
input_bind(INPUT_GAMEPAD_DPAD_LEFT, ACTION_LEFT);

// Check state
if (input_pressed(ACTION_JUMP)) {
    // Jump logic
}
```

### 6.2 Button Enum

Buttons are unified across devices:
- `INPUT_KEY_*` - Keyboard (A-Z, 0-9, F1-F12, modifiers)
- `INPUT_GAMEPAD_*` - Controller (A/B/X/Y, triggers, sticks)
- `INPUT_MOUSE_*` - Mouse buttons and wheel

### 6.3 Input State

| Function | Returns |
|----------|---------|
| `input_state(action)` | Current state (0-1, analog) |
| `input_pressed(action)` | True on press frame |
| `input_released(action)` | True on release frame |
| `input_mouse_pos()` | Current mouse position |
| `input_capture(cb, user)` | Capture all input (for text) |

---

## 7. Sound System

### 7.1 Source/Node Architecture

Sounds are split into:
- **`sound_source_t`** - Loaded audio data (shared)
- **`sound_t`** - Playing instance (node)

```c
sound_source_t *src = sound_source("assets/sfx/jump.qoa");
sound_play(src);  // Play one-shot
```

### 7.2 Sound Creation

| Function | Creates |
|----------|---------|
| `sound_source(path)` | Load from QOA file |
| `sound_source_with_samples(...)` | From raw PCM |
| `sound_source_synth_sound(...)` | From pl_synth definition |
| `sound_source_synth_song(...)` | From pl_synth song |

### 7.3 Node Control

```c
sound_t s = sound(src);  // Get node (paused)
sound_set_pitch(s, 1.5); // Faster playback
sound_set_volume(s, 0.5);
sound_set_loop(s, true);
sound_unpause(s);        // Start playing
```

### 7.4 Memory Management

```c
sound_mark_t mark = sound_mark();  // Save state
// ... load sounds ...
sound_reset(mark);  // Free all since mark
```

---

## 8. Animation System

### 8.1 Definition vs. Instance

Animations are split:
- **`anim_def_t`** - Shared definition (sequence, timing)
- **`anim_t`** - Instance state (current frame, flip, color)

```c
anim_def_t *walk_def = anim_def_with_len(
    sheet,           // Animation sheet image
    vec2i(16, 16),   // Frame size
    0.1f,            // Time per frame
    (uint16_t[]){0, 1, 2, 3},  // Sequence
    4                // Length
);

anim_t walk = anim(walk_def);  // Create instance
```

### 8.2 Animation Control

| Function | Purpose |
|----------|---------|
| `anim_rewind(anim)` | Go to first frame |
| `anim_goto(anim, index)` | Go to specific frame |
| `anim_goto_rand(anim)` | Go to random frame |
| `anim_looped(anim)` | Count completions |
| `anim_draw(anim, pos)` | Draw at position |

### 8.3 Animation Properties

```c
anim.color = rgba(255, 128, 128, 255);  // Tint
anim.flip_x = true;                      // Flip horizontally
anim.rotation = M_PI / 4;                // Rotate 45°
```

---

## 9. Image System

### 9.1 QOI Format

High Impact **only loads QOI (Quick Object Image)** format:

```c
image_t *img = image("assets/sprites/player.qoi");
```

**Advantages:**
- Extremely fast load/decode
- Small file size
- Simple format

### 9.2 Image Functions

| Function | Purpose |
|----------|---------|
| `image(path)` | Load from QOI (cached) |
| `image_with_pixels(size, pixels)` | Create from raw data |
| `image_size(img)` | Get dimensions |
| `image_draw(img, pos)` | Draw full image |
| `image_draw_ex(...)` | Draw with source/dest rects |
| `image_draw_tile(...)` | Draw single tile from sheet |

### 9.3 Memory Management

Images are automatically cached and freed on `images_reset(mark)`.

---

## 10. Physics/Collision (Trace)

### 10.1 Trace Function

The `trace()` function performs **swept AABB collision detection**:

```c
trace_t trace(map_t *map, vec2_t from, vec2_t vel, vec2_t size);
```

**Parameters:**
- `map` - Collision map (usually `engine.collision_map`)
- `from` - Starting position (top-left of AABB)
- `vel` - Movement vector
- `size` - Size of AABB

### 10.2 Trace Result

```c
typedef struct {
    int tile;          // Tile index hit (0 = no hit)
    vec2i_t tile_pos;  // Tile coordinate
    float length;      // 0..1 travel distance
    vec2_t pos;        // Final position
    vec2_t normal;     // Surface normal
} trace_t;
```

### 10.3 Entity Collision Integration

Entities use trace internally:

```c
// In entity_base_update():
trace_t tr = trace(engine.collision_map, self->pos, self->vel, self->size);
if (tr.tile != 0) {
    entity_collide(self, tr.normal, &tr);
}
```

---

## 11. Platform Abstraction

### 11.1 Platform Backends

| Platform | File | Dependencies |
|----------|------|--------------|
| SDL2 | `platform_sdl.c` | SDL2 |
| Sokol | `platform_sokol.c` | sokol_app.h, sokol_audio.h |

### 11.2 Platform Responsibilities

The platform layer handles:
- Window creation and management
- Input event processing
- Audio output setup
- File I/O (assets, save data)
- Timing (`platform_now()`)

### 11.3 Key Platform Functions

```c
vec2i_t platform_screen_size(void);
double platform_now(void);
uint32_t platform_samplerate(void);
uint8_t *platform_load_asset(path, &len);
uint8_t *platform_load_userdata(path, &len);  // Save games
void platform_store_userdata(path, data, len);
```

---

## 12. Memory Management

### 12.1 Temp Memory Arena

High Impact uses a **temporary memory arena** for loading:

```c
json_t *json = platform_load_asset_json("level.json");
// ... use json ...
temp_free(json);  // Free to arena
```

### 12.2 Mark/Reset Pattern

Resources use mark/reset for scene-based cleanup:

```c
image_mark_t img_mark = images_mark();
sound_mark_t snd_mark = sound_mark();

// Load scene resources...

// On scene cleanup:
images_reset(img_mark);  // Free images since mark
sounds_reset(snd_mark);  // Free sounds since mark
```

---

## 13. Mathematical Foundations (Types)

### 13.1 Core Types

```c
typedef struct { float x, y; } vec2_t;
typedef struct { int x, y; } vec2i_t;
typedef struct { float a, b, c, d, tx, ty; } mat3_t;
typedef union { struct { uint8_t r, g, b, a; }; uint32_t v; } rgba_t;
```

### 13.2 Vector Operations

| Function | Purpose |
|----------|---------|
| `vec2_add(a, b)` | Vector addition |
| `vec2_sub(a, b)` | Vector subtraction |
| `vec2_mulf(v, f)` | Scalar multiply |
| `vec2_dot(a, b)` | Dot product |
| `vec2_cross(a, b)` | Cross product |
| `vec2_len(v)` | Vector length |
| `vec2_dist(a, b)` | Distance |
| `vec2_from_angle(a)` | Unit vector from angle |
| `vec2_to_angle(v)` | Angle from vector |

### 13.3 Matrix Operations

```c
mat3_identity();
mat3_translate(m, t);
mat3_scale(m, s);
mat3_rotate(m, r);
vec2_transform(v, m);
```

---

## 14. Complete Game Loop

### 14.1 Frame Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│                    FRAME LOOP (60 FPS)                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Platform Events                                          │
│     - Poll input (keyboard, mouse, gamepad)                 │
│     - Handle window resize/close                            │
│                                                              │
│  2. Scene Swap (if requested)                                │
│     - old_scene->cleanup()                                  │
│     - new_scene->init()                                     │
│                                                              │
│  3. Update Phase                                             │
│     - input_clear()                                         │
│     - scene->update() ─┬─> entity_update()                 │
│                        └─> entity_base_update()            │
│                                                              │
│  4. Draw Phase                                               │
│     - render_frame_prepare()                                │
│     - scene->draw() ─┬─> map_draw(background_maps)         │
│                      └─> entity_draw()                     │
│     - render_frame_end()                                    │
│                                                              │
│  5. Timing                                                   │
│     - Calculate tick (delta time)                           │
│     - Cap to ENGINE_MAX_TICK (0.1s)                         │
│     - Update engine.time                                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 14.2 Code Flow

```c
// From platform_sdl.c / platform_sokol.c
while (!quit) {
    platform_handle_events();

    if (engine_needs_scene_swap()) {
        engine_swap_scene();
    }

    engine.update();  // Calls scene->update, entities_update
    engine.draw();    // Calls scene->draw, entities_draw

    platform_present();
    platform_wait_for_frame();
}
```

---

## 15. Comparison with Impact.js

### 15.1 Similarities

| Feature | Impact.js | High Impact |
|---------|-----------|-------------|
| Entity system | ✓ | ✓ |
| Tilemap layers | ✓ | ✓ |
| Collision detection | ✓ | ✓ (trace) |
| Animation system | ✓ | ✓ |
| Input binding | ✓ | ✓ |
| Sound/Music | ✓ | ✓ (QOA) |
| Level editor | Weltmeister | Weltmeister (shared) |
| Scene management | ig.Game | engine_t + scene_t |

### 15.2 Key Differences

| Aspect | Impact.js | High Impact |
|--------|-----------|-------------|
| Language | JavaScript | C |
| Image Format | PNG | QOI |
| Audio Format | WAV/MP3 | QOA |
| Render | Canvas 2D | OpenGL/Metal/Software |
| Memory | GC | Manual (arena) |
| Performance | ~60 FPS simple | ~60 FPS complex |
| Hot Reload | Yes | No |
| WASM | No | Yes |

### 15.3 Philosophy

**Impact.js:** "Easy to use, accessible HTML5 games"

**High Impact:** "High performance, portable 2D games in C"

---

## Deep Dive Documents

For more detailed information on specific subsystems:

- **[Entity System Deep Dive](./entity-system-deep-dive.md)** - Entity lifecycle, physics, collision
- **[Rendering Deep Dive](./rendering-deep-dive.md)** - OpenGL/Metal backends, transform stack
- **[Input System Deep Dive](./input-system-deep-dive.md)** - Binding, gamepad handling
- **[Sound System Deep Dive](./sound-system-deep-dive.md)** - QOA, pl_synth integration
- **[Map System Deep Dive](./map-system-deep-dive.md)** - Tilemaps, parallax, animation
- **[Platform Layer Deep Dive](./platform-layer-deep-dive.md)** - SDL vs Sokol backends

---

## Related Resources

- [High Impact Blog Post](https://phoboslab.org/log/2024/08/high_impact) - Design decisions
- [Biolab Disaster](https://github.com/phoboslab/high_biolab) - Example game
- [Drop](https://github.com/phoboslab/high_drop) - Minimal arcade example
- [QOI Format](https://qoiformat.org/) - Image format spec
- [QOA Format](https://qoaformat.org/) - Audio format spec
- [pl_synth](https://github.com/phoboslab/pl_synth) - Sound synthesis
