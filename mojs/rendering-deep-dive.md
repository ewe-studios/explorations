---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src
explored_at: 2026-03-20
---

# mo.js Rendering System - Deep Dive

**Scope:** HTML element animation, SVG shape rendering, Canvas vs SVG, transform systems, stroke/fill rendering

---

## Table of Contents

1. [Rendering Architecture Overview](#1-rendering-architecture-overview)
2. [SVG Shape System](#2-svg-shape-system)
3. [Bit Base Class - SVG Primitive](#3-bit-base-class---svg-primitive)
4. [Shape Types and Implementations](#4-shape-types-and-implementations)
5. [HTML Element Animation](#5-html-element-animation)
6. [Transform System](#6-transform-system)
7. [Stroke and Fill Rendering](#7-stroke-and-fill-rendering)
8. [Motion Blur System](#8-motion-blur-system)
9. [Composite Layer Optimization](#9-composite-layer-optimization)
10. [Delta-Based Property Updates](#10-delta-based-property-updates)
11. [Performance Characteristics](#11-performance-characteristics)

---

## 1. Rendering Architecture Overview

### 1.1 Dual Rendering Systems

mo.js supports two distinct rendering targets:

```
┌─────────────────────────────────────────────────────────────────┐
│                    mo.js RENDERING                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────┐    ┌────────────────────────┐       │
│  │   SVG Shape System     │    │   HTML Element System  │       │
│  │   (Shape, Burst, etc.) │    │   (Html module)        │       │
│  │                        │    │                        │       │
│  │  - SVG ellipse, rect   │    │  - Any HTMLElement     │       │
│  │  - SVG path, line      │    │  - CSS properties      │       │
│  │  - Vector graphics     │    │  - Transform3D         │       │
│  │  - Stroke-based        │    │  - Background, border  │       │
│  │                        │    │                        │       │
│  │  File: shapes/bit.js   │    │  File: html.js         │       │
│  └────────────────────────┘    └────────────────────────┘       │
│                                                                  │
│  Both systems share:                                             │
│  - Tween engine for timing                                       │
│  - Delta system for interpolation                                │
│  - Easing functions for curves                                   │
│  - Timeline for composition                                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Rendering Pipeline

```
Animation Request
       │
       ▼
┌─────────────────┐
│ Module Creation │
│ (Shape/Html)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ _extendDefaults │
│ Parse options   │
│ Create deltas   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    _render()    │
│ Create DOM/SVG  │
│ Set initial     │
│ state           │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Tween.update() │
│ Calculate       │
│ easedProgress   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ _setProgress()  │
│ Apply deltas    │
│ to props        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    _draw()      │
│ Render props    │
│ to DOM/SVG      │
└─────────────────┘
```

### 1.3 Key Rendering Classes

| Class | File | Purpose |
|-------|------|---------|
| `Bit` | `shapes/bit.babel.js` | Base SVG shape primitive |
| `Shape` | `shape.babel.js` | Animated SVG shape wrapper |
| `Html` | `html.babel.js` | HTML element animator |
| `Burst` | `burst.babel.js` | Particle burst system |
| `ShapeSwirl` | `shape-swirl.babel.js` | Sinusoidal path follower |
| `MotionPath` | `motion-path.coffee` | Path-based motion |

---

## 2. SVG Shape System

### 2.1 SVG Canvas Creation

```javascript
// Bit class creates SVG canvas
_createSVGCanvas() {
    var p = this._props;

    // Create SVG element
    this._canvas = document.createElementNS(p.ns, 'svg');
    this.ns = 'http://www.w3.org/2000/svg';

    // Create the shape element
    this.el = document.createElementNS(p.ns, p.tag);
    this._canvas.appendChild(this.el);
}
```

### 2.2 SVG Canvas Styling

```javascript
_setCanvasSize() {
    const style = this._canvas.style;

    style.display = 'block';
    style.width = '100%';
    style.height = '100%';
    style.left = '0px';
    style.top = '0px';
}
```

### 2.3 Shape Element Types

```javascript
// shapesMap.coffee - Shape registry
shapesMap =
  'circle':   require('./circle')   // SVG <ellipse>
  'rect':     require('./rect')     // SVG <rect>
  'line':     require('./line')     // SVG <line>
  'cross':    require('./cross')    // Two SVG <line> elements
  'polygon':  require('./polygon')  // SVG <polygon>
  'equal':    require('./equal')    // Two SVG <rect> elements
  'zigzag':   require('./zigzag')   // SVG <polyline>
```

---

## 3. Bit Base Class - SVG Primitive

### 3.1 Bit Defaults

```javascript
_declareDefaults() {
    this._defaults = {
        'ns': 'http://www.w3.org/2000/svg',
        'tag': 'ellipse',        // Default SVG element type
        'parent': document.body, // Where to append

        // Size
        'radius': 50,
        'radiusX': null,         // Falls back to radius
        'radiusY': null,         // Falls back to radius

        // Stroke
        'stroke': 'hotpink',
        'stroke-width': 2,
        'stroke-opacity': 1,
        'stroke-linecap': '',
        'stroke-dasharray': '',
        'stroke-dashoffset': '',

        // Fill
        'fill': 'transparent',
        'fill-opacity': 1,

        // Canvas size
        'width': 0,
        'height': 0,
    };

    // Draw map - order of attribute application
    this._drawMap = [
        'stroke', 'stroke-width', 'stroke-opacity', 'stroke-dasharray',
        'fill', 'stroke-dashoffset', 'stroke-linecap', 'fill-opacity',
        'transform',
    ];
}
```

### 3.2 Draw Loop

```javascript
_draw() {
    // Get total length (for stroke-dash calculations)
    this._props.length = this._getLength();

    // Iterate through draw map
    var len = this._drawMapLength;
    while (len--) {
        var name = this._drawMap[len];

        // Special handling for dash properties
        switch (name) {
            case 'stroke-dasharray':
            case 'stroke-dashoffset':
                this.castStrokeDash(name);
        }

        // Set attribute if changed
        this._setAttrIfChanged(name, this._props[name]);
    }

    // Cache radius
    this._state.radius = this._props.radius;
}
```

### 3.3 State Caching for Performance

```javascript
_setAttrIfChanged(name, value) {
    // Only update if value changed
    if (this._state[name] !== value) {
        this.el.setAttribute(name, value);
        this._state[name] = value;  // Cache for next comparison
    }
}
```

### 3.4 Length Calculation

```javascript
_getLength() {
    var p = this._props;
    let len = 0;

    // Check if element has getTotalLength (paths)
    var isGetLength = !!(this.el && this.el.getTotalLength);

    if (isGetLength && this.el.getAttribute('d')) {
        // Path element - use native method
        len = this.el.getTotalLength();
    } else {
        // Circle/ellipse - approximate
        len = 2 * ((p.radiusX != null) ? p.radiusX : p.radius);
    }
    return len;
}
```

---

## 4. Shape Types and Implementations

### 4.1 Circle (Ellipse)

```coffeescript
# shapes/circle.coffee
class Circle extends Bit
  _declareDefaults: ->
    super()
    this._defaults.tag = 'ellipse'

  _draw: ->
    super()
    p = this._props
    rx = if p.radiusX? then p.radiusX else p.radius
    ry = if p.radiusY? then p.radiusY else p.radius

    # Set ellipse radii
    this.el.setAttribute 'rx', rx
    this.el.setAttribute 'ry', ry
```

### 4.2 Rectangle

```coffeescript
# shapes/rect.coffee
class Rect extends Bit
  _declareDefaults: ->
    super()
    this._defaults.tag = 'rect'
    this._defaults.radius = 0     # Corner radius
    this._defaults.radiusX = null
    this._defaults.radiusY = null

  _draw: ->
    super()
    p = this._props
    rx = if p.radiusX? then p.radiusX else p.radius
    ry = if p.radiusY? then p.radiusY else p.radius

    # Set rect position and size
    this.el.setAttribute 'x', -rx
    this.el.setAttribute 'y', -ry
    this.el.setAttribute 'width', rx * 2
    this.el.setAttribute 'height', ry * 2

    # Rounded corners
    this.el.setAttribute 'rx', p.rx or 0
    this.el.setAttribute 'ry', p.ry or 0
```

### 4.3 Line

```coffeescript
# shapes/line.coffee
class Line extends Bit
  _declareDefaults: ->
    super()
    this._defaults.tag = 'line'
    this._defaults.points = 2

  _draw: ->
    super()
    p = this._props
    r = p.radius

    # Set line endpoints
    this.el.setAttribute 'x1', -r
    this.el.setAttribute 'y1', 0
    this.el.setAttribute 'x2', r
    this.el.setAttribute 'y2', 0
```

### 4.4 Polygon

```coffeescript
# shapes/polygon.coffee
class Polygon extends Bit
  _declareDefaults: ->
    super()
    this._defaults.tag = 'polygon'
    this._defaults.points = 3  # Triangle by default

  _getPoints: (n, r) ->
    points = []
    angleStep = (Math.PI * 2) / n

    for i in [0...n]
      angle = i * angleStep - Math.PI / 2
      x = Math.cos(angle) * r
      y = Math.sin(angle) * r
      points.push "#{x},#{y}"

    points.join ' '

  _draw: ->
    super()
    p = this._props
    pointsString = this._getPoints p.points, p.radius
    this.el.setAttribute 'points', pointsString
```

### 4.5 Cross

```coffeescript
# shapes/cross.coffee
class Cross extends Bit
  _declareDefaults: ->
    super()
    this._defaults.tag = 'g'  # Group element
    this._defaults.innerRadius = 0.5  # Relative to radius

  _draw: ->
    super()
    p = this._props
    r = p.radius
    ir = p.innerRadius * r

    # Create or get line elements
    line1 = this.lines?[0]
    line2 = this.lines?[1]

    # First line (horizontal)
    line1.setAttribute 'x1', -r
    line1.setAttribute 'y1', 0
    line1.setAttribute 'x2', r
    line1.setAttribute 'y2', 0

    # Second line (vertical)
    line2.setAttribute 'x1', 0
    line2.setAttribute 'y1', -ir
    line2.setAttribute 'x2', 0
    line2.setAttribute 'y2', ir
```

---

## 5. HTML Element Animation

### 5.1 Html Module Defaults

```javascript
_declareDefaults() {
    this._defaults = {
        // Transforms
        x: 0, y: 0, z: 0,
        skewX: 0, skewY: 0,
        rotateX: 0, rotateY: 0, rotateZ: 0,
        scale: 1, scaleX: 1, scaleY: 1,

        // Behavior
        isSoftHide: true,       // Use opacity vs display
        isShowStart: true,      // Show on init
        isShowEnd: true,        // Show on complete
        isForce3d: false,       // Force 3D layer
        isRefreshState: true,   // Refresh on restart

        // Excluded from automatic drawing
        el: 1,
    };

    // Properties that trigger 3D layer
    this._3dProperties = ['rotateX', 'rotateY', 'z'];

    // Properties with array values
    this._arrayPropertyMap = {
        transformOrigin: 1,
        backgroundPosition: 1,
    };

    // Unitless properties
    this._numberPropertyMap = {
        opacity: 1, scale: 1, scaleX: 1, scaleY: 1,
        rotateX: 1, rotateY: 1, rotateZ: 1,
        skewX: 1, skewY: 1,
    };

    // Properties needing vendor prefix
    this._prefixPropertyMap = {
        transform: 1,
        transformOrigin: 1,
    };
}
```

### 5.2 Transform String Generation

```javascript
_drawTransform() {
    const p = this._props;

    // 2D transform (faster, no Z-axis)
    if (!this._is3d) {
        const string = `translate(${p.x}, ${p.y}) ` +
            `rotate(${p.rotateZ}deg) ` +
            `skew(${p.skewX}deg, ${p.skewY}deg) ` +
            `scale(${p.scaleX}, ${p.scaleY})`;

        this._setStyle('transform', string);

    // 3D transform (with Z-axis)
    } else {
        const string = `translate3d(${p.x}, ${p.y}, ${p.z}) ` +
            `rotateX(${p.rotateX}deg) ` +
            `rotateY(${p.rotateY}deg) ` +
            `rotateZ(${p.rotateZ}deg) ` +
            `skew(${p.skewX}deg, ${p.skewY}deg) ` +
            `scale(${p.scaleX}, ${p.scaleY})`;

        this._setStyle('transform', string);
    }
}
```

### 5.3 Style Application

```javascript
_setStyle(name, value) {
    // Only update if changed
    if (this._state[name] !== value) {
        var style = this._props.el.style;

        // Set style
        style[name] = value;

        // Apply vendor prefix if needed
        if (this._prefixPropertyMap[name]) {
            style[`${this._prefix}${name}`] = value;
        }

        // Cache value
        this._state[name] = value;
    }
}
```

### 5.4 CSS Property Parsing

```javascript
_parseOption(key, value) {
    super._parseOption(key, value);

    var parsed = this._props[key];

    // Convert array to space-separated string
    if (h.isArray(parsed)) {
        this._props[key] = this._arrToString(parsed);
    }
}

_arrToString(arr) {
    var string = '';
    for (var i = 0; i < arr.length; i++) {
        string += `${arr[i].string} `;
    }
    return string;
}
```

### 5.5 Unit Handling

```javascript
// Unit parsing in delta system
parseUnit(value) {
    if (typeof value === 'number') {
        return {
            unit: 'px',
            isStrict: false,
            value: value,
            string: value + 'px'
        };
    }

    // Extract unit from string
    const unit = value.match(/px|%|rem|em|vw|vh|deg|rad/gim)?.[0] || 'px';
    const amount = parseFloat(value);

    return {
        unit: unit,
        isStrict: true,
        value: amount,
        string: `${amount}${unit}`
    };
}
```

---

## 6. Transform System

### 6.1 2D vs 3D Detection

```javascript
// During defaults extension
_addDefaults(obj) {
    this._is3d = false;

    for (var key in this._defaults) {
        if (obj[key] == null) {
            // Handle scaleX/scaleY fallback to scale
            if (key === 'scaleX' || key === 'scaleY') {
                obj[key] = (obj['scale'] != null)
                    ? obj['scale'] : this._defaults['scale'];
            } else {
                obj[key] = this._defaults[key];
            }
        } else {
            // Check if 3D property was set
            if (this._3dProperties.indexOf(key) !== -1) {
                this._is3d = true;
            }
        }
    }

    // Force 3D if explicitly requested
    if (this._o.isForce3d) {
        this._is3d = true;
    }

    return obj;
}
```

### 6.2 Transform Origin

```javascript
// Can be defined as string or array of parsed units
origin: '50% 50%'  // Center

// Parsed as:
[
    { unit: '%', value: 50, string: '50%' },
    { unit: '%', value: 50, string: '50%' }
]

// Applied as:
_fillOrigin() {
    var p = this._props;
    var str = '';
    for (var i = 0; i < p.origin.length; i++) {
        str += `${p.origin[i].string} `;
    }
    return str;
}
```

### 6.3 Composite Layer Creation

```javascript
// MotionPath sets composite layer
setElPosition(x, y, p) {
    const rotate = this.rotate !== 0 ? `rotate(${this.rotate}deg)` : '';
    const isComposite = this.props.isCompositeLayer && h.is3d;
    const composite = isComposite ? 'translateZ(0)' : '';

    const transform = `translate(${x}px, ${y}px) ${rotate} ${composite}`;
    h.setPrefixedStyle(this.el, 'transform', transform);
}
```

---

## 7. Stroke and Fill Rendering

### 7.1 Stroke Properties

```javascript
// SVG stroke attributes
'stroke': 'hotpink',          // Color
'stroke-width': 2,            // Thickness in px
'stroke-opacity': 1,          // 0 to 1
'stroke-linecap': 'round',    // 'butt' | 'round' | 'square'
'stroke-dasharray': '5,10',   // Dash pattern
'stroke-dashoffset': 0,       // Starting offset
```

### 7.2 Dash Array Casting

```javascript
castStrokeDash(name) {
    var p = this._props;

    // Handle array of dash values
    if (h.isArray(p[name])) {
        var stroke = '';
        for (var i = 0; i < p[name].length; i++) {
            var dash = p[name][i];
            var cast = (dash.unit === '%')
                ? this.castPercent(dash.value)
                : dash.value;
            stroke += `${cast} `;
        }
        p[name] = (stroke === '0 ') ? '' : stroke;
        return p[name];
    }

    // Handle single value
    if (typeof p[name] === 'object') {
        stroke = (p[name].unit === '%')
            ? this.castPercent(p[name].value)
            : p[name].value;
        p[name] = (stroke === 0) ? '' : stroke;
    }
}

castPercent(percent) {
    return percent * (this._props.length / 100);
}
```

### 7.3 Fill Properties

```javascript
// SVG fill attributes
'fill': 'deeppink',           // Color
'fill-opacity': 1,            // 0 to 1
```

### 7.4 Color Interpolation

```javascript
// Delta system color calculation
_calcCurrent_color(delta, easedProgress, progress) {
    const start = delta.start;  // {r, g, b, a}
    const d = delta.delta;      // {r, g, b, a}

    if (!delta.curve) {
        // Linear interpolation
        r = parseInt(start.r + easedProgress * d.r, 10);
        g = parseInt(start.g + easedProgress * d.g, 10);
        b = parseInt(start.b + easedProgress * d.b, 10);
        a = parseFloat(start.a + easedProgress * d.a);
    } else {
        // Curve-based interpolation
        const cp = delta.curve(progress);
        r = parseInt(cp * (start.r + progress * d.r), 10);
        // ... for each channel
    }

    this._o.props[name] = `rgba(${r},${g},${b},${a})`;
}
```

---

## 8. Motion Blur System

### 8.1 SVG Filter Creation

```coffeescript
# motion-path.coffee
createFilter: ->
    div = document.createElement 'div'
    @filterID = "filter-#{h.getUniqID()}"

    div.innerHTML = """
        <svg id="svg-#{@filterID}"
             style="visibility:hidden; width:0px; height:0px">
            <filter id="#{@filterID}" y="-20" x="-20"
                    width="40" height="40">
                <feOffset id="blur-offset" in="SourceGraphic"
                          dx="0" dy="0" result="offset2"/>
                <feGaussianBlur id="blur" in="offset2"
                                stdDeviation="0,0" result="blur2"/>
                <feMerge>
                    <feMergeNode in="SourceGraphic"/>
                    <feMergeNode in="blur2"/>
                </feMerge>
            </filter>
        </svg>
    """

    svg = div.querySelector "#svg-#{@filterID}"
    @filter = svg.querySelector '#blur'
    @filterOffset = svg.querySelector '#blur-offset'

    document.body.insertBefore svg, document.body.firstChild
    @el.style['filter'] = "url(##{@filterID})"
```

### 8.2 Motion Blur Calculation

```coffeescript
makeMotionBlur:(x, y) ->
    # Calculate speed from position delta
    if !@prevCoords.x? or !@prevCoords.y?
        @speedX = 0
        @speedY = 0
    else
        dX = x - @prevCoords.x
        dY = y - @prevCoords.y
        @speedX = Math.abs(dX)
        @speedY = Math.abs(dY)

    # Calculate blur based on speed
    # 1px per 1ms is considered very fast
    @blurX = h.clamp (@speedX/16) * @props.motionBlur, 0, 1
    @blurY = h.clamp (@speedY/16) * @props.motionBlur, 0, 1

    # Apply blur
    @setBlur
        blur:
            x: 3 * @blurX * @blurAmount * Math.abs(coords.x)
            y: 3 * @blurY * @blurAmount * Math.abs(coords.y)
        offset:
            x: 3 * signX * @blurX * coords.x * @blurAmount
            y: 3 * signY * @blurY * coords.y * @blurAmount

    @prevCoords.x = x
    @prevCoords.y = y
```

### 8.3 Blur Application

```coffeescript
setBlur:(o) ->
    if !@isMotionBlurReset  # Safari/IE don't support motion blur
        @filter.setAttribute 'stdDeviation', "#{o.blur.x},#{o.blur.y}"
        @filterOffset.setAttribute 'dx', o.offset.x
        @filterOffset.setAttribute 'dy', o.offset.y
```

---

## 9. Composite Layer Optimization

### 9.1 Forcing 3D Layer

```javascript
// In Shape defaults
isForce3d: false  // Enable for GPU acceleration

// When enabled:
_setElStyles() {
    if (p.isForce3d) {
        let name = 'backface-visibility';
        style[name] = 'hidden';
        style[`${h.prefix.css}${name}`] = 'hidden';
    }
}
```

### 9.2 When to Use Composite Layers

| Scenario | Recommendation |
|----------|---------------|
| Many animated elements | ✓ Use `isForce3d: true` |
| Complex SVG paths | ✓ Use `isCompositeLayer: true` |
| Simple shape animations | ✗ Not necessary |
| Mobile devices | ✓ Use sparingly (memory cost) |
| Transform-only animations | ✓ Highly recommended |

### 9.3 Memory Trade-off

```
Composite Layer Benefits:
- GPU acceleration
- No repaint on transform
- Smoother animations

Composite Layer Costs:
- Video memory allocation
- Texture upload overhead
- Limited by GPU memory
```

---

## 10. Delta-Based Property Updates

### 10.1 Delta Types

```javascript
// Delta system handles multiple value types
const deltaTypes = {
    // Color: 'rgba(255,0,0)' → 'rgba(0,255,0)'
    'color': ColorDelta,

    // Number: 0 → 100
    'number': NumberDelta,

    // Unit: '0px' → '100%'
    'unit': UnitDelta,

    // Array: [0,0] → [100,100] (stroke-dasharray)
    'array': ArrayDelta,
};
```

### 10.2 Delta Creation

```javascript
_createDeltas(options) {
    this.deltas = new Deltas({
        options: options,
        props: this._props,
        arrayPropertyMap: this._arrayPropertyMap,
        numberPropertyMap: this._numberPropertyMap,
        customProps: this._customProps,
        callbacksContext: options.callbacksContext || this,
        isChained: !!this._o.prevChainModule,
    });

    // Link to timeline if chained
    if (this._o.prevChainModule) {
        this.timeline = this.deltas.timeline;
    }
}
```

### 10.3 Progress Application

```javascript
// In Module class
_setProgress(easedProgress, progress) {
    // Apply deltas to properties
    if (this.deltas) {
        this.deltas.render(easedProgress, progress);
    }

    // Draw updated properties
    this._draw(easedProgress);
}
```

---

## 11. Performance Characteristics

### 11.1 Rendering Cost Comparison

| System | Draw Cost | Memory | Best For |
|--------|-----------|--------|----------|
| SVG (Bit) | Medium | Low-Medium | Vector shapes, strokes |
| HTML (transform) | Low | Low | UI elements, boxes |
| Motion Blur | High | Medium | High-speed movement |
| Composite Layer | Very Low | High | Complex animations |

### 11.2 Optimization Strategies

### 1. Minimize DOM Creation

```javascript
// Share shapes across animations
const sharedShape = new mojs.Shape({
    shape: 'circle',
    radius: 50
});

// Reuse instead of creating new
sharedShape.tune({ radius: 100 }).then({ radius: 50 });
```

### 2. Use Transform Over Position

```javascript
// BAD: Causes layout recalculation
el.style.left = newX + 'px';

// GOOD: GPU accelerated
el.style.transform = `translateX(${newX}px)`;
```

### 3. Batch Property Updates

```javascript
// mo.js automatically batches via delta system
// All properties updated in single _draw() call
```

### 4. Limit Motion Blur Usage

```javascript
// Motion blur has high cost
// Use only for fast-moving elements
new mojs.MotionPath({
    motionBlur: 0.5,  // 0 = disabled, 1 = maximum
    // ...
});
```

### 11.3 Debug Rendering Stats

```javascript
// Track draw calls
let drawCount = 0;

// In _draw()
drawCount++;

// Reset each frame
requestAnimationFrame(() => {
    console.log('Draws this frame:', drawCount);
    drawCount = 0;
});
```
