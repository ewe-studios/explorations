# Spline Algorithms Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Spline3d/`

This document covers the mathematical foundations and implementations of spline algorithms used in 3D modeling.

---

## Table of Contents

1. [Introduction to Splines](#introduction-to-splines)
2. [Bezier Curves](#bezier-curves)
3. [B-Spline Curves](#b-spline-curves)
4. [NURBS (Non-Uniform Rational B-Splines)](#nurbs)
5. [Subdivision Surfaces](#subdivision-surfaces)
6. [De Casteljau's Algorithm](#de-casteljaus-algorithm)
7. [Cox-de Boor Algorithm](#cox-de-boor-algorithm)
8. [Knot Vectors](#knot-vectors)
9. [Rust Implementation Guide](#rust-implementation-guide)

---

## Introduction to Splines

### What is a Spline?

A **spline** is a mathematical function that creates a smooth curve passing through or near a set of control points. The name comes from the flexible drafting tool (a "spline") used by shipbuilders and aircraft designers to draw smooth curves.

```
Control Points:  P0      P1      P2      P3
                    \     |     /
                     \    |    /
                      \   |   /
                       \  |  /
                        \ | /
                         \|/
Smooth Curve: ~~~~~~~~~~~~
```

### Why Splines in 3D Modeling?

1. **Smoothness**: C1, C2 continuity for visually smooth surfaces
2. **Local Control**: Moving one control point affects only a portion of the curve
3. **Compact Representation**: Complex shapes from few control points
4. **Mathematical Properties**: Exact evaluation, derivatives, arc length

### Types of Splines

| Type | Continuity | Local Control | Use Case |
|------|------------|---------------|----------|
| Bezier | C1 (single segment) | No (global) | Simple curves, fonts |
| B-Spline | C2 (multi-segment) | Yes | Complex curves, CAD |
| NURBS | C2 + rational | Yes | Exact conics, CAD/CAM |
| Catmull-Rom | C1 | Yes | Animation paths |

---

## Bezier Curves

### Mathematical Definition

A Bezier curve of degree `n` is defined by `n+1` control points `P0, P1, ..., Pn`:

```
B(t) = Σ(i=0 to n) [B(n,i)(t) * Pi]  for t ∈ [0, 1]
```

Where `B(n,i)(t)` are the **Bernstein polynomials**:

```
B(n,i)(t) = C(n,i) * t^i * (1-t)^(n-i)
```

And `C(n,i)` is the binomial coefficient: `C(n,i) = n! / (i! * (n-i)!)`

### Linear Bezier (n=1)

```
B(t) = (1-t)*P0 + t*P1
```

This is simply a straight line from P0 to P1.

### Quadratic Bezier (n=2)

```
B(t) = (1-t)²*P0 + 2(1-t)*t*P1 + t²*P2
```

```
        P1
       /  \
      /    \
     /      \
    P0------P2
     \      /
      \    /  <-- Curve
       ~~~~
```

### Cubic Bezier (n=3) - Most Common

```
B(t) = (1-t)³*P0 + 3(1-t)²*t*P1 + 3(1-t)*t²*P2 + t³*P3
```

```
    P1          P2
     o----------o
    /            \
   /              \
  o                o
 P0                P3
  \                /
   \              /
    ~~~~~~~~~~~~~    <-- Cubic Bezier Curve
```

### Bernstein Basis Functions (Cubic)

```
B(3,0)(t) = (1-t)³
B(3,1)(t) = 3(1-t)²t
B(3,2)(t) = 3(1-t)t²
B(3,3)(t) = t³
```

```
1.0 |  B0    B1    B2    B3
    |  / \  / \  / \  / \
0.5 | /   \/   \/   \/   \
    |/                     \
0.0 +------------------------
    0.0   0.5   1.0    t
```

### Rust Implementation

```rust
use nalgebra::{Vector2, Vector3};

/// Cubic Bezier curve evaluation
pub fn cubic_bezier_2d(
    p0: Vector2<f64>,
    p1: Vector2<f64>,
    p2: Vector2<f64>,
    p3: Vector2<f64>,
    t: f64,
) -> Vector2<f64> {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;

    mt3 * p0 + 3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3 * p3
}

/// Cubic Bezier curve evaluation (3D)
pub fn cubic_bezier_3d(
    p0: Vector3<f64>,
    p1: Vector3<f64>,
    p2: Vector3<f64>,
    p3: Vector3<f64>,
    t: f64,
) -> Vector3<f64> {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;

    mt3 * p0 + 3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3 * p3
}

/// Evaluate Bezier curve at multiple points
pub fn bezier_curve_points(
    control_points: &[Vector3<f64>],
    samples: usize,
) -> Vec<Vector3<f64>> {
    let mut points = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f64 / (samples - 1) as f64;
        points.push(bezier_evaluate(control_points, t));
    }
    points
}

/// General Bezier evaluation using De Casteljau's algorithm
pub fn bezier_evaluate(control_points: &[Vector3<f64>], t: f64) -> Vector3<f64> {
    let mut points = control_points.to_vec();
    let n = points.len();

    for level in 1..n {
        for i in 0..(n - level) {
            points[i] = (1.0 - t) * points[i] + t * points[i + 1];
        }
    }

    points[0]
}
```

---

## B-Spline Curves

### Motivation

Bezier curves have **global control**: moving any control point affects the entire curve. B-Splines (Basis Splines) solve this with **local control** through piecewise polynomial segments.

### B-Spline Definition

A B-Spline curve of degree `p`:

```
C(t) = Σ(i=0 to n) [N(i,p)(t) * Pi]
```

Where:
- `Pi` are the control points
- `N(i,p)(t)` are the B-spline basis functions (defined recursively)
- The curve is defined over a **knot vector** `U = {u0, u1, ..., um}`

### B-Spline Properties

1. **Local Control**: Each control point affects only `p+1` curve segments
2. **Continuity**: C(p-1) continuity for uniform knots
3. **Convex Hull**: Curve lies within convex hull of control points
4. **Variation Diminishing**: Curve doesn't oscillate more than control polygon

### Uniform vs Non-Uniform Knot Vectors

**Uniform B-Spline:**
```
U = {0, 1, 2, 3, 4, 5, 6, 7}  (equal spacing)
```

**Non-Uniform B-Spline:**
```
U = {0, 0, 0, 2, 5, 8, 9, 9, 9}  (unequal spacing, clamped)
```

### Open Uniform (Clamped) B-Spline

Most common in CAD. First and last knots have multiplicity `p+1`:

```
U = {0, 0, 0, 0, 1, 2, 3, 4, 4, 4, 4}  (degree 3, clamped)
                              ^^^^^^^^
                              End points interpolated
```

This makes the curve pass through the first and last control points.

---

## NURBS (Non-Uniform Rational B-Splines)

### What Makes NURBS "Rational"?

NURBS add **weights** to control points, enabling exact representation of:
- Circles and ellipses
- Parabolas and hyperbolas
- Conic sections

### NURBS Definition

```
NURBS(t) = Σ(i=0 to n) [w(i) * N(i,p)(t) * Pi] / Σ(i=0 to n) [w(i) * N(i,p)(t)]
```

Where `w(i)` are the **weights** associated with each control point.

### Homogeneous Coordinates

NURBS can be understood as projecting a higher-dimensional B-Spline:

```
Control Point in 4D: P'i = (wi*xi, wi*yi, wi*zi, wi)

Perspective Division:
  x = X' / w
  y = Y' / w
  z = Z' / w
```

### NURBS Surface

A NURBS surface is a tensor product of two NURBS curves:

```
S(u,v) = Σ(i)Σ(j) [wi,j * N(i,pu)(u) * N(j,pv)(v) * Pi,j] / Σ(i)Σ(j) [wi,j * N(i,pu)(u) * N(j,pv)(v)]
```

### Control Point Grid for NURBS Surface

```
        v
        ^
        |
    P02 o-----o-----o-----o P32
        |     |     |     |
    P01 o-----o-----o-----o P31
        |     |     |     |
    P00 o-----o-----o-----o P30 --> u
```

### Rust NURBS Structure

```rust
use nalgebra::Vector3;

/// Knot vector type
pub type KnotVector = Vec<f64>;

/// NURBS Curve representation
pub struct NurbsCurve {
    /// Control points (3D)
    pub control_points: Vec<Vector3<f64>>,
    /// Weights for each control point
    pub weights: Vec<f64>,
    /// Degree of the curve
    pub degree: usize,
    /// Knot vector
    pub knots: KnotVector,
}

impl NurbsCurve {
    /// Create a clamped uniform knot vector
    pub fn clamped_knot_vector(degree: usize, num_control_points: usize) -> KnotVector {
        let n = num_control_points - 1;
        let p = degree;
        let m = n + p + 1; // Last knot index

        let mut knots = vec![0.0; m + 1];

        // Fill internal knots uniformly
        for i in (p + 1)..=(n) {
            knots[i] = (i - p) as f64 / (n - p) as f64;
        }

        // Set end knots to 1.0
        for i in (n + 1)..=(m) {
            knots[i] = 1.0;
        }

        knots
    }

    /// Evaluate curve at parameter t
    pub fn evaluate(&self, t: f64) -> Vector3<f64> {
        let n = self.control_points.len() - 1;
        let p = self.degree;

        let mut point = Vector3::zeros();
        let mut weight_sum = 0.0;

        for i in 0..=n {
            let basis = self.basis_function(i, p, t);
            let weight = self.weights[i];
            point += weight * basis * self.control_points[i];
            weight_sum += weight * basis;
        }

        point / weight_sum
    }

    /// Cox-de Boor recursion for basis functions
    fn basis_function(&self, i: usize, p: usize, t: f64) -> f64 {
        if p == 0 {
            // Zero degree: piecewise constant
            let ui = self.knots[i];
            let ui1 = self.knots[i + 1];
            if t >= ui && t < ui1 {
                1.0
            } else {
                0.0
            }
        } else {
            // Recursive case
            let ui = self.knots[i];
            let ui_p = self.knots[i + p];
            let ui1 = self.knots[i + 1];
            let ui_p1 = self.knots[i + p + 1];

            let c1 = if ui_p - ui > 1e-10 {
                (t - ui) / (ui_p - ui) * self.basis_function(i, p - 1, t)
            } else {
                0.0
            };

            let c2 = if ui_p1 - ui1 > 1e-10 {
                (ui_p1 - t) / (ui_p1 - ui1) * self.basis_function(i + 1, p - 1, t)
            } else {
                0.0
            };

            c1 + c2
        }
    }
}
```

---

## Subdivision Surfaces

### Overview

Subdivision surfaces create smooth surfaces from arbitrary topology meshes through iterative refinement.

### Catmull-Clark Subdivision

Most common subdivision scheme for quadrilateral meshes.

**Algorithm:**

1. **Face Points**: Create new vertex at centroid of each face
2. **Edge Points**: Create new vertex on each edge
3. **Update Vertices**: Move existing vertices to new positions
4. **Reconnect**: Form new faces from new points

### Catmull-Clark Vertex Update Rule

For a vertex V with valence n:

```
V' = (F + 2R + (n-3)V) / n

Where:
  F = average of all face points adjacent to V
  R = average of all edge midpoints adjacent to V
  n = valence (number of edges connected to V)
```

### Catmull-Clark Step Visualization

```
Before (1 quad face):          After (4 quad faces):

    o-------o                      o---o---o
    |       |                      |   |   |
    |       |          -->         o---o---o
    |       |                      |   |   |
    o-------o                      o---o---o

Each face is subdivided into 4 faces.
```

### Loop Subdivision

Designed for triangle meshes.

**Edge Split Rule:**
```
New vertex on edge (v1, v2):
  Vnew = (3/8) * (v1 + v2) + (1/8) * (v3 + v4)

Where v3, v4 are the opposite vertices of the two triangles sharing the edge.
```

### Rust Subdivision Implementation

```rust
use nalgebra::Vector3;
use std::collections::{HashMap, HashSet};

/// Half-edge data structure for subdivision
pub struct HalfEdgeMesh {
    pub vertices: Vec<Vector3<f64>>,
    pub faces: Vec<Vec<usize>>,
    pub halfedges: Vec<HalfEdge>,
}

#[derive(Clone)]
pub struct HalfEdge {
    pub target: usize,          // Target vertex index
    pub twin: usize,            // Twin half-edge index
    pub face: usize,            // Incident face index
    pub next: usize,            // Next half-edge around face
    pub prev: usize,            // Previous half-edge around face
    pub edge: usize,            // Edge index
}

impl HalfEdgeMesh {
    /// Perform one Catmull-Clark subdivision step
    pub fn catmull_clark_step(&self) -> Self {
        let mut new_vertices = Vec::new();
        let mut face_points = HashMap::new();
        let mut edge_points = HashMap::new();

        // 1. Compute face points
        for (face_idx, face) in self.faces.iter().enumerate() {
            let centroid = face.iter()
                .map(|&v_idx| self.vertices[v_idx])
                .fold(Vector3::zeros(), |acc, v| acc + v)
                / face.len() as f64;
            face_points.insert(face_idx, new_vertices.len());
            new_vertices.push(centroid);
        }

        // 2. Compute edge points
        // ... (implementation continues)

        // 3. Update original vertices
        // ... (implementation continues)

        // 4. Create new faces
        // ... (implementation continues)

        HalfEdgeMesh {
            vertices: new_vertices,
            faces: /* new faces */,
            halfedges: /* rebuild half-edges */,
        }
    }
}
```

---

## De Casteljau's Algorithm

### Purpose

De Casteljau's algorithm provides a numerically stable way to evaluate Bezier curves and split them at arbitrary parameter values.

### Algorithm Description

Given control points P0, P1, ..., Pn and parameter t:

```
For r = 1 to n:
  For i = 0 to n-r:
    P[i,r] = (1-t) * P[i,r-1] + t * P[i+1,r-1]

Result: P[0,n] is the point on the curve at t
```

### Visual Representation (Cubic Bezier)

```
Level 0 (Control Points):    P0      P1      P2      P3
                               \       |       |       /
Level 1:                        P01     P12     P23
                                 \      |      /
Level 2:                          P012    P123
                                   \     /
Level 3 (Point on Curve):           P0123 = B(t)
```

### Recursive Formula

```
P[i,0] = Pi  (initial control points)
P[i,r] = (1-t) * P[i,r-1] + t * P[i+1,r-1]
```

### Rust Implementation

```rust
use nalgebra::Vector3;

/// De Casteljau's algorithm for Bezier curve evaluation
pub fn de_casteljau(control_points: &[Vector3<f64>], t: f64) -> Vector3<f64> {
    let n = control_points.len() - 1;
    let mut points = control_points.to_vec();

    // Iteratively compute linear interpolations
    for r in 1..=n {
        for i in 0..=(n - r) {
            points[i] = (1.0 - t) * points[i] + t * points[i + 1];
        }
    }

    points[0]
}

/// De Casteljau with curve splitting
/// Returns (left_curve_points, right_curve_points)
pub fn de_casteljau_split(
    control_points: &[Vector3<f64>],
    t: f64,
) -> (Vec<Vector3<f64>>, Vec<Vector3<f64>>) {
    let n = control_points.len() - 1;

    // Triangle of intermediate points
    let mut triangle: Vec<Vec<Vector3<f64>>> = vec![vec![Vector3::zeros(); n + 1]; n + 1];

    // Initialize with control points
    for i in 0..=n {
        triangle[0][i] = control_points[i];
    }

    // Compute intermediate points
    for r in 1..=n {
        for i in 0..=(n - r) {
            triangle[r][i] = (1.0 - t) * triangle[r - 1][i] + t * triangle[r - 1][i + 1];
        }
    }

    // Extract left and right control polygons
    let left: Vec<Vector3<f64>> = (0..=n).map(|i| triangle[i][0]).collect();
    let right: Vec<Vector3<f64>> = (0..=n).map(|i| triangle[n - i][i]).collect();

    (left, right)
}

/// Subdivide Bezier curve at parameter t
pub fn subdivide_bezier(control_points: &[Vector3<f64>], t: f64) -> (Vec<Vector3<f64>>, Vec<Vector3<f64>>) {
    de_casteljau_split(control_points, t)
}
```

### Applications of De Casteljau

1. **Curve Evaluation**: Stable point computation
2. **Curve Splitting**: Divide curve into two Bezier curves
3. **Bounding Volume**: Compute convex hull for intersection testing
4. **Flattening**: Recursive subdivision for rendering

---

## Cox-de Boor Algorithm

### Purpose

The Cox-de Boor algorithm recursively computes B-spline basis functions.

### Definition

**Degree 0 (Piecewise Constant):**
```
N(i,0)(t) = { 1  if ui <= t < u(i+1)
            { 0  otherwise
```

**Degree p (Recursive):**
```
N(i,p)(t) = [(t - ui) / (u(i+p) - ui)] * N(i,p-1)(t)
          + [(u(i+p+1) - t) / (u(i+p+1) - u(i+1))] * N(i+1,p-1)(t)
```

**Special Case:** When denominator is 0, the fraction is defined as 0.

### Basis Function Properties

1. **Non-negativity**: N(i,p)(t) >= 0 for all t
2. **Local Support**: N(i,p)(t) = 0 outside [ui, u(i+p+1))
3. **Partition of Unity**: Σ N(i,p)(t) = 1 for t in [up, u(m-p))
4. **Continuity**: C(p-m) at knots with multiplicity m

### Rust Implementation with Memoization

```rust
use std::collections::HashMap;

/// Cox-de Boor basis function with memoization
pub struct BSplineBasis {
    knots: Vec<f64>,
    cache: HashMap<(usize, usize, f64), f64>,
}

impl BSplineBasis {
    pub fn new(knots: Vec<f64>) -> Self {
        BSplineBasis {
            knots,
            cache: HashMap::new(),
        }
    }

    /// Compute basis function N(i,p)(t)
    pub fn basis(&mut self, i: usize, p: usize, t: f64) -> f64 {
        // Check cache
        if let Some(&value) = self.cache.get(&(i, p, t)) {
            return value;
        }

        let result = if p == 0 {
            // Degree 0: piecewise constant
            let ui = self.knots[i];
            let ui1 = self.knots[i + 1];

            // Handle last knot specially
            if i == self.knots.len() - 2 && t == ui1 {
                1.0
            } else if t >= ui && t < ui1 {
                1.0
            } else {
                0.0
            }
        } else {
            // Recursive case
            let ui = self.knots[i];
            let ui_p = self.knots[i + p];
            let ui1 = self.knots[i + 1];
            let ui_p1 = self.knots[i + p + 1];

            // First term
            let c1 = if (ui_p - ui).abs() > 1e-10 {
                ((t - ui) / (ui_p - ui)) * self.basis(i, p - 1, t)
            } else {
                0.0
            };

            // Second term
            let c2 = if (ui_p1 - ui1).abs() > 1e-10 {
                ((ui_p1 - t) / (ui_p1 - ui1)) * self.basis(i + 1, p - 1, t)
            } else {
                0.0
            };

            c1 + c2
        };

        self.cache.insert((i, p, t), result);
        result
    }

    /// Compute all non-zero basis functions at parameter t
    pub fn all_active_bases(&mut self, t: f64, p: usize) -> Vec<(usize, f64)> {
        let mut result = Vec::new();

        // Find the knot span containing t
        let span = self.find_span(t, p);

        for i in (span - p)..=span {
            let value = self.basis(i, p, t);
            if value > 1e-10 {
                result.push((i, value));
            }
        }

        result
    }

    /// Find the knot span containing t
    fn find_span(&self, t: f64, p: usize) -> usize {
        let n = self.knots.len() - p - 2;

        if t >= self.knots[n + 1] {
            return n;
        }

        // Binary search for the span
        let mut low = p;
        let mut high = self.knots.len() - 1 - p;

        while low < high {
            let mid = (low + high + 1) / 2;
            if t < self.knots[mid] {
                high = mid - 1;
            } else {
                low = mid;
            }
        }

        low
    }
}
```

---

## Knot Vectors

### Definition

A **knot vector** is a non-decreasing sequence of parameter values:

```
U = {u0, u1, ..., um}  where  ui <= u(i+1)
```

The knot vector determines:
- Parameter domain of the curve
- Continuity at internal knots
- Whether endpoints are interpolated

### Knot Vector Types

#### 1. Uniform Knot Vector
Equal spacing between knots:
```
U = {0, 1, 2, 3, 4, 5, 6}
```

Properties:
- Periodic basis functions
- Curve doesn't interpolate endpoints
- C(p-1) continuity everywhere

#### 2. Open Uniform (Clamped) Knot Vector
First and last knots have multiplicity p+1:
```
U = {0, 0, 0, 0, 1, 2, 3, 4, 4, 4, 4}  (degree 3)
     ^^^^^^^^              ^^^^^^^^
     Interpolate P0        Interpolate Pn
```

Properties:
- Curve interpolates first and last control points
- C(p-1) continuity at internal knots
- Most common in CAD applications

#### 3. Non-Uniform Knot Vector
Arbitrary spacing:
```
U = {0, 0, 0, 2.5, 3, 7, 9, 9, 9}  (degree 2)
```

Properties:
- Variable parameterization
- Can create sharp corners with knot multiplicity

### Knot Multiplicity and Continuity

**Knot Multiplicity**: Number of times a knot value is repeated

```
U = {0, 0, 0, 1, 1, 2, 3, 3, 4, 4, 4}
                ^^            ^^
           Multiplicity 2  Multiplicity 2
```

**Continuity at Knot:**
```
Continuity = C(p - m)

Where:
  p = degree
  m = knot multiplicity
```

Examples (degree 3 curve):
- Simple knot (m=1): C2 continuity
- Double knot (m=2): C1 continuity
- Triple knot (m=3): C0 continuity (position continuous only)
- Quadruple knot (m=4): Discontinuous

### Knot Insertion

Adding knots without changing curve shape (useful for refinement):

**Boehm's Algorithm:**
```
For each new knot u in [ui, u(i+1)):
  For j from i-p+1 to i:
    alpha = (u - uj) / (u(j+p) - uj)
    P'j = (1 - alpha) * P(j-1) + alpha * Pj
```

### Rust Knot Vector Utilities

```rust
/// Knot vector utilities
pub struct KnotVector {
    values: Vec<f64>,
}

impl KnotVector {
    /// Create a clamped uniform knot vector
    pub fn clamped_uniform(degree: usize, num_control_points: usize) -> Self {
        let n = num_control_points - 1;
        let p = degree;
        let m = n + p + 1;

        let mut knots = vec![0.0; m + 1];

        // Set end knots with multiplicity p+1
        for i in 0..=p {
            knots[i] = 0.0;
        }
        for i in (m - p)..=m {
            knots[i] = 1.0;
        }

        // Fill internal knots uniformly
        for i in (p + 1)..=(n) {
            knots[i] = (i - p) as f64 / (n - p) as f64;
        }

        KnotVector { values: knots }
    }

    /// Create a uniform (unclamped) knot vector
    pub fn uniform(degree: usize, num_control_points: usize) -> Self {
        let n = num_control_points - 1;
        let p = degree;
        let m = n + p + 1;

        let mut knots = vec![0.0; m + 1];

        for i in 0..=m {
            knots[i] = i as f64;
        }

        KnotVector { values: knots }
    }

    /// Get the multiplicity of a knot value
    pub fn multiplicity(&self, knot: f64) -> usize {
        const EPS: f64 = 1e-10;
        self.values.iter()
            .filter(|&&k| (k - knot).abs() < EPS)
            .count()
    }

    /// Find the knot span index for parameter t
    pub fn find_span(&self, t: f64, p: usize) -> usize {
        let n = self.values.len() - p - 2;

        if t >= self.values[n + 1] {
            return n;
        }

        // Binary search
        let mut low = p;
        let mut high = self.values.len() - 1 - p;

        while low < high {
            let mid = (low + high + 1) / 2;
            if t < self.values[mid] {
                high = mid - 1;
            } else {
                low = mid;
            }
        }

        low
    }

    /// Insert a knot into the knot vector
    pub fn insert_knot(&mut self, new_knot: f64) {
        // Find insertion position
        let pos = self.values.iter()
            .position(|&k| k > new_knot)
            .unwrap_or(self.values.len());

        self.values.insert(pos, new_knot);
    }

    /// Get the parameter domain [up, u(m-p)]
    pub fn domain(&self, p: usize) -> (f64, f64) {
        let m = self.values.len() - 1;
        (self.values[p], self.values[m - p])
    }
}
```

---

## Rust Implementation Guide

### Recommended Crates

| Crate | Purpose |
|-------|---------|
| `nalgebra` | Linear algebra, vectors, matrices |
| `bezier-rs` | Bezier curve operations |
| `nurbs` | NURBS curves and surfaces |
| `simba` | SIMD math (nalgebra backend) |
| `num-traits` | Numeric trait abstractions |

### Cargo.toml Setup

```toml
[package]
name = "spline-core"
version = "0.1.0"
edition = "2021"

[dependencies]
nalgebra = "0.32"
bezier-rs = "0.4"
num-traits = "0.2"

[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "spline_bench"
harness = false
```

### Module Structure

```
spline-core/
├── src/
│   ├── lib.rs
│   ├── bezier/
│   │   ├── mod.rs
│   │   ├── curve.rs
│   │   └── surface.rs
│   ├── bspline/
│   │   ├── mod.rs
│   │   ├── basis.rs
│   │   ├── curve.rs
│   │   └── knot_vector.rs
│   ├── nurbs/
│   │   ├── mod.rs
│   │   ├── curve.rs
│   │   └── surface.rs
│   ├── subdivision/
│   │   ├── mod.rs
│   │   ├── catmull_clark.rs
│   │   └── loop.rs
│   └── utils/
│       ├── mod.rs
│       └── evaluator.rs
```

### Complete Example: NURBS Curve

```rust
// src/nurbs/curve.rs
use nalgebra::{Vector3, MatrixXx};

/// NURBS Curve implementation
pub struct NurbsCurve {
    control_points: Vec<Vector3<f64>>,
    weights: Vec<f64>,
    knots: Vec<f64>,
    degree: usize,
}

impl NurbsCurve {
    /// Create a new NURBS curve
    pub fn new(
        control_points: Vec<Vector3<f64>>,
        weights: Vec<f64>,
        knots: Vec<f64>,
        degree: usize,
    ) -> Self {
        assert_eq!(control_points.len(), weights.len());
        assert_eq!(knots.len(), control_points.len() + degree + 1);

        NurbsCurve {
            control_points,
            weights,
            knots,
            degree,
        }
    }

    /// Create a NURBS curve that represents a circle
    pub fn circle(radius: f64, segments: usize) -> Self {
        // Circle requires degree 2 NURBS with specific weights
        let control_points = vec![
            Vector3::new(radius, 0.0, 0.0),
            Vector3::new(radius, radius, 0.0),
            Vector3::new(0.0, radius, 0.0),
            Vector3::new(-radius, radius, 0.0),
            Vector3::new(-radius, 0.0, 0.0),
            Vector3::new(-radius, -radius, 0.0),
            Vector3::new(0.0, -radius, 0.0),
            Vector3::new(radius, -radius, 0.0),
            Vector3::new(radius, 0.0, 0.0),  // Closing point
        ];

        let weights = vec![
            1.0,
            1.0 / 2.0_f64.sqrt(),
            1.0,
            1.0 / 2.0_f64.sqrt(),
            1.0,
            1.0 / 2.0_f64.sqrt(),
            1.0,
            1.0 / 2.0_f64.sqrt(),
            1.0,
        ];

        let knots = vec![
            0.0, 0.0, 0.0,
            0.25, 0.25,
            0.5, 0.5,
            0.75, 0.75,
            1.0, 1.0, 1.0,
        ];

        NurbsCurve::new(control_points, weights, knots, 2)
    }

    /// Evaluate the curve at parameter t
    pub fn evaluate(&self, t: f64) -> Vector3<f64> {
        let n = self.control_points.len() - 1;
        let p = self.degree;

        let mut point = Vector3::zeros();
        let mut weight_sum = 0.0;

        for i in 0..=n {
            let basis = self.basis_function(i, p, t);
            let weight = self.weights[i];
            point += weight * basis * self.control_points[i];
            weight_sum += weight * basis;
        }

        point / weight_sum
    }

    /// Compute basis function using Cox-de Boor
    fn basis_function(&self, i: usize, p: usize, t: f64) -> f64 {
        if p == 0 {
            let ui = self.knots[i];
            let ui1 = self.knots[i + 1];
            let is_last = i == self.knots.len() - 2;

            if t >= ui && (t < ui1 || (is_last && t == ui1)) {
                1.0
            } else {
                0.0
            }
        } else {
            let ui = self.knots[i];
            let ui_p = self.knots[i + p];
            let ui1 = self.knots[i + 1];
            let ui_p1 = self.knots[i + p + 1];

            let c1 = if (ui_p - ui).abs() > 1e-10 {
                ((t - ui) / (ui_p - ui)) * self.basis_function(i, p - 1, t)
            } else {
                0.0
            };

            let c2 = if (ui_p1 - ui1).abs() > 1e-10 {
                ((ui_p1 - t) / (ui_p1 - ui1)) * self.basis_function(i + 1, p - 1, t)
            } else {
                0.0
            };

            c1 + c2
        }
    }

    /// Tessellate curve into line segments for rendering
    pub fn tessellate(&self, segments: usize) -> Vec<Vector3<f64>> {
        let (start, end) = self.domain();
        (0..=segments)
            .map(|i| {
                let t = start + (end - start) * (i as f64 / segments as f64);
                self.evaluate(t)
            })
            .collect()
    }

    /// Get the parameter domain
    pub fn domain(&self) -> (f64, f64) {
        let p = self.degree;
        let m = self.knots.len() - 1;
        (self.knots[p], self.knots[m - p])
    }
}
```

---

## References

1. **The NURBS Book** by Les Piegl and Wayne Tiller - Comprehensive NURBS reference
2. **Fundamentals of Computer Graphics** by Peter Shirley - Graphics foundations
3. **Computer Aided Geometric Design** by Thomas W. Sederberg - CAGD theory
4. **VTK User's Guide** - VTK implementation details
5. **bezier-rs** crate documentation - Rust bezier implementation
