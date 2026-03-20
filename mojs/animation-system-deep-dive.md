---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src
explored_at: 2026-03-20
---

# mo.js Animation System - Deep Dive

**Scope:** Tween engine, Timeline composition, Delta interpolation, Forward/backward replay, Yoyo, Speed control, Thenable chaining

---

## Table of Contents

1. [Animation Architecture Overview](#1-animation-architecture-overview)
2. [Tween Engine Core](#2-tween-engine-core)
3. [Time Calculation System](#3-time-calculation-system)
4. [Period Detection and Yoyo](#4-period-detection-and-yoyo)
5. [Callback System](#5-callback-system)
6. [Timeline Composition](#6-timeline-composition)
7. [Thenable Chain Pattern](#7-thenable-chain-pattern)
8. [Delta System](#8-delta-system)
9. [Speed and Time Scale](#9-speed-and-time-scale)
10. [Forward/Backward Playback](#10-forwardbackward-playback)
11. [Stagger System](#11-stagger-system)
12. [Tunable Module Pattern](#12-tunable-module-pattern)

---

## 1. Animation Architecture Overview

### 1.1 Component Hierarchy

```
┌─────────────────────────────────────────────────────────────────────┐
│                      mo.js ANIMATION STACK                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Tween Engine                               │   │
│  │  (tween/tween.babel.js - 1,276 lines)                        │   │
│  │  - Time calculation, period detection                         │   │
│  │  - Callback dispatch, yoyo handling                           │   │
│  │  - Speed control, progress computation                        │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              ▲                                       │
│                              │                                       │
│         ┌────────────────────┼────────────────────┐                 │
│         │                    │                    │                 │
│         ▼                    ▼                    ▼                 │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐          │
│  │   Timeline  │     │   Tunable   │     │   Thenable  │          │
│  │  (compose   │     │  (modules   │     │  (then      │          │
│  │   tweens)   │     │   inherit)  │     │   chaining) │          │
│  └─────────────┘     └─────────────┘     └─────────────┘          │
│         │                    │                    │                 │
│         │                    │                    │                 │
│         ▼                    ▼                    ▼                 │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     Deltas System                            │   │
│  │  (delta/delta.babel.js, deltas.babel.js)                    │   │
│  │  - Color interpolation (RGBA)                                │   │
│  │  - Number interpolation                                      │   │
│  │  - Unit interpolation (px, %, em)                            │   │
│  │  - Array interpolation (stroke-dasharray)                    │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Easing System                             │   │
│  │  (easing/*.coffee, *.babel.js)                              │   │
│  │  - Bezier easing (cubic)                                     │   │
│  │  - Path easing (SVG-based)                                   │   │
│  │  - Approximated easing (sampled)                             │   │
│  │  - Easing mixing                                              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 Animation Flow

```
user.play() / timeline.play()
        │
        ▼
┌─────────────────────────┐
│  Tweener.add(tween)     │
│  (global RAF loop)      │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  requestAnimationFrame  │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  ig.Timer.step()        │
│  Update global time     │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  tween._update(time)    │
│  Calculate progress     │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  _getPeriod(time)       │
│  Detect current period  │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  _getProgress(time)     │
│  Calculate rawProgress  │
│  Apply yoyo flip        │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  easing(progress)       │
│  Apply easing curve     │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  deltas.render(ep, p)   │
│  Interpolate properties │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  onUpdate(progress)     │
│  User callback          │
└─────────────────────────┘
```

### 1.3 Key Animation Classes

| Class | File | Purpose |
|-------|------|---------|
| `Tween` | `tween/tween.babel.js` | Core animation engine |
| `Timeline` | `tween/timeline.babel.js` | Tween composition |
| `Tweener` | `tween/tweener.babel.js` | Global RAF manager |
| `Deltas` | `delta/deltas.babel.js` | Multi-property interpolation |
| `Delta` | `delta/delta.babel.js` | Single property delta |
| `Thenable` | `thenable.babel.js` | Then chain pattern |
| `Tunable` | `tunable.babel.js` | Module base with tune() |
| `Stagger` | `stagger.babel.js` | Staggered animations |

---

## 2. Tween Engine Core

### 2.1 Tween Defaults

```javascript
_declareDefaults() {
    this._defaults = {
        // Timing
        duration: 1000,       // Animation duration (ms)
        delay: 0,             // Initial delay (ms)
        repeat: 0,            // Number of repeats
        repeatDelay: 0,       // Delay between repeats

        // Direction
        isYoyo: false,        // Alternate on repeats
        isReverse: false,     // Start from end

        // Speed
        speed: 1,             // Playback speed multiplier
        time: null,           // Start time offset

        // Easing
        easing: 'sin.out',    // Forward easing
        backwardEasing: null, // Backward easing (yoyo)

        // Callbacks
        onStart: null,
        onUpdate: null,
        onComplete: null,
        onRepeatStart: null,
        onRepeatComplete: null,
        onFirstUpdate: null,

        // State
        isRunLess: false,     // Don't auto-start
        isChained: false,     // Part of then chain
    };
}
```

### 2.2 Tween Initialization

```javascript
constructor(o = {}) {
    this._o = h.cloneObj(o);  // Clone options
    this._props = {};
    this._vars();             // Initialize variables
    this._declareDefaults();  // Set defaults
    this._extendDefaults();   // Merge options with defaults
    this._render();           // Initial render
    this._makeTimeline();     // Create timeline
    this._makeTween();        // Create tween
}
```

### 2.3 Play/Pause/Stop

```javascript
// Start playback
play() {
    if (this._state === 'play') { return this; }

    this._state = 'play';
    this._wasStarted = true;
    this._timeShift = 0;

    // Add to global tweener
    Tweener.add(this);

    // Callback
    this._callback('onPlaybackStart');
    return this;
}

// Pause playback
pause() {
    if (this._state !== 'play') { return this; }

    this._state = 'pause';
    this._pauseTime = this._currentTime;

    // Remove from tweener
    Tweener.remove(this);

    // Callback
    this._callback('onPlaybackPause');
    return this;
}

// Stop and reset
stop(progress) {
    this._state = 'stop';
    this._wasStarted = false;

    // Remove from tweener
    Tweener.remove(this);

    // Set progress if provided
    if (progress != null) {
        this._setProgress(progress);
    }

    // Callback
    this._callback('onPlaybackStop');
    return this;
}
```

---

## 3. Time Calculation System

### 3.1 Period-Based Time Model

mo.js uses a **continuous time model** where repeats, delays, and yoyo are calculated as a continuous timeline rather than discrete segments.

```javascript
// Total repeat time includes all periods
const repeatTime = (duration + delay) * (repeat + 1);

// Start time includes shifts and delays
const startTime = currentTime + delay + negativeShift + shiftTime;

// End time is start + total duration minus initial delay
const endTime = startTime + repeatTime - delay;
```

### 3.2 Time Visualization

```
Single Period (no repeat):
├─ delay ─┼──── duration ────┤
          ▲                  ▲
       startTime          endTime

With Repeat (repeat: 2):
├─ delay ─┼─ duration ─┼─ delay ─┼─ duration ─┤
          ▲            │                    ▲
       startTime    repeat point       endTime

With Yoyo (repeat: 2, isYoyo: true):
Forward: ───────────▶
         └─ period 0 ┘
              ◀───────────
              └─ period 1 (reverse) ┘
                   ───────────▶
                   └─ period 2 (forward) ┘
```

### 3.3 Period Detection

```javascript
_getPeriod(time) {
    const TTime = this._props.delay + this._props.duration;
    const dTime = this._props.delay + time - this._props.startTime;

    // Calculate which period we're in
    const T = dTime / TTime;
    const elapsed = (time < this._props.endTime) ? dTime % TTime : 0;

    // Check if in delay gap
    if (elapsed > 0 && elapsed < this._props.delay) {
        this._delayT = T;
        return 'delay';
    }

    // Return period number
    return Math.floor(T);
}
```

### 3.4 Progress Calculation

```javascript
_getProgress(time) {
    const TTime = this._props.duration + this._props.delay;
    const dTime = time - this._props.startTime;

    // Raw progress through entire timeline
    let T = dTime / TTime;

    // Clamp to valid range
    if (time < this._props.startTime) { return 0; }
    if (time > this._props.endTime) { return 1; }

    // Get progress within current period
    let progress = (dTime % TTime - this._props.delay) / this._props.duration;

    // Handle edge cases
    if (progress < 0) { progress = 0; }
    if (progress > 1) { progress = 1; }

    return progress;
}
```

---

## 4. Period Detection and Yoyo

### 4.1 Yoyo Period Detection

```javascript
// In _update()
const T = Math.floor((time - startTime) / (duration + delay));
const isYoyo = this._props.isYoyo && (T % 2 === 1);

// On odd periods, reverse direction
if (isYoyo) {
    rawProgress = 1 - rawProgress;
    easedProgress = 1 - easedProgress;
}
```

### 4.2 Yoyo Visualization

```
Yoyo Animation (repeat: 3, isYoyo: true):

Period 0 (Forward):  0 ─────────▶ 1
                          easing: forward
Period 1 (Backward): 1 ◀───────── 0
                          easing: backward
Period 2 (Forward):  0 ─────────▶ 1
                          easing: forward
Period 3 (Backward): 1 ◀───────── 0
                          easing: backward
                          onComplete
```

### 4.3 Backward Easing

```javascript
// Use different easing for yoyo backward periods
if (this._props.isYoyo && isYoyo) {
    // Use backward easing if specified
    const easing = this._props.backwardEasing || this._props.easing;
    easedProgress = easing(rawProgress);
} else {
    easedProgress = this._props.easing(rawProgress);
}
```

---

## 5. Callback System

### 5.1 Callback Types

```javascript
// Callback declaration in defaults
_declareDefaults() {
    return {
        // Progress callbacks
        onProgress: null,      // Every frame (before others)
        onUpdate: null,        // Every frame in active area

        // Edge callbacks
        onStart: null,         // First frame of first period
        onComplete: null,      // Final period end

        // Repeat callbacks
        onRepeatStart: null,   // Start of each repeat
        onRepeatComplete: null,// End of each repeat

        // Special callbacks
        onFirstUpdate: null,   // First update in active area
        onPlaybackStart: null, // When play/resume called
        onPlaybackPause: null, // When pause called
        onPlaybackStop: null,  // When stop called
        onPlaybackComplete: null, // When fully complete
    };
}
```

### 5.2 Callback Dispatch

```javascript
_update(time, prevTime, wasYoyo, onEdge) {
    const p = this._props;
    const isForward = time >= this._prevTime;
    const isYoyo = this._getIsYoyo(time);

    // Get period and progress
    const period = this._getPeriod(time);
    const progress = this._getProgress(time);
    const easedProgress = this._getEasedProgress(progress, isYoyo);

    // Check for period change
    const isPeriodChanged = period !== this._prevPeriod;
    const isFirstPeriod = period === 0 && !this._wasStarted;

    // --- Callback dispatch ---

    // onProgress: Every frame
    this._callback('onProgress', [easedProgress, isForward]);

    // onStart: First period start
    if (isFirstPeriod && isForward) {
        this._callback('onStart', [isForward, isYoyo]);
    }

    // onFirstUpdate: First update in active area
    if (!this._wasUpdated && period !== 'delay') {
        this._wasUpdated = true;
        this._callback('onFirstUpdate', [isForward, isYoyo]);
    }

    // onUpdate: Every frame in active area
    if (period !== 'delay') {
        this._callback('onUpdate', [easedProgress, progress, isForward, isYoyo]);
    }

    // onRepeatStart: New period started
    if (isPeriodChanged && period !== 'delay') {
        this._callback('onRepeatStart', [isForward, isYoyo]);
    }

    // onRepeatComplete: Period completed
    if (isPeriodChanged && this._prevPeriod !== 'delay') {
        this._callback('onRepeatComplete', [isForward, isYoyo]);
    }

    // onComplete: All periods complete
    if (time >= this._props.endTime && !this._wasCompleted) {
        this._wasCompleted = true;
        this._callback('onComplete', [isForward, isYoyo]);
        this._callback('onPlaybackComplete');
    }

    // Cache values
    this._prevTime = time;
    this._prevPeriod = period;
    this._prevYoyo = isYoyo;
}
```

### 5.3 Callback Context

```javascript
_callback(name, args) {
    const callback = this._props[name];
    if (callback) {
        // Call with specified context
        const ctx = this._props.callbacksContext || this;
        callback.apply(ctx, args || []);
    }
}
```

---

## 6. Timeline Composition

### 6.1 Timeline Structure

```javascript
class Timeline extends Tween {
    constructor(o = {}) {
        super(o);
        this._timelines = [];  // Child tweens/timelines
    }
}
```

### 6.2 Adding Children

```javascript
// Add tweens to timeline (parallel)
add(...args) {
    this._pushTimelineArray(args);
    this._calcDimensions();
    return this;
}

// Append tweens sequentially
append(...timeline) {
    for (var tm of timeline) {
        if (h.isArray(tm)) {
            this._appendTimelineArray(tm);
        } else {
            this._appendTimeline(tm, this._timelines.length);
        }
        this._calcDimensions();
    }
    return this;
}
```

### 6.3 Timeline Duration Calculation

```javascript
_recalcDuration(timeline) {
    var p = timeline._props;

    // Calculate timeline's total time
    var timelineTime = p.repeatTime / p.speed +
                       (p.shiftTime || 0) +
                       timeline._negativeShift;

    // Parent duration is max of all children
    this._props.duration = Math.max(timelineTime, this._props.duration);
}

_recalcTotalDuration() {
    var i = this._timelines.length;
    this._props.duration = 0;

    while (i--) {
        var tm = this._timelines[i];

        // Recursively calculate child durations
        tm._recalcTotalDuration && tm._recalcTotalDuration();
        this._recalcDuration(tm);
    }

    this._calcDimensions();
}
```

### 6.4 Child Update Order

```javascript
_updateChildren(p, time, isYoyo) {
    // Determine update direction based on time flow
    var coef = (time > this._prevTime) ? -1 : 1;
    if (this._props.isYoyo && isYoyo) { coef *= -1; }

    var timeToTimelines = this._props.startTime + p * this._props.duration;
    var prevTimeToTimelines = timeToTimelines + coef;
    var len = this._timelines.length;

    for (var i = 0; i < len; i++) {
        // Update in correct direction
        var j = (timeToTimelines > prevTimeToTimelines)
            ? i : (len - 1) - i;

        this._timelines[j]._update(
            timeToTimelines,
            prevTimeToTimelines,
            this._prevYoyo,
            this._onEdge,
        );
    }
    this._prevYoyo = isYoyo;
}
```

### 6.5 Timeline Usage Example

```javascript
// Create timeline
const tl = new mojs.Timeline();

// Add parallel animations
tl.add(
    new mojs.Shape({ shape: 'circle' }).tune({ radius: 100 }),
    new mojs.Shape({ shape: 'rect' }).tune({ scale: 2 })
);

// Append sequential animation
tl.append(
    new mojs.Shape({ shape: 'circle' }).tune({ opacity: 0 })
);

// Play timeline
tl.play();
```

---

## 7. Thenable Chain Pattern

### 7.1 Then Chain Structure

```javascript
// Then creates animation sequences
new mojs.Shape({
    shape: 'circle',
    radius: 50
})
.then({ radius: 100, duration: 500 })   // Step 1
.then({ radius: 50, duration: 500 })    // Step 2
.then({ scale: 0, duration: 300 })      // Step 3
.play();
```

### 7.2 Then Implementation

```javascript
// In Thenable class
then(o) {
    if ((o == null) || !Object.keys(o).length) { return 1; }

    // Create new module with merged options
    const ModuleClass = this.constructor;
    const newModule = new ModuleClass(this._mergeThen(o));

    // Add to modules array
    this._modules.push(newModule);

    // Link to previous module
    newModule._o.prevChainModule = this._lastModule || this;

    // Update last module reference
    this._lastModule = newModule;

    return newModule;
}

_mergeThen(o) {
    // Clone current options
    const opts = h.cloneObj(this._o);

    // Merge with new options
    for (let key in o) {
        if (h.callbacksMap[key]) {
            // Override callbacks
            opts[key] = o[key];
        } else if (h.tweenOptionMap[key]) {
            // Merge tween options
            opts[key] = o[key];
        } else {
            // Merge property deltas
            opts[key] = this._mergeThenProperty(key, this._props[key], o[key]);
        }
    }

    return opts;
}
```

### 7.3 Delta Merging

```javascript
_mergeThenProperty(key, startValue, endValue) {
    const isTweenProp = h.isTweenProp(key);
    const isBoolean = typeof endValue === 'boolean';

    if (!isTweenProp && !this._nonMergeProps[key] && !isBoolean) {

        // Handle object syntax with tween options
        if (h.isObject(endValue) && endValue.to != null) {
            const tweenProps = {};
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

---

## 8. Delta System

### 8.1 Delta Types

```javascript
// Four delta types for interpolation
const deltaTypes = {
    color: (start, end) => {
        // Parse colors to RGBA
        const s = parseColor(start);
        const e = parseColor(end);
        return {
            start: s,
            delta: {
                r: e.r - s.r,
                g: e.g - s.g,
                b: e.b - s.b,
                a: e.a - s.a
            }
        };
    },

    number: (start, end) => {
        return {
            start: start,
            delta: end - start
        };
    },

    unit: (start, end) => {
        const s = parseUnit(start);
        const e = parseUnit(end);
        return {
            start: s,
            delta: e.value - s.value,
            unit: e.unit
        };
    },

    array: (start, end) => {
        // For stroke-dasharray etc.
        return start.map((s, i) => ({
            start: s,
            delta: end[i] - s
        }));
    }
};
```

### 8.2 Color Interpolation

```javascript
_calcCurrent_color(delta, easedProgress, progress) {
    const start = delta.start;  // {r, g, b, a}
    const d = delta.delta;      // {r, g, b, a}

    if (!delta.curve) {
        // Linear with easing
        r = parseInt(start.r + easedProgress * d.r, 10);
        g = parseInt(start.g + easedProgress * d.g, 10);
        b = parseInt(start.b + easedProgress * d.b, 10);
        a = parseFloat(start.a + easedProgress * d.a);
    } else {
        // Curve-based (elasticity)
        const cp = delta.curve(progress);
        r = parseInt(cp * (start.r + progress * d.r), 10);
        g = parseInt(cp * (start.g + progress * d.g), 10);
        b = parseInt(cp * (start.b + progress * d.b), 10);
        a = cp * (start.a + progress * d.a);
    }

    this._o.props[name] = `rgba(${r},${g},${b},${a})`;
}
```

### 8.3 Unit Interpolation

```javascript
// Parse unit from value
parseUnit(value) {
    if (typeof value === 'number') {
        return {
            unit: 'px',
            isStrict: false,
            value: value,
            string: value + 'px'
        };
    }

    const match = value.match(/px|%|rem|em|vw|vh|deg|rad/gim);
    const unit = match ? match[0] : 'px';
    const amount = parseFloat(value);

    return {
        unit: unit,
        isStrict: true,
        value: amount,
        string: `${amount}${unit}`
    };
}

// Calculate with units
_calcCurrent_unit(delta, easedProgress, progress) {
    const value = delta.start.value + easedProgress * delta.delta;
    this._o.props[name] = `${value}${delta.unit}`;
}
```

### 8.4 Random Values

```javascript
// Parse rand() syntax
parseIfRand(str) {
    const match = str.match(/^rand\((\d+\.?\d*),\s*(\d+\.?\d*)\)/);
    if (match) {
        return +match[1] + Math.random() * (+match[2] - +match[1]);
    }
    return str;
}

// Usage
new mojs.Burst({
    children: {
        radius: 'rand(20, 10)',    // Random 10-20
        delay: 'rand(0, 500)',     // Random delay
        opacity: 'rand(0.1, 1)'    // Random opacity
    }
});
```

---

## 9. Speed and Time Scale

### 9.1 Speed Control

```javascript
// Speed affects time progression
if (speed && playTime) {
    time = playTime + (speed * (time - playTime));
}
// speed: 0.5 = 2x slower, 2 = 2x faster
```

### 9.2 Speed Visualization

```
Normal Speed (speed: 1):
real time:  0    500   1000  1500  2000
            │─────│─────│─────│─────│
animation:  0    500   1000  1500  2000
            └──── 1000ms duration ─┘

Half Speed (speed: 0.5):
real time:  0    500   1000  1500  2000
            │─────│─────│─────│─────│
animation:  0    250   500   750   1000
            └──────── 1000ms needs 2000ms real ────────┘

Double Speed (speed: 2):
real time:  0    500   1000  1500  2000
            │─────│─────│─────│─────│
animation:  0    1000  2000  3000  4000
            └── 1000ms completes in 500ms real ──┘
```

### 9.3 Time Scale vs Speed

```javascript
// Global time scale (affects all tweens)
mojs.Tweener.timeScale = 0.5;  // Slow motion

// Per-tween speed (affects only this tween)
new mojs.Tween({
    speed: 2,  // This tween plays 2x faster
    duration: 1000
});
```

---

## 10. Forward/Backward Playback

### 10.1 Reverse Playback

```javascript
// Set reverse playback
new mojs.Tween({
    isReverse: true  // Start from end, play backward
});

// Reverse method
reverse() {
    if (!this._wasReversed) {
        this._wasReversed = true;
        this._props.isReverse = !this._props.isReverse;
    }

    this._state = 'play';
    Tweener.add(this);
    return this;
}
```

### 10.2 Backward Easing

```javascript
// Different easing for backward playback
_getEasing(progress, isYoyo) {
    const isBackward = isYoyo || this._props.isReverse;

    if (isBackward && this._props.backwardEasing) {
        return this._props.backwardEasing(progress);
    }

    return this._props.easing(progress);
}
```

### 10.3 Playback Direction Detection

```javascript
_update(time, prevTime, wasYoyo, onEdge) {
    // Detect direction from time flow
    const isForward = time >= this._prevTime;

    // Use direction for callbacks
    if (isForward) {
        this._callback('onStart', [true, isYoyo]);
    } else {
        this._callback('onStart', [false, isYoyo]);
    }
}
```

---

## 11. Stagger System

### 11.1 Stagger Timing

```javascript
// Stagger distributes animations over time
new mojs.Stagger({
    length: 5,           // Number of items
    from: 'center',      // Start from center
    delay: 100,          // Delay between each
    children: {
        shape: 'circle',
        radius: 50
    }
});
```

### 11.2 Stagger Implementation

```javascript
class Stagger {
    constructor(o) {
        this._o = o;
        this._length = o.length || 0;
        this._from = o.from || 'start';
        this._delay = o.delay || 0;

        this._createItems();
    }

    _createItems() {
        const items = [];
        const delays = this._getDelays();

        for (let i = 0; i < this._length; i++) {
            const opts = { ...this._o.children };
            opts.delay = delays[i];
            items.push(new mojs.Shape(opts));
        }

        this.items = items;
    }

    _getDelays() {
        const delays = [];
        const center = Math.floor(this._length / 2);

        for (let i = 0; i < this._length; i++) {
            let distance;

            switch (this._from) {
                case 'center':
                    distance = Math.abs(i - center);
                    break;
                case 'end':
                    distance = this._length - 1 - i;
                    break;
                default:  // 'start'
                    distance = i;
            }

            delays[i] = distance * this._delay;
        }

        return delays;
    }
}
```

### 11.3 Stagger From Values

```javascript
// Stagger from different points
'from': 'start'   // 0, 100, 200, 300, 400
'from': 'center'  // 200, 100, 0, 100, 200 (for length 5)
'from': 'end'     // 400, 300, 200, 100, 0
'from': 'random'  // Random delays
```

---

## 12. Tunable Module Pattern

### 12.1 Tunable Base Class

```javascript
class Tunable extends Thenable {
    // Tune new options on existing animation
    tune(o) {
        if (o == null) { return this; }

        // Save timeline options
        this._saveTimelineOptions(o);

        // Apply timeline options
        this.timeline._setProp(this._timelineOptions);

        // Remove tween properties
        this._removeTweenProperties(o);

        // Tune props
        this._tuneNewOptions(o);

        // Recalc time
        this._recalcModulesTime();

        return this;
    }

    _tuneNewOptions(o) {
        // Merge new options with existing
        for (let key in o) {
            if (this._defaults[key] != null) {
                this._props[key] = o[key];
            }
        }

        // Update deltas
        this.deltas.refresh(false);
    }
}
```

### 12.2 Tune vs Then

```javascript
// tune() - modify current animation
shape.tune({ radius: 100 });  // Changes current animation

// then() - create new animation step
shape.then({ radius: 100 });  // Creates new animation after current
```

### 12.3 Tune Example

```javascript
const shape = new mojs.Shape({
    shape: 'circle',
    radius: 50,
    duration: 500
});

// Tune to change radius mid-animation
shape.tune({
    radius: 100,
    duration: 1000  // New duration
});

// Original animation continues with new values
```

---

## Appendix: Complete Animation State Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                      TWEEN STATE MACHINE                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│                         ┌─────────────┐                             │
│                   ┌────▶│   STOPPED   │◀────┐                       │
│                   │     └──────┬──────┘     │                       │
│                   │            │            │                       │
│                   │     play() │            │ stop()                │
│                   │            │            │                       │
│                   │            ▼            │                       │
│                   │     ┌─────────────┐    │                       │
│                   │     │   PLAYING   │────┘                       │
│                   │     └──────┬──────┘                            │
│                   │            │                                   │
│                   │     pause()│ resume()                          │
│                   │            │                                   │
│                   │            ▼                                   │
│                   │     ┌─────────────┐                            │
│                   └─────│   PAUSED    │◀───┐                       │
│                         └─────────────┘    │                       │
│                              │             │                       │
│                         stop()│             │reverse()             │
│                              │             │                       │
│                              ▼             │                       │
│                         ┌─────────────┐    │                       │
│                         │   REVERSE   │────┘                       │
│                         └─────────────┘                            │
│                                                                      │
│  Period Transitions (with repeat/yoyo):                             │
│                                                                      │
│  Period 0 ──▶ Period 1 ──▶ Period 2 ──▶ ... ──▶ Complete           │
│     │            │            │                                      │
│     │     onRepeat   onRepeat                                        │
│     │     Complete   Complete                                        │
│  onStart                                                              │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```
