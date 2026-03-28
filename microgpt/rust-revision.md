---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/microgpt/microgpt.py
repository: https://github.com/karpathy/microgpt
revised_at: 2026-03-27
workspace: microgpt-rs
---

# Rust Revision: microgpt

## Overview

This document translates Karpathy's microgpt from pure Python into idiomatic Rust. The goal is to maintain the educational clarity of the original while leveraging Rust's type system, zero-cost abstractions, and performance characteristics.

### Key Design Decisions

1. **No External ML Crates**: To preserve the educational nature, we avoid PyTorch/candle/ndarray and implement everything from scratch
2. **Scalar Autograd**: Mirror the original Value class approach but with Rust's ownership
3. **Vec-based Tensors**: Use `Vec<f64>` for tensor storage with explicit indexing
4. **Temperature-controlled Sampling**: Maintain the same inference algorithm
5. **Training on Names Dataset**: Keep the same character-level training approach

## Workspace Structure

```
microgpt-rs/
├── Cargo.toml                 # Workspace root
├── microgpt-core/
│   ├── Cargo.toml            # Core GPT implementation
│   └── src/
│       ├── lib.rs            # Library root
│       ├── value.rs          # Autograd engine (Value class equivalent)
│       ├── tensor.rs         # Tensor operations
│       ├── tokenizer.rs      # Character-level tokenization
│       ├── model.rs          # GPT model architecture
│       ├── attention.rs      # Multi-head attention
│       ├── mlp.rs            # MLP block
│       ├── norm.rs           # RMSNorm
│       ├── loss.rs           # Cross-entropy loss
│       └── optim.rs          # Adam optimizer
├── microgpt-train/
│   ├── Cargo.toml            # Training binary
│   └── src/
│       └── main.rs           # Training loop
├── microgpt-infer/
│   ├── Cargo.toml            # Inference binary
│   └── src/
│       └── main.rs           # Sampling/generation
└── data/
    └── names.txt             # Training dataset
```

### Crate Breakdown

#### microgpt-core
- **Purpose:** Core GPT model, autograd, and training primitives
- **Type:** Library
- **Public API:** `Value`, `GptModel`, `AdamOptimizer`, `Tokenizer`, `train_step`, `sample`
- **Dependencies:** `rand`, `rand_distr`

#### microgpt-train
- **Purpose:** Training binary
- **Type:** Binary
- **Dependencies:** `microgpt-core`, `serde`, `serde_json`

#### microgpt-infer
- **Purpose:** Inference/sampling binary
- **Type:** Binary
- **Dependencies:** `microgpt-core`, `rand`

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Random number generation | `rand` | 0.8 | Standard RNG, needed for sampling and initialization |
| Probability distributions | `rand_distr` | 0.4 | Gaussian initialization for weights |
| Serialization | `serde + serde_json` | 1.0 | Model checkpoint save/load |
| Progress bar | `indicatif` | 0.17 | Training progress visualization |

## Type System Design

### Core Types

```rust
/// Scalar value with autograd support (equivalent to Python's Value class)
#[derive(Debug, Clone)]
pub struct Value {
    pub data: f64,
    pub grad: f64,
    children: Vec<Rc<RefCell<Value>>>,
    local_grads: Vec<f64>,
}

impl Value {
    pub fn new(data: f64) -> Self { /* ... */ }
    pub fn add(&self, other: &Value) -> Value { /* ... */ }
    pub fn mul(&self, other: &Value) -> Value { /* ... */ }
    pub fn pow(&self, exp: f64) -> Value { /* ... */ }
    pub fn log(&self) -> Value { /* ... */ }
    pub fn exp(&self) -> Value { /* ... */ }
    pub fn relu(&self) -> Value { /* ... */ }
    pub fn backward(&mut self) { /* ... */ }
}

/// Tokenizer for character-level encoding/decoding
pub struct CharTokenizer {
    vocab: HashMap<char, usize>,
    ivocab: Vec<char>,
    bos_token: usize,
}

/// GPT Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GptConfig {
    pub vocab_size: usize,
    pub block_size: usize,
    pub n_layer: usize,
    pub n_embd: usize,
    pub n_head: usize,
}

/// GPT Model with all parameters
pub struct GptModel {
    config: GptConfig,
    wte: Vec<Vec<Value>>,      // Token embeddings [vocab_size x n_embd]
    wpe: Vec<Vec<Value>>,      // Position embeddings [block_size x n_embd]
    layers: Vec<TransformerBlock>,
    lm_head: Vec<Vec<Value>>,  // [vocab_size x n_embd]
}

/// Transformer block
pub struct TransformerBlock {
    attn_wq: Vec<Vec<Value>>,  // [n_embd x n_embd]
    attn_wk: Vec<Vec<Value>>,  // [n_embd x n_embd]
    attn_wv: Vec<Vec<Value>>,  // [n_embd x n_embd]
    attn_wo: Vec<Vec<Value>>,  // [n_embd x n_embd]
    mlp_fc1: Vec<Vec<Value>>,  // [4*n_embd x n_embd]
    mlp_fc2: Vec<Vec<Value>>,  // [n_embd x 4*n_embd]
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MicroGptError {
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Invalid token ID: {id} is out of range [0, {vocab_size})")]
    InvalidTokenId { id: usize, vocab_size: usize },

    #[error("Sequence too long: {length} exceeds block_size {max}")]
    SequenceTooLong { length: usize, max: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, MicroGptError>;
```

### Traits

```rust
/// Forward pass through a layer/module
pub trait Forward {
    type Input;
    type Output;

    fn forward(&self, input: &Self::Input) -> Self::Output;
}

/// Optimizer trait
pub trait Optimizer {
    fn step(&mut self, params: &mut [Value], learning_rate: f64);
    fn zero_grad(&mut self, params: &mut [Value]);
}
```

## Key Rust-Specific Changes

### 1. Ownership and Borrowing for Autograd

**Source Pattern (Python):**
```python
class Value:
    def __init__(self, data, children=(), local_grads=()):
        self.data = data
        self.grad = 0
        self._children = children
        self._local_grads = local_grads
```

**Rust Translation:**
```rust
use std::{rc::Rc, cell::RefCell};

pub struct Value {
    pub data: f64,
    pub grad: f64,
    children: Vec<Rc<RefCell<Value>>>,
    local_grads: Vec<f64>,
}
```

**Rationale:** Python uses garbage collection, so circular references are fine. Rust needs `Rc<RefCell<T>>` for shared ownership and interior mutability in the computation graph.

### 2. Tensor Operations with Vec

**Source Pattern:**
```python
def linear(x, w):
    return [sum(wi * xi for wi, xi in zip(wo, x)) for wo in w]
```

**Rust Translation:**
```rust
pub fn linear(x: &[Value], w: &[Vec<Value>]) -> Vec<Value> {
    w.iter()
        .map(|row| row.iter().zip(x.iter())
            .map(|(wi, xi)| wi * xi)
            .fold(Value::new(0.0), |acc, prod| &acc + &prod))
        .collect()
}
```

**Rationale:** Iterators are zero-cost abstractions in Rust. Using `&[T]` slices avoids unnecessary cloning while maintaining clear ownership.

### 3. Explicit Type Annotations for Clarity

**Source Pattern:**
```python
n_layer = 1     # depth of the transformer
n_embd = 16     # embedding dimension
```

**Rust Translation:**
```rust
pub struct GptConfig {
    pub n_layer: usize,
    pub n_embd: usize,
    pub n_head: usize,
    pub block_size: usize,
    pub vocab_size: usize,
}
```

**Rationale:** Rust's type system makes the interface explicit. `usize` is appropriate for dimensions and sizes.

### 4. Error Handling with Result

**Source Pattern:**
```python
# Python implicitly handles errors
token_id = chars.index(ch)  # Raises ValueError if not found
```

**Rust Translation:**
```rust
match tokenizer.encode(token_str) {
    Ok(tokens) => { /* use tokens */ },
    Err(MicroGptError::UnknownCharacter { ch }) => { /* handle */ },
}
```

**Rationale:** Rust forces explicit error handling, making edge cases visible at compile time.

## Ownership & Borrowing Strategy

```rust
// Computation graph uses shared ownership
type ComputeGraph = Rc<RefCell<Value>>;

// Model weights are owned by the model, borrowed during forward pass
pub struct GptModel {
    weights: Parameters,  // Owned
}

impl GptModel {
    // Borrow weights for forward pass
    pub fn forward(&self, tokens: &[usize]) -> Vec<Value> {
        // &self means we can run multiple forward passes concurrently
    }

    // Mutable borrow for training
    pub fn parameters_mut(&mut self) -> &mut [Value] {
        // Exclusive access for gradient updates
    }
}

// Training loop pattern
fn train_step(model: &mut GptModel, optimizer: &mut Adam, batch: &[usize]) -> f64 {
    let loss = model.forward(batch);
    loss.backward();
    optimizer.step(model.parameters_mut());
    optimizer.zero_grad(model.parameters_mut());
    loss.data
}
```

## Concurrency Model

**Approach:** Single-threaded (matching original Python)

**Rationale:**
- Educational clarity over performance
- Autograd graph with `Rc<RefCell<T>>` doesn't support `Send`
- For actual training, would use a framework like PyTorch/candle

```rust
// Future: Could parallelize batch processing with rayon
use rayon::prelude::*;

fn forward_batch(&self, batch: &[Vec<usize>]) -> Vec<f64> {
    batch.par_iter()
        .map(|tokens| self.forward(tokens).data)
        .collect()
}
```

## Memory Considerations

- **Stack vs. Heap:** `Value` structs on heap via `Rc` due to graph structure
- **No `Box` needed:** `Rc` handles heap allocation
- **No `Arc` needed:** Single-threaded, `Rc` is sufficient
- **No unsafe code:** Pure safe Rust implementation

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Division by zero in softmax | Runtime check with `panic!` or `Result` |
| Out-of-bounds token access | `Option`/`Result` return types |
| Sequence exceeds block_size | Validation in `forward()` with error |
| NaN in gradients | `f64::is_nan()` checks in backward pass |
| Empty vocabulary | Type system ensures non-zero via `NonZeroUsize` |

## Code Examples

### Example: Value Autograd Class

```rust
use std::{rc::Rc, cell::RefCell, collections::HashSet};

#[derive(Debug, Clone)]
pub struct Value {
    pub data: f64,
    pub grad: f64,
    children: Vec<Rc<RefCell<Value>>>,
    local_grads: Vec<f64>,
}

impl Value {
    pub fn new(data: f64) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            data,
            grad: 0.0,
            children: Vec::new(),
            local_grads: Vec::new(),
        }))
    }

    pub fn add(a: &Rc<RefCell<Self>>, b: &Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        let a_val = a.borrow().data;
        let b_val = b.borrow().data;
        let result = Self::new(a_val + b_val);

        result.borrow_mut().children = vec![Rc::clone(a), Rc::clone(b)];
        result.borrow_mut().local_grads = vec![1.0, 1.0];

        result
    }

    pub fn mul(a: &Rc<RefCell<Self>>, b: &Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        let a_val = a.borrow().data;
        let b_val = b.borrow().data;
        let result = Self::new(a_val * b_val);

        result.borrow_mut().children = vec![Rc::clone(a), Rc::clone(b)];
        result.borrow_mut().local_grads = vec![b_val, a_val];

        result
    }

    pub fn backward(&self) {
        // Topological sort
        let mut topo = Vec::new();
        let mut visited = HashSet::new();

        fn build_topo(v: &Rc<RefCell<Value>>, topo: &mut Vec<Rc<RefCell<Value>>>, visited: &mut HashSet<usize>) {
            let id = Rc::as_ptr(v) as usize;
            if !visited.contains(&id) {
                visited.insert(id);
                for child in &v.borrow().children {
                    build_topo(child, topo, visited);
                }
                topo.push(Rc::clone(v));
            }
        }

        build_topo(&Rc::new(RefCell::new(self.clone())), &mut topo, &mut visited);

        // Set gradient of output to 1
        self.grad = 1.0;

        // Backpropagate
        for v in topo.iter().rev() {
            let grad = v.borrow().grad;
            let children = v.borrow().children.clone();
            let local_grads = v.borrow().local_grads.clone();

            for (child, local_grad) in children.iter().zip(local_grads.iter()) {
                child.borrow_mut().grad += local_grad * grad;
            }
        }
    }
}
```

### Example: RMSNorm

```rust
/// RMSNorm: Root Mean Square Layer Normalization
/// Simpler than LayerNorm - no learnable parameters
pub fn rmsnorm(x: &[Rc<RefCell<Value>>]) -> Vec<Rc<RefCell<Value>>> {
    let n = x.len() as f64;

    // Compute mean of squares: sum(x^2) / n
    let ms = x.iter()
        .map(|xi| {
            let val = xi.borrow().data;
            Value::mul(xi, &Value::new(val))
        })
        .fold(Value::new(0.0), |acc, sq| Value::add(&acc, &sq));

    let ms = Value::div(&ms, &Value::new(n));

    // Compute scale: 1 / sqrt(ms + eps)
    let scale = Value::div(
        &Value::new(1.0),
        &Value::sqrt(&Value::add(&ms, &Value::new(1e-5)))
    );

    // Apply scale to each element
    x.iter()
        .map(|xi| Value::mul(xi, &scale))
        .collect()
}
```

### Example: Multi-Head Attention

```rust
pub fn multi_head_attention(
    q: &[Rc<RefCell<Value>>],
    k_cache: &[Vec<Rc<RefCell<Value>>>],
    v_cache: &[Vec<Rc<RefCell<Value>>>],
    n_head: usize,
    head_dim: usize,
) -> Vec<Rc<RefCell<Value>>> {
    let mut output = Vec::with_capacity(q.len());

    for h in 0..n_head {
        let hs = h * head_dim;
        let he = hs + head_dim;

        // Split Q into this head
        let q_head: Vec<_> = q[hs..he].to_vec();

        // Get K, V from cache for this head
        let k_head: Vec<Vec<_>> = k_cache.iter()
            .map(|k| k[hs..he].to_vec())
            .collect();
        let v_head: Vec<Vec<_>> = v_cache.iter()
            .map(|v| v[hs..he].to_vec())
            .collect();

        // Compute attention logits: Q @ K.T / sqrt(head_dim)
        let scale = (head_dim as f64).sqrt();
        let logits: Vec<_> = k_head.iter()
            .map(|k| {
                q_head.iter().zip(k.iter())
                    .map(|(q, k)| Value::mul(q, k))
                    .fold(Value::new(0.0), |acc, prod| Value::add(&acc, &prod))
            })
            .map(|logit| Value::div(&logit, &Value::new(scale)))
            .collect();

        // Softmax
        let weights = softmax(&logits);

        // Weighted sum of values
        let head_out: Vec<_> = (0..head_dim)
            .map(|j| {
                v_head.iter().zip(weights.iter())
                    .map(|(v, w)| Value::mul(&v[j], w))
                    .fold(Value::new(0.0), |acc, weighted| Value::add(&acc, &weighted))
            })
            .collect();

        output.extend(head_out);
    }

    output
}

fn softmax(logits: &[Rc<RefCell<Value>>]) -> Vec<Rc<RefCell<Value>>> {
    let max_val = logits.iter()
        .map(|v| v.borrow().data)
        .fold(f64::NEG_INFINITY, f64::max);

    let exps: Vec<_> = logits.iter()
        .map(|v| {
            let shifted = Value::sub(v, &Value::new(max_val));
            shifted.exp()
        })
        .collect();

    let total = exps.iter()
        .fold(Value::new(0.0), |acc, e| Value::add(&acc, e));

    exps.iter()
        .map(|e| Value::div(e, &total))
        .collect()
}
```

## Migration Path

For someone wanting to migrate from the Python version:

1. **Start with Value class:** Implement autograd engine first
2. **Add tensor ops:** Linear, softmax, rmsnorm as standalone functions
3. **Build model architecture:** Compose functions into GPT forward pass
4. **Add training loop:** Port Adam optimizer and training logic
5. **Implement inference:** Add sampling with temperature
6. **Add serialization:** Save/load model checkpoints

## Performance Considerations

| Aspect | microgpt-rs | Python microgpt | Notes |
|--------|-------------|-----------------|-------|
| Scalar ops | ~10-100x faster | Baseline | Rust compilation optimizes |
| Memory | Lower overhead | GC overhead | No garbage collector |
| Concurrency | Possible with redesign | GIL limited | Could use rayon for batches |
| SIMD | Compiler auto-vectorizes | Limited | LLVM optimizations |

Expected speedup: **50-100x** for training due to:
- Compiled vs. interpreted execution
- No GIL contention
- Better cache utilization
- LLVM optimizations

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_add() {
        let a = Value::new(3.0);
        let b = Value::new(4.0);
        let c = Value::add(&a, &b);

        assert_eq!(c.borrow().data, 7.0);
    }

    #[test]
    fn test_value_backward() {
        let a = Value::new(2.0);
        let b = Value::add(&a, &a);

        b.borrow_mut().grad = 1.0;
        b.borrow().backward();

        assert_eq!(a.borrow().grad, 2.0);
    }

    #[test]
    fn test_softmax_sum_to_one() {
        let logits = vec![
            Value::new(1.0),
            Value::new(2.0),
            Value::new(3.0),
        ];

        let probs = softmax(&logits);
        let sum: f64 = probs.iter().map(|p| p.borrow().data).sum();

        assert!((sum - 1.0).abs() < 1e-6);
    }
}
```

## Open Considerations

1. **Memory Efficiency:** `Rc<RefCell<T>>` has overhead. Consider arena allocation for production.

2. **Batch Processing:** Current design is single-sample. Adding batching would improve throughput.

3. **GPU Support:** Would require migration to a framework like candle (Hugging Face) or burn.

4. **Mixed Precision:** Could add f16 support for faster training on modern hardware.

---

## From Zero to ML Engineer: First Principles

### What is a Neural Network?

A neural network is a mathematical function that:
1. Takes input (e.g., text tokens)
2. Transforms it through layers of computation
3. Produces output (e.g., predictions of next token)

The "learning" happens by adjusting internal parameters to minimize prediction errors.

### Why Autograd?

Training neural networks requires computing gradients - how much each parameter contributes to the error. Autograd automatically tracks these gradients using the chain rule of calculus.

```
Chain Rule: d(f(g(x)))/dx = d(f)/dg * d(g)/dx
```

Our `Value` class builds a computation graph during forward pass, then walks it backwards applying the chain rule.

### How Does Attention Work?

Attention answers: "Given the current position, which previous positions should I pay attention to?"

1. **Query (Q):** "What am I looking for?"
2. **Key (K):** "What do I contain?"
3. **Value (V):** "What information do I have?"

Attention = softmax(Q @ K.T / sqrt(d)) @ V

### What is RMSNorm?

Layer normalization stabilizes training by keeping activations in a consistent range. RMSNorm is a simpler variant:

```
RMSNorm(x) = x / sqrt(mean(x^2) + epsilon)
```

No learnable parameters, just normalization.

### How Does Adam Optimize?

Adam combines:
- **Momentum (m):** Remember past gradients for smoother updates
- **Adaptive learning rates (v):** Scale updates per-parameter

```
m = beta1 * m + (1 - beta1) * grad
v = beta2 * v + (1 - beta2) * grad^2
param -= lr * m / (sqrt(v) + epsilon)
```
