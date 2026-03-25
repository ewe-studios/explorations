# Vector Graphics Algorithms in Rive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.rive/rive-runtime/src/shapes/`, `renderer/src/gr_triangulator.*`

---

## Table of Contents

1. [Overview](#overview)
2. [Path Representation](#path-representation)
3. [Bezier Curve Mathematics](#bezier-curve-mathematics)
4. [Path Tessellation](#path-tessellation)
5. [Stroke Rendering](#stroke-rendering)
6. [Fill Algorithms](#fill-algorithms)
7. [Anti-Aliasing Techniques](#anti-aliasing-techniques)
8. [Gradient Rendering](#gradient-rendering)

---

## Overview

Vector graphics rendering converts mathematical descriptions of shapes into pixels. This document explains the algorithms Rive uses for path rendering, from Bezier curves to GPU triangles.

### Key Algorithm Files

| File | Algorithm | Lines |
|------|-----------|-------|
| `src/shapes/path.cpp` | Path building, vertex iteration | ~560 |
| `src/shapes/shape.cpp` | Shape composition | ~280 |
| `renderer/src/gr_triangulator.cpp` | Path tessellation | ~2,000 |
| `renderer/src/gr_triangulator.hpp` | Tessellation algorithms | ~250 |
| `renderer/src/draw.cpp` | Draw command processing | ~2,700 |
| `tess/src/contour_stroke.cpp` | Stroke extrusion | ~300 |

### Rendering Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Vector Path Definition                       │
│  MoveTo, LineTo, CubicTo, Close commands                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Path Flattening                              │
│  Convert curves to line segments (tolerance-based)              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Tessellation                                 │
│  Convert paths to triangles using sweep-line algorithm          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    GPU Rasterization                            │
│  Fragment shaders determine pixel coverage                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Path Representation

### Path Commands

A path in Rive is a sequence of drawing commands:

```cpp
enum class PathVerb {
    move,    // Move to position (starts new contour)
    line,    // Line to position
    cubic,   // Cubic Bezier curve
    close    // Close contour (line to start)
};

class RawPath {
    std::vector<PathVerb> m_Verbs;
    std::vector<Vec2D> m_Points;

    void move(const Vec2D& p) {
        m_Verbs.push_back(PathVerb::move);
        m_Points.push_back(p);
    }

    void line(const Vec2D& p) {
        m_Verbs.push_back(PathVerb::line);
        m_Points.push_back(p);
    }

    void cubic(const Vec2D& c1, const Vec2D& c2, const Vec2D& p) {
        m_Verbs.push_back(PathVerb::cubic);
        m_Points.push_back(c1);
        m_Points.push_back(c2);
        m_Points.push_back(p);
    }

    void close() {
        m_Verbs.push_back(PathVerb::close);
    }
};
```

### Path Example

```
Path Definition:
    moveTo(100, 100)
    lineTo(200, 100)
    cubicTo(250, 150, 250, 250, 200, 300)
    lineTo(100, 300)
    close()

Visual:
    (100,100) ──────────── (200,100)
        │                       │
        │                       │
        │                       ╲
        │                        ╲
        │                         ╲
        │                          ● (200,300)
        │                         ╱
        │                        ╱
    (100,300) ────────────────╱
```

### Vertex Types

```cpp
// From src/shapes/

class PathVertex {
    Vec2D m_Translation;
    virtual void computeIn() = 0;
    virtual void computeOut() = 0;
};

class StraightVertex : public PathVertex {
    float m_Radius;  // For rounded corners
};

class CubicVertex : public PathVertex {
    Vec2D m_InPoint;   // Incoming control point
    Vec2D m_OutPoint;  // Outgoing control point
};

class CubicDetachedVertex : public CubicVertex {
    // Control points can move independently
};

class CubicMirroredVertex : public CubicVertex {
    // Control points mirror each other
};
```

---

## Bezier Curve Mathematics

### Cubic Bezier Definition

A cubic Bezier curve is defined by four points:

```
P₀ = Start point
P₁ = First control point
P₂ = Second control point
P₃ = End point

Curve formula:
B(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃

where t ∈ [0, 1]
```

### Bernstein Basis Functions

```
The four Bernstein polynomials for cubic Bezier:

B₀,₃(t) = (1-t)³     (weight for P₀)
B₁,₃(t) = 3(1-t)²t   (weight for P₁)
B₂,₃(t) = 3(1-t)t²   (weight for P₂)
B₃,₃(t) = t³         (weight for P₃)

Properties:
- Sum to 1 for any t
- All non-negative for t ∈ [0, 1]
- Provide smooth interpolation
```

### Derivative (Tangent Vector)

```
First derivative (tangent direction):
B'(t) = 3(1-t)²(P₁-P₀) + 6(1-t)t(P₂-P₁) + 3t²(P₃-P₂)

Used for:
- Computing tangent direction for stroke rendering
- Finding curve extrema
- Adaptive flattening
```

### Curve Flattening

Converting curves to line segments:

```cpp
// Adaptive flattening algorithm
void flattenCurve(const Vec2D& p0, const Vec2D& p1,
                  const Vec2D& p2, const Vec2D& p3,
                  float tolerance,
                  std::vector<Vec2D>& output) {

    // Check if curve is flat enough
    float d1 = distancePointToLine(p1, p0, p3);
    float d2 = distancePointToLine(p2, p0, p3);

    if (d1 < tolerance && d2 < tolerance) {
        // Curve is flat enough - emit endpoint
        output.push_back(p3);
    } else {
        // Subdivide curve using de Casteljau algorithm
        Vec2D p01, p12, p23, p012, p123, p0123;

        // First level of subdivision
        p01  = lerp(p0, p1, 0.5f);
        p12  = lerp(p1, p2, 0.5f);
        p23  = lerp(p2, p3, 0.5f);

        // Second level
        p012 = lerp(p01, p12, 0.5f);
        p123 = lerp(p12, p23, 0.5f);

        // Final point (on the curve)
        p0123 = lerp(p012, p123, 0.5f);

        // Recursively flatten both halves
        flattenCurve(p0, p01, p012, p0123, tolerance, output);
        flattenCurve(p0123, p123, p23, p3, tolerance, output);
    }
}

// de Casteljau visualization:
//
// P₀ ●           Original curve
//    │╲
//    │ ╲
//    │  ● P₁      After one subdivision:
//    │  │╲
//    │  │ ╲       P₀ ●────● P₀₁
//    │  │  ● P₂      │╲   │╲
//    │  │  │╲        │ ╲  │ ╲
//    │  │  │ ╲       │  ●─●─● P₀₁₂
//    │  │  │  ● P₃   │  │╲│  │╲
//    └──┴──┴──┴─     └──┴─┴──┴─● (midpoint)
```

### Radius Computation for Rounded Corners

```cpp
// From src/shapes/path.cpp
float Path::computeIdealControlPointDistance(const Vec2D& toPrev,
                                             const Vec2D& toNext,
                                             float radius) {
    // Get angle between the two edges
    float angle = fabs(std::atan2(
        Vec2D::cross(toPrev, toNext),
        Vec2D::dot(toPrev, toNext)
    ));

    // Compute ideal control point distance based on "natural rounding"
    // Formula from: https://observablehq.com/@daformat/rounding-polygon-corners
    return fmin(radius,
        (4.0f / 3.0f) *
        std::tan(math::PI / (2.0f * ((2.0f * math::PI) / angle))) *
        radius *
        (angle < math::PI / 2 ? 1 + std::cos(angle) : 2.0f - std::sin(angle))
    );
}
```

---

## Path Tessellation

### Overview

Tessellation converts a path (sequence of curves and lines) into triangles suitable for GPU rendering.

### GrTriangulator Algorithm

The tessellator uses a **sweep-line algorithm** with six stages:

```cpp
// From renderer/src/gr_triangulator.hpp

class GrTriangulator {
    // Stage 1: Convert path to piecewise linear contours
    void pathToContours(const RawPath& path,
                       float tolerance,
                       const AABB& clipBounds,
                       VertexList* contours,
                       bool* isLinear);

    // Stage 2: Build mesh of edges
    void contoursToMesh(VertexList* contours,
                       int contourCnt,
                       VertexList* mesh,
                       const Comparator&);

    // Stage 3: Sort vertices (merge sort)
    static void SortedMerge(VertexList* front,
                           VertexList* back,
                           VertexList* result,
                           const Comparator&);
    static void SortMesh(VertexList* vertices,
                        const Comparator&);

    // Stage 4: Simplify (insert vertices at intersections)
    SimplifyResult simplify(VertexList* mesh,
                           const Comparator&);

    // Stage 5: Tessellate into monotone polygons
    std::tuple<Poly*, bool> tessellate(const VertexList& vertices,
                                       const Comparator&);

    // Stage 6: Triangulate polygons
    size_t polysToTriangles(Poly* polys,
                           FillRule overrideFillRule,
                           uint16_t pathID,
                           gpu::WriteOnlyMappedMemory<gpu::TriangleVertex>*);
};
```

### Stage 1: Path to Contours

```
Input: RawPath with curves
Output: Piecewise linear contour

Before flattening:
    ●━━━━━━━━━━━●
   ╱             ╲
  ╱               ╲
 ●                 ●

After flattening (tolerance = 2px):
    ●───●───●───●
   ╱    │   │   ╲
  ╱     │   │    ╲
 ●──────●───●─────●
```

### Stage 2-3: Mesh Building and Sorting

```
Mesh construction:
- Create doubly-linked list of vertices
- Each vertex knows its neighbors
- Sort by Y coordinate (then X)

Sorted vertices:
    (0, 0) ●    ← Y=0, leftmost
           │
    (50, 25) ●  ← Y=25
           │
    (100, 50) ● ← Y=50
```

### Stage 4: Simplification

Handle self-intersecting paths:

```
Self-intersecting path:
    ●─────────●
    │╲       ╱│
    │ ╲     ╱ │
    │  ╲   ╱  │
    │   ●╱    │  ← Intersection point
    │   │     │
    └───●─────┘

After simplification:
    ●─────────●
    │╲       ╱│
    │ ╲     ╱ │
    │  ●───●  │  ← New vertex added
    │  │   │  │
    │  │   │  │
    └──●───●──┘
```

### Stage 5: Monotone Polygon Tessellation

A **monotone polygon** has the property that any horizontal line intersects it at most twice.

```
Non-monotone polygon:
       ●
      ╱│╲
     ╱ │ ╲
    ●  │  ●   ← Horizontal line intersects 4 times
   ╱   │   ╲
  ╱    │    ╲
 ●─────●─────●

Split into monotone polygons:
       ●              (Polygon A)
      ╱│╲
     ╱ │ ╲
    ●──●──●           (Split edge)
   ╱         ╲
  ╱           ╲       (Polygon B)
 ●─────────────●
```

### Stage 6: Triangle Emission

```cpp
// Emit triangles from monotone polygon
size_t emitMonotonePoly(const MonotonePoly* poly,
                       uint16_t pathID,
                       gpu::WriteOnlyMappedMemory<gpu::TriangleVertex>* out) {

    Vertex* leftChain = poly->leftChain;
    Vertex* rightChain = poly->rightChain;

    // Triangle fan from top vertex
    Vertex* apex = leftChain->top();
    Vertex* left = apex->next;
    Vertex* right = rightChain->top()->next;

    while (left != nullptr && right != nullptr) {
        if (left->y < right->y) {
            // Emit triangle on left side
            emitTriangle(apex, left, right, pathID, out);
            apex = left;
            left = left->next;
        } else {
            // Emit triangle on right side
            emitTriangle(apex, right, left, pathID, out);
            apex = right;
            right = right->next;
        }
    }
}
```

### Triangle Vertex Format

```cpp
// GPU triangle vertex
struct TriangleVertex {
    float x, y;        // Position in clip space
    uint16_t pathID;   // Path identifier (for PLS)
    int16_t weight;    // Winding weight (+1 or -1)
};

// Winding weight determines fill:
// +1 = clockwise edge (adds to coverage)
// -1 = counter-clockwise edge (subtracts from coverage)
```

---

## Stroke Rendering

### Stroke Extrusion

Converting a path into a stroked outline:

```
Original path (center line):
    ●───────────●───────────●

Stroke (width = 10px):
    ┌───────────────────────┐
    │                       │
    ●───────────●───────────●
    │                       │
    └───────────────────────┘

Result: 2 triangle strips forming the outline
```

### Contour Stroke Algorithm

```cpp
// From tess/src/contour_stroke.cpp
void ContourStroke::extrude(const SegmentedContour* contour,
                           bool isClosed,
                           StrokeJoin join,
                           StrokeCap cap,
                           float strokeWidth) {

    auto points = contour->contourPoints();

    for (size_t i = 0; i < points.size(); i++) {
        Vec2D current = points[i];
        Vec2D prev = points[(i - 1 + n) % n];
        Vec2D next = points[(i + 1) % n];

        // Compute tangent vectors
        Vec2D toPrev = (prev - current).normalized();
        Vec2D toNext = (next - current).normalized();

        // Compute perpendicular stroke direction
        Vec2D perp = Vec2D(-toPrev.y, toPrev.x) * strokeWidth;

        // Generate stroke vertices
        Vec2D left = current + perp;
        Vec2D right = current - perp;

        m_TriangleStrip.push_back(left);
        m_TriangleStrip.push_back(right);

        // Handle joins at corners
        handleJoin(join, current, toPrev, toNext, strokeWidth);
    }
}
```

### Stroke Joins

```
Miter Join:
         ●
        ╱│╲    ← Extended miter point
       ╱ │ ╲
      ●──●──●
      │  │  │
      └──┴──┘

Round Join:
         ╭─●─╮
       ╱   │   ╲
      ●────●────●
      │    │    │
      └────┴────┘

Bevel Join:
      ●────●────●
      │    │    │
      └────┴────┘
```

### Miter Limit

When the miter extends too far, convert to bevel:

```cpp
float miterLength = strokeWidth / sin(angle / 2);

if (miterLength > miterLimit * strokeWidth) {
    // Use bevel join instead
    join = StrokeJoin::bevel;
}
```

### Stroke Caps

```
Butt Cap (default):
    ┌──────────┐
    │          │
    ●──────────●

Square Cap (extends by strokeWidth/2):
        ┌──────┐
        │      │
    ────●──────●────

Round Cap:
        ╭──────╮
    ────●──────●────
        ╰──────╯
```

---

## Fill Algorithms

### Fill Rules

Two fill rules determine which pixels are inside a path:

### 1. Non-Zero Winding (Clockwise)

```
Rule: Count edge crossings
- Clockwise edge: +1
- Counter-clockwise edge: -1
- If sum ≠ 0: pixel is inside

Example:
    ┌───────────────┐
    │ → → → → → →   │  Ray crosses 1 edge → INSIDE
    │               │
    │   ┌───────┐   │
    │   │ ← ←   │   │  Ray crosses 2 edges (+1, -1) → OUTSIDE
    │   └───────┘   │
    └───────────────┘
```

### 2. Even-Odd Rule

```
Rule: Count edge crossings
- If count is odd: pixel is inside
- If count is even: pixel is outside

Example:
    ┌───────────────┐
    │               │
    │   ┌───────┐   │
    │   │       │   │  Ray crosses 2 edges → OUTSIDE
    │   └───────┘   │
    │       ●       │  Ray crosses 1 edge → INSIDE
    └───────────────┘
```

### GPU Fill Implementation

```
Path Level Rendering (PLS) handles fills:

Each pixel maintains a linked list of path coverage:

Pixel (x, y):
    ┌─────────────────┐
    │ Path ID │ Weight│
    ├─────────────────┤
    │    1    │  +1   │  ← Path 1 contributes +1
    │    3    │  +1   │  ← Path 3 contributes +1
    │    5    │  -1   │  ← Path 5 (hole) contributes -1
    └─────────────────┘

Final coverage = sum of weights = +1 → Draw pixel
```

---

## Anti-Aliasing Techniques

### The Aliasing Problem

```
Without Anti-Aliasing:

    ░░░░████████░░░░░
    ░░░██████████░░░░
    ░░████████████░░░
    ░██████████████░░
    ████████████████░

Staircase artifacts (jaggies)
```

### Anti-Aliasing Solutions

### 1. Multi-Sample Anti-Aliasing (MSAA)

```
Each pixel contains multiple samples:

Pixel with 4 samples:
    ┌───────┐
    │ ●   ● │  2 of 4 samples covered
    │       │  = 50% coverage
    │ ●   ● │
    └───────┘

Final color = weighted average
```

### 2. Coverage Mask

```
Compute pixel coverage percentage:

    ┌───────────────┐
    │░░░░▓▓▓▓▓▓▓▓▓▓▓│  60% coverage
    │░░░░░░▓▓▓▓▓▓▓▓▓│
    └───────────────┘

Final alpha = coverage × shape alpha
```

### 3. Distance-Based Anti-Aliasing

```
Signed Distance Field (SDF):

    ┌─────────────────────────────────┐
    │ -3 -2 -1  0  +1 +2 +3          │
    │   ░░░▒▒▒███████▒▒▒░░░          │
    │     Inside   Outside            │
    └─────────────────────────────────┘

Fragment shader:
    float coverage = 1.0 - smoothstep(-0.5, 0.5, distance);
```

### Rive's Approach

Rive uses GPU-based anti-aliasing through:
1. **MSAA** on supported platforms
2. **Analytic anti-aliasing** in fragment shaders
3. **Supersampling** for high-DPI displays

---

## Gradient Rendering

### Gradient Types

### Linear Gradient

```
P₀ ●───────────────────● P₁
   │                   │
   │  Color interpolation
   │  from P₀ to P₁
   ▼
```

```cpp
// From renderer/src/gradient.cpp
class LinearGradient : public Gradient {
    Vec2D m_StartPoint;
    Vec2D m_EndPoint;
    std::vector<Color> m_Colors;
    std::vector<float> m_Stops;

    Color sample(float t) const {
        // Find color stops
        for (size_t i = 0; i < m_Stops.size() - 1; i++) {
            if (t >= m_Stops[i] && t <= m_Stops[i + 1]) {
                float localT = (t - m_Stops[i]) / (m_Stops[i + 1] - m_Stops[i]);
                return lerp(m_Colors[i], m_Colors[i + 1], localT);
            }
        }
        return m_Colors.back();
    }
};
```

### Radial Gradient

```
          ╭───╮
      ╭───│ ● │───╮    ● = Center
    ╭───│   │   │───╮  r = Radius
  ╭───│   ●   │   │───╮
    ╰───│   │   │───╯
      ╰───│ ● │───╯
          ╰───╯
  Color interpolates from center to edge
```

### Gradient on GPU

```
Gradient texture approach:

1. Create 1D texture from gradient stops
2. Sample texture based on position

Linear gradient:
    t = dot(position - start, normalize(end - start))

Radial gradient:
    t = distance(position, center) / radius

Fragment shader:
    vec4 color = texture(gradientTexture, t);
```

---

## Summary

Vector graphics rendering in Rive involves:

1. **Path Building**: Constructing paths from Bezier curves
2. **Flattening**: Converting curves to line segments
3. **Tessellation**: Creating triangles using sweep-line algorithms
4. **Stroke Extrusion**: Generating stroke outlines
5. **Fill Rules**: Determining interior pixels
6. **Anti-Aliasing**: Smoothing jagged edges
7. **Gradients**: Color interpolation across shapes

For related topics:
- `rendering-engine-deep-dive.md` - GPU rendering pipeline
- `wasm-web-rendering.md` - Web-specific considerations
- `rust-revision.md` - Rust implementation approach
