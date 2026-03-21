---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src/easing/
explored_at: 2026-03-20
---

# mo.js Easing Composition - Deep Dive

**Scope:** Easing mixing, Custom easing creation, Easing presets library, Easing parser and registry

---

## Table of Contents

1. [Easing Architecture Overview](#1-easing-architecture-overview)
2. [Easing Parser and Registry](#2-easing-parser-and-registry)
3. [Bezier Easing Implementation](#3-bezier-easing-implementation)
4. [Path Easing System](#4-path-easing-system)
5. [Approximated Easing](#5-approximated-easing)
6. [Easing Mixing (mix.coffee)](#6-easing-mixing-mixcoffee)
7. [Easing Presets Library](#7-easing-presets-library)
8. [Custom Easing Creation](#8-custom-easing-creation)

---

## 1. Easing Architecture Overview

### 1.1 Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    mo.js EASING SYSTEM                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                 easing.coffee                             │   │
│  │  - Easing parser and registry                             │   │
│  │  - Preset easing functions                                │   │
│  │  - mix() integration                                      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│         ┌────────────────────┼────────────────────┐             │
│         │                    │                    │             │
│         ▼                    ▼                    ▼             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │  bezier-    │     │  path-      │     │ approximate │       │
│  │  easing     │     │  easing     │     │             │       │
│  │  (cubic)    │     │  (SVG)      │     │ (sampling)  │       │
│  └─────────────┘     └─────────────┘     └─────────────┘       │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    mix.coffee                             │   │
│  │  - Multi-easing composition                               │   │
│  │  - Progress-based easing switching                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Easing Types

| Type | File | Purpose | Input |
|------|------|---------|-------|
| **Bezier** | `bezier-easing.coffee` | Cubic bezier curves | `[x1, y1, x2, y2]` |
| **Path** | `path-easing.coffee` | SVG path as easing | `"M0,0 C..."` |
| **Approximate** | `approximate.babel.js` | Function sampling | `function(progress)` |
| **Mix** | `mix.coffee` | Multi-easing composition | `[{to, value}, ...]` |

---

## 2. Easing Parser and Registry

### 2.1 Parse Easing Method

```coffeescript
parseEasing: (easing) ->
  if !easing? then easing = 'linear.none'

  type = typeof easing

  if type is 'string'
    return if easing.charAt(0).toLowerCase() is 'm'
      @path(easing)  # SVG path easing
    else
      easing = @_splitEasing(easing)
      easingParent = @[easing[0]]
      if !easingParent
        h.error "Easing with name \"#{easing[0]}\" was not found,
                  fallback to \"linear.none\" instead"
        return @['linear']['none']
      easingParent[easing[1]]

  if h.isArray(easing)
    return @bezier.apply(@, easing)

  if type is 'function'
    return easing
```

**Supported Input Formats:**

```javascript
// String name (family.variant)
mojs.easing.parseEasing('cubic.out')
mojs.easing.parseEasing('sin.inout')
mojs.easing.parseEasing('elastic.out')

// SVG path string (starts with 'm')
mojs.easing.parseEasing('M0,100 C20,0 80,0 100,100')

// Bezier array (4 control points)
mojs.easing.parseEasing([0.42, 0, 0.58, 1])

// Custom function
mojs.easing.parseEasing((progress) => progress * progress)

// Undefined/null defaults to linear.none
mojs.easing.parseEasing(null)  // (k) => k
```

### 2.2 Split Easing

```coffeescript
_splitEasing: (string) ->
  return string if typeof string is 'function'

  if typeof string is 'string' and string.length
    split = string.split '.'
    firstPart   = split[0].toLowerCase() or 'linear'
    secondPart  = split[1].toLowerCase() or 'none'
    [firstPart, secondPart]
  else
    ['linear', 'none']
```

**Parsing Examples:**
```javascript
'_splitEasing("cubic.out")'    // ['cubic', 'out']
'_splitEasing("sin")'          // ['sin', 'none']
'_splitEasing("ELASTIC.INOUT")'// ['elastic', 'inout']
'_splitEasing("")'             // ['linear', 'none']
```

### 2.3 Easing Registry Structure

```coffeescript
class Easing
  # Registered easing families
  linear:   none: (k) -> k
  ease:
    in:     bezier.apply @, [0.42, 0, 1, 1]
    out:    bezier.apply @, [0, 0, 0.58, 1]
    inout:  bezier.apply @, [0.42, 0, 0.58, 1]
  sin:
    in:     (k) -> 1 - Math.cos(k * PI / 2)
    out:    (k) -> Math.sin(k * PI / 2)
    inout:  (k) -> 0.5 * (1 - Math.cos(PI * k))
  # ... more families
```

**Access Pattern:**
```javascript
mojs.easing.sin.out       // Direct property access
mojs.easing['cubic']['in'] // Bracket notation
```

---

## 3. Bezier Easing Implementation

### 3.1 Cubic Bezier Mathematics

```coffeescript
# Coefficient calculations
A = (aA1, aA2) -> 1.0 - 3.0 * aA2 + 3.0 * aA1
B = (aA1, aA2) -> 3.0 * aA2 - 6.0 * aA1
C = (aA1) -> 3.0 * aA1

# Returns x(t) given t, x1, and x2
calcBezier = (aT, aA1, aA2) ->
  ((A(aA1, aA2) * aT + B(aA1, aA2)) * aT + C(aA1)) * aT

# Returns dx/dt given t, x1, and x2
getSlope = (aT, aA1, aA2) ->
  3.0 * A(aA1, aA2) * aT * aT + 2.0 * B(aA1, aA2) * aT + C(aA1)
```

**Mathematical Foundation:**
```
B(t) = (1-t)³·P₀ + 3(1-t)²t·P₁ + 3(1-t)t²·P₂ + t³·P₃

For x-coordinate with P₀=(0,0) and P₃=(1,1):
x(t) = (1 - 3x₂ + 3x₁)·t³ + (3x₂ - 6x₁)·t² + (3x₁)·t
x(t) = A·t³ + B·t² + C·t

Where:
  A = 1 - 3x₂ + 3x₁
  B = 3x₂ - 6x₁
  C = 3x₁
```

### 3.2 Sample Table Optimization

```coffeescript
kSplineTableSize = 11
kSampleStepSize = 1.0 / (kSplineTableSize - 1.0)  # 0.1

calcSampleValues = ->
  i = 0
  while i < kSplineTableSize
    mSampleValues[i] = calcBezier(i * kSampleStepSize, mX1, mX2)
    ++i
```

**Pre-computed Values:**
```
Sample index:  0     1      2      3      4      5      6      7      8      9     10
t value:     0.0   0.1    0.2    0.3    0.4    0.5    0.6    0.7    0.8    0.9   1.0
x(t):        [pre-computed for binary search]
```

### 3.3 Float32Array Optimization

```coffeescript
float32ArraySupported = !!Float32Array
mSampleValues = if !float32ArraySupported
  then new Array(kSplineTableSize)
  else new Float32Array(kSplineTableSize)
```

**Benefits:**
- 50% memory reduction (4 bytes vs 8 bytes per sample)
- Better CPU cache utilization
- SIMD optimization potential

### 3.4 Newton-Raphson Iteration

```coffeescript
NEWTON_ITERATIONS = 4
NEWTON_MIN_SLOPE = 0.001

newtonRaphsonIterate = (aX, aGuessT) ->
  i = 0
  while i < NEWTON_ITERATIONS
    currentSlope = getSlope(aGuessT, mX1, mX2)
    return aGuessT if currentSlope is 0.0
    currentX = calcBezier(aGuessT, mX1, mX2) - aX
    aGuessT -= currentX / currentSlope
    ++i
  aGuessT
```

**Formula:**
```
t_{n+1} = t_n - (x(t_n) - x_target) / x'(t_n)

4 iterations typically converges to sufficient precision
```

### 3.5 Binary Search with Samples

```coffeescript
getTForX = (aX) ->
  intervalStart = 0.0
  currentSample = 1
  lastSample = kSplineTableSize - 1

  # Binary search to find interval
  while currentSample != lastSample and mSampleValues[currentSample] <= aX
    intervalStart += kSampleStepSize
    ++currentSample
  --currentSample

  # Linear interpolation for initial guess
  delta = mSampleValues[currentSample + 1] - mSampleValues[currentSample]
  dist = (aX - mSampleValues[currentSample]) / delta
  guessForT = intervalStart + dist * kSampleStepSize

  initialSlope = getSlope(guessForT, mX1, mX2)

  if initialSlope >= NEWTON_MIN_SLOPE
    newtonRaphsonIterate aX, guessForT
  else
    if initialSlope == 0.0
      guessForT
    else
      binarySubdivide aX, intervalStart, intervalStart + kSampleStepSize
```

### 3.6 Binary Subdivision Fallback

```coffeescript
binarySubdivide = (aX, aIntervalStart, aIntervalEnd) ->
  currentX = 0.0
  currentT = 0.0
  i = 0

  while Math.abs(currentX - aX) > PRECISION
    currentT = aIntervalStart + 0.5 * (aIntervalEnd - aIntervalStart)
    currentX = calcBezier(currentT, mX1, mX2)
    if currentX > aX
      aIntervalEnd = currentT
    else
      aIntervalStart = currentT
    ++i

  currentT
```

### 3.7 Bezier Easing Function

```coffeescript
bezier = (mX1, mY1, mX2, mY2) ->
  # Validate control points
  if !(0 <= mX1 <= 1 and 0 <= mX2 <= 1)
    throw Error('bezier x values must be in [0, 1]')

  # Special cases for linear
  if mX1 == mY1 and mX2 == mY2
    return (k) -> k  # Linear

  # Pre-compute sample values
  calcSampleValues()

  # Return easing function
  (x) ->
    if x == 0 or x == 1
      return x

    # Get t for given x
    t = getTForX(x)

    # Calculate y using t
    calcBezier(t, mY1, mY2)
```

**Usage:**
```javascript
const easeInOut = mojs.easing.bezier(0.42, 0, 0.58, 1)
easeInOut(0.5)  // 0.5 (approximately, depends on curve)
```

---

## 4. Path Easing System

### 4.1 PathEasing Class

```coffeescript
class PathEasing
  constructor: (@o = {}) ->
    if @o.path?
      @_path = h.parsePath(@o.path)
      @_rect = @o.rect or 100
      @pathLength = @_path.getTotalLength()
      @_precompute = h.clamp (@o.precompute or 1450), 100, 10000
      @_step = 1 / @_precompute
      @_eps = 0.00001
      @_approximateMax = 5
      @_preSample()
      @create()

  create: ->
    (p) => @sample(p)
```

### 4.2 Pre-Sampling

```coffeescript
_preSample: ->
  @_samples = []

  for i in [0..@_precompute]
    progress = i * @_step
    length = @pathLength * progress
    point = @_path.getPointAtLength(length)
    @_samples[i] = {
      point:    point
      length:   length
      progress: progress
    }
```

**Sample Structure:**
```javascript
{
  point:    SVGPoint {x: 50, y: 25},
  length:   123.456,  // Distance along path
  progress: 0.5       // 0-1 progress
}
```

### 4.3 Bounds Finding with Direction Caching

```coffeescript
_findBounds: (array, p) ->
  # Return cached bounds if progress unchanged
  return @_prevBounds if p is @_boundsPrevProgress

  @_boundsStartIndex ?= 0
  len = array.length

  # Determine search direction based on progress change
  if @_boundsPrevProgress > p
    loopEnd = 0
    direction = 'reverse'
  else
    loopEnd = len
    direction = 'forward'

  # Set start/end based on direction
  if direction is 'forward'
    start = array[0]
    end = array[array.length - 1]
  else
    start = array[array.length - 1]
    end = array[0]

  # Search for bounds
  for i in [@_boundsStartIndex...loopEnd]
    value = array[i]
    pointX = value.point.x / @_rect

    if direction is 'reverse'
      buffer = pointX
      pointX = pointP
      pointP = buffer

    if pointX < pointP
      start = value
      @_boundsStartIndex = i
    else
      end = value
      break

  @_boundsPrevProgress = p
  @_prevBounds = {start, end}
```

**Optimization:** `@_boundsStartIndex` caches last found index for O(1) sequential lookups

### 4.4 Approximation

```coffeescript
_approximate: (start, end, p) ->
  deltaP = end.point.x - start.point.x
  percentP = (p - (start.point.x / @_rect)) / (deltaP / @_rect)
  start.length + percentP * (end.length - start.length)
```

**Purpose:** Linear interpolation between two sampled points

### 4.5 Recursive Refinement

```coffeescript
_findApproximate: (p, start, end, approximateMax = @_approximateMax) ->
  approximation = @_approximate(start, end, p)
  point = @_path.getPointAtLength(approximation)
  x = point.x / @_rect

  # Check if close enough
  if h.closeEnough(p, x, @_eps)
    return @_resolveY(point)

  # Max iterations reached
  return @_resolveY(point) if (--approximateMax < 1)

  # Recursive refinement
  newPoint = {point: point, length: approximation}

  if p < x
    @_findApproximate(p, start, newPoint, approximateMax)
  else
    @_findApproximate(p, newPoint, end, approximateMax)
```

### 4.6 Y Resolution

```coffeescript
_resolveY: (point) ->
  1 - (point.y / @_rect)
```

**Coordinate System:**
- SVG path: Y increases downward
- Easing: 0 at bottom, 1 at top
- Hence: `1 - (y / rect)`

### 4.7 Sample Method

```coffeescript
sample: (p) ->
  bounds = @_findBounds(@_samples, p)
  @_findApproximate(p, bounds.start, bounds.end)
```

### 4.8 Path Easing Usage

```javascript
// SVG path as easing curve
const pathEasing = mojs.easing.path('M0,100 C20,0 80,0 100,100')

// Use in animation
new mojs.Shape({
  y: {0: 100},
  easing: pathEasing
})

// Custom precompute
const customPath = mojs.easing.path({
  path: 'M0,100 Q50,0 100,100',
  precompute: 2000,  // More samples = higher precision
  rect: 100          // Coordinate system size
})
```

---

## 5. Approximated Easing

### 5.1 Function Sampling

```javascript
const _sample = (fn, n = 4) => {
  var samples = {};

  if (typeof n === 'number') {
    var p = 0;
    var samplesCount = Math.pow(10, n);  // 10,000 for n=4
    var step = 1 / samplesCount;

    samples[0] = fn(0);

    for (var i = 0; i < samplesCount - 1; i++) {
      p += step;
      var index = parseFloat(p.toFixed(n));
      samples[index] = fn(p);
    }

    samples[1] = fn(1);
    samples.base = n;
  }

  return _proximate(samples);
};
```

**Precision Trade-offs:**
```
n=3: 1,000 samples  (~0.1% precision, ~8KB)
n=4: 10,000 samples (~0.01% precision, ~80KB)
n=5: 100,000 samples (~0.001% precision, ~800KB)
```

### 5.2 Lookup with Interpolation

```javascript
const _proximate = (samples) => {
  var n = samples.base;
  var samplesAmount = Math.pow(10, n);
  var samplesStep = 1 / samplesAmount;

  var cached = function(p) {
    var newKey = RoundNumber(p, n);
    var sample = samples[newKey];

    // Exact match
    if (Math.abs(p - newKey) < samplesStep) {
      return sample;
    }

    // Linear interpolation
    if (p > newKey) {
      nextIndex = newKey + samplesStep;
      nextValue = samples[nextIndex];
    } else {
      nextIndex = newKey - samplesStep;
      nextValue = samples[nextIndex];
    }

    var dLength = nextIndex - newKey;
    var dValue = nextValue - sample;
    var progressScale = (p - newKey) / dLength;
    var coef = (nextValue > sample) ? -1 : 1;
    var scaledDifference = coef * progressScale * dValue;

    return sample + scaledDifference;
  };

  return cached;
};
```

### 5.3 Approximate Easing Usage

```javascript
// Sample any function
const customEasing = mojs.easing.approximate(
  (progress) => Math.sin(progress * Math.PI * 2) * 0.5 + 0.5,
  4  // 4 decimal precision
);

// Use in animation
new mojs.Shape({
  y: {0: 100},
  easing: customEasing
});
```

---

## 6. Easing Mixing (mix.coffee)

### 6.1 Mix Function Purpose

The `mix` function allows composing multiple easings that activate at different progress points, enabling complex easing behaviors.

### 6.2 Parse If Easing

```coffeescript
parseIfEasing = (item) ->
  if typeof item.value is 'number'
    item.value
  else
    easing.parseEasing item.value
```

**Purpose:** Parse easing string/function or return static number

### 6.3 Sort Function

```coffeescript
sort = (a, b) ->
  a.value = parseIfEasing(a)
  b.value = parseIfEasing(b)

  returnValue = 0
  a.to < b.to and (returnValue = -1)
  a.to > b.to and (returnValue = 1)
  returnValue
```

**Purpose:** Sort mix items by `to` property (progress threshold)

### 6.4 Get Nearest

```coffeescript
getNearest = (array, progress) ->
  for value, i in array
    if value.to > progress
      return i
```

**Purpose:** Find first easing whose `to` threshold exceeds current progress

### 6.5 Mix Implementation

```coffeescript
mix = (args...) ->
  # Sort if multiple easings
  if args.length > 1
    args = args.sort(sort)
  else
    # Single value - just parse easing
    args[0].value = parseIfEasing(args[0])

  (progress) ->
    index = getNearest(args, progress)

    # Return 1 if not defined
    return 1 if typeof index is 'undefined'

    if index isnt -1
      value = args[index].value

      # Return 1 at end
      return 1 if index is args.length - 1 and progress > args[index].to

      # Evaluate function or return value
      return if typeof value is 'function' then value(progress) else value
```

### 6.6 Mix Usage Examples

```javascript
// Single easing at threshold
const singleMix = mojs.easing.mix(
  {to: 0.5, value: 'sin.out'}
);
// Returns sin.out(progress) when progress > 0.5

// Multiple easings
const multiMix = mojs.easing.mix(
  {to: 0.3, value: 'quad.in'},   // 0.0 - 0.3: quad.in
  {to: 0.7, value: 'linear.none'}, // 0.3 - 0.7: linear
  {to: 1.0, value: 'cubic.out'}    // 0.7 - 1.0: cubic.out
);

// Static values mixed with easings
const staticMix = mojs.easing.mix(
  {to: 0.5, value: 0},      // Hold at 0 until 0.5
  {to: 1.0, value: 'quad.out'} // Then ease out
);

// Usage in animation
new mojs.Shape({
  y: {0: 100},
  easing: multiMix
});
```

### 6.7 Mix Creation

```coffeescript
create = (e) ->
  easing = e  // Assign easing module reference
  mix         // Return mix function
```

**Usage in easing.coffee:**
```coffeescript
easing.mix = mix easing  # Pass easing module to create
```

---

## 7. Easing Presets Library

### 7.1 Linear

```coffeescript
linear:
  none: (k) -> k
```

**Characteristics:** Constant speed, no acceleration

### 7.2 Ease (CSS-like)

```coffeescript
ease:
  in:    bezier(0.42, 0, 1, 1)    # CSS ease-in
  out:   bezier(0, 0, 0.58, 1)    # CSS ease-out
  inout: bezier(0.42, 0, 0.58, 1) # CSS ease-in-out
```

### 7.3 Sinusoidal

```coffeescript
sin:
  in:    (k) -> 1 - Math.cos(k * PI / 2)
  out:   (k) -> Math.sin(k * PI / 2)
  inout: (k) -> 0.5 * (1 - Math.cos(PI * k))
```

**Characteristics:** Smooth, natural motion, gentle acceleration

### 7.4 Quadratic

```coffeescript
quad:
  in:    (k) -> k * k
  out:   (k) -> k * (2 - k)
  inout: (k) -> if (k *= 2) < 1 then 0.5 * k * k else -0.5 * ((k -= 2) * k * k - 2)
```

**Characteristics:** Parabolic curve, moderate acceleration

### 7.5 Cubic

```coffeescript
cubic:
  in:    (k) -> k * k * k
  out:   (k) -> --k * k * k + 1
  inout: (k) -> if (k *= 2) < 1 then 0.5 * k * k * k else 0.5 * ((k -= 2) * k * k + 2)
```

**Characteristics:** Stronger acceleration than quad, CSS default

### 7.6 Quartic

```coffeescript
quart:
  in:    (k) -> k * k * k * k
  out:   (k) -> 1 - (--k * k * k * k)
  inout: (k) -> if (k *= 2) < 1 then 0.5 * k^4 else -0.5 * ((k -= 2) * k^3 - 2)
```

### 7.7 Quintic

```coffeescript
quint:
  in:    (k) -> k * k * k * k * k
  out:   (k) -> --k * k * k * k * k + 1
  inout: (k) -> if (k *= 2) < 1 then 0.5 * k^5 else 0.5 * ((k -= 2) * k^4 + 2)
```

**Characteristics:** Very strong acceleration, dramatic motion

### 7.8 Exponential

```coffeescript
expo:
  in:    (k) -> if k is 0 then 0 else Math.pow(1024, k - 1)
  out:   (k) -> if k is 1 then 1 else 1 - Math.pow(2, -10 * k)
  inout: (k) ->
    return 0 if k is 0
    return 1 if k is 1
    if (k *= 2) < 1 then 0.5 * Math.pow(1024, k - 1)
    else 0.5 * (-Math.pow(2, -10 * (k - 1)) + 2)
```

**Characteristics:** Extreme acceleration, very dramatic

### 7.9 Circular

```coffeescript
circ:
  in:    (k) -> 1 - Math.sqrt(1 - k * k)
  out:   (k) -> Math.sqrt(1 - (--k * k))
  inout: (k) -> if (k *= 2) < 1 then -0.5 * (Math.sqrt(1 - k²) - 1) else 0.5 * (Math.sqrt(1 - (k-2)²) + 1)
```

### 7.10 Back (Overshoot)

```coffeescript
back:
  in: (k) ->
    s = 1.70158
    k * k * ((s + 1) * k - s)
  out: (k) ->
    s = 1.70158
    --k * k * ((s + 1) * k + s) + 1
  inout: (k) ->
    s = 1.70158 * 1.525
    if (k *= 2) < 1 then 0.5 * (k² * ((s + 1) * k - s))
    else 0.5 * ((k -= 2) * k * ((s + 1) * k + s) + 2)
```

**Characteristics:** Anticipation/overshoot effect

### 7.11 Elastic (Spring)

```coffeescript
elastic:
  in: (k) ->
    p = 0.4
    return 0 if k is 0
    return 1 if k is 1
    a = 1
    s = p / 4
    -(a * Math.pow(2, 10 * (k -= 1)) * Math.sin((k - s) * (2 * Math.PI) / p))
  out: (k) ->
    p = 0.4
    return 0 if k is 0
    return 1 if k is 1
    a = 1
    s = p / 4
    a * Math.pow(2, -10 * k) * Math.sin((k - s) * (2 * Math.PI) / p) + 1
  inout: (k) ->
    p = 0.4
    return 0 if k is 0
    return 1 if k is 1
    a = 1
    s = p / 4
    if (k *= 2) < 1
      return -0.5 * (a * Math.pow(2, 10 * (k -= 1)) * Math.sin((k - s) * (2 * Math.PI) / p))
    a * Math.pow(2, -10 * (k -= 1)) * Math.sin((k - s) * (2 * Math.PI) / p) * 0.5 + 1
```

**Characteristics:** Spring/bounce effect with oscillation

### 7.12 Bounce

```coffeescript
bounce:
  in: (k) -> 1 - @bounce.out(1 - k)
  out: (k) ->
    if k < (1 / 2.75)
      7.5625 * k * k
    else if k < (2 / 2.75)
      7.5625 * (k -= (1.5 / 2.75)) * k + 0.75
    else if k < (2.5 / 2.75)
      7.5625 * (k -= (2.25 / 2.75)) * k + 0.9375
    else
      7.5625 * (k -= (2.625 / 2.75)) * k + 0.984375
  inout: (k) ->
    if k < 0.5 then @bounce.in(k * 2) * 0.5
    else @bounce.out(k * 2 - 1) * 0.5 + 0.5
```

**Characteristics:** Bouncing ball effect

---

## 8. Custom Easing Creation

### 8.1 Custom Function

```javascript
// Simple custom easing
const myEasing = (progress) => {
  return progress * progress * (3 - 2 * progress);  // Smoothstep
};

new mojs.Shape({
  y: {0: 100},
  easing: myEasing
});
```

### 8.2 Bezier Custom

```javascript
// Create custom bezier
const customBezier = mojs.easing.bezier(0.68, -0.55, 0.265, 1.55);  // Back-out effect

// Use
new mojs.Shape({
  scale: {1: 0},
  easing: customBezier
});
```

### 8.3 Path Custom

```javascript
// Draw custom easing curve
const customPath = mojs.easing.path({
  path: 'M0,100 C30,0 70,0 100,100',  // S-curve
  precompute: 2000  // High precision
});

new mojs.Shape({
  y: {0: 100},
  easing: customPath
});
```

### 8.4 Approximate Custom

```javascript
// Complex mathematical function
const waveEasing = mojs.easing.approximate((p) => {
  return (Math.sin(p * Math.PI * 4) + 1) / 2;
}, 5);  // 5 decimal precision

new mojs.Shape({
  x: {0: 100},
  easing: waveEasing
});
```

### 8.5 Mix Custom

```javascript
// Multi-stage easing
const stagedEasing = mojs.easing.mix(
  {to: 0.2, value: 0},           // Hold at start
  {to: 0.5, value: 'quad.in'},   // Accelerate
  {to: 0.8, value: 'linear'},    // Constant speed
  {to: 1.0, value: 'quad.out'}   // Decelerate
);

new mojs.Shape({
  y: {0: 100},
  easing: stagedEasing
});
```

---

## Summary

The mo.js easing system provides:

1. **Multiple Input Formats:** String names, bezier arrays, SVG paths, functions
2. **High Performance:** Sample tables, Float32Array, Newton-Raphson optimization
3. **Precision Control:** Configurable sample counts, recursive refinement
4. **Composition:** `mix()` for multi-stage easing
5. **Rich Presets:** 12 easing families with in/out/inout variants
6. **Extensibility:** Easy custom easing creation via any method

The easing architecture is designed for both performance (pre-computation, caching) and flexibility (any function can become an easing), making it suitable for complex motion graphics scenarios.
