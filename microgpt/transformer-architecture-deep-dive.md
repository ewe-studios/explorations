---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/microgpt/microgpt.py
created_at: 2026-03-27
topic: Transformer Architecture Deep Dive
---

# Deep Dive: Transformer Architecture - From Zero to Understanding GPT

## Introduction: What is a Transformer?

A **transformer** is a type of neural network architecture that processes sequences of data (like text) using a mechanism called **attention**.

### Why Transformers Matter

Before transformers (pre-2017), the go-to architecture for sequences was **RNNs** (Recurrent Neural Networks). But RNNs had problems:

| Problem | RNNs | Transformers |
|---------|------|--------------|
| Parallelization | Cannot parallelize (sequential) | Fully parallel |
| Long-range dependencies | Forget earlier tokens | Direct attention to any position |
| Training speed | Slow | Fast |

**Key insight of transformers:** Instead of processing tokens one by one, process them all at once and use attention to understand relationships.

### What You'll Learn

By the end of this guide, you'll understand:
1. Token embeddings - how words become vectors
2. Position embeddings - how transformers know order
3. Self-attention - the core mechanism
4. Multi-head attention - why multiple "perspectives"
5. MLP blocks - transformation layers
6. Layer normalization - training stability
7. The complete forward pass

---

## Part 1: Token and Position Embeddings

### The Problem: Computers Don't Understand Words

Neural networks only understand numbers. We need to convert text to vectors (lists of numbers).

**Simple approach: One-hot encoding**
```
Vocabulary: ["cat", "dog", "bird"]

"cat"  → [1, 0, 0]
"dog"  → [0, 1, 0]
"bird" → [0, 0, 1]
```

**Problem:** One-hot vectors don't capture similarity. "cat" and "dog" are both pets, but their vectors are orthogonal (no relationship).

### Solution: Learned Embeddings

An **embedding** is a dense vector that represents a token:

```
Vocabulary size: 50,000 words
Embedding dimension: 512

"cat"  → [0.23, -0.45, 0.12, ..., 0.89]  (512 numbers)
"dog"  → [0.21, -0.42, 0.15, ..., 0.85]  (similar to "cat"!)
"bird" → [0.10, -0.30, 0.05, ..., 0.70]
```

**Key insight:** The embedding values are **learned during training**. Similar concepts end up with similar vectors.

### In microgpt

```python
# Token embedding matrix: vocab_size × n_embd
wte = matrix(vocab_size, n_embd)

# Get embedding for token_id
tok_emb = wte[token_id]  # List of n_embd values
```

**Visualization:**
```
Vocabulary (50 tokens)
     ↓
[ wte[0] ]  → embedding for token 0  (16 dimensions)
[ wte[1] ]  → embedding for token 1
[ wte[2] ]  → embedding for token 2
...
[ wte[49]]  → embedding for token 49

Each wte[i] = [e0, e1, e2, ..., e15]  (16 values)
```

### Position Embeddings: Why Order Matters

**Problem:** Transformers process all tokens in parallel. How do they know the order?

```
"cat chased dog" vs "dog chased cat"
```

Same words, different meaning! We need to encode position information.

**Solution:** Add position embeddings to token embeddings.

```python
# Position embedding matrix: block_size × n_embd
wpe = matrix(block_size, n_embd)

# Get position embedding
pos_emb = wpe[pos_id]

# Combine: token + position
x = tok_emb + pos_emb  # Element-wise addition
```

**Why addition works:**
```
tok_emb = [t0, t1, t2, ..., t15]  (what the token is)
pos_emb = [p0, p1, p2, ..., p15]  (where the token is)
combined = [t0+p0, t1+p1, ..., t15+p15]  (what + where)
```

**Key insight:** The model learns position embeddings during training. Position 0 always gets the same embedding, position 1 always gets the same embedding, etc.

### Complete Embedding Code

```rust
pub struct Embedding {
    wte: Vec<Vec<f64>>,  // [vocab_size x n_embd]
    wpe: Vec<Vec<f64>>,  // [block_size x n_embd]
}

impl Embedding {
    pub fn forward(&self, token_id: usize, pos_id: usize) -> Vec<f64> {
        let tok_emb = &self.wte[token_id];
        let pos_emb = &self.wpe[pos_id];

        // Element-wise addition
        tok_emb.iter()
            .zip(pos_emb.iter())
            .map(|(&t, &p)| t + p)
            .collect()
    }
}
```

---

## Part 2: Self-Attention - The Core Mechanism

### The Intuition: What is Attention?

**Attention** answers the question: "When processing this token, which other tokens should I pay attention to?"

**Example:**
```
"The animal didn't cross the street because _ was tired"

To fill in the blank, what should we attend to?
- "animal" ← important! tells us what "was tired"
- "street" ← somewhat relevant
- "didn't" ← modifies the action

The model should attend more to "animal" than to "street"
```

### How Attention Works (Step by Step)

For each token, attention does:

1. **Create Query, Key, Value vectors**
   - Query: "What am I looking for?"
   - Key: "What do I contain?"
   - Value: "What information do I have?"

2. **Compute attention scores**
   - Compare my Query with all Keys
   - Higher score = more attention

3. **Weighted sum of Values**
   - Combine information from all tokens
   - Weighted by attention scores

### Mathematical Formulation

**Step 1: Linear projections**
```
For each token x:
  q = W_q @ x    (Query projection)
  k = W_k @ x    (Key projection)
  v = W_v @ x    (Value projection)
```

**Step 2: Attention scores**
```
score(i, j) = q_i · k_j / sqrt(d_k)

where:
  - i is the current position
  - j is the position we're attending to
  - d_k is the dimension of keys
  - Division by sqrt(d_k) prevents large values
```

**Step 3: Softmax to get weights**
```
attention_weights(i, j) = softmax(score(i, j))
```

**Step 4: Weighted sum**
```
output[i] = Σ_j attention_weights(i, j) × v_j
```

### Causal (Autoregressive) Attention

**Important for GPT:** We can only attend to **past** tokens, not future ones.

```
Position 0: can attend to [0]
Position 1: can attend to [0, 1]
Position 2: can attend to [0, 1, 2]
...
```

This is called **causal masking** or **autoregressive attention**.

**Why?** GPT generates text left-to-right. When predicting the next token, we shouldn't "cheat" by looking at future tokens.

### Visual Example

```
Input: "The cat sat"

Token embeddings (simplified, 4D):
  "The" → [0.1, 0.2, 0.3, 0.4]
  "cat" → [0.5, 0.6, 0.7, 0.8]
  "sat" → [0.9, 1.0, 1.1, 1.2]

After attention for "sat":
  - Attends 10% to "The"
  - Attends 30% to "cat"
  - Attends 60% to itself ("sat")

  output = 0.1 × v_The + 0.3 × v_cat + 0.6 × v_sat
```

---

## Part 3: Multi-Head Attention

### Why Multiple Heads?

**Single head limitation:** One attention mechanism might not capture all types of relationships.

**Example:** In "The cat sat on the mat because it was comfortable"

Different heads might learn:
- Head 1: Grammatical relationships (subject-verb agreement)
- Head 2: Coreference resolution ("it" → "cat" or "mat"?)
- Head 3: Semantic similarity (cat/mat are related)
- Head 4: Positional patterns (nearby words)

### How Multi-Head Works

**Split the embedding dimension:**
```
n_embd = 64
n_head = 4
head_dim = 64 / 4 = 16

Each head operates on 16 dimensions independently
```

**Process:**
```
1. Split embeddings into head_dim chunks
2. Run attention separately for each head
3. Concatenate all head outputs
4. Apply final projection
```

**Code from microgpt:**
```python
head_dim = n_embd // n_head

for h in range(n_head):
    hs = h * head_dim  # Start index for this head

    # Slice embeddings for this head
    q_h = q[hs:hs+head_dim]
    k_h = [ki[hs:hs+head_dim] for ki in keys]
    v_h = [vi[hs:hs+head_dim] for vi in values]

    # Compute attention for this head
    attn_logits = [sum(q_h[j] * k_h[t][j] for j in range(head_dim))
                   / head_dim**0.5 for t in range(len(k_h))]
    attn_weights = softmax(attn_logits)
    head_out = [sum(attn_weights[t] * v_h[t][j] for t in range(len(v_h)))
                for j in range(head_dim)]

    x_attn.extend(head_out)  # Concatenate heads
```

### Visual Representation

```
                    n_embd = 64
                    ┌────────────────────────────────┐
Input embeddings:   │ e0  e1  e2  ...              e63│
                    └────────────────────────────────┘
                          │         │         │
                          ▼         ▼         ▼
                    ┌──────────┐ ┌──────────┐ ┌──────────┐
Head 0 (dim 0-15):  │ e0...e15 │ Head 1    │ Head 2    │ Head 3    │
                    │ (attn)   │ │(e16..e31)│ │(e32..e47)│ │(e48..e63)│
                    └──────────┘ └──────────┘ └──────────┘ └──────────┘
                          │         │         │         │
                          ▼         ▼         ▼         ▼
                    ┌──────────────────────────────────────┐
Concatenated:       │ h0  h1  h2  ...                  h63 │
                    └──────────────────────────────────────┘
                          │
                          ▼
                    ┌──────────────────────────────────────┐
Output projection:  │ Wo @ concatenated                    │
                    └──────────────────────────────────────┘
```

---

## Part 4: MLP (Feed-Forward) Block

### What is the MLP Block?

After attention, each token goes through an **MLP** (Multi-Layer Perceptron) - a simple neural network.

### Architecture

```
input (n_embd)
  │
  ▼
Linear 1 (n_embd → 4×n_embd)
  │
  ▼
ReLU (activation)
  │
  ▼
Linear 2 (4×n_embd → n_embd)
  │
  ▼
output (n_embd)
```

**Why expand to 4×?** The expansion gives the model capacity to learn complex transformations. Think of it as:
1. Project to a higher-dimensional space
2. Apply non-linearity (ReLU)
3. Project back

### In Code

```python
# MLP block in microgpt
x_residual = x  # Save for residual connection
x = rmsnorm(x)  # Normalize first

# Linear 1: n_embd → 4×n_embd
x = linear(x, state_dict[f'layer{li}.mlp_fc1'])

# ReLU activation
x = [xi.relu() for xi in x]

# Linear 2: 4×n_embd → n_embd
x = linear(x, state_dict[f'layer{li}.mlp_fc2'])

# Residual connection
x = [a + b for a, b in zip(x, x_residual)]
```

### Why Residual Connections?

**Residual connection:** Add the input to the output
```
output = MLP(x) + x
```

**Benefits:**
1. **Easier training:** Gradients can flow directly through the residual
2. **Identity mapping:** If MLP learns nothing useful, it can output zeros and pass through
3. **Deeper networks:** Enables training of deeper models

---

## Part 5: RMSNorm (Layer Normalization)

### The Problem: Unstable Activations

During training, the values in neural networks can:
- Explode (become very large)
- Vanish (become very small)
- Have inconsistent scales across layers

This makes training unstable and slow.

### Solution: Normalization

**Normalization** scales values to have consistent statistics.

**LayerNorm** (original):
```
mean = average(x)
variance = average((x - mean)²)
normalized = (x - mean) / sqrt(variance + epsilon)
```

**RMSNorm** (simpler, used in microgpt):
```
RMS = sqrt(average(x²))
normalized = x / RMS
```

**Difference:** RMSNorm skips mean-centering, which saves computation with similar results.

### In Code

```python
def rmsnorm(x):
    # Compute mean of squares
    ms = sum(xi * xi for xi in x) / len(x)

    # Compute scale: 1 / sqrt(ms + epsilon)
    scale = (ms + 1e-5) ** -0.5

    # Apply scale
    return [xi * scale for xi in x]
```

**Why epsilon (1e-5)?** Prevents division by zero if RMS is 0.

### Where is RMSNorm Applied?

In microgpt (and GPT-2), RMSNorm is applied **before** each sublayer:

```
input → RMSNorm → Attention → residual add → output
input → RMSNorm → MLP → residual add → output
```

This is called **pre-norm** architecture.

---

## Part 6: The Complete Forward Pass

### Putting It All Together

Here's the complete GPT forward pass:

```python
def gpt(token_id, pos_id, keys, values):
    # 1. Token + Position Embedding
    tok_emb = state_dict['wte'][token_id]
    pos_emb = state_dict['wpe'][pos_id]
    x = [t + p for t, p in zip(tok_emb, pos_emb)]

    # 2. Initial RMSNorm
    x = rmsnorm(x)

    # 3. Transformer Blocks
    for li in range(n_layer):
        # === Attention Block ===
        x_residual = x
        x = rmsnorm(x)

        # Q, K, V projections
        q = linear(x, state_dict[f'layer{li}.attn_wq'])
        k = linear(x, state_dict[f'layer{li}.attn_wk'])
        v = linear(x, state_dict[f'layer{li}.attn_wv'])

        # Store in KV cache (for autoregressive generation)
        keys[li].append(k)
        values[li].append(v)

        # Multi-head attention
        x_attn = []
        for h in range(n_head):
            hs = h * head_dim
            q_h = q[hs:hs+head_dim]
            k_h = [ki[hs:hs+head_dim] for ki in keys[li]]
            v_h = [vi[hs:hs+head_dim] for vi in values[li]]

            # Attention scores
            attn_logits = [sum(q_h[j] * k_h[t][j] for j in range(head_dim))
                          / head_dim**0.5 for t in range(len(k_h))]
            attn_weights = softmax(attn_logits)

            # Weighted sum
            head_out = [sum(attn_weights[t] * v_h[t][j] for t in range(len(v_h)))
                       for j in range(head_dim)]
            x_attn.extend(head_out)

        # Output projection
        x = linear(x_attn, state_dict[f'layer{li}.attn_wo'])
        x = [a + b for a, b in zip(x, x_residual)]  # Residual

        # === MLP Block ===
        x_residual = x
        x = rmsnorm(x)
        x = linear(x, state_dict[f'layer{li}.mlp_fc1'])
        x = [xi.relu() for xi in x]
        x = linear(x, state_dict[f'layer{li}.mlp_fc2'])
        x = [a + b for a, b in zip(x, x_residual)]  # Residual

    # 4. Language Model Head (output logits)
    logits = linear(x, state_dict['lm_head'])
    return logits
```

### Visual Flow

```
Input tokens
     │
     ▼
┌─────────────────┐
│ Token Embedding │
│ Position Embed  │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│    RMSNorm      │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│  Transformer    │
│    Block 0      │
│  ┌───────────┐  │
│  │ Attention │  │
│  ├───────────┤  │
│  │    MLP    │  │
│  └───────────┘  │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│  Transformer    │
│    Block 1      │
│     ...         │
└─────────────────┘
     │
     ▼
┌─────────────────┐
│     LM Head     │
└─────────────────┘
     │
     ▼
Output logits (vocab_size)
```

### KV Cache for Efficient Generation

**Problem:** When generating token by token, we recompute attention for all previous tokens every time.

**Solution:** Cache the K and V vectors.

```python
# During forward pass
keys[li].append(k)    # Add current K to cache
values[li].append(v)  # Add current V to cache

# Attention uses cached K, V
k_h = [ki[hs:hs+head_dim] for ki in keys[li]]  # All previous Ks + current
v_h = [vi[hs:hs+head_dim] for vi in values[li]]  # All previous Vs + current
```

**Benefit:** O(1) per new token instead of O(n).

---

## Part 7: Understanding Dimensions

### Dimension Flow Through the Model

```
Input: token_id (scalar)
       pos_id (scalar)

After embedding: [n_embd] = [16]

After each layer: [n_embd] = [16]  (dimension preserved)

Q, K, V projections: [n_embd] → [n_embd]

Per attention head: [head_dim] = [4]

MLP intermediate: [4 × n_embd] = [64]

Output logits: [vocab_size] = [number of unique tokens]
```

### Parameter Count Breakdown

For microgpt config (vocab=65, n_embd=16, n_layer=1, n_head=4):

| Parameter | Shape | Count |
|-----------|-------|-------|
| wte | 65 × 16 | 1,040 |
| wpe | 16 × 16 | 256 |
| attn_wq | 16 × 16 | 256 |
| attn_wk | 16 × 16 | 256 |
| attn_wv | 16 × 16 | 256 |
| attn_wo | 16 × 16 | 256 |
| mlp_fc1 | 64 × 16 | 1,024 |
| mlp_fc2 | 16 × 64 | 1,024 |
| lm_head | 65 × 16 | 1,040 |
| **Total** | | **5,408** |

---

## Summary: The Transformer Architecture

1. **Embeddings** convert tokens to vectors, with position information added

2. **Self-Attention** lets each token attend to relevant previous tokens

3. **Multi-Head** attention captures different types of relationships

4. **MLP blocks** apply non-linear transformations

5. **RMSNorm** stabilizes training

6. **Residual connections** enable gradient flow

7. **KV Cache** makes generation efficient

The transformer architecture is elegant in its simplicity - just attention and MLP, repeated. Yet it's powerful enough to learn complex language patterns!

---

## Exercises

1. **Trace the forward pass:** For input "cat" at position 0, trace through each operation with actual numbers (use the microgpt initialization).

2. **Compute attention scores:** Given Q = [1, 2], K = [[3, 4], [5, 6]], compute attention scores manually.

3. **Why head_dim = n_embd / n_head?** What would happen if we used full n_embd for each head?

4. **RMSNorm calculation:** For x = [1, 2, 3, 4], compute RMSNorm(x) by hand.

---

## Next Steps

- Read **Attention Mechanism Deep Dive** for more details on attention variants
- Read **Training Loop Deep Dive** to understand how parameters are learned
- Read **Autograd Deep Dive** to understand how gradients are computed
