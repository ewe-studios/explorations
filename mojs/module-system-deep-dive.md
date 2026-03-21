---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src
explored_at: 2026-03-20
---

# mo.js Module System - Deep Dive

**Scope:** Module base class, Tunable pattern, Thenable chaining, Options parsing, Defaults inheritance, History transformation

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Module Base Class](#2-module-base-class)
3. [Defaults Inheritance System](#3-defaults-inheritance-system)
4. [Options Parsing Pipeline](#4-options-parsing-pipeline)
5. [Delta Detection and Creation](#5-delta-detection-and-creation)
6. [Thenable Pattern](#6-thenable-pattern)
7. [Tunable Pattern](#7-tunable-pattern)
8. [History Transformation](#8-history-transformation)
9. [Extensibility Patterns](#9-extensibility-patterns)
10. [Property Caching and Validation](#10-property-caching-and-validation)

---

## 1. Architecture Overview

### 1.1 Inheritance Hierarchy

```
┌─────────────────────────────────────────────────────────────────┐
│                    mo.js MODULE HIERARCHY                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Tweenable (tween/tweenable.babel.js)                           │
│    │                                                             │
│    └───▶ Thenable (src/thenable.babel.js)                       │
│            │                                                     │
│            └───▶ Tunable (src/tunable.babel.js)                 │
│                    │                                             │
│                    ├───▶ Burst (src/burst.babel.js)             │
│                    ├───▶ Shape (src/shape.babel.js)             │
│                    ├───▶ ShapeSwirl (src/shape-swirl.babel.js)  │
│                    ├───▶ Html (src/html.babel.js)               │
│                    └───▶ MotionPath (src/motion-path.coffee)    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Class Responsibilities

| Class | File | Purpose |
|-------|------|---------|
| `Module` | `src/module.babel.js` | Base class with defaults, options parsing, delta calculation |
| `Thenable` | `src/thenable.babel.js` | `.then()` chaining for animation sequences |
| `Tunable` | `src/tunable.babel.js` | `.tune()` for runtime option modification |
| `Tweenable` | `tween/tweenable.babel.js` | Tween integration (extends Tween) |

### 1.3 Data Flow

```
User Options
     │
     ▼
┌─────────────────┐
│ Module()        │
│ constructor     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ _declareDefaults│
│ Set defaults    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ _extendDefaults │
│ Merge + Parse   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ _parseOption    │
│ Delta detection │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ _getDelta       │
│ Create delta    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ _render         │
│ DOM creation    │
└─────────────────┘
```

---

## 2. Module Base Class

### 2.1 Constructor Flow

```javascript
class Module {
  constructor(o = {}) {
    this._o = o;                    // Store original options
    this._index = this._o.index || 0;

    // Property maps for parsing
    this._arrayPropertyMap = {
      strokeDashoffset: 1,
      strokeDasharray: 1,
      origin: 1,
    };

    this._skipPropsDelta = {
      timeline: 1,
      prevChainModule: 1,
      callbacksContext: 1,
    };

    this._declareDefaults();        // Step 1: Declare defaults
    this._extendDefaults();         // Step 2: Merge with options
    this._vars();                   // Step 3: Initialize variables
    this._render();                 // Step 4: Render DOM
  }
}
```

**Key Design Decisions:**

1. **Options stored raw:** `this._o` keeps original user options unchanged
2. **Parsed props separate:** `this._props` contains processed values
3. **Index for stagger:** `this._index` enables stagger calculations per instance
4. **Property maps:** Define which properties need special parsing

### 2.2 Property Assignment

```javascript
_setProp(attr, value) {
  if (typeof attr === 'object') {
    // Object syntax: _setProp({x: 100, y: 200})
    for (var key in attr) {
      this._assignProp(key, attr[key]);
    }
  } else {
    // Single property: _setProp('x', 100)
    this._assignProp(attr, value);
  }
}

_assignProp(key, value) {
  this._props[key] = value;
}
```

**Why two methods?**
- `_setProp`: Flexible API for both single and bulk updates
- `_assignProp`: Single point for property assignment (override point for subclasses)

### 2.3 Show/Hide System

```javascript
_show() {
  var p = this._props;
  if (!this.el) { return; }

  if (p.isSoftHide) {
    // Use transform for better performance
    this._showByTransform();
  } else {
    // Use display property
    this.el.style.display = 'block';
  }
  this._isShown = true;
}

_hide() {
  if (!this.el) { return; }

  if (this._props.isSoftHide) {
    // Scale to 0 instead of display:none
    h.setPrefixedStyle(this.el, 'transform', 'scale(0)');
  } else {
    this.el.style.display = 'none';
  }
  this._isShown = false;
}
```

**`isSoftHide` Pattern:**
- `true` (default): Uses `transform: scale(0)` - GPU accelerated, maintains layout
- `false`: Uses `display: none` - Removes from layout, triggers reflow

---

## 3. Defaults Inheritance System

### 3.1 Default Declaration

```javascript
_declareDefaults() {
  this._defaults = { };
}
```

**Override Pattern in Subclasses:**

```javascript
// Shape class
_declareDefaults() {
  this._defaults = {
    // Delta colors
    stroke: 'transparent',
    fill: 'deeppink',

    // Delta numbers
    strokeOpacity: 1,
    strokeWidth: 2,
    fillOpacity: 1,
    rotate: 0,
    scale: 1,
    opacity: 1,
    points: 3,
    radius: 50,

    // Delta units
    left: '50%',
    top: '50%',
    x: 0,
    y: 0,

    // Non-tweenable
    shape: 'circle',
    parent: document.body,
    isShowStart: true,
    isSoftHide: true,
  };
}
```

### 3.2 Defaults Extension

```javascript
_extendDefaults() {
  this._props = {};
  this._deltas = {};

  for (var key in this._defaults) {
    // Get value from options or fallback to default
    var value = (this._o[key] != null)
      ? this._o[key]
      : this._defaults[key];

    // Parse the option (delta detection, unit parsing, etc.)
    this._parseOption(key, value);
  }
}
```

**Key Behaviors:**

1. **Every default is processed:** Loop ensures all defaults get parsed
2. **Options take precedence:** User options override defaults
3. **Delta tracking:** `_deltas` collects animatable properties

### 3.3 Variable Initialization

```javascript
_vars() {
  this._progress = 0;
  this._strokeDasharrayBuffer = [];
}
```

**Purpose:**
- Initialize instance-specific state
- Separate from defaults (which are static)
- Called after defaults, before render

---

## 4. Options Parsing Pipeline

### 4.1 Parse Option Method

```javascript
_parseOption(name, value) {
  // Check if it's a delta property
  if (this._isDelta(value) && !this._skipPropsDelta[name]) {
    this._getDelta(name, value);
    var deltaEnd = h.getDeltaEnd(value);
    return this._assignProp(name, this._parseProperty(name, deltaEnd));
  }

  // Regular property parsing
  this._assignProp(name, this._parseProperty(name, value));
}
```

**Flow:**
1. Check if value is delta (e.g., `{0: 100}`)
2. If delta, skip tween properties (timeline, callbacksContext)
3. Extract delta and store in `_deltas`
4. Set initial prop value to delta end value

### 4.2 Property Parsing Chain

```javascript
_parseProperty(name, value) {
  // Parse HTMLElement in parent option
  if (name === 'parent') { return h.parseEl(value); }

  // Parse stagger, rand and position
  value = this._parsePreArrayProperty(name, value);

  // Parse numeric/percent values for strokeDash.. properties
  return this._parseStrokeDashOption(name, value);
}
```

### 4.3 Pre-Array Parsing

```javascript
_parsePreArrayProperty(name, value) {
  // Parse stagger and rand string values
  value = this._parseOptionString(value);

  // Parse units for position properties
  return this._parsePositionOption(name, value);
}
```

### 4.4 String Option Parsing

```javascript
_parseOptionString(value) {
  if (typeof value === 'string') {
    if (value.match(/stagger/)) {
      value = h.parseStagger(value, this._index);
    }
  }

  if (typeof value === 'string') {
    if (value.match(/rand/)) {
      value = h.parseRand(value);
    }
  }
  return value;
}
```

**Example Patterns:**

```javascript
// Rand syntax
new mojs.Shape({
  radius: 'rand(10, 50)'  // Random value between 10-50
});

// Stagger syntax
new mojs.Shape({
  delay: 'stagger(50)',   // 0, 50, 100, 150... based on index
  x: 'stagger(20, 10)'    // Start at 10, then +20 each
});
```

### 4.5 Position Option Parsing

```javascript
_parsePositionOption(key, value) {
  if (h.unitOptionMap[key]) {
    value = h.parseUnit(value).string;
  }
  return value;
}

// h.unitOptionMap includes:
// left, top, x, y, rx, ry
```

### 4.6 Stroke Dash Parsing

```javascript
_parseStrokeDashOption(key, value) {
  var result = value;

  if (this._arrayPropertyMap[key]) {
    result = [];
    switch (typeof value) {
      case 'number':
        result.push(h.parseUnit(value));
        break;
      case 'string':
        var array = value.split(' ');
        for (var i = 0; i < array.length; i++) {
          result.push(h.parseUnit(array[i]));
        }
        break;
    }
  }
  return result;
}
```

**Usage:**
```javascript
// Single value
strokeDasharray: 10       // → [{value: 10, unit: 'px'}]

// String with units
strokeDasharray: '5 10'   // → [{value: 5, unit: 'px'}, {value: 10, unit: 'px'}]

// Percent values
strokeDasharray: '50% 25%' // → [{value: 50, unit: '%'}, {value: 25, unit: '%'}]
```

---

## 5. Delta Detection and Creation

### 5.1 Delta Detection

```javascript
_isDelta(optionsValue) {
  var isObject = h.isObject(optionsValue);
  isObject = isObject && !optionsValue.unit;  // Exclude already parsed units
  return !(!isObject || h.isArray(optionsValue) || h.isDOM(optionsValue));
}
```

**What qualifies as delta:**

```javascript
// These ARE deltas (objects without unit property)
{ 0: 100 }           // Number delta
{ 'red': 'blue' }    // Color delta
{ '0px': '100%' }    // Unit delta

// These are NOT deltas
50                   // Plain number
'rand(10, 50)'       // String (parsed before delta check)
{ unit: 'px' }       // Already parsed unit
[1, 2, 3]           // Array
document.querySelector('#el')  // DOM element
```

### 5.2 Delta Creation

```javascript
_getDelta(key, optionsValue) {
  // Warning for left/top (performance recommendation)
  if ((key === 'left' || key === 'top') && !this._o.ctx) {
    h.warn(`Consider to animate x/y properties instead of left/top,
      as it would be much more performant`, optionsValue);
  }

  // Skip delta calculation for listed properties
  if (this._skipPropsDelta && this._skipPropsDelta[key]) { return; }

  // Parse delta using helpers
  delta = h.parseDelta(key, optionsValue, this._index);

  // Save delta if successfully parsed
  if (delta.type != null) { this._deltas[key] = delta; }

  // Extract end value
  var deltaEnd = (typeof delta.end === 'object')
    ? (delta.end.value === 0) ? 0 : delta.end.string
    : delta.end;

  // Set props to end value
  this._props[key] = deltaEnd;
}
```

**Delta Structure:**

```javascript
// Number delta
{
  type: 'number',
  name: 'opacity',
  start: 1,
  end: 0,
  delta: -1,
  easing: [easingFunction],
  curve: [curveFunction]
}

// Color delta
{
  type: 'color',
  name: 'fill',
  start: {r: 255, g: 0, b: 0, a: 1},
  end: {r: 0, g: 0, b: 255, a: 1},
  delta: {r: -255, g: 0, b: 255, a: 0}
}

// Unit delta
{
  type: 'unit',
  name: 'x',
  start: {value: 0, unit: 'px', isStrict: false, string: '0px'},
  end: {value: 100, unit: 'px', isStrict: false, string: '100px'},
  delta: 100
}
```

---

## 6. Thenable Pattern

### 6.1 Then Method

```javascript
then(o) {
  // Return if nothing was passed
  if ((o == null) || !Object.keys(o).length) { return 1; }

  // Merge then options with current history
  var prevRecord = this._history[this._history.length - 1],
    merged = this._mergeThenOptions(prevRecord, o);

  this._resetMergedFlags(merged);

  // Create submodule of same type
  var module = new this.constructor(merged);

  // Set master module reference
  module._masterModule = this;

  // Save to modules array
  this._modules.push(module);

  // Add module's tween to master timeline
  this.timeline.append(module);

  return this;
}
```

**Chain Creation:**

```javascript
const shape = new mojs.Shape({
  shape: 'circle',
  radius: 50
})
.then({ radius: 100, duration: 500 })    // Step 1
.then({ radius: 50, duration: 500 })     // Step 2
.then({ scale: 0, duration: 300 });      // Step 3

shape.play();
```

### 6.2 Flag Resetting

```javascript
_resetMergedFlags(obj) {
  // Submodule without timeline for perf
  obj.isTimelineLess = true;

  // Reset show flags
  obj.isShowStart = false;
  obj.isRefreshState = false;

  // Set callback context
  obj.callbacksContext = this._props.callbacksContext || this;

  // Set previous module
  obj.prevChainModule = h.getLastItem(this._modules);

  // Set master module
  obj.masterModule = this;
  return obj;
}
```

### 6.3 Then Options Merging

```javascript
_mergeThenOptions(start, end) {
  var o = {};
  this._mergeStartLoop(o, start);
  this._mergeEndLoop(o, start, end);
  this._history.push(o);
  return o;
}
```

**Start Merge (copy end values from previous step):**

```javascript
_mergeStartLoop(o, start) {
  for (var key in start) {
    var value = start[key];
    if (start[key] == null) { continue; }

    // Copy all values except tween props (except duration)
    if (!h.isTweenProp(key) || key === 'duration') {
      // If delta - copy only the end value
      if (this._isDelta(value)) {
        o[key] = h.getDeltaEnd(value);
      } else {
        o[key] = value;
      }
    }
  }
}
```

**End Merge (create deltas from start to end):**

```javascript
_mergeEndLoop(o, start, end) {
  for (var key in end) {
    if (key == 'parent') { o[key] = end[key]; continue; }

    var endValue = end[key],
      startValue = (start[key] != null)
        ? start[key] : this._defaults[key];

    startValue = this._checkStartValue(key, startValue);
    if (endValue == null) { continue; }

    o[key] = this._mergeThenProperty(key, startValue, endValue);
  }
}
```

### 6.4 Property Merging Logic

```javascript
_mergeThenProperty(key, startValue, endValue) {
  var isTweenProp = h.isTweenProp(key);
  var isBoolean = typeof endValue === 'boolean';

  if (!isTweenProp && !this._nonMergeProps[key] && !isBoolean) {

    // Handle object syntax with tween options
    if (h.isObject(endValue) && endValue.to != null) {
      var tweenProps = {};
      for (let k in endValue) {
        if (h.tweenOptionMap[k] || k === 'curve') {
          tweenProps[k] = endValue[k];
          delete endValue[k];
        }
      }
      endValue = endValue.to;
    }

    // If end value is delta
    if (this._isDelta(endValue)) {
      return {
        ...this._parseDeltaValues(key, endValue),
        ...tweenProps
      };
    }

    // If start value is delta
    if (this._isDelta(startValue)) {
      return {
        [h.getDeltaEnd(startValue)]: endValue,
        ...tweenProps
      };
    }

    // Create new delta
    return { [startValue]: endValue, ...tweenProps };
  }

  // Tween properties pass through unchanged
  return endValue;
}
```

### 6.5 History Tracking

```javascript
_vars() {
  super._vars();

  // Save master module reference
  this._masterModule = this._o.masterModule;

  // Set isChained flag
  this._isChained = !!this._masterModule;

  // Clone initial props as first history record
  var initialRecord = h.cloneObj(this._props);
  for (var key in this._arrayPropertyMap) {
    if (this._o[key]) {
      var preParsed = this._parsePreArrayProperty(key, this._o[key]);
      initialRecord[key] = preParsed;
    }
  }

  this._history = [initialRecord];

  // Modules array for then chain
  this._modules = [this];

  // Properties to exclude from then merge
  this._nonMergeProps = { shape: 1 };
}
```

---

## 7. Tunable Pattern

### 7.1 Tune Method

```javascript
tune(o) {
  if (o && Object.keys(o).length) {
    // Transform history with new options
    this._transformHistory(o);

    // Tune new options
    this._tuneNewOptions(o);

    // Restore array prop values for history storage
    this._history[0] = h.cloneObj(this._props);
    for (var key in this._arrayPropertyMap) {
      if (o[key] != null) {
        this._history[0][key] = this._preparsePropValue(key, o[key]);
      }
    }

    // Tune submodules
    this._tuneSubModules();

    // Reset tweens with new durations
    this._resetTweens();
  }
  return this;
}
```

**Tune vs Then:**
- `tune()`: Modifies CURRENT animation in place
- `then()`: Creates NEW animation step after current

```javascript
// tune - modifies existing
shape.tune({ radius: 100 });  // Current animation now goes to 100

// then - adds new step
shape.then({ radius: 100 });  // Creates new animation after current
```

### 7.2 History Transformation

```javascript
_transformHistory(o) {
  for (var key in o) {
    var value = o[key];
    // Transform history for each key
    this._transformHistoryFor(key, this._preparsePropValue(key, value));
  }
}

_transformHistoryFor(key, value) {
  for (var i = 0; i < this._history.length; i++) {
    value = this._transformHistoryRecord(i, key, value);
    // Break if no further modifications needed
    if (value == null) { break; }
  }
}
```

### 7.3 History Record Transformation

```javascript
_transformHistoryRecord(index, key, newVal, currRecord, nextRecord) {
  if (newVal == null) { return null; }

  currRecord = (currRecord == null) ? this._history[index] : currRecord;
  nextRecord = (nextRecord == null) ? this._history[index + 1] : nextRecord;

  var oldVal = currRecord[key],
    nextVal = (nextRecord == null) ? null : nextRecord[key];

  // Index 0 - always save newVal and return non-delta
  if (index === 0) {
    currRecord[key] = newVal;

    // Always return on tween properties
    if (h.isTweenProp(key) && key !== 'duration') { return null; }

    // Non-tween properties
    var isRewriteNext = this._isRewriteNext(oldVal, nextVal),
      returnVal = (this._isDelta(newVal)) ? h.getDeltaEnd(newVal) : newVal;
    return (isRewriteNext) ? returnVal : null;

  } else {
    // Later indices
    if (this._isDelta(oldVal)) {
      // Was delta, came non-delta - rewrite start of delta and stop
      currRecord[key] = { [newVal]: h.getDeltaEnd(oldVal) };
      return null;
    } else {
      currRecord[key] = newVal;
      // Continue if next has same value
      return (this._isRewriteNext(oldVal, nextVal)) ? newVal : null;
    }
  }
}
```

### 7.4 Rewrite Detection

```javascript
_isRewriteNext(currVal, nextVal) {
  if (nextVal == null && currVal != null) { return false; }

  var isEqual = (currVal === nextVal),
    isNextDelta = this._isDelta(nextVal),
    isDelta = this._isDelta(currVal),
    isValueDeltaChain = false,
    isDeltaChain = false;

  if (isDelta && isNextDelta) {
    if (h.getDeltaEnd(currVal) == h.getDeltaStart(nextVal)) {
      isDeltaChain = true;
    }
  } else if (isNextDelta) {
    isValueDeltaChain = h.getDeltaStart(nextVal) === `${currVal}`;
  }

  return isEqual || isValueDeltaChain || isDeltaChain;
}
```

**Delta Chain Detection:**
```javascript
// Delta chain: {0: 50} -> {50: 100} (end of first = start of second)
// Value-delta chain: 50 -> {50: 100} (value = start of delta)
```

### 7.5 Submodule Tuning

```javascript
_tuneSubModules() {
  for (var i = 1; i < this._modules.length; i++) {
    this._modules[i]._tuneNewOptions(this._history[i]);
  }
}
```

### 7.6 Tween Reset

```javascript
_resetTweens() {
  var shift = 0,
    tweens = this.timeline._timelines;

  if (tweens == null) { return; }

  for (var i = 0; i < tweens.length; i++) {
    var tween = tweens[i],
      prevTween = tweens[i - 1];

    shift += (prevTween) ? prevTween._props.repeatTime : 0;
    this._resetTween(tween, this._history[i], shift);
  }
  this.timeline._setProp(this._props.timeline);
  this.timeline._recalcTotalDuration();
}

_resetTween(tween, o, shift = 0) {
  o.shiftTime = shift;
  tween._setProp(o);
}
```

### 7.7 Generate Method

```javascript
generate() {
  return this.tune(this._o);
}
```

**Purpose:** Regenerate all random values from initial options

```javascript
const burst = new mojs.Burst({
  radius: { 0: 'rand(100, 200)' },  // Random radius each time
  children: {
    radius: 'rand(10, 30)'
  }
});

burst.play();

// Later - regenerate with new random values
burst.generate().play();
```

---

## 8. History Transformation

### 8.1 History Structure

```javascript
// After creation with then chain:
this._history = [
  { radius: 50, duration: 1000 },     // Initial state
  { radius: {50: 100}, duration: 500 },  // After first then
  { radius: {100: 50}, duration: 500 },  // After second then
];
```

### 8.2 Transform on Tune

```javascript
// Original history
_history = [
  { x: 0 },
  { x: {0: 100} },
  { x: {100: 200} }
];

// After tune({x: 50})
_history = [
  { x: 50 },              // Initial changed to 50
  { x: {50: 100} },       // First delta start changed
  { x: {100: 200} }       // Unchanged (no further modification needed)
];
```

### 8.3 Array Property Handling

```javascript
// After tune, restore array props as strings for history storage
this._history[0] = h.cloneObj(this._props);
for (var key in this._arrayPropertyMap) {
  if (o[key] != null) {
    this._history[0][key] = this._preparsePropValue(key, o[key]);
  }
}
```

**Why?**
- `_props` contains parsed arrays: `[{value: 5, unit: 'px'}]`
- History needs mergeable format: `'5 10'`

---

## 9. Extensibility Patterns

### 9.1 Override Points

```javascript
// 1. _declareDefaults - Override to set class-specific defaults
_declareDefaults() {
  this._defaults = {
    customProp: 'default',
    ...super._defaults
  };
}

// 2. _vars - Override to initialize class-specific state
_vars() {
  super._vars();
  this._customState = 0;
}

// 3. _render - Override to create class-specific DOM
_render() {
  super._render();
  this._createCustomElement();
}

// 4. _draw - Override to render class-specific props
_draw() {
  super._draw();
  this._drawCustomProp();
}

// 5. _parseOption - Override to add custom parsing
_parseOption(name, value) {
  if (name === 'customProp') {
    return this._parseCustomProp(value);
  }
  return super._parseOption(name, value);
}
```

### 9.2 Custom Properties

```javascript
// In subclass
_saveCustomProperties(o = {}) {
  this._customProps = o.customProperties || {};
  this._customProps = { ...this._customProps };
  this._customDraw = this._customProps.draw;
  delete this._customProps.draw;
  delete o.customProperties;
}

// Usage
new mojs.Html({
  el: '#element',
  customProperties: {
    progress: { type: 'number', default: 0 },
    draw(el, props) {
      el.style.width = props.progress + '%';
    }
  },
  progress: { 0: 100 }
});
```

### 9.3 Skip Properties

```javascript
// Properties to skip during delta calculation
this._skipPropsDelta = {
  timeline: 1,
  prevChainModule: 1,
  callbacksContext: 1,
};

// Properties to exclude from then merge
this._nonMergeProps = { shape: 1 };
```

---

## 10. Property Caching and Validation

### 10.1 State Caching

```javascript
// In Bit class (SVG shapes)
_state: {}  // Cache for attribute values

_setAttrIfChanged(name, value) {
  if (this._state[name] !== value) {
    this.el.setAttribute(name, value);
    this._state[name] = value;  // Cache for next comparison
  }
}
```

**Benefit:** Prevents redundant DOM writes

### 10.2 Delta Caching

```javascript
this._deltas = {};  // Stores parsed deltas

// After _extendDefaults:
_deltas = {
  radius: {
    type: 'number',
    start: 50,
    end: 100,
    delta: 50
  }
};
```

### 10.3 Progress Calculation

```javascript
_calcCurrentProps(easedProgress, p) {
  for (var key in this._deltas) {
    var value = this._deltas[key];

    // Get easing-specific progress
    var isCurve = !!value.curve;
    var ep = (value.easing != null && !isCurve)
      ? value.easing(p) : easedProgress;

    if (value.type === 'array') {
      // Array interpolation (stroke-dasharray)
      var arr = h.isArray(this._props[key])
        ? this._props[key] : [];
      arr.length = 0;

      var proc = (isCurve) ? value.curve(p) : null;

      for (var i = 0; i < value.delta.length; i++) {
        var item = value.delta[i],
          dash = (!isCurve)
            ? value.start[i].value + ep * item.value
            : proc * (value.start[i].value + p * item.value);
        arr.push({
          string: `${dash}${item.unit}`,
          value: dash,
          unit: item.unit,
        });
      }
      this._props[key] = arr;

    } else if (value.type === 'number') {
      this._props[key] = (!isCurve)
        ? value.start + ep * value.delta
        : value.curve(p) * (value.start + p * value.delta);

    } else if (value.type === 'unit') {
      var currentValue = (!isCurve)
        ? value.start.value + ep * value.delta
        : value.curve(p) * (value.start.value + p * value.delta);
      this._props[key] = `${currentValue}${value.end.unit}`;

    } else if (value.type === 'color') {
      var r, g, b, a;
      if (!isCurve) {
        r = parseInt(value.start.r + ep * value.delta.r, 10);
        g = parseInt(value.start.g + ep * value.delta.g, 10);
        b = parseInt(value.start.b + ep * value.delta.b, 10);
        a = parseFloat(value.start.a + ep * value.delta.a);
      } else {
        var cp = value.curve(p);
        r = parseInt(cp * (value.start.r + p * value.delta.r), 10);
        g = parseInt(cp * (value.start.g + p * value.delta.g), 10);
        b = parseInt(cp * (value.start.b + p * value.delta.b), 10);
        a = parseFloat(cp * (value.start.a + p * value.delta.a));
      }
      this._props[key] = `rgba(${r},${g},${b},${a})`;
    }
  }
}
```

---

## Summary

The mo.js module system is a sophisticated, layered architecture that provides:

1. **Consistent Inheritance:** Clear hierarchy from Module → Thenable → Tunable → Concrete classes
2. **Defaults-Driven Design:** Every property flows through defaults declaration and extension
3. **Smart Parsing:** Automatic detection of deltas, units, stagger, and rand expressions
4. **Delta System Integration:** Seamless transition from options to interpolated animation
5. **Thenable Chaining:** Promise-like API for animation sequences
6. **Runtime Tuning:** `.tune()` for modifying animations mid-flight with history transformation
7. **Extensibility:** Multiple override points for custom behavior
8. **Performance:** Property caching, state tracking, and skip lists for optimization

This architecture enables the declarative, chainable API that makes mo.js unique while maintaining performance through careful property management and delta pre-computation.
