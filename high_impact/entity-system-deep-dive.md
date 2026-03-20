---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/high_impact/src/
repository: https://github.com/phoboslab/high_impact
explored_at: 2026-03-20
language: C
parent: exploration.md
---

# High Impact Entity System - Deep Dive

**Source Files:** `entity.h`, `entity.c`, `entity_def.h`

---

## Table of Contents

1. [Entity Architecture Overview](#1-entity-architecture-overview)
2. [Entity Definition Pattern](#2-entity-definition-pattern)
3. [Entity Storage](#3-entity-storage)
4. [Entity Lifecycle](#4-entity-lifecycle)
5. [Entity Virtual Table](#5-entity-virtual-table)
6. [Entity Physics](#6-entity-physics)
7. [Entity Collision](#7-entity-collision)
8. [Entity Queries](#8-entity-queries)
9. [Entity Messaging](#9-entity-messaging)
10. [Example Entity Implementations](#10-example-entity-implementations)

---

## 1. Entity Architecture Overview

### 1.1 Design Philosophy

The entity system in high_impact follows a **hybrid ECS-like pattern**:
- **Static storage** - Pre-allocated array of entities
- **Type-based virtual table** - Function pointers per entity type
- **Component-style properties** - Struct members define behavior

### 1.2 Entity System Diagram

```
┌────────────────────────────────────────────────────────────┐
│                    ENTITY SYSTEM                            │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Static Entity Pool                       │  │
│  │  entities[ENTITIES_MAX]  (default: 1024 entities)    │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                   │
│         ┌────────────────┼────────────────┐                 │
│         │                │                │                 │
│         ▼                ▼                ▼                 │
│  ┌────────────┐   ┌────────────┐   ┌────────────┐          │
│  │  Entity 0  │   │  Entity 1  │   │  Entity N  │          │
│  │  type=0    │   │  type=1    │   │  type=2    │          │
│  │  is_alive  │   │  is_alive  │   │  is_alive  │          │
│  │  pos, vel  │   │  pos, vel  │   │  pos, vel  │          │
│  │  ...       │   │  ...       │   │  ...       │          │
│  └─────┬──────┘   └─────┬──────┘   └─────┬──────┘          │
│        │                │                │                   │
│        └────────────────┼────────────────┘                   │
│                         │                                     │
│                         ▼                                     │
│            ┌────────────────────────┐                        │
│            │   Virtual Table (vtab) │                        │
│            │   entity_vtab[type]    │                        │
│            │   - init()             │                        │
│            │   - update()           │                        │
│            │   - draw()             │                        │
│            │   - collide()          │                        │
│            │   - damage()           │                        │
│            └────────────────────────┘                        │
│                                                              │
└────────────────────────────────────────────────────────────┘
```

---

## 2. Entity Definition Pattern

### 2.1 X-Macro Type Definition

Entity types are defined using an **X-Macro pattern**:

```c
// 1. Define all entity types
#define ENTITY_TYPES() \
    X(PLAYER)          \
    X(ENEMY_BASIC)     \
    X(ENEMY_SHOOTER)   \
    X(BULLET_PLAYER)   \
    X(BULLET_ENEMY)    \
    X(PARTICLE)        \
    X(ITEM_HEALTH)     \
    X(DOOR)            \
    X(TRIGGER)         \
    X(COUNT)           // Must be last!
```

This generates:
```c
typedef enum {
    ENTITY_PLAYER,
    ENTITY_ENEMY_BASIC,
    ENTITY_ENEMY_SHOOTER,
    ENTITY_BULLET_PLAYER,
    ENTITY_BULLET_ENEMY,
    ENTITY_PARTICLE,
    ENTITY_ITEM_HEALTH,
    ENTITY_DOOR,
    ENTITY_TRIGGER,
    ENTITY_TYPES_COUNT
} entity_type_t;
```

### 2.2 Entity Struct Definition

```c
// entity_def.h - Base entity structure
typedef struct entity {
    entity_type_t type;      // Entity type enum
    entity_ref_t ref;        // Stable reference handle
    uint16_t index;          // Index in entity pool
    bool is_alive;           // Alive flag
    bool on_ground;          // Grounded check

    vec2_t pos;              // Position (pixels)
    vec2_t vel;              // Velocity (pixels/frame)
    vec2_t size;             // Collision size
    vec2_t offset;           // Visual offset from pos

    float friction;          // Surface friction (0-1)
    float restitution;       // Bounciness (0-1)
    float gravity;           // Gravity multiplier
    float max_velocity_x;    // Max horizontal speed
    float max_velocity_y;    // Max vertical speed

    int health;              // Current health
    int damage;              // Damage dealt on touch
    float invincible_time;   // Invincibility timer

    anim_t *anim;            // Current animation
    entity_vtab_t *vtab;     // Virtual table pointer

    // Entity-specific data (variable size)
    // ... custom fields defined per-type
} entity_t;
```

### 2.3 ENTITY_DEFINE Macro

```c
// In your game code, before including entity.h:
#define ENTITY_DEFINE \
    vec2_t pos; \
    vec2_t vel; \
    vec2_t size; \
    vec2_t offset; \
    float friction; \
    float restitution; \
    float gravity; \
    float max_velocity_x; \
    float max_velocity_y; \
    int health; \
    int damage; \
    float invincible_time; \
    anim_t *anim; \
    /* Custom fields: */ \
    float shoot_timer; \
    int shoot_interval; \
    uint8_t direction;

#include "entity.h"
```

---

## 3. Entity Storage

### 3.1 Static Pool Allocation

```c
// entity.c
static entity_t entities[ENTITIES_MAX];
static bool entity_slots_occupied[ENTITIES_MAX];
static int entity_count = 0;

// Spatial partitioning for queries
static entity_t *entities_by_x[ENTITIES_MAX];  // Sorted by X
static entity_t *entities_by_type[ENTITY_TYPES_COUNT][ENTITIES_MAX];
```

### 3.2 Entity References

Entities use **reference handles** instead of raw pointers:

```c
typedef struct {
    uint16_t index;    // Slot index
    uint16_t cookie;   // Validation cookie
} entity_ref_t;
```

**Benefits:**
- Stable across entity reordering
- Detects dangling references (cookie mismatch)
- Can be serialized for save games

### 3.3 Reference Functions

```c
// Get reference from entity
entity_ref_t entity_ref(entity_t *self) {
    return (entity_ref_t){
        .index = self->index,
        .cookie = self->ref.cookie
    };
}

// Get entity from reference
entity_t *entity_by_ref(entity_ref_t ref) {
    if (ref.index >= ENTITIES_MAX) return NULL;
    entity_t *ent = &entities[ref.index];
    if (!ent->is_alive || ent->ref.cookie != ref.cookie) return NULL;
    return ent;
}
```

---

## 4. Entity Lifecycle

### 4.1 Spawning

```c
entity_t *entity_spawn(entity_type_t type, vec2_t pos) {
    // Find free slot
    int slot = -1;
    for (int i = 0; i < ENTITIES_MAX; i++) {
        if (!entity_slots_occupied[i]) {
            slot = i;
            break;
        }
    }
    if (slot == -1) return NULL;  // Pool full

    // Initialize entity
    entity_t *ent = &entities[slot];
    memset(ent, 0, sizeof(entity_t));

    ent->type = type;
    ent->index = slot;
    ent->ref = (entity_ref_t){.index = slot, .cookie = rand()};
    ent->is_alive = true;
    ent->pos = pos;

    entity_slots_occupied[slot] = true;
    entity_count++;

    // Call type-specific init
    if (entity_vtab[type].init) {
        entity_vtab[type].init(ent);
    }

    return ent;
}
```

### 4.2 Settings (Level JSON)

When loaded from a level:

```c
// Called after all entities are spawned
void entity_settings_from_json(entity_t *ent, json_t *def) {
    if (entity_vtab[ent->type].settings) {
        entity_vtab[ent->type].settings(ent, def);
    }
}
```

**Example JSON:**
```json
{
    "type": "enemy_shooter",
    "x": 200,
    "y": 100,
    "settings": {
        "shoot_interval": 60,
        "health": 3,
        "name": "guard_1"
    }
}
```

### 4.3 Update Loop

```c
void entities_update(void) {
    for (int i = 0; i < ENTITIES_MAX; i++) {
        entity_t *ent = &entities[i];
        if (!ent->is_alive) continue;

        // Call type-specific update
        if (entity_vtab[ent->type].update) {
            entity_vtab[ent->type].update(ent);
        }
    }
}
```

### 4.4 Kill and Cleanup

```c
void entity_kill(entity_t *ent) {
    ent->is_alive = false;

    if (entity_vtab[ent->type].kill) {
        entity_vtab[ent->type].kill(ent);
    }
}

void entities_cleanup(void) {
    // Remove dead entities
    for (int i = 0; i < ENTITIES_MAX; i++) {
        if (!entities[i].is_alive) {
            entity_slots_occupied[i] = false;
            entity_count--;
        }
    }
}
```

---

## 5. Entity Virtual Table

### 5.1 VTab Structure

```c
typedef struct {
    void (*load)(void);           // One-time type init
    void (*init)(entity_t *self); // Per-entity init
    void (*settings)(entity_t*, json_t*);  // JSON settings
    void (*update)(entity_t *self);
    void (*draw)(entity_t*, vec2_t viewport);
    void (*kill)(entity_t *self);
    void (*touch)(entity_t*, entity_t *other);
    void (*collide)(entity_t*, vec2_t normal, trace_t*);
    void (*damage)(entity_t*, entity_t *other, float damage);
    void (*trigger)(entity_t*, entity_t *other);
    void (*message)(entity_t*, entity_message_t, void*);
} entity_vtab_t;
```

### 5.2 VTab Array

```c
entity_vtab_t entity_vtab[ENTITY_TYPES_COUNT] = {
    [ENTITY_PLAYER] = {
        .load = player_load,
        .init = player_init,
        .update = player_update,
        .draw = player_draw,
        .collide = player_collide,
        .damage = player_damage,
    },
    [ENTITY_ENEMY_SHOOTER] = {
        .load = enemy_shooter_load,
        .init = enemy_shooter_init,
        .update = enemy_shooter_update,
        .draw = enemy_shooter_draw,
        .kill = enemy_shooter_death,
        .collide = enemy_shooter_collide,
        .damage = enemy_shooter_damage,
    },
    // ...
};
```

### 5.3 Macro Wrappers

```c
#define entity_init(ENTITY) \
    entity_vtab[ENTITY->type].init(ENTITY)

#define entity_update(ENTITY) \
    entity_vtab[ENTITY->type].update(ENTITY)

#define entity_draw(ENTITY, VIEWPORT) \
    entity_vtab[ENTITY->type].draw(ENTITY, VIEWPORT)

#define entity_kill(ENTITY) \
    (ENTITY->is_alive = false, \
     entity_vtab[ENTITY->type].kill(ENTITY))

#define entity_collide(ENTITY, NORMAL, TRACE) \
    entity_vtab[ENTITY->type].collide(ENTITY, NORMAL, TRACE)

#define entity_damage(ENTITY, OTHER, DAMAGE) \
    entity_vtab[ENTITY->type].damage(ENTITY, OTHER, DAMAGE)
```

---

## 6. Entity Physics

### 6.1 Base Update Function

```c
void entity_base_update(entity_t *self) {
    // Apply gravity
    if (self->gravity != 0) {
        self->vel.y += engine.gravity * self->gravity * engine.tick;
    }

    // Apply friction
    if (self->on_ground) {
        self->vel.x *= self->friction;
    }

    // Clamp velocity
    if (self->max_velocity_x != 0) {
        self->vel.x = clamp(self->vel.x,
            -self->max_velocity_x, self->max_velocity_x);
    }
    if (self->max_velocity_y != 0) {
        self->vel.y = clamp(self->vel.y,
            -self->max_velocity_y, self->max_velocity_y);
    }

    // Trace movement
    trace_t tr = trace(engine.collision_map,
                       self->pos, self->vel, self->size);

    if (tr.tile != 0) {
        // Collision detected
        self->pos = tr.pos;

        // Check for bounce
        if (self->restitution > 0 &&
            vec2_len(self->vel) > ENTITY_MIN_BOUNCE_VELOCITY) {
            self->vel = vec2_mul(self->vel, -self->restitution);
        } else {
            self->vel = vec2(0, 0);
        }

        self->on_ground = tr.normal.y > 0;

        if (entity_vtab[self->type].collide) {
            entity_vtab[self->type].collide(self, tr.normal, &tr);
        }
    } else {
        // No collision, apply full movement
        self->pos = vec2_add(self->pos, self->vel);
        self->on_ground = false;
    }
}
```

### 6.2 Physics Properties

| Property | Purpose | Typical Values |
|----------|---------|----------------|
| `friction` | Ground friction | 0.7-0.95 |
| `restitution` | Bounciness | 0.0-0.5 |
| `gravity` | Gravity multiplier | 0.0-2.0 |
| `max_velocity_x` | Max horizontal speed | 5-15 |
| `max_velocity_y` | Max vertical speed | 10-20 |

---

## 7. Entity Collision

### 7.1 Entity vs. World

Handled by `entity_base_update()` using `trace()`.

### 7.2 Entity vs. Entity

Uses **broad phase + narrow phase**:

```c
void entities_check_collisions(void) {
    // Broad phase: sweep and prune on X axis
    sort_entities_by_x();

    for (int i = 0; i < entity_count; i++) {
        entity_t *a = entities_by_x[i];
        if (!a->is_alive) continue;

        for (int j = i + 1; j < entity_count; j++) {
            entity_t *b = entities_by_x[j];
            if (!b->is_alive) break;

            // Early out on X axis
            if (b->pos.x > a->pos.x + a->size.x + ENTITY_MAX_SIZE) {
                break;
            }

            // Check collision mask
            if (!(a->check_against & (1 << b->type))) continue;

            // Narrow phase: AABB overlap
            if (entity_is_touching(a, b)) {
                entity_touch(a, b);
                entity_touch(b, a);
            }
        }
    }
}
```

### 7.3 Touch Callback

```c
void entity_touch(entity_t *self, entity_t *other) {
    if (entity_vtab[self->type].touch) {
        entity_vtab[self->type].touch(self, other);
    }
}
```

---

## 8. Entity Queries

### 8.1 Proximity Query

```c
entity_list_t entities_by_proximity(
    entity_t *ent,
    float radius,
    entity_type_t type
) {
    entity_list_t list = {0};

    for (int i = 0; i < ENTITIES_MAX; i++) {
        entity_t *other = &entities[i];
        if (!other->is_alive) continue;
        if (type != ENTITY_TYPE_NONE && other->type != type) continue;

        if (entity_dist(ent, other) <= radius) {
            list.entities[list.count++] = other;
        }
    }

    return list;
}
```

### 8.2 Location Query

```c
entity_list_t entities_by_location(
    vec2_t pos,
    float radius,
    entity_type_t type,
    entity_t *exclude
) {
    entity_list_t list = {0};

    for (int i = 0; i < ENTITIES_MAX; i++) {
        entity_t *other = &entities[i];
        if (!other->is_alive) continue;
        if (other == exclude) continue;
        if (type != ENTITY_TYPE_NONE && other->type != type) continue;

        if (vec2_dist(pos, other->pos) <= radius) {
            list.entities[list.count++] = other;
        }
    }

    return list;
}
```

### 8.3 Query Results

```c
typedef struct {
    entity_t *entities[64];  // Max results
    int count;
} entity_list_t;

// Usage
entity_list_t nearby = entities_by_proximity(player, 100, ENTITY_ENEMY_BASIC);
for (int i = 0; i < nearby.count; i++) {
    entity_t *enemy = nearby.entities[i];
    // Process enemy
}
```

---

## 9. Entity Messaging

### 9.1 Message Enum

```c
typedef enum {
    MESSAGE_NONE,
    MESSAGE_SHOOT,
    MESSAGE_EXPLODE,
    MESSAGE_PLAY_SOUND,
    MESSAGE_SET_ANIMATION,
    // Custom messages...
} entity_message_t;
```

### 9.2 Message Dispatch

```c
void entity_message(entity_t *self, entity_message_t msg, void *data) {
    if (entity_vtab[self->type].message) {
        entity_vtab[self->type].message(self, msg, data);
    }
}
```

### 9.3 Example Message Handler

```c
void enemy_shooter_message(entity_t *self, entity_message_t msg, void *data) {
    switch (msg) {
        case MESSAGE_SHOOT:
            entity_spawn(ENTITY_BULLET_ENEMY,
                        vec2(self->pos.x, self->pos.y + 10));
            break;
        case MESSAGE_EXPLODE:
            // Spawn particles
            for (int i = 0; i < 10; i++) {
                entity_t *p = entity_spawn(ENTITY_PARTICLE, self->pos);
                p->vel = vec2(randf(-5, 5), randf(-5, 5));
            }
            break;
    }
}
```

---

## 10. Example Entity Implementations

### 10.1 Player Entity

```c
typedef enum {
    PLAYER_IDLE,
    PLAYER_WALK,
    PLAYER_JUMP,
    PLAYER_ATTACK,
} player_anim_t;

void player_init(entity_t *self) {
    self->size = vec2(16, 24);
    self->friction = 0.8f;
    self->gravity = 1.0f;
    self->max_velocity_x = 8.0f;
    self->max_velocity_y = 15.0f;
    self->health = 3;

    self->anim = anim(player_anims[PLAYER_IDLE]);
}

void player_update(entity_t *self) {
    // Input handling
    float move = 0;
    if (input_state(ACTION_LEFT)) move = -1;
    if (input_state(ACTION_RIGHT)) move = 1;

    self->vel.x = move * self->max_velocity_x;

    // Jump
    if (input_pressed(ACTION_JUMP) && self->on_ground) {
        self->vel.y = -self->max_velocity_y;
    }

    // Update animation
    if (move != 0) {
        self->anim->flip_x = move < 0;
        self->anim->def = player_anims[PLAYER_WALK];
    } else {
        self->anim->def = player_anims[PLAYER_IDLE];
    }

    // Base physics
    entity_base_update(self);
}

void player_collide(entity_t *self, vec2_t normal, trace_t *trace) {
    if (normal.y < 0) {
        // Hit ceiling
        self->vel.y = 0;
    }
}
```

### 10.2 Enemy Shooter

```c
typedef struct {
    entity_t base;
    float shoot_timer;
    int shoot_interval;
    uint8_t direction;
    int patrol_distance;
    float patrol_start_x;
} enemy_shooter_t;

void enemy_shooter_init(entity_t *self) {
    enemy_shooter_t *e = (enemy_shooter_t *)self;

    self->size = vec2(16, 16);
    self->friction = 0.9f;
    self->max_velocity_x = 3.0f;
    self->health = 3;

    e->shoot_interval = 60;  // Frames
    e->shoot_timer = 0;
    e->direction = 1;
    e->patrol_start_x = self->pos.x;

    self->anim = anim(enemy_anims[ENEMY_WALK]);
}

void enemy_shooter_update(entity_t *self) {
    enemy_shooter_t *e = (enemy_shooter_t *)self;

    // Patrol movement
    if (fabsf(self->pos.x - e->patrol_start_x) > e->patrol_distance) {
        e->direction = -e->direction;
    }
    self->vel.x = e->direction * self->max_velocity_x;

    // Shooting logic
    e->shoot_timer++;
    if (e->shoot_timer >= e->shoot_interval) {
        e->shoot_timer = 0;

        // Check if player is in range
        entity_list_t players = entities_by_proximity(
            self, 200, ENTITY_PLAYER);
        if (players.count > 0) {
            entity_message(self, MESSAGE_SHOOT, NULL);
        }
    }

    // Animation
    self->anim->flip_x = e->direction < 0;

    entity_base_update(self);
}

void enemy_shooter_damage(entity_t *self, entity_t *other, float damage) {
    self->health -= damage;

    // Flash effect
    self->anim->color = rgba(255, 128, 128, 255);

    if (self->health <= 0) {
        entity_kill(self);
    }
}

void enemy_shooter_death(entity_t *self) {
    // Spawn particles
    for (int i = 0; i < 8; i++) {
        entity_t *p = entity_spawn(ENTITY_PARTICLE, self->pos);
        p->vel = vec2(randf(-4, 4), randf(-4, 4));
    }

    // Drop item chance
    if (randf(0, 1) < 0.25f) {
        entity_spawn(ENTITY_ITEM_HEALTH, self->pos);
    }
}
```

---

## Related Documents

- **[Main Exploration](./exploration.md)** - Overall architecture
- **[Physics Deep Dive](./physics-deep-dive.md)** - Trace and collision details
- **[Animation Deep Dive](./animation-deep-dive.md)** - Animation system
