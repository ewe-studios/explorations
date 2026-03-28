---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/microgpt/microgpt.py
created_at: 2026-03-27
topic: Autograd and Backpropagation from First Principles
---

# Deep Dive: Autograd and Backpropagation - From Zero to ML Engineer

## Introduction: Why Should You Care About Autograd?

If you're new to machine learning, you might wonder: **what is autograd and why is it so important?**

This guide will take you from having zero knowledge about automatic differentiation to understanding exactly how neural networks "learn" - step by step, with no skipped details.

### What You'll Learn

1. What derivatives are (calculus, but simple)
2. The chain rule - the one calculus rule that powers all of deep learning
3. How to build a computation graph
4. How backpropagation actually works
5. How to implement it all in code

By the end, you'll understand the engine that drives all neural network training.

---

## Part 1: Calculus for Neural Networks (Yes, You Can Understand This)

### What is a Derivative?

**Simple definition:** A derivative tells you how much something changes when you change something else.

**Example:** Imagine you're adjusting the volume knob on a speaker.

- Turn the knob a little → volume changes a little
- The "derivative" is: how much does volume change per unit of knob turning?

**Mathematical notation:**
```
If y = f(x), then dy/dx = "how much does y change when x changes?"
```

**Concrete examples:**

```
y = 2x
dy/dx = 2  (y changes 2x as fast as x)

y = x²
dy/dx = 2x  (the rate depends on current x)

y = constant
dy/dx = 0  (y doesn't change at all)
```

### Why Derivatives Matter for ML

In machine learning:
- **y** = the loss (how wrong our predictions are)
- **x** = the model parameters (weights we can adjust)
- **dy/dx** = how to adjust weights to reduce loss

**The key insight:** If we know the derivative, we know which direction to move each parameter to make the model better!

---

## Part 2: The Chain Rule - The One Rule to Rule Them All

### What is the Chain Rule?

Neural networks are **compositions of functions** - one function after another.

```
input → f1 → intermediate → f2 → output
```

The chain rule tells us: **how does the output change when I change the input, through multiple functions?**

**Formula:**
```
If z = f(y) and y = g(x)
Then dz/dx = dz/dy × dy/dx
```

**Translation:** "The rate of change of z with respect to x equals the rate of change of z with respect to y, times the rate of change of y with respect to x."

### Visual Example

```
x --(multiply by 3)--> y --(add 5)--> z

Let's trace:
- x = 2
- y = 3x = 6
- z = y + 5 = 11

Now, how does z change when x changes?

Using chain rule:
dz/dx = dz/dy × dy/dx
dz/dy = 1 (z = y + 5, so z changes 1:1 with y)
dy/dx = 3 (y = 3x, so y changes 3x as fast as x)
dz/dx = 1 × 3 = 3
```

**Intuition:** A small change in x gets multiplied by 3 to affect y, then that change in y passes directly to z. So the total effect is 3x.

### Chain Rule in Neural Networks

A neural network is just a giant chain of operations:

```
input → embed → attention → mlp → head → loss
```

To train, we need: **how does loss change with respect to each parameter?**

The chain rule gives us the answer by chaining backward through all operations.

---

## Part 3: Building a Computation Graph

### What is a Computation Graph?

A **computation graph** is a way to represent mathematical expressions as a tree/graph structure.

**Example expression:** `(a + b) × (b + c)`

As a graph:
```
        ×
       / \
      +   +
     / \ / \
    a  b b  c
```

Each node represents:
1. A value (the result of the operation)
2. The operation itself (+, ×, etc.)
3. Pointers to child nodes (the inputs)

### Why Use Computation Graphs?

1. **Automatic gradient computation:** The graph structure tells us exactly how to apply the chain rule
2. **Efficiency:** We can compute all gradients in one backward pass
3. **Flexibility:** Works for any expression, not just neural networks

### Implementing a Value Class

Let's build the core data structure:

```rust
pub struct Value {
    pub data: f64,                    // The numeric value
    pub grad: f64,                    // The gradient (derivative of loss w.r.t. this)
    children: Vec<Rc<RefCell<Value>>>, // Inputs to this operation
    local_grads: Vec<f64>,            // Derivatives w.r.t. each child
}
```

**Each Value knows:**
- What value it holds (`data`)
- How much it contributes to the loss (`grad`)
- What values it was computed from (`children`)
- How changes propagate to its parents (`local_grads`)

### Implementing Operations

**Addition:**
```rust
impl Value {
    pub fn add(a: &Rc<RefCell<Self>>, b: &Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        let result = Self::new(a.borrow().data + b.borrow().data);

        // d(a+b)/da = 1, d(a+b)/db = 1
        result.borrow_mut().children = vec![Rc::clone(a), Rc::clone(b)];
        result.borrow_mut().local_grads = vec![1.0, 1.0];

        result
    }
}
```

**Multiplication:**
```rust
impl Value {
    pub fn mul(a: &Rc<RefCell<Self>>, b: &Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        let a_val = a.borrow().data;
        let b_val = b.borrow().data;
        let result = Self::new(a_val * b_val);

        // d(a*b)/da = b, d(a*b)/db = a
        result.borrow_mut().children = vec![Rc::clone(a), Rc::clone(b)];
        result.borrow_mut().local_grads = vec![b_val, a_val];

        result
    }
}
```

**Power:**
```rust
impl Value {
    pub fn pow(&self, exp: f64) -> Rc<RefCell<Self>> {
        let result = Self::new(self.data.powf(exp));

        // d(x^n)/dx = n * x^(n-1)
        result.borrow_mut().children = vec![Rc::clone(self)];
        result.borrow_mut().local_grads = vec![exp * self.data.powf(exp - 1.0)];

        result
    }
}
```

**Exponential:**
```rust
impl Value {
    pub fn exp(&self) -> Rc<RefCell<Self>> {
        let result = Self::new(self.data.exp());

        // d(e^x)/dx = e^x
        result.borrow_mut().children = vec![Rc::clone(self)];
        result.borrow_mut().local_grads = vec![result.borrow().data];

        result
    }
}
```

**Logarithm:**
```rust
impl Value {
    pub fn log(&self) -> Rc<RefCell<Self>> {
        let result = Self::new(self.data.ln());

        // d(ln(x))/dx = 1/x
        result.borrow_mut().children = vec![Rc::clone(self)];
        result.borrow_mut().local_grads = vec![1.0 / self.data];

        result
    }
}
```

**ReLU (Rectified Linear Unit):**
```rust
impl Value {
    pub fn relu(&self) -> Rc<RefCell<Self>> {
        let result = Self::new(self.data.max(0.0));

        // d(relu(x))/dx = 1 if x > 0, else 0
        result.borrow_mut().children = vec![Rc::clone(self)];
        result.borrow_mut().local_grads = vec![if self.data > 0.0 { 1.0 } else { 0.0 }];

        result
    }
}
```

---

## Part 4: The Backward Pass - Backpropagation Explained

### What is Backpropagation?

**Backpropagation** (short for "backpropagation of error") is the algorithm that:
1. Starts from the loss
2. Walks backward through the computation graph
3. Applies the chain rule at each node
4. Computes gradients for all parameters

### Step-by-Step Example

Let's trace through a simple computation:

```
Computation: loss = (a × b + c)²
Where: a = 2, b = 3, c = 1
```

**Forward pass (building the graph):**
```
Step 1: d = a × b = 2 × 3 = 6
Step 2: e = d + c = 6 + 1 = 7
Step 3: loss = e² = 49
```

Graph structure:
```
      loss = e²
        |
        e = d + c
       / \
      d   c
      |
      a × b
```

**Backward pass (computing gradients):**

```
We want: d(loss)/da, d(loss)/db, d(loss)/dc

Start: d(loss)/d(loss) = 1 (by definition)

Step 1: Gradient through e²
  d(loss)/de = d(loss)/d(e²) × d(e²)/de
             = 1 × 2e
             = 2 × 7 = 14

Step 2: Gradient through d + c
  d(loss)/dd = d(loss)/de × d(e)/dd = 14 × 1 = 14
  d(loss)/dc = d(loss)/de × d(e)/dc = 14 × 1 = 14

Step 3: Gradient through a × b
  d(loss)/da = d(loss)/dd × d(d)/da = 14 × b = 14 × 3 = 42
  d(loss)/db = d(loss)/dd × d(d)/db = 14 × a = 14 × 2 = 28
```

**Final gradients:**
```
d(loss)/da = 42
d(loss)/db = 28
d(loss)/dc = 14
```

**Interpretation:**
- Changing `a` has the biggest impact on the loss
- Changing `c` has the smallest impact
- To reduce the loss, we'd adjust these values in the opposite direction of their gradients

### The Backward Algorithm

Here's the algorithm implemented:

```rust
impl Value {
    pub fn backward(&self) {
        // Step 1: Topological sort (parents before children)
        let mut topo = Vec::new();
        let mut visited = HashSet::new();

        fn build_topo(
            v: &Rc<RefCell<Value>>,
            topo: &mut Vec<Rc<RefCell<Value>>>,
            visited: &mut HashSet<usize>
        ) {
            let id = Rc::as_ptr(v) as usize;
            if !visited.contains(&id) {
                visited.insert(id);
                // Visit children first
                for child in &v.borrow().children {
                    build_topo(child, topo, visited);
                }
                // Then add this node
                topo.push(Rc::clone(v));
            }
        }

        build_topo(&Rc::new(RefCell::new(self.clone())), &mut topo, &mut visited);

        // Step 2: Set output gradient to 1
        self.grad = 1.0;

        // Step 3: Backpropagate in reverse topological order
        for v in topo.iter().rev() {
            let grad = v.borrow().grad;
            let children = v.borrow().children.clone();
            let local_grads = v.borrow().local_grads.clone();

            // Apply chain rule at each node
            for (child, local_grad) in children.iter().zip(local_grads.iter()) {
                child.borrow_mut().grad += local_grad * grad;
            }
        }
    }
}
```

### Why Reverse Topological Order?

We need to process nodes **after** all their dependents have been processed.

```
Correct order (backward): loss → e → d,c → a,b
Wrong order: a,b before d (would compute gradients incorrectly)
```

Reverse topological order guarantees that when we process a node, we've already accumulated all the gradients flowing into it from downstream.

---

## Part 5: Using Autograd for Neural Networks

### A Simple Neural Network Example

Let's build the simplest possible neural network:

```
input (x) → weight (w) → prediction (y_pred) → compare to target (y) → loss
```

**Forward pass:**
```rust
// Single neuron: y_pred = w × x
let x = Value::new(2.0);     // Input
let w = Value::new(0.5);     // Weight (to be learned)
let y_target = Value::new(1.0); // Desired output

// Forward: prediction
let y_pred = Value::mul(&w, &x);  // 0.5 × 2 = 1.0

// Loss: mean squared error
let error = Value::sub(&y_pred, &y_target);  // 1.0 - 1.0 = 0.0
let loss = Value::pow(&error, 2.0);  // 0.0² = 0.0
```

**Backward pass:**
```rust
loss.borrow_mut().backward();

// Now w.grad tells us how to adjust w to reduce loss
println!("Gradient w.r.t. w: {}", w.borrow().grad);
```

**Weight update (gradient descent):**
```rust
let learning_rate = 0.01;
let mut new_w = w.borrow().data - learning_rate * w.borrow().grad;
```

### Scaling to Real Networks

A real neural network has millions of parameters, but the principle is identical:

1. **Forward pass:** Compute predictions, building computation graph
2. **Compute loss:** Compare predictions to targets
3. **Backward pass:** Call `backward()` to compute all gradients
4. **Update weights:** Adjust each parameter by `-learning_rate × gradient`

That's exactly what the microgpt `Value` class does, just scaled up to:
- Transformer architecture (attention + MLP)
- Multiple layers
- Many parameters

---

## Part 6: Common Questions and Misconceptions

### Q: Why is it called "automatic" differentiation?

**A:** Because the computer automatically computes derivatives for you. You don't need to manually derive gradients - just define the forward computation.

**Before autograd:**
```python
# Manually derived gradient for loss = (wx - y)²
# d(loss)/dw = 2(wx - y) × x
gradient = 2 * (w * x - y) * x
```

**With autograd:**
```python
# Just define the forward pass
loss = (w * x - y) ** 2
loss.backward()  # Autograd computes gradient automatically
gradient = w.grad
```

### Q: What's the difference between numerical and symbolic differentiation?

**Numerical differentiation** (finite differences):
```
f'(x) ≈ (f(x + h) - f(x)) / h
```
- Simple but slow and inaccurate for large networks
- Requires 2 function evaluations per parameter

**Symbolic differentiation:**
- Manipulates mathematical expressions symbolically
- Can produce complex expressions
- Used by computer algebra systems

**Automatic differentiation (what we use):**
- Computes exact derivatives (not approximations)
- Efficient: one forward + one backward pass
- Scales to millions of parameters

### Q: Why do we need a computation graph? Why not just compute gradients directly?

**A:** The graph:
1. Tracks dependencies (what depends on what)
2. Enables efficient gradient computation via chain rule
3. Allows reusing intermediate results
4. Makes it possible to handle arbitrary network architectures

### Q: What is a "leaf" node vs. "intermediate" node?

**Leaf nodes:** Inputs/parameters (not computed from other values)
**Intermediate nodes:** Results of operations (computed from other values)

In neural networks:
- **Leaf nodes we care about:** Parameters (weights) - we want their gradients
- **Intermediate nodes:** Activations - needed for chain rule but discarded after

---

## Summary: Key Takeaways

1. **Derivatives** tell you how much something changes when you change something else

2. **The chain rule** lets you compute derivatives through compositions of functions:
   ```
   dz/dx = dz/dy × dy/dx
   ```

3. **Computation graphs** represent expressions as trees, enabling automatic gradient computation

4. **Backpropagation** is:
   - Forward pass: compute values, build graph
   - Backward pass: apply chain rule in reverse order

5. **The Value class** implements:
   - Operations: add, mul, pow, exp, log, relu
   - Each stores: data, grad, children, local_grads
   - `backward()` applies chain rule to compute all gradients

6. **Neural network training** uses autograd to compute gradients, then updates parameters to reduce loss

---

## Exercises to Test Your Understanding

1. **Manual gradient computation:** For `f(x) = (2x + 3)²`, compute df/dx at x = 1 by hand, then verify with the chain rule.

2. **Build a computation graph:** Draw the graph for `f(a,b,c) = (a + b) × (b + c)²`

3. **Implement a new operation:** Add a `tanh()` function to the Value class. What's the derivative? (Answer: 1 - tanh²(x))

4. **Trace backpropagation:** Given `loss = (a × b - c)²` with a=2, b=3, c=5, trace through the backward pass and compute all gradients.

---

## Next Steps

Now that you understand autograd:
1. Read the **Transformer Architecture Deep Dive** to understand the model structure
2. Read the **Attention Mechanism Deep Dive** to understand how GPT processes sequences
3. Read the **Training Loop Deep Dive** to understand how models actually learn

You now have the foundation to understand how neural networks learn!
