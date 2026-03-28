---
title: "GGML Format Deep Dive: Tensor Library and Quantization"
subtitle: "Complete guide to GGML computation graphs, GGUF format, and quantization types"
based_on: "llama.cpp ggml/ source code"
level: "Intermediate - Requires LLM fundamentals"
---

# GGML Format Deep Dive

## Table of Contents

1. [GGML Architecture Overview](#1-ggml-architecture-overview)
2. [Tensor Types and Memory Layout](#2-tensor-types-and-memory-layout)
3. [Computation Graphs](#3-computation-graphs)
4. [GGUF File Format Specification](#4-gguf-file-format-specification)
5. [Quantization Types Deep Dive](#5-quantization-types-deep-dive)
6. [Backend Implementations](#6-backend-implementations)
7. [Rust Translation Patterns](#7-rust-translation-patterns)

---

## 1. GGML Architecture Overview

### 1.1 What is GGML?

GGML (Ggerganov's Generic Machine Learning Library) is a pure C library for tensor computation designed for:

- **LLM inference** (primary use case)
- **Quantized operations** (first-class support)
- **Multi-backend** (CPU, CUDA, Metal, Vulkan, SYCL, OpenCL)
- **Zero dependencies** (standard C library only)

```
┌─────────────────────────────────────────────────────────┐
│                    GGML Architecture                     │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │              High-Level API (llama.cpp)             ││
│  │  - Model loading   - KV cache management            ││
│  │  - Sampling        - Batch processing               ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │                                 │
│                        ▼                                 │
│  ┌─────────────────────────────────────────────────────┐│
│  │              GGML Tensor Operations                 ││
│  │  - ggml_mul_mat()  - ggml_add()                     ││
│  │  - ggml_norm()     - ggml_soft_max()                ││
│  │  - ggml_rope()     - ggml_scale()                   ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │                                 │
│                        ▼                                 │
│  ┌─────────────────────────────────────────────────────┐│
│  │              Computation Graph                      ││
│  │  - Build phase: Create nodes                        ││
│  │  - Compute phase: Execute with threading            ││
│  └─────────────────────┬───────────────────────────────┘│
│                        │                                 │
│        ┌───────────────┼───────────────┐                │
│        ▼               ▼               ▼                │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐           │
│  │   CPU    │   │   CUDA   │   │  Metal   │           │
│  │ Backend  │   │ Backend  │   │ Backend  │           │
│  └──────────┘   └──────────┘   └──────────┘           │
│                                                        │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Core Data Structures

```c
// ggml.h - Core structures

// Tensor object
struct ggml_tensor {
    enum ggml_type type;      // Data type (FP32, FP16, Q4_0, etc.)
    struct ggml_backend_buffer * buffer;  // Memory buffer

    int64_t ne[GGML_MAX_DIMS];  // Number of elements per dimension
    size_t  nb[GGML_MAX_DIMS];  // Stride in bytes per dimension

    // Source tensors (for operations)
    struct ggml_tensor * src[GGML_MAX_SRC];

    // Operation information
    enum ggml_op op;
    ggml_op_params op_params;

    // Data pointer
    void * data;

    // Graph node indices
    int32_t perf_first;
    int32_t perf_last;
};

// Memory context (arena allocator)
struct ggml_context {
    size_t mem_size;
    void * mem_buffer;
    bool owns_buffer;

    // Allocated tensors
    struct ggml_tensor ** tensors;
    int n_tensors;
};

// Computation graph
struct ggml_cgraph {
    int n_nodes;
    int n_leafs;

    struct ggml_tensor ** nodes;
    struct ggml_tensor ** grads;
    struct ggml_tensor ** leafs;

    // Execution order
    int32_t * perf_order;
};
```

### 1.3 Tensor Dimensions and Strides

```c
// GGML supports up to 4 dimensions
#define GGML_MAX_DIMS 4

// Example: 2D tensor [768, 1024] (width=768, height=1024)
struct ggml_tensor * t = ggml_new_tensor_2d(ctx, GGML_TYPE_F32, 768, 1024);

// Dimensions (ne = number of elements)
t->ne[0] = 768;    // Width (columns)
t->ne[1] = 1024;   // Height (rows)
t->ne[2] = 1;      // Depth (unused for 2D)
t->ne[3] = 1;      // Time (unused for 2D)

// Strides (nb = number of bytes)
t->nb[0] = 4;      // 4 bytes per F32 element
t->nb[1] = 768*4;  // Bytes per row
t->nb[2] = 0;      // Unused
t->nb[3] = 0;      // Unused

// Total size
size_t size = ggml_nbytes(t);  // 768 * 1024 * 4 = 3,145,728 bytes
```

**Visual:**

```
2D Tensor [width=4, height=3]:
┌─────────────────────────────────┐
│ ne[0] = 4 (columns)             │
│ ne[1] = 3 (rows)                │
│                                  │
│ Row-major layout:               │
│ [0,0] [1,0] [2,0] [3,0]  ← nb[0] = 4 bytes
│ [0,1] [1,1] [2,1] [3,1]     stride
│ [0,2] [1,2] [2,2] [3,2]         │
│                                  │
│ nb[1] = 4 * 4 = 16 bytes/row    │
└─────────────────────────────────┘
```

---

## 2. Tensor Types and Memory Layout

### 2.1 GGML Type Enum

```c
enum ggml_type {
    GGML_TYPE_F32     = 0,   // 32-bit float
    GGML_TYPE_F16     = 1,   // 16-bit float
    GGML_TYPE_Q4_0    = 2,   // 4-bit quantized (block size 32)
    GGML_TYPE_Q4_1    = 3,   // 4-bit quantized with scale
    GGML_TYPE_Q5_0    = 6,   // 5-bit quantized
    GGML_TYPE_Q5_1    = 7,
    GGML_TYPE_Q8_0    = 8,   // 8-bit quantized
    GGML_TYPE_Q8_1    = 9,
    GGML_TYPE_Q2_K    = 10,  // 2-bit K-quants
    GGML_TYPE_Q3_K    = 11,  // 3-bit K-quants
    GGML_TYPE_Q4_K    = 12,  // 4-bit K-quants
    GGML_TYPE_Q5_K    = 13,  // 5-bit K-quants
    GGML_TYPE_Q6_K    = 14,  // 6-bit K-quants
    GGML_TYPE_Q8_K    = 15,  // 8-bit K-quants
    GGML_TYPE_IQ2_XXS = 16,  // Integer quantization
    GGML_TYPE_IQ2_XS  = 17,
    GGML_TYPE_IQ3_XXS = 18,
    // ... many more
    GGML_TYPE_COUNT,
};
```

### 2.2 Type Traits

```c
// Each type has associated traits
struct ggml_type_traits {
    const char * type_name;
    size_t type_size;         // Size in bytes for one element
    size_t blck_size;         // Block size for quantized types
    size_t blck_size_scaled;  // Scaled block size
    float to_float;           // Dequantization function
    size_t from_float;        // Quantization function
};

// Example: Q4_0 traits
static const struct ggml_type_traits q4_0_traits = {
    .type_name = "Q4_0",
    .type_size = sizeof(block_q4_0),  // 2 + 16 = 18 bytes per block
    .blck_size = 32,                   // 32 weights per block
    .to_float = dequantize_q4_0,
    .from_float = quantize_q4_0,
};

// Effective bits per weight
// Q4_0: 18 bytes * 8 bits / 32 weights = 4.5 bits/weight
```

### 2.3 Quantized Block Layouts

```c
// Q4_0 block structure
typedef struct {
    ggml_fp16_t d;           // Scale factor (2 bytes)
    uint8_t qs[QK4_0 / 2];   // Quantized weights (16 bytes for 32 weights)
} block_q4_0;                // Total: 18 bytes

// Memory layout for 32 weights:
// [d (2B)] [qs0 (1B)] [qs1 (1B)] ... [qs15 (1B)]
//           │
//           └─ Each byte contains 2 4-bit quants:
//              [q0,q1] [q2,q3] ... [q30,q31]

// Q4_K block structure (improved quality)
typedef struct {
    ggml_half d[2];          // Two scale factors
    uint8_t scales[8];       // 8 scale modifiers
    uint8_t qs[QK4_0 / 2];   // Quantized weights
} block_q4_K;                // Total: 28 bytes for 64 weights
```

**Visual:**

```
Q4_0 Block (32 weights → 18 bytes):
┌──────────────────────────────────────────────────────┐
│  Scale (d)  │  Quants (16 bytes, 2 per byte)        │
│  2 bytes    │  [q0,q1] [q2,q3] ... [q30,q31]       │
│             │  └──── 4 bits each ────┘              │
└──────────────────────────────────────────────────────┘

Q4_K Block (64 weights → 28 bytes):
┌──────────────────────────────────────────────────────┐
│  d[2]   │  scales[8]  │  Quants (16 bytes)          │
│  4 bytes│  8 bytes    │  [q0,q1] ... [q62,q63]     │
│         │             │  └──── 4 bits each ────┘    │
└──────────────────────────────────────────────────────┘
```

---

## 3. Computation Graphs

### 3.1 Building Computation Graphs

```c
// Example: Layer normalization
// y = scale * (x - mean) / sqrt(variance + eps) + bias

struct ggml_tensor * layer_norm(
    struct ggml_context * ctx,
    struct ggml_tensor * x,
    struct ggml_tensor * scale,
    struct ggml_tensor * bias,
    float eps
) {
    // Compute mean
    struct ggml_tensor * mean = ggml_mean(ctx, x);

    // Subtract mean
    struct ggml_tensor * centered = ggml_sub(ctx, x, mean);

    // Compute variance
    struct ggml_tensor * variance = ggml_sqr(ctx, centered);
    variance = ggml_mean(ctx, variance);

    // Compute 1/sqrt(variance + eps)
    struct ggml_tensor * inv_std = ggml_add1(ctx, variance,
                                              ggml_new_f32(ctx, eps));
    inv_std = ggml_sqrt(ctx, inv_std);
    inv_std = ggml_div(ctx, ggml_new_f32(ctx, 1.0f), inv_std);

    // Normalize
    struct ggml_tensor * normalized = ggml_mul(ctx, centered, inv_std);

    // Apply scale and bias
    struct ggml_tensor * scaled = ggml_mul(ctx, normalized, scale);
    struct ggml_tensor * output = ggml_add(ctx, scaled, bias);

    return output;
}
```

### 3.2 Graph Execution

```c
// Build the computation graph
struct ggml_cgraph * gf = ggml_new_graph(ctx);
ggml_build_forward_expand(gf, output_tensor);

// Execute with threading
struct ggml_cplan cplan = ggml_graph_plan(gf, n_threads, NULL);

// Allocate work buffer if needed
void * work_buffer = NULL;
if (cplan.work_size > 0) {
    work_buffer = malloc(cplan.work_size);
    cplan.work_data = work_buffer;
}

// Compute
ggml_graph_compute(gf, &cplan);

// Cleanup
if (work_buffer) free(work_buffer);
```

### 3.3 Graph Structure

```
Computation Graph for Transformer Block:

              Input
                │
        ┌───────┴───────┐
        │               │
        ▼               │
   ┌─────────┐          │
   │  Layer  │          │
   │  Norm 1 │          │
   └────┬────┘          │
        │               │
        ▼               │
   ┌─────────┐          │
   │   Self  │          │
   │Attention│          │
   └────┬────┘          │
        │               │
        ▼               │
   ┌─────────┐          │
   │   Add   │◄─────────┘ (residual)
   │  +Norm  │
   └────┬────┘
        │
        ▼
   ┌─────────┐
   │   FFN   │
   │  (MLP)  │
   └────┬────┘
        │
        ▼
   ┌─────────┐
   │   Add   │◄─── (residual)
   └────┬────┘
        │
        ▼
      Output

Each node is a ggml_tensor with op field set
```

---

## 4. GGUF File Format Specification

### 4.1 GGUF Header

```c
// GGUF magic number
#define GGUF_MAGIC 0x46554747  // "GGUF" in little-endian

// Header structure
struct gguf_header {
    uint32_t magic;           // 0x46554747
    uint32_t version;         // 3 (current version)
    uint64_t tensor_count;    // Number of tensors
    uint64_t kv_count;        // Number of key-value pairs
};
```

### 4.2 Key-Value Metadata

```c
// KV types
enum gguf_type {
    GGUF_TYPE_UINT8   = 0,
    GGUF_TYPE_INT8    = 1,
    GGUF_TYPE_UINT16  = 2,
    GGUF_TYPE_INT16   = 3,
    GGUF_TYPE_UINT32  = 4,
    GGUF_TYPE_INT32   = 5,
    GGUF_TYPE_FLOAT32 = 6,
    GGUF_TYPE_BOOL    = 7,
    GGUF_TYPE_STRING  = 8,
    GGUF_TYPE_ARRAY   = 9,
};

// Common metadata keys
"general.architecture"         // "llama", "gemma", "mistral", etc.
"general.parameter_count"      // Total parameters
"llama.embedding_length"       // Hidden dimension
"llama.block_count"            // Number of transformer blocks
"llama.feed_forward_length"    // FFN intermediate dimension
"llama.rope.freq_base"         // RoPE frequency base
"llama.attention.head_count"   // Number of attention heads
"llama.attention.head_count_kv"// KV heads (for GQA)
"tokenizer.ggml.tokens"        // Token vocabulary
"tokenizer.ggml.scores"        // Token scores
"tokenizer.ggml.token_type"    // Token types
```

### 4.3 Tensor Info and Data

```c
// Tensor info structure
struct gguf_tensor_info {
    uint32_t name_len;
    char * name;              // Tensor name

    uint32_t n_dims;          // Number of dimensions (2-4)
    uint64_t ne[4];           // Dimensions
    enum ggml_type type;      // Data type
    uint64_t offset;          // Offset from data section start
};

// File layout:
// [Header]
// [KV section]
//   - KV 0
//   - KV 1
//   - ...
// [Tensor info section]
//   - Tensor 0 info
//   - Tensor 1 info
//   - ...
// [Alignment padding]
// [Tensor data section]
//   - Tensor 0 data (aligned to 32 bytes)
//   - Tensor 1 data
//   - ...
```

### 4.4 Python GGUF Parser

```python
import struct
from enum import IntEnum

class GGUFType(IntEnum):
    UINT8 = 0
    INT8 = 1
    UINT16 = 2
    INT16 = 3
    UINT32 = 4
    INT32 = 5
    FLOAT32 = 6
    BOOL = 7
    STRING = 8
    ARRAY = 9

def read_gguf(path):
    with open(path, 'rb') as f:
        # Read header
        magic = struct.unpack('<I', f.read(4))[0]
        assert magic == 0x46554747, "Invalid GGUF magic"

        version = struct.unpack('<I', f.read(4))[0]
        tensor_count = struct.unpack('<Q', f.read(8))[0]
        kv_count = struct.unpack('<Q', f.read(8))[0]

        print(f"GGUF version {version}")
        print(f"Tensors: {tensor_count}, KVs: {kv_count}")

        # Read metadata
        metadata = {}
        for _ in range(kv_count):
            key = read_string(f)
            value_type = struct.unpack('<I', f.read(4))[0]
            value = read_value(f, value_type)
            metadata[key] = value

        # Read tensor info
        tensors = []
        for _ in range(tensor_count):
            name = read_string(f)
            n_dims = struct.unpack('<I', f.read(4))[0]
            ne = [struct.unpack('<Q', f.read(8))[0] for _ in range(n_dims)]
            dtype = struct.unpack('<I', f.read(4))[0]
            offset = struct.unpack('<Q', f.read(8))[0]

            tensors.append({
                'name': name,
                'shape': ne,
                'dtype': dtype,
                'offset': offset
            })

        return metadata, tensors
```

---

## 5. Quantization Types Deep Dive

### 5.1 K-Quants Architecture

K-quants (developed by @k-quant) use block-wise quantization with multiple scales:

```
Q2_K Structure (256 weights):
┌─────────────────────────────────────────────────────┐
│  scales (12 bytes)  │  mins (12 bytes)             │
│  d (2 bytes)        │  qs (64 bytes = 256 * 2 bits)│
└─────────────────────────────────────────────────────┘
Total: 90 bytes for 256 weights = 2.81 bits/weight

Q3_K Structure (256 weights):
┌─────────────────────────────────────────────────────┐
│  scales (12 bytes)  │  qs (96 bytes = 256 * 3 bits)│
└─────────────────────────────────────────────────────┘
Total: 110 bytes for 256 weights = 3.44 bits/weight
```

### 5.2 Quantization Implementation

```c
// Q4_0 quantization
size_t quantize_q4_0(const float * src, void * dst, int64_t nrow, int64_t n_per_row) {
    const int64_t nc = n_per_row;
    const int64_t nr = nrow;

    // Q4_0 processes 32 weights per block
    const int64_t qk = QK4_0;  // 32
    const int64_t ql = qk / 2; // 16 quantized bytes

    size_t dst_size = (nc * nr) / qk * sizeof(block_q4_0);

    for (int64_t i = 0; i < nr; i++) {
        const float * row = src + i * nc;
        block_q4_0 * blocks = (block_q4_0 *) ((char *) dst + i * (nc / qk) * sizeof(block_q4_0));

        for (int64_t j = 0; j < nc; j += qk) {
            // Find max absolute value in block
            float amax = 0;
            for (int64_t l = 0; l < qk; l++) {
                amax = fmaxf(amax, fabsf(row[j + l]));
            }

            // Compute scale (map to [-8, 7] range)
            const float d = amax / ((1 << 3) - 1);
            const float id = d ? 1.0f / d : 0;

            blocks[j / qk].d = GGML_FP32_TO_FP16(d);

            // Quantize each weight
            for (int64_t l = 0; l < qk; l += 2) {
                const float v0 = row[j + l] * id;
                const float v1 = row[j + l + 1] * id;

                const int8_t qi0 = MAX(-8, MIN(7, (int8_t)(v0 + 8.5f)));
                const int8_t qi1 = MAX(-8, MIN(7, (int8_t)(v1 + 8.5f)));

                // Pack 2 quants per byte
                blocks[j / qk].qs[l / 2] = qi0 | (qi1 << 4);
            }
        }
    }

    return dst_size;
}
```

### 5.3 Dequantization

```c
// Q4_0 dequantization
void dequantize_q4_0(const void * src, float * dst, int64_t k) {
    const block_q4_0 * blocks = (const block_q4_0 *) src;
    const int64_t nb = k / QK4_0;

    for (int64_t i = 0; i < nb; i++) {
        const float d = GGML_FP16_TO_FP32(blocks[i].d);

        for (int64_t j = 0; j < QK4_0; j += 2) {
            const uint8_t q = blocks[i].qs[j / 2];

            // Unpack and dequantize
            dst[i * QK4_0 + j]     = d * ((int8_t)(q & 0x0F) - 8);
            dst[i * QK4_0 + j + 1] = d * ((int8_t)(q >> 4) - 8);
        }
    }
}
```

### 5.4 Quantization Quality Comparison

```
Model: LLaMA 3.2 1B
Benchmark: MMLU (accuracy)

┌──────────────────────────────────────────┐
│ Type   │ Size    │ MMLU  │ Relative     │
├──────────────────────────────────────────┤
│ F16    │ 2.0 GB  │ 62.4% │ 100%         │
│ Q8_0   │ 1.1 GB  │ 62.3% │ 99.8%        │
│ Q6_K   │ 0.85 GB │ 62.1% │ 99.5%        │
│ Q5_K_M │ 0.75 GB │ 61.8% │ 99.0%        │
│ Q4_K_M │ 0.65 GB │ 61.2% │ 98.1%        │
│ Q3_K_M │ 0.55 GB │ 59.8% │ 95.8%        │
│ Q2_K   │ 0.45 GB │ 56.2% │ 90.1%        │
└──────────────────────────────────────────┘

Recommendation: Q4_K_M for most use cases
- Best size/quality trade-off
- Works well with imatrix calibration
- Good for 4-bit GPUs
```

### 5.5 Importance Matrix (imatrix)

For better low-bit quantization, use calibration data:

```bash
# Generate importance matrix
./build/bin/llama-imatrix \
    -m model-f16.gguf \
    -f calibration-text.txt \
    -o model-imatrix.dat \
    --n-paths 100

# Quantize with importance matrix
./build/bin/llama-quantize \
    --imatrix model-imatrix.dat \
    model-f16.gguf \
    model-q3_k_m.gguf \
    Q3_K_M
```

**Why it works:**
- Some weights matter more than others
- imatrix identifies important weights
- Important weights get more bits
- Better quality at same size

---

## 6. Backend Implementations

### 6.1 Backend Architecture

```c
// Backend interface
struct ggml_backend_i {
    const char * (*get_name)(ggml_backend_t);
    void (*free)(ggml_backend_t);

    // Buffer operations
    ggml_backend_buffer_type_t (*get_default_buffer_type)(ggml_backend_t);
    ggml_backend_buffer_t (*alloc_buffer)(ggml_backend_t, size_t);

    // Tensor operations
    bool (*supports_op)(ggml_backend_t, const struct ggml_tensor *);
    bool (*supports_buft)(ggml_backend_t, ggml_backend_buffer_type_t);

    // Compute
    ggml_backend_graph_plan_t (*graph_plan_create)(ggml_backend_t, ggml_cgraph *);
    void (*graph_plan_free)(ggml_backend_t, ggml_backend_graph_plan_t);
    enum ggml_status (*graph_plan_compute)(ggml_backend_graph_plan_t);
    enum ggml_status (*graph_compute)(ggml_backend_t, ggml_cgraph *);
};
```

### 6.2 CPU Backend (Default)

```c
// CPU backend uses threading
void ggml_graph_compute_cpu(struct ggml_cgraph * cgraph, int n_threads) {
    // Parallelize across nodes
    for (int i = 0; i < cgraph->n_nodes; i++) {
        struct ggml_tensor * node = cgraph->nodes[i];

        // Multi-threaded execution
        if (ggml_is_parallelizable(node) && n_threads > 1) {
            ggml_compute_forward_multithread(node, n_threads);
        } else {
            ggml_compute_forward_single(node);
        }
    }
}

// Uses BLAS for matrix multiplication
void ggml_compute_forward_mul_mat_f32(struct ggml_tensor * dst) {
    // src0: [n, m]
    // src1: [m, p]
    // dst:  [n, p]

    cblas_sgemm(
        CblasRowMajor,
        CblasNoTrans,    // src0
        CblasTrans,      // src1
        n, p, m,
        1.0f,
        src0->data, m,
        src1->data, m,
        0.0f,
        dst->data, n
    );
}
```

### 6.3 CUDA Backend

```c
// CUDA matrix multiplication
void ggml_compute_forward_mul_mat_cuda(struct ggml_tensor * dst) {
    const float * a = (float *) dst->src[0]->data;  // Device memory
    const float * b = (float *) dst->src[1]->data;
    float * c = (float *) dst->data;

    // cuBLAS call
    cublasSgemm(
        g_cublas_handles[dst->src[0]->view_src ? dst->src[0]->view_src->res_ddid : 0],
        CUBLAS_OP_N,
        CUBLAS_OP_T,
        ne00, ne11, ne10,
        &alpha,
        a, ne00,
        b, ne10,
        &beta,
        c, ne00
    );
}

// Kernel launch for quantized ops
__global__ void quantize_q4_0_cuda(const float * x, void * y, int n) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        // Quantize in parallel
        quantize_block_q4_0(x + idx * 32, (block_q4_0 *)y + idx);
    }
}
```

### 6.4 Metal Backend (Apple Silicon)

```metal
// Metal shader for attention
kernel void attention_kernel(
    device const float * Q [[buffer(0)]],
    device const float * K [[buffer(1)]],
    device const float * V [[buffer(2)]],
    device float * output [[buffer(3)]],

    uint3 gid [[thread_position_in_grid]]
) {
    // Compute attention scores
    float score = 0;
    for (int i = 0; i < head_dim; i++) {
        score += Q[gid.z * head_dim + i] * K[gid.y * head_dim + i];
    }
    score *= rsqrt(head_dim);

    // Softmax and weighted sum
    // ...
}
```

---

## 7. Rust Translation Patterns

### 7.1 Type System Design

```rust
// GGML types as Rust enum
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgmlType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q2_K = 10,
    Q3_K = 11,
    Q4_K = 12,
    Q5_K = 13,
    Q6_K = 14,
}

impl GgmlType {
    pub fn type_size(&self) -> usize {
        match self {
            GgmlType::F32 => 4,
            GgmlType::F16 => 2,
            GgmlType::Q4_0 => 18,  // block_q4_0 size
            // ...
        }
    }

    pub fn blck_size(&self) -> usize {
        match self {
            GgmlType::F32 | GgmlType::F16 => 1,
            GgmlType::Q4_0 | GgmlType::Q8_0 => 32,
            GgmlType::Q2_K | GgmlType::Q3_K | GgmlType::Q4_K => 256,
            // ...
        }
    }

    pub fn bits_per_weight(&self) -> f32 {
        self.type_size() as f32 * 8.0 / self.blck_size() as f32
    }
}
```

### 7.2 Tensor Struct

```rust
use std::sync::Arc;

pub struct GgmlTensor {
    pub type_: GgmlType,
    pub ne: [u64; 4],  // Number of elements
    pub nb: [usize; 4], // Strides in bytes

    // Source tensors for operations
    pub src: [Option<Arc<GgmlTensor>>; GGML_MAX_SRC],

    // Operation
    pub op: GgmlOp,
    pub op_params: GgmlOpParams,

    // Data
    pub data: Arc<dyn GgmlBuffer>,
}

impl GgmlTensor {
    pub fn new_2d(
        ctx: &GgmlContext,
        type_: GgmlType,
        ne0: u64,
        ne1: u64,
    ) -> Self {
        let nb0 = type_.type_size();
        let nb1 = ne0 as usize * nb0;

        GgmlTensor {
            type_,
            ne: [ne0, ne1, 1, 1],
            nb: [nb0, nb1, 0, 0],
            src: Default::default(),
            op: GgmlOp::None,
            op_params: GgmlOpParams::default(),
            data: ctx.alloc(ne0 as usize * ne1 as usize * type_.type_size()),
        }
    }

    pub fn nbytes(&self) -> usize {
        let mut size = 1usize;
        for i in 0..4 {
            if self.ne[i] > 0 {
                size = size.saturating_mul(self.ne[i] as usize);
            }
        }
        // For quantized types, adjust for block size
        if self.type_.blck_size() > 1 {
            size / self.type_.blck_size() * self.type_.type_size()
        } else {
            size * self.type_.type_size()
        }
    }
}
```

### 7.3 Valtron Integration

```rust
use valtron::{TaskIterator, TaskStatus};

// Async-free tensor operation
pub struct TensorMulMat {
    a: Arc<GgmlTensor>,
    b: Arc<GgmlTensor>,
    result: Option<Arc<GgmlTensor>>,
    state: MulState,
}

enum MulState {
    Init,
    Computing,
    Done,
}

impl TaskIterator for TensorMulMat {
    type Ready = Arc<GgmlTensor>;
    type Pending = ();

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match self.state {
            MulState::Init => {
                self.state = MulState::Computing;
                Some(TaskStatus::Pending(()))
            }
            MulState::Computing => {
                let result = ggml_mul_mat(&self.a, &self.b);
                self.result = Some(result.clone());
                self.state = MulState::Done;
                Some(TaskStatus::Ready(result))
            }
            MulState::Done => None,
        }
    }
}

// Execute
let task = TensorMulMat {
    a: tensor_a,
    b: tensor_b,
    result: None,
    state: MulState::Init,
};

// Single-threaded execution
valtron::single::initialize_pool(seed);
let result = valtron::single::execute(task);
```

---

## Summary

### Key Takeaways

1. **GGML is a tensor library** with custom quantized types optimized for LLM inference
2. **Computation graphs** are built lazily and executed with threading
3. **GGUF is the file format** containing metadata and quantized tensors
4. **K-quants** (Q2_K through Q6_K) provide best quality/size trade-offs
5. **Multiple backends** (CPU, CUDA, Metal, Vulkan) share the same API
6. **Rust translation** requires careful ownership and type safety

### Next Steps

Continue to:
- [02-inference-optimization-deep-dive.md](02-inference-optimization-deep-dive.md) — KV caching and sampling
- [rust-revision.md](rust-revision.md) — Complete Rust translation guide

---

*This document complements the official GGML documentation. Refer to the source code for authoritative implementation details.*
