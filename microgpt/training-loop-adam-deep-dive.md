---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/microgpt/microgpt.py
created_at: 2026-03-27
topic: Training Loop and Adam Optimizer Deep Dive
---

# Deep Dive: Training Loop and Adam Optimizer - From Zero to Understanding How Models Learn

## Introduction: How Do Neural Networks Actually Learn?

You've seen the model architecture (transformer). You've seen how it makes predictions (forward pass). But how does it **improve** over time?

This guide explains:
1. What loss functions are
2. How gradient descent works
3. The Adam optimizer step-by-step
4. The complete training loop
5. Why learning rate decay matters

By the end, you'll understand exactly how microgpt transforms from random weights to generating coherent names.

---

## Part 1: The Loss Function - Measuring Wrongness

### What is Loss?

**Loss** is a number that measures how wrong the model's predictions are.

- **High loss** = predictions are bad
- **Low loss** = predictions are good
- **Goal of training:** Minimize loss

### Cross-Entropy Loss (Used in microgpt)

For language modeling, we use **cross-entropy loss**.

**Intuition:** The model outputs probabilities over all possible next tokens. Cross-entropy measures how "surprised" the model is by the actual next token.

**Formula:**
```
loss = -log(p[target_token])
```

**Example:**
```
Vocabulary: ["cat", "dog", "bird", "fish"]
Target (actual next word): "cat"

Model A predicts:
  P(cat) = 0.7, P(dog) = 0.2, P(bird) = 0.05, P(fish) = 0.05
  loss = -log(0.7) = 0.357  (low loss, good!)

Model B predicts:
  P(cat) = 0.1, P(dog) = 0.3, P(bird) = 0.4, P(fish) = 0.2
  loss = -log(0.1) = 2.303  (high loss, bad!)
```

**Why log?**
- Penalizes confident wrong predictions heavily
- Mathematically convenient (plays nice with softmax)

### In microgpt

```python
# Forward pass through model
logits = gpt(token_id, pos_id, keys, values)

# Softmax to get probabilities
probs = softmax(logits)

# Cross-entropy loss for target token
loss_t = -probs[target_id].log()
```

### Average Loss Over Sequence

For a sequence of tokens, we average the loss:

```python
losses = []
for pos_id in range(n):
    token_id, target_id = tokens[pos_id], tokens[pos_id + 1]
    logits = gpt(token_id, pos_id, keys, values)
    probs = softmax(logits)
    loss_t = -probs[target_id].log()
    losses.append(loss_t)

# Average loss over the sequence
loss = (1 / n) * sum(losses)
```

**Why average?**
- Makes loss comparable across sequences of different lengths
- Gradient magnitudes stay consistent

---

## Part 2: Gradient Descent - The Core Idea

### The Intuition

Imagine you're on a mountain, blindfolded, and want to reach the valley.

**Strategy:** Feel the slope under your feet, and step downhill.

**Gradient descent is the same:**
- **Mountain** = loss function (high loss = high elevation)
- **Your position** = current parameter values
- **Slope** = gradient (derivative of loss w.r.t. parameters)
- **Stepping downhill** = updating parameters in opposite direction of gradient

### The Update Rule

```
new_param = old_param - learning_rate × gradient
```

**Breaking it down:**
- `gradient` = direction of steepest increase
- `-gradient` = direction of steepest decrease
- `learning_rate` = how big a step to take

**Visual:**
```
Loss landscape (simplified, 1 parameter):

     loss
      ^
      |    *
      |   / \
      |  /   \
      | /     \
      |/       \____
      +-----------------> param
         ^
         current position
         gradient points right (uphill)
         we move left (downhill)
```

### In Code (Simple Gradient Descent)

```python
learning_rate = 0.01

for param in params:
    param.data -= learning_rate * param.grad
    param.grad = 0  # Reset for next iteration
```

---

## Part 3: Adam Optimizer - Gradient Descent on Steroids

### Why Adam?

Simple gradient descent has problems:

1. **Fixed learning rate:** Same step size for all parameters
2. **Noisy gradients:** Can oscillate in rough terrain
3. **No memory:** Each step independent

**Adam** (Adaptive Moment Estimation) fixes these issues with:
1. **Momentum:** Remember past gradients for smoother updates
2. **Adaptive learning rates:** Each parameter gets its own step size

### Adam's Two Moments

**First Moment (m):** Exponential moving average of gradients
```
m = beta1 × m + (1 - beta1) × gradient
```

**Intuition:** If gradients consistently point the same way, build up speed in that direction (like a ball rolling downhill).

**Second Moment (v):** Exponential moving average of squared gradients
```
v = beta2 × v + (1 - beta2) × gradient²
```

**Intuition:** If gradients are large (steep slope), slow down. If gradients are small (gentle slope), speed up.

### Adam Update Rule

```python
# Update biased first moment estimate
m[i] = beta1 * m[i] + (1 - beta1) * p.grad

# Update biased second moment estimate
v[i] = beta2 * v[i] + (1 - beta2) * p.grad ** 2

# Bias correction (moments are initialized at 0)
m_hat = m[i] / (1 - beta1 ** (step + 1))
v_hat = v[i] / (1 - beta2 ** (step + 1))

# Parameter update
p.data -= lr_t * m_hat / (v_hat ** 0.5 + eps_adam)
```

### Why Bias Correction?

**Problem:** Moments are initialized at 0, so early estimates are biased toward 0.

**Example:**
```
Step 0: m = 0 (initialized)
Step 1: m = 0.85 × 0 + 0.15 × grad = 0.15 × grad  (way too small!)
Step 2: m = 0.85 × (0.15 × grad) + 0.15 × grad = 0.2775 × grad
...
Step 100: m ≈ actual moving average
```

**Bias correction fixes this:**
```
m_hat = m / (1 - beta1 ** step)

Step 1: m_hat = (0.15 × grad) / (1 - 0.85) = grad  (correct!)
Step 2: m_hat = (0.2775 × grad) / (1 - 0.85²) ≈ correct
```

### Adam Hyperparameters in microgpt

```python
learning_rate = 0.01   # Base step size
beta1 = 0.85           # Momentum decay (typical: 0.9)
beta2 = 0.99           # Second moment decay (typical: 0.999)
eps_adam = 1e-8        # Numerical stability
```

**Note:** microgpt uses `beta1 = 0.85` instead of the typical `0.9`. This gives less momentum, which can help with the noisy gradients of small-batch training.

---

## Part 4: Learning Rate Decay

### Why Decay the Learning Rate?

**Early training:** Large updates needed to find the right region
**Late training:** Small updates needed to fine-tune

**Analogy:**
- Driving to a city: highway (fast) → local streets (slow) → parking spot (very slow)

### Linear Decay in microgpt

```python
lr_t = learning_rate * (1 - step / num_steps)
```

**Visualization:**
```
learning_rate
     ^
0.01 |*
     | \
     |  \
     |   \
     |    \
0.00 |_____\___> step
     0     1000
```

**At step 0:** lr = 0.01 × (1 - 0) = 0.01
**At step 500:** lr = 0.01 × (1 - 0.5) = 0.005
**At step 1000:** lr = 0.01 × (1 - 1) = 0.0

**Why linear?** Simple and effective. Other options: cosine decay, exponential decay.

---

## Part 5: The Complete Training Loop

### Putting It All Together

```python
# Initialize buffers for Adam
m = [0.0] * len(params)  # First moment
v = [0.0] * len(params)  # Second moment

# Training loop
for step in range(num_steps):
    # === 1. Get a training sample ===
    doc = docs[step % len(docs)]  # Cycle through dataset
    tokens = [BOS] + [uchars.index(ch) for ch in doc] + [BOS]
    n = min(block_size, len(tokens) - 1)

    # === 2. Forward pass ===
    keys, values = [[] for _ in range(n_layer)], [[] for _ in range(n_layer)]
    losses = []
    for pos_id in range(n):
        token_id, target_id = tokens[pos_id], tokens[pos_id + 1]
        logits = gpt(token_id, pos_id, keys, values)
        probs = softmax(logits)
        loss_t = -probs[target_id].log()
        losses.append(loss_t)
    loss = (1 / n) * sum(losses)

    # === 3. Backward pass (compute gradients) ===
    loss.backward()

    # === 4. Adam optimizer update ===
    lr_t = learning_rate * (1 - step / num_steps)  # LR decay
    for i, p in enumerate(params):
        # Update moments
        m[i] = beta1 * m[i] + (1 - beta1) * p.grad
        v[i] = beta2 * v[i] + (1 - beta2) * p.grad ** 2

        # Bias correction
        m_hat = m[i] / (1 - beta1 ** (step + 1))
        v_hat = v[i] / (1 - beta2 ** (step + 1))

        # Update parameter
        p.data -= lr_t * m_hat / (v_hat ** 0.5 + eps_adam)

        # Reset gradient for next iteration
        p.grad = 0

    # === 5. Logging ===
    print(f"step {step+1:4d} / {num_steps:4d} | loss {loss.data:.4f}", end='\r')
```

### Training Loop Visualization

```
┌─────────────────────────────────────────────────────┐
│                   Training Loop                      │
├─────────────────────────────────────────────────────┤
│  1. Sample batch from dataset                       │
│     └─> tokens = [BOS, 'e', 'm', 'm', 'a', BOS]     │
│                                                      │
│  2. Forward pass                                    │
│     └─> loss = 2.718                                 │
│                                                      │
│  3. Backward pass (loss.backward())                 │
│     └─> All param.grad values populated             │
│                                                      │
│  4. Adam update                                     │
│     ├─> Update m (first moment)                     │
│     ├─> Update v (second moment)                    │
│     ├─> Bias correction                             │
│     └─> Update params                               │
│                                                      │
│  5. Zero gradients                                  │
│     └─> Ready for next iteration                    │
└─────────────────────────────────────────────────────┘
```

### Expected Training Progress

For microgpt on names dataset (1000 steps):

```
step    1 / 1000 | loss 2.7183  (random guessing)
step   50 / 1000 | loss 1.8432  (learning patterns)
step  100 / 1000 | loss 1.2156  (coherent names)
step  500 / 1000 | loss 0.5421  (realistic names)
step 1000 / 1000 | loss 0.3102  (memorized patterns)
```

**What's happening:**
- Early: Model learns character-level patterns (vowels, consonants)
- Middle: Learns name structure (syllables, common endings)
- Late: Memorizes specific name patterns from dataset

---

## Part 6: Understanding Gradient Flow

### The Full Gradient Path

Let's trace how a gradient flows backward through the model:

```
loss = -log(p[target])
  │
  └─> d(loss)/d(p[target]) = -1/p[target]
       │
       └─> d(p[target])/d(logits) = softmax derivative
            │
            └─> d(logits)/d(lm_head) = x (activations from last layer)
                 │
                 └─> d(x)/d(layer_n_output) = ...
                      │
                      └─> (continues backward through all layers)
```

### Gradient Through Attention

```
attention_output = attention_weights @ values
  │
  ├─> d(output)/d(attention_weights) = values
  │    │
  │    └─> d(attention_weights)/d(softmax_input) = softmax derivative
  │         │
  │         └─> d(softmax_input)/d(Q, K) = Q, K gradients
  │
  └─> d(output)/d(values) = attention_weights
```

### Why Gradients Can Vanish or Explode

**Problem:** Gradients are products of many terms (chain rule).

```
gradient = term1 × term2 × term3 × ... × termN
```

- If each term < 1: gradient → 0 (vanishing)
- If each term > 1: gradient → ∞ (exploding)

**Solutions in microgpt:**
1. **RMSNorm:** Keeps activations in reasonable range
2. **Residual connections:** Provides gradient "highways"
3. **Learning rate decay:** Prevents large updates late in training

---

## Part 7: Common Questions

### Q: Why cycle through the dataset with `step % len(docs)`?

**A:** This is **online learning** style - we use one sample at a time, cycling through the dataset.

**Alternatives:**
- **Batch training:** Multiple samples per step (better gradient estimates)
- **Full batch:** All samples per step (expensive, but most stable)
- **Epochs:** Complete passes through dataset (more common convention)

### Q: Why reset gradients with `p.grad = 0`?

**A:** Gradients accumulate by design! This allows:
- **Gradient accumulation:** Simulate larger batches
- **Multiple losses:** Add gradients from different objectives

But for simple training, we zero after each step.

### Q: What if learning rate is too high?

**A:** Training becomes unstable:
```
step    1 | loss 2.71
step    2 | loss 5.43  (increasing!)
step    3 | loss 12.81
step    4 | loss inf   (exploded!)
```

**Solution:** Lower learning rate, use gradient clipping.

### Q: What if learning rate is too low?

**A:** Training is very slow:
```
step    1 | loss 2.71
step  100 | loss 2.69  (barely improving!)
step  500 | loss 2.65
```

**Solution:** Increase learning rate, use learning rate warmup.

---

## Summary: Key Takeaways

1. **Loss** measures prediction quality (cross-entropy for language)

2. **Gradient descent** updates parameters to minimize loss:
   ```
   param = param - lr × gradient
   ```

3. **Adam** improves gradient descent with:
   - Momentum (first moment) for smoother updates
   - Adaptive learning rates (second moment)

4. **Bias correction** fixes zero-initialized moments

5. **Learning rate decay** helps convergence

6. **Training loop:**
   - Sample data
   - Forward pass → loss
   - Backward pass → gradients
   - Optimizer step → update params
   - Zero gradients

---

## Exercises

1. **Manual Adam step:** Given param = 1.0, grad = 0.5, m = 0.1, v = 0.01, compute the new parameter value.

2. **Learning rate schedule:** Plot the learning rate over 1000 steps with lr = 0.01 and linear decay.

3. **Loss interpretation:** If loss = 1.0, what is P(target)? (Answer: e^(-1) ≈ 0.37)

4. **Gradient flow:** Trace the gradient from loss back to wte (token embeddings) through the computation graph.

---

## Next Steps

- Read **Inference and Sampling Deep Dive** to understand how trained models generate text
- Read **ML First Principles** for broader context on machine learning
- Implement your own training loop in Rust based on this understanding
