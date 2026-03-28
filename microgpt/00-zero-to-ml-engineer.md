---
title: "Zero to ML Engineer: A First-Principles Journey Through microgpt"
subtitle: "Complete textbook-style guide from mathematical foundations to training neural networks"
based_on: "Karpathy's microgpt - Pure Python GPT Implementation"
level: "Beginner to Intermediate - No prior ML knowledge assumed"
---

# Zero to ML Engineer: First-Principles Guide

## Table of Contents

1. [What is Machine Learning?](#1-what-is-machine-learning)
2. [Mathematical Foundations](#2-mathematical-foundations)
3. [Neural Networks from Scratch](#3-neural-networks-from-scratch)
4. [The Transformer Architecture](#4-the-transformer-architecture)
5. [Training Deep Networks](#5-training-deep-networks)
6. [From microgpt to Production](#6-from-microgpt-to-production)

---

## 1. What is Machine Learning?

### 1.1 The Fundamental Question

**What problem does machine learning solve?**

Traditional programming follows this pattern:
```
INPUT + PROGRAM (rules written by human) → OUTPUT
```

Example: A calculator program. You write explicit rules:
- If user presses "+" then add numbers
- If user presses "×" then multiply numbers

**Machine learning inverts this:**
```
INPUT + OUTPUT (examples) → PROGRAM (learned rules)
```

Example: Teaching a computer to recognize names. You don't write rules like "names start with capital letters" because:
- Some names don't (e.g., "van Gogh")
- There are too many exceptions
- The rules are hard to articulate

Instead, you show thousands of examples of names and non-names, and the computer **learns the pattern**.

### 1.2 The Learning Problem Formalized

A machine learning problem has three components:

**1. Task (T):** What should the model do?
- For microgpt: Generate plausible names character-by-character

**2. Experience (E):** What data does it learn from?
- For microgpt: A list of 32,000 names like "emma", "olivia", "alex"

**3. Performance Measure (P):** How do we know it's improving?
- For microgpt: How closely generated names match real name patterns (measured by loss)

### 1.3 Types of Learning

| Type | What You Have | Example |
|------|---------------|---------|
| **Supervised** | Input-Output pairs | Names → Next character |
| **Unsupervised** | Only inputs | Cluster similar names |
| **Reinforcement** | Rewards/penalties | Learn via trial and error |

**microgpt uses supervised learning:** Given previous characters, predict the next character.

---

## 2. Mathematical Foundations

### 2.1 Functions: The Language of ML

**Definition:** A function maps inputs to outputs.
```
f(x) = y
```

In ML, our model IS a function:
```
model(input_tokens) = predicted_next_token
```

**Key insight:** Learning = finding the right function parameters.

### 2.2 Derivatives: Measuring Change

**Question:** How does the output change when I tweak the input?

**Definition:** The derivative measures the rate of change.
```
f(x) = x²
f'(x) = 2x  (the derivative)
```

At x = 3:
- f(3) = 9
- f'(3) = 6 (for every unit x increases, f(x) increases by 6)

**Why this matters for ML:**
- We want to know: "How should I adjust my model's parameters to reduce error?"
- The derivative tells us the direction and magnitude of adjustment.

### 2.3 The Chain Rule: Derivatives of Compositions

**Scenario:** You have nested functions:
```
y = f(g(x))
```

**Question:** What is dy/dx?

**Answer (Chain Rule):**
```
dy/dx = df/dg × dg/dx
```

**Why this matters for ML:**
- Neural networks are compositions of many functions
- To find how loss changes with respect to early-layer parameters, we multiply derivatives along the path
- This is **backpropagation**

### 2.4 Worked Example: Chain Rule in Action

Let's compute the derivative of a composed function step by step:

```
g(x) = x + 2
f(g) = g²
y = f(g(x)) = (x + 2)²
```

**Method 1: Expand first**
```
y = (x + 2)² = x² + 4x + 4
dy/dx = 2x + 4
```

**Method 2: Chain Rule**
```
dg/dx = 1
df/dg = 2g
dy/dx = df/dg × dg/dx = 2g × 1 = 2(x + 2) = 2x + 4 ✓
```

**This is exactly what backpropagation does**, but with thousands of nested functions.

### 2.5 Partial Derivatives: Multiple Inputs

**Scenario:** A function with multiple inputs:
```
f(x, y) = x² + xy + y²
```

**Question:** How does f change when x changes (holding y constant)?

**Answer:** Partial derivative with respect to x:
```
∂f/∂x = 2x + y  (treat y as a constant)
∂f/∂y = x + 2y  (treat x as a constant)
```

**Why this matters:** Neural network parameters are high-dimensional. We need partial derivatives for each parameter.

### 2.6 Gradients: The Direction of Steepest Ascent

**Definition:** The gradient is a vector of all partial derivatives:
```
∇f = [∂f/∂x₁, ∂f/∂x₂, ..., ∂f/∂xₙ]
```

**Geometric interpretation:** The gradient points in the direction of steepest increase.

**For optimization:** We go in the **opposite** direction (gradient descent):
```
new_x = old_x - learning_rate × gradient
```

---

## 3. Neural Networks from Scratch

### 3.1 The Neuron: Basic Computational Unit

**Biological inspiration:** A neuron receives signals, processes them, and sends output.

**Mathematical model:**
```
output = activation(w₁x₁ + w₂x₂ + ... + wₙxₙ + b)
```

Where:
- xᵢ = inputs
- wᵢ = weights (learnable parameters)
- b = bias (learnable parameter)
- activation = non-linear function (e.g., ReLU, sigmoid)

### 3.2 Implementing a Neuron in Python

```python
def neuron(inputs, weights, bias):
    # Compute weighted sum
    weighted_sum = sum(w * x for w, x in zip(weights, inputs)) + bias

    # Apply activation (ReLU example)
    output = max(0, weighted_sum)  # ReLU: outputs 0 for negative, identity for positive

    return output
```

### 3.3 From Neurons to Networks

**Single layer:** Multiple neurons processing the same inputs:
```
layer_outputs = [neuron(inputs, w_i, b_i) for each neuron i]
```

**Multiple layers:** Output of one layer becomes input to next:
```
hidden = layer1(inputs)
output = layer2(hidden)
```

**Why multiple layers?** Each layer learns increasingly abstract representations:
- Layer 1: Character patterns (e.g., "th", "qu")
- Layer 2: Syllable structures
- Layer 3: Name-level patterns

### 3.4 The Learning Algorithm: Gradient Descent

**Goal:** Find weights that minimize prediction error.

**Algorithm:**
1. Initialize weights randomly
2. Forward pass: Compute predictions
3. Compute loss (error between prediction and target)
4. Backward pass: Compute gradients (how much each weight contributed to error)
5. Update weights: `weight = weight - learning_rate × gradient`
6. Repeat until loss is low

### 3.5 Autograd: Automatic Differentiation

**Problem:** Computing gradients by hand is error-prone and tedious.

**Solution:** Build a computation graph during forward pass, then apply chain rule automatically.

**How microgpt's Value class works:**

```python
class Value:
    def __init__(self, data, children=(), local_grads=()):
        self.data = data              # The actual value
        self.grad = 0                 # Accumulated gradient
        self._children = children     # Parent nodes in graph
        self._local_grads = local_grads  # Local derivatives

    def __add__(self, other):
        # Forward: add values
        # Backward: gradient flows with multiplier 1
        return Value(self.data + other.data, (self, other), (1, 1))

    def __mul__(self, other):
        # Forward: multiply values
        # Backward: gradient of self is other.data, gradient of other is self.data
        return Value(self.data * other.data, (self, other), (other.data, self.data))

    def backward(self):
        # 1. Topological sort (process children before parents)
        topo = []
        visited = set()
        def build_topo(v):
            if v not in visited:
                visited.add(v)
                for child in v._children:
                    build_topo(child)
                topo.append(v)
        build_topo(self)

        # 2. Set output gradient to 1 (dL/dL = 1)
        self.grad = 1

        # 3. Apply chain rule in reverse order
        for v in reversed(topo):
            for child, local_grad in zip(v._children, v._local_grads):
                child.grad += local_grad * v.grad  # Chain rule!
```

### 3.6 Backward Pass: Step-by-Step Example

**Computation:** `c = a * b`, where `a = 3`, `b = 4`

**Forward pass:**
```
a = Value(3)
b = Value(4)
c = a * b  # c.data = 12
```

**Backward pass** (assuming dc/dc = 1):
```
c.grad = 1
dc/da = b.data = 4
dc/db = a.data = 3
a.grad += 4 * 1 = 4
b.grad += 3 * 1 = 3
```

**Interpretation:** If we increase `a` by a tiny amount, `c` increases by 4 times that amount.

---

## 4. The Transformer Architecture

### 4.1 Why Transformers?

**Problem with earlier architectures:**

| Architecture | Limitation |
|-------------|------------|
| Feed-forward networks | No notion of sequence order |
| RNNs/LSTMs | Sequential processing (slow), forget long-range dependencies |

**Transformer solution:**
- Process all positions in parallel
- Use attention to connect any two positions directly
- Scale efficiently with data and compute

### 4.2 Embeddings: Converting Tokens to Vectors

**Problem:** Neural networks work with numbers, but we have text.

**Solution:** Look up each token in an embedding table:
```python
token_id = 5  # e.g., the character 'e'
embedding = embedding_table[5]  # e.g., [0.2, -0.5, 0.8, ...]
```

**microgpt has two embeddings:**
1. **Token embedding:** What character is it?
2. **Position embedding:** Where is it in the sequence?

```python
tok_emb = wte[token_id]    # Token embedding
pos_emb = wpe[position_id] # Position embedding
x = [t + p for t, p in zip(tok_emb, pos_emb)]  # Combine
```

**Why add them?** The network learns to separate "what" from "where" in different dimensions.

### 4.3 Attention: The Core Innovation

**Key idea:** Every position should attend to (gather information from) relevant positions.

**Intuition:** In "the cat sat on the mat", to understand "mat", you might want to attend to "cat".

#### 4.3.1 The Attention Mechanism

**Step 1: Compute Query, Key, Value**
```python
query = linear(x, wq)  # "What am I looking for?"
key = linear(x, wk)    # "What do I contain?"
value = linear(x, wv)  # "What information do I have?"
```

**Step 2: Compute attention scores**
```python
# Dot product of query with all keys
score[t] = sum(q[i] * k[t][i] for i in range(dim)) / sqrt(dim)
```

**Step 3: Softmax to get weights**
```python
weights = softmax(scores)  # Converts to probabilities (sum to 1)
```

**Step 4: Weighted sum of values**
```python
output[t][i] = sum(weights[t] * value[t][i] for t in range(num_tokens))
```

#### 4.3.2 Why Divide by sqrt(dim)?

**Problem:** Large dimensions cause dot products to be huge.

**Consequence:** Softmax saturates (becomes very confident), gradients vanish.

**Solution:** Scale by `1/sqrt(dim)` to keep values in a reasonable range.

### 4.4 Multi-Head Attention

**Idea:** Have multiple "heads" attending to different things.

```python
# Split embedding into heads
for head in range(n_head):
    head_start = head * head_dim
    head_end = head_start + head_dim

    q_head = query[head_start:head_end]
    k_head = [k[head_start:head_end] for k in all_keys]
    v_head = [v[head_start:head_end] for v in all_values]

    # Compute attention for this head
    head_output = attention(q_head, k_head, v_head)

# Concatenate all heads
output = concatenate(all_head_outputs)
```

**Why multiple heads?** Different heads learn different attention patterns:
- Head 1: Attend to previous character (local patterns)
- Head 2: Attend to start of name (prefix patterns)
- Head 3: Attend to similar characters (repetition patterns)

### 4.5 Feed-Forward Network (MLP)

**After attention, process each position independently:**

```python
x_residual = x
x = rmsnorm(x)  # Normalize
x = linear(x, fc1)  # Expand to 4× dimension
x = relu(x)  # Non-linearity
x = linear(x, fc2)  # Project back
x = x + x_residual  # Residual connection
```

**Why expand to 4×?** Gives the network capacity to learn complex transformations.

### 4.6 RMSNorm: Stabilizing Training

**Problem:** As values flow through many layers, they can explode or vanish.

**Solution:** Normalize at each layer.

**LayerNorm:** `x_normalized = (x - mean) / std`

**RMSNorm (simpler):** `x_normalized = x / sqrt(mean(x²) + epsilon)`

```python
def rmsnorm(x):
    ms = sum(xi * xi for xi in x) / len(x)  # Mean of squares
    scale = (ms + 1e-5) ** -0.5  # 1 / sqrt(ms + epsilon)
    return [xi * scale for xi in x]
```

**Why no mean subtraction?** Empirically works as well with fewer operations.

### 4.7 Residual Connections: The Highway for Gradients

**Pattern:** `output = sublayer(x) + x`

**Why?**
1. Easier to learn identity (if sublayer should do nothing, output ≈ input)
2. Gradients flow directly through the addition (no vanishing through many layers)

```python
# In attention block
x_attn = attention(x)
x = x_attn + x  # Residual

# In MLP block
x_mlp = mlp(x)
x = x_mlp + x  # Residual
```

---

## 5. Training Deep Networks

### 5.1 The Training Loop

```python
for step in range(num_steps):
    # 1. Get a batch of data
    doc = docs[step % len(docs)]
    tokens = [BOS] + [encode(c) for c in doc] + [BOS]

    # 2. Forward pass
    keys, values = [], []  # KV cache for autoregressive generation
    for pos in range(len(tokens) - 1):
        logits = gpt(tokens[pos], pos, keys, values)
        probs = softmax(logits)
        loss = -log(probs[tokens[pos + 1]])  # Cross-entropy

    # 3. Backward pass
    loss.backward()

    # 4. Update parameters
    optimizer.step()

    # 5. Reset gradients
    optimizer.zero_grad()
```

### 5.2 Loss Functions: Measuring Error

**Cross-entropy loss** (used in microgpt):
```
loss = -log(probability_of_correct_token)
```

**Why this loss?**
- If probability of correct token is 1.0, loss = 0 (perfect)
- If probability is 0.01, loss = -log(0.01) = 4.6 (bad)
- Strongly penalizes confident wrong predictions

### 5.3 The Adam Optimizer

**Problem:** Vanilla gradient descent can be slow or unstable.

**Adam solution:** Track momentum and adaptive learning rates.

```python
# Parameters
beta1 = 0.85   # Momentum decay
beta2 = 0.99   # Variance decay
eps = 1e-8     # Numerical stability

# For each parameter p:
m[i] = beta1 * m[i] + (1 - beta1) * p.grad  # First moment (momentum)
v[i] = beta2 * v[i] + (1 - beta2) * p.grad ** 2  # Second moment (variance)

# Bias correction (important in early steps)
m_hat = m[i] / (1 - beta1 ** (step + 1))
v_hat = v[i] / (1 - beta2 ** (step + 1))

# Update
p.data -= learning_rate * m_hat / (sqrt(v_hat) + eps)
```

**Intuition:**
- **Momentum (m):** Smooths updates (don't oscillate, keep moving in consistent direction)
- **Variance (v):** Adapt learning rate per parameter (larger updates for rare features)

### 5.4 Learning Rate Scheduling

**microgpt uses linear decay:**
```python
lr_t = learning_rate * (1 - step / num_steps)
```

**Why decay learning rate?**
- Early training: Large steps to quickly find good region
- Late training: Small steps to fine-tune without overshooting

### 5.5 Autoregressive Generation

**Training:** Predict next token given all previous tokens (parallel)

**Inference:** Generate one token at a time, feeding back the output:

```python
token_id = BOS
for pos in range(max_length):
    logits = gpt(token_id, pos, keys, values)
    probs = softmax(logits / temperature)
    token_id = sample_from(probs)  # Random sample based on probabilities
    if token_id == BOS:
        break  # End of sequence
    output.append(decode(token_id))
```

### 5.6 Temperature: Controlling Creativity

**Formula:** `probs = softmax(logits / temperature)`

| Temperature | Effect | Use Case |
|-------------|--------|----------|
| 0.1 | Very deterministic, picks highest probability | Code generation |
| 0.5 | Balanced (microgpt default) | Name generation |
| 1.0 | Full randomness per probabilities | Creative writing |
| 2.0 | More random than softmax | Exploration |

**Why it works:** Dividing by T < 1 amplifies differences; dividing by T > 1 flattens distribution.

---

## 6. From microgpt to Production

### 6.1 What microgpt Teaches

1. **Simplicity:** The core algorithm is ~200 lines
2. **Autograd:** Automatic differentiation is conceptually simple
3. **Transformers:** The architecture is just repeated attention + MLP blocks
4. **Training:** Gradient descent is universal

### 6.2 What microgpt Doesn't Teach (Production Concerns)

| Aspect | microgpt | Production |
|--------|----------|------------|
| **Speed** | Pure Python, scalar ops | GPU, batched matrix ops |
| **Memory** | Full precision, no optimization | Mixed precision, gradient checkpointing |
| **Scale** | ~76K parameters, 1 layer | Billions of parameters, dozens of layers |
| **Data** | 32K names | Web-scale datasets |
| **Evaluation** | Visual inspection of outputs | Quantitative metrics, human evaluation |

### 6.3 Scaling Up: What Changes

**Matrix operations:** Replace nested loops with GPU kernels
```python
# microgpt (scalar)
result = sum(wi * xi for wi, xi in zip(weights, inputs))

# Production (matrix)
result = weights @ inputs  # GPU-accelerated matrix multiply
```

**Batching:** Process multiple examples in parallel
```python
# microgpt (single sequence)
for pos in range(n):
    logits = gpt(tokens[pos], pos)

# Production (batch)
logits = gpt(batch_tokens)  # Shape: [batch_size, seq_len, vocab_size]
```

**Mixed precision:** Use 16-bit floats for speed, 32-bit for stability
```python
# Forward pass in FP16
with autocast():
    output = model(inputs)

# Optimizer in FP32
optimizer.step()
```

### 6.4 Your Path Forward

**To become an ML engineer:**

1. **Understand the fundamentals** (this document)
2. **Implement from scratch** (you've seen microgpt)
3. **Use frameworks** (PyTorch, JAX) for real projects
4. **Read papers** (Attention Is All You Need, Adam paper)
5. **Build projects** (fine-tune models, create applications)
6. **Study production systems** (Hugging Face, vllm, TGI)

**Key resources:**
- [Andrej Karpathy's YouTube](https://youtube.com/@AndrejKarpathy) - Neural Networks: Zero to Hero
- [PyTorch tutorials](https://pytorch.org/tutorials/)
- [Hugging Face Course](https://huggingface.co/learn)
- [The Transformer Family](https://lilianweng.github.io/posts/2020-04-07-the-transformer-family/)

---

## Appendix A: Calculus Reference

### Derivatives of Common Functions

| Function | Derivative |
|----------|------------|
| f(x) = c | f'(x) = 0 |
| f(x) = x | f'(x) = 1 |
| f(x) = x² | f'(x) = 2x |
| f(x) = xⁿ | f'(x) = nxⁿ⁻¹ |
| f(x) = eˣ | f'(x) = eˣ |
| f(x) = ln(x) | f'(x) = 1/x |
| f(x) = sin(x) | f'(x) = cos(x) |
| f(x) = ReLU(x) | f'(x) = 1 if x > 0, else 0 |

### Chain Rule Patterns

```
f(g(x))     →  f'(g(x)) × g'(x)
f(g(h(x)))  →  f'(g(h(x))) × g'(h(x)) × h'(x)
```

---

## Appendix B: Linear Algebra Reference

### Vector Operations

```python
# Dot product
a · b = sum(a[i] * b[i] for i in range(len(a)))

# Vector addition
a + b = [a[i] + b[i] for i in range(len(a))]

# Scalar multiplication
c * a = [c * a[i] for i in range(len(a))]
```

### Matrix Operations

```python
# Matrix-vector multiplication
y = W @ x  where  y[i] = sum(W[i][j] * x[j] for j in range(len(x)))

# Matrix-matrix multiplication
C = A @ B  where  C[i][j] = sum(A[i][k] * B[k][j] for k in range(...))
```

### Why Matrices for Neural Networks?

**Efficiency:** GPUs are optimized for matrix operations.

**Batching:** Process many inputs at once:
```
[batch_size, input_dim] @ [input_dim, output_dim] = [batch_size, output_dim]
```

---

*This document is a living textbook. Revisit sections as concepts become clearer through implementation.*
