---
title: "Autograd and Backpropagation: A Complete Deep Dive"
subtitle: "Understanding how neural networks learn through automatic differentiation"
based_on: "microgpt/microgpt.py - Value class implementation"
prerequisites: "Basic calculus (derivatives, chain rule)"
---

# Autograd and Backpropagation Deep Dive

## Table of Contents

1. [Why Automatic Differentiation?](#1-why-automatic-differentiation)
2. [The Computation Graph](#2-the-computation-graph)
3. [Forward Mode vs Reverse Mode AD](#3-forward-mode-vs-reverse-mode-ad)
4. [Implementing the Value Class](#4-implementing-the-value-class)
5. [The Backward Pass Algorithm](#5-the-backward-pass-algorithm)
6. [Worked Examples](#6-worked-examples)
7. [Common Pitfalls and Optimizations](#7-common-pitfalls-and-optimizations)

---

## 1. Why Automatic Differentiation?

### 1.1 The Gradient Computation Problem

**Neural network training requires gradients:**
```
For each parameter p: ∂Loss/∂p
```

**Manual differentiation is impractical:**
- A small network might have thousands of parameters
- Each gradient requires applying the chain rule through many layers
- Changing the architecture means recomputing all gradients
- Human errors in derivative calculations are common

**Numerical approximation is too slow:**
```python
# Numerical gradient (finite differences)
def numerical_gradient(f, x, h=1e-5):
    return (f(x + h) - f(x - h)) / (2 * h)

# For 1 million parameters, this requires 2 million forward passes!
```

**Symbolic differentiation explodes in complexity:**
- Produces human-readable formulas
- But formulas grow exponentially with network depth
- Doesn't handle control flow (if statements, loops)

### 1.2 Automatic Differentiation: The Solution

**Key insight:** Any computation is a composition of elementary operations.

```python
# This computation:
y = (x1 * x2 + x3) * x4

# Is composed of:
t1 = x1 * x2      # multiplication
t2 = t1 + x3      # addition
y = t2 * x4       # multiplication
```

**AD tracks these operations and applies the chain rule automatically.**

### 1.3 The Two Modes of AD

| Mode | When to use | Computes |
|------|-------------|----------|
| **Forward** | Many inputs, few outputs | ∂y/∂x₁, ∂y/∂x₂, ... one at a time |
| **Reverse** | Few inputs, many outputs (neural nets!) | All ∂y/∂xᵢ in one pass |

**Neural networks:** Loss is a scalar, parameters are millions → Reverse mode is efficient.

---

## 2. The Computation Graph

### 2.1 What is a Computation Graph?

**Definition:** A directed acyclic graph (DAG) where:
- **Nodes** are values (intermediate results and parameters)
- **Edges** are operations (addition, multiplication, etc.)

**Example computation:** `e = (a * b) + (c * d)`

```
        a       b       c       d
         \     /         \     /
          * (t1)          * (t2)
             \           /
              + (e = output)
```

### 2.2 Building the Graph in Code

```python
class Value:
    def __init__(self, data, children=(), local_grads=()):
        self.data = data              # The numeric value
        self.grad = 0                 # Accumulated gradient (dL/dself)
        self._children = children     # Parent nodes (what went into this)
        self._local_grads = local_grads  # Local derivatives

    def __repr__(self):
        return f"Value(data={self.data}, grad={self.grad})"
```

**When we compute:**
```python
a = Value(2.0)
b = Value(3.0)
c = a * b  # Creates new Value with children=(a, b)
```

**The graph structure:**
```
  a(2.0)   b(3.0)
     \     /
      c(6.0)
```

### 2.3 Local Gradients: The Building Blocks

**Each operation knows its own derivative:**

```python
def __add__(self, other):
    # Forward: y = x1 + x2
    # Backward: dy/dx1 = 1, dy/dx2 = 1
    other = other if isinstance(other, Value) else Value(other)
    return Value(self.data + other.data, (self, other), (1, 1))

def __mul__(self, other):
    # Forward: y = x1 * x2
    # Backward: dy/dx1 = x2, dy/dx2 = x1
    other = other if isinstance(other, Value) else Value(other)
    return Value(self.data * other.data, (self, other), (other.data, self.data))

def __pow__(self, other):
    # Forward: y = x^n
    # Backward: dy/dx = n * x^(n-1)
    return Value(self.data ** other, (self,), (other * self.data ** (other - 1),))
```

**Key insight:** The `local_grads` tuple contains ∂output/∂input for each input.

---

## 3. Forward Mode vs Reverse Mode AD

### 3.1 Forward Mode: Pushing Derivatives Forward

**Idea:** Compute derivatives alongside the forward pass.

**For each node, track:** (value, derivative w.r.t. input)

```python
# Compute f(x) = (x * 2) + 1 at x = 3, with derivative

# x = 3, dx/dx = 1
x = (3, 1)

# t = x * 2
# t_value = 3 * 2 = 6
# dt/dx = (dx/dx) * 2 = 1 * 2 = 2
t = (6, 2)

# y = t + 1
# y_value = 6 + 1 = 7
# dy/dx = dt/dx = 2
y = (7, 2)

# Result: f(3) = 7, f'(3) = 2 ✓
```

**Limitation:** One forward pass per input derivative.

### 3.2 Reverse Mode: Backpropagating Gradients

**Idea:** First compute all values (forward pass), then compute gradients backward.

**Two phases:**

1. **Forward pass:** Build graph, compute all values
2. **Backward pass:** Start from output, apply chain rule backward

```python
# Same computation: y = (x * 2) + 1 at x = 3

# Forward pass (build graph)
x = Value(3)
t = x * 2  # t.data = 6
y = t + 1  # y.data = 7

# Backward pass (compute gradients)
y.grad = 1  # dy/dy = 1 (seed)

# Backprop through y = t + 1
# dy/dt = 1, so: t.grad += 1 * y.grad = 1
t.grad = 1

# Backprop through t = x * 2
# dt/dx = 2, so: x.grad += 2 * t.grad = 2
x.grad = 2

# Result: x.grad = 2 = dy/dx ✓
```

**Advantage:** One backward pass gives all input derivatives.

### 3.3 Why Reverse Mode for Neural Networks?

**Neural network structure:**
- Millions of parameters (inputs to the loss function)
- One loss value (output)

**Forward mode:** Millions of forward passes (impractical)
**Reverse mode:** One backward pass (efficient!)

---

## 4. Implementing the Value Class

### 4.1 Complete Implementation with Explanations

```python
class Value:
    """Scalar value that tracks computation graph for automatic differentiation."""

    __slots__ = ('data', 'grad', '_children', '_local_grads')

    def __init__(self, data, children=(), local_grads=()):
        self.data = data                # Numeric value
        self.grad = 0                   # Gradient of loss w.r.t. this value
        self._children = children       # Parent nodes in computation graph
        self._local_grads = local_grads # Local derivatives ∂self/∂child

    # === Arithmetic operations ===

    def __add__(self, other):
        """
        Forward: out = self.data + other.data
        Backward: dout/dself = 1, dout/dother = 1

        Chain rule: dL/dself = dL/dout × dout/dself = dL/dout × 1
        """
        other = other if isinstance(other, Value) else Value(other)
        out = Value(self.data + other.data, (self, other), (1, 1))
        return out

    def __mul__(self, other):
        """
        Forward: out = self.data * other.data
        Backward: dout/dself = other.data, dout/dother = self.data

        Chain rule: dL/dself = dL/dout × other.data
        """
        other = other if isinstance(other, Value) else Value(other)
        out = Value(self.data * other.data, (self, other), (other.data, self.data))
        return out

    def __pow__(self, other):
        """
        Forward: out = self.data ** other
        Backward: dout/dself = other * self.data ** (other - 1)

        This is the power rule: d(x^n)/dx = n * x^(n-1)
        """
        if isinstance(other, Value):
            raise NotImplementedError("Only constant exponents supported")
        out = Value(self.data ** other, (self,), (other * self.data ** (other - 1),))
        return out

    def __truediv__(self, other):
        """Division: a / b = a * b^(-1)"""
        return self * other ** -1

    # === Non-linearities ===

    def relu(self):
        """
        Forward: out = max(0, self.data)
        Backward: dout/dself = 1 if self.data > 0 else 0

        ReLU creates a 'gate' that either passes gradient (when positive)
        or blocks it (when negative).
        """
        out = Value(max(0, self.data), (self,), (float(self.data > 0),))
        return out

    def log(self):
        """
        Forward: out = ln(self.data)
        Backward: dout/dself = 1 / self.data

        d(ln(x))/dx = 1/x
        """
        out = Value(math.log(self.data), (self,), (1 / self.data,))
        return out

    def exp(self):
        """
        Forward: out = e^self.data
        Backward: dout/dself = e^self.data = out.data

        d(e^x)/dx = e^x (the derivative equals the function!)
        """
        out = Value(math.exp(self.data), (self,), (math.exp(self.data),))
        return out

    # === Helper operations ===

    def __neg__(self):
        """Negation: -x = -1 * x"""
        return self * -1

    def __sub__(self, other):
        """Subtraction: a - b = a + (-b)"""
        return self + (-other)

    def __radd__(self, other):
        """Right addition: 3 + Value(2)"""
        return self + other

    def __rmul__(self, other):
        """Right multiplication: 3 * Value(2)"""
        return self * other

    def __rsub__(self, other):
        """Right subtraction: 3 - Value(2)"""
        return Value(other) - self

    def __rtruediv__(self, other):
        """Right division: 3 / Value(2)"""
        return Value(other) / self
```

### 4.2 Memory Optimization: `__slots__`

```python
__slots__ = ('data', 'grad', '_children', '_local_grads')
```

**Why?** Python objects normally have a `__dict__` for attributes, which:
- Uses extra memory per object
- Is slightly slower to access

**With `__slots__`:** Python allocates fixed space for listed attributes only.

**Impact:** For millions of Value objects during training, this saves significant memory.

---

## 5. The Backward Pass Algorithm

### 5.1 The Chain Rule in Graph Form

**For any node in the graph:**
```
dL/dnode = Σ (dL/dchild × dchild/dnode)
           for all children that depend on node
```

**In words:** The gradient of the loss w.r.t. a node is the sum of:
- The gradient flowing from each child
- Multiplied by the local gradient at that child

### 5.2 Topological Sort: Ordering the Backward Pass

**Problem:** We must process nodes in reverse topological order (children before parents).

**Why?** To compute a node's gradient, we need all its children's gradients first.

```python
def backward(self):
    # Step 1: Build topological ordering
    topo = []
    visited = set()

    def build_topo(v):
        if v not in visited:
            visited.add(v)
            for child in v._children:
                build_topo(child)  # Recursively visit children first
            topo.append(v)  # Add after children (post-order)

    build_topo(self)

    # Step 2: Set gradient of output to 1
    # This is dL/dL = 1 (the loss gradient w.r.t. itself)
    self.grad = 1

    # Step 3: Backpropagate in reverse topological order
    for v in reversed(topo):
        # v.grad now contains dL/dv (accumulated from all paths)
        # Distribute this gradient to v's children
        for child, local_grad in zip(v._children, v._local_grads):
            # Chain rule: dL/dchild += dL/dv × dv/dchild
            child.grad += local_grad * v.grad
```

### 5.3 Visualizing the Backward Pass

**Forward computation:** `e = (a * b) + c`, where a=2, b=3, c=1

```
Forward graph:          Backward flow:

  a=2    b=3    c=1       dL/de = 1 (seed)
    \    /      |            ↑
     * (t=6)    |            |
       \       /             |
        + (e=7) ←------------+

Local gradients:
- dt/da = b = 3
- dt/db = a = 2
- de/dt = 1
- de/dc = 1
```

**Backward pass step-by-step:**

```
1. Initialize: e.grad = 1

2. Backprop through e = t + c:
   t.grad += de/dt × e.grad = 1 × 1 = 1
   c.grad += de/dc × e.grad = 1 × 1 = 1

3. Backprop through t = a * b:
   a.grad += dt/da × t.grad = 3 × 1 = 3
   b.grad += dt/db × t.grad = 2 × 1 = 2

Final gradients:
- da/de = 3 ✓ (∂e/∂a = ∂(ab)/∂a = b = 3)
- db/de = 2 ✓ (∂e/∂b = ∂(ab)/∂b = a = 2)
- dc/de = 1 ✓ (∂e/∂c = 1)
```

### 5.4 Gradient Accumulation

**Why `+=` instead of `=`?**

```python
child.grad += local_grad * v.grad  # Accumulate
```

**Reason:** A node may be used multiple times in the computation.

**Example:** `y = x * x` (x is used twice)

```python
x = Value(3)
y = x * x  # x appears twice in children
```

**Graph:**
```
     x
    / \
   *   *
    \ /
     + (if we had y = x*x + x)
```

**Backward:**
- First path: gradient flows through first use of x
- Second path: gradient flows through second use of x
- Total gradient = sum of both paths

This is the multivariate chain rule in action!

---

## 6. Worked Examples

### 6.1 Example 1: Simple Multiplication

```python
a = Value(2.0)
b = Value(3.0)
c = a * b
c.backward()

print(f"a.grad = {a.grad}")  # Expected: 3.0 (dc/da = b)
print(f"b.grad = {b.grad}")  # Expected: 2.0 (dc/db = a)
```

**Trace:**
```
Forward:
  a.data = 2.0, a.grad = 0
  b.data = 3.0, b.grad = 0
  c.data = 6.0, c.grad = 0
  c._children = (a, b)
  c._local_grads = (3.0, 2.0)  # (b.data, a.data)

Backward:
  1. c.grad = 1 (seed)
  2. Process c:
     a.grad += 3.0 * 1 = 3.0
     b.grad += 2.0 * 1 = 2.0
  3. Process a: no children, done
  4. Process b: no children, done

Result: a.grad = 3.0, b.grad = 2.0 ✓
```

### 6.2 Example 2: Neuron Forward and Backward

```python
# A single neuron: output = relu(w1*x1 + w2*x2 + b)

x1 = Value(2.0)
x2 = Value(0.0)
w1 = Value(-3.0)
w2 = Value(1.0)
b = Value(6.8)

# Forward pass
n = x1 * w1  # -6.0
n = n + x2 * w2  # -6.0 + 0.0 = -6.0
n = n + b  # -6.0 + 6.8 = 0.8
o = n.relu()  # max(0, 0.8) = 0.8

# Backward pass
o.backward()

print(f"∂o/∂w1 = {w1.grad}")  # How does output change if w1 changes?
print(f"∂o/∂x1 = {x1.grad}")
```

**Manual verification:**
```
o = relu(w1*x1 + w2*x2 + b)
o = relu(-3*2 + 1*0 + 6.8)
o = relu(0.8) = 0.8

For w1:
  ∂o/∂w1 = ∂o/∂n × ∂n/∂(w1*x1) × ∂(w1*x1)/∂w1
  ∂o/∂n = 1 (since n=0.8 > 0, ReLU derivative is 1)
  ∂n/∂(w1*x1) = 1
  ∂(w1*x1)/∂w1 = x1 = 2
  ∂o/∂w1 = 1 × 1 × 2 = 2

Wait, let's trace through the actual computation...

The computation is:
n1 = x1 * w1 = 2 * (-3) = -6
n2 = x2 * w2 = 0 * 1 = 0
n3 = n1 + n2 = -6 + 0 = -6
n4 = n3 + b = -6 + 6.8 = 0.8
o = n4.relu() = 0.8

Backward:
o.grad = 1
n4.grad = 1 × 1 = 1 (ReLU derivative at 0.8 is 1)
n3.grad = 1 (from n4 = n3 + b, derivative is 1)
b.grad = 1
n1.grad = 1 (from n3 = n1 + n2)
n2.grad = 1

n1 = x1 * w1, local_grads = (w1, x1) = (-3, 2)
x1.grad += -3 × 1 = -3
w1.grad += 2 × 1 = 2

Result: w1.grad = 2, x1.grad = -3
```

### 6.3 Example 3: Cross-Entropy Loss

```python
# Softmax + cross-entropy for a 3-class classification

logits = [Value(2.0), Value(1.0), Value(0.1)]  # Raw scores
target = 0  # Correct class is index 0

# Softmax
max_val = max(l.data for l in logits)  # 2.0
exps = [(l - max_val).exp() for l in logits]  # Numerical stability
total = sum(exps)
probs = [e / total for e in exps]

# Cross-entropy loss: -log(p[target])
loss = -probs[target].log()

# Backward
loss.backward()

print(f"∂loss/∂logits[0] = {logits[0].grad}")
```

**Why this matters:** This is exactly what happens in microgpt's training loop!

---

## 7. Common Pitfalls and Optimizations

### 7.1 Pitfall: Forgetting to Zero Gradients

```python
# Wrong!
for step in range(steps):
    loss = forward()
    loss.backward()
    update_params()  # Gradients still accumulated from previous step!

# Correct
for step in range(steps):
    loss = forward()
    loss.backward()
    update_params()
    zero_gradients()  # Reset grads to 0
```

### 7.2 Pitfall: Modifying Data During Backward

```python
# Don't do this!
def backward(self):
    for v in topo:
        v.data = 0  # Breaking the computation!
```

**Why?** The backward pass may need original values for gradient computation.

### 7.3 Optimization: Gradient Checkpointing

**Problem:** Storing all intermediate values for backprop uses too much memory.

**Solution:** Recompute some values during backward pass instead of storing.

**Trade-off:** More computation, less memory.

### 7.4 Optimization: Fusing Operations

```python
# Instead of separate operations:
x = a + b
y = x * c

# Fuse into one:
y = (a + b) * c  # One node instead of two
```

**Benefit:** Fewer nodes in graph, faster backward pass.

---

## Summary

1. **Automatic differentiation** computes gradients by applying the chain rule to a computation graph.

2. **Reverse mode AD** (backpropagation) is efficient for functions with many inputs and one output.

3. **The Value class** tracks:
   - `data`: The numeric value
   - `grad`: Accumulated gradient
   - `_children`: Parent nodes
   - `_local_grads`: Local derivatives

4. **The backward pass:**
   - Topologically sorts the graph
   - Seeds output gradient to 1
   - Applies chain rule in reverse order

5. **Gradient accumulation** (`+=`) handles nodes used multiple times.

---

*Next: Read transformer-architecture-deep-dive.md to understand how autograd enables training massive language models.*
