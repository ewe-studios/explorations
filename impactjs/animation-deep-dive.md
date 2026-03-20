---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/Impact/lib/impact
explored_at: 2026-03-20
---

# Impact.js Animation System - Deep Dive

**Scope:** Sprite animations, animation sheets, frame timing, sequence arrays, flip/rotation, animated tiles

---

## Table of Contents

1. [Animation Architecture Overview](#1-animation-architecture-overview)
2. [Animation Sheet Structure](#2-animation-sheet-structure)
3. [Animation Class Internals](#3-animation-class-internals)
4. [Frame Timing System](#4-frame-timing-system)
5. [Sequence Arrays and Looping](#5-sequence-arrays-and-looping)
6. [Animation States and Transitions](#6-animation-states-and-transitions)
7. [Flip and Rotation System](#7-flip-and-rotation-system)
8. [Entity Animation Integration](#8-entity-animation-integration)
9. [Animated Tiles System](#9-animated-tiles-system)
10. [Animation Performance](#10-animation-performance)
11. [Practical Examples](#11-practical-examples)

---

## 1. Animation Architecture Overview

### 1.1 Component Relationship

```
┌─────────────────────────────────────────────────────────────────┐
│                    ANIMATION HIERARCHY                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ig.Entity                                                       │
│     │                                                            │
│     ├─► animSheet: ig.AnimationSheet                            │
│     │   └─► image: ig.Image (sprite sheet)                      │
│     │   └─► tileWidth, tileHeight                               │
│     │                                                            │
│     ├─► anims: { [name: string]: ig.Animation }                 │
│     │   ├─► idle: ig.Animation                                  │
│     │   ├─► run: ig.Animation                                   │
│     │   └─► jump: ig.Animation                                  │
│     │                                                            │
│     └─► currentAnim: ig.Animation (currently playing)           │
│         └─► Updates via update()                                │
│         └─► Renders via draw()                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Animation Data Flow

```
Sprite Sheet (PNG)
       │
       ▼
ig.AnimationSheet
  └─► Loads image
  └─► Defines tile size
       │
       ▼
ig.Animation
  └─► References sheet
  └─► Defines sequence [0, 1, 2, 3]
  └─► Defines frameTime (seconds per frame)
       │
       ▼
ig.Entity.anims['run']
  └─► Named animation registry
       │
       ▼
entity.currentAnim
  └─► Currently active animation
  └─► Updated each frame
  └─► Drawn at entity position
```

---

## 2. Animation Sheet Structure

### 2.1 AnimationSheet Class

```javascript
ig.AnimationSheet = ig.Class.extend({
    width: 8,
    height: 8,
    image: null,

    init: function( path, tileWidth, tileHeight ) {
        this.tileWidth = tileWidth || 8;
        this.tileHeight = tileHeight || 8;
        this.image = new ig.Image( path );
    }
});
```

### 2.2 Sprite Sheet Layout

```
Sprite Sheet: gfx/player.png
┌────────────────────────────────────────────────────────────┐
│                                                            │
│  Tile 0   Tile 1   Tile 2   Tile 3                        │
│  ┌───┐   ┌───┐   ┌───┐   ┌───┐                           │
│  │ ░ │   │ ▒ │   │ ▓ │   │ █ │  Row 0 (y=0)              │
│  └───┘   └───┘   └───┘   └───┘                           │
│     ▲       ▲       ▲       ▲                              │
│     │       │       │       │                              │
│   x=0     x=16    x=32    x=48  (tileWidth=16)            │
│                                                            │
│  Tile 4   Tile 5   Tile 6   Tile 7                        │
│  ┌───┐   ┌───┐   ┌───┐   ┌───┐                           │
│  │ ░ │   │ ▒ │   │ ▓ │   │ █ │  Row 1 (y=16)             │
│  └───┘   └───┘   └───┘   └───┘                           │
│                                                            │
│  ...                                                       │
│                                                            │
└────────────────────────────────────────────────────────────┘
      ▲
      │
  tileHeight=16

Sheet dimensions: 64x64 pixels
Tiles per row: 64 / 16 = 4 tiles
Total tiles: (64/16) * (64/16) = 16 tiles
```

### 2.3 Tile Index Calculation

```javascript
// Given a tile index, calculate position in sprite sheet
function getTilePosition( index, tileWidth, sheetWidth ) {
    return {
        x: Math.floor( index * tileWidth ) % sheetWidth,
        y: Math.floor( index * tileWidth / sheetWidth ) * tileHeight
    };
}

// Example: Tile 5 in 64x64 sheet with 16x16 tiles
getTilePosition( 5, 16, 64 )
  → x: (5 * 16) % 64 = 80 % 64 = 16
  → y: (5 * 16 / 64) * 16 = 1.25 * 16 = 16
  → Position: (16, 16) ✓ (Tile 5 in diagram above)
```

### 2.4 Creating an AnimationSheet

```javascript
// In your entity definition
EntityPlayer = ig.Entity.extend({
    animSheet: null,

    init: function( x, y, settings ) {
        this.parent( x, y, settings );

        // Load sprite sheet
        this.animSheet = new ig.AnimationSheet(
            'gfx/player.png',  // Path to sprite sheet
            16,                 // Tile width in pixels
            16                  // Tile height in pixels
        );
    }
});
```

---

## 3. Animation Class Internals

### 3.1 Animation Properties

```javascript
ig.Animation = ig.Class.extend({
    // Configuration
    sheet: null,           // Reference to AnimationSheet
    frameTime: 0.05,       // Seconds per frame
    sequence: [],          // Array of tile indices
    stop: false,           // Stop on last frame

    // State
    timer: null,           // ig.Timer for frame timing
    frame: 0,              // Current frame index (0 to sequence.length-1)
    tile: 0,               // Current tile index (from sequence[frame])
    loopCount: 0,          // Number of complete loops
    flip: {x: false, y: false},  // Flip state
    angle: 0,              // Rotation angle (radians)

    // Callbacks
    loop: null,            // Called on each loop
    stopCallback: null,    // Called when animation stops
});
```

### 3.2 Animation Initialization

```javascript
ig.Animation = ig.Class.extend({
    init: function( sheet, frameTime, sequence, stop ) {
        this.sheet = sheet;
        this.frameTime = frameTime;
        this.sequence = sequence;
        this.stop = stop;
        this.timer = new ig.Timer();
        this.frame = 0;
        this.tile = this.sequence[this.frame];
        this.loopCount = 0;
    }
});
```

### 3.3 Memory Efficiency

Note that animations use **reference semantics**:

```javascript
// BAD: Creates new AnimationSheet instance
EntityPlayer = ig.Entity.extend({
    init: function() {
        this.animSheet = new ig.AnimationSheet('gfx/player.png', 16, 16);
    }
});
EntityEnemy = ig.Entity.extend({
    init: function() {
        this.animSheet = new ig.AnimationSheet('gfx/player.png', 16, 16);  // Duplicate!
    }
});

// GOOD: Share AnimationSheet
var playerSheet = new ig.AnimationSheet('gfx/player.png', 16, 16);

EntityPlayer = ig.Entity.extend({
    animSheet: playerSheet  // Shared reference
});
EntityEnemy = ig.Entity.extend({
    animSheet: playerSheet  // Same reference, no duplicate load
});
```

---

## 4. Frame Timing System

### 4.1 Timer-Based Frame Advancement

```javascript
ig.Animation.prototype.update = function() {
    // Calculate how many frames should have passed
    var frameTotal = Math.floor( this.timer.delta() / this.frameTime );

    // Calculate loop count and current frame
    this.loopCount = Math.floor( frameTotal / this.sequence.length );
    this.frame = frameTotal % this.sequence.length;

    // Get the tile index for current frame
    this.tile = this.sequence[ this.frame ];

    // Handle animation completion
    if( this.stop && this.frame == this.sequence.length - 1 ) {
        this.timer.set( 0 );  // Reset timer
        if( this.stopCallback ) {
            this.stopCallback();
        }
    }
};
```

### 4.2 Timer Mechanics

```javascript
// ig.Timer provides delta time
ig.Timer = ig.Class.extend({
    target: 0,
    base: 0,

    init: function( seconds ) {
        this.base = ig.Timer.time;
        this.target = seconds || 0;
    },

    // Time elapsed since last tick
    tick: function() {
        var delta = ig.Timer.time - this.last;
        this.last = ig.Timer.time;
        return delta;
    },

    // Time since animation started
    delta: function() {
        return (ig.Timer.time - this.base - this.target);
    },

    // Reset timer
    set: function( seconds ) {
        this.target = seconds || 0;
        this.base = ig.Timer.time;
    }
});
```

### 4.3 Frame Timing Example

```
Animation: Run cycle
Frame time: 0.1 seconds (100ms per frame)
Sequence: [0, 1, 2, 3] (4 frames)

Time (ms) | Timer.delta | frameTotal | loopCount | frame | tile
──────────┼─────────────┼────────────┼───────────┼───────┼─────
    0     │      0      │     0      │     0     │   0   │  0
   50     │     50      │     0      │     0     │   0   │  0
  100     │    100      │     1      │     0     │   1   │  1
  150     │    150      │     1      │     0     │   1   │  1
  200     │    200      │     2      │     0     │   2   │  2
  300     │    300      │     3      │     0     │   3   │  3
  400     │    400      │     4      │     1     │   0   │  0  ← Loop!
  500     │    500      │     5      │     1     │   1   │  1

// Calculation at t=400ms:
frameTotal = floor(400 / 100) = 4
loopCount = floor(4 / 4) = 1
frame = 4 % 4 = 0
tile = sequence[0] = 0
```

### 4.4 Frame Time and FPS Relationship

```
Frame Time    →    Animation FPS
────────────      ───────────────
0.016 (16ms)  →   60 FPS (very fast)
0.033 (33ms)  →   30 FPS (smooth)
0.05  (50ms)  →   20 FPS (standard)
0.1   (100ms) →   10 FPS (deliberate)
0.2   (200ms) →    5 FPS (slow)

// For a 4-frame animation at 60 FPS game loop:
// - 0.05s frameTime = each frame displays for 3 game frames
// - 0.1s frameTime = each frame displays for 6 game frames
```

---

## 5. Sequence Arrays and Looping

### 5.1 Sequence Array Structure

```javascript
// Simple 4-frame loop
var runAnim = new ig.Animation( sheet, 0.1, [0, 1, 2, 3], false );

// Ping-pong pattern (forward then reverse)
var breathAnim = new ig.Animation( sheet, 0.15, [0, 1, 2, 1], false );

// Non-sequential frames (skip frames)
var attackAnim = new ig.Animation( sheet, 0.08, [0, 2, 4, 5], false );

// Single frame (static)
var idleAnim = new ig.Animation( sheet, 1, [0], false );

// One-shot animation (stops on last frame)
var deathAnim = new ig.Animation( sheet, 0.1, [0, 1, 2, 3, 4, 5], true );
```

### 5.2 Sequence Pattern Examples

```
Sprite Sheet: 8 frames of player animation
┌────┬────┬────┬────┬────┬────┬────┬────┐
│ 0  │ 1  │ 2  │ 3  │ 4  │ 5  │ 6  │ 7  │
│Idle│Run1│Run2│Run3│Jump│Fall│Hit │Die │
└────┴────┴────┴────┴────┴────┴────┴────┘

Idle Animation (subtle breathing):
  sequence: [0, 0, 0, 1]  // Hold frame 0, briefly show 1
  frameTime: 0.2

Run Animation (continuous):
  sequence: [1, 2, 3]  // Skip idle frame
  frameTime: 0.08
  loops forever

Jump Animation (one-shot):
  sequence: [4]  // Single frame
  frameTime: 0.1
  stop: true

Death Animation (one-shot, ends on last frame):
  sequence: [5, 6, 7]
  frameTime: 0.15
  stop: true
  stopCallback: function() { entity.remove(); }
```

### 5.3 Loop Callback

```javascript
ig.Animation.prototype.loop = function() {
    // Called every time the animation loops
};

// Usage: Play sound on loop
entity.anims['run'].loop = function() {
    ig.game.sound.play('footstep');
};
```

### 5.4 Stop Callback

```javascript
// Usage: Remove entity after death animation
entity.anims['die'].stopCallback = function() {
    entity.remove();
};

// Or trigger next state
entity.anims['attack'].stopCallback = function() {
    entity.currentAnim = entity.anims['idle'];
};
```

---

## 6. Animation States and Transitions

### 6.1 Animation Registry

```javascript
EntityPlayer = ig.Entity.extend({
    animSheet: null,

    init: function( x, y, settings ) {
        this.parent( x, y, settings );

        // Load sprite sheet
        this.animSheet = new ig.AnimationSheet( 'gfx/player.png', 16, 16 );

        // Define animations
        this.addAnim( 'idle', 0.2, [0] );
        this.addAnim( 'run', 0.08, [1, 2, 3] );
        this.addAnim( 'jump', 0.1, [4] );
        this.addAnim( 'fall', 0.1, [5] );
        this.addAnim( 'hit', 0.1, [6], true );  // One-shot
        this.addAnim( 'die', 0.15, [0, 1, 2, 3, 4, 5, 6, 7], true );
    }
});
```

### 6.2 addAnim Helper Method

```javascript
ig.Entity.prototype.addAnim = function( name, frameTime, sequence, stop ) {
    if( !this.animSheet ) {
        throw( 'No animSheet to add the animation '+name+' to.' );
    }

    var a = new ig.Animation( this.animSheet, frameTime, sequence, stop );
    this.anims[name] = a;

    // Set as current if none exists
    if( !this.currentAnim ) {
        this.currentAnim = a;
    }

    return a;
};
```

### 6.3 State Machine Pattern

```javascript
EntityPlayer = ig.Entity.extend({
    state: 'idle',

    update: function() {
        // State transitions
        if ( ig.input.state('left') || ig.input.state('right') ) {
            if ( this.state !== 'run' ) {
                this.state = 'run';
                this.currentAnim = this.anims['run'];
            }
        }
        else if ( !this.standing ) {
            if ( this.vel.y < 0 ) {
                if ( this.state !== 'jump' ) {
                    this.state = 'jump';
                    this.currentAnim = this.anims['jump'];
                }
            } else {
                if ( this.state !== 'fall' ) {
                    this.state = 'fall';
                    this.currentAnim = this.anims['fall'];
                }
            }
        }
        else {
            if ( this.state !== 'idle' ) {
                this.state = 'idle';
                this.currentAnim = this.anims['idle'];
            }
        }

        this.parent();
    }
});
```

### 6.4 Animation Blending (Manual Implementation)

Impact.js doesn't support animation blending natively. Implement manually:

```javascript
EntityPlayer = ig.Entity.extend({
    prevAnim: null,
    blendFactor: 0,  // 0 = prevAnim, 1 = currentAnim
    blendDuration: 0.1,

    setAnimation: function( animName, blend ) {
        if ( blend ) {
            this.prevAnim = this.currentAnim;
            this.currentAnim = this.anims[animName];
            this.blendFactor = 0;
        } else {
            this.currentAnim = this.anims[animName];
        }
    },

    update: function() {
        this.parent();

        // Handle blending
        if ( this.prevAnim ) {
            this.blendFactor += ig.system.tick / this.blendDuration;
            if ( this.blendFactor >= 1 ) {
                this.prevAnim = null;
                this.blendFactor = 0;
            }
        }
    },

    draw: function() {
        if ( this.prevAnim && this.blendFactor < 1 ) {
            // Draw both animations with alpha blending
            var ctx = ig.system.context;
            ctx.save();
            ctx.globalAlpha = 1 - this.blendFactor;
            this.prevAnim.draw( this.pos.x - ig.game._rscreen.x, this.pos.y );
            ctx.globalAlpha = this.blendFactor;
            this.currentAnim.draw( this.pos.x - ig.game._rscreen.x, this.pos.y );
            ctx.restore();
        } else {
            this.currentAnim.draw( this.pos.x - ig.game._rscreen.x, this.pos.y );
        }
    }
});
```

---

## 7. Flip and Rotation System

### 7.1 Flip Properties

```javascript
ig.Animation = ig.Class.extend({
    flip: {
        x: false,  // Flip horizontally
        y: false   // Flip vertically
    },
    angle: 0  // Rotation in radians
});
```

### 7.2 Flipping for Direction

```javascript
EntityPlayer = ig.Entity.extend({
    update: function() {
        this.parent();

        // Face direction of movement
        if ( this.vel.x > 0 ) {
            this.currentAnim.flip.x = false;
        }
        else if ( this.vel.x < 0 ) {
            this.currentAnim.flip.x = true;
        }

        // Flip animation when falling
        this.currentAnim.flip.y = ( this.vel.y > 0 );
    }
});
```

### 7.3 Visual Effect of Flip

```
Original:                flip.x = true:
┌──────────┐             ┌──────────┐
│   →      │             │      ←   │
│  Player  │  flipX()    │  Player  │
│   ◢◣     │   ─────▶    │    ◥◤    │
└──────────┘             └──────────┘

Original:                flip.y = true:
┌──────────┐             ┌──────────┐
│    ↑     │             │    ↓     │
│  Player  │  flipY()    │  Player  │
│   ◢◣     │   ─────▶    │   ◥▴     │
└──────────┘             └──────────┘
```

### 7.4 Rotation

```javascript
// Set animation angle (in radians)
entity.currentAnim.angle = Math.PI / 4;  // 45 degrees

// Rotate based on velocity (e.g., rolling)
var angle = Math.atan2( this.vel.y, this.vel.x );
this.currentAnim.angle = angle;
```

### 7.5 Combined Flip and Rotation

```javascript
ig.Animation.prototype.draw = function( x, y ) {
    if ( this.flip.x || this.flip.y || this.angle ) {
        ig.system.context.save();

        var w = this.frameWidth * ig.system.scale;
        var h = this.frameHeight * ig.system.scale;

        // Translate to center
        ig.system.context.translate( x + w/2, y + h/2 );

        // Apply rotation
        if ( this.angle ) {
            ig.system.context.rotate( this.angle );
        }

        // Apply flip
        ig.system.context.scale(
            this.flip.x ? -1 : 1,
            this.flip.y ? -1 : 1
        );

        // Draw centered
        this.sheet.image.drawTile(
            -w/2, -h/2,
            this.tile,
            this.frameWidth,
            this.frameHeight
        );

        ig.system.context.restore();
    } else {
        this.sheet.image.drawTile( x, y, this.tile, this.frameWidth, this.frameHeight );
    }
};
```

---

## 8. Entity Animation Integration

### 8.1 Entity Update Cycle

```javascript
ig.Entity.prototype.update = function() {
    // Save previous position
    this.last.x = this.pos.x;
    this.last.y = this.pos.y;

    // Apply gravity
    this.vel.y += ig.game.gravity * ig.system.tick * this.gravityFactor;

    // Apply acceleration and friction
    this.vel.x = this.getNewVelocity( this.vel.x, this.accel.x, this.friction.x, this.maxVel.x );
    this.vel.y = this.getNewVelocity( this.vel.y, this.accel.y, this.friction.y, this.maxVel.y );

    // Movement and collision
    var mx = this.vel.x * ig.system.tick;
    var my = this.vel.y * ig.system.tick;
    var res = ig.game.collisionMap.trace(
        this.pos.x, this.pos.y, mx, my, this.size.x, this.size.y
    );
    this.handleMovementTrace( res );

    // Update animation
    if ( this.currentAnim ) {
        this.currentAnim.update();
    }
};
```

### 8.2 Entity Draw Cycle

```javascript
ig.Entity.prototype.draw = function() {
    if ( this.currentAnim ) {
        this.currentAnim.draw(
            this.pos.x - ig.game._rscreen.x,
            this.pos.y - ig.game._rscreen.y
        );
    }
};
```

### 8.3 Complete Entity Example

```javascript
EntityPlayer = ig.Entity.extend({
    // Animation properties
    animSheet: null,
    anims: {},
    currentAnim: null,

    // Movement settings
    speed: 100,
    jumpForce: 200,

    init: function( x, y, settings ) {
        this.parent( x, y, settings );

        // Load sprite sheet
        this.animSheet = new ig.AnimationSheet( 'gfx/player.png', 16, 16 );

        // Define animations
        this.addAnim( 'idle', 0.2, [0] );
        this.addAnim( 'run', 0.08, [1, 2, 3] );
        this.addAnim( 'jump', 0.1, [4] );
        this.addAnim( 'fall', 0.1, [5] );
        this.addAnim( 'attack', 0.1, [6, 7, 8], true );
    },

    update: function() {
        // Input handling
        if ( ig.input.state('left') ) {
            this.vel.x = -this.speed;
        } else if ( ig.input.state('right') ) {
            this.vel.x = this.speed;
        } else {
            this.vel.x = 0;
        }

        if ( ig.input.pressed('jump') && this.standing ) {
            this.vel.y = -this.jumpForce;
        }

        if ( ig.input.pressed('attack') && this.currentAnim !== this.anims['attack'] ) {
            this.currentAnim = this.anims['attack'];
        }

        // State-based animation selection
        if ( this.currentAnim !== this.anims['attack'] ) {
            if ( !this.standing ) {
                this.currentAnim = this.vel.y < 0 ? this.anims['jump'] : this.anims['fall'];
            } else if ( this.vel.x !== 0 ) {
                this.currentAnim = this.anims['run'];
            } else {
                this.currentAnim = this.anims['idle'];
            }
        }

        // Face direction
        if ( this.vel.x > 0 ) {
            this.currentAnim.flip.x = false;
        } else if ( this.vel.x < 0 ) {
            this.currentAnim.flip.x = true;
        }

        this.parent();
    },

    draw: function() {
        this.parent();

        // Debug: Show current animation name
        if ( ig.game.debug ) {
            ig.system.context.fillStyle = 'white';
            ig.system.context.font = '10px monospace';
            ig.system.context.fillText(
                this.currentAnim === this.anims['idle'] ? 'idle' :
                this.currentAnim === this.anims['run'] ? 'run' :
                this.currentAnim === this.anims['jump'] ? 'jump' : 'fall',
                this.pos.x - ig.game._rscreen.x,
                this.pos.y - ig.game._rscreen.y - 5
            );
        }
    }
});
```

---

## 9. Animated Tiles System

### 9.1 Background Map Animations

```javascript
ig.BackgroundMap = ig.Map.extend({
    // Animated tiles: anims[tileIndex-1] = ig.Animation
    anims: {}
});
```

### 9.2 Setting Up Animated Tiles

```javascript
// In your game class
MyGame = ig.Game.extend({
    init: function() {
        // ...
    },

    loadLevel: function( data ) {
        this.parent( data );

        // Set up animated tiles for each background map
        for ( var i = 0; i < this.backgroundMaps.length; i++ ) {
            var map = this.backgroundMaps[i];

            // Water animation (tiles 10, 11, 12)
            map.anims[9] = new ig.Animation(
                map.tiles,  // Use the map's tileset
                0.2,        // 200ms per frame
                [9, 10, 11] // Tile indices (0-based, so tile 10-1)
            );

            // Fire animation (tiles 20, 21)
            map.anims[19] = new ig.Animation(
                map.tiles,
                0.1,
                [19, 20]
            );
        }
    }
});
```

### 9.3 Animated Tile Drawing

```javascript
ig.BackgroundMap.prototype.drawTiled = function() {
    // ... tile iteration ...

    for( var mapY = -1, pxY = pxMinY; pxY < pxMaxY; mapY++, pxY += this.tilesize) {
        // ...

        for( var mapX = -1, pxX = pxMinX; pxX < pxMaxX; mapX++, pxX += this.tilesize ) {
            // ...

            if( (tile = this.data[tileY][tileX]) ) {
                // Check for animation
                if( (anim = this.anims[tile-1]) ) {
                    anim.draw( pxX, pxY );  // Draw animated tile
                }
                else {
                    this.tiles.drawTile( pxX, pxY, tile-1, this.tilesize );
                }
            }
        }
    }
};
```

### 9.4 Animated Tile Example

```
Tilemap with animated water:
┌──────────────────────────────────────────┐
│ Grass: tile 1 (static)                   │
│ Water: tiles 10, 11, 12 (animated)       │
│                                          │
│ ┌────┬────┬────┬────┬────┐              │
│ │ 1  │ 1  │ 1  │ 1  │ 1  │  Grass row   │
│ ├────┼────┼────┼────┼────┤              │
│ │ 1  │ 10 │ 11 │ 12 │ 1  │  Water row 1 │
│ ├────┼────┼────┼────┼────┤              │
│ │ 1  │ 10 │ 11 │ 12 │ 1  │  Water row 2 │
│ ├────┼────┼────┼────┼────┤              │
│ │ 1  │ 1  │ 1  │ 1  │ 1  │  Grass row   │
│ └────┴────┴────┴────┴────┘              │
│                                          │
│ Animation sequence: [9, 10, 11]          │
│ (tile indices are 0-based in code)       │
└──────────────────────────────────────────┘
```

---

## 10. Animation Performance

### 10.1 Update Cost Analysis

```
Animation update cost per frame:
├─ Timer.delta() call: O(1)
├─ Math.floor division: O(1)
├─ Modulo operation: O(1)
└─ Array access: O(1)

Total: O(1) per animation

For 100 entities with animations:
→ 100 animation updates per frame
→ ~6000 animation updates per second at 60 FPS
```

### 10.2 Optimization Strategies

### 1. Skip Update for Single-Frame Animations

```javascript
ig.Animation.prototype.update = function() {
    // Skip update for single-frame animations
    if ( this.sequence.length <= 1 ) {
        return;
    }

    // Skip update for stopped animations
    if ( this.stop && this.frame === this.sequence.length - 1 ) {
        return;
    }

    // ... normal update ...
};
```

### 2. Distance Culling

```javascript
ig.Game.prototype.draw = function() {
    // ... draw backgrounds ...

    // Only draw/update visible entity animations
    for ( var i = 0; i < this.entities.length; i++ ) {
        var ent = this.entities[i];
        if ( this.isEntityVisible( ent ) ) {
            ent.draw();
        }
    }
};
```

### 3. Animation LOD (Level of Detail)

```javascript
EntityPlayer = ig.Entity.extend({
    update: function() {
        this.parent();

        // Reduce animation frame rate for distant entities
        var distance = ig.game.screen.dist( this.pos );
        if ( distance > 400 ) {
            this.currentAnim.frameTime = 0.2;  // Half frame rate
        } else {
            this.currentAnim.frameTime = 0.1;  // Normal frame rate
        }
    }
});
```

### 10.3 Memory Optimization

```javascript
// Share animations between entity instances
var EntityPlayer = ig.Entity.extend({
    // Static properties (shared across all instances)
    staticInstantiate: function() {
        // Create animations once
        this.prototype.sharedAnims = {
            idle: null,
            run: null,
            jump: null
        };
        return null;
    },

    init: function( x, y, settings ) {
        this.parent( x, y, settings );

        // Create shared AnimationSheet
        if ( !this.sharedAnims.idle ) {
            var sheet = new ig.AnimationSheet( 'gfx/player.png', 16, 16 );
            this.sharedAnims.idle = new ig.Animation( sheet, 0.2, [0] );
            this.sharedAnims.run = new ig.Animation( sheet, 0.08, [1, 2, 3] );
            this.sharedAnims.jump = new ig.Animation( sheet, 0.1, [4] );
        }

        // Clone animations for this instance
        this.anims = {};
        for ( var name in this.sharedAnims ) {
            this.anims[name] = this.sharedAnims[name];  // Reference, not copy
        }
        this.currentAnim = this.anims.idle;
    }
});
```

---

## 11. Practical Examples

### 11.1 Walking Character

```javascript
EntityNPC = ig.Entity.extend({
    init: function( x, y, settings ) {
        this.parent( x, y, settings );
        this.animSheet = new ig.AnimationSheet( 'gfx/npc.png', 16, 16 );
        this.addAnim( 'idle', 0.3, [0, 1] );
        this.addAnim( 'walk', 0.12, [2, 3, 4, 5] );
        this.addAnim( 'talk', 0.15, [6, 7, 8, 7] );
    },

    update: function() {
        this.parent();

        // Simple patrol behavior
        if ( this.pos.x < 100 ) {
            this.vel.x = 30;
            this.currentAnim = this.anims.walk;
            this.currentAnim.flip.x = false;
        } else if ( this.pos.x > 300 ) {
            this.vel.x = -30;
            this.currentAnim = this.anims.walk;
            this.currentAnim.flip.x = true;
        } else {
            this.vel.x = 0;
            this.currentAnim = this.anims.idle;
        }
    }
});
```

### 11.2 Attack Combo

```javascript
EntityPlayer = ig.Entity.extend({
    comboChain: [],
    comboIndex: 0,

    init: function() {
        this.parent();
        this.addAnim( 'attack1', 0.08, [10, 11, 12], true );
        this.addAnim( 'attack2', 0.08, [13, 14, 15], true );
        this.addAnim( 'attack3', 0.1, [16, 17, 18, 19], true );
        this.comboChain = ['attack1', 'attack2', 'attack3'];

        // Set stop callbacks
        this.anims['attack1'].stopCallback = this.onAttackComplete.bind(this);
        this.anims['attack2'].stopCallback = this.onAttackComplete.bind(this);
        this.anims['attack3'].stopCallback = this.onAttackComplete.bind(this);
    },

    attack: function() {
        if ( this.currentAnim === this.anims['attack1'] ||
             this.currentAnim === this.anims['attack2'] ||
             this.currentAnim === this.anims['attack3'] ) {
            return;  // Already attacking
        }

        this.comboIndex = 0;
        this.startAttack();
    },

    startAttack: function() {
        var animName = this.comboChain[this.comboIndex];
        this.currentAnim = this.anims[animName];
    },

    onAttackComplete: function() {
        this.comboIndex++;
        if ( this.comboIndex < this.comboChain.length ) {
            this.startAttack();  // Continue combo
        } else {
            this.currentAnim = this.anims.idle;  // End combo
        }
    }
});
```

### 11.3 Animated Background (Flickering Lights)

```javascript
MyGame = ig.Game.extend({
    loadLevel: function( data ) {
        this.parent( data );

        // Find the light map
        var lightMap = this.getMapByName('lights');

        // Set up flickering light animations (random timing)
        lightMap.anims[49] = new ig.Animation( lightMap.tiles, 0.05, [49, 50] );
        lightMap.anims[51] = new ig.Animation( lightMap.tiles, 0.08, [51, 52, 51] );

        // Vary the timing for natural look
        lightMap.anims[49].frameTime = 0.05 + Math.random() * 0.05;
        lightMap.anims[51].frameTime = 0.08 + Math.random() * 0.04;
    }
});
```

### 11.4 Death/Respawn Sequence

```javascript
EntityPlayer = ig.Entity.extend({
    isDead: false,

    die: function() {
        if ( this.isDead ) return;
        this.isDead = true;

        // Play death animation
        this.currentAnim = this.anims['die'];
        this.currentAnim.stopCallback = this.onDeathComplete.bind(this);

        // Disable collision
        this.collides = ig.Entity.COLLIDES.NEVER;
        this.type = ig.Entity.TYPE.NONE;
    },

    onDeathComplete: function() {
        // Wait a moment, then respawn
        setTimeout( this.respawn.bind(this), 1000 );
    },

    respawn: function() {
        this.isDead = false;
        this.pos.x = this.startPos.x;
        this.pos.y = this.startPos.y;
        this.vel.x = 0;
        this.vel.y = 0;
        this.currentAnim = this.anims.idle;
        this.collides = ig.Entity.COLLIDES.ACTIVE;
        this.type = ig.Entity.TYPE.A;
    }
});
```

---

## Appendix A: Complete Animation State Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                      ANIMATION STATE MACHINE                                │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                           ┌─────────────┐                                  │
│                      ┌───▶│    IDLE     │◀──┐                              │
│                      │    │  [0], 0.2s  │   │                              │
│                      │    └──────┬──────┘   │                              │
│                      │           │          │                              │
│            ┌─────────┘           │          └─────────┐                    │
│            │                     │                    │                    │
│            │         ┌───────────┴───────────┐        │                    │
│            │         │                       │        │                    │
│            ▼         ▼                       │        │                    │
│      ┌──────────┐ ┌──────┐             ┌──────────┐  │                    │
│      │   RUN    │ │ JUMP │             │  ATTACK  │──┘                    │
│      │ [1,2,3], │ │ [4], │             │[6,7,8],  │  (one-shot)           │
│      │  0.08s   │ │0.1s  │             │  0.1s    │                       │
│      └────┬─────┘ └──┬───┘             └────┬─────┘                       │
│           │          │                      │                             │
│           │          │                      ▼                             │
│           │          │               ┌─────────────┐                      │
│           │          └──────────────▶│    FALL     │                      │
│           │                          │    [5],     │                      │
│           │                          │    0.1s     │                      │
│           │                          └─────────────┘                      │
│           │                                                              │
│           └──────────────────────────────────────────────────────────────┘│
│                              (standing = true)                            │
│                                                                           │
│  Additional states (not shown):                                           │
│  - HIT: Triggered by damage, one-shot, returns to idle                   │
│  - DIE: Triggered by death, one-shot, removes entity                     │
│  - TALK: Triggered by interaction, loops while talking                   │
│                                                                           │
└────────────────────────────────────────────────────────────────────────────┘
```

### Appendix B: Frame Timing Reference

```
Game Loop FPS → System Tick
────────────────────────────
60 FPS → 0.0167s (16.7ms)
30 FPS → 0.0333s (33.3ms)

Animation Speed Recommendations:
────────────────────────────────────
Idle breathing:     0.2-0.3s per frame
Walking:           0.1-0.15s per frame
Running:          0.05-0.08s per frame
Attack swipe:     0.05-0.1s per frame
Falling:               0.1s (single frame)
Water ripple:     0.15-0.25s per frame
Fire flicker:     0.05-0.1s per frame
UI pulse:              0.3-0.5s per frame
```
