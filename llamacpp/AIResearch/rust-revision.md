---
title: "llama.cpp Rust Revision: Complete Translation Guide"
subtitle: "From C/C++ to Rust with ggml-rs and valtron executor"
based_on: "llama.cpp source + whisper-rs patterns"
level: "Advanced - Requires understanding of all previous documents"
---

# llama.cpp Rust Revision

## Table of Contents

1. [Overview and Design Goals](#1-overview-and-design-goals)
2. [Type System Design](#2-type-system-design)
3. [GGML Tensor Operations in Rust](#3-ggml-tensor-operations-in-rust)
4. [Model Architecture Translation](#4-model-architecture-translation)
5. [KV Cache Management](#5-kv-cache-management)
6. [Inference Pipeline with Valtron](#6-inference-pipeline-with-valtron)
7. [Memory Management](#7-memory-management)
8. [Backend Integration](#8-backend-integration)
9. [Complete Example: LLaMA Inference](#9-complete-example-llama-inference)

---

## 1. Overview and Design Goals

### 1.1 What We're Translating

| llama.cpp Component | Rust Equivalent |
|--------------------|-----------------|
| `ggml_tensor` | `GgmlTensor` struct with Arc |
| `ggml_context` | `GgmlContext` arena allocator |
| `ggml_cgraph` | `ComputeGraph` builder |
| `llama_model` | `LlamaModel` with Arc tensors |
| `llama_context` | `LlamaContext` with KV cache |
| `llama_sampler` | `Sampler` trait + implementations |
| CUDA/Metal backends | `Backend` trait + implementations |

### 1.2 Design Principles

```rust
// 1. Zero-cost abstractions where possible
// 2. Safe by default, unsafe when needed (FFI)
// 3. Arc-based sharing for tensors
// 4. Valtron for async-free execution
// 5. Trait-based backend abstraction

// Key differences from C++:
// - No raw pointers (use Arc, Box, references)
// - No manual memory management (arena allocators)
// - Explicit error handling (Result<T, GgmlError>)
// - Type-safe quantization (enum GgmlType)
```

### 1.3 Crate Structure

```
llama-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── ggml/
│   │   ├── mod.rs          # GGML types
│   │   ├── tensor.rs       # GgmlTensor
│   │   ├── context.rs      # GgmlContext
│   │   ├── graph.rs        # ComputeGraph
│   │   └── ops.rs          # Tensor operations
│   ├── model/
│   │   ├── mod.rs          # Model loading
│   │   ├── loader.rs       # GGUF parser
│   │   ├── architecture.rs # Arch-specific logic
│   │   └── weights.rs      # Weight tensors
│   ├── inference/
│   │   ├── mod.rs          # Inference API
│   │   ├── kv_cache.rs     # KV cache management
│   │   ├── batch.rs        # Batch processing
│   │   └── logits.rs       # Logits processing
│   ├── sampling/
│   │   ├── mod.rs          # Sampling API
│   │   ├── sampler.rs      # Sampler trait
│   │   ├── temperature.rs  # Temperature sampling
│   │   ├── top_k.rs        # Top-K sampling
│   │   ├── top_p.rs        # Top-P sampling
│   │   └── grammar.rs      # Grammar constraints
│   ├── backends/
│   │   ├── mod.rs          # Backend trait
│   │   ├── cpu.rs          # CPU backend
│   │   ├── cuda.rs         # CUDA backend (optional)
│   │   └── metal.rs        # Metal backend (optional)
│   └── valtron/
│       ├── mod.rs          # Valtron integration
│       ├── forward.rs      # Forward pass task
│       └── sampling.rs     # Sampling task
│
└── examples/
    ├── simple.rs           # Basic inference
    ├── server.rs           # HTTP server
    └── chat.rs             # Chat interface
```

---

## 2. Type System Design

### 2.1 GGML Types

```rust
/// GGML data types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GgmlType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q5_0 = 6,
    Q5_1 = 7,
    Q8_0 = 8,
    Q8_1 = 9,
    Q2_K = 10,
    Q3_K = 11,
    Q4_K = 12,
    Q5_K = 13,
    Q6_K = 14,
    Q8_K = 15,
    IQ2_XXS = 16,
    IQ2_XS = 17,
    IQ3_XXS = 18,
    IQ1_S = 19,
    IQ4_NL = 20,
    IQ4_XS = 21,
    I8 = 22,
    I16 = 23,
    I32 = 24,
    I64 = 25,
    F64 = 26,
    BF16 = 27,
}

impl GgmlType {
    /// Size of one block in bytes
    pub const fn type_size(&self) -> usize {
        match self {
            GgmlType::F32 | GgmlType::I32 => 4,
            GgmlType::F16 | GgmlType::I16 => 2,
            GgmlType::F64 | GgmlType::I64 => 8,
            GgmlType::Q4_0 => 18,  // block_q4_0
            GgmlType::Q4_1 => 20,  // block_q4_1
            GgmlType::Q8_0 => 34,  // block_q8_0
            GgmlType::Q2_K => 28 + 32 + 4 + 2,  // block_q2_K
            GgmlType::Q3_K => 12 + 64 + 2 + 2,  // block_q3_K
            GgmlType::Q4_K => 2 + 2 + 12 + 64 + 2,  // block_q4_K
            GgmlType::Q5_K => 2 + 2 + 12 + 64 + 2,  // block_q5_K
            GgmlType::Q6_K => 2 * 4 + 16 + 128 + 2,  // block_q6_K
            _ => unimplemented!(),
        }
    }

    /// Number of weights per block
    pub const fn blck_size(&self) -> usize {
        match self {
            GgmlType::F32 | GgmlType::F16 | GgmlType::BF16 => 1,
            GgmlType::Q4_0 | GgmlType::Q4_1 | GgmlType::Q8_0 | GgmlType::Q8_1 => 32,
            GgmlType::Q2_K | GgmlType::Q3_K | GgmlType::Q4_K | GgmlType::Q5_K | GgmlType::Q6_K => 256,
            _ => 1,
        }
    }

    /// Bits per weight
    pub fn bits_per_weight(&self) -> f32 {
        self.type_size() as f32 * 8.0 / self.blck_size() as f32
    }

    /// Check if quantized
    pub fn is_quantized(&self) -> bool {
        matches!(self,
            GgmlType::Q4_0 | GgmlType::Q4_1 |
            GgmlType::Q5_0 | GgmlType::Q5_1 |
            GgmlType::Q8_0 | GgmlType::Q8_1 |
            GgmlType::Q2_K | GgmlType::Q3_K |
            GgmlType::Q4_K | GgmlType::Q5_K | GgmlType::Q6_K |
            GgmlType::IQ2_XXS | GgmlType::IQ2_XS |
            GgmlType::IQ3_XXS | GgmlType::IQ1_S |
            GgmlType::IQ4_NL | GgmlType::IQ4_XS
        )
    }
}
```

### 2.2 Tensor Structure

```rust
use std::sync::Arc;

const GGML_MAX_DIMS: usize = 4;
const GGML_MAX_SRC: usize = 10;

/// GGML tensor
#[derive(Debug, Clone)]
pub struct GgmlTensor {
    /// Data type
    pub type_: GgmlType,

    /// Number of elements in each dimension
    pub ne: [u64; GGML_MAX_DIMS],

    /// Stride in bytes for each dimension
    pub nb: [usize; GGML_MAX_DIMS],

    /// Source tensors (for operations)
    pub src: [Option<Arc<GgmlTensor>>; GGML_MAX_SRC],

    /// Operation that produced this tensor
    pub op: GgmlOp,

    /// Operation parameters
    pub op_params: GgmlOpParams,

    /// Tensor data
    pub data: Arc<dyn GgmlBuffer>,

    /// Tensor name (for debugging)
    pub name: Option<String>,
}

impl GgmlTensor {
    /// Create a new 1D tensor
    pub fn new_1d(
        type_: GgmlType,
        ne0: u64,
        data: Arc<dyn GgmlBuffer>,
    ) -> Self {
        let nb0 = type_.type_size();
        Self {
            type_,
            ne: [ne0, 1, 1, 1],
            nb: [nb0, 0, 0, 0],
            src: Default::default(),
            op: GgmlOp::None,
            op_params: GgmlOpParams::default(),
            data,
            name: None,
        }
    }

    /// Create a new 2D tensor
    pub fn new_2d(
        type_: GgmlType,
        ne0: u64,
        ne1: u64,
        data: Arc<dyn GgmlBuffer>,
    ) -> Self {
        let nb0 = type_.type_size();
        let nb1 = ne0 as usize * nb0;
        Self {
            type_,
            ne: [ne0, ne1, 1, 1],
            nb: [nb0, nb1, 0, 0],
            src: Default::default(),
            op: GgmlOp::None,
            op_params: GgmlOpParams::default(),
            data,
            name: None,
        }
    }

    /// Total number of elements
    pub fn n_elements(&self) -> usize {
        self.ne.iter()
            .filter(|&&n| n > 0)
            .product::<u64>() as usize
    }

    /// Total size in bytes
    pub fn nbytes(&self) -> usize {
        let n = self.n_elements();
        if self.type_.blck_size() > 1 {
            n / self.type_.blck_size() * self.type_.type_size()
        } else {
            n * self.type_.type_size()
        }
    }

    /// Get dimension
    pub fn dim(&self, i: usize) -> u64 {
        self.ne.get(i).copied().unwrap_or(1)
    }

    /// Get stride
    pub fn stride(&self, i: usize) -> usize {
        self.nb.get(i).copied().unwrap_or(0)
    }
}
```

### 2.3 Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GgmlError {
    #[error("Out of memory: needed {needed} bytes, have {available} bytes")]
    OutOfMemory { needed: usize, available: usize },

    #[error("Invalid tensor shape: {0}")]
    InvalidShape(String),

    #[error("Invalid quantization: {0}")]
    InvalidQuantization(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("GGUF parse error: {0}")]
    GgufParseError(String),

    #[error("Model loading error: {0}")]
    ModelLoadError(String),

    #[error("Inference error: {0}")]
    InferenceError(String),

    #[error("Sampling error: {0}")]
    SamplingError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GgmlError>;
```

---

## 3. GGML Tensor Operations in Rust

### 3.1 Operation Enum

```rust
/// GGML operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgmlOp {
    None,
    Add,
    Sub,
    Mul,
    Div,
    Sqr,
    Sqrt,
    Log,
    Exp,
    Sin,
    Cos,
    Tanh,
    Silu,
    Gelu,
    GeluQuick,
    Relu,
    LeakyRelu,
    SoftMax,
    SoftMaxBack,
    Rope,
    RopeBack,
    RmsNorm,
    RmsNormBack,
    MulMat,
    MulMatId,
    Scale,
    Set,
    Copy,
    Cont,
    Reshape,
    Permute,
    Transpose,
    GetRows,
    GetRowsBack,
    Diag,
    DiagMaskInf,
    Sum,
    SumRows,
    Mean,
    Repeat,
    RepeatBack,
    Abs,
    Sign,
    Neg,
    Ceil,
    Floor,
    Round,
    Clamp,
    Im2Col,
    Conv2d,
    ConvTranspose2d,
    Pool2d,
    Upscale,
    FlashAttn,
    FlashAttnBack,
    SsmConv,
    SsmScan,
    Mamba,
    RwkvWkv6,
    RwkvWkv7,
}
```

### 3.2 Operation Implementations

```rust
/// Matrix multiplication: dst = src0 @ src1
pub fn ggml_mul_mat(
    ctx: &GgmlContext,
    src0: &Arc<GgmlTensor>,
    src1: &Arc<GgmlTensor>,
) -> Arc<GgmlTensor> {
    assert_eq!(src0.ne[0], src1.ne[0], "Dimension mismatch");

    let ne0 = src0.ne[1];  // Output rows
    let ne1 = src1.ne[1];  // Output cols

    let mut dst = ctx.alloc_tensor_2d(
        GgmlType::F32,
        ne0,
        ne1,
    );

    dst.op = GgmlOp::MulMat;
    dst.src[0] = Some(src0.clone());
    dst.src[1] = Some(src1.clone());

    dst
}

/// RMSNorm: y = x / rms * weight
pub fn ggml_rms_norm(
    ctx: &GgmlContext,
    src: &Arc<GgmlTensor>,
    eps: f32,
) -> Arc<GgmlTensor> {
    let mut dst = ctx.alloc_tensor_like(src, GgmlType::F32);

    dst.op = GgmlOp::RmsNorm;
    dst.op_params.rms_norm.eps = eps;
    dst.src[0] = Some(src.clone());

    dst
}

/// RoPE: Rotary position embeddings
pub fn ggml_rope(
    ctx: &GgmlContext,
    src: &Arc<GgmlTensor>,
    positions: &Arc<GgmlTensor>,
    n_dims: u32,
    mode: RopeMode,
    n_ctx: u32,
    freq_base: f32,
    freq_scale: f32,
) -> Arc<GgmlTensor> {
    let mut dst = ctx.alloc_tensor_like(src, src.type_);

    dst.op = GgmlOp::Rope;
    dst.op_params.rope = RopeParams {
        n_dims,
        mode,
        n_ctx,
        freq_base,
        freq_scale,
    };
    dst.src[0] = Some(src.clone());
    dst.src[1] = Some(positions.clone());

    dst
}

/// SiLU activation: silu(x) = x * sigmoid(x)
pub fn ggml_silu(
    ctx: &GgmlContext,
    src: &Arc<GgmlTensor>,
) -> Arc<GgmlTensor> {
    let mut dst = ctx.alloc_tensor_like(src, src.type_);

    dst.op = GgmlOp::Silu;
    dst.src[0] = Some(src.clone());

    dst
}

/// Linear transformation: y = x @ weight
pub fn ggml_linear(
    ctx: &GgmlContext,
    src: &Arc<GgmlTensor>,
    weight: &Arc<GgmlTensor>,
) -> Arc<GgmlTensor> {
    let result = ggml_mul_mat(ctx, weight, src);
    result
}
```

### 3.3 Compute Graph

```rust
/// Compute graph
pub struct ComputeGraph {
    /// Graph nodes (in compute order)
    nodes: Vec<Arc<GgmlTensor>>,

    /// Leaf tensors (inputs)
    leafs: Vec<Arc<GgmlTensor>>,

    /// Number of threads
    n_threads: u32,
}

impl ComputeGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            leafs: Vec::new(),
            n_threads: num_cpus::get() as u32,
        }
    }

    /// Add a tensor to the graph
    pub fn add_tensor(&mut self, tensor: Arc<GgmlTensor>) {
        // Add leafs first
        for src in &tensor.src {
            if let Some(src) = src {
                if !self.leafs.iter().any(|t| Arc::ptr_eq(t, src)) {
                    if src.op == GgmlOp::None {
                        self.leafs.push(src.clone());
                    }
                }
            }
        }

        // Add node
        if !self.nodes.iter().any(|t| Arc::ptr_eq(t, &tensor)) {
            self.nodes.push(tensor);
        }
    }

    /// Build graph from output tensor (backwards traversal)
    pub fn build_from_output(output: &Arc<GgmlTensor>) -> Self {
        let mut graph = Self::new();
        graph.build_recursive(output);
        graph
    }

    fn build_recursive(&mut self, tensor: &Arc<GgmlTensor>) {
        // Add sources first (depth-first)
        for src in &tensor.src {
            if let Some(src) = src {
                self.build_recursive(src);
            }
        }

        // Add this tensor
        self.add_tensor(tensor.clone());
    }

    /// Execute the graph
    pub fn compute(&self, backend: &dyn Backend) -> Result<()> {
        backend.graph_compute(self)
    }
}
```

---

## 4. Model Architecture Translation

### 4.1 Model Loading from GGUF

```rust
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

/// GGUF file loader
pub struct GgufLoader {
    reader: BufReader<File>,
}

impl GgufLoader {
    pub fn new(path: &str) -> Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }

    pub fn load_model(&mut self) -> Result<LlamaModel> {
        // Read header
        let header = self.read_header()?;

        // Read metadata
        let metadata = self.read_metadata(header.kv_count)?;

        // Read tensor info
        let tensor_infos = self.read_tensor_infos(header.tensor_count)?;

        // Load tensors
        let mut tensors = HashMap::new();
        for info in tensor_infos {
            let tensor = self.load_tensor(&info)?;
            tensors.insert(info.name, tensor);
        }

        // Build model
        LlamaModel::from_tensors(&metadata, tensors)
    }

    fn read_header(&mut self) -> Result<GgufHeader> {
        let mut magic = [0u8; 4];
        self.reader.read_exact(&mut magic)?;

        if &magic != b"GGUF" {
            return Err(GgmlError::GgufParseError("Invalid GGUF magic".into()));
        }

        let version = self.read_u32()?;
        let tensor_count = self.read_u64()?;
        let kv_count = self.read_u64()?;

        Ok(GgufHeader {
            magic,
            version,
            tensor_count,
            kv_count,
        })
    }

    fn load_tensor(&mut self, info: &TensorInfo) -> Result<Arc<GgmlTensor>> {
        // Seek to tensor data
        self.reader.seek(SeekFrom::Start(info.offset))?;

        // Allocate buffer
        let size = info.nbytes();
        let mut buffer = vec![0u8; size];
        self.reader.read_exact(&mut buffer)?;

        // Create tensor
        let data = Arc::new(CpuBuffer::new(buffer));
        let tensor = GgmlTensor::new_from_info(info, data);

        Ok(tensor)
    }
}
```

### 4.2 LLaMA Model Structure

```rust
/// LLaMA model
pub struct LlamaModel {
    /// Model hyperparameters
    pub hparams: LlamaHparams,

    /// Token embeddings
    pub token_embeddings: Arc<GgmlTensor>,

    /// Transformer layers
    pub layers: Vec<TransformerBlock>,

    /// Output normalization
    pub output_norm: Arc<GgmlTensor>,

    /// Output weights (optional, may be tied with embeddings)
    pub output: Option<Arc<GgmlTensor>>,

    /// Vocabulary
    pub vocab: Vocabulary,
}

/// Transformer block
pub struct TransformerBlock {
    /// Attention normalization
    pub attn_norm: Arc<GgmlTensor>,

    /// Query projection
    pub wq: Arc<GgmlTensor>,

    /// Key projection
    pub wk: Arc<GgmlTensor>,

    /// Value projection
    pub wv: Arc<GgmlTensor>,

    /// Output projection
    pub wo: Arc<GgmlTensor>,

    /// FFN normalization
    pub ffn_norm: Arc<GgmlTensor>,

    /// FFN gate projection (SwiGLU)
    pub w_gate: Arc<GgmlTensor>,

    /// FFN up projection (SwiGLU)
    pub w_up: Arc<GgmlTensor>,

    /// FFN down projection (SwiGLU)
    pub w_down: Arc<GgmlTensor>,

    /// MoE gate (optional)
    pub moe_gate: Option<Arc<GgmlTensor>>,

    /// MoE experts (optional)
    pub moe_experts: Option<Vec<MoeExpert>>,
}

struct MoeExpert {
    w_gate: Arc<GgmlTensor>,
    w_up: Arc<GgmlTensor>,
    w_down: Arc<GgmlTensor>,
}

impl LlamaModel {
    pub fn from_tensors(
        metadata: &GgmlMetadata,
        tensors: HashMap<String, Arc<GgmlTensor>>,
    ) -> Result<Self> {
        // Parse hyperparameters from metadata
        let hparams = LlamaHparams::from_metadata(metadata)?;

        // Extract token embeddings
        let token_embeddings = tensors
            .get("token_embd.weight")
            .ok_or_else(|| GgmlError::ModelLoadError("Missing token embeddings".into()))?
            .clone();

        // Extract layers
        let mut layers = Vec::with_capacity(hparams.n_layer as usize);
        for i in 0..hparams.n_layer {
            let layer = TransformerBlock::from_tensors(&tensors, i)?;
            layers.push(layer);
        }

        // Extract output norm
        let output_norm = tensors
            .get("output_norm.weight")
            .or_else(|| tensors.get("norm.weight"))
            .ok_or_else(|| GgmlError::ModelLoadError("Missing output norm".into()))?
            .clone();

        // Extract output weights (may be tied)
        let output = tensors.get("output.weight").cloned();

        // Extract vocabulary
        let vocab = Vocabulary::from_metadata(metadata)?;

        Ok(Self {
            hparams,
            token_embeddings,
            layers,
            output_norm,
            output,
            vocab,
        })
    }

    pub fn n_vocab(&self) -> usize {
        self.hparams.n_vocab as usize
    }

    pub fn n_embd(&self) -> usize {
        self.hparams.n_embd as usize
    }

    pub fn n_layer(&self) -> usize {
        self.hparams.n_layer as usize
    }
}
```

---

## 5. KV Cache Management

### 5.1 KV Cache Structure

```rust
/// KV Cache
pub struct KvCache {
    /// Per-layer KV tensors
    layers: Vec<KvLayer>,

    /// Cell metadata
    cells: Vec<KvCell>,

    /// Configuration
    n_ctx: u32,
    n_seq_max: u32,

    /// Sliding window (if applicable)
    window_size: Option<u32>,
}

struct KvLayer {
    /// Key cache: [n_tokens, n_kv_heads, head_dim]
    k_cache: GgmlTensor,

    /// Value cache: [n_tokens, n_kv_heads, head_dim]
    v_cache: GgmlTensor,
}

struct KvCell {
    pos: i32,
    seq_id: i32,
}

impl KvCache {
    pub fn new(
        n_layers: u32,
        n_ctx: u32,
        n_embd: u32,
        n_head_kv: u32,
        head_dim: u32,
        type_k: GgmlType,
        type_v: GgmlType,
    ) -> Self {
        let mut layers = Vec::with_capacity(n_layers as usize);

        for _ in 0..n_layers {
            let k_size = (n_ctx * n_head_kv * head_dim) as usize;
            let v_size = (n_ctx * n_head_kv * head_dim) as usize;

            layers.push(KvLayer {
                k_cache: GgmlTensor::new_1d(type_k, k_size as u64, ...),
                v_cache: GgmlTensor::new_1d(type_v, v_size as u64, ...),
            });
        }

        Self {
            layers,
            cells: vec![KvCell { pos: -1, seq_id: -1 }; n_ctx as usize],
            n_ctx,
            n_seq_max: 1,
            window_size: None,
        }
    }

    /// Remove tokens from a sequence
    pub fn seq_rm(&mut self, seq_id: i32, p0: i32, p1: i32) {
        for cell in &mut self.cells {
            if cell.seq_id == seq_id && cell.pos >= p0 && cell.pos < p1 {
                cell.seq_id = -1;  // Free
            }
        }
    }

    /// Copy sequence
    pub fn seq_cp(&mut self, seq_id_src: i32, seq_id_dst: i32, p0: i32, p1: i32) {
        for cell in &mut self.cells {
            if cell.seq_id == seq_id_src && cell.pos >= p0 && cell.pos < p1 {
                cell.seq_id = seq_id_dst;
            }
        }
    }

    /// Keep only specified sequence
    pub fn seq_keep(&mut self, seq_id: i32) {
        for cell in &mut self.cells {
            if cell.seq_id != seq_id {
                cell.seq_id = -1;
            }
        }
    }

    /// Get position for new token
    pub fn alloc_pos(&mut self, seq_id: i32) -> Option<i32> {
        // Find free cell
        for (i, cell) in self.cells.iter_mut().enumerate() {
            if cell.seq_id == -1 {
                cell.pos = i as i32;
                cell.seq_id = seq_id;
                return Some(i as i32);
            }
        }
        None  // Cache full
    }
}
```

---

## 6. Inference Pipeline with Valtron

### 6.1 Forward Pass Task

```rust
use valtron::{TaskIterator, TaskStatus, NoSpawner};

/// LLM forward pass task
pub struct LlamaForward {
    model: Arc<LlamaModel>,
    tokens: Vec<TokenId>,
    positions: Vec<i32>,
    kv_cache: Arc<Mutex<KvCache>>,
    state: ForwardState,
    current_layer: usize,
    hidden: Option<Arc<GgmlTensor>>,
}

enum ForwardState {
    Embedding,
    ProcessingLayers,
    FinalNorm,
    Logits,
    Done,
}

impl TaskIterator for LlamaForward {
    type Ready = Vec<f32>;  // Logits
    type Pending = ForwardProgress;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            ForwardState::Embedding => {
                // Look up token embeddings
                let mut hidden = self.model.token_embeddings.clone();

                // Sum embeddings for all tokens
                // (simplified - actual implementation uses ggml operations)
                self.hidden = Some(hidden);
                self.state = ForwardState::ProcessingLayers;
                self.current_layer = 0;

                Some(TaskStatus::Pending(ForwardProgress::Embedding))
            }

            ForwardState::ProcessingLayers => {
                if self.current_layer >= self.model.layers.len() {
                    self.state = ForwardState::FinalNorm;
                    return Some(TaskStatus::Pending(ForwardProgress::LayerNorm));
                }

                // Process one layer
                let layer = &self.model.layers[self.current_layer];
                let hidden = self.hidden.as_ref().unwrap();
                let mut kv_cache = self.kv_cache.lock().unwrap();

                // Attention
                let normed = rms_norm(hidden, &layer.attn_norm);
                let q = linear(&normed, &layer.wq);
                let k = linear(&normed, &layer.wk);
                let v = linear(&normed, &layer.wv);

                // Apply RoPE
                let q = apply_rope(q, &self.positions);
                let k = apply_rope(k, &self.positions);

                // Update KV cache
                kv_cache.update(self.current_layer, &k, &v, &self.positions);

                // Compute attention
                let attn = grouped_query_attention(
                    &q,
                    &kv_cache.layers[self.current_layer].k_cache,
                    &kv_cache.layers[self.current_layer].v_cache,
                );
                let attn_out = linear(&attn, &layer.wo);

                // Residual
                let mut hidden = add(hidden, &attn_out);

                // FFN
                let normed = rms_norm(&hidden, &layer.ffn_norm);
                let gate = linear(&normed, &layer.w_gate);
                let up = linear(&normed, &layer.w_up);
                let gate = silu(gate);
                let ffn_in = mul(&gate, &up);
                let ffn_out = linear(&ffn_in, &layer.w_down);

                // Final residual
                hidden = add(&hidden, &ffn_out);

                self.hidden = Some(hidden);
                self.current_layer += 1;

                Some(TaskStatus::Pending(ForwardProgress::Layer(self.current_layer as u32 - 1)))
            }

            ForwardState::FinalNorm => {
                // Apply output norm
                let hidden = rms_norm(
                    self.hidden.as_ref().unwrap(),
                    &self.model.output_norm,
                );
                self.hidden = Some(hidden);
                self.state = ForwardState::Logits;

                Some(TaskStatus::Pending(ForwardProgress::OutputNorm))
            }

            ForwardState::Logits => {
                // Compute logits (last token only)
                let hidden = &self.hidden.as_ref().unwrap();
                let logits = if let Some(output) = &self.model.output {
                    linear(hidden, output)
                } else {
                    // Tied embeddings
                    linear(hidden, &self.model.token_embeddings)
                };

                // Get logits for last position
                let logits = get_last_token_logits(&logits);

                self.state = ForwardState::Done;

                Some(TaskStatus::Ready(logits))
            }

            ForwardState::Done => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ForwardProgress {
    Embedding,
    Layer(u32),
    LayerNorm,
    OutputNorm,
}
```

### 6.2 Sampling Task

```rust
/// Sampling task
pub struct SamplingTask {
    logits: Vec<f32>,
    temperature: f32,
    top_k: i32,
    top_p: f32,
    penalty_last_n: usize,
    penalty_repeat: f32,
    grammar: Option<Arc<Grammar>>,
    state: SamplingState,
}

enum SamplingState {
    Init,
    Temperature,
    TopK,
    TopP,
    Penalty,
    Grammar,
    Sample,
    Done,
}

impl TaskIterator for SamplingTask {
    type Ready = TokenId;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            SamplingState::Init => {
                // Apply temperature
                apply_temperature(&mut self.logits, self.temperature);
                self.state = SamplingState::TopK;
                Some(TaskStatus::Pending(()))
            }

            SamplingState::TopK => {
                if self.top_k > 0 {
                    apply_top_k(&mut self.logits, self.top_k);
                }
                self.state = SamplingState::TopP;
                Some(TaskStatus::Pending(()))
            }

            SamplingState::TopP => {
                if self.top_p > 0.0 && self.top_p < 1.0 {
                    apply_top_p(&mut self.logits, self.top_p);
                }
                self.state = SamplingState::Penalty;
                Some(TaskStatus::Pending(()))
            }

            SamplingState::Penalty => {
                if self.penalty_last_n > 0 {
                    apply_penalty(
                        &mut self.logits,
                        self.penalty_last_n,
                        self.penalty_repeat,
                    );
                }
                self.state = SamplingState::Grammar;
                Some(TaskStatus::Pending(()))
            }

            SamplingState::Grammar => {
                if let Some(grammar) = &self.grammar {
                    apply_grammar(&mut self.logits, grammar);
                }
                self.state = SamplingState::Sample;
                Some(TaskStatus::Pending(()))
            }

            SamplingState::Sample => {
                let token = sample_from_logits(&self.logits);
                self.state = SamplingState::Done;
                Some(TaskStatus::Ready(token))
            }

            SamplingState::Done => None,
        }
    }
}
```

### 6.3 Complete Inference Loop

```rust
/// Complete inference executor
pub struct TextGenerator {
    model: Arc<LlamaModel>,
    kv_cache: Arc<Mutex<KvCache>>,
    max_tokens: usize,
    temperature: f32,
    top_k: i32,
    top_p: f32,
}

impl TextGenerator {
    pub fn generate(
        &self,
        prompt: &str,
    ) -> Result<GenerationOutput> {
        // Tokenize prompt
        let tokens = self.model.vocab.encode(prompt)?;

        // Initialize valtron executor
        valtron::single::initialize_pool(random_seed());

        // Create forward task
        let forward = LlamaForward {
            model: self.model.clone(),
            tokens: tokens.clone(),
            positions: (0..tokens.len() as i32).collect(),
            kv_cache: self.kv_cache.clone(),
            state: ForwardState::Embedding,
            current_layer: 0,
            hidden: None,
        };

        // Execute forward pass
        let mut logits = valtron::single::execute(forward)?;

        // Generate tokens
        let mut generated = Vec::new();
        let mut all_tokens = tokens;

        for i in 0..self.max_tokens {
            // Create sampling task
            let sample = SamplingTask {
                logits: logits.clone(),
                temperature: self.temperature,
                top_k: self.top_k,
                top_p: self.top_p,
                penalty_last_n: 64,
                penalty_repeat: 1.0,
                grammar: None,
                state: SamplingState::Init,
            };

            // Sample token
            let next_token = valtron::single::execute(sample)?;

            // Check for EOS
            if self.model.vocab.is_eos(next_token) {
                break;
            }

            generated.push(next_token);
            all_tokens.push(next_token);

            // Forward pass for single token
            let forward = LlamaForward {
                model: self.model.clone(),
                tokens: vec![next_token],
                positions: vec![(all_tokens.len() - 1) as i32],
                kv_cache: self.kv_cache.clone(),
                state: ForwardState::Embedding,
                current_layer: 0,
                hidden: None,
            };

            logits = valtron::single::execute(forward)?;
        }

        // Decode output
        let text = self.model.vocab.decode(&generated)?;

        Ok(GenerationOutput {
            tokens: generated,
            text,
            timings: GenerationTimings::default(),
        })
    }
}
```

---

## 7. Memory Management

### 7.1 Arena Allocator

```rust
/// GGML context (arena allocator)
pub struct GgmlContext {
    buffer: Vec<u8>,
    offset: usize,
    tensors: Vec<Arc<GgmlTensor>>,
    alignment: usize,
}

impl GgmlContext {
    pub fn new(size: usize) -> Self {
        Self {
            buffer: vec![0u8; size],
            offset: 0,
            tensors: Vec::new(),
            alignment: 16,  // GGML_MEM_ALIGN
        }
    }

    pub fn alloc(&mut self, size: usize) -> &mut [u8] {
        // Align offset
        let aligned = (self.offset + self.alignment - 1) & !(self.alignment - 1);

        if aligned + size > self.buffer.len() {
            panic!("Out of memory: needed {} bytes", size);
        }

        let slice = &mut self.buffer[aligned..aligned + size];
        self.offset = aligned + size;
        slice
    }

    pub fn alloc_tensor(&mut self, type_: GgmlType, ne: [u64; 4]) -> Arc<GgmlTensor> {
        // Calculate size
        let n_elements: usize = ne.iter()
            .filter(|&&n| n > 0)
            .map(|&n| n as usize)
            .product();

        let size = if type_.blck_size() > 1 {
            n_elements / type_.blck_size() * type_.type_size()
        } else {
            n_elements * type_.type_size()
        };

        // Allocate data
        let data = self.alloc(size);
        let buffer = Arc::new(SliceBuffer::new(data));

        // Create tensor
        let tensor = GgmlTensor::new_with_ne(type_, ne, buffer);
        let tensor = Arc::new(tensor);

        self.tensors.push(tensor.clone());
        tensor
    }
}
```

---

## 8. Backend Integration

### 8.1 Backend Trait

```rust
/// Backend trait for compute execution
pub trait Backend: Send + Sync {
    /// Backend name
    fn name(&self) -> &str;

    /// Check if backend supports an operation
    fn supports_op(&self, op: GgmlOp) -> bool;

    /// Check if backend supports a buffer type
    fn supports_buft(&self, buft: BufferType) -> bool;

    /// Execute a compute graph
    fn graph_compute(&self, graph: &ComputeGraph) -> Result<()>;

    /// Set number of threads
    fn set_n_threads(&mut self, n_threads: u32);
}
```

### 8.2 CPU Backend

```rust
/// CPU backend
pub struct CpuBackend {
    n_threads: u32,
}

impl CpuBackend {
    pub fn new() -> Self {
        Self {
            n_threads: num_cpus::get() as u32,
        }
    }
}

impl Backend for CpuBackend {
    fn name(&self) -> &str {
        "CPU"
    }

    fn supports_op(&self, op: GgmlOp) -> bool {
        // CPU supports all operations
        true
    }

    fn supports_buft(&self, buft: BufferType) -> bool {
        matches!(buft, BufferType::Cpu)
    }

    fn graph_compute(&self, graph: &ComputeGraph) -> Result<()> {
        // Execute nodes in order
        for node in &graph.nodes {
            self.compute_node(node)?;
        }
        Ok(())
    }

    fn set_n_threads(&mut self, n_threads: u32) {
        self.n_threads = n_threads;
    }
}

impl CpuBackend {
    fn compute_node(&self, node: &GgmlTensor) -> Result<()> {
        match node.op {
            GgmlOp::MulMat => self.compute_mul_mat(node),
            GgmlOp::RmsNorm => self.compute_rms_norm(node),
            GgmlOp::Rope => self.compute_rope(node),
            GgmlOp::Silu => self.compute_silu(node),
            // ... handle all operations
            _ => Err(GgmlError::BackendError(format!(
                "Unsupported operation: {:?}",
                node.op
            ))),
        }
    }

    fn compute_mul_mat(&self, dst: &GgmlTensor) -> Result<()> {
        let src0 = dst.src[0].as_ref().unwrap();
        let src1 = dst.src[1].as_ref().unwrap();

        // BLAS-style matrix multiplication
        // (simplified - actual implementation uses proper strides)

        let data0 = src0.data.as_slice();
        let data1 = src1.data.as_slice();
        let mut data_dst = dst.data.as_slice_mut();

        let m = src0.dim(1) as usize;
        let n = src1.dim(1) as usize;
        let k = src0.dim(0) as usize;

        // Simple O(mnk) implementation
        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0f32;
                for l in 0..k {
                    sum += data0[i * k + l] * data1[l * n + j];
                }
                data_dst[i * n + j] = sum;
            }
        }

        Ok(())
    }
}
```

---

## 9. Complete Example: LLaMA Inference

### 9.1 Full Example

```rust
use llama_rs::{LlamaModel, TextGenerator, GgmlType};
use valtron;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load model
    let mut loader = GgufLoader::new("models/llama-3.2-1b-instruct.q4_k_m.gguf")?;
    let model = Arc::new(loader.load_model()?);

    println!("Loaded model: {} parameters", model.hparams.n_params());

    // Create KV cache
    let kv_cache = Arc::new(Mutex::new(KvCache::new(
        model.n_layer() as u32,
        4096,  // context size
        model.n_embd() as u32,
        model.hparams.n_head_kv,
        model.hparams.head_dim(),
        GgmlType::F16,  // KV cache type
        GgmlType::F16,
    )));

    // Create generator
    let generator = TextGenerator {
        model: model.clone(),
        kv_cache: kv_cache.clone(),
        max_tokens: 256,
        temperature: 0.7,
        top_k: 40,
        top_p: 0.9,
    };

    // Generate
    let prompt = "Hello, my name is";
    println!("Prompt: {}", prompt);

    let output = generator.generate(prompt)?;

    println!("Generated: {}", output.text);
    println!("Tokens/second: {}", output.timings.tps());

    Ok(())
}
```

### 9.2 HTTP Server Example

```rust
// Using basic HTTP (no tokio/async)
use std::net::TcpListener;
use std::io::{Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Arc::new(load_model("models/llama.gguf")?);
    let listener = TcpListener::bind("127.0.0.1:8080")?;

    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let model = model.clone();

        // Handle request (blocking)
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            let n = stream.read(&mut buffer).unwrap();

            // Parse request
            let request = String::from_utf8_lossy(&buffer[..n]);
            let prompt = extract_prompt(&request);

            // Generate response
            let generator = TextGenerator::new(model);
            let output = generator.generate(&prompt).unwrap();

            // Send response
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"text\":\"{}\"}}",
                output.text.replace('"', "\\\"")
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
    }

    Ok(())
}
```

---

## Summary

### Key Takeaways

1. **Arc-based tensor sharing** replaces raw pointers
2. **Valtron TaskIterator** replaces async/await
3. **Trait-based backends** enable CPU/GPU/Metal support
4. **GGUF parsing** loads quantized weights
5. **Arena allocation** manages tensor memory efficiently
6. **Type-safe enums** for operations and quantization types

### Next Steps

Continue to:
- [production-grade.md](production-grade.md) — Production deployment guide
- [05-valtron-integration.md](05-valtron-integration.md) — Lambda deployment

---

*This guide complements the whisper-rs and ggml-rs crates. Refer to those projects for working implementations.*
