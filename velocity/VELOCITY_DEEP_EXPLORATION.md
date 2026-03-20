---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/velocity
repository: https://github.com/velocity-animate/velocity-animate
explored_at: 2026-03-20
---

# Velocity Animation Library: Deep Technical Exploration

## Executive Summary

This document provides an exhaustive technical analysis of the Velocity animation library (velocity-animate), focusing on its core architecture. This exploration is designed for developers who need to understand the implementation details sufficiently to create a compatible clone or extend the library.

**Source Files Analyzed:**
- `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/velocity/src/Velocity/tick.ts` - Tick loop and RAF timing
- `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/velocity/src/Velocity/tweens.ts` - Tween pattern matching
- `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/velocity/src/Velocity/easing/bezier.ts` - Cubic bezier mathematics
- `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/velocity/src/Velocity/queue.ts` - Queue system
- `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/velocity/src/Velocity/css/*.ts` - CSS property handling
- `/home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/velocity/src/Velocity/easing/*.ts` - All easing functions

---

## Table of Contents

1. [Tick Loop Architecture](#1-tick-loop-architecture)
2. [Tween Pattern Matching System](#2-tween-pattern-matching-system)
3. [Cubic Bezier Mathematics](#3-cubic-bezier-mathematics)
4. [Queue System](#4-queue-system)
5. [CSS Property Handling](#5-css-property-handling)
6. [Easing Functions Reference](#6-easing-functions-reference)
7. [Performance Characteristics](#7-performance-characteristics)

---

## 1. Tick Loop Architecture

### 1.1 Overview

The tick loop is the heartbeat of Velocity's animation system. It runs on `requestAnimationFrame` (RAF) and processes all active animations each frame.

**File:** `tick.ts`

### 1.2 RAF Timing System and Frame Skipping Logic

```typescript
const FRAME_TIME = 1000 / 60,  // ~16.67ms for 60fps

export function tick(timestamp?: number | boolean) {
    if (ticking) {
        return;  // Prevent double-calling
    }
    ticking = true;

    if (timestamp !== false) {
        const timeCurrent = performance.now(),
            deltaTime = lastTick ? timeCurrent - lastTick : FRAME_TIME;

        if (deltaTime >= defaults.minFrameTime || !lastTick) {
            lastTick = timeCurrent;
            // ... process animations
        }
    }
    // ... schedule next frame
    ticking = false;
}
```

**Key Timing Variables:**

| Variable | Type | Purpose |
|----------|------|---------|
| `FRAME_TIME` | `const` | 16.67ms (1000/60) - target frame duration |
| `lastTick` | `let` | Timestamp of last processed frame |
| `timeCurrent` | `const` | Current `performance.now()` timestamp |
| `deltaTime` | `const` | Time elapsed since last frame |
| `minFrameTime` | `config` | Minimum time between frames (based on fpsLimit) |

**Frame Skipping Logic:**

```typescript
if (deltaTime >= defaults.minFrameTime || !lastTick) {
    lastTick = timeCurrent;
    // Process animations
}
```

The `minFrameTime` is calculated as:
```
minFrameTime = FUZZY_MS_PER_SECOND / fpsLimit
             = 980 / 60
             = 16.333... ms
```

Note: `FUZZY_MS_PER_SECOND = 980` (not 1000) provides slight tolerance for frame timing variations.

### 1.3 WebWorker Background Thread Mechanism

When a tab becomes hidden, browsers throttle `requestAnimationFrame` to conserve battery. Velocity maintains 30fps accuracy in background tabs using a WebWorker.

```typescript
function workerFn(this: Worker) {
    let interval: any;

    this.onmessage = (e) => {
        switch (e.data) {
            case true:   // Start background ticking
                if (!interval) {
                    interval = setInterval(() => {
                        this.postMessage(true);
                    }, 1000 / 30);  // 30fps
                }
                break;

            case false:  // Stop background ticking
                if (interval) {
                    clearInterval(interval);
                    interval = 0;
                }
                break;
        }
    };
}

try {
    worker = new Worker(URL.createObjectURL(
        new Blob([`(${workerFn})()`])
    ));

    worker.onmessage = (e: MessageEvent) => {
        if (e.data === true) {
            tick();  // Trigger animation frame
        }
    };

    if (!State.isMobile && document.hidden !== undefined) {
        document.addEventListener("visibilitychange", () => {
            worker.postMessage(State.isTicking && document.hidden);
        });
    }
} catch (e) {
    // Fallback for IE10 where blob-based workers fail
}
```

**Architecture Diagram:**

```
┌─────────────────────────────────────────────────────────────┐
│                     Main Thread                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │  tick()      │◄───│  RAF Proxy   │◄───│  rAF Shim    │  │
│  │  Animation   │    │  (setTimeout)│    │  (rAF or     │  │
│  │  Processing  │    │              │    │   fallback)  │  │
│  └──────┬───────┘    └──────────────┘    └──────────────┘  │
│         │                                                   │
│         ▼                                                   │
│  ┌──────────────┐                                          │
│  │  completed   │  Set of animations to finalize           │
│  │  progressed  │  Set of animations to report progress    │
│  └──────────────┘                                          │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │ postMessage(true) - tick trigger
                              │ postMessage("") - async callbacks
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     WebWorker (Background)                   │
│  ┌──────────────┐                                           │
│  │  setInterval │  30fps (33.33ms) when tab hidden         │
│  │  1000/30     │                                           │
│  └──────────────┘                                           │
└─────────────────────────────────────────────────────────────┘
```

**Visibility Change Handling:**

```typescript
document.addEventListener("visibilitychange", () => {
    worker.postMessage(State.isTicking && document.hidden);
});
```

When `document.hidden === true` and animations are ticking, the worker starts its 30fps interval.

### 1.4 `timeStart` Adjustment for Paused Animations

```typescript
// If this animation is paused then skip processing unless
// it has been set to resume.
if (flags & AnimationFlags.PAUSED) {
    // Update the time start to accommodate the paused
    // completion amount.
    activeCall.timeStart += deltaTime;
    continue;
}
```

**Mechanism:**

1. When paused, the animation's `timeStart` is continuously incremented by `deltaTime` each frame
2. This effectively "pushes" the start time forward, maintaining the same `percentComplete`
3. When resumed, the animation continues from exactly where it paused

**Mathematical Representation:**

```
While paused:
  timeStart[n+1] = timeStart[n] + deltaTime

percentComplete = (timeCurrent - timeStart) / duration

Since timeStart increases with timeCurrent, percentComplete remains constant.
```

### 1.5 Sync Animation Batching System

Velocity supports synchronizing multiple animations to start simultaneously.

```typescript
// AnimationFlags
export const enum AnimationFlags {
    EXPANDED = 1 << 0,
    READY    = 1 << 1,
    STARTED  = 1 << 2,
    SYNC     = 1 << 5,  // Sync flag
    // ...
}

// In tick():
// First loop: mark animations as READY
if (!(flags & AnimationFlags.READY)) {
    activeCall._flags |= AnimationFlags.READY;
    options._ready++;
}

// Second loop: check sync readiness
if ((flags & AnimationFlags.SYNC) && options._ready < options._total) {
    activeCall.timeStart += deltaTime;  // Delay start
    continue;
}
```

**Sync State Variables (in `options`):**

| Variable | Purpose |
|----------|---------|
| `_total` | Total number of animations in this sync group |
| `_ready` | Count of animations marked as READY |
| `_started` | Count of animations that have begun |
| `_completed` | Count of animations that have finished |
| `_first` | First animation (used for progress callback) |

**Sync Flow:**

```
1. All animations added to queue
2. First tick: _ready increments for each
3. When _ready === _total, all start simultaneously
4. Each animation waits by incrementing timeStart
```

### 1.6 Speed Control Implementation

```typescript
if (speed !== 1) {
    // On the first frame we may have a shorter delta
    activeCall.timeStart = timeStart += Math.min(
        deltaTime,
        timeCurrent - timeStart
    ) * (1 - speed);
}
```

**Effect Analysis:**

| Speed Value | Effect on timeStart | Result |
|-------------|---------------------|--------|
| `speed = 1` | No change | Normal playback |
| `speed = 2` | `timeStart -= deltaTime` | 2x faster (start time pushed back) |
| `speed = 0.5` | `timeStart += deltaTime * 0.5` | 2x slower (start time pushed forward) |

**Mathematical Relationship:**

```
percentComplete = (timeCurrent - timeStart) / duration

With speed adjustment:
timeStart' = timeStart + deltaTime * (1 - speed)

Effective elapsed time:
elapsed' = timeCurrent - timeStart'
         = timeCurrent - timeStart - deltaTime * (1 - speed)
         = elapsed - deltaTime * (1 - speed)

For speed > 1: elapsed' > elapsed (faster)
For speed < 1: elapsed' < elapsed (slower)
```

### 1.7 Dual-Loop Structure

The tick function uses two separate loops for a critical reason:

```typescript
// Loop 1: Mark animations as READY and handle pause state
for (activeCall = State.first;
     activeCall && activeCall !== State.firstNew;
     activeCall = activeCall._next) {

    // Check for deleted elements
    if (!element.parentNode || !data) {
        freeAnimationCall(activeCall);
        continue;
    }

    // Handle paused state
    if (flags & AnimationFlags.PAUSED) {
        activeCall.timeStart += deltaTime;
        continue;
    }

    // Mark as READY
    if (!(flags & AnimationFlags.READY)) {
        activeCall._flags |= AnimationFlags.READY;
        options._ready++;
    }
}

// Loop 2: Process READY animations
for (activeCall = State.first;
     activeCall && activeCall !== State.firstNew;
     activeCall = nextCall) {

    nextCall = activeCall._next;

    // Skip non-ready or paused
    if (!(flags & AnimationFlags.READY) || (flags & AnimationFlags.PAUSED)) {
        continue;
    }

    // Check sync readiness
    if ((flags & AnimationFlags.SYNC) && options._ready < options._total) {
        activeCall.timeStart += deltaTime;
        continue;
    }

    // ... process animation frame
}
```

**Why Two Loops?**

The dual-loop structure ensures that **all sync animations receive the exact same `timeStart` value**. If there were a single loop:

1. Animation A gets marked READY, starts immediately
2. Animation B gets marked READY on next iteration, starts slightly later

With two loops:
1. Loop 1: Mark ALL animations as READY, count them
2. Loop 2: Now that `_ready === _total`, all sync animations start with identical `timeStart`

### 1.8 Frame Time Threshold Logic

```typescript
if (deltaTime >= defaults.minFrameTime || !lastTick) {
    lastTick = timeCurrent;
    // Process frame
}
```

**Purpose:**

1. **Prevents excessive CPU usage** - Only processes when enough time has elapsed
2. **Handles variable frame rates** - Adapts to system capabilities
3. **First frame exception** - `!lastTick` ensures the first frame always processes

**Default Values:**

```typescript
defaults.fpsLimit = 60;
defaults.minFrameTime = FUZZY_MS_PER_SECOND / 60
                      = 980 / 60
                      = 16.333... ms
```

---

## 2. Tween Pattern Matching System

### 2.1 Overview

Velocity's tween system can animate between complex CSS value strings by finding patterns and interpolating numeric values within them.

**File:** `tweens.ts`

### 2.2 The `rxToken` Regex and Tokenization

```typescript
const rxToken = /((?:[+\-*/]=)?(?:[+-]?\d*\.\d+|[+-]?\d+)[a-z%]*|(?:.(?!$|[+-]?\d|[+\-*/]=[+-]?\d))+.|.)/g,
    rxNumber = /^([+\-*/]=)?([+-]?\d*\.\d+|[+-]?\d+)(.*)$/;
```

**`rxToken` Breakdown:**

```
rxToken = /
    # Group 1: Number with optional operator and unit
    (
        (?:[+\-*/]=)?           # Optional relative operator: +=, -=, *=, /=
        (?:
            [+-]?\d*\.\d+       # Decimal number: .5, 1.5, -1.5
            |
            [+-]?\d+            # Integer: 1, -1, +1
        )
        [a-z%]*                 # Optional unit: px, em, %, etc.
    )
    |
    # Group 2: Non-numeric characters (operators, letters, etc.)
    (
        (?:
            .                   # Any character
            (?!                 # NOT followed by:
                $               # End of string
                |
                [+-]?\d         # A number
                |
                [+\-*/]=[+-]?\d # A relative operator with number
            )
        )+
        |
        .                       # Any single character (fallback)
    )
/g
```

**Tokenization Examples:**

```javascript
// Example 1: Simple pixel value
"100px".match(rxToken)
// Result: ["100px"]

// Example 2: Transform with multiple values
"translateX(50px)".match(rxToken)
// Result: ["translateX(", "50", "px)", ""]

// Example 3: Complex transform
"translate(10px, 20px) rotate(45deg)".match(rxToken)
// Result: [
//   "translate(", "10", "px, ", "20", "px) ",
//   "rotate(", "45", "deg)"
// ]

// Example 4: Relative animation
"+=50px".match(rxToken)
// Result: ["+=50px"]

// Example 5: RGB color
"rgb(255, 128, 64)".match(rxToken)
// Result: ["rgb(", "255", ", ", "128", ", ", "64", ")"]
```

### 2.3 The `findPattern` Algorithm Step-by-Step

The `findPattern` function compares multiple CSS value strings and extracts a pattern with animatable numeric sequences.

**Function Signature:**
```typescript
export function findPattern(parts: ReadonlyArray<string>, propertyName: string): Sequence
```

**Algorithm Steps:**

```typescript
// Step 1: Tokenize all input strings
for (let part = 0; part < partsLength; part++) {
    if (isString(parts[part])) {
        tokens[part] = cloneArray(parts[part].match(rxToken));
        indexes[part] = 0;
        numbers = numbers || tokens[part].length > 1;
    }
}

// Step 2: Iterate through tokens, finding patterns
while (more) {
    const bits: ([number, string] | [number, string, boolean])[] = [],
        units: string[] = [];
    let text: string, isUnitless = false, hasNumbers = false;

    // Step 2a: Extract tokens from each part
    for (let part = 0; part < partsLength; part++) {
        const index = indexes[part]++,
            token = tokens[part][index];

        if (token) {
            const num = token.match(rxNumber);

            if (num) {
                // It's a number with optional unit
                bits[part] = [parseFloat(num[2]), num[3], !!num[1]];
                units.push(num[3]);
            } else {
                // It's a string delimiter
                text = token;
            }
        }
    }

    // Step 2b: Process collected tokens
    if (text) {
        addString(text);  // Add to pattern as static text
    } else if (units.length) {
        if (units.length === 1) {
            // All same units - simple animation
            pattern.push(false);  // false = numeric placeholder
            for (let part = 0; part < partsLength; part++) {
                sequence[part].push(bits[part][0]);  // Store number
            }
            addString(units[0]);  // Add unit to pattern
        } else {
            // Multiple units - wrap in calc()
            addString("calc(");
            // ... handle unit conversion
            addString(")");
        }
    }
}
```

**Example Walkthrough:**

```javascript
// Animating from "translateX(0px)" to "translateX(100px)"
findPattern(["translateX(0px)", "translateX(100px)"], "transform")

// Step 1: Tokenize
tokens[0] = ["translateX(", "0", "px)"]
tokens[1] = ["translateX(", "100", "px)"]

// Step 2: First iteration
text = "translateX("  // Both match
addString("translateX(")

// Step 3: Second iteration
bits[0] = [0, "px"]
bits[1] = [100, "px"]
units = ["px", "px"] → unique: ["px"]

units.length === 1, so:
pattern.push(false)  // Numeric placeholder
sequence[0].push(0)
sequence[1].push(100)
addString("px)")

// Result:
{
    pattern: ["translateX(", false, "px)"],
    [0]: [0],  // Start values
    [1]: [100] // End values
}
```

### 2.4 Unit Conversion via `calc()` Nesting

When animating between different units, Velocity wraps values in CSS `calc()` expressions:

```typescript
if (units.length > 1) {
    addString("calc(");
    const patternCalc = pattern.length - 1;

    for (let i = 0; i < units.length; i++) {
        const unit = units[i],
            firstLetter = unit[0],
            isComplex = firstLetter === "*" || firstLetter === "/",
            isMaths = isComplex || firstLetter === "+" || firstLetter === "-";

        if (isComplex) {
            pattern[patternCalc] += "(";
            addString(")");
        }
        if (i) {
            addString(` ${isMaths ? firstLetter : "+"} `);
        }
        pattern.push(false);
        for (let part = 0; part < partsLength; part++) {
            const bit = bits[part],
                value = bit[1] === unit
                    ? bit[0]
                    : bit.length === 3
                        ? sequence[part - 1][sequence[part - 1].length - 1]
                        : isComplex ? 1 : 0;

            sequence[part].push(value);
        }
        addString(isMaths ? unit.substring(1) : unit);
    }
    addString(")");
}
```

**Example: Animating `width` from `50%` to `200px`:**

```javascript
findPattern(["50%", "200px"], "width")

// Result pattern:
pattern: [false, ""]
sequence[0]: [50, 0]  // 50% + 0px
sequence[1]: [0, 200] // 0% + 200px

// Rendered as: calc(50% + 200px)
// At 50% completion: calc(25% + 100px)
```

**Generated CSS:**
```css
/* Start */
width: calc(50% + 0px);

/* Middle (50%) */
width: calc(25% + 100px);

/* End */
width: calc(0% + 200px);  /* Simplified to 200px */
```

### 2.5 RGBA Rounding Handling

CSS `rgb()` and `rgba()` require integer values for R, G, B channels (alpha can be fractional).

```typescript
// In findPattern():
for (let i = 0, inRGB = 0; i < pattern.length; i++) {
    const text = pattern[i];

    if (isString(text)) {
        if (inRGB && text.indexOf(",") >= 0) {
            inRGB++;
        } else if (text.indexOf("rgb") >= 0) {
            inRGB = 1;
        }
    } else if (inRGB) {
        if (inRGB < 4) {
            pattern[i] = true;  // Mark for rounding
        } else {
            inRGB = 0;
        }
    }
}
```

**Pattern Markers:**
- `false` = numeric value (no rounding)
- `true` = numeric value (round to integer)
- `string` = static text

**Example:**

```javascript
findPattern(["rgb(0, 0, 0)", "rgb(255, 128, 64)"], "color")

// Pattern after RGB processing:
pattern: ["rgb(", true, ", ", true, ", ", true, ")"]
//                        ↑                      ↑
//                   Round these              Don't round

// sequence[0]: [0, 0, 0]
// sequence[1]: [255, 128, 64]
```

**Rendering with rounding:**

```typescript
// In tick():
const result = easing(tweenPercent, startValue, endValue, property);
currentValue += pattern[i] !== true ? result : Math.round(result);
```

### 2.6 `explodeTween` - Converting String Tweens to Sequences

```typescript
function explodeTween(propertyName: string, tween: VelocityTween, duration: number, starting?: boolean) {
    const startValue: string = tween.start,
        endValue: string = tween.end;

    if (!isString(endValue) || !isString(startValue)) {
        return;
    }

    let sequence: Sequence = findPattern([startValue, endValue], propertyName);

    // Fallback: If pattern matching fails, try number replacement
    if (!sequence && starting) {
        const startNumbers = startValue.match(/\d\.?\d*/g) || ["0"],
            count = startNumbers.length;
        let index = 0;

        sequence = findPattern([
            endValue.replace(/\d+\.?\d*/g, () => {
                return startNumbers[index++ % count];
            }),
            endValue
        ], propertyName);
    }

    if (sequence) {
        sequence[0].percent = 0;
        sequence[1].percent = 1;
        tween.sequence = sequence;

        // Handle special easings
        switch (tween.easing) {
            case Easings["at-start"]:
            case Easings["during"]:
            case Easings["at-end"]:
                sequence[0].easing = sequence[1].easing = tween.easing;
                break;
        }
    }
}
```

**Fallback Strategy:**

When `findPattern` can't match patterns (e.g., "rotate(0deg)" to "scale(1)"), the fallback:

1. Extracts numbers from start: `["0"]`
2. Replaces numbers in end with start numbers: `"scale(0)"`
3. Re-runs `findPattern` with matched structure

### 2.7 Special Easing Cases: "at-start", "during", "at-end"

These are string-specific easings that don't interpolate:

```typescript
// string.ts
export function atStart(percentComplete, startValue, endValue) {
    return percentComplete === 0 ? startValue : endValue;
}

export function during(percentComplete, startValue, endValue) {
    return percentComplete === 0 || percentComplete === 1 ? startValue : endValue;
}

export function atEnd(percentComplete, startValue, endValue) {
    return percentComplete === 1 ? endValue : startValue;
}
```

**Use Cases:**

| Easing | Behavior | Example |
|--------|----------|---------|
| `at-start` | Switch to end value immediately after 0% | `display: none` → `display: block` |
| `during` | Only show end value while animating (0 < t < 1) | `visibility: hidden` → `visibility: visible` |
| `at-end` | Stay at start value until 100% | Final state commits |

**Implementation in tweens.ts:**

```typescript
// In returnStringType():
const isDisplay = propertyName === "display",
    isVisibility = propertyName === "visibility";

for (let part = 0; part < partsLength; part++) {
    sequence[part].easing = validateEasing(
        (isDisplay && value === "none") ||
        (isVisibility && value === "hidden") ||
        (!isDisplay && !isVisibility)
            ? "at-end"
            : "at-start",
        400
    );
}
```

---

## 3. Cubic Bezier Mathematics

### 3.1 Overview

Cubic Bezier curves define the rate of change (easing) over time. Velocity uses the standard CSS cubic-bezier format: `cubic-bezier(x1, y1, x2, y2)`.

**File:** `bezier.ts`

### 3.2 Cubic Bezier Fundamentals

A cubic Bezier curve is defined by four points:
- P0 = (0, 0) - Start point (fixed)
- P1 = (x1, y1) - First control point
- P2 = (x2, y2) - Second control point
- P3 = (1, 1) - End point (fixed)

**Parametric Equation:**

```
B(t) = (1-t)³·P0 + 3(1-t)²·t·P1 + 3(1-t)·t²·P2 + t³·P3

where t ∈ [0, 1]
```

**Component Form:**

```
x(t) = (1-t)³·0 + 3(1-t)²·t·x1 + 3(1-t)·t²·x2 + t³·1
     = 3(1-t)²·t·x1 + 3(1-t)·t²·x2 + t³

y(t) = (1-t)³·0 + 3(1-t)²·t·y1 + 3(1-t)·t²·y2 + t³·1
     = 3(1-t)²·t·y1 + 3(1-t)·t²·y2 + t³
```

**Simplified Polynomial Form:**

Velocity uses a more efficient form:

```typescript
function A(aA1, aA2) { return 1 - 3 * aA2 + 3 * aA1; }
function B(aA1, aA2) { return 3 * aA2 - 6 * aA1; }
function C(aA1) { return 3 * aA1; }

function calcBezier(aT, aA1, aA2) {
    return ((A(aA1, aA2) * aT + B(aA1, aA2)) * aT + C(aA1)) * aT;
}
```

**Expanded:**

```
x(t) = ((1 - 3x2 + 3x1)·t + (3x2 - 6x1))·t + 3x1)·t
     = (1 - 3x2 + 3x1)·t³ + (3x2 - 6x1)·t² + 3x1·t

y(t) = ((1 - 3y2 + 3y1)·t + (3y2 - 6y1))·t + 3y1)·t
```

### 3.3 Newton-Raphson Iteration

To find the Y value for a given X (time progress), we must **invert** the X curve. This requires solving:

```
x(t) = givenX  for t
```

Velocity uses Newton-Raphson iteration:

```typescript
const NEWTON_ITERATIONS = 4,
    NEWTON_MIN_SLOPE = 0.001;

function getSlope(aT, aA1, aA2) {
    return 3 * A(aA1, aA2) * aT * aT + 2 * B(aA1, aA2) * aT + C(aA1);
}

function newtonRaphsonIterate(aX, aGuessT) {
    for (let i = 0; i < NEWTON_ITERATIONS; ++i) {
        const currentSlope = getSlope(aGuessT, mX1, mX2);

        if (currentSlope === 0) {
            return aGuessT;  // Avoid division by zero
        }

        const currentX = calcBezier(aGuessT, mX1, mX2) - aX;
        aGuessT -= currentX / currentSlope;
    }

    return aGuessT;
}
```

**Newton-Raphson Formula:**

```
tₙ₊₁ = tₙ - f(tₙ) / f'(tₙ)

Where:
  f(t) = x(t) - targetX
  f'(t) = dx/dt (slope of x curve)
```

**Iteration Example:**

```
Given: x1=0.42, y1=0, x2=1, y2=1 (easeIn)
Target: x = 0.5 (50% through animation)

Initial guess: t₀ = 0.5

Iteration 1:
  x(0.5) = 0.5(3 - 3·1 + 0.42·3) = 0.5(3 - 3 + 1.26) = 0.63
  slope = 3(0.5)²(1-3·1+3·0.42) + 2(0.5)(3·1-6·0.42) + 3·0.42
        = 0.75(-0.74) + 1(0.48) + 1.26 = -0.555 + 0.48 + 1.26 = 1.185

  t₁ = 0.5 - (0.63 - 0.5) / 1.185 = 0.5 - 0.11 = 0.39

Iteration 2-4: Continue refining...
Final: t ≈ 0.38
```

### 3.4 Spline Sample Table Precomputation

To accelerate the Newton-Raphson search, Velocity precomputes 11 sample points:

```typescript
const kSplineTableSize = 11,
    kSampleStepSize = 1 / (kSplineTableSize - 1);  // 0.1

const mSampleValues = new Float32Array(kSplineTableSize);

function calcSampleValues() {
    for (let i = 0; i < kSplineTableSize; ++i) {
        mSampleValues[i] = calcBezier(i * kSampleStepSize, mX1, mX2);
    }
}
```

**Sample Table:**

```
Index | t value | x(t) stored
------|---------|-------------
  0   |  0.0    | x(0.0) = 0.0
  1   |  0.1    | x(0.1)
  2   |  0.2    | x(0.2)
  ... |  ...    | ...
 10   |  1.0    | x(1.0) = 1.0
```

### 3.5 Binary Search Fallback

When Newton-Raphson fails (flat slope), Velocity falls back to binary subdivision:

```typescript
const SUBDIVISION_PRECISION = 0.0000001,
    SUBDIVISION_MAX_ITERATIONS = 10;

function binarySubdivide(aX, aA, aB) {
    let currentX, currentT, i = 0;

    do {
        currentT = aA + (aB - aA) / 2;
        currentX = calcBezier(currentT, mX1, mX2) - aX;
        if (currentX > 0) {
            aB = currentT;
        } else {
            aA = currentT;
        }
    } while (Math.abs(currentX) > SUBDIVISION_PRECISION &&
             ++i < SUBDIVISION_MAX_ITERATIONS);

    return currentT;
}
```

**Binary Search Visualization:**

```
Target X = 0.5
Interval [0, 1]

Iteration 1: t = 0.5, x(0.5) = 0.63 > 0.5 → new interval [0, 0.5]
Iteration 2: t = 0.25, x(0.25) = 0.2 < 0.5 → new interval [0.25, 0.5]
Iteration 3: t = 0.375, x(0.375) = 0.45 < 0.5 → new interval [0.375, 0.5]
Iteration 4: t = 0.4375, x(0.4375) = 0.55 > 0.5 → new interval [0.375, 0.4375]
...
```

### 3.6 `getTForX` - Inverting the Bezier X Curve

The complete inversion algorithm combines all techniques:

```typescript
function getTForX(aX) {
    const lastSample = kSplineTableSize - 1;
    let intervalStart = 0, currentSample = 1;

    // Step 1: Find interval using sample table
    for (; currentSample !== lastSample &&
           mSampleValues[currentSample] <= aX;
           ++currentSample) {
        intervalStart += kSampleStepSize;
    }
    --currentSample;

    // Step 2: Linear interpolation guess
    const dist = (aX - mSampleValues[currentSample]) /
                 (mSampleValues[currentSample + 1] - mSampleValues[currentSample]),
        guessForT = intervalStart + dist * kSampleStepSize,
        initialSlope = getSlope(guessForT, mX1, mX2);

    // Step 3: Choose solving method
    if (initialSlope >= NEWTON_MIN_SLOPE) {
        return newtonRaphsonIterate(aX, guessForT);
    } else if (initialSlope === 0) {
        return guessForT;
    } else {
        return binarySubdivide(aX, intervalStart,
                               intervalStart + kSampleStepSize);
    }
}
```

**Algorithm Flow:**

```
                    ┌─────────────────┐
                    │  Input: targetX │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │ Find interval   │
                    │ using samples   │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │ Linear guess    │
                    │ for initial t   │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │ Calculate slope │
                    │ at guess point  │
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────▼────┐        ┌────▼────┐        ┌────▼────┐
    │slope >= │        │slope = 0│        │slope <  │
    │0.001    │        │         │        │0.001    │
    └────┬────┘        └────┬────┘        └────┬────┘
         │                  │                  │
    ┌────▼────┐             │            ┌────▼────┐
    │ Newton  │             │            │ Binary  │
    │ Raphson │             │            │ Search  │
    │ (4 iter)│             │            │         │
    └────┬────┘             │            └────┬────┘
         │                  │                  │
         └──────────────────┼──────────────────┘
                            │
                   ┌────────▼────────┐
                   │  Output: t      │
                   └─────────────────┘
```

### 3.7 Final Y Calculation

Once we have `t` for the given `x` (percentComplete), calculating the eased value is straightforward:

```typescript
const f = (percentComplete: number, startValue: number, endValue: number, property?: string) => {
    if (!precomputed) {
        precompute();
    }
    if (percentComplete === 0) {
        return startValue;
    }
    if (percentComplete === 1) {
        return endValue;
    }
    if (mX1 === mY1 && mX2 === mY2) {
        return startValue + percentComplete * (endValue - startValue);  // Linear
    }

    return startValue + calcBezier(getTForX(percentComplete), mY1, mY2) * (endValue - startValue);
};
```

**Complete Formula:**

```
easedValue = startValue + Y(t) · (endValue - startValue)

where t = getTForX(percentComplete)
      Y(t) = calcBezier(t, y1, y2)
```

**Step-by-step Example:**

```javascript
// easeIn: cubic-bezier(0.42, 0, 1, 1)
// Request: 50% complete, animate from 0 to 100

const mX1 = 0.42, mY1 = 0, mX2 = 1, mY2 = 1;

// Step 1: Find t where x(t) = 0.5
t = getTForX(0.5);  // ≈ 0.38 (from Newton-Raphson)

// Step 2: Calculate Y(t)
Y(0.38) = ((1 - 3·1 + 3·0) · 0.38 + (3·1 - 6·0)) · 0.38 + 3·0) · 0.38
        = ((-2) · 0.38 + 3) · 0.38 · 0.38
        = (-0.76 + 3) · 0.1444
        = 2.24 · 0.1444
        ≈ 0.324

// Step 3: Apply to value range
result = 0 + 0.324 · (100 - 0) = 32.4

// At 50% time, we're only at 32.4% of the animation (easeIn)
```

### 3.8 All Registered Easing Functions with Control Points

**Standard CSS Easings:**

| Name | Control Points (x1, y1, x2, y2) | Visual Description |
|------|--------------------------------|-------------------|
| `ease` | (0.25, 0.1, 0.25, 1) | Slow start, fast middle, slow end |
| `easeIn` / `ease-in` | (0.42, 0, 1, 1) | Starts slow, accelerates |
| `easeOut` / `ease-out` | (0, 0, 0.58, 1) | Starts fast, decelerates |
| `easeInOut` / `ease-in-out` | (0.42, 0, 0.58, 1) | Slow start and end, fast middle |

**Sine Easings:**

| Name | Control Points | Description |
|------|---------------|-------------|
| `easeInSine` | (0.47, 0, 0.745, 0.715) | Gentle acceleration |
| `easeOutSine` | (0.39, 0.575, 0.565, 1) | Gentle deceleration |
| `easeInOutSine` | (0.445, 0.05, 0.55, 0.95) | Symmetric sine wave |

**Quad Easings (Quadratic):**

| Name | Control Points |
|------|---------------|
| `easeInQuad` | (0.55, 0.085, 0.68, 0.53) |
| `easeOutQuad` | (0.25, 0.46, 0.45, 0.94) |
| `easeInOutQuad` | (0.455, 0.03, 0.515, 0.955) |

**Cubic Easings:**

| Name | Control Points |
|------|---------------|
| `easeInCubic` | (0.55, 0.055, 0.675, 0.19) |
| `easeOutCubic` | (0.215, 0.61, 0.355, 1) |
| `easeInOutCubic` | (0.645, 0.045, 0.355, 1) |

**Quart Easings (Fourth Power):**

| Name | Control Points |
|------|---------------|
| `easeInQuart` | (0.895, 0.03, 0.685, 0.22) |
| `easeOutQuart` | (0.165, 0.84, 0.44, 1) |
| `easeInOutQuart` | (0.77, 0, 0.175, 1) |

**Quint Easings (Fifth Power):**

| Name | Control Points |
|------|---------------|
| `easeInQuint` | (0.755, 0.05, 0.855, 0.06) |
| `easeOutQuint` | (0.23, 1, 0.32, 1) |
| `easeInOutQuint` | (0.86, 0, 0.07, 1) |

**Exponential Easings:**

| Name | Control Points | Description |
|------|---------------|-------------|
| `easeInExpo` | (0.95, 0.05, 0.795, 0.035) | Very slow start, sudden acceleration |
| `easeOutExpo` | (0.19, 1, 0.22, 1) | Sustained speed, sudden stop |
| `easeInOutExpo` | (1, 0, 0, 1) | Instant jump to full speed |

**Circular Easings:**

| Name | Control Points |
|------|---------------|
| `easeInCirc` | (0.6, 0.04, 0.98, 0.335) |
| `easeOutCirc` | (0.075, 0.82, 0.165, 1) |
| `easeInOutCirc` | (0.785, 0.135, 0.15, 0.86) |

**Other Built-in Easings:**

| Name | Type | Formula/Description |
|------|------|---------------------|
| `linear` | Linear | `f(t) = t` |
| `swing` | Cosine | `f(t) = 0.5 - cos(t·π)/2` |
| `spring` | Exponential decay | `f(t) = 1 - cos(t·4.5π)·e^(-t·6)` |

---

## 4. Queue System

### 4.1 Overview

Velocity's queue system manages multiple animations per element, supporting both named queues and the default queue.

**File:** `queue.ts`

### 4.2 Doubly-Linked List Structure

Animations are stored in a doubly-linked list for efficient insertion and removal:

```typescript
interface AnimationCall {
    _next?: AnimationCall;  // Next animation in queue
    _prev?: AnimationCall;  // Previous animation in queue
    _flags: number;
    // ... other properties
}
```

**Visual Representation:**

```
State.first → [Animation1] ↔ [Animation2] ↔ [Animation3] ← State.last
                  ↑              ↑              ↑
               _next→2       _next→3        _next→undefined
               _prev←undef   _prev←1        _prev←2
```

### 4.3 Named Queue vs Unnamed Queue Storage

Queues are stored in `ElementData`:

```typescript
interface ElementData {
    queueList: {[name: string]: AnimationCall};      // Named queues
    lastAnimationList: {[name: string]: AnimationCall};
    lastFinishList: {[name: string]: number};
}
```

**Queue Naming:**
- Empty string `""` = Default queue
- Any other string = Named queue
- `false` = Immediate execution (bypass queue)

**Storage Locations:**

| Queue Type | Stored In | Behavior |
|------------|-----------|----------|
| Default (`""`) | `data.queueList[""]` | Standard FIFO |
| Named (`"fade"`) | `data.queueList["fade"]` | Independent FIFO |
| Immediate (`false`) | `State.first/last` | Global list, runs immediately |

### 4.4 State.first, State.last, State.firstNew

```typescript
interface VelocityState {
    first?: AnimationCall;     // First active animation
    last?: AnimationCall;      // Last active animation
    firstNew?: AnimationCall;  // First unprocessed animation
}
```

**Purpose:**

| Variable | Purpose | Reset When |
|----------|---------|------------|
| `first` | Head of active animation list | No more animations |
| `last` | Tail of active animation list | No more animations |
| `firstNew` | First animation needing tween expansion | All tweens expanded |

**Flow Diagram:**

```
┌─────────────────────────────────────────────────────────────┐
│ Queue Addition Phase                                         │
│                                                               │
│  data.queueList[""] = [Anim1] → [Anim2]                      │
│  State.firstNew → [Anim2] (needs expansion)                 │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Tick Phase 1: Expand Tweens                                  │
│                                                               │
│  while (State.firstNew) {                                    │
│      validateTweens(State.firstNew);                         │
│  }                                                           │
│  // firstNew advanced during expansion                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Tick Phase 2: Process Active Animations                      │
│                                                               │
│  for (call = State.first; call !== State.firstNew; ...) {   │
│      // Process only expanded animations                    │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
```

### 4.5 `queue()` Function

```typescript
export function queue(element: HTMLorSVGElement, animation: AnimationCall, queueName: string | false): void {
    const data = Data(element);

    if (queueName !== false) {
        data.lastAnimationList[queueName] = animation;
    }

    if (queueName === false) {
        // Immediate execution
        animate(animation);
    } else {
        if (!isString(queueName)) {
            queueName = "";
        }
        let last = data.queueList[queueName];

        if (!last) {
            if (last === null) {
                data.queueList[queueName] = animation;
            } else {
                data.queueList[queueName] = null;  // Mark as running
                animate(animation);
            }
        } else {
            // Append to end of linked list
            while (last._next) {
                last = last._next;
            }
            last._next = animation;
            animation._prev = last;
        }
    }
}
```

**Queue State Values:**

| Value | Meaning |
|-------|---------|
| `undefined` | Queue doesn't exist |
| `null` | Queue exists but is empty (actively running) |
| `AnimationCall` | First animation in queue |

### 4.6 `dequeue()` Function

```typescript
export function dequeue(element: HTMLorSVGElement, queueName?: string | boolean, skip?: boolean): AnimationCall {
    if (queueName !== false) {
        if (!isString(queueName)) {
            queueName = "";
        }
        const data = Data(element),
            animation = data.queueList[queueName];

        if (animation) {
            data.queueList[queueName] = animation._next || null;
            if (!skip) {
                animate(animation);
            }
        } else if (animation === null) {
            delete data.queueList[queueName];
        }

        return animation;
    }
}
```

### 4.7 `freeAnimationCall()` Cleanup Process

```typescript
export function freeAnimationCall(animation: AnimationCall): void {
    const next = animation._next,
        prev = animation._prev,
        queueName = animation.queue == null ? animation.options.queue : animation.queue;

    // Update State.firstNew if needed
    if (State.firstNew === animation) {
        State.firstNew = next;
    }

    // Remove from State list
    if (State.first === animation) {
        State.first = next;
    } else if (prev) {
        prev._next = next;
    }

    if (State.last === animation) {
        State.last = prev;
    } else if (next) {
        next._prev = prev;
    }

    // Clear references for named queues
    if (queueName) {
        const data = Data(animation.element);
        if (data) {
            animation._next = animation._prev = undefined;
        }
    }
}
```

**Cleanup Steps:**

1. **Update `firstNew`** if this animation hasn't been processed yet
2. **Remove from State list** by updating neighboring pointers
3. **Clear references** to allow garbage collection

**Visual Removal:**

```
Before removal of Animation2:
[Anim1] ↔ [Anim2] ↔ [Anim3]

After removal:
[Anim1] ──────→ [Anim3]
   ↑              ↑
   └───────←──────┘

Code:
Anim1._next = Anim3
Anim3._prev = Anim1
Anim2._next = undefined
Anim2._prev = undefined
```

---

## 5. CSS Property Handling

### 5.1 Color Normalization

Velocity normalizes all color values to `rgba()` format for consistent interpolation.

**File:** `css/fixColors.ts`

**Color Formats Supported:**
- Hex 3-digit: `#FFF`, `#abc`
- Hex 6-digit: `#FFFFFF`, `#aabbcc`
- RGB: `rgb(255, 128, 64)`
- RGBA: `rgba(255, 128, 64, 0.5)`
- Color names: `red`, `blue`, `transparent`

**Regex Patterns:**

```typescript
const rxColor6 = /^#([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i,   // #RRGGBB
    rxColor3 = /^#([a-f\d])([a-f\d])([a-f\d])$/i,              // #RGB
    rxColorName = /(rgba?\(\s*)?(\b[a-z]+\b)/g,                // color names
    rxRGB = /rgb(a?)\(([^\)]+)\)/gi,                           // rgb/rgba
    rxSpaces = /\s+/g;
```

**Conversion Chain:**

```
hex → rgba() ← rgb/rgba() ← color names
                  ↓
            Normalized format for tweening
```

**fixColors() Implementation:**

```typescript
export function fixColors(str: string): string {
    return str
        .replace(rxColor6, makeRGBA)
        .replace(rxColor3, ($0, r, g, b) => {
            return makeRGBA($0, r + r, g + g, b + b);  // Expand #RGB to #RRGGBB
        })
        .replace(rxColorName, ($0, $1, $2) => {
            if (ColorNames[$2]) {
                return ($1 ? $1 : "rgba(") + ColorNames[$2] + ($1 ? "" : ",1)");
            }
            return $0;
        })
        .replace(rxRGB, ($0, $1, $2: string) => {
            return `rgba(${$2.replace(rxSpaces, "") + ($1 ? "" : ",1")})`;
        });
}
```

**Color Name Table (sample):**

```typescript
export const ColorNames: {[name: string]: string} = {
    "black": "0,0,0",
    "white": "255,255,255",
    "red": "255,0,0",
    "green": "0,128,0",
    "blue": "0,0,255",
    // ... full list in colors.ts
};
```

**Conversion Examples:**

```javascript
fixColors("#FFF")           // → "rgba(255,255,255,1)"
fixColors("#aabbcc")        // → "rgba(170,187,204,1)"
fixColors("red")            // → "rgba(255,0,0,1)"
fixColors("rgba(255, 0, 0, 0.5)")  // → "rgba(255,0,0,0.5)"
fixColors("rgb(255, 128, 64)")     // → "rgba(255,128,64,1)"
```

### 5.2 Unit Normalization Across Browsers

Velocity handles browser inconsistencies in unit reporting:

**File:** `css/getPropertyValue.ts`

**Special Cases:**

1. **Auto TRBL values** (top, right, bottom, left):

```typescript
if (computedValue === "auto") {
    switch (property) {
        case "width":
        case "height":
            computedValue = getWidthHeight(element, property);
            break;

        case "top":
        case "left":
            const position = getPropertyValue(element, "position");
            if (position === "fixed" || (position === "absolute")) {
                computedValue = element.getBoundingClientRect()[property] + "px";
                break;
            }
        // Fallthrough
        default:
            computedValue = "0px";
            break;
    }
}
```

2. **Display none handling:**

```typescript
if (computedStyle["display"] === "none") {
    switch (property) {
        case "width":
        case "height":
            setPropertyValue(element, "display", "auto");
            computedValue = getWidthHeight(element, property);
            setPropertyValue(element, "display", "none");
            return String(computedValue);
    }
}
```

3. **Border color normalization:**

```typescript
/* IE and Firefox only return individual border side colors.
   We return top border's color as a polyfill. */
```

### 5.3 setPropertyValue Chain and Vendor Prefix Handling

**File:** `css/setPropertyValue.ts`

```typescript
export function setPropertyValue(element: HTMLorSVGElement, propertyName: string, propertyValue: any, fn?: VelocityNormalizationsFn) {
    const noCache = NoCacheNormalizations.has(propertyName),
        data = !noCache && Data(element);

    if (noCache || (data && data.cache[propertyName] !== propertyValue)) {
        if (!noCache) {
            data.cache[propertyName] = propertyValue || undefined;
        }
        fn = fn || getNormalization(element, propertyName);
        if (fn) {
            fn(element, propertyValue);
        }
        if (Velocity.debug >= 2) {
            console.info(`Set "${propertyName}": "${propertyValue}"`, element);
        }
    }
}
```

**Normalization Chain:**

```
setPropertyValue()
    │
    ├─► Check cache (skip if unchanged)
    │
    ├─► Get normalization function
    │   │
    │   └─► Search through element types
    │       (HTMLElement, SVGElement, etc.)
    │
    └─► Apply normalization
        │
        ├─► CSS properties → element.style
        ├─► SVG attributes → element.setAttribute()
        └─► Special properties → custom handlers
```

**Vendor Prefix Handling:**

```typescript
// In normalizations/style.ts
const prefixedProperty = getPrefixedProperty(element, propertyName);
element.style[prefixedProperty] = propertyValue;
```

**Prefix Detection:**

```typescript
// Uses prefixElement cache
const prefixElement = document.createElement("div");
const supported = propertyName in prefixElement.style;
```

---

## 6. Easing Functions Reference

### 6.1 Mathematical Formulas

**Linear:**
```
f(t) = t
```

**Swing (Cosine):**
```
f(t) = 0.5 - cos(t·π) / 2
```

**Spring:**
```
f(t) = 1 - cos(t·4.5π) · e^(-6t)
```

**Step:**
```
f(t) = startValue + round(t · steps) · (1/steps) · (endValue - startValue)
```

**Spring RK4 (Runge-Kutta 4th Order):**

The spring physics simulation uses RK4 integration:

```typescript
// State: {x: position, v: velocity, tension, friction}
function springAccelerationForState(state) {
    return (-state.tension * state.x) - (state.friction * state.v);
}

// RK4 Integration
function springIntegrateState(state, dt) {
    const a = {dx: state.v, dv: springAccelerationForState(state)},
        b = springEvaluateStateWithDerivative(state, dt * 0.5, a),
        c = springEvaluateStateWithDerivative(state, dt * 0.5, b),
        d = springEvaluateStateWithDerivative(state, dt, c),
        dxdt = (a.dx + 2*(b.dx + c.dx) + d.dx) / 6,
        dvdt = (a.dv + 2*(b.dv + c.dv) + d.dv) / 6;

    state.x += dxdt * dt;
    state.v += dvdt * dt;
    return state;
}
```

**Bounce:**

```typescript
function easeOutBouncePercent(t) {
    if (t < 1/2.75) {
        return 7.5625 * t * t;
    }
    if (t < 2/2.75) {
        return 7.5625 * (t -= 1.5/2.75) * t + 0.75;
    }
    if (t < 2.5/2.75) {
        return 7.5625 * (t -= 2.25/2.75) * t + 0.9375;
    }
    return 7.5625 * (t -= 2.625/2.75) * t + 0.984375;
}
```

**Elastic:**

```typescript
// easeInElastic
f(t) = -amplitude · 2^(10(t-1)) · sin((t - s) · 2π / period)

// easeOutElastic
f(t) = amplitude · 2^(-10t) · sin((t - s) · 2π / period) + 1

// easeInOutElastic
f(t) = {
    t < 0.5: -0.5 · amplitude · 2^(20t-10) · sin((2t-1-s) · 2π / period)
    t >= 0.5: amplitude · 2^(-20t+10) · sin((2t-1-s) · 2π / period) · 0.5 + 1
}

where s = period / (2π) · arcsin(1 / amplitude)
```

**Back:**

```typescript
// easeInBack
f(t) = t² · ((amount+1) · t - amount)

// easeOutBack
f(t) = (t-1)² · ((amount+1) · (t-1) + amount) + 1

// easeInOutBack
f(t) = {
    t < 0.5: 0.5 · (2t)² · ((amount+1) · 2t - amount)
    t >= 0.5: 0.5 · ((2t-2)² · ((amount+1) · (2t-2) + amount) + 2)
}

where amount = 1.7 (default, multiplied by 1.525 forInOut)
```

### 6.2 Complete Easing Registry

| Easing Name | Type | Parameters / Control Points |
|-------------|------|----------------------------|
| `linear` | Linear | - |
| `swing` | Cosine | - |
| `spring` | Exponential | - |
| `ease` | Bezier | (0.25, 0.1, 0.25, 1) |
| `easeIn` | Bezier | (0.42, 0, 1, 1) |
| `easeOut` | Bezier | (0, 0, 0.58, 1) |
| `easeInOut` | Bezier | (0.42, 0, 0.58, 1) |
| `easeInSine` | Bezier | (0.47, 0, 0.745, 0.715) |
| `easeOutSine` | Bezier | (0.39, 0.575, 0.565, 1) |
| `easeInOutSine` | Bezier | (0.445, 0.05, 0.55, 0.95) |
| `easeInQuad` | Bezier | (0.55, 0.085, 0.68, 0.53) |
| `easeOutQuad` | Bezier | (0.25, 0.46, 0.45, 0.94) |
| `easeInOutQuad` | Bezier | (0.455, 0.03, 0.515, 0.955) |
| `easeInCubic` | Bezier | (0.55, 0.055, 0.675, 0.19) |
| `easeOutCubic` | Bezier | (0.215, 0.61, 0.355, 1) |
| `easeInOutCubic` | Bezier | (0.645, 0.045, 0.355, 1) |
| `easeInQuart` | Bezier | (0.895, 0.03, 0.685, 0.22) |
| `easeOutQuart` | Bezier | (0.165, 0.84, 0.44, 1) |
| `easeInOutQuart` | Bezier | (0.77, 0, 0.175, 1) |
| `easeInQuint` | Bezier | (0.755, 0.05, 0.855, 0.06) |
| `easeOutQuint` | Bezier | (0.23, 1, 0.32, 1) |
| `easeInOutQuint` | Bezier | (0.86, 0, 0.07, 1) |
| `easeInExpo` | Bezier | (0.95, 0.05, 0.795, 0.035) |
| `easeOutExpo` | Bezier | (0.19, 1, 0.22, 1) |
| `easeInOutExpo` | Bezier | (1, 0, 0, 1) |
| `easeInCirc` | Bezier | (0.6, 0.04, 0.98, 0.335) |
| `easeOutCirc` | Bezier | (0.075, 0.82, 0.165, 1) |
| `easeInOutCirc` | Bezier | (0.785, 0.135, 0.15, 0.86) |
| `easeInBack` | Polynomial | amount = 1.7 |
| `easeOutBack` | Polynomial | amount = 1.7 |
| `easeInOutBack` | Polynomial | amount = 1.7 × 1.525 |
| `easeInBounce` | Piecewise | - |
| `easeOutBounce` | Piecewise | - |
| `easeInOutBounce` | Piecewise | - |
| `easeInElastic` | Trigonometric | amplitude = 1, period = 0.3 |
| `easeOutElastic` | Trigonometric | amplitude = 1, period = 0.3 |
| `easeInOutElastic` | Trigonometric | amplitude = 1, period = 0.45 |
| `at-start` | Special | Instant switch |
| `during` | Special | Only during animation |
| `at-end` | Special | Switch at completion |

---

## 7. Performance Characteristics

### 7.1 Memory Optimization

**Float32Array for Spline Tables:**

```typescript
const float32ArraySupported = "Float32Array" in window;
const mSampleValues = float32ArraySupported
    ? new Float32Array(kSplineTableSize)
    : new Array(kSplineTableSize);
```

**Benefit:** Float32Array uses 4 bytes per value vs 8 bytes for Number, reducing memory by 50%.

### 7.2 Caching Strategies

**Property Value Cache:**

```typescript
interface ElementData {
    cache: Properties<string>;  // 80x faster than element.style access
    computedStyle?: CSSStyleDeclaration;  // 50% faster
}
```

**Step Easing Cache:**

```typescript
const cache: {[steps: number]: VelocityEasingFn} = {};

export function generateStep(steps): VelocityEasingFn {
    return cache[steps] || (cache[steps] = (percentComplete, startValue, endValue) => {
        return startValue + Math.round(percentComplete * steps) * (1/steps) * (endValue - startValue);
    });
}
```

### 7.3 Frame Skipping Benefits

By skipping frames when `deltaTime < minFrameTime`:

1. **Reduced CPU usage** - No unnecessary calculations
2. **Battery efficiency** - Critical for mobile devices
3. **Consistent timing** - Prevents animation speedup on high-refresh displays

### 7.4 WebWorker Overhead

**Costs:**
- Worker creation: ~5-10ms initial cost
- Message passing: ~0.5-1ms per message
- Memory: Additional ~100KB for worker context

**Benefits:**
- Maintains 30fps in background tabs (vs 1-4fps with RAF alone)
- Accurate `percentComplete` tracking during tab switches

### 7.5 Benchmark Estimates

| Operation | Time (ms) | Notes |
|-----------|-----------|-------|
| RAF tick (no animations) | 0.1-0.3 | Empty loop overhead |
| RAF tick (1 animation) | 0.3-0.5 | Single element, simple property |
| RAF tick (100 animations) | 5-15 | 100 elements, mixed properties |
| Pattern matching (simple) | 0.05-0.1 | "0px" to "100px" |
| Pattern matching (complex) | 0.5-1.0 | "transform: translateX(0) rotate(0)" |
| Bezier calculation | 0.01-0.05 | With cached spline table |
| Color parsing | 0.02-0.05 | Single color conversion |

---

## Appendix A: File Structure

```
velocity/src/Velocity/
├── tick.ts              # Main animation loop
├── tweens.ts            # Pattern matching and tween expansion
├── queue.ts             # Queue management
├── state.ts             # Global state
├── defaults.ts          # Default options
├── complete.ts          # Animation completion handling
├── data.ts              # Element data storage
├── options.ts           # Option validation
├── sequences.ts         # Named sequence handling
├── camelCase.ts         # CSS property name normalization
├── easing/
│   ├── bezier.ts        # Cubic bezier mathematics
│   ├── easings.ts       # Easing registry
│   ├── spring_rk4.ts    # Spring physics simulation
│   ├── step.ts          # Step easing
│   ├── string.ts        # at-start/during/at-end easings
│   ├── back.ts          # Back easings
│   ├── bounce.ts        # Bounce easings
│   └── elastic.ts       # Elastic easings
├── css/
│   ├── getPropertyValue.ts  # Property retrieval
│   ├── setPropertyValue.ts  # Property setting
│   ├── fixColors.ts         # Color normalization
│   ├── augmentDimension.ts  # Box model calculations
│   └── removeNestedCalc.ts  # calc() cleanup
└── normalizations/
    ├── normalizations.ts    # Registration and lookup
    ├── scroll.ts            # Scroll property handling
    ├── dimensions.ts        # Width/height normalizations
    ├── display.ts           # Display property handling
    ├── style.ts             # Style property handling
    └── svg/
        ├── attributes.ts    # SVG attribute handling
        └── dimensions.ts    # SVG dimension handling
```

---

## Appendix B: Key Data Structures

```typescript
// Animation Call (doubly-linked list node)
interface AnimationCall {
    _next?: AnimationCall;
    _prev?: AnimationCall;
    _flags: number;
    tweens?: {[property: string]: VelocityTween};
    element?: HTMLorSVGElement;
    elements?: VelocityResult;
    options?: StrictVelocityOptions;
    timeStart?: number;
    ellapsedTime?: number;
    percentComplete?: number;
    // ... option properties
}

// Tween Structure
interface VelocityTween {
    fn: VelocityNormalizationsFn;
    sequence?: Sequence;
    easing?: VelocityEasingFn;
    start?: string;
    end?: string;
}

// Sequence (pattern-matched values)
interface Sequence extends Array<TweenStep> {
    pattern: (string | boolean)[];
}

interface TweenStep extends Array<string | number> {
    percent?: number;
    easing?: VelocityEasingFn | null;
    [index: number]: string | number;
}

// Element Data
interface ElementData {
    types: number;
    cache: Properties<string>;
    computedStyle?: CSSStyleDeclaration;
    count: number;
    queueList: {[name: string]: AnimationCall};
    lastAnimationList: {[name: string]: AnimationCall};
    lastFinishList: {[name: string]: number};
    window: Window;
}
```

---

## Conclusion

Velocity's architecture demonstrates several sophisticated techniques:

1. **Dual-loop tick processing** for sync animation batching
2. **WebWorker fallback** for background tab accuracy
3. **Newton-Raphson with binary search fallback** for bezier inversion
4. **Token-based pattern matching** for complex CSS value interpolation
5. **Doubly-linked list queues** for efficient animation management
6. **Comprehensive caching** for performance optimization

This exploration provides the foundation needed to implement a compatible animation library or extend Velocity with custom functionality.
