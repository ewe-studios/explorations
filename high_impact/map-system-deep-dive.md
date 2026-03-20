---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/high_impact/src/
repository: https://github.com/phoboslab/high_impact
explored_at: 2026-03-20
language: C
parent: exploration.md
---

# High Impact Map System - Deep Dive

**Source Files:** `map.h`, `map.c`

---

## Table of Contents

1. [Map Architecture Overview](#1-map-architecture-overview)
2. [Map Structure](#2-map-structure)
3. [Loading Maps from JSON](#3-loading-maps-from-json)
4. [Tile Index Bias](#4-tile-index-bias)
5. [Drawing Maps](#5-drawing-maps)
6. [Tile Animations](#6-tile-animations)
7. [Collision Maps](#7-collision-maps)
8. [Parallax Backgrounds](#8-parallax-backgrounds)
9. [Map Queries](#9-map-queries)

---

## 1. Map Architecture Overview

### 1.1 Map Types

High Impact uses maps for:

| Type | Purpose | Example |
|------|---------|---------|
| **Collision Map** | Entity vs. world collision | Solid tiles, hazards |
| **Background Map** | Visual backdrop (parallax) | Sky, distant buildings |
| **Foreground Map** | Visual overlay | Ceiling details, HUD |

### 1.2 Map System Diagram

```
┌────────────────────────────────────────────────────────────┐
│                      MAP SYSTEM                             │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Map Data (Tile Grid)                   │   │
│  │  ┌───┬───┬───┬───┬───┐                             │   │
│  │  │ 0 │ 1 │ 2 │ 3 │ 0 │  Tile indices               │   │
│  │  ├───┼───┼───┼───┼───┤  (bias +1 for display)      │   │
│  │  │ 3 │ 2 │ 1 │ 0 │ 1 │                             │   │
│  │  └───┴───┴───┴───┴───┘                             │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                 │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Tileset Image                          │   │
│  │  ┌───┬───┬───┬───┐                                 │   │
│  │  │ 0 │ 1 │ 2 │ 3 │  Each tile = tile_size x        │   │
│  │  ├───┼───┼───┼───┤  tile_size pixels               │   │
│  │  │ 4 │ 5 │ 6 │ 7 │                                 │   │
│  │  └───┴───┴───┴───┘                                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                 │
│              ┌─────────────┴─────────────┐                  │
│              │                           │                  │
│              ▼                           ▼                  │
│  ┌──────────────────────┐    ┌──────────────────────┐      │
│  │   Map Drawing        │    │   Collision Trace    │      │
│  │   (render tiles)     │    │   (AABB vs. tiles)   │      │
│  └──────────────────────┘    └──────────────────────┘      │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

---

## 2. Map Structure

### 2.1 Map Definition

```c
typedef struct map {
    vec2i_t size;           // Size in tiles (e.g., 32x24)
    uint16_t tile_size;     // Tile size in pixels (e.g., 16, 32)
    char name[16];          // Map name ("collision", "background")
    float distance;         // Parallax factor (1.0 = normal)
    bool repeat;            // Tile infinitely (for backgrounds)
    bool foreground;        // Draw in front of entities
    image_t *tileset;       // Source image for tiles
    map_anim_def_t **anims; // Tile animations
    uint16_t *data;         // Tile indices (size.x * size.y)
    uint16_t max_tile;      // Highest tile index in map
} map_t;
```

### 2.2 Map Creation

```c
// Create map with explicit data
uint16_t tiles[] = {
    1, 2, 3, 1,
    4, 0, 0, 5,
    6, 7, 8, 6,
};

map_t *map = map_with_data(
    16,              // tile_size
    vec2i(4, 3),     // size (4 tiles wide, 3 tiles tall)
    tiles            // data array
);

// Map dimensions in pixels:
// width = 4 * 16 = 64px
// height = 3 * 16 = 48px
```

### 2.3 Memory Layout

```
data[] = [tile_index for each cell]
         Row-major order (left-to-right, top-to-bottom)

Index in data[] = y * size.x + x

Example for 4x3 map:
data[0]  = tile at (0, 0)  top-left
data[3]  = tile at (3, 0)  top-right
data[4]  = tile at (0, 1)  left-middle
data[11] = tile at (3, 2)  bottom-right
```

---

## 3. Loading Maps from JSON

### 3.1 JSON Format (Weltmeister)

```json
{
    "name": "level_1",
    "layers": [
        {
            "name": "background",
            "width": 32,
            "height": 24,
            "tilesize": 16,
            "tilesetName": "assets/tiles/sky.qoi",
            "distance": 0.5,
            "repeat": true,
            "foreground": false,
            "data": [
                [1, 1, 1, 1, 1, 1, 1, 1, ...],
                [2, 2, 2, 2, 2, 2, 2, 2, ...],
                ...
            ]
        },
        {
            "name": "collision",
            "width": 32,
            "height": 24,
            "tilesize": 16,
            "tilesetName": null,
            "distance": 1.0,
            "repeat": false,
            "foreground": false,
            "data": [
                [0, 0, 0, 0, 0, 0, 0, 0, ...],
                [0, 0, 0, 0, 0, 0, 0, 0, ...],
                [1, 1, 1, 1, 1, 1, 1, 1, ...],
            ]
        }
    ]
}
```

### 3.2 Loading Function

```c
map_t *map_from_json(json_t *def) {
    map_t *map = malloc(sizeof(map_t));

    // Basic properties
    strncpy(map->name, def->children["name"]->value.string, 16);
    map->size.x = def->children["width"]->value.int_val;
    map->size.y = def->children["height"]->value.int_val;
    map->tile_size = def->children["tilesize"]->value.int_val;

    // Optional properties
    json_t *dist = def->children["distance"];
    map->distance = dist ? dist->value.float_val : 1.0f;

    json_t *rep = def->children["repeat"];
    map->repeat = rep ? rep->value.bool_val : false;

    json_t *fg = def->children["foreground"];
    map->foreground = fg ? fg->value.bool_val : false;

    // Load tileset
    json_t *tileset_name = def->children["tilesetName"];
    if (tileset_name && tileset_name->value.string) {
        map->tileset = image(tileset_name->value.string);
    } else {
        map->tileset = NULL;  // Collision map
    }

    // Allocate and fill data
    int tile_count = map->size.x * map->size.y;
    map->data = malloc(tile_count * sizeof(uint16_t));

    // Parse data array (2D -> 1D)
    json_t *data_array = def->children["data"];
    int i = 0;
    json_t *row = data_array->children;
    while (row) {
        json_t *tile = row->children;
        while (tile) {
            map->data[i++] = tile->value.int_val;
            tile = tile->next;
        }
        row = row->next;
    }

    // Find max tile index
    map->max_tile = 0;
    for (int j = 0; j < tile_count; j++) {
        if (map->data[j] > map->max_tile) {
            map->max_tile = map->data[j];
        }
    }

    return map;
}
```

### 3.3 Level Loading

```c
void scene_init(void) {
    engine_load_level("assets/levels/level_1.json");
}

// engine_load_level internally calls:
// - map_from_json() for each layer
// - engine_add_background_map() for visual layers
// - engine_set_collision_map() for collision layer
```

---

## 4. Tile Index Bias

### 4.1 The +1 Bias Rule

**Important:** Tile indices in map data have a **+1 bias**:

| Index | Meaning |
|-------|---------|
| 0 | Empty/blank (no tile drawn) |
| 1 | First tile from tileset (index 0 in image) |
| 2 | Second tile from tileset (index 1 in image) |
| N | Nth tile from tileset (index N-1 in image) |

### 4.2 Why the Bias?

```c
// Without bias, 0 would mean "first tile"
// With bias, 0 means "no tile" - useful for:
// - Empty space in background maps
// - Non-collidable tiles in collision maps
// - Clearer JSON representation

// Drawing handles the bias:
void map_draw_tile(map_t *map, int tile_index, vec2i_t pos) {
    if (tile_index == 0) return;  // Skip empty tiles
    // Bias -1 to get actual tileset index
    image_draw_tile(map->tileset, tile_index - 1, ...);
}
```

### 4.3 Collision Map Convention

```
Collision map tile values:
0 = Empty (no collision)
1+ = Solid (collision)

Custom tile properties can be encoded:
1 = Solid ground
2 = One-way platform
3 = Hazard (damages player)
4 = Water (slows movement)
5 = Ice (low friction)
```

---

## 5. Drawing Maps

### 5.1 Basic Draw Function

```c
void map_draw(map_t *map, vec2_t offset) {
    if (!map->tileset) return;  // No tileset (collision map)

    // Apply parallax
    vec2_t parallax_offset = vec2(
        offset.x * map->distance,
        offset.y * map->distance
    );

    // Calculate visible tile range
    vec2i_t start_tile = vec2i(
        floorf(-parallax_offset.x / map->tile_size),
        floorf(-parallax_offset.y / map->tile_size)
    );

    vec2i_t end_tile = vec2i(
        start_tile.x + ceilf(RENDER_WIDTH / map->tile_size) + 1,
        start_tile.y + ceilf(RENDER_HEIGHT / map->tile_size) + 1
    );

    // Clamp to map bounds
    start_tile.x = max(0, start_tile.x);
    start_tile.y = max(0, start_tile.y);
    end_tile.x = min(map->size.x, end_tile.x);
    end_tile.y = min(map->size.y, end_tile.y);

    // Draw visible tiles
    for (int y = start_tile.y; y < end_tile.y; y++) {
        for (int x = start_tile.x; x < end_tile.x; x++) {
            int tile_index = map->data[y * map->size.x + x];
            if (tile_index == 0) continue;  // Skip empty

            // Check for animation
            map_anim_def_t *anim = map_get_anim(map, x, y);
            if (anim) {
                // Draw animated tile
                anim_draw(anim, vec2(
                    x * map->tile_size + parallax_offset.x,
                    y * map->tile_size + parallax_offset.y
                ));
            } else {
                // Draw static tile
                image_draw_tile(map->tileset, tile_index - 1,
                    vec2i(map->tile_size, map->tile_size),
                    vec2(
                        x * map->tile_size + parallax_offset.x,
                        y * map->tile_size + parallax_offset.y
                    )
                );
            }
        }
    }
}
```

### 5.2 Viewport Culling

Only tiles visible in the viewport are drawn:

```
Map: 32x24 tiles (512x384 px at 16px tiles)
Viewport: 1280x720 px
Camera at: (100, 50)

Visible tiles:
start_x = floor(-100 / 16) = -7 (clamped to 0)
start_y = floor(-50 / 16) = -4 (clamped to 0)
end_x = 0 + ceil(1280 / 16) + 1 = 81 (clamped to 32)
end_y = 0 + ceil(720 / 16) + 1 = 47 (clamped to 24)

Draw calls: 32 * 24 = 768 tiles (worst case, full map visible)
```

---

## 6. Tile Animations

### 6.1 Animation Definition

```c
typedef struct map_anim_def {
    uint16_t tile;          // Tile index to animate
    float frame_time;       // Seconds per frame
    uint16_t sequence_len;  // Number of frames
    uint16_t sequence[];    // Frame sequence (tile indices)
} map_anim_def_t;
```

### 6.2 Setting Animations

```c
// Animate water tiles (tiles 10, 11, 12, 13)
// Tile 5 in the map will cycle through these frames
map_set_anim(map, 5, 0.2f, {10, 11, 12, 13});

// Animate a flickering torch (tiles 20, 21)
map_set_anim(map, 8, 0.1f, {20, 21, 20, 21});

// Macro expands to:
map_set_anim_with_len(map, 5, 0.2f,
    (uint16_t[]){10, 11, 12, 13}, 4);
```

### 6.3 Animation Storage

```c
// Sparse array - only store animations for tiles that have them
map->anims = calloc(map->max_tile + 1, sizeof(map_anim_def_t*));

// Set animation for tile 5
map->anims[5] = create_anim_def(...);
```

### 6.4 Drawing Animated Tiles

```c
map_anim_def_t *map_get_anim(map_t *map, int x, int y) {
    int tile = map->data[y * map->size.x + x];
    if (tile == 0) return NULL;
    return map->anims[tile];
}

// In map_draw():
map_anim_def_t *anim = map_get_anim(map, x, y);
if (anim) {
    // Calculate current frame based on time
    float time = fmod(engine.time, anim->frame_time * anim->sequence_len);
    int frame = (int)(time / anim->frame_time);
    int tile_index = anim->sequence[frame];

    image_draw_tile(map->tileset, tile_index - 1, ...);
}
```

---

## 7. Collision Maps

### 7.1 Collision Map Properties

```json
{
    "name": "collision",
    "width": 32,
    "height": 24,
    "tilesize": 16,
    "tilesetName": null,  // No visual representation
    "data": [...]
}
```

### 7.2 Tile Properties Encoding

```c
// Simple: 0 = empty, 1+ = solid
int is_solid(int tile) {
    return tile != 0;
}

// Advanced: Encode properties in tile index
#define TILE_SOLID    0x0001
#define TILE_ONE_WAY  0x0002
#define TILE_HAZARD   0x0004
#define TILE_WATER    0x0008
#define TILE_ICE      0x0010

int get_tile_properties(int tile) {
    return tile & 0xFFFF;  // Lower 16 bits
}

bool is_solid(int tile) {
    return (tile & TILE_SOLID) != 0;
}

bool is_one_way(int tile) {
    return (tile & TILE_ONE_WAY) != 0;
}
```

### 7.3 Trace Integration

```c
// trace.c - Swept AABB collision
trace_t trace(map_t *map, vec2_t from, vec2_t vel, vec2_t size) {
    trace_t result = {
        .tile = 0,
        .tile_pos = vec2i(0, 0),
        .length = 1.0f,
        .pos = vec2_add(from, vel),
        .normal = vec2(0, 0),
    };

    if (!map) return result;

    // Convert to tile space
    int tile_size = map->tile_size;
    vec2_t start = vec2_divf(from, tile_size);
    vec2_t end = vec2_divf(vec2_add(from, vel), tile_size);
    vec2_t dimensions = vec2_divf(size, tile_size);

    // Simple implementation: check start and end positions
    int start_tile_x = floorf(start.x);
    int start_tile_y = floorf(start.y);

    int end_tile_x = floorf(end.x);
    int end_tile_y = floorf(end.y);

    // Check for collision at end position
    int tile = map_tile_at(map, vec2i(end_tile_x, end_tile_y));
    if (tile != 0) {
        result.tile = tile;
        result.tile_pos = vec2i(end_tile_x, end_tile_y);

        // Calculate collision normal based on overlap
        vec2_t overlap = vec2(
            fmodf(end.x, 1.0f) < 0.5f ?
                fmodf(end.x, 1.0f) : fmodf(end.x, 1.0f) - 1.0f,
            fmodf(end.y, 1.0f) < 0.5f ?
                fmodf(end.y, 1.0f) : fmodf(end.y, 1.0f) - 1.0f
        );

        if (fabsf(overlap.x) < fabsf(overlap.y)) {
            result.normal = vec2(overlap.x < 0 ? -1 : 1, 0);
            result.pos.x = (overlap.x < 0 ?
                end_tile_x : end_tile_x + 1) * tile_size -
                (overlap.x < 0 ? size.x : 0);
        } else {
            result.normal = vec2(0, overlap.y < 0 ? -1 : 1);
            result.pos.y = (overlap.y < 0 ?
                end_tile_y : end_tile_y + 1) * tile_size -
                (overlap.y < 0 ? size.y : 0);
        }
        result.length = 0.0f;
    }

    return result;
}
```

---

## 8. Parallax Backgrounds

### 8.1 Distance Factor

```c
// distance = 1.0: Moves with camera (same speed)
// distance = 0.5: Moves at half speed (distant)
// distance = 0.0: Doesn't move (very distant, like sky)
// distance = 2.0: Moves faster than camera (foreground)

map_t *sky = map_from_json(...);
sky->distance = 0.0f;    // Static background
sky->repeat = true;      // Tile infinitely

map_t *buildings = map_from_json(...);
buildings->distance = 0.5f;  // Half speed parallax

map_t *collision = map_from_json(...);
collision->distance = 1.0f;  // Same speed as camera
```

### 8.2 Drawing with Parallax

```c
void engine_draw_background_maps(void) {
    for (int i = 0; i < engine.background_maps_len; i++) {
        map_t *map = engine.background_maps[i];

        // Apply distance factor
        vec2_t offset = vec2(
            engine.viewport.x * map->distance,
            engine.viewport.y * map->distance
        );

        map_draw(map, offset);
    }
}
```

### 8.3 Repeat Mode

```c
// For repeating backgrounds (sky, distant scenery)
if (map->repeat) {
    // Wrap tile coordinates
    int x_wrapped = ((x % map->size.x) + map->size.x) % map->size.x;
    int y_wrapped = ((y % map->size.y) + map->size.y) % map->size.y;
    return map->data[y_wrapped * map->size.x + x_wrapped];
}
```

---

## 9. Map Queries

### 9.1 Get Tile at Position

```c
// Get tile at tile coordinates
int map_tile_at(map_t *map, vec2i_t tile_pos) {
    if (tile_pos.x < 0 || tile_pos.x >= map->size.x ||
        tile_pos.y < 0 || tile_pos.y >= map->size.y) {
        return map->repeat ?
            map->data[((tile_pos.y % map->size.y + map->size.y) % map->size.y) *
                      map->size.x +
                      ((tile_pos.x % map->size.x + map->size.x) % map->size.x)]
            : 0;
    }
    return map->data[tile_pos.y * map->size.x + tile_pos.x];
}

// Get tile at pixel coordinates
int map_tile_at_px(map_t *map, vec2_t px_pos) {
    return map_tile_at(map, vec2i(
        px_pos.x / map->tile_size,
        px_pos.y / map->tile_size
    ));
}
```

### 9.2 Usage in Game Logic

```c
// Check if player is over water
vec2_t player_center = entity_center(player);
int tile = map_tile_at_px(engine.collision_map, player_center);
if (tile == TILE_WATER) {
    player->vel.x *= 0.5f;  // Slow down in water
}

// Check if enemy is on ground
vec2_t feet_pos = vec2(player->pos.x, player->pos.y + player->size.y + 1);
int ground_tile = map_tile_at_px(engine.collision_map, feet_pos);
if (ground_tile != 0) {
    player->on_ground = true;
}
```

---

## Related Documents

- **[Main Exploration](./exploration.md)** - Overall architecture
- **[Entity System Deep Dive](./entity-system-deep-dive.md)** - Entity collision
- **[Rendering Deep Dive](./rendering-deep-dive.md)** - Tile drawing
