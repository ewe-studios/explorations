---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs/Impact/lib/impact
explored_at: 2026-03-20
---

# Impact.js Rendering System - Deep Dive

**Scope:** Canvas 2D rendering pipeline, image drawing, tilemap rendering, scale modes, position interpolation

---

## Table of Contents

1. [Rendering Architecture Overview](#1-rendering-architecture-overview)
2. [Canvas 2D Context Setup](#2-canvas-2d-context-setup)
3. [Scale Modes (CRISP vs SMOOTH)](#3-scale-modes-crisp-vs-smooth)
4. [Position Interpolation Modes](#4-position-interpolation-modes)
5. [Image Loading and Caching](#5-image-loading-and-caching)
6. [Image Drawing Pipeline](#6-image-drawing-pipeline)
7. [Sprite Sheet Tile Drawing](#7-sprite-sheet-tile-drawing)
8. [Flip and Rotation Transforms](#8-flip-and-rotation-transforms)
9. [Background Map Rendering](#9-background-map-rendering)
10. [Pre-Rendered Chunk Optimization](#10-pre-rendered-chunk-optimization)
11. [Entity Rendering](#11-entity-rendering)
12. [Render Order and Layering](#12-render-order-and-layering)
13. [Screen Culling and Viewport](#13-screen-culling-and-viewport)
14. [Retina/High-DPI Support](#14-retinahigh-dpi-support)
15. [Performance Characteristics](#15-performance-characteristics)

---

## 1. Rendering Architecture Overview

### 1.1 The Rendering Pipeline

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         GAME RENDER FRAME                                   │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ig.Game.draw()                                                             │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────┐                                                        │
│  │ ig.system.clear │ ────▶ Fill canvas with clearColor                     │
│  └─────────────────┘                                                        │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐           │
│  │ Calculate Screen Position (with rounding for pixel-perfect) │           │
│  │ game._rscreen = round(game.screen) / scale                   │           │
│  └─────────────────────────────────────────────────────────────┘           │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐           │
│  │ RENDER BACKGROUND MAPS (non-foreground layers)              │           │
│  │ For each map: map.setScreenPos() + map.draw()               │           │
│  │   ├─▶ If preRender: drawPreRendered() (cached chunks)      │           │
│  │   └─▶ Else: drawTiled() (tile-by-tile)                     │           │
│  └─────────────────────────────────────────────────────────────┘           │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐           │
│  │ RENDER ENTITIES (sorted by zIndex)                          │           │
│  │ For each entity: entity.draw()                              │           │
│  │   └─▶ currentAnim.draw(pos - screen)                       │           │
│  │        └─▶ sheet.image.drawTile()                          │           │
│  └─────────────────────────────────────────────────────────────┘           │
│       │                                                                     │
│       ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐           │
│  │ RENDER FOREGROUND MAPS (overlay layers)                     │           │
│  │ For each foreground map: map.draw()                         │           │
│  └─────────────────────────────────────────────────────────────┘           │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Key Rendering Components

| Component | File | Responsibility |
|-----------|------|----------------|
| `ig.System` | `system.js` | Canvas context, scale mode, draw mode, clear |
| `ig.Image` | `image.js` | Image loading, caching, draw/drawTile |
| `ig.Map` | `map.js` | Base tilemap data structure |
| `ig.BackgroundMap` | `background-map.js` | Parallax scrolling, pre-rendered chunks |
| `ig.CollisionMap` | `collision-map.js` | Tile-based collision (not visual) |
| `ig.Entity` | `entity.js` | Entity positioning and animation drawing |
| `ig.Animation` | `animation.js` | Sprite animation frame drawing |

---

## 2. Canvas 2D Context Setup

### 2.1 System Initialization

```javascript
ig.System = ig.Class.extend({
    init: function( canvasId, fps, width, height, scale ) {
        this.canvas = ig.$(canvasId);
        this.context = this.canvas.getContext('2d');
        this.resize( width, height, scale );

        // Set draw mode (position calculation method)
        this.getDrawPos = ig.System.drawMode;

        // Apply scale mode if scaling
        if( this.scale != 1 ) {
            ig.System.scaleMode = ig.System.SCALE.CRISP;
        }
        ig.System.scaleMode( this.canvas, this.context );
    }
});
```

### 2.2 Canvas Sizing and Resolution

```javascript
ig.System.prototype.resize = function( width, height, scale ) {
    this.width = width;
    this.height = height;
    this.scale = scale || this.scale;

    // Real pixel dimensions (accounting for scale)
    this.realWidth = this.width * this.scale;
    this.realHeight = this.height * this.scale;

    // Set canvas element dimensions
    this.canvas.width = this.realWidth;
    this.canvas.height = this.realHeight;
};
```

**Key Insight:** Impact.js distinguishes between:
- `width/height`: Logical game resolution (e.g., 320x240)
- `realWidth/realHeight`: Actual canvas pixel dimensions (e.g., 640x480 with scale=2)

### 2.3 Clear Operation

```javascript
ig.System.prototype.clear = function( color ) {
    this.context.fillStyle = color;
    this.context.fillRect( 0, 0, this.realWidth, this.realHeight );
};
```

---

## 3. Scale Modes (CRISP vs SMOOTH)

### 3.1 Scale Mode Enumeration

```javascript
ig.System.SCALE = {
    CRISP: function( canvas, context ) {
        // Disable image smoothing for pixel-perfect rendering
        ig.setVendorAttribute( context, 'imageSmoothingEnabled', false );

        // CSS property for pixelated rendering
        canvas.style.imageRendering = '-moz-crisp-edges';
        canvas.style.imageRendering = '-o-crisp-edges';
        canvas.style.imageRendering = '-webkit-optimize-contrast';
        canvas.style.imageRendering = 'crisp-edges';
        canvas.style.msInterpolationMode = 'nearest-neighbor';
    },

    SMOOTH: function( canvas, context ) {
        // Enable bicubic smoothing
        ig.setVendorAttribute( context, 'imageSmoothingEnabled', true );
        canvas.style.imageRendering = '';
        canvas.style.msInterpolationMode = '';
    }
};
```

### 3.2 Visual Comparison

```
CRISP Mode (Nearest-Neighbor):
┌───┬───┬───┬───┐
│███│███│░░░│░░░│  ← Sharp pixel edges
├───┼───┼───┼───┤
│███│███│░░░│░░░│
├───┼───┼───┼───┤
│░░░│░░░│███│███│
└───┴───┴───┴───┘

SMOOTH Mode (Bicubic):
┌───┬───┬───┬───┐
│██ │▒▒▒│▒░░│░░░│  ← Blended edges
├───┼───┼───┼───┤
│▓▓ │▒▒▒│▒░░│░░░│
├───┼───┼───┼───┤
│▒▒▒│▒░░│░▓▓│▓░░│
└───┴───┴───┴───┘
```

### 3.3 When to Use Each

| Mode | Use Case | Example Games |
|------|----------|---------------|
| `CRISP` | Pixel art, retro games, tile-based graphics | Platformers, RPGs, Metroidvanias |
| `SMOOTH` | Hand-drawn art, vector-style graphics, photography | Puzzle games, visual novels |

---

## 4. Position Interpolation Modes

### 4.1 Draw Mode Enumeration

```javascript
ig.System.DRAW = {
    // Round before scaling - authentic pixel art look
    AUTHENTIC: function( p ) {
        return Math.round(p) * this.scale;
    },

    // Scale then round - smoother movement
    SMOOTH: function( p ) {
        return Math.round(p * this.scale);
    },

    // Pure subpixel - smoothest but may cause blur
    SUBPIXEL: function( p ) {
        return p * this.scale;
    }
};

// Default is SMOOTH
ig.System.drawMode = ig.System.DRAW.SMOOTH;
```

### 4.2 Visual Behavior Comparison

```
Position = 10.7, Scale = 2

AUTHENTIC: Math.round(10.7) * 2 = 11 * 2 = 22px
           └─▶ Snaps to pixel grid BEFORE scaling
           └─▶ Movement appears in "steps" of 0.5 pixels

SMOOTH: Math.round(10.7 * 2) = Math.round(21.4) = 21px
        └─▶ Allows subpixel precision at logical resolution
        └─▶ Smooth movement, still aligned to physical pixels

SUBPIXEL: 10.7 * 2 = 21.4px
          └─▶ True subpixel positioning
          └─▶ Smoothest movement but may cause blur/aliasing
```

### 4.3 Usage in Entity Drawing

```javascript
// In game.js
this._rscreen = {
    x: ig.system.getDrawPos(this.screen.x) / ig.system.scale,
    y: ig.system.getDrawPos(this.screen.y) / ig.system.scale
};

// Entity draws relative to rounded screen position
entity.draw( entity.pos.x - this._rscreen.x, entity.pos.y - this._rscreen.y );
```

---

## 5. Image Loading and Caching

### 5.1 Image Caching System

```javascript
// Global cache: path → Image instance
ig.Image.cache = {};

// Prevent duplicate loading
ig.Image.staticInstantiate = function( path ) {
    return ig.Image.cache[path] || null;
};
```

### 5.2 Image Loading Process

```javascript
ig.Image = ig.Class.extend({
    data: null,        // The actual HTMLImageElement
    width: 0,
    height: 0,
    loaded: false,
    path: '',

    load: function( loadCallback ) {
        if( this.loaded ) {
            if( loadCallback ) {
                loadCallback( this.path, true );  // Already loaded
            }
            return;
        }

        // Already loading?
        if( this.data ) {
            this.loadCallback = loadCallback;
            return;
        }

        this.loadCallback = loadCallback;
        this.data = new Image();

        var that = this;
        this.data.onload = function() {
            that.width = that.data.width;
            that.height = that.data.height;
            that.loaded = true;

            // Add to cache
            ig.Image.cache[that.path] = that;

            // Call callback
            if( that.loadCallback ) {
                that.loadCallback(that.path, false);
            }
        };

        this.data.src = this.path;
    }
});
```

### 5.3 Cache Behavior

```
┌─────────────────────────────────────────────────────────────────┐
│                    IMAGE CACHE FLOW                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  new ig.Image('gfx/sprite.png')                                 │
│         │                                                        │
│         ▼                                                        │
│  ┌─────────────────┐                                             │
│  │ staticInstantiate │ ──▶ Check cache['gfx/sprite.png']        │
│  └─────────────────┘                                             │
│         │                                                        │
│    ┌────┴────┐                                                   │
│    │         │                                                   │
│    ▼         ▼                                                   │
│  Cached   Not Cached                                             │
│    │         │                                                    │
│    │         ▼                                                    │
│    │    ┌─────────────┐                                          │
│    │    │ Create new  │                                          │
│    │    │ HTMLImage   │                                          │
│    │    └─────────────┘                                          │
│    │         │                                                   │
│    │         ▼                                                   │
│    │    ┌─────────────┐                                          │
│    │    │ Set onload  │                                          │
│    │    │ Set src     │                                          │
│    │    └─────────────┘                                          │
│    │         │                                                   │
│    │         ▼                                                   │
│    │    ┌─────────────┐                                          │
│    └───▶│   .loaded   │◀─────────────────────────────────┐      │
│         │   = true    │                                   │      │
│         └─────────────┘                                   │      │
│               │                                           │      │
│               ▼                                           │      │
│         ┌─────────────┐                                   │      │
│         │ Add to      │◀── Same path reuses this instance │      │
│         │ ig.Image.cache │                               │      │
│         └─────────────┘                                   │      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Image Drawing Pipeline

### 6.1 Full Image Draw

```javascript
ig.Image.prototype.draw = function( targetX, targetY, sourceX, sourceY, width, height ) {
    if( !this.loaded ) { return; }

    var scale = ig.system.scale;

    // Convert source coordinates to scaled pixels
    sourceX = sourceX ? sourceX * scale : 0;
    sourceY = sourceY ? sourceY * scale : 0;
    width = (width ? width : this.width) * scale;
    height = (height ? height : this.height) * scale;

    // Draw using Canvas 2D drawImage
    ig.system.context.drawImage(
        this.data,      // Source image
        sourceX, sourceY, width, height,  // Source rect
        ig.system.getDrawPos(targetX),    // Destination X
        ig.system.getDrawPos(targetY),    // Destination Y
        width, height   // Destination size
    );

    ig.Image.drawCount++;  // Debug statistics
};
```

### 6.2 drawImage Parameters Breakdown

```javascript
context.drawImage(
    image,              // HTMLImageElement
    sx, sy,             // Source X, Y in the image
    sWidth, sHeight,    // Source width, height
    dx, dy,             // Destination X, Y on canvas
    dWidth, dHeight     // Destination width, height
);
```

---

## 7. Sprite Sheet Tile Drawing

### 7.1 Tile Calculation from Sprite Sheet

```javascript
ig.Image.prototype.drawTile = function(
    targetX, targetY,   // Where to draw
    tile,               // Tile index (0-based)
    tileWidth, tileHeight,  // Dimensions of each tile
    flipX, flipY        // Flip flags
) {
    tileHeight = tileHeight ? tileHeight : tileWidth;

    if( !this.loaded ) { return; }

    var scale = ig.system.scale;
    var tileWidthScaled = Math.floor(tileWidth * scale);
    var tileHeightScaled = Math.floor(tileHeight * scale);

    // Calculate source position in sprite sheet
    // Tiles are arranged left-to-right, top-to-bottom
    var sourceX = ( Math.floor(tile * tileWidth) % this.width ) * scale;
    var sourceY = ( Math.floor(tile * tileWidth / this.width) * tileHeight ) * scale;

    // ... flip handling ...

    ig.system.context.drawImage(
        this.data,
        sourceX, sourceY,
        tileWidthScaled, tileHeightScaled,
        ig.system.getDrawPos(targetX) * scaleX - (flipX ? tileWidthScaled : 0),
        ig.system.getDrawPos(targetY) * scaleY - (flipY ? tileHeightScaled : 0),
        tileWidthScaled, tileHeightScaled
    );
};
```

### 7.2 Sprite Sheet Layout Example

```
Sprite Sheet: gfx/player.png (32x32 tiles)
┌─────────────────────────────────────────────────┐
│ Tile 0 │ Tile 1 │ Tile 2 │ Tile 3              │
│ (0,0)  │ (32,0) │ (64,0) │ (96,0)              │
├────────┼────────┼────────┼─────────────────────┤
│ Tile 4 │ Tile 5 │ Tile 6 │ Tile 7              │
│ (0,32) │ (32,32)│ (64,32)│ (96,32)             │
├────────┼────────┼────────┼─────────────────────┤
│ Tile 8 │ Tile 9 │ Tile 10│ Tile 11             │
│ (0,64) │ (32,64)│ (64,64)│ (96,64)             │
└────────┴────────┴────────┴─────────────────────┘

For tile 5:
  sourceX = (5 * 32) % 128 * scale = 160 % 128 * scale = 32 * scale
  sourceY = (5 * 32 / 128) * 32 * scale = 1.25 * 32 * scale = 32 * scale
```

---

## 8. Flip and Rotation Transforms

### 8.1 Canvas Transform for Flipping

```javascript
ig.Image.prototype.drawTile = function(...) {
    var scaleX = flipX ? -1 : 1;
    var scaleY = flipY ? -1 : 1;

    if( flipX || flipY ) {
        // Save current transform
        ig.system.context.save();

        // Apply scale transform (flip)
        ig.system.context.scale( scaleX, scaleY );
    }

    // Draw with adjusted position
    ig.system.context.drawImage(
        this.data,
        sourceX, sourceY,
        tileWidthScaled, tileHeightScaled,
        ig.system.getDrawPos(targetX) * scaleX - (flipX ? tileWidthScaled : 0),
        ig.system.getDrawPos(targetY) * scaleY - (flipY ? tileHeightScaled : 0),
        tileWidthScaled, tileHeightScaled
    );

    if( flipX || flipY ) {
        // Restore original transform
        ig.system.context.restore();
    }
};
```

### 8.2 Flip Visual Example

```
Original (no flip):        FlipX (horizontal):
┌──────────┐               ┌──────────┐
│    →     │               │     ←    │
│  Player  │   flipX()     │  Player  │
│   ◢◣     │   ───────▶    │    ◥◤    │
└──────────┘               └──────────┘

Original:                  FlipY (vertical):
┌──────────┐               ┌──────────┐
│    ↑     │               │    ↓     │
│  Player  │   flipY()     │  Player  │
│   ◢◣     │   ───────▶    │   ◥▴     │
└──────────┘               └──────────┘
```

### 8.3 Rotation Support (via Animation)

```javascript
ig.Animation = ig.Class.extend({
    flip: {x: false, y: false},
    angle: 0,  // Radians

    draw: function( x, y ) {
        if( this.flip.x || this.flip.y || this.angle ) {
            ig.system.context.save();

            // Translate to center of animation
            var w = this.frameWidth * ig.system.scale;
            var h = this.frameHeight * ig.system.scale;
            ig.system.context.translate( x + w/2, y + h/2 );

            // Apply rotation
            if( this.angle ) {
                ig.system.context.rotate( this.angle );
            }

            // Apply flip
            ig.system.context.scale(
                this.flip.x ? -1 : 1,
                this.flip.y ? -1 : 1
            );

            // Draw centered
            this.sheet.image.drawTile(
                -w/2, -h/2,  // Centered position
                this.tile,
                this.frameWidth,
                this.frameHeight
            );

            ig.system.context.restore();
        } else {
            this.sheet.image.drawTile(
                x, y,
                this.tile,
                this.frameWidth,
                this.frameHeight
            );
        }
    }
});
```

---

## 9. Background Map Rendering

### 9.1 Background Map Structure

```javascript
ig.BackgroundMap = ig.Map.extend({
    tiles: null,           // ig.Image tileset
    scroll: {x: 0, y: 0},  // Current scroll position
    distance: 1,           // Parallax factor (1 = same speed as screen)
    repeat: false,         // Tile at edges
    foreground: false,     // Draw after entities
    preRender: false,      // Use pre-rendered chunks
    enabled: true,

    // Animated tiles: anims[tileIndex-1] = Animation
    anims: {}
});
```

### 9.2 Parallax Scrolling

```javascript
ig.BackgroundMap.prototype.setScreenPos = function( x, y ) {
    // Scroll position divided by distance = parallax offset
    this.scroll.x = x / this.distance;
    this.scroll.y = y / this.distance;
};
```

### 9.3 Parallax Distance Values

```
distance = 0.5  → Background moves at HALF screen speed (distant)
distance = 1.0  → Background moves WITH screen (same plane)
distance = 2.0  → Background moves at DOUBLE speed (closer than screen)
```

### 9.4 Draw Method Decision Tree

```
ig.BackgroundMap.draw()
        │
        ▼
    ┌─────────────┐
    │ preRender?  │
    └──────┬──────┘
           │
      ┌────┴────┐
      │         │
     Yes        No
      │         │
      ▼         ▼
┌───────────┐ ┌───────────┐
│drawPre-   │ │drawTiled()│
│Rendered() │ │           │
└───────────┘ └───────────┘
```

---

## 10. Pre-Rendered Chunk Optimization

### 10.1 Chunk System Overview

For large tilemaps, rendering tile-by-tile is expensive. Pre-rendering to offscreen canvases reduces draw calls.

```javascript
ig.BackgroundMap.prototype.preRenderMapToChunks = function() {
    var totalWidth = this.width * this.tilesize * ig.system.scale,
        totalHeight = this.height * this.tilesize * ig.system.scale;

    // Adjust chunk size for smaller layers
    this.chunkSize = Math.min( Math.max(totalWidth, totalHeight), this.chunkSize );

    var chunkCols = Math.ceil(totalWidth / this.chunkSize),
        chunkRows = Math.ceil(totalHeight / this.chunkSize);

    this.preRenderedChunks = [];
    for( var y = 0; y < chunkRows; y++ ) {
        this.preRenderedChunks[y] = [];
        for( var x = 0; x < chunkCols; x++ ) {
            this.preRenderedChunks[y][x] = this.preRenderChunk(
                x, y,
                (x == chunkCols-1) ? totalWidth - x * this.chunkSize : this.chunkSize,
                (y == chunkRows-1) ? totalHeight - y * this.chunkSize : this.chunkSize
            );
        }
    }
};
```

### 10.2 Chunk Pre-Rendering Process

```javascript
ig.BackgroundMap.prototype.preRenderChunk = function( cx, cy, w, h ) {
    // Calculate tile range
    var tw = w / this.tilesize / ig.system.scale + 1,
        th = h / this.tilesize / ig.system.scale + 1;

    var tx = Math.floor(cx * this.chunkSize / this.tilesize / ig.system.scale),
        ty = Math.floor(cy * this.chunkSize / this.tilesize / ig.system.scale);

    // Create offscreen canvas
    var chunk = ig.$new('canvas');
    chunk.width = w;
    chunk.height = h;
    chunk.retinaResolutionEnabled = false;  // Opt out for Ejecta

    var chunkContext = chunk.getContext('2d');
    ig.System.scaleMode(chunk, chunkContext);

    // Temporarily use chunk context for drawing
    var screenContext = ig.system.context;
    ig.system.context = chunkContext;

    // Draw all tiles in this chunk's area
    for( var x = 0; x < tw; x++ ) {
        for( var y = 0; y < th; y++ ) {
            if( x + tx < this.width && y + ty < this.height ) {
                var tile = this.data[y+ty][x+tx];
                if( tile ) {
                    this.tiles.drawTile(
                        x * this.tilesize - nx,
                        y * this.tilesize - ny,
                        tile - 1, this.tilesize
                    );
                }
            }
        }
    }

    ig.system.context = screenContext;

    // Convert canvas to Image (Chrome 49 workaround)
    var image = new Image();
    image.src = chunk.toDataURL();
    image.width = chunk.width;
    image.height = chunk.height;

    return image;
};
```

### 10.3 Chunk Layout Visualization

```
Large Map (2048x2048)
┌──────────────────────────────────────┐
│ Chunk 0,0 │ Chunk 1,0 │ Chunk 2,0   │
│ (512x512) │ (512x512) │ (512x512)   │
├───────────┼───────────┼─────────────┤
│ Chunk 0,1 │ Chunk 1,1 │ Chunk 2,1   │
│ (512x512) │ (512x512) │ (512x512)   │
├───────────┼───────────┼─────────────┤
│ Chunk 0,2 │ Chunk 1,2 │ Chunk 2,2   │
│ (512x512) │ (512x512) │ (512x512)   │
└───────────┴───────────┴─────────────┘

Screen Viewport (320x240) shown as [====]
Only draws chunks that intersect viewport:
┌──────────────────────────────────────┐
│           │           │               │
├───────────┼───────────┼─────────────┤
│        [===============]             │
│        [===Chunk 1,1===]             │
│        [=======┬=======]             │
├───────────┼─────[─Chunk 2,1─]───────┤
│           │     [       ]           │
└───────────┴─────[───────]───────────┘
```

### 10.4 Drawing Pre-Rendered Chunks

```javascript
ig.BackgroundMap.prototype.drawPreRendered = function() {
    if( !this.preRenderedChunks ) {
        this.preRenderMapToChunks();
    }

    var dx = ig.system.getDrawPos(this.scroll.x),
        dy = ig.system.getDrawPos(this.scroll.y);

    // Handle repeating maps
    if( this.repeat ) {
        var w = this.width * this.tilesize * ig.system.scale;
        dx = (dx % w + w) % w;
        var h = this.height * this.tilesize * ig.system.scale;
        dy = (dy % h + h) % h;
    }

    // Calculate which chunks are visible
    var minChunkX = Math.max( Math.floor(dx / this.chunkSize), 0 ),
        minChunkY = Math.max( Math.floor(dy / this.chunkSize), 0 ),
        maxChunkX = Math.ceil((dx + ig.system.realWidth) / this.chunkSize),
        maxChunkY = Math.ceil((dy + ig.system.realHeight) / this.chunkSize);

    // Draw visible chunks
    for( var cy = minChunkY; cy < maxChunkY; cy++ ) {
        for( var cx = minChunkX; cx < maxChunkX; cx++ ) {
            var chunk = this.preRenderedChunks[cy % maxRealChunkY][cx % maxRealChunkX];

            var x = -dx + cx * this.chunkSize;
            var y = -dy + cy * this.chunkSize;

            ig.system.context.drawImage( chunk, x, y );
            ig.Image.drawCount++;
        }
    }
};
```

---

## 11. Entity Rendering

### 11.1 Entity Draw Method

```javascript
ig.Entity.prototype.draw = function() {
    if( this.currentAnim ) {
        // Draw current animation frame
        this.currentAnim.draw(
            this.pos.x - ig.game._rscreen.x,
            this.pos.y - ig.game._rscreen.y
        );
    }
};
```

### 11.2 Animation Drawing

```javascript
ig.Animation.prototype.draw = function( x, y ) {
    if( !this.sheet.image.loaded ) { return; }

    // Handle flip/rotation
    if( this.flip.x || this.flip.y || this.angle ) {
        ig.system.context.save();

        var w = this.frameWidth * ig.system.scale;
        var h = this.frameHeight * ig.system.scale;

        // Translate to center for rotation
        ig.system.context.translate( x + w/2, y + h/2 );

        if( this.angle ) {
            ig.system.context.rotate( this.angle );
        }

        ig.system.context.scale(
            this.flip.x ? -1 : 1,
            this.flip.y ? -1 : 1
        );

        this.sheet.image.drawTile(
            -w/2, -h/2,
            this.tile,
            this.frameWidth,
            this.frameHeight
        );

        ig.system.context.restore();
    } else {
        this.sheet.image.drawTile(
            x, y,
            this.tile,
            this.frameWidth,
            this.frameHeight
        );
    }
};
```

---

## 12. Render Order and Layering

### 12.1 Complete Render Sequence

```javascript
ig.Game.prototype.draw = function() {
    // 1. Clear screen
    ig.system.clear( this.clearColor );

    // 2. Calculate rounded screen position
    this._rscreen = {
        x: ig.system.getDrawPos(this.screen.x) / ig.system.scale,
        y: ig.system.getDrawPos(this.screen.y) / ig.system.scale
    };

    // 3. Draw background maps (non-foreground)
    for( var i = 0; i < this.backgroundMaps.length; i++ ) {
        var map = this.backgroundMaps[i];
        if( !map.foreground && map.enabled ) {
            map.setScreenPos( this.screen.x, this.screen.y );
            map.draw();
        }
    }

    // 4. Draw entities (sorted by zIndex)
    for( var i = 0; i < this.entities.length; i++ ) {
        this.entities[i].draw();
    }

    // 5. Draw foreground maps (overlay)
    for( var i = 0; i < this.backgroundMaps.length; i++ ) {
        var map = this.backgroundMaps[i];
        if( map.foreground && map.enabled ) {
            map.setScreenPos( this.screen.x, this.screen.y );
            map.draw();
        }
    }
};
```

### 12.2 Layer Order Diagram

```
┌─────────────────────────────────────────────────────┐
│  FOREGROUND MAPS (drawn last, on top)              │
│  - UI overlay tiles                                 │
│  - Ceiling/foreground decoration                    │
├─────────────────────────────────────────────────────┤
│  ENTITIES (sorted by zIndex)                        │
│  - High zIndex: flying enemies, projectiles         │
│  - Medium zIndex: player, NPCs                      │
│  - Low zIndex: ground items, effects                │
├─────────────────────────────────────────────────────┤
│  BACKGROUND MAPS (drawn first, behind)             │
│  - Non-foreground: terrain, platforms               │
│  - Parallax layers: distant scenery                 │
└─────────────────────────────────────────────────────┘
```

### 12.3 Entity Sorting

```javascript
ig.Game.SORT = {
    Z_INDEX: function( a, b ) {
        return a.zIndex - b.zIndex;
    }
};

ig.Game.prototype.sortEntities = function() {
    this.entities.sort( this.sortBy );
};
```

---

## 13. Screen Culling and Viewport

### 13.1 Viewport Culling in drawTiled

```javascript
ig.BackgroundMap.prototype.drawTiled = function() {
    var tileOffsetX = (this.scroll.x / this.tilesize).toInt(),
        tileOffsetY = (this.scroll.y / this.tilesize).toInt(),
        pxOffsetX = this.scroll.x % this.tilesize,
        pxOffsetY = this.scroll.y % this.tilesize,

        // Calculate screen bounds in pixels
        pxMinX = -pxOffsetX - this.tilesize,
        pxMinY = -pxOffsetY - this.tilesize,
        pxMaxX = ig.system.width + this.tilesize - pxOffsetX,
        pxMaxY = ig.system.height + this.tilesize - pxOffsetY;

    // Only iterate over visible tiles
    for( var mapY = -1, pxY = pxMinY; pxY < pxMaxY; mapY++, pxY += this.tilesize) {
        var tileY = mapY + tileOffsetY;

        // Handle repeat/clamp
        if( tileY >= this.height || tileY < 0 ) {
            if( !this.repeat ) { continue; }  // Cull
            tileY = (tileY % this.height + this.height) % this.height;
        }

        for( var mapX = -1, pxX = pxMinX; pxX < pxMaxX; mapX++, pxX += this.tilesize ) {
            var tileX = mapX + tileOffsetX;

            if( tileX >= this.width || tileX < 0 ) {
                if( !this.repeat ) { continue; }  // Cull
                tileX = (tileX % this.width + this.width) % this.width;
            }

            // Draw only non-zero tiles
            if( (tile = this.data[tileY][tileX]) ) {
                // ... draw tile or animation ...
            }
        }
    }
};
```

### 13.2 Entity Culling (Manual Implementation)

Impact.js doesn't cull entities automatically. Implement in your game:

```javascript
MyGame = ig.Game.extend({
    draw: function() {
        this.parent();  // Draw backgrounds

        // Only draw visible entities
        for( var i = 0; i < this.entities.length; i++ ) {
            var ent = this.entities[i];
            if( this.isEntityVisible(ent) ) {
                ent.draw();
            }
        }
    },

    isEntityVisible: function( ent ) {
        var margin = 64;  // Extra margin for smooth scrolling
        return ent.pos.x + ent.size.x + margin > this.screen.x &&
               ent.pos.y + ent.size.y + margin > this.screen.y &&
               ent.pos.x - margin < this.screen.x + ig.system.width &&
               ent.pos.y - margin < this.screen.y + ig.system.height;
    }
});
```

---

## 14. Retina/High-DPI Support

### 14.1 Backing Store Pixel Ratio

```javascript
// Get vendor-prefixed backing store ratio
ig.getVendorAttribute = function( ctx, attr ) {
    return ctx[attr] ||
           ctx['moz' + attr.charAt(0).toUpperCase() + attr.slice(1)] ||
           ctx['webkit' + attr.charAt(0).toUpperCase() + attr.slice(1)] ||
           ctx['ms' + attr.charAt(0).toUpperCase() + attr.slice(1)];
};
```

### 14.2 High-DPI Pixel Extraction

```javascript
ig.getImagePixels = function( image, x, y, width, height ) {
    var canvas = ig.$new('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    var ctx = canvas.getContext('2d');

    ig.System.SCALE.CRISP(canvas, ctx);

    // Get backing store ratio
    var ratio = ig.getVendorAttribute( ctx, 'backingStorePixelRatio' ) || 1;

    var realWidth = image.width / ratio,
        realHeight = image.height / ratio;

    canvas.width = Math.ceil( realWidth );
    canvas.height = Math.ceil( realHeight );

    ctx.drawImage( image, 0, 0, realWidth, realHeight );

    return (ratio === 1)
        ? ctx.getImageData( x, y, width, height )
        : ctx.getImageDataHD( x, y, width, height );
};
```

---

## 15. Performance Characteristics

### 15.1 Draw Call Optimization

| Technique | Impact | When to Use |
|-----------|--------|-------------|
| Pre-rendered chunks | ⬇️ 90% draw calls | Large static backgrounds |
| Entity culling | ⬇️ 50-80% draw calls | Large worlds, many entities |
| Sprite sheets | ⬇️ Texture binds | All sprite-based games |
| Chunk caching | ⬇️ CPU rasterization | Tile-heavy games |

### 15.2 Draw Count Statistics

```javascript
// Debug: track draw calls per frame
ig.Image.drawCount = 0;

// In game loop
draw: function() {
    ig.Image.drawCount = 0;
    // ... draw calls ...
    console.log('Draw calls:', ig.Image.drawCount);
}
```

### 15.3 Performance Recommendations

1. **Use preRender: true** for large, static background layers
2. **Enable entity culling** for worlds larger than screen
3. **Batch by tileset** - minimize tileset switches
4. **Use CRISP scale mode** for pixel art (sharper, no blur)
5. **Keep sprite sheets power-of-2** for GPU optimization
6. **Limit entity count** - use object pooling for projectiles/particles

---

## Appendix A: Complete Rendering Flow Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                           FULL RENDER FRAME                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. FRAME START                                                             │
│     └─▶ ig.system.tick = delta time                                        │
│                                                                             │
│  2. CLEAR                                                                   │
│     └─▶ context.fillStyle = game.clearColor                               │
│     └─▶ context.fillRect(0, 0, realWidth, realHeight)                     │
│                                                                             │
│  3. CALCULATE SCREEN POSITION                                               │
│     └─▶ _rscreen = round(screen) / scale                                  │
│                                                                             │
│  4. FOR EACH BACKGROUND MAP (non-foreground)                               │
│     ├─▶ map.setScreenPos(screen.x, screen.y)                              │
│     ├─▶ If preRender: drawPreRendered()                                    │
│     │   ├─▶ Calculate visible chunks                                       │
│     │   └─▶ Draw chunk images                                              │
│     └─▶ Else: drawTiled()                                                  │
│         ├─▶ Calculate visible tile range                                   │
│         └─▶ For each visible tile: drawTile()                              │
│                                                                             │
│  5. FOR EACH ENTITY (sorted by zIndex)                                     │
│     ├─▶ If currentAnim: anim.draw(pos - _rscreen)                         │
│     │   ├─▶ Apply flip/rotation transforms                                 │
│     │   └─▶ sheet.image.drawTile()                                         │
│     └─▶ Else: custom draw()                                                │
│                                                                             │
│  6. FOR EACH FOREGROUND MAP                                                 │
│     └─▶ Same as step 4                                                     │
│                                                                             │
│  7. FRAME END                                                               │
│     └─▶ Request next animation frame                                       │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```
