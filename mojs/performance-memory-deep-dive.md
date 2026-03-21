---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src
explored_at: 2026-03-20
---

# mo.js Performance and Memory Management - Deep Dive

**Scope:** DOM batching, Render queue management, Attribute caching, GC considerations, Object pooling, Cleanup patterns, Memory leak prevention

---

## Table of Contents

1. [Performance Architecture Overview](#1-performance-architecture-overview)
2. [DOM Batching Strategies](#2-dom-batching-strategies)
3. [Attribute and Style Caching](#3-attribute-and-style-caching)
4. [Render Queue Management](#4-render-queue-management)
5. [Object Pooling in Burst](#5-object-pooling-in-burst)
6. [Memory Management Patterns](#6-memory-management-patterns)
7. [Cleanup and Destroy Methods](#7-cleanup-and-destroy-methods)
8. [Reference Clearing](#8-reference-clearing)
9. [Event Listener Cleanup](#9-event-listener-cleanup)
10. [GC Considerations](#10-gc-considerations)
11. [Retina/High-DPI Optimization](#11-retinahigh-dpi-optimization)
12. [Visibility Handling](#12-visibility-handling)

---

## 1. Performance Architecture Overview

### 1.1 Performance Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                  mo.js PERFORMANCE STACK                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              ANIMATION LOOP OPTIMIZATION                 │    │
│  │  - Single RAF loop (Tweener)                            │    │
│  │  - Reverse iteration for removal                        │    │
│  │  - Visibility pause                                     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│         ┌────────────────────┼────────────────────┐             │
│         ▼                    ▼                    ▼             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │    DELTA    │     │   RENDER    │     │    MEMORY   │       │
│  │  OPTIMIZATION│    │  OPTIMIZATION│    │  OPTIMIZATION│      │
│  │  - Pre-calc │    │  - Caching  │    │  - Cleanup  │       │
│  │  - Float32  │    │  - Batch    │    │  - Pooling  │       │
│  │  - Samples  │    │  - Compare  │    │  - Clear    │       │
│  └─────────────┘     └─────────────┘     └─────────────┘       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Key Performance Principles

1. **Compute Once, Use Many Times:** All expensive calculations happen during initialization
2. **Cache Everything:** DOM reads/writes minimized through caching
3. **Batch Operations:** Group DOM changes to reduce reflows
4. **Pool Expensive Objects:** Reuse rather than recreate
5. **Clean Up Explicitly:** Prevent memory leaks in long-running apps

---

## 2. DOM Batching Strategies

### 2.1 Single Draw Loop

```javascript
// In Bit class (SVG shapes)
_draw() {
  // Calculate length once
  this._props.length = this._getLength();

  // Batch all attribute updates in single loop
  var len = this._drawMapLength;
  while (len--) {
    var name = this._drawMap[len];

    // Special handling for dash properties
    switch (name) {
      case 'stroke-dasharray':
      case 'stroke-dashoffset':
        this.castStrokeDash(name);
    }

    // Set attribute if changed (with caching)
    this._setAttrIfChanged(name, this._props[name]);
  }

  // Cache radius
  this._state.radius = this._props.radius;
}
```

**Draw Map:**
```javascript
_drawMap = [
  'stroke', 'stroke-width', 'stroke-opacity', 'stroke-dasharray',
  'fill', 'stroke-dashoffset', 'stroke-linecap', 'fill-opacity',
  'transform'
];
```

### 2.2 Shape Property Batching

```javascript
// In Shape class
_draw() {
  var p = this._props,
      bP = this.shapeModule._props;

  // Batch all property transfers
  bP.rx = p.rx;
  bP.ry = p.ry;
  bP.stroke = p.stroke;
  bP['stroke-width'] = p.strokeWidth;
  bP['stroke-opacity'] = p.strokeOpacity;
  bP['stroke-dasharray'] = p.strokeDasharray;
  bP['stroke-dashoffset'] = p.strokeDashoffset;
  bP['stroke-linecap'] = p.strokeLinecap;
  bP['fill'] = p.fill;
  bP['fill-opacity'] = p.fillOpacity;
  bP.radius = p.radius;
  bP.radiusX = p.radiusX;
  bP.radiusY = p.radiusY;
  bP.points = p.points;

  // Single draw call
  this.shapeModule._draw();
  this._drawEl();
}
```

### 2.3 HTML Style Batching

```javascript
// In Html class
_draw() {
  const p = this._props;

  // Batch style applications
  for (var i = 0; i < this._drawProps.length; i++) {
    var name = this._drawProps[i];
    this._setStyle(name, p[name]);
  }

  // Draw transforms (batched into single property)
  this._drawTransform();

  // Custom draw callback
  this._customDraw && this._customDraw(this._props.el, this._props);
}

_drawTransform() {
  const p = this._props;
  const string = (!this._is3d)
    ? `translate(${p.x}, ${p.y}) rotate(${p.rotateZ}deg) skew(${p.skewX}deg, ${p.skewY}deg) scale(${p.scaleX}, ${p.scaleY})`
    : `translate3d(${p.x}, ${p.y}, ${p.z}) rotateX(${p.rotateX}deg) rotateY(${p.rotateY}deg) rotateZ(${p.rotateZ}deg) skew(${p.skewX}deg, ${p.skewY}deg) scale(${p.scaleX}, ${p.scaleY})`;

  // Single transform write
  this._setStyle('transform', string);
}
```

---

## 3. Attribute and Style Caching

### 3.1 SVG Attribute Caching

```javascript
// In Bit class
_state: {}  // Cache for attribute values

_setAttrIfChanged(name, value) {
  // Only update if value changed from cached value
  if (this._state[name] !== value) {
    this.el.setAttribute(name, value);
    this._state[name] = value;  // Update cache
  }
}
```

**Performance Impact:**
```
Without caching:
  Frame 1: setAttribute('rx', 50)   // DOM write
  Frame 2: setAttribute('rx', 50)   // DOM write (redundant)
  Frame 3: setAttribute('rx', 50)   // DOM write (redundant)

With caching:
  Frame 1: setAttribute('rx', 50)   // DOM write, cache = 50
  Frame 2: 50 === 50, skip          // No DOM write
  Frame 3: 50 === 50, skip          // No DOM write
```

### 3.2 Style Caching

```javascript
// In Html class
_setStyle(name, value) {
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

### 3.3 State Initialization

```javascript
// In Bit class
_render() {
  if (this._isRendered) { return; }
  this._isRendered = true;

  this._createSVGCanvas();
  this._setCanvasSize();
  this._props.parent.appendChild(this._canvas);

  // Initialize state cache
  this._state = {};
}
```

---

## 4. Render Queue Management

### 4.1 Single RAF Loop (Tweener)

```javascript
// In Tweener class
class Tweener {
  constructor() {
    this.tweens = [];
    this._isRunning = false;
  }

  _loop() {
    if (!this._isRunning) { return false; }

    // Update all tweens with current time
    this._update(window.performance.now());

    // Stop loop if no tweens
    if (!this.tweens.length) {
      return this._isRunning = false;
    }

    requestAnimationFrame(this._loop);
    return this;
  }

  _update(time) {
    var i = this.tweens.length;

    // Reverse iteration for efficient removal
    while (i--) {
      var tween = this.tweens[i];

      // Update tween, remove if complete
      if (tween && tween._update(time) === true) {
        this.remove(tween);
        tween._onTweenerFinish();
        tween._prevTime = undefined;
      }
    }
  }
}
```

### 4.2 Reverse Iteration Pattern

```javascript
// WHY reverse iteration?

// Forward iteration with removal (PROBLEMATIC)
for (var i = 0; i < tweens.length; i++) {
  if (tween._update(time) === true) {
    tweens.splice(i, 1);  // Shifts all subsequent indices!
    i--;  // Must manually adjust index
  }
}

// Reverse iteration (EFFICIENT)
var i = tweens.length;
while (i--) {
  if (tween._update(time) === true) {
    tweens.splice(i, 1);  // No index shift for remaining items
  }
}
```

**Benefits:**
- No length property lookup each iteration
- Removal doesn't affect remaining indices
- Slightly faster than forward for loop

### 4.3 Tween Deduplication

```javascript
add(tween) {
  // Don't add if already running
  if (tween._isRunning) { return; }

  tween._isRunning = true;
  this.tweens.push(tween);
  this._startLoop();
}

_startLoop() {
  if (this._isRunning) { return; }
  this._isRunning = true;
  requestAnimationFrame(this._loop.bind(this));
}
```

---

## 5. Object Pooling in Burst

### 5.1 Burst Children Pattern

While mo.js doesn't implement explicit object pooling, the Burst system creates children efficiently:

```javascript
// In Burst class
_createSwirls() {
  this._swirls = [];
  this._packs = [];

  for (var i = 0; i < this._props.count; i++) {
    var options = this._getChildOption(this._o, i);

    // Add burst-specific properties
    this._addBurstProperties(options, i);

    // Create ShapeSwirl for each child
    var swirl = new ShapeSwirl(options);
    this._swirls.push(swirl);

    // Add to timeline
    this.timeline.append(swirl);
  }
}
```

### 5.2 Child Option Caching

```javascript
_getChildOption(o, i) {
  // Cache child options if provided as function
  if (typeof o.children === 'function') {
    return o.children(i);
  }

  // Clone to prevent reference issues
  return h.cloneObj(o.children);
}
```

### 5.3 Memory-Efficient Child Creation

```javascript
// Batch child creation during initialization
//而不是 each frame
_createSwirls() {
  // Create all children upfront
  // Reuse options structure where possible
  // Share tween timeline across all children
}
```

---

## 6. Memory Management Patterns

### 6.1 State Object Pattern

```javascript
// Each module maintains its own state
_vars() {
  this._progress = 0;
  this._strokeDasharrayBuffer = [];
  this._isShown = false;
  this._isRendered = false;
  this._wasStarted = false;
  this._wasCompleted = false;
}
```

### 6.2 Options Cloning

```javascript
// In Module constructor
constructor(o = {}) {
  this._o = o;  // Reference to original options

  // Clone for internal use to prevent external mutation
  this._props = {};
  this._deltas = {};
}

// In Thenable
_mergeThenOptions(start, end) {
  var o = {};  // New object for merged options
  this._mergeStartLoop(o, start);
  this._mergeEndLoop(o, start, end);
  this._history.push(o);  // Store in history
  return o;
}
```

### 6.3 Array Reuse

```javascript
// In Module._calcCurrentProps
if (value.type === 'array') {
  var arr;

  // Reuse existing array if available
  if (h.isArray(this._props[key])) {
    arr = this._props[key];
    arr.length = 0;  // Clear without creating new array
  } else {
    arr = [];
  }

  // Populate existing array
  for (var i = 0; i < value.delta.length; i++) {
    // ... push to arr
  }

  this._props[key] = arr;
}
```

**GC Benefit:** Reusing arrays reduces garbage collector pressure

---

## 7. Cleanup and Destroy Methods

### 7.1 Visibility Cleanup

```javascript
// In Tweener class
_onVisibilityChange() {
  if (document[this._visibilityHidden]) {
    // Tab hidden - save and pause
    this._savePlayingTweens();
    for (const t of this._savedTweens) {
      t.pause();
    }
  } else {
    // Tab visible - restore and resume
    for (const t of this._savedTweens) {
      t.resume();
    }
  }
}

_savePlayingTweens() {
  this._savedTweens = [];
  for (var i = 0; i < this.tweens.length; i++) {
    if (this.tweens[i]._state === 'play') {
      this._savedTweens.push(this.tweens[i]);
    }
  }
}
```

### 7.2 Resize Handler Cleanup

```javascript
// In vendor/resize.coffee
destroy: ->
  clearInterval @interval
  @interval = null
  window.isAnyResizeEventInited = false

  # Restore original prototype methods
  for proto in @allowedProtos
    if !proto::? then continue

    if proto::addEventListener
      proto::addEventListener = Element::addEventListener
    else if proto::attachEvent
      proto::attachEvent = Element::attachEvent

    if proto::removeEventListener
      proto::removeEventListener = Element::removeEventListener
    else if proto::detachEvent
      proto::detachEvent = Element::detachEvent
```

### 7.3 Tween Cleanup

```javascript
// In Tween class
_onTweenerFinish() {
  // Called when tween completes and is removed from Tweener
  this._isRunning = false;

  // Optionally clear references
  // this._props = null;
  // this._o = null;
}
```

---

## 8. Reference Clearing

### 8.1 Timeline Reference Management

```javascript
// In Timeline class
add(...args) {
  this._pushTimelineArray(args);
  this._calcDimensions();
  return this;
}

_pushTimelineArray(array) {
  for (var tm of array) {
    this._timelines.push(tm);
    // Timeline holds reference to children
    // Children do NOT hold reference back to parent (prevents cycles)
  }
}
```

### 8.2 Module Chain References

```javascript
// In Thenable class
then(o) {
  var module = new this.constructor(merged);

  // Master reference (parent -> child)
  module._masterModule = this;

  // Add to modules array (for iteration)
  this._modules.push(module);

  // Add to timeline (for playback)
  this.timeline.append(module);

  return this;
}
```

**Memory Consideration:**
- Parent holds reference to children via `_modules`
- Children hold reference to parent via `_masterModule`
- This creates a reference cycle - must be cleaned up explicitly if needed

### 8.3 Delta Reference Pattern

```javascript
// Deltas hold references to props
constructor(o = {}) {
  this._o = o;
  this._props = o.props;  // Reference, not copy

  // Deltas reference the same props object
  this._mainDeltas = [];
  this._childDeltas = [];
}
```

---

## 9. Event Listener Cleanup

### 9.1 Visibility Event

```javascript
// In Tweener initialization
constructor() {
  this._visibilityHidden = this._getVisibilityHidden();

  if (this._visibilityHidden) {
    document.addEventListener(
      'visibilitychange',
      this._onVisibilityChange.bind(this)
    );
  }
}

_getVisibilityHidden() {
  if (typeof document.hidden !== 'undefined') {
    return 'hidden';
  }
  if (typeof document.mozHidden !== 'undefined') {
    return 'mozHidden';
  }
  if (typeof document.msHidden !== 'undefined') {
    return 'msHidden';
  }
  if (typeof document.webkitHidden !== 'undefined') {
    return 'webkitHidden';
  }
}
```

### 9.2 Resize Event (vendor/resize.coffee)

```coffeescript
handleResize: (args) ->
  el = args.that

  if !@timerElements[el.tagName.toLowerCase()]
    # Create iframe for resize detection
    iframe = document.createElement 'iframe'
    el.appendChild iframe

    # Style iframe
    iframe.style.width = '100%'
    iframe.style.height = '100%'
    iframe.style.position = 'absolute'
    iframe.style.zIndex = -999
    iframe.style.opacity = 0

    # Listen for iframe resize
    iframe.contentWindow?.onresize = (e) => @dispatchEvent el

    el.iframe = iframe  # Store reference for cleanup
  else
    @initTimer(el)

  el.isAnyResizeEventInited = true
```

**Cleanup on removeEventListener:**
```coffeescript
wrappedRemover = ->
  @isAnyResizeEventInited = false
  @iframe and @removeChild @iframe  # Remove and dereference iframe
  remover.apply(@, arguments)
```

---

## 10. GC Considerations

### 10.1 Float32Array for Samples

```coffeescript
# In bezier-easing.coffee
float32ArraySupported = !!Float32Array
kSplineTableSize = 11

mSampleValues = if !float32ArraySupported
  then new Array(kSplineTableSize)    # 8 bytes per element
  else new Float32Array(kSplineTableSize)  # 4 bytes per element
```

**Memory Savings:**
```
Array:         11 elements × 8 bytes = 88 bytes + object overhead
Float32Array:  11 elements × 4 bytes = 44 bytes (contiguous memory)
Savings: ~50% reduction
```

### 10.2 Inline Function Avoidance

```javascript
// BAD: Creates new function every frame
_draw() {
  this.items.forEach(item => {
    item.update();
  });
}

// GOOD: Reuse function reference
_draw() {
  for (var i = 0; i < this.items.length; i++) {
    this.items[i].update();
  }
}
```

### 10.3 Closure Memory

```javascript
// In Delta class
constructor(o = {}) {
  this._o = o;

  // Callback stored on tween
  this._createTween(o.tweenOptions);
}

_createTween(o = {}) {
  var it = this;

  // Closures capture 'this' context
  o.callbackOverrides = {
    onUpdate(ep, p) { it._calcCurrentProps(ep, p); },
  };

  this.tween = new Tween(o);
}
```

**Consideration:** Closures retain references to outer scope - be mindful of what they capture

---

## 11. Retina/High-DPI Optimization

### 11.1 Numeric Precision

All mo.js delta calculations use native JavaScript numbers (IEEE 754 double-precision):

```javascript
// No rounding until final render
delta = {
  start: 0,
  end: 100,
  delta: 100
};

// Full precision during interpolation
current = start + easedProgress * delta;  // e.g., 50.123456789

// Rounded only when applied to DOM
element.setAttribute('rx', Math.round(current));
```

**Benefits:**
- 53 bits of precision (~15-17 decimal digits)
- No cumulative rounding errors
- Seamless high-DPI support

### 11.2 Transform-Based Animation

```javascript
// Use transforms instead of position properties
// BAD: Triggers layout recalculation
el.style.left = newX + 'px';

// GOOD: GPU accelerated, no layout
el.style.transform = `translateX(${newX}px)`;
```

### 11.3 Composite Layer Creation

```javascript
// In MotionPath class
setElPosition(x, y, p) {
  const rotate = this.rotate !== 0 ? `rotate(${this.rotate}deg)` : '';

  // Force composite layer for GPU acceleration
  const isComposite = this.props.isCompositeLayer && h.is3d;
  const composite = isComposite ? 'translateZ(0)' : '';

  const transform = `translate(${x}px, ${y}px) ${rotate} ${composite}`;
  h.setPrefixedStyle(this.el, 'transform', transform);
}
```

**When to Use Composite Layers:**

| Scenario | Recommendation |
|----------|---------------|
| Many animated elements | ✓ `isCompositeLayer: true` |
| Complex SVG paths | ✓ `isCompositeLayer: true` |
| Simple shape animations | ✗ Not necessary |
| Mobile devices | ⚠ Use sparingly (memory cost) |
| Transform-only animations | ✓ Highly recommended |

---

## 12. Visibility Handling

### 12.1 Tab Visibility Detection

```javascript
// In Tweener
_getVisibilityHidden() {
  if (typeof document.hidden !== 'undefined') {
    return 'hidden';  // Standard
  }
  if (typeof document.mozHidden !== 'undefined') {
    return 'mozHidden';  // Firefox
  }
  if (typeof document.msHidden !== 'undefined') {
    return 'msHidden';  // IE
  }
  if (typeof document.webkitHidden !== 'undefined') {
    return 'webkitHidden';  // Chrome/Safari
  }
  return undefined;
}
```

### 12.2 Automatic Pause/Resume

```javascript
_onVisibilityChange() {
  if (document[this._visibilityHidden]) {
    // Tab hidden - pause all playing tweens
    this._savePlayingTweens();
    for (const t of this._savedTweens) {
      t.pause();
    }
  } else {
    // Tab visible - resume all
    for (const t of this._savedTweens) {
      t.resume();
    }
  }
}
```

**Benefits:**
- Prevents wasted CPU cycles on hidden tabs
- Preserves animation state for seamless resume
- Extends battery life on mobile devices

### 12.3 Performance Impact

```
Without visibility handling:
- Animation continues at 60fps even when tab hidden
- Battery drain continues
- CPU usage remains high

With visibility handling:
- Animation pauses immediately when tab hidden
- Battery preserved
- CPU usage drops to near-zero
- Seamless resume when tab becomes visible
```

---

## Summary

mo.js implements several performance and memory optimization strategies:

### Performance Optimizations
1. **DOM Batching:** Single draw loop, grouped property updates
2. **Attribute Caching:** `_setAttrIfChanged` prevents redundant DOM writes
3. **Render Queue:** Single RAF loop with reverse iteration
4. **Transform Usage:** GPU-accelerated animations via `translate3d`

### Memory Optimizations
1. **Float32Array:** 50% memory reduction for sample tables
2. **Array Reuse:** Clear and reuse arrays instead of creating new
3. **Reference Management:** Careful parent-child reference patterns
4. **Visibility Handling:** Pause animations when tab hidden

### Cleanup Patterns
1. **Event Listener Cleanup:** Proper removal on destroy
2. **Resize Handler Cleanup:** Restore original prototypes
3. **Reference Clearing:** Explicit cleanup for long-running apps

### Best Practices for Users
1. Use `isCompositeLayer: true` for complex animations
2. Use `isSoftHide: true` (default) for show/hide
3. Reuse shape instances across animations
4. Call `.stop()` or `.pause()` on completed animations in long-running apps
5. Be mindful of Burst count - each child is a full ShapeSwirl instance

These optimizations enable mo.js to handle complex motion graphics at 60fps while maintaining reasonable memory usage even in long-running applications.
