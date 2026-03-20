---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/Impact/lib/impact
explored_at: 2026-03-20
---

# Impact.js Game Loop and Delta Timing - Deep Dive

**Scope:** Game loop architecture, requestAnimationFrame integration, delta time calculation, FPS management, update/render cycle, timing synchronization

---

## Table of Contents

1. [Game Loop Architecture Overview](#1-game-loop-architecture-overview)
2. [System Class and Main Loop](#2-system-class-and-main-loop)
3. [RequestAnimationFrame Integration](#3-requestanimationframe-integration)
4. [Delta Time System](#4-delta-time-system)
5. [Timer Implementation](#5-timer-implementation)
6. [Time Scale and Slow Motion](#6-time-scale-and-slow-motion)
7. [Frame Rate Management](#7-frame-rate-management)
8. [Update and Render Cycle](#8-update-and-render-cycle)
9. [Fixed vs Variable Time Step](#9-fixed-vs-variable-time-step)
10. [Entity Update Order](#10-entity-update-order)
11. [Level Loading and State Transitions](#11-level-loading-and-state-transitions)
12. [Pause and Resume](#12-pause-and-resume)
13. [Performance Monitoring](#13-performance-monitoring)
14. [Rust Reimplementation Considerations](#14-rust-reimplementation-considerations)

---

## 1. Game Loop Architecture Overview

### 1.1 The Core Game Loop

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         GAME LOOP (60 FPS Target)                           │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐                                                        │
│  │ requestAnimationFrame │ ◀────────────────────────────────┐             │
│  └────────┬──────────┘                                       │             │
│           │                                                   │             │
│           ▼                                                   │             │
│  ┌────────────────────────────────────────────────────────┐  │             │
│  │              ig.System.run()                           │  │             │
│  │                                                        │  │             │
│  │  1. ig.Timer.step()                                    │  │             │
│  │     └─▶ Update global time                             │  │             │
│  │                                                        │  │             │
│  │  2. this.tick = this.clock.tick()                      │  │             │
│  │     └─▶ Get delta time since last frame                │  │             │
│  │                                                        │  │             │
│  │  3. this.delegate.run()                                │  │             │
│  │     └─▶ ig.Game.update()                              │  │             │
│  │     └─▶ ig.Game.draw()                                │  │             │
│  │                                                        │  │             │
│  │  4. ig.input.clearPressed()                            │  │             │
│  │     └─▶ Reset input state                              │  │             │
│  └────────────────────────────────────────────────────────┘  │             │
│           │                                                   │             │
│           └───────────────────────────────────────────────────┘             │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Component Responsibilities

| Component | File | Responsibility |
|-----------|------|----------------|
| `ig.System` | `system.js` | Main loop, timing, canvas context |
| `ig.Game` | `game.js` | Update/render delegation, entity management |
| `ig.Timer` | `timer.js` | Delta time calculation, time scale |
| `ig.Entity` | `entity.js` | Per-entity update (physics, animation) |
| `ig.Input` | `input.js` | Input polling, key state management |

### 1.3 Initialization Sequence

```javascript
// User's main.js
ig.module('game.main')
.requires('impact.impact', 'impact.game', 'game.entities.player')
.defines(function() {
    MyGame = ig.Game.extend({
        init: function() {
            // Load level
            this.loadLevel('level01.json');
        }
    });

    // Start the engine
    ig.main('#canvas', MyGame, 60, 320, 240, 2);
    //                      ▲       ▲    ▲    ▲    ▲    ▲
    //                      │       │    │    │    │    └─ Scale (2x = 640x480 canvas)
    //                      │       │    │    │    └────── Height (240px logical)
    //                      │       │    │    └─────────── Width (320px logical)
    //                      │       │    └──────────────── FPS target
    //                      │       └───────────────────── Game class
    //                      └───────────────────────────── CSS selector
});
```

---

## 2. System Class and Main Loop

### 2.1 System Class Structure

```javascript
ig.System = ig.Class.extend({
    // Timing
    fps: 60,
    tick: 1/60,
    accumulatedTime: 0,

    // Canvas
    canvas: null,
    context: null,
    width: 0,
    height: 0,
    scale: 1,
    realWidth: 0,
    realHeight: 0,

    // Loop control
    running: false,
    delegate: null,  // The game instance

    init: function( canvasId, fps, width, height, scale ) {
        this.canvas = ig.$(canvasId);
        this.context = this.canvas.getContext('2d');
        this.fps = fps;
        this.tick = 1 / fps;
        this.resize( width, height, scale );

        // Set draw mode
        this.getDrawPos = ig.System.drawMode;

        // Apply scale mode
        if( this.scale != 1 ) {
            ig.System.scaleMode = ig.System.SCALE.CRISP;
        }
        ig.System.scaleMode( this.canvas, this.context );
    },

    // Set animation delegate (the game)
    setGame: function( game ) {
        this.delegate = game;
    },

    // Main loop entry point
    run: function() {
        this.running = true;
        ig.setAnimation( this.loop.bind(this) );
    },

    // Loop function called by requestAnimationFrame
    loop: function() {
        // Update global timer
        ig.Timer.step();

        // Get delta time
        this.tick = this.clock.tick();

        // Run game update and render
        this.delegate.run();

        // Reset input state
        ig.input.clearPressed();
    }
});
```

### 2.2 Animation Frame Wrapper

```javascript
// Cross-browser requestAnimationFrame
ig.setAnimation = function( callback ) {
    if( window.webkitRequestAnimationFrame ) {
        webkitRequestAnimationFrame( callback );
    }
    else if( window.mozRequestAnimationFrame ) {
        mozRequestAnimationFrame( callback );
    }
    else if( window.requestAnimationFrame ) {
        requestAnimationFrame( callback );
    }
    else {
        // Fallback to setTimeout
        window.setInterval( callback, 1000/60 );
    }
};
```

---

## 3. RequestAnimationFrame Integration

### 3.1 Browser Timing Integration

```javascript
// How requestAnimationFrame works with Impact's loop:

Browser: ─────┬─────────┬─────────┬─────────┬─────────┬────▶ Time
              │         │         │         │         │
              ▼         ▼         ▼         ▼         ▼
           rAF call  rAF call  rAF call  rAF call  rAF call
              │         │         │         │         │
              ▼         ▼         ▼         ▼         ▼
           ig.Timer  ig.Timer  ig.Timer  ig.Timer  ig.Timer
           .step()   .step()   .step()   .step()   .step()
              │         │         │         │         │
              ▼         ▼         ▼         ▼         ▼
           game.run  game.run  game.run  game.run  game.run
           (update)  (update)  (update)  (update)  (update)
              │         │         │         │         │
              ▼         ▼         ▼         ▼         ▼
           game.draw game.draw game.draw game.draw game.draw
```

### 3.2 Frame Timing Variability

```
Actual frame times (milliseconds) from requestAnimationFrame:

Frame 1: 16.7ms (60 FPS)
Frame 2: 16.8ms
Frame 3: 17.2ms  ← Slight slowdown
Frame 4: 16.5ms
Frame 5: 33.3ms  ← Dropped frame! (two rAF calls combined)
Frame 6: 16.6ms

Impact handles this via delta time:
- Each frame's logic uses actual elapsed time
- Physics and movement scale proportionally
- No "speed up" or "slow down" perceived
```

---

## 4. Delta Time System

### 4.1 Global Time Management

```javascript
// Static properties on ig.Timer
ig.Timer.time = Number.MIN_VALUE;     // Current game time
ig.Timer._last = 0;                    // Last frame's timestamp
ig.Timer.timeScale = 1;                // Time multiplier (for slow-mo)
ig.Timer.maxStep = 0.05;               // Cap delta at 50ms

// Called every frame by ig.System.loop
ig.Timer.step = function() {
    var current = Date.now();
    var delta = (current - ig.Timer._last) / 1000;  // Convert to seconds

    // Apply time scale and cap maximum delta
    ig.Timer.time += Math.min(delta, ig.Timer.maxStep) * ig.Timer.timeScale;

    ig.Timer._last = current;
};
```

### 4.2 Delta Time in Game Logic

```javascript
// Entity movement with delta time
ig.Entity.prototype.update = function() {
    // Apply gravity (scaled by delta time)
    this.vel.y += ig.game.gravity * ig.system.tick * this.gravityFactor;

    // Apply acceleration (scaled by delta time)
    this.vel.x = this.getNewVelocity(
        this.vel.x,
        this.accel.x,
        this.friction.x,
        this.maxVel.x
    );

    // Calculate movement distance (velocity * delta time)
    var mx = this.vel.x * ig.system.tick;
    var my = this.vel.y * ig.system.tick;

    // Trace collision with movement vector
    var res = ig.game.collisionMap.trace(
        this.pos.x, this.pos.y, mx, my, this.size.x, this.size.y
    );
    this.handleMovementTrace( res );
};
```

### 4.3 Why Delta Time Matters

```
Without delta time (fixed movement per frame):
- At 60 FPS: player moves 1px/frame = 60px/second
- At 30 FPS: player moves 1px/frame = 30px/second ← Half speed!
- Game runs at different speeds on different devices

With delta time (time-scaled movement):
- At 60 FPS: player moves 1px * 0.0167s = 0.0167px/frame = 60px/second
- At 30 FPS: player moves 1px * 0.0333s = 0.0333px/frame = 60px/second ← Same!
- Game runs at same speed regardless of frame rate
```

---

## 5. Timer Implementation

### 5.1 Timer Class

```javascript
ig.Timer = ig.Class.extend({
    target: 0,      // Target time (for countdown)
    base: 0,        // Start time reference
    last: 0,        // Last tick time
    pausedAt: 0,    // Pause timestamp (0 = not paused)

    init: function( seconds ) {
        this.base = ig.Timer.time;
        this.last = ig.Timer.time;
        this.target = seconds || 0;
    },

    // Set timer to specific value
    set: function( seconds ) {
        this.target = seconds || 0;
        this.base = ig.Timer.time;
        this.pausedAt = 0;
    },

    // Reset to current time
    reset: function() {
        this.base = ig.Timer.time;
        this.pausedAt = 0;
    },

    // Get time since last tick
    tick: function() {
        var delta = ig.Timer.time - this.last;
        this.last = ig.Timer.time;
        return (this.pausedAt ? 0 : delta);
    },

    // Get elapsed time since timer started
    delta: function() {
        return (this.pausedAt || ig.Timer.time) - this.base - this.target;
    },

    // Pause timer
    pause: function() {
        if( !this.pausedAt ) {
            this.pausedAt = ig.Timer.time;
        }
    },

    // Unpause timer
    unpause: function() {
        if( this.pausedAt ) {
            // Adjust base to account for paused duration
            this.base += ig.Timer.time - this.pausedAt;
            this.pausedAt = 0;
        }
    }
});
```

### 5.2 Timer Usage Examples

```javascript
// Cooldown timer
EntityPlayer = ig.Entity.extend({
    shootTimer: null,
    shootCooldown: 0.5,  // 500ms

    init: function() {
        this.parent();
        this.shootTimer = new ig.Timer();
    },

    shoot: function() {
        if ( this.shootTimer.delta() > this.shootCooldown ) {
            // Fire projectile
            ig.game.spawnEntity('EntityBullet', this.pos.x, this.pos.y);
            this.shootTimer.set(0);  // Reset timer
        }
    }
});

// Animation frame timing
ig.Animation = ig.Class.extend({
    update: function() {
        // Calculate frame based on elapsed time
        var frameTotal = Math.floor( this.timer.delta() / this.frameTime );
        this.loopCount = Math.floor( frameTotal / this.sequence.length );
        this.frame = frameTotal % this.sequence.length;
        this.tile = this.sequence[this.frame];
    }
});
```

### 5.3 Timer Delta Explained

```
Timer timeline:

t=0ms:     timer.set(0)
           └─▶ base = ig.Timer.time = 0
           └─▶ delta() = time - base - target = 0 - 0 - 0 = 0

t=100ms:   delta() = 100 - 0 - 0 = 100ms

t=200ms:   timer.set(0.5)  // Set 500ms target
           └─▶ base = 200
           └─▶ target = 500
           └─▶ delta() = 200 - 200 - 500 = -500ms (counting down)

t=700ms:   delta() = 700 - 200 - 500 = 0ms (reached target!)

t=800ms:   delta() = 800 - 200 - 500 = 100ms (positive = elapsed)
```

---

## 6. Time Scale and Slow Motion

### 6.1 Global Time Scale

```javascript
// Static property
ig.Timer.timeScale = 1;  // Normal speed

// Slow motion (50% speed)
ig.Timer.timeScale = 0.5;

// Fast forward (200% speed)
ig.Timer.timeScale = 2;

// Freeze time
ig.Timer.timeScale = 0;
```

### 6.2 Time Scale Implementation

```javascript
// In ig.Timer.step():
ig.Timer.step = function() {
    var current = Date.now();
    var delta = (current - ig.Timer._last) / 1000;

    // Apply time scale to delta
    ig.Timer.time += Math.min(delta, ig.Timer.maxStep) * ig.Timer.timeScale;

    ig.Timer._last = current;
};
```

### 6.3 Time Scale Effects

```
With timeScale = 0.5 (slow motion):

Real time:     0ms    100ms   200ms   300ms   400ms   500ms
               │       │       │       │       │       │
               ▼       ▼       ▼       ▼       ▼       ▼
Game time:     0ms     50ms   100ms   150ms   200ms   250ms
                      ▲                               │
                      └─▶ Game world runs at half speed

Effects:
- Entity movement appears slowed
- Projectiles move in slow motion
- Animations play at half speed
- Gravity affects objects more slowly
- Player input still responds at normal speed (not scaled)
```

### 6.4 Selective Time Scaling

```javascript
// Apply time scale only to specific systems
EntityPlayer = ig.Entity.extend({
    update: function() {
        // Player movement ignores time scale (responsive controls)
        this.vel.x = this.getNewVelocity(
            this.vel.x,
            this.accel.x,
            this.friction.x,
            this.maxVel.x
        );

        // Physics respects time scale
        this.vel.y += ig.game.gravity * ig.system.tick * this.gravityFactor;

        this.parent();
    }
});

// Or scale specific timers independently
EntityBullet = ig.Entity.extend({
    speed: 200,

    update: function() {
        // Bullet ignores time scale (always moves at full speed)
        var mx = this.speed * ig.system.tick / ig.Timer.timeScale;
        this.pos.x += mx;

        this.parent();
    }
});
```

---

## 7. Frame Rate Management

### 7.1 Target FPS

```javascript
// In ig.main():
ig.main = function( canvasId, gameClass, fps, width, height, scale ) {
    ig.system = new ig.System( canvasId, fps, width, height, scale );
    ig.input = new ig.Input();
    ig.soundManager = new ig.SoundManager();
    ig.music = new ig.Music();
    ig.ready = true;

    // Create game instance
    var game = new gameClass();
    ig.system.setGame( game );
    ig.system.run();
};
```

### 7.2 FPS vs Tick Relationship

```
Target FPS  →  System Tick
────────────────────────────
60  →  0.0167s (16.7ms)
30  →  0.0333s (33.3ms)
24  →  0.0417s (41.7ms)
15  →  0.0667s (66.7ms)

// Calculation
tick = 1 / fps
```

### 7.3 Variable Frame Rate Handling

```javascript
// Impact uses variable time step:
// - Logic runs every rAF callback
// - Delta time varies based on actual elapsed time

ig.System.loop = function() {
    ig.Timer.step();  // Update global time with actual delta
    this.tick = this.clock.tick();  // Get frame's delta time

    // Game logic uses this.tick
    this.delegate.run();
};

// Pros:
// - Smooth animation regardless of frame rate
// - No "spiral of death" from catching up

// Cons:
// - Physics may be inconsistent at very low FPS
// - Fast objects may tunnel through walls at low FPS
```

### 7.4 Maximum Delta Cap

```javascript
// Prevent spiral of death when tab is backgrounded
ig.Timer.maxStep = 0.05;  // Cap at 50ms (20 FPS equivalent)

// When tab is backgrounded, rAF may pause for seconds
// When tab is restored, delta would be huge without cap
ig.Timer.step = function() {
    var current = Date.now();
    var delta = (current - ig.Timer._last) / 1000;

    // Cap prevents teleportation from huge movement values
    ig.Timer.time += Math.min(delta, ig.Timer.maxStep) * ig.Timer.timeScale;

    ig.Timer._last = current;
};
```

---

## 8. Update and Render Cycle

### 8.1 Game.run() Method

```javascript
ig.Game = ig.Class.extend({
    run: function() {
        // Update phase
        this.update();

        // Render phase
        this.draw();
    },

    update: function() {
        // Update entities
        for( var i = 0; i < this.entities.length; i++ ) {
            this.entities[i].update();
        }

        // Update background map animations
        for( var key in this.backgroundAnims ) {
            var anims = this.backgroundAnims[key];
            for( var i in anims ) {
                anims[i].update();
            }
        }

        // Sort entities if needed
        if( this._doSortEntities ) {
            this.sortEntities();
            this._doSortEntities = false;
        }

        // Handle deferred level load
        if( this._levelToLoad ) {
            this.loadLevel( this._levelToLoad );
            this._levelToLoad = null;
        }

        // Remove dead entities
        for( var i = this.entities.length-1; i >= 0; i-- ) {
            if( this.entities[i]._killed ) {
                this.entities.splice(i, 1);
            }
        }
    },

    draw: function() {
        // Clear screen
        ig.system.clear( this.clearColor );

        // Calculate rounded screen position
        this._rscreen = {
            x: ig.system.getDrawPos(this.screen.x) / ig.system.scale,
            y: ig.system.getDrawPos(this.screen.y) / ig.system.scale
        };

        // Draw background maps (non-foreground)
        for( var i = 0; i < this.backgroundMaps.length; i++ ) {
            var map = this.backgroundMaps[i];
            if( !map.foreground && map.enabled ) {
                map.setScreenPos( this.screen.x, this.screen.y );
                map.draw();
            }
        }

        // Draw entities
        for( var i = 0; i < this.entities.length; i++ ) {
            this.entities[i].draw();
        }

        // Draw foreground maps
        for( var i = 0; i < this.backgroundMaps.length; i++ ) {
            var map = this.backgroundMaps[i];
            if( map.foreground && map.enabled ) {
                map.setScreenPos( this.screen.x, this.screen.y );
                map.draw();
            }
        }
    }
});
```

### 8.2 Update/Render Timing Diagram

```
Frame 1                              Frame 2
─────────────────────────────────────────────────────────▶ Time
│                                    │
▼                                    ▼
┌──────────────────────────┐        ┌──────────────────────────┐
│     UPDATE PHASE         │        │     UPDATE PHASE         │
│  ────────────────────    │        │  ────────────────────    │
│  • Entity physics        │        │  • Entity physics        │
│  • Entity animations     │        │  • Entity animations     │
│  • Collision detection   │        │  • Collision detection   │
│  • Game logic            │        │  • Game logic            │
│                          │        │                          │
│  Duration: ~8ms          │        │  Duration: ~8ms          │
└──────────────────────────┘        └──────────────────────────┘
│                                    │
▼                                    ▼
┌──────────────────────────┐        ┌──────────────────────────┐
│     RENDER PHASE         │        │     RENDER PHASE         │
│  ────────────────────    │        │  ────────────────────    │
│  • Clear screen          │        │  • Clear screen          │
│  • Draw backgrounds      │        │  • Draw backgrounds      │
│  • Draw entities         │        │  • Draw entities         │
│  • Draw foreground       │        │  • Draw foreground       │
│                          │        │                          │
│  Duration: ~8ms          │        │  Duration: ~8ms          │
└──────────────────────────┘        └──────────────────────────┘
│                                    │
└────────────────────────────────────┘
         Total: ~16.7ms (60 FPS)
```

---

## 9. Fixed vs Variable Time Step

### 9.1 Impact's Variable Time Step

```javascript
// Impact uses variable time step:
// - Update runs once per frame
// - Delta time varies

ig.Game.prototype.update = function() {
    // Each entity uses ig.system.tick (variable delta)
    for( var i = 0; i < this.entities.length; i++ ) {
        this.entities[i].update();  // Uses ig.system.tick internally
    }
};
```

### 9.2 Fixed Time Step Alternative

```javascript
// Fixed time step implementation (for comparison):
ig.Game.prototype.update = function() {
    this.accumulatedTime += ig.system.tick;

    // Update at fixed 60 Hz intervals
    while( this.accumulatedTime >= this.fixedStep ) {
        this.fixedUpdate();
        this.accumulatedTime -= this.fixedStep;
    }
};

ig.Game.prototype.fixedUpdate = function() {
    // Always runs at 60 updates/second
    // Independent of render frame rate
    for( var i = 0; i < this.entities.length; i++ ) {
        this.entities[i].fixedUpdate();
    }
};
```

### 9.3 Comparison

| Aspect | Variable Time Step | Fixed Time Step |
|--------|-------------------|-----------------|
| **Smoothness** | Smooth animation | May stutter if FPS varies |
| **Physics consistency** | Varies with FPS | Consistent regardless of FPS |
| **Implementation** | Simple | More complex |
| **CPU usage** | Lower | Higher (catch-up logic) |
| **Used by Impact** | ✓ | ✗ |

### 9.4 Why Impact Uses Variable Time Step

1. **Simplicity**: Single update call per frame
2. **Canvas 2D**: No strict physics requirements
3. **Target games**: 2D platformers don't need frame-perfect physics
4. **Performance**: Less CPU overhead than fixed step

---

## 10. Entity Update Order

### 10.1 Sequential Entity Updates

```javascript
ig.Game.prototype.update = function() {
    // Entities are updated in array order
    for( var i = 0; i < this.entities.length; i++ ) {
        this.entities[i].update();
    }
};
```

### 10.2 Update Order Implications

```
Entity array order:
[0] EntityPlayer
[1] EntityEnemy1
[2] EntityEnemy2
[3] EntityBullet1
[4] EntityBullet2

Update sequence:
1. Player moves
2. Enemy1 moves (sees player's OLD position)
3. Enemy2 moves (sees player's OLD position)
4. Bullet1 moves
5. Bullet2 moves

This can cause frame-delay in interactions:
- Enemy sees player position from previous frame
- Bullets may miss fast-moving targets
```

### 10.3 Controlling Update Order

```javascript
// Sort entities by type for predictable updates
MyGame = ig.Game.extend({
    update: function() {
        // Update projectiles first
        var projectiles = this.getEntitiesByType('EntityBullet');
        for( var i = 0; i < projectiles.length; i++ ) {
            projectiles[i].update();
        }

        // Then enemies
        var enemies = this.getEntitiesByType('EntityEnemy');
        for( var i = 0; i < enemies.length; i++ ) {
            enemies[i].update();
        }

        // Finally player
        this.player.update();

        // Remove dead entities
        // ...
    }
});
```

---

## 11. Level Loading and State Transitions

### 11.1 Level Load Process

```javascript
ig.Game.prototype.loadLevel = function( data ) {
    // Reset screen position
    this.screen = {x: 0, y: 0};

    // Clear entities
    this.entities = [];
    this.namedEntities = {};

    // Spawn entities from level data
    for( var i = 0; i < data.entities.length; i++ ) {
        var ent = data.entities[i];
        this.spawnEntity( ent.type, ent.x, ent.y, ent.settings );
    }
    this.sortEntities();

    // Load maps
    this.collisionMap = ig.CollisionMap.staticNoCollision;
    this.backgroundMaps = [];
    for( var i = 0; i < data.layer.length; i++ ) {
        var ld = data.layer[i];
        if( ld.name == 'collision' ) {
            this.collisionMap = new ig.CollisionMap(ld.tilesize, ld.data);
        }
        else {
            var newMap = new ig.BackgroundMap(
                ld.tilesize, ld.data, ld.tilesetName
            );
            newMap.anims = this.backgroundAnims[ld.tilesetName] || {};
            newMap.repeat = ld.repeat;
            newMap.distance = ld.distance;
            newMap.foreground = !!ld.foreground;
            newMap.preRender = !!ld.preRender;
            newMap.name = ld.name;
            this.backgroundMaps.push( newMap );
        }
    }

    // Call ready() on all entities
    for( var i = 0; i < this.entities.length; i++ ) {
        this.entities[i].ready();
    }
};
```

### 11.2 Deferred Level Loading

```javascript
// Load level at end of current frame
ig.Game.prototype.loadLevelDeferred = function( data ) {
    this._levelToLoad = data;
};

// In game loop:
ig.Game.prototype.update = function() {
    // ... entity updates ...

    if( this._levelToLoad ) {
        this.loadLevel( this._levelToLoad );
        this._levelToLoad = null;
    }
};

// Usage:
ig.game.loadLevelDeferred( levelData );
// Level loads after current frame's updates complete
```

### 11.3 Level Transition Effects

```javascript
MyGame = ig.Game.extend({
    transition: null,

    loadLevel: function( data ) {
        // Start fade-out transition
        this.transition = {
            phase: 'out',
            alpha: 0,
            speed: 2
        };

        // Wait for transition before loading
        this._pendingLevelData = data;
    },

    update: function() {
        if ( this.transition ) {
            this.updateTransition();
            return;  // Skip normal update during transition
        }

        this.parent();
    },

    draw: function() {
        this.parent();

        // Draw transition overlay
        if ( this.transition ) {
            ig.system.context.fillStyle = 'rgba(0,0,0,' + this.transition.alpha + ')';
            ig.system.context.fillRect(0, 0, ig.system.width, ig.system.height);
        }
    },

    updateTransition: function() {
        this.transition.alpha += this.transition.speed * ig.system.tick;

        if ( this.transition.phase === 'out' && this.transition.alpha >= 1 ) {
            // Fade-out complete, load level
            this.parent();  // Call original loadLevel
            this.transition.phase = 'in';
            this.transition.alpha = 1;
        }
        else if ( this.transition.phase === 'in' && this.transition.alpha <= 0 ) {
            // Fade-in complete
            this.transition = null;
        }
    }
});
```

---

## 12. Pause and Resume

### 12.1 Pause Implementation

```javascript
MyGame = ig.Game.extend({
    paused: false,

    update: function() {
        // Skip updates when paused
        if ( this.paused ) {
            return;
        }

        this.parent();
    },

    draw: function() {
        this.parent();

        // Draw pause overlay
        if ( this.paused ) {
            ig.system.context.fillStyle = 'rgba(0,0,0,0.5)';
            ig.system.context.fillRect(0, 0, ig.system.width, ig.system.height);

            ig.system.context.fillStyle = 'white';
            ig.system.context.font = '20px monospace';
            ig.system.context.fillText(
                'PAUSED',
                ig.system.width / 2 - 40,
                ig.system.height / 2
            );
        }
    },

    togglePause: function() {
        this.paused = !this.paused;

        if ( this.paused ) {
            // Pause all entity timers
            for ( var i = 0; i < this.entities.length; i++ ) {
                if ( this.entities[i].currentAnim ) {
                    this.entities[i].currentAnim.timer.pause();
                }
            }
        } else {
            // Resume all entity timers
            for ( var i = 0; i < this.entities.length; i++ ) {
                if ( this.entities[i].currentAnim ) {
                    this.entities[i].currentAnim.timer.unpause();
                }
            }
        }
    }
});
```

### 12.2 Pause Effects on Game Loop

```
Normal operation:
───────────────────────────────────────▶ Time
│  Update  │  Update  │  Update  │
│  Draw    │  Draw    │  Draw    │

Paused (updates skipped, draws continue):
───────────────────────────────────────▶ Time
│  Draw    │  Draw    │  Draw    │
│  (skip)  │  (skip)  │  (skip)  │

Note: requestAnimationFrame continues calling loop()
      but update() is skipped
```

---

## 13. Performance Monitoring

### 13.1 Debug Statistics

```javascript
MyGame = ig.Game.extend({
    debug: true,
    fps: 0,
    frameTime: 0,

    update: function() {
        this.parent();

        // Calculate FPS
        this.fps = 1 / ig.system.tick;
        this.frameTime = ig.system.tick * 1000;  // ms
    },

    draw: function() {
        this.parent();

        if ( this.debug ) {
            // Draw debug overlay
            ig.system.context.fillStyle = 'rgba(0,0,0,0.5)';
            ig.system.context.font = '10px monospace';
            ig.system.context.fillText('FPS: ' + this.fps.toFixed(1), 5, 15);
            ig.system.context.fillText('Frame: ' + this.frameTime.toFixed(2) + 'ms', 5, 25);
            ig.system.context.fillText('Entities: ' + this.entities.length, 5, 35);
            ig.system.context.fillText('Draw calls: ' + ig.Image.drawCount, 5, 45);
        }
    }
});
```

### 13.2 Performance Metrics

```
Key metrics to monitor:
────────────────────────────────────────────
FPS          │  Target: 60 | Warning: < 30
Frame Time   │  Target: 16ms | Warning: > 33ms
Entity Count │  Warning: > 200 (depends on complexity)
Draw Calls   │  Warning: > 500 per frame
Memory       │  Monitor for leaks (images, sounds)
```

---

## 14. Rust Reimplementation Considerations

### 14.1 Game Loop Architecture

```rust
// Recommended Rust structure using winit for window/events

use std::time::{Duration, Instant};
use winit::event_loop::EventLoop;
use winit::window::Window;

pub struct GameLoop {
    target_fps: u32,
    target_frame_time: Duration,
    accumulated_time: Duration,
    last_frame_time: Instant,
    time_scale: f32,
    max_delta: Duration,
}

impl GameLoop {
    pub fn new(target_fps: u32) -> Self {
        Self {
            target_fps,
            target_frame_time: Duration::from_secs_f32(1.0 / target_fps as f32),
            accumulated_time: Duration::ZERO,
            last_frame_time: Instant::now(),
            time_scale: 1.0,
            max_delta: Duration::from_millis(50), // 20 FPS cap
        }
    }

    pub fn run<F>(&mut self, mut game: Box<dyn Game>, mut render_fn: F)
    where
        F: FnMut(&mut dyn Game),
    {
        let mut event_loop = EventLoop::new();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::MainEventsCleared => {
                    // Request redraw
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    self.step(&mut game);
                    render_fn(&mut *game);
                }
                Event::LoopDestroyed => {
                    game.on_close();
                }
                _ => {}
            }
        });
    }

    pub fn step(&mut self, game: &mut dyn Game) {
        let now = Instant::now();
        let mut delta = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        // Apply time scale and cap
        delta = Duration::from_secs_f32(
            delta.as_secs_f32() * self.time_scale
        );
        delta = delta.min(self.max_delta);

        // Update game time
        self.accumulated_time += delta;

        // Update game (variable time step)
        game.update(delta.as_secs_f32());
    }
}

pub trait Game {
    fn update(&mut self, delta_time: f32);
    fn render(&mut self);
    fn on_close(&mut self);
}
```

### 14.2 Delta Time in Rust

```rust
// Using f32 for seconds (matches Impact's approach)

pub struct Timer {
    base: Instant,
    target: f32,  // seconds
    paused_at: Option<Instant>,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            base: Instant::now(),
            target: 0.0,
            paused_at: None,
        }
    }

    pub fn delta(&self) -> f32 {
        let elapsed = self.base.elapsed().as_secs_f32();
        elapsed - self.target
    }

    pub fn set(&mut self, seconds: f32) {
        self.base = Instant::now();
        self.target = seconds;
        self.paused_at = None;
    }

    pub fn pause(&mut self) {
        if self.paused_at.is_none() {
            self.paused_at = Some(Instant::now());
        }
    }

    pub fn unpause(&mut self) {
        if let Some(paused_at) = self.paused_at {
            // Adjust base to account for paused duration
            let paused_duration = paused_at.elapsed();
            self.base += paused_duration;
            self.paused_at = None;
        }
    }
}
```

### 14.3 Timing Considerations for Rust

1. **Use `Instant` for monotonic time**: Avoids issues with system clock changes
2. **Cap delta time**: Prevent spiral of death on frame drops
3. **Time scale as f32**: 1.0 = normal, 0.5 = slow-mo, 0.0 = pause
4. **Duration vs f32**: Convert to f32 for game logic (matches Impact)
5. **Event loop integration**: winit handles rAF equivalent on all platforms

### 14.4 Complete Rust Game Loop Example

```rust
use std::time::{Duration, Instant};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent};
use winit::window::Window;

pub struct GameEngine {
    game: Box<dyn Game>,
    loop_state: GameLoop,
}

struct GameLoop {
    last_update: Instant,
    accumulator: Duration,
    fixed_delta: Duration,
    max_delta: Duration,
    time_scale: f32,
}

impl GameLoop {
    fn new(fixed_fps: u32) -> Self {
        Self {
            last_update: Instant::now(),
            accumulator: Duration::ZERO,
            fixed_delta: Duration::from_secs_f32(1.0 / fixed_fps as f32),
            max_delta: Duration::from_secs_f32(0.1), // 100ms cap
            time_scale: 1.0,
        }
    }

    fn update(&mut self) -> Duration {
        let now = Instant::now();
        let mut delta = now.duration_since(self.last_update);
        self.last_update = now;

        // Apply time scale
        delta = Duration::from_secs_f32(delta.as_secs_f32() * self.time_scale);

        // Cap delta to prevent spiral of death
        delta = delta.min(self.max_delta);

        self.accumulator += delta;
        delta
    }

    fn should_fixed_update(&self) -> bool {
        self.accumulator >= self.fixed_delta
    }

    fn consume_fixed_step(&mut self) {
        self.accumulator -= self.fixed_delta;
    }
}

impl GameEngine {
    pub fn new(game: Box<dyn Game>) -> Self {
        Self {
            game,
            loop_state: GameLoop::new(60),
        }
    }

    pub fn run(mut self, window: Window) {
        let mut event_loop = EventLoop::new();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    self.game.on_close();
                    *control_flow = ControlFlow::Exit;
                }
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    // Variable time step update
                    let delta = self.loop_state.update();
                    self.game.update(delta.as_secs_f32());

                    // Optional: Fixed time step for physics
                    while self.loop_state.should_fixed_update() {
                        self.game.fixed_update();
                        self.loop_state.consume_fixed_step();
                    }

                    // Render
                    self.game.render();
                }
                _ => {}
            }
        });
    }
}

pub trait Game {
    fn update(&mut self, delta_time: f32);
    fn fixed_update(&mut self) {
        // Default: no fixed update
    }
    fn render(&mut self);
    fn on_close(&mut self) {}
}
```

---

## Appendix A: Complete Timing Flow Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         COMPLETE TIMING FLOW                                │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  BROWSER rAF (requestAnimationFrame)                                       │
│         │                                                                   │
│         ▼                                                                   │
│  ┌─────────────────┐                                                        │
│  │ ig.System.loop │                                                        │
│  └────────┬────────┘                                                        │
│           │                                                                  │
│           ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐           │
│  │ 1. ig.Timer.step()                                          │           │
│  │    ├─ current = Date.now()                                  │           │
│  │    ├─ delta = (current - _last) / 1000                      │           │
│  │    ├─ time += min(delta, maxStep) * timeScale              │           │
│  │    └─ _last = current                                       │           │
│  └─────────────────────────────────────────────────────────────┘           │
│           │                                                                  │
│           ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐           │
│  │ 2. system.tick = clock.tick()                               │           │
│  │    └─ Returns delta since last tick                         │           │
│  └─────────────────────────────────────────────────────────────┘           │
│           │                                                                  │
│           ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐           │
│  │ 3. game.run()                                               │           │
│  │    ├─ game.update()                                         │           │
│  │    │   ├─ Update entities (physics + delta)                │           │
│  │    │   ├─ Update animations (timer.delta())                │           │
│  │    │   └─ Process collisions                               │           │
│  │    └─ game.draw()                                           │           │
│  │        ├─ Clear screen                                      │           │
│  │        ├─ Draw backgrounds                                  │           │
│  │        ├─ Draw entities                                     │           │
│  │        └─ Draw foreground                                   │           │
│  └─────────────────────────────────────────────────────────────┘           │
│           │                                                                  │
│           ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐           │
│  │ 4. input.clearPressed()                                     │           │
│  │    └─ Reset key pressed state                               │           │
│  └─────────────────────────────────────────────────────────────┘           │
│           │                                                                  │
│           └──────────────────────────────────────────────────────┐         │
│                                                                  │         │
│                                                                  ▼         │
│                                                           Next rAF        │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

## Appendix B: Timing Constants Reference

```javascript
// Default values in Impact.js
ig.Timer.maxStep = 0.05;        // 50ms = 20 FPS minimum
ig.Timer.timeScale = 1.0;       // Normal speed
ig.system.fps = 60;             // Target FPS
ig.system.tick = 1/60;          // ~0.0167s per frame

// Common time scale values
0.0  =  Time freeze (pause)
0.25 =  Extreme slow motion (25% speed)
0.5  =  Slow motion (50% speed)
1.0  =  Normal speed
2.0  =  Fast forward (200% speed)
```
