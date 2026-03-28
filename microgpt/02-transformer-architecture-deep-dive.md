---
title: "Transformer Architecture: Complete Deep Dive"
subtitle: "From attention mechanism to GPT forward pass - line by line explanation"
based_on: "microgpt/microgpt.py - GPT architecture implementation"
prerequisites: "Autograd basics, neural network fundamentals"
---

# Transformer Architecture Deep Dive

## Table of Contents

1. [The Transformer Revolution](#1-the-transformer-revolution)
2. [Embeddings: From Tokens to Vectors](#2-embeddings-from-tokens-to-vectors)
3. [Attention Is All You Need](#3-attention-is-all-you-need)
4. [Multi-Head Attention Explained](#4-multi-head-attention-explained)
5. [The Complete GPT Forward Pass](#5-the-complete-gpt-forward-pass)
6. [Why Each Component Matters](#6-why-each-component-matters)

---

## 1. The Transformer Revolution

### 1.1 Pre-Transformer Architecture Landscape

Before 2017, sequence modeling was dominated by:

| Architecture | Strength | Weakness |
|-------------|----------|----------|
| **RNN** | Natural sequence handling | Sequential computation (slow) |
| **LSTM** | Long-term dependencies | Still sequential, complex |
| **GRU** | Faster than LSTM | Still sequential |

**The fundamental bottleneck:** Processing must be sequential because position `t` depends on position `t-1`.

### 1.2 The Transformer Insight

**Paper:** "Attention Is All You Need" (Vaswani et al., 2017)

**Key insight:** Recurrence is not necessary. Use attention to connect any two positions directly.

```
RNN processing:         Transformer processing:

t=1 → t=2 → t=3 → t=4   t=1 ←──→ t=2
                        ↑       ↑
                        └──→ t=3 ←──→ t=4

All positions can be processed in parallel!
```

### 1.3 GPT: Decoder-Only Transformer

**GPT architecture choices:**
- **Decoder-only:** No encoder, generates text autoregressively
- **Causal attention:** Can only attend to previous positions
- **Pre-LayerNorm:** Normalization before each sublayer (more stable training)

---

## 2. Embeddings: From Tokens to Vectors

### 2.1 Tokenization in microgpt

**Character-level tokenization:**
```python
# Dataset: ["emma", "olivia", "alex"]
uchars = sorted(set(''.join(docs)))  # Unique characters
# Result: ['a', 'e', 'i', 'l', 'm', 'n', 'o', 'r', 'v', 'x']

BOS = len(uchars)  # Special token ID for Beginning Of Sequence
vocab_size = len(uchars) + 1  # Total vocabulary
```

**Why character-level?**
- Small vocabulary (~50-100 chars vs 50K+ for word-level)
- No unknown tokens (can generate any string)
- Slower inference (more tokens per sequence)

### 2.2 Token Embeddings

```python
# state_dict['wte'] = vocab_size × n_embd matrix
# Each row is the embedding for one token

token_id = 5  # e.g., the character 'e'
token_embedding = state_dict['wte'][token_id]  # n_embd-dimensional vector
```

**What are embeddings?**
- Learnable vectors that represent each token
- Similar tokens should have similar embeddings
- Trained end-to-end with the rest of the model

**Visualization (2D projection):**
```
     ^
     |    'a'  'e'
     |         'i'
     |    'o'
     |         'u'
     +------------->

Vowels cluster together!
```

### 2.3 Position Embeddings

**Problem:** Transformers have no notion of position (unlike RNNs).

**Solution:** Add position embeddings to token embeddings.

```python
# state_dict['wpe'] = block_size × n_embd matrix
# Each row is the embedding for one position

position_id = 3  # 4th position in sequence
position_embedding = state_dict['wpe'][position_id]

# Combine
x = [t + p for t, p in zip(token_embedding, position_embedding)]
```

**Why addition and not concatenation?**
- Keeps dimension the same (n_embd)
- Position information is "added" to token meaning
- Network learns to separate position from content in different dimensions

### 2.4 The Embedding Forward Pass in microgpt

```python
def gpt(token_id, pos_id, keys, values):
    # 1. Look up embeddings
    tok_emb = state_dict['wte'][token_id]  # Shape: [n_embd]
    pos_emb = state_dict['wpe'][pos_id]    # Shape: [n_embd]

    # 2. Combine embeddings
    x = [t + p for t, p in zip(tok_emb, pos_emb)]  # Shape: [n_embd]

    # 3. Initial normalization
    x = rmsnorm(x)

    # 4. Pass through transformer blocks
    for li in range(n_layer):
        x = transformer_block(x, li, keys, values)

    # 5. Output projection to vocabulary
    logits = linear(x, state_dict['lm_head'])
    return logits
```

---

## 3. Attention Is All You Need

### 3.1 The Attention Intuition

**Human analogy:** When reading "The animal didn't cross the street because **it** was too tired", you know "it" refers to "animal" not "street".

**Attention mechanism:** Let each token attend to (gather information from) relevant tokens.

### 3.2 Query, Key, Value: The Database Analogy

| Component | Shape | Purpose | Analogy |
|-----------|-------|---------|---------|
| **Query (Q)** | [n_embd] | "What am I looking for?" | Search query |
| **Key (K)** | [n_embd] | "What do I contain?" | Database index |
| **Value (V)** | [n_embd] | "What information do I have?" | Actual data |

**Process:**
1. Compute similarity: `score = Q · K` (dot product)
2. Convert to weights: `weights = softmax(scores)`
3. Weighted sum: `output = Σ weights × V`

### 3.3 Scaled Dot-Product Attention

```python
# For a single head
def attention(query, keys, values):
    # 1. Compute attention scores
    attn_logits = []
    for t in range(len(keys)):
        # Dot product of query with key at position t
        score = sum(q * k for q, k in zip(query, keys[t]))
        # Scale by sqrt(head_dim) for numerical stability
        score = score / (len(query) ** 0.5)
        attn_logits.append(score)

    # 2. Softmax to get attention weights
    attn_weights = softmax(attn_logits)

    # 3. Weighted sum of values
    output = []
    for j in range(len(values[0])):  # For each dimension
        weighted_sum = sum(attn_weights[t] * values[t][j]
                          for t in range(len(values)))
        output.append(weighted_sum)

    return output
```

### 3.4 Why Scale by sqrt(d)?

**Problem:** For large dimensions, dot products become very large.

```python
# If Q and K are independent with mean 0, variance 1:
# E[Q · K] = 0
# Var[Q · K] = d  (sum of d independent products)

# After scaling by 1/sqrt(d):
# Var[Q · K / sqrt(d)] = d / d = 1  ✓
```

**Consequence without scaling:**
- Large scores → softmax becomes very confident (one-hot-like)
- Gradients through softmax become tiny
- Training stalls

### 3.5 Causal (Autoregressive) Attention

**GPT constraint:** Each position can only attend to previous positions.

**Implementation in microgpt:** KV cache that grows during sequence processing.

```python
# In the forward pass, for each position:
keys[li].append(k)    # Add current K to cache
values[li].append(v)  # Add current V to cache

# Attention attends to all cached positions (0 to current)
# This enforces causality naturally
```

**Why this works:**
- At position 0: Cache has 1 item, attends to position 0 only
- At position 1: Cache has 2 items, attends to positions 0,1
- At position t: Cache has t+1 items, attends to positions 0..t

---

## 4. Multi-Head Attention Explained

### 4.1 Why Multiple Heads?

**Single head limitation:** Must compress all attention patterns into one space.

**Multiple heads:** Each head can learn different attention patterns.

```
Head 0: Attends to previous character (local patterns)
        "th" → attends to "t" when at "h"

Head 1: Attends to name start (prefix patterns)
        Any position → attends to first character

Head 2: Attends to similar characters (repetition)
        Second 'n' in "emma" → attends to first 'n'

Head 3: Attends to vowel patterns
        Vowels → attend to other vowels
```

### 4.2 Splitting Embeddings into Heads

```python
n_embd = 16   # Total embedding dimension
n_head = 4    # Number of heads
head_dim = n_embd // n_head  # = 4

# Split the embedding
for h in range(n_head):
    hs = h * head_dim  # Start index for this head
    he = hs + head_dim  # End index

    q_head = q[hs:he]  # Shape: [head_dim]
    k_h = [ki[hs:he] for ki in keys[li]]  # List of [head_dim]
    v_h = [vi[hs:he] for vi in values[li]]  # List of [head_dim]

    # Compute attention for this head
    head_output = attention(q_head, k_h, v_h)
    x_attn.extend(head_output)  # Concatenate all heads

# Final shape: [n_head × head_dim] = [n_embd]
```

### 4.3 Visualizing Multi-Head Attention

```
Input: [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]  (n_embd=16)
               │
               ▼
        Split into heads
               │
    ┌──────────┼──────────┬──────────┐
    ▼          ▼          ▼          ▼
Head 0      Head 1     Head 2     Head 3
[0,1,2,3]  [4,5,6,7]  [8,9,10,11] [12,13,14,15]
    │          │          │           │
    ▼          ▼          ▼           ▼
 Attention  Attention  Attention   Attention
    │          │          │           │
    ▼          ▼          ▼           ▼
 [o0,o1,   [o4,o5,   [o8,o9,    [o12,o13,
  o2,o3]    o6,o7]    o10,o11]   o14,o15]
    │          │          │           │
    └──────────┴──────────┴───────────┘
               │
               ▼
       Concatenate
               │
               ▼
[o0,o1,o2,o3,o4,o5,o6,o7,o8,o9,o10,o11,o12,o13,o14,o15]
```

### 4.4 Output Projection

**After multi-head attention:**
```python
# x_attn has shape [n_embd] (concatenated heads)
# Apply learned linear transformation
x = linear(x_attn, state_dict[f'layer{li}.attn_wo'])
```

**Why?** Allows the model to mix information across heads and learn complex combinations.

---

## 5. The Complete GPT Forward Pass

### 5.1 Full microgpt Implementation with Annotations

```python
def gpt(token_id, pos_id, keys, values):
    """
    GPT forward pass for a single token at a single position.

    Args:
        token_id: Integer ID of the token (0 to vocab_size-1)
        pos_id: Position in sequence (0 to block_size-1)
        keys: List of KV caches for keys, one per layer
        values: List of KV caches for values, one per layer

    Returns:
        logits: Raw scores over vocabulary (before softmax)
    """

    # === EMBEDDING LAYER ===

    # Look up token and position embeddings
    tok_emb = state_dict['wte'][token_id]  # [n_embd]
    pos_emb = state_dict['wpe'][pos_id]    # [n_embd]

    # Combine embeddings (element-wise addition)
    x = [t + p for t, p in zip(tok_emb, pos_emb)]  # [n_embd]

    # Initial layer normalization
    x = rmsnorm(x)

    # === TRANSFORMER BLOCKS ===

    for li in range(n_layer):
        # --- MULTI-HEAD ATTENTION ---

        # Residual connection (save input to block)
        x_residual = x

        # Pre-attention normalization
        x = rmsnorm(x)

        # Compute Q, K, V projections
        q = linear(x, state_dict[f'layer{li}.attn_wq'])  # [n_embd]
        k = linear(x, state_dict[f'layer{li}.attn_wk'])  # [n_embd]
        v = linear(x, state_dict[f'layer{li}.attn_wv'])  # [n_embd]

        # Add K, V to cache (for autoregressive generation)
        keys[li].append(k)
        values[li].append(v)

        # Multi-head attention
        x_attn = []
        for h in range(n_head):
            # Split into head dimensions
            hs = h * head_dim
            he = hs + head_dim

            q_h = q[hs:he]  # Query for this head
            k_h = [ki[hs:he] for ki in keys[li]]  # All cached keys for this head
            v_h = [vi[hs:he] for vi in values[li]]  # All cached values for this head

            # Compute attention scores
            attn_logits = [
                sum(q_h[j] * k_h[t][j] for j in range(head_dim)) / (head_dim ** 0.5)
                for t in range(len(k_h))
            ]

            # Softmax to get attention weights
            attn_weights = softmax(attn_logits)

            # Weighted sum of values
            head_out = [
                sum(attn_weights[t] * v_h[t][j] for t in range(len(v_h)))
                for j in range(head_dim)
            ]

            x_attn.extend(head_out)  # Concatenate heads

        # Output projection
        x = linear(x_attn, state_dict[f'layer{li}.attn_wo'])

        # Residual connection
        x = [a + b for a, b in zip(x, x_residual)]

        # --- MLP BLOCK ---

        # Residual connection
        x_residual = x

        # Pre-MLP normalization
        x = rmsnorm(x)

        # First linear layer (expands to 4× n_embd)
        x = linear(x, state_dict[f'layer{li}.mlp_fc1'])  # [4 × n_embd]

        # ReLU activation
        x = [xi.relu() for xi in x]

        # Second linear layer (projects back to n_embd)
        x = linear(x, state_dict[f'layer{li}.mlp_fc2'])  # [n_embd]

        # Residual connection
        x = [a + b for a, b in zip(x, x_residual)]

    # === OUTPUT LAYER ===

    # Project to vocabulary size (language model head)
    logits = linear(x, state_dict['lm_head'])  # [vocab_size]

    return logits
```

### 5.2 Execution Trace for a Sample Sequence

**Input:** "em" (tokens: [BOS, 'e', 'm'])

```
Position 0 (BOS):
  tok_emb = wte[BOS]
  pos_emb = wpe[0]
  x = tok_emb + pos_emb
  x = rmsnorm(x)
  q0, k0, v0 = compute...
  keys[li] = [k0]
  values[li] = [v0]
  attn attends to: [k0] → only self
  output0 = attention(q0, [k0], [v0])

Position 1 ('e'):
  tok_emb = wte['e']
  pos_emb = wpe[1]
  x = tok_emb + pos_emb
  x = rmsnorm(x)
  q1, k1, v1 = compute...
  keys[li] = [k0, k1]
  values[li] = [v0, v1]
  attn attends to: [k0, k1] → BOS and 'e'
  output1 = attention(q1, [k0,k1], [v0,v1])

Position 2 ('m'):
  tok_emb = wte['m']
  pos_emb = wpe[2]
  ...
  keys[li] = [k0, k1, k2]
  values[li] = [v0, v1, v2]
  attn attends to: [k0, k1, k2] → BOS, 'e', 'm'
  output2 = attention(q2, [k0,k1,k2], [v0,v1,v2])
```

---

## 6. Why Each Component Matters

### 6.1 Component Ablation Summary

| Remove... | Effect on Training | Effect on Final Quality |
|-----------|-------------------|------------------------|
| Residual connections | Diverges or very slow | N/A (won't converge) |
| LayerNorm | Unstable, needs lower LR | Lower quality |
| Multi-head (single head) | Slower convergence | Lower quality |
| MLP block | Much slower convergence | Significantly lower |
| Position embeddings | Can't learn order | Garbage output |

### 6.2 Residual Connections: Gradient Highways

**Without residuals:**
```
Gradient must flow through: MLP → Attn → MLP → Attn → ...
Each layer multiplies gradient → vanishing/exploding
```

**With residuals:**
```
Gradient can flow directly: x → x + sublayer(x) → x
Addition preserves gradient magnitude
```

### 6.3 LayerNorm: Training Stability

**Problem:** Without normalization, activations can grow/shrink through layers.

**RMSNorm formula:**
```python
def rmsnorm(x):
    ms = sum(xi * xi for xi in x) / len(x)  # Mean of squares
    scale = (ms + 1e-5) ** -0.5  # 1 / sqrt(ms)
    return [xi * scale for xi in x]
```

**Effect:** Keeps activations in consistent range regardless of layer depth.

### 6.4 MLP Block: Non-Linearity and Capacity

**Why expand to 4×?**
- Gives network capacity for complex transformations
- Expansion ratio is a hyperparameter (some models use 2×, 8×, etc.)

**Why ReLU?**
- Simple, effective non-linearity
- GPT-2 uses GELU, but ReLU works for small models
- ReLU: f(x) = max(0, x)

---

## Summary

1. **Embeddings** convert tokens to vectors that the network can process.

2. **Attention** allows each position to gather information from relevant positions.

3. **Multi-head** attention learns multiple complementary attention patterns.

4. **Residual connections** enable training of deep networks by providing gradient highways.

5. **LayerNorm** stabilizes training by keeping activations in consistent ranges.

6. **MLP blocks** add non-linearity and transformation capacity.

---

*Next: Read training-loop-adam-deep-dive.md to understand how to train this architecture.*
