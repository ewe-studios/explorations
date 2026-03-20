---
location: /home/darkvoid/Boxxed/@formulas/src.Gaming/src.Impactjs
repository: N/A (commercial license)
explored_at: 2026-03-20
---

# Impact.js Game Engine - Comprehensive Technical Exploration

**Version Analyzed:** Impact 1.24
**Author:** Dominique Walter
**License:** Commercial (http://impactjs.com/)
**Primary Use:** 2D HTML5 Canvas Game Engine

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Module System](#2-module-system)
3. [Class System](#3-class-system)
4. [Rendering System](#4-rendering-system)
5. [Tilemap System](#5-tilemap-system)
6. [Animation System](#6-animation-system)
7. [Entity System](#7-entity-system)
8. [Input System](#8-input-system)
9. [Audio System](#9-audio-system)
10. [Physics/Collision System](#10-physicscollision-system)
11. [World/Level System](#11-worldlevel-system)
12. [Timer System](#12-timer-system)
13. [Font System](#13-font-system)
14. [Debug System](#14-debug-system)
15. [Weltmeister Level Editor](#15-weltmeister-level-editor)
16. [Complete Game Loop](#16-complete-game-loop)
17. [Mathematical Foundations](#17-mathematical-foundations)
18. [Performance Considerations](#18-performance-considerations)

---

## 1. Architecture Overview

### 1.1 Core Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           IMPACT.JS ENGINE                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │    ig.main   │───▶│  ig.System   │───▶│  ig.Input    │              │
│  │   (Entry)    │    │  (Loop/Disp) │    │   (Input)    │              │
│  └──────────────┘    └──────────────┘    └──────────────┘              │
│         │                   │                       │                   │
│         │                   ▼                       │                   │
│         │            ┌──────────────┐               │                   │
│         │            │  ig.Loader   │               │                   │
│         │            │  (Resources) │               │                   │
│         │            └──────────────┘               │                   │
│         │                   │                       │                   │
│         ▼                   ▼                       ▼                   │
│  ┌─────────────────────────────────────────────────────────────┐       │
│  │                      ig.game (User Game)                     │       │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │       │
│  │  │  Entities   │  │  Tilemaps   │  │   Animations        │  │       │
│  │  │  (Update/   │  │  (Render/   │  │   (Sprite Sheets)   │  │       │
│  │  │   Draw)     │  │   Collision)│  │                     │  │       │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  │       │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │       │
│  │  │   Sound     │  │    Font     │  │     Timer           │  │       │
│  │  │  (WebAudio/ │  │  (Bitmap/   │  │   (Delta Time)      │  │       │
│  │  │   HTML5)    │  │   Metrics)  │  │                     │  │       │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  │       │
│  └─────────────────────────────────────────────────────────────┘       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Component Responsibility Matrix

| Component | Responsibility | Key Files |
|-----------|---------------|-----------|
| `ig` | Global namespace, module loader, utility functions | `impact.js` |
| `ig.System` | Game loop, canvas context, timing | `system.js` |
| `ig.Input` | Keyboard, mouse, touch, gamepad input | `input.js` |
| `ig.Loader` | Resource preloading with progress display | `loader.js` |
| `ig.Game` | Main game class, entity management, level loading | `game.js` |
| `ig.Entity` | Base entity class with physics and collision | `entity.js` |
| `ig.Map` | Base tilemap class | `map.js` |
| `ig.CollisionMap` | Tile-based collision detection | `collision-map.js` |
| `ig.BackgroundMap` | Parallax scrolling background maps | `background-map.js` |
| `ig.Animation` / `ig.AnimationSheet` | Sprite animation handling | `animation.js` |
| `ig.Image` | Image loading and drawing | `image.js` |
| `ig.Font` | Bitmap font rendering | `font.js` |
| `ig.Sound` / `ig.Music` / `ig.SoundManager` | Audio playback | `sound.js` |
| `ig.Timer` | Delta time and timing utilities | `timer.js` |

### 1.3 Initialization Flow

```
User calls ig.main()
        │
        ▼
┌───────────────────┐
│ Create ig.System  │ ──▶ Canvas setup, context, resize
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Create ig.Input   │ ──▶ Event listeners setup
└───────────────────┘
        │
        ▼
┌──────────────────────┐
│ Create ig.SoundMgr   │ ──▶ Format detection, WebAudio context
└──────────────────────┘
        │
        ▼
┌───────────────────┐
│ Create ig.Music   │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Create ig.Loader  │ ──▶ Begin resource loading
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Load Resources    │ ──▶ Images, Sounds, Fonts
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Instantiate Game  │ ──▶ Call game class constructor
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ Start Run Loop    │ ──▶ requestAnimationFrame
└───────────────────┘
```

---

## 2. Module System

### 2.1 Module Definition Pattern

Impact.js uses a custom AMD-like module system that handles dependencies asynchronously.

```javascript
ig.module('impact.game')
.requires(
    'impact.impact',
    'impact.entity',
    'impact.collision-map',
    'impact.background-map'
)
.defines(function(){ "use strict";
    // Module code here
    // ig.game, ig.entity, etc. are now available
});
```

### 2.2 Module System Implementation

```javascript
// Module registry
ig.modules = {};

// Current module being defined
ig._current = null;

// Queue of modules waiting for dependencies
ig._loadQueue = [];

// Counter for scripts loading
ig._waitForOnload = 0;

// Define a new module
ig.module = function( name ) {
    if( ig._current ) {
        throw( "Module '"+ig._current.name+"' defines nothing" );
    }
    if( ig.modules[name] && ig.modules[name].body ) {
        throw( "Module '"+name+"' is already defined" );
    }

    ig._current = {name: name, requires: [], loaded: false, body: null};
    ig.modules[name] = ig._current;
    ig._loadQueue.push(ig._current);
    return ig;
};

// Specify dependencies
ig.requires = function() {
    ig._current.requires = Array.prototype.slice.call(arguments);
    return ig;
};

// Define module body
ig.defines = function( body ) {
    ig._current.body = body;
    ig._current = null;
    ig._initDOMReady();
};
```

### 2.3 Dependency Resolution Algorithm

The module system uses a topological sort approach:

```javascript
_execModules: function() {
    var modulesLoaded = false;
    for( var i = 0; i < ig._loadQueue.length; i++ ) {
        var m = ig._loadQueue[i];
        var dependenciesLoaded = true;

        for( var j = 0; j < m.requires.length; j++ ) {
            var name = m.requires[j];
            if( !ig.modules[name] ) {
                // Dependency not loaded - load the script
                dependenciesLoaded = false;
                ig._loadScript( name, m.name );
            }
            else if( !ig.modules[name].loaded ) {
                // Module exists but body not executed
                dependenciesLoaded = false;
            }
        }

        if( dependenciesLoaded && m.body ) {
            // All dependencies satisfied - execute module
            ig._loadQueue.splice(i, 1);
            m.loaded = true;
            m.body();
            modulesLoaded = true;
            i--;
        }
    }

    // Recursively process - new modules may now be ready
    if( modulesLoaded ) {
        ig._execModules();
    }

    // Error: circular dependencies or missing modules
    else if( !ig.baked && ig._waitForOnload == 0 && ig._loadQueue.length != 0 ) {
        // Report unresolved dependencies
    }
}
```

### 2.4 Script Loading Mechanism

```javascript
_loadScript: function( name, requiredFrom ) {
    ig.modules[name] = {name: name, requires:[], loaded: false, body: null};
    ig._waitForOnload++;

    // Convert module path: 'impact.entity' → 'lib/impact/entity.js'
    var path = ig.prefix + ig.lib + name.replace(/\./g, '/') + '.js' + ig.nocache;

    var script = ig.$new('script');
    script.type = 'text/javascript';
    script.src = path;
    script.onload = function() {
        ig._waitForOnload--;
        ig._execModules();
    };
    script.onerror = function() {
        throw( 'Failed to load module '+name+' at ' + path );
    };
    ig.$('head')[0].appendChild(script);
}
```

---

## 3. Class System

### 3.1 Prototypal Inheritance Implementation

Impact.js uses a class system based on John Resig's Simple JavaScript Inheritance:

```javascript
var initializing = false;
var fnTest = /xyz/.test(function(){xyz;}) ? /\bparent\b/ : /.*/;

ig.Class = function(){};

ig.Class.extend = function(prop) {
    var parent = this.prototype;

    initializing = true;
    var prototype = new this();
    initializing = false;

    // Copy properties, wrapping functions that use 'parent'
    for( var name in prop ) {
        if(
            typeof(prop[name]) == "function" &&
            typeof(parent[name]) == "function" &&
            fnTest.test(prop[name])
        ) {
            // Wrap function to support this.parent()
            prototype[name] = (function(name, fn){
                return function() {
                    var tmp = this.parent;
                    this.parent = parent[name];
                    var ret = fn.apply(this, arguments);
                    this.parent = tmp;
                    return ret;
                };
            })( name, prop[name] );
        }
        else {
            prototype[name] = prop[name];
        }
    }

    // Constructor function
    function Class() {
        if( !initializing ) {
            // Support staticInstantiate for object pooling
            if( this.staticInstantiate ) {
                var obj = this.staticInstantiate.apply(this, arguments);
                if( obj ) {
                    return obj;
                }
            }
            // Deep copy object properties
            for( var p in this ) {
                if( typeof(this[p]) == 'object' ) {
                    this[p] = ig.copy(this[p]);
                }
            }
            // Call init constructor
            if( this.init ) {
                this.init.apply(this, arguments);
            }
        }
        return this;
    }

    Class.prototype = prototype;
    Class.prototype.constructor = Class;
    Class.extend = ig.Class.extend;
    Class.classId = prototype.classId = ++lastClassId;

    return Class;
};
```

### 3.2 Class ID System

Each class gets a unique ID for entity pooling:

```javascript
var lastClassId = 0;
Class.classId = prototype.classId = ++lastClassId;
```

### 3.3 Object Copy and Merge Utilities

```javascript
// Deep copy
ig.copy = function( object ) {
    if(
       !object || typeof(object) != 'object' ||
       object instanceof HTMLElement ||
       object instanceof ig.Class
    ) {
        return object;
    }
    else if( object instanceof Array ) {
        var c = [];
        for( var i = 0, l = object.length; i < l; i++) {
            c[i] = ig.copy(object[i]);
        }
        return c;
    }
    else {
        var c = {};
        for( var i in object ) {
            c[i] = ig.copy(object[i]);
        }
        return c;
    }
};

// Deep merge
ig.merge = function( original, extended ) {
    for( var key in extended ) {
        var ext = extended[key];
        if(
            typeof(ext) != 'object' ||
            ext instanceof HTMLElement ||
            ext instanceof ig.Class ||
            ext === null
        ) {
            original[key] = ext;
        }
        else {
            if( !original[key] || typeof(original[key]) != 'object' ) {
                original[key] = (ext instanceof Array) ? [] : {};
            }
            ig.merge( original[key], ext );
        }
    }
    return original;
};
```

---

## 4. Rendering System

### 4.1 Canvas 2D Rendering Only

**Important:** Impact.js 1.24 does NOT support WebGL. All rendering is done through the Canvas 2D API.

```javascript
// System initialization
ig.System = ig.Class.extend({
    init: function( canvasId, fps, width, height, scale ) {
        this.canvas = ig.$(canvasId);
        this.context = this.canvas.getContext('2d');
        this.resize( width, height, scale );

        // Set draw mode (position calculation)
        this.getDrawPos = ig.System.drawMode;

        // Apply scale mode
        if( this.scale != 1 ) {
            ig.System.scaleMode = ig.System.SCALE.CRISP;
        }
        ig.System.scaleMode( this.canvas, this.context );
    }
});
```

### 4.2 Scale Modes

```javascript
ig.System.SCALE = {
    // Pixel-perfect, nearest-neighbor scaling
    CRISP: function( canvas, context ) {
        ig.setVendorAttribute( context, 'imageSmoothingEnabled', false );
        canvas.style.imageRendering = '-moz-crisp-edges';
        canvas.style.imageRendering = '-o-crisp-edges';
        canvas.style.imageRendering = '-webkit-optimize-contrast';
        canvas.style.imageRendering = 'crisp-edges';
        canvas.style.msInterpolationMode = 'nearest-neighbor';
    },

    // Smooth/bicubic scaling
    SMOOTH: function( canvas, context ) {
        ig.setVendorAttribute( context, 'imageSmoothingEnabled', true );
        canvas.style.imageRendering = '';
        canvas.style.msInterpolationMode = '';
    }
};
```

### 4.3 Position Interpolation Modes

```javascript
ig.System.DRAW = {
    // Round before scaling - authentic pixel art look
    AUTHENTIC: function( p ) { return Math.round(p) * this.scale; },

    // Scale then round - smoother movement
    SMOOTH: function( p ) { return Math.round(p * this.scale); },

    // Pure subpixel - smoothest but may cause blur
    SUBPIXEL: function( p ) { return p * this.scale; }
};

// Default is SMOOTH
ig.System.drawMode = ig.System.DRAW.SMOOTH;
```

### 4.4 Image Drawing System

```javascript
ig.Image = ig.Class.extend({
    data: null,        // The actual Image object
    width: 0,
    height: 0,
    loaded: false,
    path: '',

    // Draw entire image
    draw: function( targetX, targetY, sourceX, sourceY, width, height ) {
        if( !this.loaded ) { return; }

        var scale = ig.system.scale;
        sourceX = sourceX ? sourceX * scale : 0;
        sourceY = sourceY ? sourceY * scale : 0;
        width = (width ? width : this.width) * scale;
        height = (height ? height : this.height) * scale;

        ig.system.context.drawImage(
            this.data, sourceX, sourceY, width, height,
            ig.system.getDrawPos(targetX),
            ig.system.getDrawPos(targetY),
            width, height
        );

        ig.Image.drawCount++;  // For debug stats
    },

    // Draw single tile from sprite sheet
    drawTile: function( targetX, targetY, tile, tileWidth, tileHeight, flipX, flipY ) {
        tileHeight = tileHeight ? tileHeight : tileWidth;

        if( !this.loaded ) { return; }

        var scale = ig.system.scale;
        var tileWidthScaled = Math.floor(tileWidth * scale);
        var tileHeightScaled = Math.floor(tileHeight * scale);

        var scaleX = flipX ? -1 : 1;
        var scaleY = flipY ? -1 : 1;

        if( flipX || flipY ) {
            ig.system.context.save();
            ig.system.context.scale( scaleX, scaleY );
        }

        // Calculate source position in sprite sheet
        var sourceX = ( Math.floor(tile * tileWidth) % this.width ) * scale;
        var sourceY = ( Math.floor(tile * tileWidth / this.width) * tileHeight ) * scale;

        ig.system.context.drawImage(
            this.data,
            sourceX, sourceY,
            tileWidthScaled, tileHeightScaled,
            ig.system.getDrawPos(targetX) * scaleX - (flipX ? tileWidthScaled : 0),
            ig.system.getDrawPos(targetY) * scaleY - (flipY ? tileHeightScaled : 0),
            tileWidthScaled, tileHeightScaled
        );

        if( flipX || flipY ) {
            ig.system.context.restore();
        }

        ig.Image.drawCount++;
    }
});
```

### 4.5 Image Caching System

```javascript
ig.Image.cache = {};

ig.Image.staticInstantiate = function( path ) {
    return ig.Image.cache[path] || null;
};

// This prevents loading the same image twice
ig.Image.prototype.load = function( loadCallback ) {
    if( this.loaded ) {
        if( loadCallback ) {
            loadCallback( this.path, true );
        }
        return;
    }
    // ... load image
    ig.Image.cache[this.path] = this;
};
```

### 4.6 Retina/High-DPI Support

```javascript
// Get pixels accounting for backing store ratio
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

### 4.7 Clear/Screen Setup

```javascript
ig.System.prototype.clear = function( color ) {
    this.context.fillStyle = color;
    this.context.fillRect( 0, 0, this.realWidth, this.realHeight );
};

ig.System.prototype.resize = function( width, height, scale ) {
    this.width = width;
    this.height = height;
    this.scale = scale || this.scale;

    this.realWidth = this.width * this.scale;
    this.realHeight = this.height * this.scale;
    this.canvas.width = this.realWidth;
    this.canvas.height = this.realHeight;
};
```

### 4.8 Rendering Flow Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                        GAME RENDER FRAME                           │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  1. ig.system.clear(color)                                         │
│     └─▶ Fill canvas with clearColor                               │
│                                                                    │
│  2. Calculate rounded screen position                              │
│     game._rscreen = round(game.screen) / scale                     │
│                                                                    │
│  3. For each BackgroundMap (non-foreground):                       │
│     map.setScreenPos(screen.x, screen.y)                          │
│     map.draw()                                                     │
│        ├─▶ If preRender: drawPreRendered()                        │
│        │    └─▶ Draw cached canvas chunks                         │
│        └─► Else: drawTiled()                                       │
│             └─▶ Draw tiles in visible area only                   │
│                                                                    │
│  4. For each Entity:                                               │
│     entity.draw()                                                  │
│        └─▶ currentAnim.draw(pos - screen)                         │
│             └─▶ sheet.image.drawTile()                            │
│                                                                    │
│  5. For each BackgroundMap (foreground):                          │
│     map.draw()                                                     │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

---

## 5. Tilemap System

### 5.1 Map Data Structure

Maps are stored as 2D arrays of tile indices:

```javascript
ig.Map = ig.Class.extend({
    tilesize: 8,
    width: 1,
    height: 1,
    pxWidth: 1,
    pxHeight: 1,
    data: [[]],  // 2D array: data[y][x] = tileIndex
    name: null,

    init: function( tilesize, data ) {
        this.tilesize = tilesize;
        this.data = data;
        this.height = data.length;
        this.width = data[0].length;
        this.pxWidth = this.width * this.tilesize;
        this.pxHeight = this.height * this.tilesize;
    },

    getTile: function( x, y ) {
        var tx = Math.floor( x / this.tilesize );
        var ty = Math.floor( y / this.tilesize );
        if( tx >= 0 && tx < this.width && ty >= 0 && ty < this.height ) {
            return this.data[ty][tx];
        }
        return 0;  // Out of bounds = empty tile
    },

    setTile: function( x, y, tile ) {
        var tx = Math.floor( x / this.tilesize );
        var ty = Math.floor( y / this.tilesize );
        if( tx >= 0 && tx < this.width && ty >= 0 && ty < this.height ) {
            this.data[ty][tx] = tile;
        }
    }
});
```

### 5.2 Background Map (Visual Layers)

```javascript
ig.BackgroundMap = ig.Map.extend({
    tiles: null,           // ig.Image tileset
    scroll: {x: 0, y: 0},  // Current scroll position
    distance: 1,           // Parallax scrolling factor
    repeat: false,         // Tile/repeat at edges
    tilesetName: '',
    foreground: false,     // Draw after entities
    enabled: true,

    // Pre-rendering optimization
    preRender: false,
    preRenderedChunks: null,
    chunkSize: 512,
    debugChunks: false,

    anims: {},  // Animated tiles: anims[tileIndex-1] = Animation

    setScreenPos: function( x, y ) {
        this.scroll.x = x / this.distance;
        this.scroll.y = y / this.distance;
    }
});
```

### 5.3 Pre-Rendered Chunk System

For large maps, pre-rendering to offscreen canvases improves performance:

```javascript
preRenderMapToChunks: function() {
    var totalWidth = this.width * this.tilesize * ig.system.scale,
        totalHeight = this.height * this.tilesize * ig.system.scale;

    // Adjust chunk size for small layers
    this.chunkSize = Math.min( Math.max(totalWidth, totalHeight), this.chunkSize );

    var chunkCols = Math.ceil(totalWidth / this.chunkSize),
        chunkRows = Math.ceil(totalHeight / this.chunkSize);

    this.preRenderedChunks = [];
    for( var y = 0; y < chunkRows; y++ ) {
        this.preRenderedChunks[y] = [];
        for( var x = 0; x < chunkCols; x++ ) {
            var chunkWidth = (x == chunkCols-1)
                ? totalWidth - x * this.chunkSize
                : this.chunkSize;
            var chunkHeight = (y == chunkRows-1)
                ? totalHeight - y * this.chunkSize
                : this.chunkSize;

            this.preRenderedChunks[y][x] = this.preRenderChunk(
                x, y, chunkWidth, chunkHeight
            );
        }
    }
},

preRenderChunk: function( cx, cy, w, h ) {
    // Calculate tile range for this chunk
    var tw = w / this.tilesize / ig.system.scale + 1,
        th = h / this.tilesize / ig.system.scale + 1;

    var tx = Math.floor(cx * this.chunkSize / this.tilesize / ig.system.scale),
        ty = Math.floor(cy * this.chunkSize / this.tilesize / ig.system.scale);

    // Create offscreen canvas
    var chunk = ig.$new('canvas');
    chunk.width = w;
    chunk.height = h;
    chunk.retinaResolutionEnabled = false;

    var chunkContext = chunk.getContext('2d');
    ig.System.scaleMode(chunk, chunkContext);

    // Temporarily use chunk context for drawing
    var screenContext = ig.system.context;
    ig.system.context = chunkContext;

    for( var x = 0; x < tw; x++ ) {
        for( var y = 0; y < th; y++ ) {
            if( x + tx < this.width && y + ty < this.height ) {
                var tile = this.data[y+ty][x+tx];
                if( tile ) {
                    this.tiles.drawTile(
                        x * this.tilesize, y * this.tilesize,
                        tile - 1, this.tilesize
                    );
                }
            }
        }
    }

    ig.system.context = screenContext;

    // Convert to Image for Chrome 49+ compatibility
    var image = new Image();
    image.src = chunk.toDataURL();
    image.width = chunk.width;
    image.height = chunk.height;

    return image;
}
```

### 5.4 Drawing Pre-Rendered Maps

```javascript
drawPreRendered: function() {
    if( !this.preRenderedChunks ) {
        this.preRenderMapToChunks();
    }

    var dx = ig.system.getDrawPos(this.scroll.x),
        dy = ig.system.getDrawPos(this.scroll.y);

    // Handle repeat/wrap
    if( this.repeat ) {
        var w = this.width * this.tilesize * ig.system.scale;
        dx = (dx%w + w) % w;
        var h = this.height * this.tilesize * ig.system.scale;
        dy = (dy%h + h) % h;
    }

    // Calculate visible chunk range
    var minChunkX = Math.max( Math.floor(dx / this.chunkSize), 0 ),
        minChunkY = Math.max( Math.floor(dy / this.chunkSize), 0 ),
        maxChunkX = Math.ceil((dx+ig.system.realWidth) / this.chunkSize),
        maxChunkY = Math.ceil((dy+ig.system.realHeight) / this.chunkSize);

    // Draw visible chunks only
    var nudgeY = 0;
    for( var cy = minChunkY; cy < maxChunkY; cy++ ) {
        var nudgeX = 0;
        for( var cx = minChunkX; cx < maxChunkX; cx++ ) {
            var chunk = this.preRenderedChunks[cy % maxRealChunkY][cx % maxRealChunkX];

            var x = -dx + cx * this.chunkSize - nudgeX;
            var y = -dy + cy * this.chunkSize - nudgeY;

            ig.system.context.drawImage( chunk, x, y);
            ig.Image.drawCount++;
        }
    }
}
```

### 5.5 Dynamic Tile Drawing (Non-PreRendered)

```javascript
drawTiled: function() {
    var tileOffsetX = (this.scroll.x / this.tilesize).toInt(),
        tileOffsetY = (this.scroll.y / this.tilesize).toInt(),
        pxOffsetX = this.scroll.x % this.tilesize,
        pxOffsetY = this.scroll.y % this.tilesize,
        pxMinX = -pxOffsetX - this.tilesize,
        pxMinY = -pxOffsetY - this.tilesize,
        pxMaxX = ig.system.width + this.tilesize - pxOffsetX,
        pxMaxY = ig.system.height + this.tilesize - pxOffsetY;

    // Only iterate over visible tiles
    for( var mapY = -1, pxY = pxMinY; pxY < pxMaxY; mapY++, pxY += this.tilesize) {
        var tileY = mapY + tileOffsetY;

        // Handle repeat/wrap
        if( tileY >= this.height || tileY < 0 ) {
            if( !this.repeat ) { continue; }
            tileY = (tileY%this.height + this.height) % this.height;
        }

        for( var mapX = -1, pxX = pxMinX; pxX < pxMaxX; mapX++, pxX += this.tilesize ) {
            var tileX = mapX + tileOffsetX;

            if( tileX >= this.width || tileX < 0 ) {
                if( !this.repeat ) { continue; }
                tileX = (tileX%this.width + this.width) % this.width;
            }

            // Draw tile or animated tile
            if( (tile = this.data[tileY][tileX]) ) {
                if( (anim = this.anims[tile-1]) ) {
                    anim.draw( pxX, pxY );
                }
                else {
                    this.tiles.drawTile( pxX, pxY, tile-1, this.tilesize );
                }
            }
        }
    }
}
```

### 5.6 Parallax Scrolling

```javascript
// Each background map has a distance factor
// distance = 1: Same speed as camera (normal layer)
// distance = 2: Half speed of camera (distant background)
// distance = 0.5: Double speed of camera (foreground overlay)

setScreenPos: function( x, y ) {
    this.scroll.x = x / this.distance;
    this.scroll.y = y / this.distance;
}

// In game.draw():
for( var i = 0; i < this.backgroundMaps.length; i++ ) {
    var map = this.backgroundMaps[i];
    map.setScreenPos( this.screen.x, this.screen.y );
    map.draw();
}
```

---

## 6. Animation System

### 6.1 Animation Sheet (Sprite Sheet)

```javascript
ig.AnimationSheet = ig.Class.extend({
    width: 8,
    height: 8,
    image: null,

    init: function( path, width, height ) {
        this.width = width;
        this.height = height;
        this.image = new ig.Image( path );
    }
});
```

### 6.2 Animation Class

```javascript
ig.Animation = ig.Class.extend({
    sheet: null,           // Reference to AnimationSheet
    timer: null,           // ig.Timer for frame timing

    sequence: [],          // Array of tile indices [0, 1, 2, 3]
    flip: {x: false, y: false},  // Flip horizontally/vertically
    pivot: {x: 0, y: 0},   // Rotation pivot point

    frameTime: 0,          // Seconds per frame
    frame: 0,              // Current frame index in sequence
    tile: 0,               // Current tile index to draw
    stop: false,           // Stop at end or loop
    loopCount: 0,          // How many times looped
    alpha: 1,              // Transparency
    angle: 0,              // Rotation in radians

    init: function( sheet, frameTime, sequence, stop ) {
        this.sheet = sheet;
        this.pivot = {x: sheet.width/2, y: sheet.height/2 };
        this.timer = new ig.Timer();

        this.frameTime = frameTime;
        this.sequence = sequence;
        this.stop = !!stop;
        this.tile = this.sequence[0];
    },

    rewind: function() {
        this.timer.set();
        this.loopCount = 0;
        this.frame = 0;
        this.tile = this.sequence[0];
        return this;
    },

    gotoFrame: function( f ) {
        // Offset timer to jump to specific frame
        this.timer.set( this.frameTime * -f - 0.0001 );
        this.update();
    },

    gotoRandomFrame: function() {
        this.gotoFrame( Math.floor(Math.random() * this.sequence.length) );
    },

    update: function() {
        var frameTotal = Math.floor(this.timer.delta() / this.frameTime);
        this.loopCount = Math.floor(frameTotal / this.sequence.length);

        if( this.stop && this.loopCount > 0 ) {
            this.frame = this.sequence.length - 1;
        }
        else {
            this.frame = frameTotal % this.sequence.length;
        }
        this.tile = this.sequence[ this.frame ];
    },

    draw: function( targetX, targetY ) {
        // Culling check
        var bbsize = Math.max(this.sheet.width, this.sheet.height);
        if(
           targetX > ig.system.width || targetY > ig.system.height ||
           targetX + bbsize < 0 || targetY + bbsize < 0
        ) {
            return;  // Off screen
        }

        if( this.alpha != 1) {
            ig.system.context.globalAlpha = this.alpha;
        }

        if( this.angle == 0 ) {
            // No rotation - direct draw
            this.sheet.image.drawTile(
                targetX, targetY,
                this.tile, this.sheet.width, this.sheet.height,
                this.flip.x, this.flip.y
            );
        }
        else {
            // With rotation
            ig.system.context.save();
            ig.system.context.translate(
                ig.system.getDrawPos(targetX + this.pivot.x),
                ig.system.getDrawPos(targetY + this.pivot.y)
            );
            ig.system.context.rotate( this.angle );
            this.sheet.image.drawTile(
                -this.pivot.x, -this.pivot.y,
                this.tile, this.sheet.width, this.sheet.height,
                this.flip.x, this.flip.y
            );
            ig.system.context.restore();
        }

        if( this.alpha != 1) {
            ig.system.context.globalAlpha = 1;
        }
    }
});
```

### 6.3 Entity Animation Integration

```javascript
ig.Entity = ig.Class.extend({
    animSheet: null,  // Shared sprite sheet
    anims: {},        // Named animations: {idle: Animation, run: Animation}
    currentAnim: null,

    addAnim: function( name, frameTime, sequence, stop ) {
        if( !this.animSheet ) {
            throw( 'No animSheet to add the animation '+name+' to.' );
        }
        var a = new ig.Animation( this.animSheet, frameTime, sequence, stop );
        this.anims[name] = a;
        if( !this.currentAnim ) {
            this.currentAnim = a;
        }
        return a;
    },

    update: function() {
        // ... physics update ...

        if( this.currentAnim ) {
            this.currentAnim.update();
        }
    },

    draw: function() {
        if( this.currentAnim ) {
            this.currentAnim.draw(
                this.pos.x - this.offset.x - ig.game._rscreen.x,
                this.pos.y - this.offset.y - ig.game._rscreen.y
            );
        }
    }
});
```

### 6.4 Animation Usage Example

```javascript
EntityPlayer = ig.Entity.extend({
    animSheet: new ig.AnimationSheet('media/player.png', 32, 32),

    init: function(x, y, settings) {
        this.parent(x, y, settings);

        // Define animations
        this.addAnim('idle', 0.2, [0]);
        this.addAnim('run', 0.1, [0, 1, 2, 3]);
        this.addAnim('jump', 0.2, [4], true);  // Don't loop
    },

    update: function() {
        if( this.vel.y < 0 ) {
            this.currentAnim = this.anims.jump;
        }
        else if( this.vel.x !== 0 ) {
            this.currentAnim = this.anims.run;
            this.currentAnim.flip.x = (this.vel.x < 0);
        }
        else {
            this.currentAnim = this.anims.idle;
        }

        this.parent();
    }
});
```

---

## 7. Entity System

### 7.1 Entity Base Class

```javascript
ig.Entity = ig.Class.extend({
    // Identification
    id: 0,
    settings: {},

    // Size and position
    size: {x: 16, y: 16},
    offset: {x: 0, y: 0},

    // Physics state
    pos: {x: 0, y: 0},
    last: {x: 0, y: 0},    // Previous position
    vel: {x: 0, y: 0},     // Velocity
    accel: {x: 0, y: 0},   // Acceleration
    friction: {x: 0, y: 0},// Friction
    maxVel: {x: 100, y: 100},

    // Rendering
    zIndex: 0,

    // Physics options
    gravityFactor: 1,
    standing: false,
    bounciness: 0,
    minBounceVelocity: 40,

    // Animation
    anims: {},
    animSheet: null,
    currentAnim: null,

    // Gameplay
    health: 10,

    // Collision configuration
    type: 0,           // Entity type for checks (TYPE.NONE, A, B, BOTH)
    checkAgainst: 0,   // Which types to check against
    collides: 0,       // Collision behavior (COLLIDES.NEVER, LITE, PASSIVE, ACTIVE, FIXED)

    _killed: false,    // Internal: marked for removal

    // Slope standing angle range
    slopeStanding: {min: (44).toRad(), max: (136).toRad() },

    init: function( x, y, settings ) {
        this.id = ++ig.Entity._lastId;
        this.pos.x = this.last.x = x;
        this.pos.y = this.last.y = y;
        ig.merge( this, settings );
    }
});
```

### 7.2 Entity Lifecycle

```
┌─────────────────────────────────────────────────────────────────┐
│                      ENTITY LIFECYCLE                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  SPAWN                                                           │
│  ├─▶ game.spawnEntity(type, x, y, settings)                    │
│  ├─▶ new EntityClass(x, y, settings)                            │
│  ├─▶ staticInstantiate() - check pool (optional)                │
│  ├─▶ init(x, y, settings) - constructor                         │
│  ├─▶ Add to game.entities[]                                     │
│  └─▶ Add to game.namedEntities[name] (if has name)             │
│                                                                  │
│  READY (after level load)                                       │
│  └─▶ entity.ready() - post-initialization                       │
│                                                                  │
│  UPDATE (every frame)                                           │
│  ├─▶ Save last position: last = pos                             │
│  ├─▶ Apply gravity: vel.y += gravity * tick * gravityFactor     │
│  ├─▶ Apply acceleration/friction                                │
│  ├─▶ Movement & collision: collisionMap.trace()                 │
│  ├─▶ handleMovementTrace() - resolve collisions                 │
│  └─▶ Update currentAnim                                         │
│                                                                  │
│  CHECK (spatial hash)                                           │
│  ├─▶ Insert into spatial hash grid                              │
│  ├─▶ Check against entities in same cells                       │
│  ├─▶ entity.check(other) - overlap detection                    │
│  └─▶ entity.collideWith(other, axis) - collision response       │
│                                                                  │
│  DRAW                                                           │
│  └─▶ currentAnim.draw(pos - screen)                             │
│                                                                  │
│  KILL                                                           │
│  ├─▶ entity.kill()                                              │
│  ├─▶ Set _killed = true                                         │
│  ├─▶ Set type = TYPE.NONE                                       │
│  ├─▶ Set checkAgainst = TYPE.NONE                               │
│  ├─▶ Set collides = COLLIDES.NEVER                              │
│  ├─▶ Add to game._deferredKill[]                                │
│  └─▶ After update: erase() and remove from entities[]           │
│                                                                  │
│  ERASE (cleanup)                                                │
│  └─▶ entity.erase() - custom cleanup (override)                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 7.3 Entity Update and Physics

```javascript
ig.Entity.prototype.update = function() {
    // Save previous position
    this.last.x = this.pos.x;
    this.last.y = this.pos.y;

    // Apply gravity
    this.vel.y += ig.game.gravity * ig.system.tick * this.gravityFactor;

    // Calculate new velocity
    this.vel.x = this.getNewVelocity(
        this.vel.x, this.accel.x, this.friction.x, this.maxVel.x
    );
    this.vel.y = this.getNewVelocity(
        this.vel.y, this.accel.y, this.friction.y, this.maxVel.y
    );

    // Movement & collision
    var mx = this.vel.x * ig.system.tick;
    var my = this.vel.y * ig.system.tick;
    var res = ig.game.collisionMap.trace(
        this.pos.x, this.pos.y, mx, my, this.size.x, this.size.y
    );
    this.handleMovementTrace( res );

    // Update animation
    if( this.currentAnim ) {
        this.currentAnim.update();
    }
};

ig.Entity.prototype.getNewVelocity = function( vel, accel, friction, max ) {
    if( accel ) {
        // Acceleration with max clamp
        return ( vel + accel * ig.system.tick ).limit( -max, max );
    }
    else if( friction ) {
        // Friction deceleration
        var delta = friction * ig.system.tick;

        if( vel - delta > 0) {
            return vel - delta;
        }
        else if( vel + delta < 0 ) {
            return vel + delta;
        }
        else {
            return 0;  // Stopped
        }
    }
    return vel.limit( -max, max );
};
```

### 7.4 Movement Trace and Collision Response

```javascript
ig.Entity.prototype.handleMovementTrace = function( res ) {
    this.standing = false;

    // Vertical collision (floor/ceiling)
    if( res.collision.y ) {
        if( this.bounciness > 0 && Math.abs(this.vel.y) > this.minBounceVelocity ) {
            // Bounce
            this.vel.y *= -this.bounciness;
        }
        else {
            // Land/stop
            if( this.vel.y > 0 ) {
                this.standing = true;
            }
            this.vel.y = 0;
        }
    }

    // Horizontal collision (walls)
    if( res.collision.x ) {
        if( this.bounciness > 0 && Math.abs(this.vel.x) > this.minBounceVelocity ) {
            this.vel.x *= -this.bounciness;
        }
        else {
            this.vel.x = 0;
        }
    }

    // Slope collision
    if( res.collision.slope ) {
        var s = res.collision.slope;

        if( this.bounciness > 0 ) {
            // Reflect velocity off slope normal
            var proj = this.vel.x * s.nx + this.vel.y * s.ny;
            this.vel.x = (this.vel.x - s.nx * proj * 2) * this.bounciness;
            this.vel.y = (this.vel.y - s.ny * proj * 2) * this.bounciness;
        }
        else {
            // Slide along slope
            var lengthSquared = s.x * s.x + s.y * s.y;
            var dot = (this.vel.x * s.x + this.vel.y * s.y)/lengthSquared;

            this.vel.x = s.x * dot;
            this.vel.y = s.y * dot;

            // Check if standing on slope
            var angle = Math.atan2( s.x, s.y );
            if( angle > this.slopeStanding.min && angle < this.slopeStanding.max ) {
                this.standing = true;
            }
        }
    }

    // Apply resolved position
    this.pos = res.pos;
};
```

### 7.5 Entity-Entity Collision Detection

```javascript
// Spatial hash grid for broadphase
ig.Game.prototype.checkEntities = function() {
    var hash = {};
    var cellSize = this.cellSize;  // Default 64

    for( var e = 0; e < this.entities.length; e++ ) {
        var entity = this.entities[e];

        // Skip entities that don't participate
        if(
            entity.type == ig.Entity.TYPE.NONE &&
            entity.checkAgainst == ig.Entity.TYPE.NONE &&
            entity.collides == ig.Entity.COLLIDES.NEVER
        ) {
            continue;
        }

        var checked = {};
        var xmin = Math.floor( entity.pos.x/cellSize ),
            ymin = Math.floor( entity.pos.y/cellSize ),
            xmax = Math.floor( (entity.pos.x+entity.size.x)/cellSize ) + 1,
            ymax = Math.floor( (entity.pos.y+entity.size.y)/cellSize ) + 1;

        // Insert into each cell the entity overlaps
        for( var x = xmin; x < xmax; x++ ) {
            for( var y = ymin; y < ymax; y++ ) {

                if( !hash[x] ) {
                    hash[x] = {};
                    hash[x][y] = [entity];
                }
                else if( !hash[x][y] ) {
                    hash[x][y] = [entity];
                }
                else {
                    // Check against each entity in this cell
                    var cell = hash[x][y];
                    for( var c = 0; c < cell.length; c++ ) {

                        // AABB overlap test and not already checked
                        if( entity.touches(cell[c]) && !checked[cell[c].id] ) {
                            checked[cell[c].id] = true;
                            ig.Entity.checkPair( entity, cell[c] );
                        }
                    }
                    cell.push(entity);
                }
            }
        }
    }
};

// AABB overlap test
ig.Entity.prototype.touches = function( other ) {
    return !(
        this.pos.x >= other.pos.x + other.size.x ||
        this.pos.x + this.size.x <= other.pos.x ||
        this.pos.y >= other.pos.y + other.size.y ||
        this.pos.y + this.size.y <= other.pos.y
    );
};
```

### 7.6 Collision Pair Resolution

```javascript
ig.Entity.checkPair = function( a, b ) {
    // Type-based check (for triggers, etc.)
    if( a.checkAgainst & b.type ) {
        a.check( b );
    }

    if( b.checkAgainst & a.type ) {
        b.check( a );
    }

    // Physical collision response
    // Sum must be > ACTIVE (4) for collision
    // NEVER=0, LITE=1, PASSIVE=2, ACTIVE=4, FIXED=8
    if(
        a.collides && b.collides &&
        a.collides + b.collides > ig.Entity.COLLIDES.ACTIVE
    ) {
        ig.Entity.solveCollision( a, b );
    }
};

ig.Entity.solveCollision = function( a, b ) {
    // Determine weak entity (won't move)
    var weak = null;
    if(
        a.collides == ig.Entity.COLLIDES.LITE ||
        b.collides == ig.Entity.COLLIDES.FIXED
    ) {
        weak = a;
    }
    else if(
        b.collides == ig.Entity.COLLIDES.LITE ||
        a.collides == ig.Entity.COLLIDES.FIXED
    ) {
        weak = b;
    }

    // Determine collision axis based on previous positions
    // Horizontal overlap in last frame = vertical collision
    if(
        a.last.x + a.size.x > b.last.x &&
        a.last.x < b.last.x + b.size.x
    ) {
        // Vertical collision
        if( a.last.y < b.last.y ) {
            ig.Entity.seperateOnYAxis( a, b, weak );
        }
        else {
            ig.Entity.seperateOnYAxis( b, a, weak );
        }
        a.collideWith( b, 'y' );
        b.collideWith( a, 'y' );
    }
    else if(
        a.last.y + a.size.y > b.last.y &&
        a.last.y < b.last.y + b.size.y
    ) {
        // Horizontal collision
        if( a.last.x < b.last.x ) {
            ig.Entity.seperateOnXAxis( a, b, weak );
        }
        else {
            ig.Entity.seperateOnXAxis( b, a, weak );
        }
        a.collideWith( b, 'x' );
        b.collideWith( a, 'x' );
    }
};
```

### 7.7 Axis Separation

```javascript
ig.Entity.seperateOnXAxis = function( left, right, weak ) {
    var nudge = (left.pos.x + left.size.x - right.pos.x);

    if( weak ) {
        // Only move weak entity
        var strong = left === weak ? right : left;
        weak.vel.x = -weak.vel.x * weak.bounciness + strong.vel.x;

        var resWeak = ig.game.collisionMap.trace(
            weak.pos.x, weak.pos.y,
            weak == left ? -nudge : nudge, 0,
            weak.size.x, weak.size.y
        );
        weak.pos.x = resWeak.pos.x;
    }
    else {
        // Both move
        var v2 = (left.vel.x - right.vel.x)/2;
        left.vel.x = -v2;
        right.vel.x = v2;

        var resLeft = ig.game.collisionMap.trace(
            left.pos.x, left.pos.y, -nudge/2, 0, left.size.x, left.size.y
        );
        left.pos.x = Math.floor(resLeft.pos.x);

        var resRight = ig.game.collisionMap.trace(
            right.pos.x, right.pos.y, nudge/2, 0, right.size.x, right.size.y
        );
        right.pos.x = Math.ceil(resRight.pos.x);
    }
};

ig.Entity.seperateOnYAxis = function( top, bottom, weak ) {
    var nudge = (top.pos.y + top.size.y - bottom.pos.y);

    if( weak ) {
        var strong = top === weak ? bottom : top;
        weak.vel.y = -weak.vel.y * weak.bounciness + strong.vel.y;

        // Check if riding platform
        var nudgeX = 0;
        if( weak == top && Math.abs(weak.vel.y - strong.vel.y) < weak.minBounceVelocity ) {
            weak.standing = true;
            nudgeX = strong.vel.x * ig.system.tick;
        }

        var resWeak = ig.game.collisionMap.trace(
            weak.pos.x, weak.pos.y,
            nudgeX, weak == top ? -nudge : nudge,
            weak.size.x, weak.size.y
        );
        weak.pos.y = resWeak.pos.y;
        weak.pos.x = resWeak.pos.x;
    }
    else if( ig.game.gravity && (bottom.standing || top.vel.y > 0) ) {
        // Bottom entity is standing - only bounce top
        var resTop = ig.game.collisionMap.trace(
            top.pos.x, top.pos.y,
            0, -(top.pos.y + top.size.y - bottom.pos.y),
            top.size.x, top.size.y
        );
        top.pos.y = resTop.pos.y;

        if( top.bounciness > 0 && top.vel.y > top.minBounceVelocity ) {
            top.vel.y *= -top.bounciness;
        }
        else {
            top.standing = true;
            top.vel.y = 0;
        }
    }
    else {
        // Both move
        var v2 = (top.vel.y - bottom.vel.y)/2;
        top.vel.y = -v2;
        bottom.vel.y = v2;

        var resTop = ig.game.collisionMap.trace(
            top.pos.x, top.pos.y,
            bottom.vel.x * ig.system.tick, -nudge/2,
            top.size.x, top.size.y
        );
        top.pos.y = resTop.pos.y;

        var resBottom = ig.game.collisionMap.trace(
            bottom.pos.x, bottom.pos.y, 0, nudge/2,
            bottom.size.x, bottom.size.y
        );
        bottom.pos.y = resBottom.pos.y;
    }
};
```

### 7.8 Entity Pooling System

```javascript
ig.EntityPool = {
    pools: {},

    // Mixin for entity classes
    mixin: {
        staticInstantiate: function( x, y, settings ) {
            return ig.EntityPool.getFromPool( this.classId, x, y, settings );
        },

        erase: function() {
            ig.EntityPool.putInPool( this );
        }
    },

    // Enable pooling for a class
    enableFor: function( Class ) {
        Class.inject(this.mixin);
    },

    // Get entity from pool
    getFromPool: function( classId, x, y, settings ) {
        var pool = this.pools[classId];
        if( !pool || !pool.length ) { return null; }

        var instance = pool.pop();
        instance.reset(x, y, settings);
        return instance;
    },

    // Return entity to pool
    putInPool: function( instance ) {
        if( !this.pools[instance.classId] ) {
            this.pools[instance.classId] = [instance];
        }
        else {
            this.pools[instance.classId].push(instance);
        }
    },

    drainAllPools: function() {
        this.pools = {};
    }
};

// Entity reset for pooling
ig.Entity.prototype.reset = function( x, y, settings ) {
    var proto = this.constructor.prototype;
    this.pos.x = x;
    this.pos.y = y;
    this.last.x = x;
    this.last.y = y;
    this.vel.x = proto.vel.x;
    this.vel.y = proto.vel.y;
    this.accel.x = proto.accel.x;
    this.accel.y = proto.accel.y;
    this.health = proto.health;
    this._killed = proto._killed;
    this.standing = proto.standing;

    this.type = proto.type;
    this.checkAgainst = proto.checkAgainst;
    this.collides = proto.collides;

    ig.merge( this, settings );
};
```

### 7.9 Entity Sorting

```javascript
ig.Game.SORT = {
    // By z-index property
    Z_INDEX: function( a, b ){ return a.zIndex - b.zIndex; },

    // By rightmost X position (for side-scrollers)
    POS_X: function( a, b ){ return (a.pos.x+a.size.x) - (b.pos.x+b.size.x); },

    // By bottom Y position (depth sorting)
    POS_Y: function( a, b ){ return (a.pos.y+a.size.y) - (b.pos.y+b.size.y); }
};

ig.Game.prototype.sortEntities = function() {
    this.entities.sort( this.sortBy );
};
```

---

## 8. Input System

### 8.1 Key Codes

```javascript
ig.KEY = {
    // Mouse
    'MOUSE1': -1,
    'MOUSE2': -3,
    'MWHEEL_UP': -4,
    'MWHEEL_DOWN': -5,

    // Keyboard
    'BACKSPACE': 8, 'TAB': 9, 'ENTER': 13, 'PAUSE': 19,
    'CAPS': 20, 'ESC': 27, 'SPACE': 32,
    'PAGE_UP': 33, 'PAGE_DOWN': 34, 'END': 35, 'HOME': 36,
    'LEFT_ARROW': 37, 'UP_ARROW': 38, 'RIGHT_ARROW': 39, 'DOWN_ARROW': 40,
    'INSERT': 45, 'DELETE': 46,
    '_0': 48, '_1': 49, /* ... */ '_9': 57,
    'A': 65, 'B': 66, /* ... */ 'Z': 90,
    'NUMPAD_0': 96, /* ... */ 'NUMPAD_9': 105,
    'F1': 112, /* ... */ 'F12': 123,
    'SHIFT': 16, 'CTRL': 17, 'ALT': 18,
    'PLUS': 187, 'COMMA': 188, 'MINUS': 189, 'PERIOD': 190
};
```

### 8.2 Input Class

```javascript
ig.Input = ig.Class.extend({
    bindings: {},      // keyCode -> action name
    actions: {},       // action -> isHeld
    presses: {},       // action -> wasPressedThisFrame
    locks: {},         // action -> isLocked (for press detection)
    delayedKeyup: {},  // action -> wasReleasedThisFrame

    isUsingMouse: false,
    isUsingKeyboard: false,
    isUsingAccelerometer: false,

    mouse: {x: 0, y: 0},
    accel: {x: 0, y: 0, z: 0},

    // Initialize mouse event listeners
    initMouse: function() {
        if( this.isUsingMouse ) { return; }
        this.isUsingMouse = true;

        ig.system.canvas.addEventListener('wheel', this.mousewheel.bind(this), false );
        ig.system.canvas.addEventListener('contextmenu', this.contextmenu.bind(this), false );
        ig.system.canvas.addEventListener('mousedown', this.keydown.bind(this), false );
        ig.system.canvas.addEventListener('mouseup', this.keyup.bind(this), false );
        ig.system.canvas.addEventListener('mousemove', this.mousemove.bind(this), false );

        // Touch support
        if( ig.ua.touchDevice ) {
            ig.system.canvas.addEventListener('touchstart', this.keydown.bind(this), false );
            ig.system.canvas.addEventListener('touchend', this.keyup.bind(this), false );
            ig.system.canvas.addEventListener('touchcancel', this.keyup.bind(this), false );
            ig.system.canvas.addEventListener('touchmove', this.mousemove.bind(this), false );

            // MS Pointer Events
            ig.system.canvas.addEventListener('MSPointerDown', this.keydown.bind(this), false );
            ig.system.canvas.addEventListener('MSPointerUp', this.keyup.bind(this), false );
            ig.system.canvas.addEventListener('MSPointerMove', this.mousemove.bind(this), false );
            ig.system.canvas.style.msTouchAction = 'none';
        }
    },

    // Initialize keyboard event listeners
    initKeyboard: function() {
        if( this.isUsingKeyboard ) { return; }
        this.isUsingKeyboard = true;
        window.addEventListener('keydown', this.keydown.bind(this), false );
        window.addEventListener('keyup', this.keyup.bind(this), false );
    },

    // Initialize accelerometer
    initAccelerometer: function() {
        if( this.isUsingAccelerometer ) { return; }
        this.isUsingAccelerometer = true;
        window.addEventListener('devicemotion', this.devicemotion.bind(this), false );
    }
});
```

### 8.3 Event Handlers

```javascript
ig.Input.prototype.mousewheel = function( event ) {
    var code = event.deltaY < 0 ? ig.KEY.MWHEEL_UP : ig.KEY.MWHEEL_DOWN;
    var action = this.bindings[code];
    if( action ) {
        this.actions[action] = true;
        this.presses[action] = true;
        this.delayedKeyup[action] = true;
        event.stopPropagation();
        event.preventDefault();
    }
};

ig.Input.prototype.mousemove = function( event ) {
    var internalWidth = ig.system.canvas.offsetWidth || ig.system.realWidth;
    var scale = ig.system.scale * (internalWidth / ig.system.realWidth);

    var pos = {left: 0, top: 0};
    if( ig.system.canvas.getBoundingClientRect ) {
        pos = ig.system.canvas.getBoundingClientRect();
    }

    // Handle both mouse and touch
    var ev = event.touches ? event.touches[0] : event;
    this.mouse.x = (ev.clientX - pos.left) / scale;
    this.mouse.y = (ev.clientY - pos.top) / scale;
};

ig.Input.prototype.keydown = function( event ) {
    // Ignore input in text fields
    var tag = event.target.tagName;
    if( tag == 'INPUT' || tag == 'TEXTAREA' ) { return; }

    var code = event.type == 'keydown'
        ? event.keyCode
        : (event.button == 2 ? ig.KEY.MOUSE2 : ig.KEY.MOUSE1);

    // Focus window on mouse click (for iframes)
    if( code < 0 && !ig.ua.mobile ) {
        window.focus();
    }

    // Update mouse position on touch/click
    if( event.type == 'touchstart' || event.type == 'mousedown' ) {
        this.mousemove( event );
    }

    var action = this.bindings[code];
    if( action ) {
        this.actions[action] = true;
        // Only register press if not locked
        if( !this.locks[action] ) {
            this.presses[action] = true;
            this.locks[action] = true;
        }
        event.preventDefault();
    }
};

ig.Input.prototype.keyup = function( event ) {
    var tag = event.target.tagName;
    if( tag == 'INPUT' || tag == 'TEXTAREA' ) { return; }

    var code = event.type == 'keyup'
        ? event.keyCode
        : (event.button == 2 ? ig.KEY.MOUSE2 : ig.KEY.MOUSE1);

    var action = this.bindings[code];
    if( action ) {
        this.delayedKeyup[action] = true;
        event.preventDefault();
    }
};
```

### 8.4 Binding and State

```javascript
ig.Input.prototype.bind = function( key, action ) {
    if( key < 0 ) { this.initMouse(); }
    else if( key > 0 ) { this.initKeyboard(); }
    this.bindings[key] = action;
};

ig.Input.prototype.unbind = function( key ) {
    var action = this.bindings[key];
    this.delayedKeyup[action] = true;
    this.bindings[key] = null;
};

ig.Input.prototype.unbindAll = function() {
    this.bindings = {};
    this.actions = {};
    this.presses = {};
    this.locks = {};
    this.delayedKeyup = {};
};

// Check if action is currently held
ig.Input.prototype.state = function( action ) {
    return this.actions[action];
};

// Check if action was pressed this frame
ig.Input.prototype.pressed = function( action ) {
    return this.presses[action];
};

// Check if action was released this frame
ig.Input.prototype.released = function( action ) {
    return !!this.delayedKeyup[action];
};

// Clear frame-based states (called each frame)
ig.Input.prototype.clearPressed = function() {
    for( var action in this.delayedKeyup ) {
        this.actions[action] = false;
        this.locks[action] = false;
    }
    this.delayedKeyup = {};
    this.presses = {};
};
```

### 8.5 Touch Bindings

```javascript
ig.Input.prototype.bindTouch = function( selector, action ) {
    var element = ig.$( selector );

    var that = this;
    element.addEventListener('touchstart', function(ev) {
        that.touchStart( ev, action );
    }, false);
    element.addEventListener('touchend', function(ev) {
        that.touchEnd( ev, action );
    }, false);
};

ig.Input.prototype.touchStart = function( event, action ) {
    this.actions[action] = true;
    this.presses[action] = true;
    event.stopPropagation();
    event.preventDefault();
    return false;
};

ig.Input.prototype.touchEnd = function( event, action ) {
    this.delayedKeyup[action] = true;
    event.stopPropagation();
    event.preventDefault();
    return false;
};
```

### 8.6 Input Usage Example

```javascript
MyGame = ig.Game.extend({
    init: function() {
        // Bind inputs
        ig.input.bind( ig.KEY.LEFT_ARROW, 'left' );
        ig.input.bind( ig.KEY.RIGHT_ARROW, 'right' );
        ig.input.bind( ig.KEY.SPACE, 'jump' );
        ig.input.bind( ig.KEY.MOUSE1, 'shoot' );

        // Or touch bindings
        ig.input.bindTouch( '#jumpButton', 'jump' );
    },

    update: function() {
        // Check held
        if( ig.input.state('left') ) {
            player.vel.x = -player.speed;
        }

        // Check pressed (single frame)
        if( ig.input.pressed('jump') ) {
            player.vel.y = -player.jumpForce;
        }

        // Check released
        if( ig.input.released('shoot') ) {
            this.shoot();
        }
    }
});
```

---

## 9. Audio System

### 9.1 Sound Manager

```javascript
ig.SoundManager = ig.Class.extend({
    clips: {},         // path -> [Audio] or WebAudioSource
    volume: 1,
    format: null,      // Detected format (MP3, OGG, etc.)
    audioContext: null,// WebAudio context

    init: function() {
        // Check for Audio support
        if( !ig.Sound.enabled || !window.Audio ) {
            ig.Sound.enabled = false;
            return;
        }

        // Detect best format
        var probe = new Audio();
        for( var i = 0; i < ig.Sound.use.length; i++ ) {
            var format = ig.Sound.use[i];
            if( probe.canPlayType(format.mime) ) {
                this.format = format;
                break;
            }
        }

        if( !this.format ) {
            ig.Sound.enabled = false;
            return;
        }

        // Create WebAudio context if available
        if( ig.Sound.enabled && ig.Sound.useWebAudio ) {
            this.audioContext = new AudioContext();
            this.boundWebAudioUnlock = this.unlockWebAudio.bind(this);

            // Unlock on user interaction (iOS requirement)
            ig.system.canvas.addEventListener('touchstart', this.boundWebAudioUnlock, false);
            ig.system.canvas.addEventListener('mousedown', this.boundWebAudioUnlock, false);
        }
    },

    unlockWebAudio: function() {
        ig.system.canvas.removeEventListener('touchstart', this.boundWebAudioUnlock, false);
        ig.system.canvas.removeEventListener('mousedown', this.boundWebAudioUnlock, false);

        // Create and play empty buffer to unlock WebAudio
        var buffer = this.audioContext.createBuffer(1, 1, 22050);
        var source = this.audioContext.createBufferSource();
        source.buffer = buffer;
        source.connect(this.audioContext.destination);
        source.start(0);
    }
});
```

### 9.2 Sound Loading

```javascript
ig.SoundManager.prototype.load = function( path, multiChannel, loadCallback ) {
    if( multiChannel && ig.Sound.useWebAudio ) {
        return this.loadWebAudio( path, multiChannel, loadCallback );
    }
    else {
        return this.loadHTML5Audio( path, multiChannel, loadCallback );
    }
};

ig.SoundManager.prototype.loadWebAudio = function( path, multiChannel, loadCallback ) {
    var realPath = ig.prefix + path.replace(/[^\.]+$/, this.format.ext) + ig.nocache;

    // Already loaded?
    if( this.clips[path] ) {
        return this.clips[path];
    }

    var audioSource = new ig.Sound.WebAudioSource();
    this.clips[path] = audioSource;

    var request = new XMLHttpRequest();
    request.open('GET', realPath, true);
    request.responseType = 'arraybuffer';

    var that = this;
    request.onload = function(ev) {
        that.audioContext.decodeAudioData(request.response,
            function(buffer) {
                audioSource.buffer = buffer;
                if( loadCallback ) {
                    loadCallback( path, true, ev );
                }
            },
            function(ev) {
                if( loadCallback ) {
                    loadCallback( path, false, ev );
                }
            }
        );
    };
    request.send();

    return audioSource;
};

ig.SoundManager.prototype.loadHTML5Audio = function( path, multiChannel, loadCallback ) {
    var realPath = ig.prefix + path.replace(/[^\.]+$/, this.format.ext) + ig.nocache;

    // Already loaded?
    if( this.clips[path] ) {
        if( this.clips[path] instanceof ig.Sound.WebAudioSource ) {
            return this.clips[path];  // Loaded as WebAudio
        }

        // Add more channels if needed
        if( multiChannel && this.clips[path].length < ig.Sound.channels ) {
            for( var i = this.clips[path].length; i < ig.Sound.channels; i++ ) {
                var a = new Audio( realPath );
                a.load();
                this.clips[path].push( a );
            }
        }
        return this.clips[path][0];
    }

    var clip = new Audio( realPath );

    if( loadCallback ) {
        if( ig.ua.mobile ) {
            // Mobile browsers don't preload, fake success
            setTimeout(function(){
                loadCallback( path, true, null );
            }, 0);
        }
        else {
            clip.addEventListener( 'canplaythrough', function cb(ev){
                clip.removeEventListener('canplaythrough', cb, false);
                loadCallback( path, true, ev );
            }, false );
        }
    }

    clip.preload = 'auto';
    clip.load();

    this.clips[path] = [clip];
    if( multiChannel ) {
        for( var i = 1; i < ig.Sound.channels; i++ ) {
            var a = new Audio(realPath);
            a.load();
            this.clips[path].push( a );
        }
    }

    return clip;
};
```

### 9.3 Sound Playback

```javascript
ig.SoundManager.prototype.get = function( path ) {
    var channels = this.clips[path];

    // WebAudio source
    if( channels && channels instanceof ig.Sound.WebAudioSource ) {
        return channels;
    }

    // HTML5 Audio - find available channel
    for( var i = 0, clip; clip = channels[i++]; ) {
        if( clip.paused || clip.ended ) {
            if( clip.ended ) {
                clip.currentTime = 0;
            }
            return clip;
        }
    }

    // All channels busy - rewind first
    channels[0].pause();
    channels[0].currentTime = 0;
    return channels[0];
};
```

### 9.4 Sound Class

```javascript
ig.Sound = ig.Class.extend({
    path: '',
    volume: 1,
    currentClip: null,
    multiChannel: true,
    _loop: false,

    init: function( path, multiChannel ) {
        this.path = path;
        this.multiChannel = (multiChannel !== false);
        this.load();
    },

    load: function( loadCallback ) {
        if( !ig.Sound.enabled ) {
            if( loadCallback ) { loadCallback( this.path, true ); }
            return;
        }

        if( ig.ready ) {
            ig.soundManager.load( this.path, this.multiChannel, loadCallback );
        }
        else {
            ig.addResource( this );
        }
    },

    play: function() {
        if( !ig.Sound.enabled ) { return; }

        this.currentClip = ig.soundManager.get( this.path );
        this.currentClip.loop = this._loop;
        this.currentClip.volume = ig.soundManager.volume * this.volume;
        this.currentClip.play();
    },

    stop: function() {
        if( this.currentClip ) {
            this.currentClip.pause();
            this.currentClip.currentTime = 0;
        }
    },

    setLooping: function( loop ) {
        this._loop = loop;
        if( this.currentClip ) {
            this.currentClip.loop = loop;
        }
    }
});
```

### 9.5 WebAudio Source

```javascript
ig.Sound.WebAudioSource = ig.Class.extend({
    sources: [],  // Active source nodes
    gain: null,   // Gain node for volume
    buffer: null, // Decoded audio buffer
    _loop: false,

    init: function() {
        this.gain = ig.soundManager.audioContext.createGain();
        this.gain.connect(ig.soundManager.audioContext.destination);
    },

    play: function() {
        if( !this.buffer ) { return; }

        var source = ig.soundManager.audioContext.createBufferSource();
        source.buffer = this.buffer;
        source.connect(this.gain);
        source.loop = this._loop;

        var that = this;
        this.sources.push(source);
        source.onended = function(){ that.sources.erase(source); };

        source.start(0);
    },

    pause: function() {
        for( var i = 0; i < this.sources.length; i++ ) {
            try{ this.sources[i].stop(); } catch(err){}
        }
    },

    getVolume: function() {
        return this.gain.gain.value;
    },

    setVolume: function( volume ) {
        this.gain.gain.value = volume;
    }
});
```

### 9.6 Music System

```javascript
ig.Music = ig.Class.extend({
    tracks: [],
    namedTracks: {},
    currentTrack: null,
    currentIndex: 0,
    random: false,

    _volume: 1,
    _loop: false,
    _fadeInterval: 0,
    _fadeTimer: null,
    _endedCallbackBound: null,

    init: function() {
        this._endedCallbackBound = this._endedCallback.bind(this);
    },

    add: function( music, name ) {
        if( !ig.Sound.enabled ) { return; }

        var path = music instanceof ig.Sound ? music.path : music;
        var track = ig.soundManager.load(path, false);  // Single channel for music

        track.loop = this._loop;
        track.volume = this._volume;
        track.addEventListener( 'ended', this._endedCallbackBound, false );
        this.tracks.push( track );

        if( name ) {
            this.namedTracks[name] = track;
        }

        if( !this.currentTrack ) {
            this.currentTrack = track;
        }
    },

    play: function( name ) {
        if( name && this.namedTracks[name] ) {
            var newTrack = this.namedTracks[name];
            if( newTrack != this.currentTrack ) {
                this.stop();
                this.currentTrack = newTrack;
            }
        }
        else if( !this.currentTrack ) {
            return;
        }
        this.currentTrack.play();
    },

    stop: function() {
        if( !this.currentTrack ) { return; }
        this.currentTrack.pause();
        this.currentTrack.currentTime = 0;
    },

    pause: function() {
        if( !this.currentTrack ) { return; }
        this.currentTrack.pause();
    },

    next: function() {
        if( !this.tracks.length ) { return; }

        this.stop();
        this.currentIndex = this.random
            ? Math.floor(Math.random() * this.tracks.length)
            : (this.currentIndex + 1) % this.tracks.length;
        this.currentTrack = this.tracks[this.currentIndex];
        this.play();
    },

    _endedCallback: function() {
        if( this._loop ) {
            this.play();
        }
        else {
            this.next();
        }
    },

    fadeOut: function( time ) {
        if( !this.currentTrack ) { return; }

        clearInterval( this._fadeInterval );
        this._fadeTimer = new ig.Timer( time );
        this._fadeInterval = setInterval( this._fadeStep.bind(this), 50 );
    },

    _fadeStep: function() {
        var v = this._fadeTimer.delta()
            .map(-this._fadeTimer.target, 0, 1, 0)
            .limit( 0, 1 )
            * this._volume;

        if( v <= 0.01 ) {
            this.stop();
            this.currentTrack.volume = this._volume;
            clearInterval( this._fadeInterval );
        }
        else {
            this.currentTrack.volume = v;
        }
    },

    getVolume: function() { return this._volume; },
    setVolume: function( v ) {
        this._volume = v.limit(0,1);
        for( var i in this.tracks ) {
            this.tracks[i].volume = this._volume;
        }
    },

    getLooping: function() { return this._loop; },
    setLooping: function( l ) {
        this._loop = l;
        for( var i in this.tracks ) {
            this.tracks[i].loop = l;
        }
    }
});
```

### 9.7 Sound Formats

```javascript
ig.Sound.FORMAT = {
    MP3:  {ext: 'mp3', mime: 'audio/mpeg'},
    M4A:  {ext: 'm4a', mime: 'audio/mp4; codecs=mp4a.40.2'},
    OGG:  {ext: 'ogg', mime: 'audio/ogg; codecs=vorbis'},
    WEBM: {ext: 'webm', mime: 'audio/webm; codecs=vorbis'},
    CAF:  {ext: 'caf', mime: 'audio/x-caf'}
};

// Preferred format order
ig.Sound.use = [ig.Sound.FORMAT.OGG, ig.Sound.FORMAT.MP3];

// Number of channels for multi-channel sounds
ig.Sound.channels = 4;

// Use WebAudio if available
ig.Sound.useWebAudio = !!window.AudioContext;
```

---

## 10. Physics/Collision System

### 10.1 Collision Map (Tile-Based)

```javascript
ig.CollisionMap = ig.Map.extend({
    lastSlope: 1,    // Highest slope tile index
    tiledef: null,   // Tile definitions for slopes

    init: function( tilesize, data, tiledef ) {
        this.parent( tilesize, data );
        this.tiledef = tiledef || ig.CollisionMap.defaultTileDef;

        // Find highest slope index
        for( var t in this.tiledef ) {
            if( t|0 > this.lastSlope ) {
                this.lastSlope = t|0;
            }
        }
    },

    // Trace movement and return collision result
    trace: function( x, y, vx, vy, objectWidth, objectHeight ) {
        var res = {
            collision: {x: false, y: false, slope: false},
            pos: {x: x, y: y},
            tile: {x: 0, y: 0}
        };

        // Subdivide movement into tile-sized steps
        var steps = Math.ceil((Math.max(Math.abs(vx), Math.abs(vy))+0.1) / this.tilesize);

        if( steps > 1 ) {
            var sx = vx / steps;
            var sy = vy / steps;

            for( var i = 0; i < steps && (sx || sy); i++ ) {
                this._traceStep( res, x, y, sx, sy, objectWidth, objectHeight, vx, vy, i );
                x = res.pos.x;
                y = res.pos.y;
                if( res.collision.x ) { sx = 0; vx = 0; }
                if( res.collision.y ) { sy = 0; vy = 0; }
                if( res.collision.slope ) { break; }
            }
        }
        else {
            this._traceStep( res, x, y, vx, vy, objectWidth, objectHeight, vx, vy, 0 );
        }

        return res;
    }
});
```

### 10.2 Trace Step Algorithm

```javascript
ig.CollisionMap.prototype._traceStep = function( res, x, y, vx, vy, width, height, rvx, rvy, step ) {
    res.pos.x += vx;
    res.pos.y += vy;

    var t = 0;

    // Horizontal collision check
    if( vx ) {
        var pxOffsetX = (vx > 0 ? width : 0);   // Right edge for +vx, left for -vx
        var tileOffsetX = (vx < 0 ? this.tilesize : 0);

        var firstTileY = Math.max( Math.floor(y / this.tilesize), 0 );
        var lastTileY = Math.min( Math.ceil((y + height) / this.tilesize), this.height );
        var tileX = Math.floor( (res.pos.x + pxOffsetX) / this.tilesize );
        var prevTileX = Math.floor( (x + pxOffsetX) / this.tilesize );

        // Skip prevTileX check if same or out of bounds
        if( step > 0 || tileX == prevTileX || prevTileX < 0 || prevTileX >= this.width ) {
            prevTileX = -1;
        }

        if( tileX >= 0 && tileX < this.width ) {
            for( var tileY = firstTileY; tileY < lastTileY; tileY++ ) {
                // Check previous tile (for slope lines)
                if( prevTileX != -1 ) {
                    t = this.data[tileY][prevTileX];
                    if( t > 1 && t <= this.lastSlope &&
                        this._checkTileDef(res, t, x, y, rvx, rvy, width, height, prevTileX, tileY) ) {
                        break;
                    }
                }

                // Check current tile
                t = this.data[tileY][tileX];
                if( t == 1 || t > this.lastSlope || // Full solid tile
                    (t > 1 && this._checkTileDef(res, t, x, y, rvx, rvy, width, height, tileX, tileY)) ) {

                    if( t > 1 && t <= this.lastSlope && res.collision.slope ) {
                        break;
                    }

                    res.collision.x = true;
                    res.tile.x = t;
                    x = res.pos.x = tileX * this.tilesize - pxOffsetX + tileOffsetX;
                    rvx = 0;
                    break;
                }
            }
        }
    }

    // Vertical collision check (similar logic)
    if( vy ) {
        var pxOffsetY = (vy > 0 ? height : 0);
        var tileOffsetY = (vy < 0 ? this.tilesize : 0);

        var firstTileX = Math.max( Math.floor(res.pos.x / this.tilesize), 0 );
        var lastTileX = Math.min( Math.ceil((res.pos.x + width) / this.tilesize), this.width );
        var tileY = Math.floor( (res.pos.y + pxOffsetY) / this.tilesize );
        var prevTileY = Math.floor( (y + pxOffsetY) / this.tilesize );

        if( step > 0 || tileY == prevTileY || prevTileY < 0 || prevTileY >= this.height ) {
            prevTileY = -1;
        }

        if( tileY >= 0 && tileY < this.height ) {
            for( var tileX = firstTileX; tileX < lastTileX; tileX++ ) {
                if( prevTileY != -1 ) {
                    t = this.data[prevTileY][tileX];
                    if( t > 1 && t <= this.lastSlope &&
                        this._checkTileDef(res, t, x, y, rvx, rvy, width, height, tileX, prevTileY) ) {
                        break;
                    }
                }

                t = this.data[tileY][tileX];
                if( t == 1 || t > this.lastSlope ||
                    (t > 1 && this._checkTileDef(res, t, x, y, rvx, rvy, width, height, tileX, tileY)) ) {

                    if( t > 1 && t <= this.lastSlope && res.collision.slope ) {
                        break;
                    }

                    res.collision.y = true;
                    res.tile.y = t;
                    res.pos.y = tileY * this.tilesize - pxOffsetY + tileOffsetY;
                    break;
                }
            }
        }
    }
};
```

### 10.3 Slope Tile Definitions

```javascript
// Tile definition format: [x1, y1, x2, y2, solid]
// Line from (x1,y1) to (x2,y2) in tile coordinates (0-1)
// solid = true if tile is filled behind the line

var H = 1/2, N = 1/3, M = 2/3;
var SOLID = true, NON_SOLID = false;

ig.CollisionMap.defaultTileDef = {
    /* 15 degree NE */  5: [0,1, 1,M, SOLID],  6: [0,M, 1,N, SOLID],  7: [0,N, 1,0, SOLID],
    /* 22.5 degree NE */ 3: [0,1, 1,H, SOLID],  4: [0,H, 1,0, SOLID],
    /* 45 degree NE */   2: [0,1, 1,0, SOLID],
    /* 67.5 degree NE */ 10: [H,1, 1,0, SOLID], 21: [0,1, H,0, SOLID],
    /* 75 degree NE */   32: [M,1, 1,0, SOLID], 43: [N,1, M,0, SOLID], 54: [0,1, N,0, SOLID],

    /* 15 degree SE */   27: [0,0, 1,N, SOLID], 28: [0,N, 1,M, SOLID], 29: [0,M, 1,1, SOLID],
    /* 22.5 degree SE */ 25: [0,0, 1,H, SOLID], 26: [0,H, 1,1, SOLID],
    /* 45 degree SE */   24: [0,0, 1,1, SOLID],
    /* 67.5 degree SE */ 11: [0,0, H,1, SOLID], 22: [H,0, 1,1, SOLID],
    /* 75 degree SE */   33: [0,0, N,1, SOLID], 44: [N,0, M,1, SOLID], 55: [M,0, 1,1, SOLID],

    /* 15 degree NW */   16: [1,N, 0,0, SOLID], 17: [1,M, 0,N, SOLID], 18: [1,1, 0,M, SOLID],
    /* 22.5 degree NW */ 14: [1,H, 0,0, SOLID], 15: [1,1, 0,H, SOLID],
    /* 45 degree NW */   13: [1,1, 0,0, SOLID],
    /* 67.5 degree NW */  8: [H,1, 0,0, SOLID], 19: [1,1, H,0, SOLID],
    /* 75 degree NW */   30: [N,1, 0,0, SOLID], 41: [M,1, N,0, SOLID], 52: [1,1, M,0, SOLID],

    /* 15 degree SW */   38: [1,M, 0,1, SOLID], 39: [1,N, 0,M, SOLID], 40: [1,0, 0,N, SOLID],
    /* 22.5 degree SW */ 36: [1,H, 0,1, SOLID], 37: [1,0, 0,H, SOLID],
    /* 45 degree SW */   35: [1,0, 0,1, SOLID],
    /* 67.5 degree SW */  9: [1,0, H,1, SOLID], 20: [H,0, 0,1, SOLID],
    /* 75 degree SW */   31: [1,0, M,1, SOLID], 42: [M,0, N,1, SOLID], 53: [N,0, 0,1, SOLID],

    /* One-way platforms */
    /* Go N */  12: [0,0, 1,0, NON_SOLID],
    /* Go S */  23: [1,1, 0,1, NON_SOLID],
    /* Go E */  34: [1,0, 1,1, NON_SOLID],
    /* Go W */  45: [0,1, 0,0, NON_SOLID]
};
```

### 10.4 Slope Collision Check

```javascript
ig.CollisionMap.prototype._checkTileDef = function( res, t, x, y, vx, vy, width, height, tileX, tileY ) {
    var def = this.tiledef[t];
    if( !def ) { return false; }

    // Line endpoints in world coordinates
    var lx = (tileX + def[0]) * this.tilesize,
        ly = (tileY + def[1]) * this.tilesize,
        lvx = (def[2] - def[0]) * this.tilesize,
        lvy = (def[3] - def[1]) * this.tilesize,
        solid = def[4];

    // Find box corner to test (based on line direction)
    var tx = x + vx + (lvy < 0 ? width : 0) - lx,
        ty = y + vy + (lvx > 0 ? height : 0) - ly;

    // Is box corner behind the line? (cross product test)
    if( lvx * ty - lvy * tx > 0 ) {

        // Check if approaching from correct side (dot product with normal)
        if( vx * -lvy + vy * lvx < 0 ) {
            return solid;  // Wrong side, only solid tiles collide
        }

        // Calculate line normal
        var length = Math.sqrt(lvx * lvx + lvy * lvy);
        var nx = lvy/length,   // Normal X
            ny = -lvx/length;  // Normal Y

        // Project out of line
        var proj = tx * nx + ty * ny;
        var px = nx * proj,
            py = ny * proj;

        // Full tile collision if projection > movement
        if( px*px+py*py >= vx*vx+vy*vy ) {
            return solid || (lvx * (ty-vy) - lvy * (tx-vx) < 0.5);
        }

        // Slope collision - slide along surface
        res.pos.x = x + vx - px;
        res.pos.y = y + vy - py;
        res.collision.slope = {x: lvx, y: lvy, nx: nx, ny: ny};
        return true;
    }

    return false;
};
```

### 10.5 Collision Types

```javascript
ig.Entity.COLLIDES = {
    NEVER:   0,  // No collision
    LITE:    1,  // Entity moves, other stays fixed
    PASSIVE: 2,  // Both move on collision
    ACTIVE:  4,  // Both move on collision
    FIXED:   8   // Entity stays fixed, other moves
};

// Collision rules:
// - LITE vs LITE: No collision (1+1 = 2, not > 4)
// - LITE vs PASSIVE: No collision (1+2 = 3, not > 4)
// - LITE vs ACTIVE: Collision! (1+4 = 5 > 4), LITE moves
// - PASSIVE vs ACTIVE: Collision! (2+4 = 6 > 4), both move
// - ACTIVE vs ACTIVE: Collision! (4+4 = 8 > 4), both move
// - FIXED vs ANY: Collision!, other moves
```

### 10.6 Entity Types (for Checks)

```javascript
ig.Entity.TYPE = {
    NONE: 0,  // No type checks
    A: 1,     // Type A
    B: 2,     // Type B
    BOTH: 3   // Both A and B
};

// Bitwise check:
// if( a.checkAgainst & b.type ) { a.check(b); }
//
// Examples:
// - checkAgainst=A, type=A: 1 & 1 = 1 (true) - check!
// - checkAgainst=A, type=B: 1 & 2 = 0 (false) - no check
// - checkAgainst=BOTH, type=A: 3 & 1 = 1 (true) - check!
// - checkAgainst=BOTH, type=B: 3 & 2 = 2 (true) - check!
```

---

## 11. World/Level System

### 11.1 Level Data Structure

```javascript
// Level data format (JSON)
{
    "entities": [
        {
            "type": "EntityPlayer",
            "x": 100,
            "y": 200,
            "settings": {
                "health": 100,
                "speed": 200
            }
        },
        {
            "type": "EntityEnemy",
            "x": 400,
            "y": 200,
            "settings": {}
        }
    ],
    "layer": [
        {
            "name": "background",
            "width": 32,
            "height": 20,
            "tilesize": 32,
            "tilesetName": "media/tiles/bg.png",
            "distance": 2,
            "repeat": true,
            "preRender": false,
            "foreground": false,
            "data": [[0,0,0,...], [0,0,0,...], ...]
        },
        {
            "name": "collision",
            "width": 32,
            "height": 20,
            "tilesize": 32,
            "data": [[0,0,0,...], [0,1,1,...], ...]
        },
        {
            "name": "main",
            "width": 32,
            "height": 20,
            "tilesize": 32,
            "tilesetName": "media/tiles/main.png",
            "distance": 1,
            "foreground": false,
            "data": [[0,0,0,...], ...]
        }
    ]
}
```

### 11.2 Level Loading

```javascript
ig.Game.prototype.loadLevel = function( data ) {
    this.screen = {x: 0, y: 0};

    // Clear entities
    this.entities = [];
    this.namedEntities = {};

    // Spawn entities
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
            // Special collision map layer
            this.collisionMap = new ig.CollisionMap(ld.tilesize, ld.data);
        }
        else {
            // Background/visual layer
            var newMap = new ig.BackgroundMap(ld.tilesize, ld.data, ld.tilesetName);
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

### 11.3 Coordinate Systems

```
┌─────────────────────────────────────────────────────────────────┐
│                     COORDINATE SYSTEMS                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  WORLD COORDINATES                                               │
│  - Absolute position in game world                              │
│  - entity.pos.x, entity.pos.y                                   │
│  - Independent of camera/screen                                  │
│                                                                  │
│  SCREEN COORDINATES                                             │
│  - Position relative to camera view                             │
│  - screenX = worldX - game.screen.x                             │
│  - screenY = worldY - game.screen.y                             │
│                                                                  │
│  DRAW COORDINATES                                               │
│  - Final canvas coordinates (scaled)                            │
│  - drawX = getDrawPos(screenX) * scale                          │
│                                                                  │
│  TILE COORDINATES                                               │
│  - Grid position in map                                         │
│  - tileX = floor(worldX / tilesize)                             │
│  - tileY = floor(worldY / tilesize)                             │
│                                                                  │
│  PARALLAX OFFSET                                                │
│  - For background maps with distance != 1                       │
│  - scroll.x = screen.x / distance                               │
│  - scroll.y = screen.y / distance                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 11.4 Entity Spawning

```javascript
ig.Game.prototype.spawnEntity = function( type, x, y, settings ) {
    var entityClass = typeof(type) === 'string'
        ? ig.global[type]
        : type;

    if( !entityClass ) {
        throw("Can't spawn entity of type " + type);
    }

    // Try pool first (if using entity pooling)
    var ent = new (entityClass)( x, y, settings || {} );

    this.entities.push( ent );

    if( ent.name ) {
        this.namedEntities[ent.name] = ent;
    }

    return ent;
};
```

### 11.5 Entity Queries

```javascript
ig.Game.prototype.getEntityByName = function( name ) {
    return this.namedEntities[name];
};

ig.Game.prototype.getEntitiesByType = function( type ) {
    var entityClass = typeof(type) === 'string'
        ? ig.global[type]
        : type;

    var a = [];
    for( var i = 0; i < this.entities.length; i++ ) {
        var ent = this.entities[i];
        if( ent instanceof entityClass && !ent._killed ) {
            a.push( ent );
        }
    }
    return a;
};

ig.Game.prototype.getMapByName = function( name ) {
    if( name == 'collision' ) {
        return this.collisionMap;
    }

    for( var i = 0; i < this.backgroundMaps.length; i++ ) {
        if( this.backgroundMaps[i].name == name ) {
            return this.backgroundMaps[i];
        }
    }

    return null;
};
```

---

## 12. Timer System

### 12.1 Timer Implementation

```javascript
ig.Timer = ig.Class.extend({
    target: 0,       // Target time in seconds
    base: 0,         // Base time when started
    last: 0,         // Last tick time
    pausedAt: 0,     // Time when paused (0 = not paused)

    init: function( seconds ) {
        this.base = ig.Timer.time;
        this.last = ig.Timer.time;
        this.target = seconds || 0;
    },

    // Set countdown target
    set: function( seconds ) {
        this.target = seconds || 0;
        this.base = ig.Timer.time;
        this.pausedAt = 0;
    },

    // Reset without changing target
    reset: function() {
        this.base = ig.Timer.time;
        this.pausedAt = 0;
    },

    // Get delta since last tick (for frame timing)
    tick: function() {
        var delta = ig.Timer.time - this.last;
        this.last = ig.Timer.time;
        return (this.pausedAt ? 0 : delta);
    },

    // Get time elapsed since start (negative = counting down)
    delta: function() {
        return (this.pausedAt || ig.Timer.time) - this.base - this.target;
    },

    pause: function() {
        if( !this.pausedAt ) {
            this.pausedAt = ig.Timer.time;
        }
    },

    unpause: function() {
        if( this.pausedAt ) {
            this.base += ig.Timer.time - this.pausedAt;
            this.pausedAt = 0;
        }
    }
});

// Static time tracking
ig.Timer._last = 0;
ig.Timer.time = Number.MIN_VALUE;
ig.Timer.timeScale = 1;
ig.Timer.maxStep = 0.05;  // Max delta to prevent spiral of death

ig.Timer.step = function() {
    var current = Date.now();
    var delta = (current - ig.Timer._last) / 1000;

    // Apply time scale and clamp max step
    ig.Timer.time += Math.min(delta, ig.Timer.maxStep) * ig.Timer.timeScale;
    ig.Timer._last = current;
};
```

### 12.2 Timer Usage Patterns

```javascript
// Countdown timer
var timer = new ig.Timer(5);  // 5 second countdown
if( timer.delta() >= 0 ) {
    // Time's up!
}

// Elapsed time
var timer = new ig.Timer();
var elapsed = timer.delta();  // Seconds since creation

// Frame delta (used in System)
var tick = timer.tick();  // Seconds since last tick

// Slow motion
ig.Timer.timeScale = 0.5;  // Half speed
ig.Timer.timeScale = 1.0;  // Normal speed
ig.Timer.timeScale = 2.0;  // Fast forward
```

---

## 13. Font System

### 13.1 Bitmap Font Loading

```javascript
ig.Font = ig.Image.extend({
    widthMap: [],      // Width of each character
    indices: [],       // X position of each character in sprite
    firstChar: 32,     // ASCII code of first character (space)
    alpha: 1,
    letterSpacing: 1,
    lineSpacing: 0,

    // Auto-detect character widths from bottom row of font image
    onload: function( ev ) {
        this._loadMetrics( this.data );
        this.parent( ev );
        this.height -= 2; // Last 2 lines contain no visual data
    },

    _loadMetrics: function( image ) {
        this.widthMap = [];
        this.indices = [];

        // Analyze bottom row of font image
        var px = ig.getImagePixels( image, 0, image.height-1, image.width, 1 );

        var currentWidth = 0;
        for( var x = 0; x < image.width; x++ ) {
            var index = x * 4 + 3; // Alpha channel

            if( px.data[index] > 127 ) {
                // Opaque pixel - part of character
                currentWidth++;
            }
            else if( px.data[index] < 128 && currentWidth ) {
                // Transparent after opaque - end of character
                this.widthMap.push( currentWidth );
                this.indices.push( x-currentWidth );
                currentWidth = 0;
            }
        }
        this.widthMap.push( currentWidth );
        this.indices.push( x-currentWidth );
    }
});
```

### 13.2 Font Rendering

```javascript
ig.Font.prototype.draw = function( text, x, y, align ) {
    if( typeof(text) != 'string' ) {
        text = text.toString();
    }

    // Handle multiline
    if( text.indexOf('\n') !== -1 ) {
        var lines = text.split( '\n' );
        var lineHeight = this.height + this.lineSpacing;
        for( var i = 0; i < lines.length; i++ ) {
            this.draw( lines[i], x, y + i * lineHeight, align );
        }
        return;
    }

    // Alignment
    if( align == ig.Font.ALIGN.RIGHT || align == ig.Font.ALIGN.CENTER ) {
        var width = this._widthForLine( text );
        x -= align == ig.Font.ALIGN.CENTER ? width/2 : width;
    }

    if( this.alpha !== 1 ) {
        ig.system.context.globalAlpha = this.alpha;
    }

    // Draw each character
    for( var i = 0; i < text.length; i++ ) {
        var c = text.charCodeAt(i);
        x += this._drawChar( c - this.firstChar, x, y );
    }

    if( this.alpha !== 1 ) {
        ig.system.context.globalAlpha = 1;
    }
    ig.Image.drawCount += text.length;
};

ig.Font.prototype._drawChar = function( c, targetX, targetY ) {
    if( !this.loaded || c < 0 || c >= this.indices.length ) { return 0; }

    var scale = ig.system.scale;

    var charX = this.indices[c] * scale;
    var charY = 0;
    var charWidth = this.widthMap[c] * scale;
    var charHeight = this.height * scale;

    ig.system.context.drawImage(
        this.data,
        charX, charY,
        charWidth, charHeight,
        ig.system.getDrawPos(targetX), ig.system.getDrawPos(targetY),
        charWidth, charHeight
    );

    return this.widthMap[c] + this.letterSpacing;
};
```

### 13.3 Text Measurement

```javascript
ig.Font.prototype.widthForString = function( text ) {
    if( text.indexOf('\n') !== -1 ) {
        var lines = text.split( '\n' );
        var width = 0;
        for( var i = 0; i < lines.length; i++ ) {
            width = Math.max( width, this._widthForLine(lines[i]) );
        }
        return width;
    }
    else {
        return this._widthForLine( text );
    }
};

ig.Font.prototype._widthForLine = function( text ) {
    var width = 0;
    for( var i = 0; i < text.length; i++ ) {
        width += this.widthMap[text.charCodeAt(i) - this.firstChar];
    }
    if( text.length > 0 ) {
        width += this.letterSpacing * (text.length - 1);
    }
    return width;
};

ig.Font.prototype.heightForString = function( text ) {
    return text.split('\n').length * (this.height + this.lineSpacing);
};
```

---

## 14. Debug System

### 14.1 Debug Panel Architecture

```javascript
ig.Debug = ig.Class.extend({
    options: {},
    panels: {},
    numbers: {},
    container: null,
    panelMenu: null,
    numberContainer: null,
    activePanel: null,

    init: function() {
        // Create container
        this.container = ig.$new('div');
        this.container.className = 'ig_debug';

        // Create panel menu
        this.panelMenu = ig.$new('div');
        this.panelMenu.className = 'ig_debug_panel_menu';

        // Create stats container
        this.numberContainer = ig.$new('div');
        this.numberContainer.className = 'ig_debug_stats';
    },

    addPanel: function( panelDef ) {
        var panel = new (panelDef.type)( panelDef.name, panelDef.label );

        if( panelDef.options ) {
            for( var i = 0; i < panelDef.options.length; i++ ) {
                var opt = panelDef.options[i];
                panel.addOption( new ig.DebugOption(opt.name, opt.object, opt.property) );
            }
        }

        this.panels[ panel.name ] = panel;
        this.container.appendChild( panel.container );

        // Create menu item
        var menuItem = ig.$new('div');
        menuItem.className = 'ig_debug_menu_item';
        menuItem.textContent = panel.label;
        menuItem.addEventListener('click', (function(ev){
            this.togglePanel(panel);
        }).bind(this), false);

        panel.menuItem = menuItem;
        this.panelMenu.appendChild( menuItem );
    },

    beforeRun: function() {
        if( this.activePanel ) {
            this.activePanel.beforeRun();
        }
    },

    afterRun: function() {
        if( this.activePanel ) {
            this.activePanel.afterRun();
        }

        // Update stats
        this.showNumber( 'ms',  this.debugTime.toFixed(2) );
        this.showNumber( 'fps',  Math.round(1000/this.debugTickAvg) );
        this.showNumber( 'draws', ig.Image.drawCount );
        this.showNumber( 'entities', ig.game.entities.length );

        ig.Image.drawCount = 0;
    }
});
```

### 14.2 Performance Graph Panel

```javascript
ig.DebugGraphPanel = ig.DebugPanel.extend({
    clocks: {},
    marks: [],
    height: 128,
    ms: 64,  // Graph represents 64ms

    addClock: function( name, description, color ) {
        this.clocks[name] = {
            description: description,
            color: color,
            current: 0,
            start: Date.now(),
            avg: 0,
            html: numberElement
        };
    },

    beginClock: function( name ) {
        this.clocks[name].start = Date.now();
    },

    endClock: function( name ) {
        var c = this.clocks[name];
        c.current = Math.round(Date.now() - c.start);
        c.avg = c.avg * 0.8 + c.current * 0.2;  // EMA
    },

    afterRun: function() {
        // Shift graph left
        this.ctx.drawImage( this.graph, -1, 0 );

        // Draw new column
        var x = this.graph.width-1;
        var y = this.height;

        for( var ci in this.clocks ) {
            var c = this.clocks[ci];

            if( c.color && c.current > 0 ) {
                this.ctx.fillStyle = c.color;
                var h = c.current * (this.height/this.ms);
                y -= h;
                this.ctx.fillRect( x, y, 1, h );
                c.current = 0;
            }
        }
    }
});

// Injected into game loop
ig.Game.inject({
    draw: function() {
        ig.graph.beginClock('draw');
        this.parent();
        ig.graph.endClock('draw');
    },

    update: function() {
        ig.graph.beginClock('update');
        this.parent();
        ig.graph.endClock('update');
    },

    checkEntities: function() {
        ig.graph.beginClock('checks');
        this.parent();
        ig.graph.endClock('checks');
    }
});
```

### 14.3 Entity Debug Options

```javascript
ig.Entity.inject({
    colors: {
        names: '#fff',
        velocities: '#0f0',
        boxes: '#f00'
    },

    draw: function() {
        this.parent();

        // Show collision boxes
        if( ig.Entity._debugShowBoxes ) {
            ig.system.context.strokeStyle = this.colors.boxes;
            ig.system.context.lineWidth = 1.0;
            ig.system.context.strokeRect(
                ig.system.getDrawPos(this.pos.x.round() - ig.game.screen.x) - 0.5,
                ig.system.getDrawPos(this.pos.y.round() - ig.game.screen.y) - 0.5,
                this.size.x * ig.system.scale,
                this.size.y * ig.system.scale
            );
        }

        // Show velocity vectors
        if( ig.Entity._debugShowVelocities ) {
            var x = this.pos.x + this.size.x/2;
            var y = this.pos.y + this.size.y/2;
            this._debugDrawLine( this.colors.velocities,
                x, y, x + this.vel.x, y + this.vel.y );
        }

        // Show names
        if( ig.Entity._debugShowNames ) {
            if( this.name ) {
                ig.system.context.fillStyle = this.colors.names;
                ig.system.context.fillText(
                    this.name,
                    ig.system.getDrawPos(this.pos.x - ig.game.screen.x),
                    ig.system.getDrawPos(this.pos.y - ig.game.screen.y)
                );
            }
        }
    }
});

ig.Entity._debugEnableChecks = true;
ig.Entity._debugShowBoxes = false;
ig.Entity._debugShowVelocities = false;
ig.Entity._debugShowNames = false;
```

---

## 15. Weltmeister Level Editor

### 15.1 Editor Architecture

Weltmeister is the official level editor for Impact.js, built with Impact.js itself.

```
┌─────────────────────────────────────────────────────────────────┐
│                    WELTMEISTER EDITOR                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Canvas Area                                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                                                          │   │
│  │  Rendered Layers (back to front):                       │   │
│  │  1. Background maps (non-foreground)                    │   │
│  │  2. Entities (when not in foreground mode)              │   │
│  │  3. Foreground maps                                     │   │
│  │  4. Selection cursor / brush preview                    │   │
│  │  5. Grid overlay (optional)                             │   │
│  │  6. Coordinate labels                                   │   │
│  │                                                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  Side Panel                                                      │
│  ┌──────────────────┐                                           │
│  │ Layers List      │  - Drag to reorder                        │
│  │ [1] main    [x]  │  - Click to select                        │
│  │ [2] collision[x] │  - Number = hotkey                        │
│  │ [3] entities  [x]│  - [x] = visibility toggle                │
│  │                  │                                           │
│  │ [+ Add Layer]    │                                           │
│  │ [- Remove]       │                                           │
│  └──────────────────┘                                           │
│                                                                  │
│  Layer Settings Panel                                           │
│  ┌──────────────────┐                                           │
│  │ Name: [main    ] │                                           │
│  │ Tileset: [..]    │  - Browse button                          │
│  │ Tilesize: [32  ] │                                           │
│  │ Width:  [64   ]  │                                           │
│  │ Height: [64   ]  │                                           │
│  │ [x] Pre-render   │                                           │
│  │ [x] Repeat       │                                           │
│  │ [x] Collision    │  - Makes layer a collision layer          │
│  │ Distance: [1  ]  │  - Parallax factor                        │
│  │ [Save Settings]  │                                           │
│  └──────────────────┘                                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 15.2 Editor Modes

```javascript
wm.Weltmeister.prototype.MODE = {
    DRAW: 1,         // Default - draw tiles, select entities
    TILESELECT: 2,   // Selecting tile from tileset popup
    ENTITYSELECT: 4  // Selecting entity type from menu
};
```

### 15.3 Tile Selection and Brush System

```javascript
wm.EditMap = ig.Class.extend({
    brush: [[1]],  // Current brush (2D array of tile indices)
    isSelecting: false,
    selectStart: {x: 0, y: 0},
    tileSelect: null,  // Tile selection popup

    beginEditing: function() {
        // Start of tile drawing
    },

    beginSelecting: function( mouseX, mouseY ) {
        this.isSelecting = true;
        this.selectStart = {
            x: Math.floor((mouseX + this.scroll.x) / this.tilesize),
            y: Math.floor((mouseY + this.scroll.y) / this.tilesize)
        };
    },

    endSelecting: function( mouseX, mouseY ) {
        this.isSelecting = false;

        var endX = Math.floor((mouseX + this.scroll.x) / this.tilesize);
        var endY = Math.floor((mouseY + this.scroll.y) / this.tilesize);

        // Create brush from selection
        var startX = Math.min(this.selectStart.x, endX);
        var startY = Math.min(this.selectStart.y, endY);
        var width = Math.abs(endX - this.selectStart.x) + 1;
        var height = Math.abs(endY - this.selectStart.y) + 1;

        var brush = [];
        for( var y = 0; y < height; y++ ) {
            brush[y] = [];
            for( var x = 0; x < width; x++ ) {
                brush[y][x] = this.data[startY+y][startX+x];
            }
        }

        return brush;
    }
});
```

### 15.4 Entity Editing

```javascript
wm.EditEntities = ig.Class.extend({
    entities: [],      // Editor entities
    selected: null,    // Currently selected entity
    entityClasses: {}, // Known entity types

    spawnEntity: function( type, x, y, settings ) {
        var entity = {
            type: type,
            x: x,
            y: y,
            settings: settings || {},
            size: {x: 16, y: 16},  // Default or from class
            name: settings.name
        };
        this.entities.push( entity );
        return entity;
    },

    selectEntityAt: function( x, y ) {
        for( var i = this.entities.length - 1; i >= 0; i-- ) {
            var ent = this.entities[i];
            if( x >= ent.x && x < ent.x + ent.size.x &&
                y >= ent.y && y < ent.y + ent.size.y ) {
                return this.selectEntity( ent );
            }
        }
        return this.selectEntity( null );
    },

    getSaveData: function() {
        var data = [];
        for( var i = 0; i < this.entities.length; i++ ) {
            var ent = this.entities[i];
            data.push({
                type: ent.type,
                x: ent.x,
                y: ent.y,
                settings: ent.settings
            });
        }
        return data;
    }
});
```

### 15.5 Undo System

```javascript
wm.Undo = ig.Class.extend({
    levels: [],
    maxLevels: 100,
    currentLevel: -1,

    beginMapDraw: function() {
        // Start new undo level for map drawing
        this.currentLevel++;
        this.levels[this.currentLevel] = {
            type: 'map',
            changes: []
        };
        // Truncate any redo states
        this.levels.length = this.currentLevel + 1;
    },

    pushMapDraw: function( layer, x, y, oldTile, newTile ) {
        this.levels[this.currentLevel].changes.push({
            layer: layer,
            x: x,
            y: y,
            old: oldTile,
            new: newTile
        });

        // Check max levels
        if( this.levels.length > this.maxLevels ) {
            this.levels.shift();
            this.currentLevel--;
        }
    },

    undo: function() {
        if( this.currentLevel < 0 ) { return; }

        var level = this.levels[this.currentLevel];

        if( level.type == 'map' ) {
            // Reverse all changes
            for( var i = level.changes.length - 1; i >= 0; i-- ) {
                var change = level.changes[i];
                change.layer.data[change.y][change.x] = change.old;
            }
        }

        this.currentLevel--;
    },

    redo: function() {
        if( this.currentLevel >= this.levels.length - 1 ) { return; }

        this.currentLevel++;
        var level = this.levels[this.currentLevel];

        if( level.type == 'map' ) {
            for( var i = 0; i < level.changes.length; i++ ) {
                var change = level.changes[i];
                change.layer.data[change.y][change.x] = change.new;
            }
        }
    }
});
```

### 15.6 Save Format

```javascript
// Level is saved as an Impact module
ig.module('game.levels.my-level')
.requires('impact.image', 'game.entities.player')
.defines(function(){
    LevelMyLevel={
        "entities":[...],
        "layer":[...]
    };
    LevelMyLevelResources=[
        new ig.Image('media/tiles/main.png'),
        new ig.Image('media/tiles/bg.png')
    ];
});
```

---

## 16. Complete Game Loop

### 16.1 Full Run Loop Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         IMPACT.JS GAME LOOP                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ig.main(canvas, GameClass, fps, width, height, scale)                  │
│    │                                                                     │
│    ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ 1. SYSTEM INITIALIZATION                                        │    │
│  │    - Create ig.System (canvas, context, resize)                │    │
│  │    - Create ig.Input (bind events)                              │    │
│  │    - Create ig.SoundManager (format detection)                 │    │
│  │    - Create ig.Music                                            │    │
│  │    - Set ig.ready = true                                        │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│    │                                                                     │
│    ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ 2. RESOURCE LOADING (ig.Loader)                                 │    │
│  │    - Show loading screen                                        │    │
│  │    - Load all Images:                                           │    │
│  │        new Image() → onload → resize if scale != 1             │    │
│  │    - Load all Sounds:                                           │    │
│  │        WebAudio: XHR + decodeAudioData                          │    │
│  │        HTML5: new Audio() + canplaythrough                      │    │
│  │    - Load all Fonts:                                            │    │
│  │        new Image() → onload → _loadMetrics()                   │    │
│  │    - Update progress bar                                        │    │
│  │    - When complete: clearInterval, setGame()                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│    │                                                                     │
│    ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ 3. GAME INSTANTIATION                                           │    │
│  │    - ig.game = new GameClass()                                  │    │
│  │    - ig.system.setDelegate(ig.game)                            │    │
│  │    - ig.system.startRunLoop()                                  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│    │                                                                     │
│    ▼                                                                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ 4. RUN LOOP (requestAnimationFrame)                             │    │
│  │                                                                  │    │
│  │    ┌────────────────────────────────────────────────────────┐   │    │
│  │    │ EACH FRAME:                                            │   │    │
│  │    │                                                         │   │    │
│  │    │ 1. ig.Timer.step()                                     │   │    │
│  │    │    - delta = (now - last) / 1000                       │   │    │
│  │    │    - time += min(delta, maxStep) * timeScale          │   │    │
│  │    │    - last = now                                        │   │    │
│  │    │                                                         │   │    │
│  │    │ 2. system.tick = clock.tick()                          │   │    │
│  │    │    - Delta time for this frame                         │   │    │
│  │    │                                                         │   │    │
│  │    │ 3. ig.debug.beforeRun()                                │   │    │
│  │    │    - Start profiling timers                             │   │    │
│  │    │                                                         │   │    │
│  │    │ 4. delegate.run() ← GAME UPDATE/DRAW                  │   │    │
│  │    │    │                                                     │   │    │
│  │    │    ▼                                                     │   │    │
│  │    │    ┌─────────────────────────────────────────────────┐ │   │    │
│  │    │    │ GAME.update()                                   │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ a. Level loading (if deferred)                  │ │   │    │
│  │    │    │    - loadLevel(data)                            │ │   │    │
│  │    │    │    - Spawn entities from data                   │ │   │    │
│  │    │    │    - Create collision map                       │ │   │    │
│  │    │    │    - Create background maps                     │ │   │    │
│  │    │    │    - Call entity.ready()                        │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ b. ig.debug.beginClock('update')               │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ c. updateEntities()                             │ │   │    │
│  │    │    │    - For each entity:                           │ │   │    │
│  │    │    │        - last = pos                             │ │   │    │
│  │    │    │        - vel.y += gravity * tick               │ │   │    │
│  │    │    │        - Apply accel/friction                   │ │   │    │
│  │    │    │        - collisionMap.trace()                   │ │   │    │
│  │    │    │        - handleMovementTrace()                  │ │   │    │
│  │    │    │        - currentAnim.update()                   │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ d. ig.debug.endClock('update')                 │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ e. ig.debug.beginClock('checks')               │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ f. checkEntities()                              │ │   │    │
│  │    │    │    - Spatial hash grid insertion                │ │   │    │
│  │    │    │    - For each cell:                            │ │   │    │
│  │    │    │        - touches() test                         │ │   │    │
│  │    │    │        - checkPair()                           │ │   │    │
│  │    │    │            - a.check(b) / b.check(a)           │ │   │    │
│  │    │    │            - solveCollision() if collides      │ │   │    │
│  │    │    │                - seperateOnXAxis/YAxis         │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ g. ig.debug.endClock('checks')                 │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ h. Remove killed entities                       │ │   │    │
│  │    │    │    - For each in _deferredKill:                │ │   │    │
│  │    │    │        - entity.erase()                         │ │   │    │
│  │    │    │        - entities.erase(entity)                │ │   │    │
│  │    │    │    - _deferredKill = []                        │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ i. Sort entities (if autoSort)                 │ │   │    │
│  │    │    │    - entities.sort(sortBy)                     │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ j. Update background anims                      │ │   │    │
│  │    │    │    - For each tileset anim: anim.update()      │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    └─────────────────────────────────────────────────┘ │   │    │
│  │    │                                                         │   │    │
│  │    │    ┌─────────────────────────────────────────────────┐ │   │    │
│  │    │    │ GAME.draw()                                     │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ a. ig.debug.beginClock('draw')                 │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ b. ig.system.clear(clearColor)                 │ │   │    │
│  │    │    │    - fillRect(0, 0, realWidth, realHeight)     │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ c. Calculate _rscreen (rounded screen pos)      │ │   │    │
│  │    │    │    - _rscreen = round(screen) / scale          │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ d. Draw background maps (non-foreground)        │ │   │    │
│  │    │    │    - map.setScreenPos(screen.x, screen.y)      │ │   │    │
│  │    │    │    - map.draw()                                 │ │   │    │
│  │    │    │        - If preRender: drawPreRendered()       │ │   │    │
│  │    │    │        - Else: drawTiled()                     │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ e. Draw entities                                │ │   │    │
│  │    │    │    - For each: entity.draw()                   │ │   │    │
│  │    │    │        - currentAnim.draw(pos - _rscreen)      │ │   │    │
│  │    │    │            - sheet.image.drawTile()            │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ f. Draw background maps (foreground)            │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    │ g. ig.debug.endClock('draw')                   │ │   │    │
│  │    │    │                                                  │ │   │    │
│  │    │    └─────────────────────────────────────────────────┘ │   │    │
│  │    │                                                         │   │    │
│  │    │ 5. ig.input.clearPressed()                             │   │    │
│  │    │    - Clear press/release states                        │   │    │
│  │    │                                                         │   │    │
│  │    │ 6. Handle game switch (if any)                         │   │    │
│  │    │    - If newGameClass: setGameNow()                     │   │    │
│  │    │                                                         │   │    │
│  │    │ 7. ig.debug.afterRun()                                 │   │    │
│  │    │    - Update FPS, ms, entity count stats               │   │    │
│  │    │    - Update active debug panel                         │   │    │
│  │    │                                                         │   │    │
│  │    └────────────────────────────────────────────────────────┘   │    │
│  │                                                                  │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 16.2 Frame Timing Breakdown

```
Frame N                                              Frame N+1
│                                                      │
├─ Timer.step()                                       │
│  delta = 16.67ms (60 FPS)                          │
│  time += delta * timeScale                         │
│                                                      │
├─ system.tick = 0.01667                              │
│                                                      │
├─ UPDATE PHASE                                       │
│  ├─ Entity Update: 2-5ms                           │
│  │  - Physics calculations                          │
│  │  - Collision map trace                          │
│  │  - Animation update                             │
│  │                                                  │
│  └─ Entity Checks: 1-3ms                           │
│     - Spatial hash build                           │
│     - Pairwise collision resolution                │
│                                                      │
├─ DRAW PHASE                                         │
│  ├─ Clear: 0.1ms                                   │
│  ├─ Background Maps: 1-3ms                         │
│  │  - Chunk drawing OR tile iteration              │
│  │                                                  │
│  ├─ Entities: 1-2ms                                │
│  │  - Sprite drawing                              │
│  │                                                  │
│  └─ Total Draw: 2-5ms                              │
│                                                      │
├─ Total Frame: 5-13ms (75-200 FPS potential)        │
│                                                      │
└─ Wait for next VSync (requestAnimationFrame)       │
```

---

## 17. Mathematical Foundations

### 17.1 Number Extensions

```javascript
// Map value from one range to another
Number.prototype.map = function(istart, istop, ostart, ostop) {
    return ostart + (ostop - ostart) * ((this - istart) / (istop - istart));
};
// Example: 5.map(0, 10, 0, 100) = 50

// Clamp value to range
Number.prototype.limit = function(min, max) {
    return Math.min(max, Math.max(min, this));
};
// Example: 150.limit(0, 100) = 100

// Round to precision
Number.prototype.round = function(precision) {
    precision = Math.pow(10, precision || 0);
    return Math.round(this * precision) / precision;
};
// Example: 3.14159.round(2) = 3.14

// Convert degrees to radians
Number.prototype.toRad = function() {
    return (this / 180) * Math.PI;
};

// Convert radians to degrees
Number.prototype.toDeg = function() {
    return (this * 180) / Math.PI;
};
```

### 17.2 Collision Mathematics

```javascript
// AABB Overlap Test
ig.Entity.prototype.touches = function( other ) {
    return !(
        this.pos.x >= other.pos.x + other.size.x ||  // Right of other
        this.pos.x + this.size.x <= other.pos.x ||   // Left of other
        this.pos.y >= other.pos.y + other.size.y ||  // Below other
        this.pos.y + this.size.y <= other.pos.y      // Above other
    );
};
// Returns true if rectangles overlap

// Line Side Test (for slope collision)
// Cross product to determine which side of line a point is on
// lvx, lvy = line vector
// tx, ty = test point relative to line start
if( lvx * ty - lvy * tx > 0 ) {
    // Point is on "positive" side of line
}

// Line Normal Calculation
var length = Math.sqrt(lvx * lvx + lvy * lvy);
var nx = lvy / length;   // Perpendicular X (rotated 90)
var ny = -lvx / length;  // Perpendicular Y

// Vector Projection onto Normal
var proj = tx * nx + ty * ny;  // Dot product
var px = nx * proj;  // Projection vector X
var py = ny * proj;  // Projection vector Y

// Reflect Velocity off Normal (bounce)
var proj = vel.x * nx + vel.y * ny;  // Dot product
vel.x = (vel.x - nx * proj * 2) * bounciness;
vel.y = (vel.y - ny * proj * 2) * bounciness;

// Project Velocity onto Slope (slide)
var lengthSquared = s.x * s.x + s.y * s.y;
var dot = (vel.x * s.x + vel.y * s.y) / lengthSquared;
vel.x = s.x * dot;
vel.y = s.y * dot;
```

### 17.3 Distance and Angle Calculations

```javascript
// Distance between entity centers
ig.Entity.prototype.distanceTo = function( other ) {
    var xd = (this.pos.x + this.size.x/2) - (other.pos.x + other.size.x/2);
    var yd = (this.pos.y + this.size.y/2) - (other.pos.y + other.size.y/2);
    return Math.sqrt( xd*xd + yd*yd );
};

// Angle from this entity to other
ig.Entity.prototype.angleTo = function( other ) {
    return Math.atan2(
        (other.pos.y + other.size.y/2) - (this.pos.y + this.size.y/2),
        (other.pos.x + other.size.x/2) - (this.pos.x + this.size.x/2)
    );
};
```

### 17.4 Parallax Scrolling Math

```javascript
// Background map scroll based on distance factor
ig.BackgroundMap.prototype.setScreenPos = function( x, y ) {
    // distance = 1: scroll same as screen (normal layer)
    // distance = 2: scroll at half speed (distant background)
    // distance = 0.5: scroll at double speed (foreground overlay)
    this.scroll.x = x / this.distance;
    this.scroll.y = y / this.distance;
};

// Example:
// screen.x = 100
// distance = 2
// scroll.x = 100 / 2 = 50  (background moves slower)
```

### 17.5 Tile Coordinate Conversion

```javascript
// World → Tile
var tileX = Math.floor( worldX / tilesize );
var tileY = Math.floor( worldY / tilesize );

// Tile → World
var worldX = tileX * tilesize;
var worldY = tileY * tilesize;

// Pixel offset within tile
var pixelX = worldX % tilesize;
var pixelY = worldY % tilesize;
```

---

## 18. Performance Considerations

### 18.1 Optimization Strategies

#### 1. Pre-rendered Background Maps

```javascript
// Enable for static layers
map.preRender = true;

// Chunks are rendered once to offscreen canvases
// Significantly faster for large, static backgrounds
// Memory cost: ~1 canvas per 512x512 pixel chunk
```

#### 2. Entity Pooling

```javascript
// Enable pooling for frequently spawned entities
ig.EntityPool.enableFor( EntityBullet );
ig.EntityPool.enableFor( EntityParticle );

// Entities are reused instead of garbage collected
// Prevents GC pauses during gameplay
```

#### 3. Spatial Hash Collision

```javascript
// Default cell size is 64 pixels
game.cellSize = 64;

// Adjust based on entity sizes:
// - Smaller cellSize: More cells, fewer false positives
// - Larger cellSize: Fewer cells, more entities per cell
```

#### 4. Entity Type Filtering

```javascript
// Configure entities to only check/collide when necessary
EntityBullet = ig.Entity.extend({
    type: ig.Entity.TYPE.A,
    checkAgainst: ig.Entity.TYPE.B,  // Only check vs type B
    collides: ig.Entity.COLLIDES.LITE  // Light collision
});

// Skip collision for non-interactive entities
EntityDecoration = ig.Entity.extend({
    type: ig.Entity.TYPE.NONE,
    checkAgainst: ig.Entity.TYPE.NONE,
    collides: ig.Entity.COLLIDES.NEVER
});
```

#### 5. Culling

```javascript
// Built into animation drawing
ig.Animation.prototype.draw = function( targetX, targetY ) {
    var bbsize = Math.max(this.sheet.width, this.sheet.height);

    if(
       targetX > ig.system.width || targetY > ig.system.height ||
       targetX + bbsize < 0 || targetY + bbsize < 0
    ) {
        return;  // Skip off-screen drawing
    }
    // ... draw
};
```

### 18.2 Memory Management

```javascript
// Image caching - automatic
ig.Image.cache[path];  // Prevents duplicate loads

// Manual cache clearing if needed
for( var path in ig.Image.cache ) {
    delete ig.Image.cache[path];
}

// Entity cleanup on level load
ig.EntityPool.drainAllPools();  // Clear pooled entities

// Sound channel management
// - SFX: 4 channels pooled (configurable)
// - Music: Single channel, HTML5 streaming
```

### 18.3 Performance Metrics

```javascript
// Available via debug panel:
// - ms: Frame time in milliseconds
// - fps: Frames per second
// - draws: Draw calls per frame
// - entities: Active entity count

// Performance clocks (debug mode):
// - update: Entity update time
// - checks: Collision detection time
// - draw: Rendering time
// - lag: Frame pacing variance

// Target budgets (60 FPS):
// - Total frame: 16.67ms
// - Update: < 5ms
// - Draw: < 5ms
// - Checks: < 3ms
```

---

## Appendix A: Class Reference

| Class | File | Purpose |
|-------|------|---------|
| `ig` | impact.js | Global namespace, module system |
| `ig.Class` | impact.js | Base class for inheritance |
| `ig.System` | system.js | Game loop, canvas, timing |
| `ig.Input` | input.js | Input handling |
| `ig.Loader` | loader.js | Resource loading |
| `ig.Game` | game.js | Main game class |
| `ig.Entity` | entity.js | Base entity |
| `ig.EntityPool` | entity-pool.js | Object pooling |
| `ig.Map` | map.js | Base map class |
| `ig.CollisionMap` | collision-map.js | Tile collision |
| `ig.BackgroundMap` | background-map.js | Parallax backgrounds |
| `ig.Image` | image.js | Image loading/drawing |
| `ig.AnimationSheet` | animation.js | Sprite sheet |
| `ig.Animation` | animation.js | Animation |
| `ig.Font` | font.js | Bitmap fonts |
| `ig.Timer` | timer.js | Timing utilities |
| `ig.Sound` | sound.js | Sound effects |
| `ig.Music` | sound.js | Background music |
| `ig.SoundManager` | sound.js | Audio management |

---

## Appendix B: File Structure

```
lib/
├── impact/
│   ├── impact.js          # Core engine, module system, class system
│   ├── game.js            # ig.Game base class
│   ├── entity.js          # ig.Entity base class
│   ├── entity-pool.js     # Entity pooling
│   ├── system.js          # Game loop, canvas
│   ├── input.js           # Input handling
│   ├── loader.js          # Resource loading
│   ├── image.js           # Image handling
│   ├── animation.js       # Animations
│   ├── map.js             # Base map class
│   ├── collision-map.js   # Collision detection
│   ├── background-map.js  # Background maps
│   ├── font.js            # Font rendering
│   ├── sound.js           # Audio system
│   ├── timer.js           # Timer utilities
│   └── debug/
│       ├── debug.js       # Debug module
│       ├── menu.js        # Debug menu
│       ├── graph-panel.js # Performance graph
│       ├── entities-panel.js # Entity debug
│       └── maps-panel.js  # Map debug
└── weltmeister/
    ├── weltmeister.js     # Level editor
    ├── edit-map.js        # Map editing
    ├── edit-entities.js   # Entity editing
    ├── tile-select.js     # Tile selection
    ├── undo.js            # Undo system
    └── ...
```

---

## Appendix C: Default Values

```javascript
// Entity defaults
size: {x: 16, y: 16}
offset: {x: 0, y: 0}
vel: {x: 0, y: 0}
accel: {x: 0, y: 0}
friction: {x: 0, y: 0}
maxVel: {x: 100, y: 100}
gravityFactor: 1
bounciness: 0
minBounceVelocity: 40

// System defaults
fps: 30
scale: 1
drawMode: SMOOTH

// Sound defaults
channels: 4
useWebAudio: true (if available)

// Collision defaults
type: TYPE.NONE
checkAgainst: TYPE.NONE
collides: COLLIDES.NEVER

// Timer defaults
timeScale: 1
maxStep: 0.05 (50ms)
```

---

**Document Version:** 1.0
**Impact.js Version:** 1.24
**Generated:** 2026-03-20
