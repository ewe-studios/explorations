---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.animations/mojs/src
explored_at: 2026-03-20
---

# mo.js Splines, Curves and Path Animation - Deep Dive

**Scope:** MotionPath system, SVG path parsing, Bezier easing, Cubic bezier curves, Custom curve animations, Path-based easing, Spline drawing

---

## Table of Contents

1. [MotionPath Architecture](#1-motionpath-architecture)
2. [SVG Path Parsing](#2-svg-path-parsing)
3. [Arc to Path Conversion](#3-arc-to-path-conversion)
4. [Quadratic Bezier Curves](#4-quadratic-bezier-curves)
5. [Path Sampling and Points](#5-path-sampling-and-points)
6. [Path-Based Easing](#6-path-based-easing)
7. [Bezier Easing Implementation](#7-bezier-easing-implementation)
8. [Custom Curve Animations](#8-custom-curve-animations)
9. [Motion Blur on Paths](#9-motion-blur-on-paths)
10. [Rotation Along Paths](#10-rotation-along-paths)
11. [Scaling and Containers](#11-scaling-and-containers)
12. [Then Chaining on Paths](#12-then-chaining-on-paths)

---

## 1. MotionPath Architecture

### 1.1 MotionPath Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                      MOTIONPATH SYSTEM                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐  │
│  │   Path Source   │───▶│  Path Parser    │───▶│  SVG Path El    │  │
│  │                 │    │                 │    │                 │  │
│  │ - CSS selector  │    │ - parsePath()   │    │ getTotalLength()│  │
│  │ - SVG path str  │    │ - curveToPath() │    │ getPointAtLen() │  │
│  │ - Arc shift     │    │                 │    │                 │  │
│  └─────────────────┘    └─────────────────┘    └────────┬────────┘  │
│                                                         │           │
│                                                         ▼           │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐  │
│  │   Transform     │◀───│   Progress      │◀───│   Tween         │  │
│  │                 │    │                 │    │                 │  │
│  │ translate(x,y)  │    │ len = start +   │    │ duration: 1000  │  │
│  │ rotate(angle)   │    │       p*sliced  │    │ easing: linear  │  │
│  │ filter (blur)   │    │ point = path    │    │                 │  │
│  └─────────────────┘    │   .getPointAt   │    └─────────────────┘  │
│                         └─────────────────┘                          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 MotionPath Defaults

```coffeescript
# motion-path.coffee
defaults:
    # Path definition
    path: null              # CSS selector, SVG path, or arc {x,y}

    # Curve settings (for arc paths)
    curvature:
        x: '75%'            # Horizontal control point
        y: '50%'            # Vertical control point

    # Timing
    duration: 1000          # Animation duration (ms)
    delay: 0                # Initial delay (ms)
    repeat: 0               # Number of repeats
    yoyo: false             # Yoyo playback
    easing: null            # Easing function

    # Position
    offsetX: 0              # Horizontal offset from path
    offsetY: 0              # Vertical offset from path

    # Path bounds
    pathStart: 0            # Start position (0-1)
    pathEnd: 1              # End position (0-1)

    # Rotation
    isRotation: false       # Rotate element to follow path
    rotationOffset: null    # Additional rotation offset

    # Motion blur
    motionBlur: 0           # Blur amount (0-1)

    # Behavior
    isRunLess: false        # Don't auto-start
    isPresetPosition: true  # Set initial position
    isReverse: false        # Reverse direction
    isCompositeLayer: true  # GPU acceleration
```

### 1.3 MotionPath Initialization

```coffeescript
constructor: (@o = {}) ->
    @vars()
    @createTween()
    @

vars: ->
    @getScaler = h.bind @getScaler, @
    @resize = resize
    @props = h.cloneObj @defaults
    @extendOptions @o

    # Reset motionBlur for Safari/IE
    @isMotionBlurReset = h.isSafari or h.isIE
    @isMotionBlurReset and (@props.motionBlur = 0)

    @history = [h.cloneObj @props]
    @postVars()

postVars: ->
    # Clamp path bounds
    @props.pathStart = h.clamp @props.pathStart, 0, 1
    @props.pathEnd = h.clamp @props.pathEnd, @props.pathStart, 1

    # Initialize motion blur vars
    @rotate = 0
    @speedX = 0
    @speedY = 0
    @blurX = 0
    @blurY = 0
    @prevCoords = {}
    @blurAmount = 20

    # Clamp motionBlur
    @props.motionBlur = h.clamp @props.motionBlur, 0, 1

    @onUpdate = @props.onUpdate

    # Validate element
    if !@o.el
        h.error 'Missed "el" option.'
        return true

    @el = @parseEl @props.el
    @props.motionBlur > 0 and @createFilter()

    # Get and validate path
    @path = @getPath()
    if !@path.getAttribute('d')
        h.error 'Path has no coordinates'
        return true

    @len = @path.getTotalLength()
    @slicedLen = @len * (@props.pathEnd - @props.pathStart)
    @startLen = @props.pathStart * @len
```

---

## 2. SVG Path Parsing

### 2.1 Path Input Types

```coffeescript
getPath: ->
    path = h.parsePath @props.path
    return path if path  # Already a valid path

    # Convert arc shift to path
    if @props.path.x or @props.path.y
        @curveToPath
            start: x: 0, y: 0
            shift:
                x: (@props.path.x or 0)
                y: (@props.path.y or 0)
            curvature:
                x: @props.curvature.x or @defaults.curvature.x
                y: @props.curvature.y or @defaults.curvature.y
```

### 2.2 Path Parsing Helper

```javascript
// h.parsePath() handles multiple input types
parsePath(path) {
    // Already SVGPathElement
    if (path instanceof SVGPathElement) {
        return path;
    }

    // CSS Selector
    if (typeof path === 'string') {
        const el = document.querySelector(path);
        if (el && el instanceof SVGPathElement) {
            return el;
        }

        // Try to parse as SVG path commands
        if (path.match(/^[MmZzLlHhVvCcSsQqTtAa]/)) {
            return createPathFromString(path);
        }
    }

    return null;
}
```

### 2.3 Path String Creation

```javascript
function createPathFromString(pathString) {
    const path = document.createElementNS(NS, 'path');
    path.setAttribute('d', pathString);
    return path;
}
```

### 2.4 Supported Path Commands

```
Command | Name              | Params | Example
--------|-------------------|--------|------------------
M       | Move to           | x y    | M 100,200
L       | Line to           | x y    | L 300,400
H       | Horizontal line   | x      | H 500
V       | Vertical line     | y      | V 300
C       | Cubic Bezier      | cp1x cp1y cp2x cp2y x y | C 100,100 200,200 300,300
S       | Smooth cubic      | cp2x cp2y x y | S 200,200 300,300
Q       | Quadratic Bezier  | cpx cpy x y | Q 150,150 300,300
T       | Smooth quadratic  | x y    | T 400,400
A       | Arc               | rx ry x-axis-rot large-arc sweep x y
Z       | Close path        | (none) | Z
```

---

## 3. Arc to Path Conversion

### 3.1 Arc Definition

```coffeescript
# Convert arc (shift + curvature) to SVG path
curveToPath: (o) ->
    path = document.createElementNS h.NS, 'path'

    start = o.start          # Starting point
    endPoint =               # End point calculated from shift
        x: start.x + o.shift.x
        y: start.y + o.shift.y
    curvature = o.curvature  # Control point offsets

    # Calculate distance and angle
    dX = o.shift.x
    dY = o.shift.y
    radius = Math.sqrt(dX*dX + dY*dY)  # Length of shift
    percent = radius / 100              # For percentage calculations
    rotation = Math.atan(dY/dX) * (180/Math.PI) + 90

    # Adjust rotation for negative X
    if o.shift.x < 0
        rotation = rotation + 180
```

### 3.2 Control Point Calculation

```coffeescript
# Get first control point (X curvature)
curvatureX = h.parseUnit curvature.x
curvatureX = if curvatureX.unit is '%'
    then curvatureX.value * percent
    else curvatureX.value

curveXPoint = h.getRadialPoint
    center: x: start.x, y: start.y
    radius: curvatureX
    rotate: rotation

# Get second control point (Y curvature)
curvatureY = h.parseUnit curvature.y
curvatureY = if curvatureY.unit is '%'
    then curvatureY.value * percent
    else curvatureY.value

curvePoint = h.getRadialPoint
    center: x: curveXPoint.x, y: curveXPoint.y
    radius: curvatureY
    rotate: rotation + 90  # Perpendicular to first
```

### 3.3 Radial Point Calculation

```javascript
// h.getRadialPoint - Get point at angle and distance
getRadialPoint(o) {
    const center = o.center || { x: 0, y: 0 };
    const radius = o.radius || 0;
    const rotate = o.rotate || 0;

    // Convert degrees to radians, adjust for SVG coordinate system
    const rad = (rotate - 90) * Math.PI / 180;

    return {
        x: center.x + Math.cos(rad) * radius,
        y: center.y + Math.sin(rad) * radius
    };
}
```

### 3.4 Path String Generation

```coffeescript
# Create quadratic Bezier path
path.setAttribute 'd', """
    M#{start.x},#{start.y}
    Q#{curvePoint.x},#{curvePoint.y}
    #{endPoint.x},#{endPoint.y}
"""

# Example output:
# M 0,0 Q 150,100 300,0
# (Start at origin, curve through control point, end at 300,0)
```

### 3.5 Arc Visualization

```
Arc from (0,0) to (300, 0):

Direct Line:
(0,0) ──────────────────────▶ (300,0)

With Curvature (x:75%, y:50%):
            control point
                 ●
                /   \
               /     \
              /       \
(0,0) ●──────           ──────▶ ● (300,0)
         quadratic curve

Path commands:
M 0,0              - Move to start
Q 225,150          - Quadratic control point (75% of 300 = 225, 50% up = 150)
  300,0            - End point
```

---

## 4. Quadratic Bezier Curves

### 4.1 Quadratic Curve Formula

```
B(t) = (1-t)²·P₀ + 2(1-t)·t·P₁ + t²·P₂

Where:
- P₀ = Start point
- P₁ = Control point
- P₂ = End point
- t  = Parameter (0 to 1)

Expanded:
x(t) = (1-t)²·x₀ + 2(1-t)·t·x₁ + t²·x₂
y(t) = (1-t)²·y₀ + 2(1-t)·t·y₁ + t²·y₂
```

### 4.2 Point on Curve

```javascript
function getPointOnQuadraticBezier(t, p0, p1, p2) {
    const mt = 1 - t;
    const mt2 = mt * mt;
    const t2 = t * t;

    return {
        x: mt2 * p0.x + 2 * mt * t * p1.x + t2 * p2.x,
        y: mt2 * p0.y + 2 * mt * t * p1.y + t2 * p2.y
    };
}
```

### 4.3 SVG getPointAtLength

```javascript
// SVG provides native method for any path type
const path = document.querySelector('path');
const length = path.getTotalLength();

// Get point at specific distance along path
const point = path.getPointAtLength(distance);

// Returns: { x: Number, y: Number }
```

### 4.4 MotionPath Progress to Point

```coffeescript
setProgress: (p, isInit) ->
    # Calculate current length along path
    len = @startLen + if !@props.isReverse
        then p * @slicedLen      # Forward
        else (1 - p) * @slicedLen # Reverse

    # Get point at that length
    point = @path.getPointAtLength len

    # Apply offsets
    x = point.x + @props.offsetX
    y = point.y + @props.offsetY

    # Calculate rotation if needed
    @_getCurrentRotation point, len, p

    # Apply transform
    @_setTransformOrigin p
    @_setTransform x, y, p, isInit

    # Apply motion blur if enabled
    @props.motionBlur and @makeMotionBlur x, y
```

---

## 5. Path Sampling and Points

### 5.1 Path Length Sampling

```javascript
// Sample points along path for custom processing
function samplePath(path, numPoints) {
    const length = path.getTotalLength();
    const points = [];
    const step = length / (numPoints - 1);

    for (let i = 0; i < numPoints; i++) {
        const dist = i * step;
        points.push(path.getPointAtLength(dist));
    }

    return points;
}
```

### 5.2 Tangent Calculation

```coffeescript
_getCurrentRotation: (point, len, p) ->
    isTransformFunOrigin = typeof @props.transformOrigin is 'function'

    if @props.isRotation or @props.rotationOffset? or isTransformFunOrigin
        # Get previous point (1 unit back)
        prevPoint = @path.getPointAtLength len - 1

        # Calculate angle between points
        x1 = point.y - prevPoint.y
        x2 = point.x - prevPoint.x
        atan = Math.atan(x1 / x2)

        # Handle vertical tangent
        !isFinite(atan) and (atan = 0)

        # Convert to degrees
        @rotate = atan * h.RAD_TO_DEG

        # Apply rotation offset
        if (typeof @props.rotationOffset) isnt 'function'
            @rotate += @props.rotationOffset or 0
        else
            @rotate = @props.rotationOffset.call @, @rotate, p
    else
        @rotate = 0
```

### 5.3 Normal Vector

```javascript
// Get normal (perpendicular) to path at point
function getPathNormal(path, length) {
    const point = path.getPointAtLength(length);
    const prevPoint = path.getPointAtLength(length - 1);

    // Tangent vector
    const tx = point.x - prevPoint.x;
    const ty = point.y - prevPoint.y;

    // Normalize
    const len = Math.sqrt(tx * tx + ty * ty);
    const nx = -ty / len;  // Perpendicular (normal)
    const ny = tx / len;

    return { x: nx, y: ny };
}
```

### 5.4 Offset from Path

```javascript
// Offset point from path along normal
function offsetFromPath(path, length, offset) {
    const point = path.getPointAtLength(length);
    const normal = getPathNormal(path, length);

    return {
        x: point.x + normal.x * offset,
        y: point.y + normal.y * offset
    };
}
```

---

## 6. Path-Based Easing

### 6.1 Path Easing Concept

```coffeescript
# easing/path-easing.coffee
# Use SVG path to define custom easing curve

class PathEasing
    constructor: (@path) ->
        @len = @path.getTotalLength()

    # Get Y value (output) for X value (input)
    get: (x) ->
        # Clamp input
        x = h.clamp x, 0, 1

        # Convert X to pixel position
        targetX = x * @pathBBox.width

        # Find point on path at this X
        point = @findPointAtX targetX

        # Convert Y to 0-1 range (inverted for SVG coords)
        1 - (point.y / @pathBBox.height)
```

### 6.2 Path Easing Usage

```javascript
// Define easing with SVG path
const path = document.querySelector('#easing-curve');
const easing = new mojs.PathEasing(path);

// Use in animation
new mojs.Tween({
    duration: 1000,
    easing: easing,
    onUpdate: (progress) => {
        // progress follows the path's Y values
    }
});
```

### 6.3 Easing Curve Visualization

```
Progress
  1 │                 ┌─── End (1,1)
    │              ╱
    │           ╱
    │        ╱
0.5│     ╱          Linear: diagonal
    │  ╱
    │╱
  0 └───────────────────
    0   0.5    1    Time

  1 │      ╭─── End (1,1)
    │    ╱
    │  ╱
    │╱
0.5│                  Ease-out
    │
    │
    │
  0 └───────────────────
    0   0.5    1    Time
```

---

## 7. Bezier Easing Implementation

### 7.1 Cubic Bezier Easing

```coffeescript
# easing/bezier-easing.coffee
# Cubic Bezier curve for easing
# B(t) = (1-t)³·P₀ + 3(1-t)²·t·P₁ + 3(1-t)·t²·P₂ + t³·P₃

class BezierEasing
    generate: (mX1, mY1, mX2, mY2) ->
        # Validate inputs
        if arguments.length < 4
            return @error 'Bezier function expects 4 arguments'

        # X values must be in [0, 1] range
        if (mX1 < 0 or mX1 > 1 or mX2 < 0 or mX2 > 1)
            return @error 'Bezier x values should be > 0 and < 1'

        # Constants for Newton-Raphson iteration
        NEWTON_ITERATIONS = 4
        NEWTON_MIN_SLOPE = 0.001
        SUBDIVISION_PRECISION = 0.0000001
        SUBDIVISION_MAX_ITERATIONS = 10

        # Sample table for binary search
        kSplineTableSize = 11
        kSampleStepSize = 1.0 / (kSplineTableSize - 1.0)
```

### 7.2 Bezier Coefficient Calculation

```coffeescript
# Coefficient functions
# Simplified cubic Bezier: x(t) = ((A·t + B)·t + C)·t

A = (aA1, aA2) -> 1.0 - 3.0 * aA2 + 3.0 * aA1
B = (aA1, aA2) -> 3.0 * aA2 - 6.0 * aA1
C = (aA1) -> 3.0 * aA1

# Calculate x(t) or y(t)
calcBezier = (aT, aA1, aA2) ->
    ((A(aA1, aA2) * aT + B(aA1, aA2)) * aT + C(aA1)) * aT

# Calculate slope (derivative)
getSlope = (aT, aA1, aA2) ->
    3.0 * A(aA1, aA2) * aT * aT + 2.0 * B(aA1, aA2) * aT + C(aA1)
```

### 7.3 Newton-Raphson Method

```coffeescript
# Newton-Raphson iteration to find t for given x
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

### 7.4 Binary Subdivision

```coffeescript
# Binary subdivision fallback
binarySubdivide = (aX, aA, aB) ->
    currentX = undefined
    currentT = undefined
    i = 0

    loop
        currentT = aA + (aB - aA) / 2.0
        currentX = calcBezier(currentT, mX1, mX2) - aX

        if currentX > 0.0
            aB = currentT
        else
            aA = currentT

        isBig = Math.abs(currentX) > SUBDIVISION_PRECISION
        unless isBig and ++i < SUBDIVISION_MAX_ITERATIONS
            break

    currentT
```

### 7.5 Sample Values Table

```coffeescript
# Precompute sample values for binary search optimization
calcSampleValues = ->
    i = 0
    while i < kSplineTableSize
        mSampleValues[i] = calcBezier(i * kSampleStepSize, mX1, mX2)
        ++i

getTForX = (aX) ->
    # Find interval using sample values
    intervalStart = 0.0
    currentSample = 1
    lastSample = kSplineTableSize - 1

    while currentSample != lastSample and mSampleValues[currentSample] <= aX
        intervalStart += kSampleStepSize
        ++currentSample

    --currentSample

    # Interpolate initial guess
    delta = (mSampleValues[currentSample + 1] - mSampleValues[currentSample])
    dist = (aX - mSampleValues[currentSample]) / delta
    guessForT = intervalStart + dist * kSampleStepSize

    # Refine with Newton-Raphson or binary subdivision
    initialSlope = getSlope(guessForT, mX1, mX2)

    if initialSlope >= NEWTON_MIN_SLOPE
        newtonRaphsonIterate aX, guessForT
    else if initialSlope == 0.0
        guessForT
    else
        binarySubdivide aX, intervalStart, intervalStart + kSampleStepSize
```

### 7.6 Final Easing Function

```coffeescript
# The easing function returned to caller
f = (aX) ->
    if !_precomputed then precompute()

    # Linear shortcut
    if mX1 == mY1 and mX2 == mY2
        return aX

    # Guarantee extremes
    return 0 if aX == 0
    return 1 if aX == 1

    # Get t for input x, then calculate y
    calcBezier getTForX(aX), mY1, mY2

str = "bezier(" + [mX1, mY1, mX2, mY2] + ")"
f.toStr = -> str
f  # Return function
```

### 7.7 CSS Equivalent

```javascript
// mo.js bezier(0.25, 0.1, 0.25, 1)
// CSS equivalent:
// transition-timing-function: cubic-bezier(0.25, 0.1, 0.25, 1);

// Common easing presets:
mojs.easing.ease = 'bezier(0.25, 0.1, 0.25, 1)'
mojs.easing.easeIn = 'bezier(0.42, 0, 1, 1)'
mojs.easing.easeOut = 'bezier(0, 0, 0.58, 1)'
mojs.easing.easeInOut = 'bezier(0.42, 0, 0.58, 1)'
```

---

## 8. Custom Curve Animations

### 8.1 Elasticity Curves

```javascript
// Delta system supports custom curves for elasticity
new mojs.Shape({
    radius: {
        0: 100,
        curve: mojs.easing.bezier(0.175, 0.885, 0.32, 1.275)  // Overshoot
    }
});

// Curve receives raw progress (0-1) and returns modified value
// Values > 1 create overshoot (elasticity)
```

### 8.2 Curve Function Signature

```javascript
// Custom curve function
function customCurve(progress) {
    // progress: 0 to 1
    // return: modified progress (can exceed 1 for overshoot)

    // Example: bounce effect
    if (progress < 0.5) {
        return progress * 2;  // Speed up first half
    } else {
        return 0.5 + (progress - 0.5) * 0.5;  // Slow down second half
    }
}

// Use in delta
new mojs.Shape({
    radius: {
        0: 100,
        curve: customCurve
    }
});
```

### 8.3 Approximated Easing

```javascript
// easing/approximate.babel.js
// Create easing by sampling a function

function approximate(fn, samples = 10) {
    const samplesArr = [];

    for (let i = 0; i <= samples; i++) {
        samplesArr.push(fn(i / samples));
    }

    return function(progress) {
        const index = Math.floor(progress * samples);
        const t = progress * samples - index;

        const v0 = samplesArr[index];
        const v1 = samplesArr[index + 1] || v0;

        return v0 + (v1 - v0) * t;
    };
}

// Usage
const customEasing = approximate((x) => Math.sin(x * Math.PI / 2));
```

### 8.4 Easing Mixing

```coffeescript
# easing/mix.coffee
# Mix two easing functions

mix: (easing1, easing2, amount = 0.5) ->
    (t) ->
        v1 = easing1 t
        v2 = easing2 t
        v1 + (v2 - v1) * amount

# Usage
mixedEasing = mojs.easing.mix(
    mojs.easing.sin.out,
    mojs.easing.circ.out,
    0.5  # 50/50 mix
)
```

---

## 9. Motion Blur on Paths

### 9.1 Motion Blur Calculation

```coffeescript
makeMotionBlur: (x, y) ->
    # Calculate speed from position delta
    if !@prevCoords.x? or !@prevCoords.y?
        @speedX = 0
        @speedY = 0
    else
        dX = x - @prevCoords.x
        dY = y - @prevCoords.y
        @speedX = Math.abs dX
        @speedY = Math.abs dY

    # Calculate blur amount based on speed
    # 1px per 1ms is considered very fast
    @blurX = h.clamp (@speedX / 16) * @props.motionBlur, 0, 1
    @blurY = h.clamp (@speedY / 16) * @props.motionBlur, 0, 1

    # Calculate rotation for blur direction
    absoluteRotation = tailRotation - @rotate
    coords = @rotToCoords absoluteRotation

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

### 9.2 SVG Filter for Blur

```coffeescript
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

### 9.3 Blur Application

```coffeescript
setBlur: (o) ->
    if !@isMotionBlurReset  # Safari/IE don't support it
        @filter.setAttribute 'stdDeviation', "#{o.blur.x},#{o.blur.y}"
        @filterOffset.setAttribute 'dx', o.offset.x
        @filterOffset.setAttribute 'dy', o.offset.y
```

---

## 10. Rotation Along Paths

### 10.1 Auto Rotation

```coffeescript
# Enable rotation to follow path
new mojs.MotionPath({
    el: '.element',
    path: 'M 0,0 Q 150,150 300,0',
    isRotation: true  # Auto-rotate
})
```

### 10.2 Rotation Offset

```coffeescript
# Add fixed rotation offset
new mojs.MotionPath({
    isRotation: true,
    rotationOffset: 90  # Add 90 degrees
})

# Or function-based offset
rotationOffset: (currentRotation, progress) ->
    if currentRotation < 0
        90
    else
        -90
```

### 10.3 Transform Origin on Path

```coffeescript
# Dynamic transform origin
transformOrigin: (currentRotation, progress) ->
    "#{6 * currentRotation}% 0"

# Usage
new mojs.MotionPath({
    isRotation: true,
    transformOrigin: (rotation, p) ->
        "#{rotation}% 50%"
})
```

---

## 11. Scaling and Containers

### 11.1 Container Scaling

```coffeescript
# MotionPath scales to fit container
getScaler: ->
    @cSize =
        width:  @container.offsetWidth  or 0
        height: @container.offsetHeight or 0

    start = @path.getPointAtLength 0
    end   = @path.getPointAtLength @len

    size = {}
    @scaler = {}

    size.width = if end.x >= start.x
        then end.x - start.x
        else start.x - end.x

    size.height = if end.y >= start.y
        then end.y - start.y
        else start.y - end.y

    # Scale based on fill rule
    switch @fillRule
        when 'all'
            @calcWidth size
            @calcHeight size
        when 'width'
            @calcWidth size
            @scaler.y = @scaler.x  # Uniform scaling
        when 'height'
            @calcHeight size
            @scaler.x = @scaler.y

calcWidth: (size) ->
    @scaler.x = @cSize.width / size.width
    !isFinite(@scaler.x) and (@scaler.x = 1)

calcHeight: (size) ->
    @scaler.y = @cSize.height / size.height
    !isFinite(@scaler.y) and (@scaler.y = 1)
```

### 11.2 Fill Rules

```javascript
// How path scales to container
fillRule: 'all'      // Scale to fit both dimensions
fillRule: 'width'    // Scale based on width, uniform
fillRule: 'height'   // Scale based on height, uniform
```

---

## 12. Then Chaining on Paths

### 12.1 Sequential Path Animations

```coffeescript
# Then chaining for MotionPath
then: (o) ->
    prevOptions = @history[@history.length - 1]
    opts = {}

    # Copy previous options
    for key, value of prevOptions
        # Don't copy callbacks and tween options (except duration)
        if !h.callbacksMap[key] and !h.tweenOptionMap[key] or key is 'duration'
            o[key] ?= value
        else
            o[key] ?= undefined  # Override callbacks

    # Build tween options
    if h.tweenOptionMap[key]
        opts[key] = if key isnt 'duration'
            then o[key]
            else if o[key]? then o[key] else prevOptions[key]

    @history.push o

    # Create new tween in timeline
    it = @
    opts.onUpdate = (p) => @setProgress p
    opts.onStart = => @props.onStart?.apply @
    opts.onComplete = => @props.onComplete?.apply @
    opts.onFirstUpdate = -> it.tuneOptions it.history[@index]
    opts.isChained = !o.delay

    @timeline.append new Tween opts
    @
```

### 12.2 Path Chain Example

```javascript
const mp = new mojs.MotionPath({
    el: '.ball',
    path: { x: 300, y: 0 },  // Arc to right
    duration: 500
});

// Then chain for complex paths
mp.then({
    path: { x: 0, y: 300 },  // Arc down
    duration: 500
})
.then({
    path: { x: -300, y: 0 },  // Arc left
    duration: 500
})
.then({
    path: { x: 0, y: -300 },  // Arc up (back to start)
    duration: 500
});

mp.play();  // Plays all segments sequentially
```

---

## Appendix: Complete Path Animation Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                    PATH ANIMATION FLOW                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. PATH INITIALIZATION                                             │
│     ┌──────────────┐                                                │
│     │ getPath()    │ ──▶ Parse CSS/SVG/arc input                   │
│     └──────────────┘                                                │
│              │                                                       │
│              ▼                                                       │
│     ┌──────────────┐                                                │
│     │ getTotalLen()│ ──▶ Calculate path length                     │
│     └──────────────┘                                                │
│                                                                      │
│  2. PROGRESS CALCULATION                                            │
│     ┌──────────────┐                                                │
│     │ setProgress()│ ──▶ p * slicedLen + startLen                  │
│     └──────────────┘                                                │
│              │                                                       │
│              ▼                                                       │
│     ┌──────────────┐                                                │
│     │getPointAtLen │ ──▶ SVG native method                         │
│     └──────────────┘                                                │
│                                                                      │
│  3. TRANSFORM APPLICATION                                           │
│     ┌──────────────┐                                                │
│     │getCurrentRot │ ──▶ Calculate tangent angle                   │
│     └──────────────┘                                                │
│              │                                                       │
│              ▼                                                       │
│     ┌──────────────┐                                                │
│     │setTransform  │ ──▶ translate(x,y) rotate(angle)              │
│     └──────────────┘                                                │
│                                                                      │
│  4. OPTIONAL EFFECTS                                                │
│     ┌──────────────┐                                                │
│     │motionBlur    │ ──▶ SVG filter with stdDeviation              │
│     └──────────────┘                                                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```
