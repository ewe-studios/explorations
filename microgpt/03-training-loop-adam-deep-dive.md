---
title: "Training Loop and Adam Optimizer: Complete Deep Dive"
subtitle: "From loss computation to parameter updates - the complete training algorithm"
based_on: "microgpt/microgpt.py - Training loop and Adam implementation"
prerequisites: "Backpropagation basics, gradient descent fundamentals"
---

# Training Loop and Adam Optimizer Deep Dive

## Table of Contents

1. [The Training Problem](#1-the-training-problem)
2. [Loss Functions: Measuring Error](#2-loss-functions-measuring-error)
3. [The Training Loop Architecture](#3-the-training-loop-architecture)
4. [Adam Optimizer Deep Dive](#4-adam-optimizer-deep-dive)
5. [Learning Rate Scheduling](#5-learning-rate-scheduling)
6. [Complete Training Trace](#6-complete-training-trace)
7. [Debugging Training Issues](#7-debugging-training-issues)

---

## 1. The Training Problem

### 1.1 What Are We Optimizing?

**Goal:** Find parameters θ that minimize the loss on our dataset.

```
θ* = argmin_θ E_{(x,y)~data}[L(f(x;θ), y)]
```

**In microgpt:**
- θ = all parameters in state_dict (wte, wpe, attention weights, MLP weights, lm_head)
- x = input token sequence
- y = target token (next token in sequence)
- f(x;θ) = GPT forward pass output (logits)
- L = cross-entropy loss

### 1.2 The Optimization Landscape

**Visualization (2D slice):**
```
     Loss
      ^
      |     /\
      |    /  \
      |   /    \
      |  /      \___
      | /          \__
      |/______________\__  θ
```

**Gradient descent:** Move in direction of steepest descent.

```
θ_new = θ_old - learning_rate × ∇L(θ_old)
```

### 1.3 Why Not Just Use Vanilla Gradient Descent?

**Problems with vanilla GD:**

| Problem | Description | Consequence |
|---------|-------------|-------------|
| **Oscillations** | Gradient bounces between valley walls | Slow convergence |
| **Different scales** | Some parameters need large updates, others small | Must use small LR for all |
| **Local minima** | Can get stuck in suboptimal solutions | Poor final performance |
| **Saddle points** | Gradient ≈ 0 but not at minimum | Training stalls |

**Adam solves these problems.**

---

## 2. Loss Functions: Measuring Error

### 2.1 Cross-Entropy Loss in microgpt

```python
# Forward pass produces logits (raw scores)
logits = gpt(token_id, pos_id, keys, values)  # [vocab_size]

# Convert to probabilities
probs = softmax(logits)  # [vocab_size], sums to 1

# Cross-entropy loss for target token
loss = -probs[target_id].log()
```

**Why this formula?**

```
Let p = prob assigned to correct token

If p = 1.0 (perfect): loss = -log(1) = 0
If p = 0.5 (uncertain): loss = -log(0.5) = 0.69
If p = 0.1 (confident wrong): loss = -log(0.1) = 2.3
If p = 0.01 (very wrong): loss = -log(0.01) = 4.6
```

**Key property:** Heavily penalizes confident wrong predictions.

### 2.2 Average Loss Over Sequence

```python
losses = []
for pos_id in range(n):
    token_id, target_id = tokens[pos_id], tokens[pos_id + 1]
    logits = gpt(token_id, pos_id, keys, values)
    probs = softmax(logits)
    loss_t = -probs[target_id].log()
    losses.append(loss_t)

# Average loss across all positions
loss = (1 / n) * sum(losses)
```

**Why average?**
- Normalize for different sequence lengths
- Each position contributes equally to gradient
- More stable training

### 2.3 Backward Pass Through Loss

```python
# loss = -log(p[target])
# We need: ∂loss/∂logits

# Chain rule:
# ∂loss/∂logits[i] = ∂loss/∂p[target] × ∂p[target]/∂logits[i]

# For the target class:
# ∂loss/∂logits[target] = -1/p[target] × p[target] × (1 - p[target])
#                       = -(1 - p[target])
#                       = p[target] - 1

# For non-target classes:
# ∂loss/∂logits[i] = -1/p[target] × p[target] × (-p[i])
#                  = p[i]
```

**Result:** The gradient of cross-entropy + softmax is simply:
```
∂loss/∂logits = probs - one_hot(target)
```

---

## 3. The Training Loop Architecture

### 3.1 Complete Training Loop with Annotations

```python
# === HYPERPARAMETERS ===
learning_rate = 0.01    # Base learning rate
beta1 = 0.85            # Momentum decay (first moment)
beta2 = 0.99            # Variance decay (second moment)
eps_adam = 1e-8         # Numerical stability constant
num_steps = 1000        # Total training iterations

# === OPTIMIZER STATE ===
m = [0.0] * len(params)  # First moment buffer (momentum)
v = [0.0] * len(params)  # Second moment buffer (variance)

# === TRAINING LOOP ===
for step in range(num_steps):

    # --- 1. GET DATA ---

    # Select document (cyclic through dataset)
    doc = docs[step % len(docs)]

    # Tokenize with BOS wrappers
    tokens = [BOS] + [uchars.index(ch) for ch in doc] + [BOS]
    n = min(block_size, len(tokens) - 1)  # Truncate to context length

    # --- 2. FORWARD PASS ---

    # Initialize KV caches for autoregressive attention
    keys, values = [[] for _ in range(n_layer)], [[] for _ in range(n_layer)]

    losses = []
    for pos_id in range(n):
        token_id, target_id = tokens[pos_id], tokens[pos_id + 1]

        # Forward through GPT model
        logits = gpt(token_id, pos_id, keys, values)

        # Softmax to get probabilities
        probs = softmax(logits)

        # Cross-entropy loss for this position
        loss_t = -probs[target_id].log()
        losses.append(loss_t)

    # Average loss over sequence
    loss = (1 / n) * sum(losses)

    # --- 3. BACKWARD PASS ---

    loss.backward()  # Computes gradients for all parameters

    # --- 4. OPTIMIZER STEP ---

    # Learning rate decay (linear)
    lr_t = learning_rate * (1 - step / num_steps)

    for i, p in enumerate(params):
        # Update first moment estimate (momentum)
        m[i] = beta1 * m[i] + (1 - beta1) * p.grad

        # Update second moment estimate (variance)
        v[i] = beta2 * v[i] + (1 - beta2) * p.grad ** 2

        # Bias correction (important in early steps)
        m_hat = m[i] / (1 - beta1 ** (step + 1))
        v_hat = v[i] / (1 - beta2 ** (step + 1))

        # Adam update
        p.data -= lr_t * m_hat / (v_hat ** 0.5 + eps_adam)

        # Zero gradient for next iteration
        p.grad = 0

    # --- 5. LOGGING ---

    print(f"step {step+1:4d} / {num_steps:4d} | loss {loss.data:.4f}", end='\r')
```

### 3.2 Execution Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      Training Loop                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  step = 0                                                   │
│    │                                                         │
│    ▼                                                         │
│  ┌─────────────┐                                            │
│  │ Get Data    │───> doc = docs[0], tokens = [BOS, e, m, ..]│
│  └─────────────┘                                            │
│    │                                                         │
│    ▼                                                         │
│  ┌─────────────┐                                            │
│  │ Forward     │───> logits, probs, loss                    │
│  └─────────────┘                                            │
│    │                                                         │
│    ▼                                                         │
│  ┌─────────────┐                                            │
│  │ Backward    │───> p.grad for all params                  │
│  └─────────────┘                                            │
│    │                                                         │
│    ▼                                                         │
│  ┌─────────────┐                                            │
│  │ Adam Update │───> p.data updated, p.grad = 0             │
│  └─────────────┘                                            │
│    │                                                         │
│    ▼                                                         │
│  step = 1 ...                                               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Adam Optimizer Deep Dive

### 4.1 The Adam Algorithm Explained

**Adam = Adaptive Moment Estimation**

**Two key ideas:**
1. **Momentum:** Track running average of gradients (smooth updates)
2. **Adaptive LR:** Track running average of squared gradients (per-param LR)

### 4.2 First Moment: Momentum

```python
m[i] = beta1 * m[i] + (1 - beta1) * p.grad
```

**Intuition:**
- Think of m as "velocity" of the parameter
- Consistent gradient direction → m accumulates → faster movement
- Oscillating gradient → m cancels out → slower movement

**Example:**
```
Step 0: grad = 1.0
  m = 0.85 * 0 + 0.15 * 1.0 = 0.15

Step 1: grad = 1.0 (same direction)
  m = 0.85 * 0.15 + 0.15 * 1.0 = 0.2775 (increased!)

Step 2: grad = -1.0 (opposite direction)
  m = 0.85 * 0.2775 + 0.15 * (-1.0) = 0.0859 (decreased!)
```

### 4.3 Second Moment: Adaptive Learning Rates

```python
v[i] = beta2 * v[i] + (1 - beta2) * p.grad ** 2
```

**Intuition:**
- v tracks "variance" or magnitude of gradients
- Large gradients → large v → smaller effective LR
- Small gradients → small v → larger effective LR

**Why this helps:**
- Rare features (small gradients) get larger updates
- Frequent features (large gradients) get smaller updates
- Automatically adapts to different parameter scales

### 4.4 Bias Correction

```python
m_hat = m[i] / (1 - beta1 ** (step + 1))
v_hat = v[i] / (1 - beta2 ** (step + 1))
```

**Why needed?**

At step 0:
```
m[0] = 0.85 * 0 + 0.15 * grad = 0.15 * grad  (too small!)
```

The initialization at 0 biases m toward 0, especially in early steps.

**Bias correction fixes this:**
```
At step 0:
  correction = 1 - 0.85^1 = 0.15
  m_hat = (0.15 * grad) / 0.15 = grad  ✓

At step 1:
  correction = 1 - 0.85^2 = 0.2775
  m_hat = m / 0.2775  (less correction needed)
```

### 4.5 The Adam Update Rule

```python
p.data -= lr_t * m_hat / (v_hat ** 0.5 + eps_adam)
```

**Breaking it down:**

| Component | Purpose |
|-----------|---------|
| `lr_t` | Global learning rate (possibly scheduled) |
| `m_hat` | Direction and magnitude of update |
| `v_hat ** 0.5` | Scale adjustment per parameter |
| `eps_adam` | Prevent division by zero |

**Interpretation:**
- Move in direction of momentum
- Scale by inverse of gradient magnitude
- Large v (frequent large gradients) → smaller step
- Small v (infrequent gradients) → larger step

### 4.6 Adam Hyperparameter Sensitivity

| Hyperparameter | Typical Value | Effect if too low | Effect if too high |
|----------------|---------------|-------------------|--------------------|
| **learning_rate** | 0.001-0.01 | Slow convergence | Divergence/oscillation |
| **beta1** | 0.85-0.9 | Less momentum, noisy | Overshooting |
| **beta2** | 0.99-0.999 | Less adaptation | Too aggressive LR |
| **eps_adam** | 1e-8 | Numerical issues | No effect |

---

## 5. Learning Rate Scheduling

### 5.1 Linear Decay in microgpt

```python
lr_t = learning_rate * (1 - step / num_steps)
```

**Visualization:**
```
LR
^
|0.01 |\
|     | \
|     |  \
|     |   \
|     |    \
|0.0  |_____\
  +-----------→ step
  0         1000
```

### 5.2 Why Decay Learning Rate?

**Early training:**
- Parameters far from optimal
- Large LR helps quickly find good region
- Loss landscape is roughly convex at coarse scale

**Late training:**
- Parameters near good solution
- Small LR helps fine-tune without overshooting
- Loss landscape has fine structure

### 5.3 Common Scheduling Strategies

| Schedule | Formula | Use Case |
|----------|---------|----------|
| **Constant** | lr = lr_0 | Simple baselines |
| **Linear decay** | lr = lr_0 × (1 - t/T) | microgpt, many LLMs |
| **Cosine** | lr = lr_0 × 0.5 × (1 + cos(πt/T)) | Smooth decay |
| **Warmup + decay** | lr increases, then decreases | Large models |

---

## 6. Complete Training Trace

### 6.1 Step-by-Step Trace (First 3 Steps)

**Initial state:**
```
params = [all randomly initialized]
m = [all zeros]
v = [all zeros]
step = 0
```

**Step 0:**
```
1. Get data: doc = "emma", tokens = [BOS, e, m, m, a, BOS]
2. Forward pass:
   - pos=0: logits_0 = gpt(BOS, 0), target='e', loss_0 = -log(p['e'])
   - pos=1: logits_1 = gpt('e', 1), target='m', loss_1 = -log(p['m'])
   ...
   - loss = (loss_0 + loss_1 + ... + loss_4) / 5 = 2.7183
3. Backward pass:
   - loss.backward() computes all gradients
4. Adam update:
   - lr_t = 0.01 * (1 - 0/1000) = 0.01
   - For each param:
     m = 0.85 * 0 + 0.15 * grad = 0.15 * grad
     v = 0.99 * 0 + 0.01 * grad^2 = 0.01 * grad^2
     m_hat = m / (1 - 0.85^1) = m / 0.15 = grad
     v_hat = v / (1 - 0.99^1) = v / 0.01 = grad^2
     param -= 0.01 * grad / (sqrt(grad^2) + 1e-8)
            = 0.01 * grad / (|grad| + 1e-8)
            ≈ 0.01 * sign(grad)  (first step is sign-based!)
5. Log: step    1 / 1000 | loss 2.7183
```

**Step 1:**
```
1. Get data: doc = "olivia", tokens = [BOS, o, l, i, v, i, a, BOS]
2. Forward pass: loss = 2.6891
3. Backward pass: compute gradients
4. Adam update:
   - lr_t = 0.01 * (1 - 1/1000) = 0.00999
   - m = 0.85 * (0.15 * grad_0) + 0.15 * grad_1
   - v = 0.99 * (0.01 * grad_0^2) + 0.01 * grad_1^2
   - (bias correction with step=1)
5. Log: step    2 / 1000 | loss 2.6891
```

### 6.2 Loss Trajectory Example

```
step  1 / 1000 | loss 2.7183
step  2 / 1000 | loss 2.6891
step  3 / 1000 | loss 2.6542
step  4 / 1000 | loss 2.6103
step  5 / 1000 | loss 2.5587
...
step 10 / 1000 | loss 2.3456
step 50 / 1000 | loss 1.8234
step 100 / 1000 | loss 1.4567
step 500 / 1000 | loss 0.8234
step 1000 / 1000 | loss 0.6123
```

**Interpretation:**
- Initial rapid decrease (model learns obvious patterns)
- Slower decrease later (fine-tuning)
- Final loss depends on model capacity and dataset

---

## 7. Debugging Training Issues

### 7.1 Loss Not Decreasing

**Possible causes:**

| Cause | Symptom | Fix |
|-------|---------|-----|
| LR too low | Very slow decrease | Increase LR 10× |
| LR too high | Loss oscillates or NaN | Decrease LR 10× |
| Bad initialization | Loss stuck | Reduce init std |
| Vanishing gradients | Early layers not learning | Check LayerNorm |

### 7.2 Loss Becomes NaN

**Possible causes:**

| Cause | Fix |
|-------|-----|
| Exploding gradients | Gradient clipping |
| Division by zero | Check eps_adam |
| Log of zero | Add eps to softmax |

### 7.3 Overfitting Detection

```python
# Monitor validation loss (not just training)
val_loss = compute_validation_loss()
train_loss = current_training_loss

if val_loss - train_loss > threshold:
    print("Overfitting detected!")
    # Options: reduce model size, add dropout, more data
```

---

## Summary

1. **Training loop** cycles through: get data → forward → backward → update → repeat.

2. **Cross-entropy loss** measures prediction quality; gradients flow backward.

3. **Adam optimizer** combines momentum (smooth updates) with adaptive learning rates.

4. **Learning rate scheduling** helps with coarse-to-fine optimization.

5. **Debugging** requires monitoring loss trajectory and gradient statistics.

---

*Next: Read inference-sampling-deep-dive.md to understand how to generate text with the trained model.*
