---
title: "Inference and Sampling: Complete Deep Dive"
subtitle: "From trained weights to generated text - autoregressive decoding explained"
based_on: "microgpt/microgpt.py - Inference loop implementation"
prerequisites: "Training loop understanding, softmax and probabilities"
---

# Inference and Sampling Deep Dive

## Table of Contents

1. [Training vs. Inference](#1-training-vs-inference)
2. [Autoregressive Generation](#2-autoregressive-generation)
3. [Temperature-Controlled Sampling](#3-temperature-controlled-sampling)
4. [The Complete Inference Loop](#4-the-complete-inference-loop)
5. [KV Cache Optimization](#5-kv-cache-optimization)
6. [Sampling Strategies Comparison](#6-sampling-strategies-comparison)
7. [Debugging Generation Issues](#7-debugging-generation-issues)

---

## 1. Training vs. Inference

### 1.1 Key Differences

| Aspect | Training | Inference |
|--------|----------|-----------|
| **Goal** | Learn parameters | Generate output |
| **Input** | Full sequence with targets | Initial prompt (or BOS) |
| **Attention** | Sees all previous positions | Sees generated positions only |
| **Computation** | Forward + Backward | Forward only |
| **Output** | Loss value | Generated tokens |

### 1.2 Same Model, Different Usage

**Training:**
```python
# All tokens known, process in parallel
tokens = [BOS, 'e', 'm', 'm', 'a', BOS]
for pos in range(len(tokens) - 1):
    logits = gpt(tokens[pos], pos, keys, values)
    loss = -log(probs[tokens[pos + 1]])
```

**Inference:**
```python
# Generate one token at a time
token = BOS
for pos in range(max_length):
    logits = gpt(token, pos, keys, values)
    token = sample(logits)  # Pick next token
    output.append(token)
```

---

## 2. Autoregressive Generation

### 2.1 What Does "Autoregressive" Mean?

**Definition:** Each output depends on previous outputs.

```
y_1 = f(x)
y_2 = f(x, y_1)
y_3 = f(x, y_1, y_2)
...
```

**For text generation:**
```
token_1 = model(prompt)
token_2 = model(prompt, token_1)
token_3 = model(prompt, token_1, token_2)
...
```

### 2.2 Why Autoregressive?

**Advantage:** Natural for sequential data.

```
Generating "emma":
- Step 0: Generate 'e' (given BOS)
- Step 1: Generate 'm' (given 'e')
- Step 2: Generate 'm' (given 'm')
- Step 3: Generate 'a' (given 'm')
- Step 4: Generate BOS/end (given 'a')
```

**Each step conditions on all previous tokens.**

### 2.3 Autoregressive Attention

**During inference, attention can only see:**
1. The initial prompt (if any)
2. Previously generated tokens

**Implementation via KV cache:**
```python
keys = [[] for _ in range(n_layer)]
values = [[] for _ in range(n_layer)]

for pos in range(max_length):
    # Forward pass
    logits = gpt(token_id, pos, keys, values)

    # Inside gpt(), K and V are appended:
    # keys[li].append(k)  # Cache grows
    # values[li].append(v)

    # Attention attends to all cached positions
```

---

## 3. Temperature-Controlled Sampling

### 3.1 The Sampling Problem

**After forward pass, we have logits:**
```
logits = [2.3, 0.5, -1.2, 3.1, ...]  # One per vocabulary item
```

**Question:** How do we pick the next token?

### 3.2 Greedy Decoding (No Temperature)

```python
# Always pick highest probability
next_token = argmax(logits)
```

**Problem:** Output is repetitive and boring.

```
Greedy generation for names:
"emma" → "emma" → "emma" → ... (same output always)
```

### 3.3 Temperature Scaling

```python
# Scale logits before softmax
probs = softmax(logits / temperature)
```

**Effect of different temperatures:**

```
T = 0.1 (Very low):
  logits = [2.3, 0.5, -1.2]
  logits/T = [23, 5, -12]
  probs = [0.9999, 0.0001, 0.0000]  → Almost greedy

T = 0.5 (Low, microgpt default):
  logits/T = [4.6, 1.0, -2.4]
  probs = [0.95, 0.04, 0.01]  → Mostly highest, some variation

T = 1.0 (No scaling):
  logits/T = [2.3, 0.5, -1.2]
  probs = [0.77, 0.13, 0.03]  → True model distribution

T = 2.0 (High):
  logits/T = [1.15, 0.25, -0.6]
  probs = [0.52, 0.28, 0.09]  → More uniform, creative
```

### 3.2 Mathematical Effect

```
softmax(x/T)_i = exp(x_i/T) / Σ_j exp(x_j/T)
```

**As T → 0:**
```
softmax(x/T) → one_hot(argmax(x))  # Greedy
```

**As T → ∞:**
```
softmax(x/T) → uniform distribution  # Random
```

### 3.3 Choosing Temperature

| Task | Recommended T | Reason |
|------|---------------|--------|
| Code generation | 0.1-0.3 | Deterministic, correct output |
| Name generation | 0.5-0.7 | Some variety, plausible names |
| Story writing | 0.7-1.0 | Creative, varied |
| Brainstorming | 1.0-2.0 | Maximum diversity |

---

## 4. The Complete Inference Loop

### 4.1 microgpt Inference Implementation

```python
# === CONFIGURATION ===
temperature = 0.5       # Controls randomness
max_length = block_size # Maximum generation length
num_samples = 20        # How many names to generate

# === GENERATION LOOP ===
for sample_idx in range(num_samples):

    # Initialize KV caches (fresh for each sample)
    keys, values = [[] for _ in range(n_layer)], [[] for _ in range(n_layer)]

    # Start with BOS token
    token_id = BOS
    sample = []

    # Generate tokens autoregressively
    for pos_id in range(max_length):
        # 1. Forward pass through model
        logits = gpt(token_id, pos_id, keys, values)

        # 2. Apply temperature and convert to probabilities
        scaled_logits = [l / temperature for l in logits]
        probs = softmax(scaled_logits)

        # 3. Sample from distribution
        token_id = random.choices(
            range(vocab_size),
            weights=[p.data for p in probs]
        )[0]

        # 4. Check for end of sequence
        if token_id == BOS:
            break  # Model generated EOS

        # 5. Convert token to character
        sample.append(uchars[token_id])

    # Print generated sample
    print(f"sample {sample_idx+1:2d}: {''.join(sample)}")
```

### 4.2 Execution Trace: Generating "emma"

```
Initial: token_id = BOS, sample = [], pos_id = 0

Step 0:
  logits = gpt(BOS, 0, keys, values)  # Shape: [vocab_size]
  probs = softmax(logits / 0.5)
  sampled: token_id = 1  # Assume 'e' is token 1
  sample = ['e']
  keys, values now contain BOS info

Step 1:
  logits = gpt('e', 1, keys, values)  # Sees BOS
  probs = softmax(logits / 0.5)
  sampled: token_id = 4  # Assume 'm' is token 4
  sample = ['e', 'm']
  keys, values now contain BOS, 'e' info

Step 2:
  logits = gpt('m', 2, keys, values)  # Sees BOS, 'e'
  sampled: token_id = 4  # 'm' again
  sample = ['e', 'm', 'm']

Step 3:
  logits = gpt('m', 3, keys, values)  # Sees BOS, 'e', 'm'
  sampled: token_id = 0  # 'a'
  sample = ['e', 'm', 'm', 'a']

Step 4:
  logits = gpt('a', 4, keys, values)  # Sees BOS, 'e', 'm', 'm'
  sampled: token_id = BOS  # End of sequence!
  break

Output: "emma"
```

### 4.3 Expected Output

```
--- inference (new, hallucinated names) ---
sample  1: emma
sample  2: lialla
sample  3: vely
sample  4: olron
sample  5: aiva
sample  6: marelly
sample  7: ivy
sample  8: elee
sample  9: venla
sample 10: osly
sample 11: aely
sample 12: lilly
sample 13: vema
sample 14: arya
sample 15: elia
sample 16: milia
sample 17: eline
sample 18: ari
sample 19: vena
sample 20: oma
```

**Analysis:**
- Names are plausible (follow name-like patterns)
- Variety in output (not all the same)
- Some are real names (emma, lilly, ari)
- Some are invented but name-like (lialla, venla)

---

## 5. KV Cache Optimization

### 5.1 The Redundant Computation Problem

**Naive inference (without cache):**
```python
# At each step, recompute all previous K, V
for pos in range(max_length):
    for prev_pos in range(pos + 1):
        k = compute_key(tokens[prev_pos])  # Redundant!
        v = compute_value(tokens[prev_pos])  # Redundant!
    attention(q[pos], all_ks, all_vs)
```

**Complexity:** O(n²) forward computations

### 5.2 KV Cache Solution

```python
keys = [[] for _ in range(n_layer)]
values = [[] for _ in range(n_layer)]

for pos in range(max_length):
    # Compute K, V for current position only
    k = compute_key(token)  # New computation
    v = compute_value(token)  # New computation

    # Append to cache
    keys[layer].append(k)
    values[layer].append(v)

    # Attention uses cached K, V
    attention(q, keys[layer], values[layer])
```

**Complexity:** O(n) forward computations

### 5.3 Memory vs. Compute Trade-off

| Approach | Memory | Compute |
|----------|--------|---------|
| No cache | O(1) | O(n²) |
| KV cache | O(n) | O(n) |

**For long sequences:** KV cache is essential.

### 5.4 KV Cache in microgpt

```python
# Inside gpt() function
def gpt(token_id, pos_id, keys, values):
    ...
    for li in range(n_layer):
        # Compute K, V for current position
        k = linear(x, state_dict[f'layer{li}.attn_wk'])
        v = linear(x, state_dict[f'layer{li}.attn_wv'])

        # Append to cache (grows with sequence)
        keys[li].append(k)
        values[li].append(v)

        # Attention attends to ALL cached positions
        # This is what makes it autoregressive
        ...
```

---

## 6. Sampling Strategies Comparison

### 6.1 Overview

| Strategy | How it Works | Pros | Cons |
|----------|--------------|------|------|
| **Greedy** | argmax(logits) | Simple, deterministic | Repetitive, boring |
| **Temperature** | softmax(logits/T) | Controllable randomness | May sample unlikely tokens |
| **Top-k** | Sample from top k tokens | Avoids very unlikely tokens | k is fixed |
| **Top-p (nucleus)** | Sample from smallest set with prob ≥ p | Adaptive vocabulary size | More complex |

### 6.2 Top-k Sampling

```python
def top_k_sample(logits, k=50, temperature=1.0):
    # Get indices of top k logits
    top_indices = argsort(logits)[-k:]

    # Zero out all other logits
    filtered_logits = [logits[i] if i in top_indices else -float('inf')
                       for i in range(len(logits))]

    # Sample from filtered distribution
    probs = softmax([l / temperature for l in filtered_logits])
    return random.choices(range(len(logits)), weights=probs)
```

**Effect:** Never samples from bottom (vocab_size - k) tokens.

### 6.3 Top-p (Nucleus) Sampling

```python
def top_p_sample(logits, p=0.9, temperature=1.0):
    # Sort logits descending
    sorted_indices = argsort(logits, descending=True)
    sorted_probs = softmax([logits[i] for i in sorted_indices])

    # Find smallest set with cumulative prob >= p
    cumsum = 0
    cutoff = len(sorted_probs)
    for i, prob in enumerate(sorted_probs):
        cumsum += prob
        if cumsum >= p:
            cutoff = i + 1
            break

    # Keep only top indices
    top_indices = sorted_indices[:cutoff]

    # Sample from truncated distribution
    filtered_logits = [logits[i] if i in top_indices else -float('inf')
                       for i in range(len(logits))]
    probs = softmax([l / temperature for l in filtered_logits])
    return random.choices(range(len(logits)), weights=probs)
```

**Effect:** Vocabulary size adapts to confidence.
- Confident (peaked distribution): Small vocabulary
- Uncertain (flat distribution): Large vocabulary

### 6.4 Strategy Recommendations

| Use Case | Strategy | Parameters |
|----------|----------|------------|
| Code | Greedy | - |
| Names | Temperature | T=0.5-0.7 |
| Chat | Top-p + Temperature | p=0.9, T=0.7 |
| Creative writing | Top-k + Temperature | k=50, T=0.8-1.0 |

---

## 7. Debugging Generation Issues

### 7.1 Repetitive Output

**Symptom:** "the the the the..." or "hello hello hello..."

**Causes:**
- Temperature too low → Model is too confident
- Model is undertrained → Falls back to common patterns

**Fix:**
- Increase temperature (try 0.8, 1.0)
- Use top-p sampling (p=0.9)
- Train longer

### 7.2 Gibberish Output

**Symptom:** "xqzj klp mnbv..."

**Causes:**
- Temperature too high → Sampling random tokens
- Model not trained properly

**Fix:**
- Decrease temperature (try 0.3, 0.5)
- Check training loss decreased

### 7.3 Short Output

**Symptom:** Model always generates EOS quickly

**Causes:**
- Model learned to end sequences early
- EOS token too probable

**Fix:**
- Penalize EOS logits
- Check dataset has longer sequences
- Train with length penalties

### 7.4 Logits Not Changing

**Symptom:** Same token generated regardless of context

**Causes:**
- Model collapsed during training
- All probability mass on few tokens

**Fix:**
- Check for training divergence
- Reduce learning rate
- Check gradient flow

---

## Summary

1. **Autoregressive generation** produces tokens one at a time, conditioning on all previous tokens.

2. **Temperature scaling** controls the randomness of sampling by sharpening or flattening the probability distribution.

3. **KV caching** avoids redundant computation by storing key-value pairs from previous steps.

4. **Sampling strategies** offer different trade-offs between diversity and coherence.

5. **Debugging** requires understanding the interaction between model quality and sampling parameters.

---

*This completes the microgpt deep dive series. You now have textbook-level understanding of: autograd, transformers, training, and inference.*
