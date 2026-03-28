---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/microgpt/microgpt.py
created_at: 2026-03-27
topic: Inference and Sampling - How GPT Generates Text
---

# Deep Dive: Inference and Sampling - From Zero to Understanding Text Generation

## Introduction: From Training to Generation

You've trained a model. Now what? How does it generate new text?

This guide covers:
1. Autoregressive generation - one token at a time
2. Temperature scaling - controlling randomness
3. Sampling strategies
4. KV cache - efficient generation
5. The complete inference loop

By the end, you'll understand exactly how GPT models produce output.

---

## Part 1: Autoregressive Generation

### What Does "Autoregressive" Mean?

**Autoregressive** = predicting the next value based on previous values.

**For text:**
```
Step 0: [BOS]
Step 1: [BOS] → predict → "E"
Step 2: [BOS, "E"] → predict → "m"
Step 3: [BOS, "E", "m"] → predict → "m"
Step 4: [BOS, "E", "m", "m"] → predict → "a"
Step 5: [BOS, "E", "m", "m", "a"] → predict → [BOS] (end)

Result: "Emma"
```

**Key property:** Each token depends only on previous tokens (causal).

### The Inference Loop

```python
# Inference: generate new text
for sample_idx in range(20):  # Generate 20 samples
    # Reset KV cache for new sequence
    keys, values = [[] for _ in range(n_layer)], [[] for _ in range(n_layer)]

    # Start with BOS token
    token_id = BOS
    sample = []

    # Generate up to block_size tokens
    for pos_id in range(block_size):
        # Forward pass through model
        logits = gpt(token_id, pos_id, keys, values)

        # Convert to probabilities
        probs = softmax([l / temperature for l in logits])

        # Sample next token
        token_id = random.choices(
            range(vocab_size),
            weights=[p.data for p in probs]
        )[0]

        # Check for end of sequence
        if token_id == BOS:
            break

        sample.append(uchars[token_id])

    print(f"sample {sample_idx+1:2d}: {''.join(sample)}")
```

### Step-by-Step Execution

Let's trace through generating "Emma":

```
Initial state:
  token_id = BOS (special beginning token)
  sample = []
  keys = [], values = []  (empty KV cache)

=== Iteration 1 ===
  pos_id = 0
  logits = gpt(BOS, 0, keys, values)  # Forward pass
  probs = softmax(logits / temperature)

  Say probs are: P('E')=0.3, P('A')=0.2, P('L')=0.1, ...
  Sample: 'E' (random choice weighted by probs)
  sample = ['E']
  keys and values now contain KV pairs for BOS

=== Iteration 2 ===
  pos_id = 1
  token_id = 'E'
  logits = gpt('E', 1, keys, values)  # Uses cached KV!
  probs = softmax(logits / temperature)

  Say probs are: P('m')=0.4, P('a')=0.2, P('i')=0.1, ...
  Sample: 'm'
  sample = ['E', 'm']

=== Iteration 3 ===
  pos_id = 2
  token_id = 'm'
  logits = gpt('m', 2, keys, values)
  probs = softmax(logits / temperature)

  Say probs are: P('m')=0.5, P('n')=0.1, P('a')=0.1, ...
  Sample: 'm'
  sample = ['E', 'm', 'm']

=== Iteration 4 ===
  pos_id = 3
  token_id = 'm'
  ...
  Sample: 'a'
  sample = ['E', 'm', 'm', 'a']

=== Iteration 5 ===
  pos_id = 4
  token_id = 'a'
  ...
  Sample: BOS (end of sequence!)
  Break out of loop

Final output: "Emma"
```

---

## Part 2: Temperature Scaling

### What is Temperature?

**Temperature** controls the randomness of predictions.

**Formula:**
```
probs = softmax(logits / temperature)
```

### How Temperature Affects Output

**Low temperature (e.g., 0.2):**
```
Logits: [2.0, 1.0, 0.5]

With T = 0.2:
  Scaled: [10.0, 5.0, 2.5]
  Probs: [0.88, 0.11, 0.01]  (very peaked, deterministic)
```

**Temperature = 1.0 (neutral):**
```
Logits: [2.0, 1.0, 0.5]

With T = 1.0:
  Scaled: [2.0, 1.0, 0.5]
  Probs: [0.58, 0.21, 0.13]  (balanced)
```

**High temperature (e.g., 2.0):**
```
Logits: [2.0, 1.0, 0.5]

With T = 2.0:
  Scaled: [1.0, 0.5, 0.25]
  Probs: [0.42, 0.26, 0.20]  (more uniform, creative)
```

### Visual Comparison

```
Temperature effect on probability distribution:

P(token)
1.0 |         T=0.2 (peaky)
    |        /
0.8 |       /
    |      /    T=1.0 (balanced)
0.6 |     /    /
    |    /    /   T=2.0 (flat)
0.4 |   /    /   /
    |  /    /   /
0.2 | /    /   /
    |/____/___/____
    token0 token1 token2
```

### When to Use Different Temperatures

| Temperature | Use Case | Example Output |
|-------------|----------|----------------|
| 0.1 - 0.3 | Deterministic, factual | "The capital of France is Paris" |
| 0.5 - 0.7 | Balanced (default for microgpt) | "Emma" (realistic names) |
| 1.0 - 2.0 | Creative, diverse | "Xylophia" (unique names) |
| > 2.0 | Very random | "qzxlm" (nonsense) |

### In microgpt

```python
temperature = 0.5  # Default: balanced

probs = softmax([l / temperature for l in logits])
```

---

## Part 3: Sampling Strategies

### Greedy Decoding (No Sampling)

**Strategy:** Always pick the most likely token.

```python
token_id = argmax(probs)  # Highest probability
```

**Pros:**
- Deterministic (reproducible)
- Often highest quality

**Cons:**
- Can get stuck in loops
- Less diverse output

### Random Sampling (Used in microgpt)

**Strategy:** Sample from the probability distribution.

```python
token_id = random.choices(range(vocab_size), weights=probs)[0]
```

**Pros:**
- Diverse output
- More creative

**Cons:**
- Non-deterministic
- Can produce low-quality sequences

### Top-K Sampling

**Strategy:** Only sample from top K most likely tokens.

```python
top_k = 50
indices = argsort(probs)[-top_k:]  # Top K indices
probs = zero_except(probs, indices)
probs = probs / sum(probs)  # Renormalize
token_id = sample(probs)
```

**Pros:**
- Filters out unlikely tokens
- Better quality than pure sampling

**Cons:**
- Hyperparameter K needs tuning

### Top-P (Nucleus) Sampling

**Strategy:** Sample from smallest set of tokens whose cumulative probability exceeds P.

```python
top_p = 0.9
sorted_indices = argsort(probs)[::-1]  # Descending
cumsum = 0
cutoff = 0
for i, idx in enumerate(sorted_indices):
    cumsum += probs[idx]
    if cumsum >= top_p:
        cutoff = i + 1
        break

probs = zero_except(probs, sorted_indices[:cutoff])
probs = probs / sum(probs)
token_id = sample(probs)
```

**Example:**
```
Tokens: [A, B, C, D, E, ...]
Probs:  [0.4, 0.3, 0.15, 0.1, 0.05, ...]

With top_p = 0.9:
  Cumsum: [0.4, 0.7, 0.85, 0.95, ...]
  Cutoff at index 3 (cumsum >= 0.9)
  Sample from: [A, B, C, D] (renormalized)
```

---

## Part 4: KV Cache - Efficient Generation

### The Problem: Redundant Computation

**Naive approach:** Recompute all previous tokens every step.

```
Step 1: forward([BOS])
Step 2: forward([BOS, E])     # Recomputes BOS!
Step 3: forward([BOS, E, m])  # Recomputes BOS, E!
Step 4: forward([BOS, E, m, m])  # Recomputes BOS, E, m!
```

**Complexity:** O(n²) for sequence length n

### The Solution: KV Cache

**Key insight:** K and V for previous tokens don't change!

```
Step 1: forward(BOS)
  → Compute K_BOS, V_BOS
  → Cache: [(K_BOS, V_BOS)]

Step 2: forward(E)
  → Compute K_E, V_E
  → Cache: [(K_BOS, V_BOS), (K_E, V_E)]
  → Attention uses all cached K, V

Step 3: forward(m)
  → Compute K_m, V_m
  → Cache: [(K_BOS, V_BOS), (K_E, V_E), (K_m, V_m)]
  → Attention uses all cached K, V
```

**Complexity:** O(n) for sequence length n

### In microgpt

```python
# Initialize empty caches
keys, values = [[] for _ in range(n_layer)], [[] for _ in range(n_layer)]

# In each forward iteration
for pos_id in range(block_size):
    logits = gpt(token_id, pos_id, keys, values)
    # ... sampling ...

# Inside gpt() function - caching happens here
def gpt(token_id, pos_id, keys, values):
    # ... embeddings ...

    for li in range(n_layer):
        # Compute Q, K, V
        q = linear(x, state_dict[f'layer{li}.attn_wq'])
        k = linear(x, state_dict[f'layer{li}.attn_wk'])
        v = linear(x, state_dict[f'layer{li}.attn_wv'])

        # Append to cache
        keys[li].append(k)
        values[li].append(v)

        # Attention uses all cached keys/values
        k_h = [ki[hs:hs+head_dim] for ki in keys[li]]
        v_h = [vi[hs:hs+head_dim] for vi in values[li]]
```

### Memory vs. Compute Tradeoff

**KV Cache:**
- **Memory:** O(n × layers × heads × dim) - stores all K, V
- **Compute:** O(n) - each step is constant time

**No Cache:**
- **Memory:** O(1) - only current step
- **Compute:** O(n²) - recomputes everything

For long sequences, caching is essential!

---

## Part 5: The Complete Inference Pipeline

### Full Inference Code (Rust-style)

```rust
pub struct Sampler {
    model: GptModel,
    temperature: f64,
    rng: ThreadRng,
}

impl Sampler {
    pub fn generate(&mut self, max_tokens: usize) -> String {
        let mut tokens = vec![BOS_TOKEN];
        let mut keys = vec![Vec::new(); self.model.config.n_layer];
        let mut values = vec![Vec::new(); self.model.config.n_layer];

        for pos in 0..max_tokens {
            // Forward pass
            let logits = self.model.forward(
                tokens[tokens.len() - 1],
                pos,
                &mut keys,
                &mut values,
            );

            // Apply temperature
            let scaled: Vec<f64> = logits
                .iter()
                .map(|l| l / self.temperature)
                .collect();

            // Softmax
            let probs = softmax(&scaled);

            // Sample next token
            let next_token = self.sample_from_probs(&probs);

            // Check for end of sequence
            if next_token == BOS_TOKEN {
                break;
            }

            tokens.push(next_token);
        }

        // Decode tokens to string
        self.tokenizer.decode(&tokens[1..])  // Skip BOS
    }

    fn sample_from_probs(&mut self, probs: &[f64]) -> usize {
        // Random sampling
        let indices: Vec<usize> = (0..probs.len()).collect();
        self.rng
            .choose_multiple_weighted(&indices, 1, |i| probs[*i])
            .next()
            .unwrap()
    }
}
```

### Example Output

After training microgpt on names dataset:

```
--- inference (new, hallucinated names) ---
sample  1: emma
sample  2: lialla
sample  3: vely
sample  4: gemma
sample  5: olivia
sample  6: lia
sample  7: noellena
sample  8: ariana
sample  9: mabella
sample 10: cora
sample 11: zella
sample 12: isabella
sample 13: lydia
sample 14: natacha
sample 15: lily
sample 16: miana
sample 17: stella
sample 18: viana
sample 19: elia
sample 20: lulla
```

**Observations:**
- Some are real names (emma, olivia, isabella, lily, stella)
- Some are variations (lialla, natacha, mabella)
- All follow name-like patterns (start with capital, end with vowel)

---

## Part 6: Common Questions

### Q: Why does generation stop?

**A:** Two conditions:
1. **EOS token:** Model generates BOS (which marks end in this implementation)
2. **Max length:** Reached block_size limit

### Q: Can we generate longer than block_size?

**A:** Not without modification. The model only learned positions 0 to block_size-1.

**Solutions:**
- **Interpolation:** Extend position embeddings
- **RoPE:** Rotary position embeddings (better extrapolation)
- **ALiBi:** Attention with linear biases (no position embeddings)

### Q: Why sometimes repeat tokens?

**A:** Model can get stuck in loops:
```
... "lilililililili" ...
```

**Causes:**
- High probability of repeating pattern
- Sampling variance

**Fixes:**
- **Repetition penalty:** Penalize tokens that appear recently
- **Beam search:** Track multiple hypotheses
- **Lower temperature:** More deterministic

### Q: How do we make generation faster?

**A:** Optimizations:
1. **KV cache** (already covered) - essential!
2. **Quantization:** Use int8 instead of f64
3. **Batching:** Generate multiple sequences in parallel
4. **Compiled models:** Use GPU/TPU acceleration

---

## Summary: Key Takeaways

1. **Autoregressive generation** predicts one token at a time, conditioning on previous tokens

2. **Temperature** controls randomness:
   - Low = deterministic
   - High = creative

3. **Sampling strategies:**
   - Greedy (highest probability)
   - Random (microgpt default)
   - Top-K / Top-P (filter unlikely tokens)

4. **KV cache** is essential for efficient generation - O(n) vs O(n²)

5. **Inference loop:**
   - Start with BOS
   - Forward pass → logits → probs → sample
   - Append to sequence
   - Repeat until EOS or max length

---

## Exercises

1. **Temperature calculation:** Given logits [3.0, 1.0, 0.5], compute probabilities for T = 0.5, 1.0, 2.0.

2. **KV cache tracing:** For a 3-token sequence, trace the contents of keys and values after each iteration.

3. **Implement top-K sampling:** Write code that samples from only the top 10 most likely tokens.

4. **Repetition penalty:** Modify the sampling code to penalize tokens that appear in the last 5 positions.

---

## Next Steps

- Read **ML First Principles** for broader context
- Implement your own inference loop in Rust
- Experiment with different sampling strategies
