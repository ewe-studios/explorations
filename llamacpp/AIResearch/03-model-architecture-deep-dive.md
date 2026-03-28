---
title: "Model Architecture Deep Dive: LLaMA, Mistral, and MoE Models"
subtitle: "Complete guide to transformer architectures supported by llama.cpp"
based_on: "llama.cpp src/llama-model.cpp, src/llama-arch.cpp"
level: "Intermediate - Requires inference optimization knowledge"
---

# Model Architecture Deep Dive

## Table of Contents

1. [LLM Architecture Overview](#1-llm-architecture-overview)
2. [LLaMA Architecture](#2-llama-architecture)
3. [Mistral and Mixtral MoE](#3-mistral-and-mixtral-moe)
4. [Gemma Architecture](#4-gemma-architecture)
5. [Qwen Architecture](#5-qwen-architecture)
6. [Attention Mechanisms](#6-attention-mechanisms)
7. [Feed-Forward Networks](#7-feed-forward-networks)
8. [Position Encodings](#8-position-encodings)
9. [Rust Translation Patterns](#9-rust-translation-patterns)

---

## 1. LLM Architecture Overview

### 1.1 The Transformer Family Tree

```
                                    Transformer (2017)
                                         │
              ┌──────────────────────────┼──────────────────────────┐
              │                          │                          │
              ▼                          ▼                          ▼
        Encoder-Decoder            Decoder-Only              Encoder-Only
        (T5, BART)                 (GPT, LLaMA)              (BERT)
                                         │
              ┌──────────────────────────┼──────────────────────────┐
              │                          │                          │
              ▼                          ▼                          ▼
         GPT-2/3/4                  LLaMA Family              Claude/PaLM
              │                          │
              │              ┌───────────┼───────────┐
              │              │           │           │
              ▼              ▼           ▼           ▼
         Code Llama    LLaMA 2    LLaMA 3    LLaMA 3.2
```

### 1.2 Common Architecture Parameters

```c
// llama.cpp architecture parameters
struct llama_hparams {
    uint32_t n_vocab;           // Vocabulary size (32000, 128256, etc.)
    uint32_t n_ctx_train;       // Training context size
    uint32_t n_embd;            // Embedding dimension
    uint32_t n_head;            // Number of attention heads
    uint32_t n_head_kv;         // KV heads (for GQA, = n_head for MHA)
    uint32_t n_layer;           // Number of transformer layers
    uint32_t n_rot;             // RoPE dimensions
    uint32_t n_ff;              // FFN intermediate dimension

    // Attention
    uint32_t n_expert;          // Experts per layer (MoE)
    uint32_t n_expert_used;     // Active experts (MoE)

    // Normalization
    float norm_eps;             // Layer norm epsilon
    float norm_rms_eps;         // RMS norm epsilon

    // RoPE
    float rope_freq_base;       // RoPE base frequency
    float rope_dim;             // RoPE dimension fraction

    // Attention
    float attn_scale;           // Attention scale
    bool    causal_attn;        // Causal (decoder-only)
    bool    head_dim_mul;       // Head dimension multiplier
};
```

### 1.3 Architecture Comparison

| Model | Params | Layers | Embd | Heads | FFN | Vocab |
|-------|--------|--------|------|-------|-----|-------|
| LLaMA 3.2 1B | 1.2B | 16 | 2048 | 32 | 8192 | 128K |
| LLaMA 3.2 3B | 3.2B | 28 | 3072 | 24 | 8192 | 128K |
| LLaMA 3 8B | 8B | 32 | 4096 | 32 | 14336 | 128K |
| LLaMA 2 7B | 7B | 32 | 4096 | 32 | 11008 | 32K |
| Mistral 7B | 7B | 32 | 4096 | 32 | 14336 | 32K |
| Mixtral 8x7B | 47B (12.9B active) | 32 | 4096 | 32 | 14336 | 32K |
| Gemma 2B | 2B | 18 | 2048 | 8 | 8192 | 256K |
| Qwen 2.5 7B | 7B | 28 | 3584 | 28 | 18944 | 152K |

---

## 2. LLaMA Architecture

### 2.1 LLaMA Block Structure

```
┌─────────────────────────────────────────────────────────┐
│                  LLaMA Transformer Block                 │
│                                                          │
│  Input                                                   │
│    │                                                     │
│    ▼                                                     │
│  ┌─────────────────────────────┐                        │
│  │      Pre-Norm (RMSNorm)     │                        │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────┐                        │
│  │   Grouped Query Attention   │  ← GQA (n_head_kv < n_head)
│  │   (RoPE positional enc.)    │  ← Rotary embeddings   │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│    ┌────────────┴────────────┐                         │
│    │      Residual Add       │                         │
│    └────────────┬────────────┘                         │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────┐                        │
│  │      Pre-Norm (RMSNorm)     │                        │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────┐                        │
│  │      SwiGLU FFN             │  ← (x @ W_gate) * SiLU(x @ W_up) @ W_down
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│    ┌────────────┴────────────┐                         │
│    │      Residual Add       │                         │
│    └────────────┬────────────┘                         │
│                 │                                       │
│                 ▼                                       │
│              Output                                     │
│                                                        │
└─────────────────────────────────────────────────────────┘
```

### 2.2 RMSNorm (Root Mean Square Layer Normalization)

```c
// RMSNorm vs LayerNorm
// LayerNorm: y = (x - mean) / std * gamma + beta
// RMSNorm:   y = x / rms * gamma  (no mean subtraction, no bias)

void llama_rms_norm_f32(const float * x, float * dst, int n, float eps, const float * weight) {
    // Compute RMS
    float sum = 0.0f;
    for (int i = 0; i < n; i++) {
        sum += x[i] * x[i];
    }
    float rms = sqrtf(sum / n + eps);

    // Normalize and scale
    for (int i = 0; i < n; i++) {
        dst[i] = x[i] / rms * weight[i];
    }
}

// Why RMSNorm?
// - 7-10% faster (no mean computation)
// - Same quality for LLMs
// - Used in LLaMA, Mistral, Gemma
```

### 2.3 RoPE (Rotary Position Embeddings)

```c
// RoPE rotates query and key vectors based on position
// No learned parameters - purely functional

void llama_rope_f32(
    const float * src,
    int64_t n_dims,
    int64_t ne0,
    int64_t ne1,
    int64_t ne2,
    int64_t ne3,
    int64_t nb0,
    int64_t nb1,
    int64_t nb2,
    int64_t nb3,
    int64_t n_tokens,
    float freq_base,
    float freq_scale,
    const int32_t * positions
) {
    const float theta_scale = powf(freq_base, -1.0f / n_dims);

    for (int64_t i3 = 0; i3 < ne3; i3++) {
        for (int64_t i2 = 0; i2 < ne2; i2++) {
            const int64_t pos = positions[i2];

            for (int64_t i1 = 0; i1 < ne1; i1++) {
                for (int64_t i0 = 0; i0 < ne0; i0 += 2) {
                    const float theta = powf(theta_scale, i0) * freq_scale * pos;
                    const float cos_theta = cosf(theta);
                    const float sin_theta = sinf(theta);

                    const float * src_ptr = (const float *)((char *)src + i3*nb3 + i2*nb2 + i1*nb1 + i0*nb0);
                    float * dst_ptr = (float *)((char *)dst + i3*nb3 + i2*nb2 + i1*nb1 + i0*nb0);

                    // RoPE rotation
                    const float x0 = src_ptr[0];
                    const float x1 = src_ptr[1];

                    dst_ptr[0] = x0 * cos_theta - x1 * sin_theta;
                    dst_ptr[1] = x0 * sin_theta + x1 * cos_theta;
                }
            }
        }
    }
}

// Visual: 2D rotation in embedding space
// Position 0: [cos(0), sin(0)] = [1, 0]
// Position 1: [cos(θ), sin(θ)]
// Position 2: [cos(2θ), sin(2θ)]
// ...
```

**Why RoPE works:**
- Relative positions encoded naturally
- Extrapolates to longer contexts
- No learned parameters to store

### 2.4 SwiGLU FFN

```c
// SwiGLU: Gated Linear Unit with SiLU activation
// FFN(x) = (x @ W_gate ⊗ SiLU(x @ W_up)) @ W_down

void llama_ffn_swiglu_f32(
    const float * input,
    float * output,
    const float * weight_gate,
    const float * weight_up,
    const float * weight_down,
    int n_embd,
    int n_ff
) {
    // Temporary buffers
    float * gate = malloc(n_ff * sizeof(float));
    float * up = malloc(n_ff * sizeof(float));

    // Gate projection: x @ W_gate
    for (int i = 0; i < n_ff; i++) {
        gate[i] = 0;
        for (int j = 0; j < n_embd; j++) {
            gate[i] += input[j] * weight_gate[j * n_ff + i];
        }
    }

    // Up projection: x @ W_up
    for (int i = 0; i < n_ff; i++) {
        up[i] = 0;
        for (int j = 0; j < n_embd; j++) {
            up[i] += input[j] * weight_up[j * n_ff + i];
        }
    }

    // SiLU activation on gate
    for (int i = 0; i < n_ff; i++) {
        gate[i] = gate[i] / (1.0f + expf(-gate[i]));  // SiLU
    }

    // Element-wise multiplication
    for (int i = 0; i < n_ff; i++) {
        gate[i] *= up[i];
    }

    // Down projection: gate @ W_down
    for (int i = 0; i < n_embd; i++) {
        output[i] = 0;
        for (int j = 0; j < n_ff; j++) {
            output[i] += gate[j] * weight_down[j * n_embd + i];
        }
    }

    free(gate);
    free(up);
}
```

### 2.5 LLaMA Architecture Evolution

```
LLaMA 1 (Feb 2023):
- Standard decoder-only transformer
- RMSNorm pre-normalization
- SwiGLU FFN
- RoPE positions
- 7B, 13B, 33B, 65B

LLaMA 2 (Jul 2023):
- GQA for 70B model (8 KV heads)
- Larger context (4K tokens)
- Better initialization

LLaMA 3 (Apr 2024):
- Larger vocabulary (128K tokens)
- GQA for all models (8 KV heads)
- Larger context (8K tokens)
- Improved tokenizer (tiktoken-based)

LLaMA 3.2 (Sep 2024):
- Small models (1B, 3B) for edge devices
- Multimodal support (vision encoder)
- Text+image input
```

---

## 3. Mistral and Mixtral MoE

### 3.1 Mistral 7B Architecture

```
Mistral 7B innovations:

1. Sliding Window Attention (SWA)
   - Attention window: 4096 tokens
   - Allows attending beyond window during training
   - Enables longer context at inference

2. Rolling Buffer Attention
   - Evict old tokens when cache full
   - Shift positions instead of recomputing

3. GQA throughout
   - 32 query heads, 8 KV heads
   - 4x smaller KV cache

4. Larger sliding window for later layers
   - Last layers: full attention
   - Early layers: sliding window
```

### 3.2 Mixtral 8x7B MoE

```
┌─────────────────────────────────────────────────────────┐
│                  Mixtral Sparse MoE Block                │
│                                                          │
│  Input                                                   │
│    │                                                     │
│    ▼                                                     │
│  ┌─────────────────────────────┐                        │
│  │      RMSNorm                │                        │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────┐                        │
│  │      GQA Attention          │                        │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│    ┌────────────┴────────────┐                         │
│    │      Residual Add       │                         │
│    └────────────┬────────────┘                         │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────┐                        │
│  │      RMSNorm                │                        │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│          ┌──────┴──────┐                               │
│          ▼             ▼                               │
│  ┌───────────┐   ┌───────────┐        ┌───────────┐   │
│  │ Expert 1  │   │ Expert 2  │  ...   │ Expert 8  │   │
│  │  (SwiGLU) │   │  (SwiGLU) │        │  (SwiGLU) │   │
│  └─────┬─────┘   └─────┬─────┘        └─────┬─────┘   │
│        │               │                     │         │
│        └───────────────┼─────────────────────┘         │
│                        │                                │
│              Top-2 Routing (weighted sum)              │
│                        │                                │
│                        ▼                                │
│              Residual Add                               │
│                        │                                │
│                        ▼                                │
│                     Output                              │
│                                                        │
└─────────────────────────────────────────────────────────┘
```

### 3.3 MoE Routing

```c
// Mixtral routing: Top-2 experts per token
void moe_routing(
    const float * hidden,
    float * output,
    const float * gate_weight,
    const float ** expert_weights,
    int n_experts,
    int n_experts_active,
    int n_embd,
    int n_ff
) {
    // Compute gating scores
    float scores[8];
    for (int e = 0; e < n_experts; e++) {
        scores[e] = 0;
        for (int i = 0; i < n_embd; i++) {
            scores[e] += hidden[i] * gate_weight[e * n_embd + i];
        }
    }

    // Select top-2 experts
    int top_experts[2];
    float top_scores[2];
    select_top_k(scores, n_experts, 2, top_experts, top_scores);

    // Softmax over selected experts
    float sum_exp = 0;
    for (int i = 0; i < 2; i++) {
        sum_exp += expf(top_scores[i]);
    }
    float weights[2] = {expf(top_scores[0]) / sum_exp, expf(top_scores[1]) / sum_exp};

    // Apply selected experts
    for (int i = 0; i < 2; i++) {
        float expert_out[4096];
        apply_expert(hidden, expert_out, expert_weights[top_experts[i]]);

        // Weighted sum
        for (int j = 0; j < n_embd; j++) {
            output[j] += weights[i] * expert_out[j];
        }
    }
}
```

### 3.4 MoE Efficiency

```
Mixtral 8x7B parameter count:

Total parameters: 8 experts × 7B = 56B + router = ~47B
Active per token: 2 experts × 1.3B = 2.6B + router = ~12.9B

Memory: 47B × 2 bytes (FP16) = 94 GB (Q4_K_M: 26 GB)
Compute per token: 12.9B ops

vs LLaMA 7B:
Memory: 7B × 2 bytes = 14 GB (Q4_K_M: 4 GB)
Compute per token: 7B ops

Mixtral is ~2x slower but significantly better quality!
```

---

## 4. Gemma Architecture

### 4.1 Gemma 2 Architecture

```
Gemma 2 (Google, 2024):

Key features:
- Similar to LLaMA but with differences
- Post-norm instead of pre-norm
- Local attention in alternating layers
- Logit soft-capping
- Larger head dimensions
```

```
┌─────────────────────────────────────────────────────────┐
│                   Gemma 2 Block                          │
│                                                          │
│  Input                                                   │
│    │                                                     │
│    ▼                                                     │
│  ┌─────────────────────────────┐                        │
│  │      RMSNorm (input)        │  ← Pre-norm            │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────┐                        │
│  │    Local Attention          │  ← 4096 window        │
│  │    (alternating layers)     │                        │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│  ┌─────────────────────────────┐                        │
│  │      Post-Norm              │  ← Post-norm          │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│    ┌────────────┴────────────┐                         │
│    │      Residual Add       │                         │
│    └────────────┬────────────┘                         │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────┐                        │
│  │      RMSNorm (FFN input)    │                        │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│                 ▼                                       │
│  ┌─────────────────────────────┐                        │
│  │      GeGLU FFN              │  ← GeLU, not SiLU     │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│  ┌─────────────────────────────┐                        │
│  │      Post-Norm              │                        │
│  └──────────────┬──────────────┘                        │
│                 │                                       │
│    ┌────────────┴────────────┐                         │
│    │      Residual Add       │                         │
│    └────────────┬────────────┘                         │
│                 │                                       │
│                 ▼                                       │
│              Output                                     │
│                                                        │
└─────────────────────────────────────────────────────────┘
```

### 4.2 GeGLU vs SwiGLU

```c
// GeGLU (used in Gemma):
// FFN(x) = (x @ W_up ⊗ GeLU(x @ W_gate)) @ W_down

// SwiGLU (used in LLaMA):
// FFN(x) = (x @ W_up ⊗ SiLU(x @ W_gate)) @ W_down

// Activation functions:
float gelu(float x) {
    return 0.5f * x * (1.0f + tanhf(0.7978845608028654f * (x + 0.044715f * x * x * x)));
}

float silu(float x) {
    return x / (1.0f + expf(-x));
}

// GeGLU: smoother activation, better for smaller models
// SwiGLU: slightly better for larger models
```

### 4.3 Logit Soft-Capping

```c
// Gemma uses logit soft-capping to prevent extreme values
void gemma_softcap_logits(float * logits, int n_vocab, float softcap) {
    for (int i = 0; i < n_vocab; i++) {
        logits[i] = tanhf(logits[i] / softcap) * softcap;
    }
}

// Effect:
// - Prevents very high/low logits
// - Stabilizes training
// - Similar to gradient clipping
```

---

## 5. Qwen Architecture

### 5.1 Qwen 2.5 Architecture

```
Qwen 2.5 (Alibaba, 2024):

Key features:
- LLaMA-like architecture
- GQA throughout
- SwiGLU FFN
- RoPE with YaRN (for long context)
- Larger intermediate size
- 128K context support
```

```c
// Qwen uses YARN for length extrapolation
void qwen_rope_yarn(
    float * Q, float * K,
    int n_tokens,
    int n_dims,
    float freq_base,
    float original_max_length,
    float current_length
) {
    // YARN: YaRN (Yet another RoPE for long context)
    // Scales RoPE frequencies based on sequence length

    float scale_factor = current_length / original_max_length;
    float attention_factor = sqrtf(1.0f + logf(scale_factor) / logf(original_max_length));

    // Apply scaled RoPE
    for (int i = 0; i < n_tokens; i++) {
        float theta_scale = powf(freq_base, -1.0f / n_dims);
        float freq = theta_scale * attention_factor;

        if (i < original_max_length) {
            // Standard RoPE for short sequences
            freq *= 1.0f;
        } else {
            // Scaled RoPE for long sequences
            freq /= scale_factor;
        }

        // Apply rotation...
    }
}
```

---

## 6. Attention Mechanisms

### 6.1 Multi-Head Attention (MHA)

```
Standard MHA:
Q: [batch, seq, n_heads, head_dim]
K: [batch, seq, n_heads, head_dim]
V: [batch, seq, n_heads, head_dim]

for each head:
    attention = softmax(Q @ K.T / sqrt(head_dim)) @ V

output = concat(all heads) @ W_o

Parameters: n_heads × (K + V + O) matrices
```

### 6.2 Grouped Query Attention (GQA)

```
GQA (LLaMA 2 70B, LLaMA 3, Mistral):
Q: [batch, seq, n_heads, head_dim]
K: [batch, seq, n_kv_heads, head_dim]  ← Fewer!
V: [batch, seq, n_kv_heads, head_dim]  ← Fewer!

# Reshape Q to match KV groups
Q_grouped = Q.reshape(batch, seq, n_kv_heads, n_heads // n_kv_heads, head_dim)

attention = softmax(Q_grouped @ K.T / sqrt(head_dim)) @ V

KV cache reduction: n_heads / n_kv_heads times smaller!
```

### 6.3 Multi-Query Attention (MQA)

```
MQA (Falcon, StarCoder):
Q: [batch, seq, n_heads, head_dim]
K: [batch, seq, 1, head_dim]  ← Single!
V: [batch, seq, 1, head_dim]  ← Single!

All heads share the same K, V

Maximum KV cache reduction, but quality loss!
```

### 6.4 Sliding Window Attention (SWA)

```c
// Sliding window masks attention to recent tokens
void apply_sliding_window_mask(
    float * attention_scores,
    int seq_len,
    int window_size,
    int current_pos
) {
    for (int i = 0; i < seq_len; i++) {
        // Mask tokens outside window
        if (current_pos - i > window_size) {
            attention_scores[i] = -INFINITY;
        }
    }
}

// Mistral uses 4096 token window
// Allows infinite generation with bounded memory
```

### 6.5 Flash Attention

```
Flash Attention optimizes memory access:

Standard Attention:
1. Load Q, K, V from HBM (slow)
2. Compute S = Q @ K.T
3. Compute P = softmax(S)
4. Compute O = P @ V
5. Store O to HBM

Flash Attention:
1. Load Q, K, V blocks to SRAM (fast)
2. Compute attention for block
3. Accumulate partial results
4. Repeat for all blocks
5. Single write to HBM

Result: 2-3x faster for long sequences!
```

---

## 7. Feed-Forward Networks

### 7.1 FFN Variants

```c
// Standard FFN (GPT-2)
FFN(x) = gelu(x @ W1) @ W2

// SwiGLU (LLaMA, Mistral, Qwen)
FFN(x) = (x @ W_gate ⊗ silu(x @ W_up)) @ W_down

// GeGLU (Gemma, PaLM)
FFN(x) = (x @ W_gate ⊗ gelu(x @ W_up)) @ W_down

// ReGLU
FFN(x) = (x @ W_gate ⊗ relu(x @ W_up)) @ W_down

// NoFFN (some MoE models)
FFN(x) = x  ← Skip connection only
```

### 7.2 FFN Size Scaling

```
Standard scaling:
n_ff = 4 × n_embd

LLaMA scaling (SwiGLU has 3 matrices):
n_ff = (8/3) × n_embd  ← Adjusted for efficiency

Example:
LLaMA 7B: n_embd = 4096, n_ff = 11008 (≈ 2.67×)
LLaMA 3 8B: n_embd = 4096, n_ff = 14336 (3.5×)
Mistral 7B: n_embd = 4096, n_ff = 14336 (3.5×)
```

---

## 8. Position Encodings

### 8.1 RoPE Deep Dive

```
RoPE rotates vectors in 2D planes:

For position m and dimension i:
- Rotation angle: θ_i = 10000^(-2i/d)
- Rotation matrix: [[cos(mθ_i), -sin(mθ_i)], [sin(mθ_i), cos(mθ_i)]]

Applied to query and key:
Q_m = RoPE(Q, m)
K_n = RoPE(K, n)

Dot product gives relative position:
Q_m · K_n = Q · RoPE(K, n-m)

Key insight: Relative positions encoded naturally!
```

### 8.2 RoPE Scaling for Long Context

```
Methods to extend RoPE beyond training length:

1. Linear Scaling (Naive)
   scale = target_length / training_length
   θ_scaled = θ / scale
   Simple but degrades quality

2. NTK-Aware Scaled RoPE
   θ_scaled = θ × (scale)^(d/(d-2))
   Better preserves high-frequency components

3. YaRN (Yet another RoPE)
   Dynamic scaling factor based on position
   Best quality for long contexts

4. RoPE Breakpoint
   Keep original RoPE up to training length
   Linear scale beyond
```

---

## 9. Rust Translation Patterns

### 9.1 Model Architecture in Rust

```rust
#[derive(Debug, Clone)]
pub enum LlamaArch {
    Llama,
    Mistral,
    Gemma,
    Qwen,
    // ...
}

#[derive(Debug, Clone)]
pub struct LlamaHparams {
    pub n_vocab: u32,
    pub n_ctx_train: u32,
    pub n_embd: u32,
    pub n_head: u32,
    pub n_head_kv: u32,
    pub n_layer: u32,
    pub n_ff: u32,
    pub n_rot: u32,
    pub norm_rms_eps: f32,
    pub rope_freq_base: f32,
    pub arch: LlamaArch,

    // MoE
    pub n_expert: u32,
    pub n_expert_used: u32,

    // Attention
    pub attn_window: Option<u32>,  // Sliding window size
}

impl LlamaHparams {
    pub fn n_gqa(&self) -> u32 {
        self.n_head / self.n_head_kv
    }

    pub fn head_dim(&self) -> u32 {
        self.n_embd / self.n_head
    }

    pub fn is_moe(&self) -> bool {
        self.n_expert > 1
    }
}
```

### 9.2 Transformer Block

```rust
use std::sync::Arc;

pub struct TransformerBlock {
    // Attention
    attn_norm: Arc<GgmlTensor>,
    wq: Arc<GgmlTensor>,
    wk: Arc<GgmlTensor>,
    wv: Arc<GgmlTensor>,
    wo: Arc<GgmlTensor>,

    // FFN
    ffn_norm: Arc<GgmlTensor>,
    w_gate: Arc<GgmlTensor>,
    w_up: Arc<GgmlTensor>,
    w_down: Arc<GgmlTensor>,

    // MoE (optional)
    gate: Option<Arc<GgmlTensor>>,
    experts: Option<Vec<Expert>>,
}

struct Expert {
    w_gate: Arc<GgmlTensor>,
    w_up: Arc<GgmlTensor>,
    w_down: Arc<GgmlTensor>,
}

impl TransformerBlock {
    pub fn forward(
        &self,
        x: &GgmlTensor,
        cache: &mut KvCache,
        positions: &[i32],
    ) -> GgmlTensor {
        // Attention path
        let normed = rms_norm(x, &self.attn_norm);
        let q = linear(&normed, &self.wq);
        let k = linear(&normed, &self.wk);
        let v = linear(&normed, &self.wv);

        // Apply RoPE
        let q = apply_rope(q, positions);
        let k = apply_rope(k, positions);

        // Update cache and compute attention
        cache.update(&k, &v);
        let attn_out = grouped_query_attention(q, &cache.k, &cache.v, &cache.mask);
        let attn_out = linear(&attn_out, &self.wo);

        // Residual
        let x = add(x, &attn_out);

        // FFN path
        let normed = rms_norm(&x, &self.ffn_norm);
        let gate = linear(&normed, &self.w_gate);
        let up = linear(&normed, &self.w_up);

        // SwiGLU
        let gate = silu(gate);
        let ffn_input = mul(&gate, &up);
        let ffn_out = linear(&ffn_input, &self.w_down);

        // Final residual
        add(&x, &ffn_out)
    }
}
```

### 9.3 Valtron Model Forward Pass

```rust
use valtron::{TaskIterator, TaskStatus};

pub struct ModelForward {
    model: Arc<LlamaModel>,
    tokens: Vec<TokenId>,
    positions: Vec<i32>,
    cache: Arc<Mutex<KvCache>>,
    state: ForwardState,
    output: Option<Vec<f32>>,
}

enum ForwardState {
    Embedding,
    ProcessingLayers(usize),
    FinalNorm,
    Output,
    Done,
}

impl TaskIterator for ModelForward {
    type Ready = Vec<f32>;
    type Pending = ComputeProgress;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match self.state {
            ForwardState::Embedding => {
                // Look up token embeddings
                self.state = ForwardState::ProcessingLayers(0);
                Some(TaskStatus::Pending(ComputeProgress::Embedding))
            }
            ForwardState::ProcessingLayers(layer) => {
                if layer >= self.model.n_layers {
                    self.state = ForwardState::FinalNorm;
                    return Some(TaskStatus::Pending(ComputeProgress::LayerNorm));
                }

                // Process one layer
                let block = &self.model.layers[layer];
                let mut cache = self.cache.lock().unwrap();

                // Forward through block
                // ...

                self.state = ForwardState::ProcessingLayers(layer + 1);
                Some(TaskStatus::Pending(ComputeProgress::Layer(layer as u32)))
            }
            ForwardState::FinalNorm => {
                // Final RMSNorm
                self.state = ForwardState::Output;
                Some(TaskStatus::Pending(ComputeProgress::OutputNorm))
            }
            ForwardState::Output => {
                // Output projection to vocabulary
                let logits = lm_head(&self.hidden, &self.model.output);
                self.output = Some(logits.clone());
                self.state = ForwardState::Done;
                Some(TaskStatus::Ready(logits))
            }
            ForwardState::Done => None,
        }
    }
}
```

### 9.4 MoE Routing in Rust

```rust
pub struct MoeLayer {
    gate: Arc<GgmlTensor>,
    experts: Vec<Expert>,
    n_expert_used: usize,
}

impl MoeLayer {
    pub fn forward(&self, x: &GgmlTensor) -> GgmlTensor {
        // Compute gating scores
        let scores = linear(x, &self.gate);  // [seq_len, n_experts]

        // Select top-k experts
        let (top_experts, top_weights) = self.select_top_k(&scores, self.n_expert_used);

        // Apply experts and combine
        let mut output = zeros_like(x);

        for (expert_idx, (expert_id, weight)) in top_experts.iter().enumerate() {
            let expert_out = self.experts[*expert_id].forward(x);
            output = add(&output, &mul_scalar(&expert_out, *weight));
        }

        output
    }

    fn select_top_k(
        &self,
        scores: &[f32],
        k: usize,
    ) -> (Vec<usize>, Vec<f32>) {
        // Softmax over experts
        let probs = softmax(scores);

        // Select top-k
        let mut indices: Vec<usize> = (0..probs.len()).collect();
        indices.sort_by(|a, b| probs[*b].partial_cmp(&probs[*a]).unwrap());
        indices.truncate(k);

        // Renormalize weights
        let weights: Vec<f32> = indices.iter()
            .map(|&i| probs[i])
            .collect();
        let sum: f32 = weights.iter().sum();
        let weights: Vec<f32> = weights.iter().map(|&w| w / sum).collect();

        (indices, weights)
    }
}
```

---

## Summary

### Key Takeaways

1. **LLaMA** uses pre-norm, RMSNorm, SwiGLU, RoPE, and GQA
2. **Mistral** adds sliding window attention for efficiency
3. **Mixtral** uses sparse MoE with 8 experts, 2 active per token
4. **Gemma** uses post-norm, GeGLU, and logit soft-capping
5. **Qwen** uses YaRN for long context RoPE scaling
6. **GQA** significantly reduces KV cache size with minimal quality loss
7. **RoPE** encodes relative positions through rotation

### Next Steps

Continue to:
- [rust-revision.md](rust-revision.md) — Complete Rust translation guide
- [production-grade.md](production-grade.md) — Production deployment

---

*This document complements the official llama.cpp documentation. Refer to the source code for authoritative implementation details.*
