---
title: "Production-Grade microgpt: From Toy to System"
subtitle: "Building a production-ready GPT implementation based on microgpt principles"
based_on: "microgpt architecture with production enhancements"
level: "Advanced - Production engineering considerations"
---

# Production-Grade microgpt: Rust Implementation Guide

## Table of Contents

1. [What microgpt Leaves Out](#1-what-microgpt-leaves-out)
2. [Production Architecture Overview](#2-production-architecture-overview)
3. [Performance Optimizations](#3-performance-optimizations)
4. [Memory Management](#4-memory-management)
5. [Batching and Throughput](#5-batching-and-throughput)
6. [Model Serialization](#6-model-serialization)
7. [Serving Infrastructure](#7-serving-infrastructure)
8. [Monitoring and Observability](#8-monitoring-and-observability)

---

## 1. What microgpt Leaves Out

### 1.1 Educational vs. Production Trade-offs

| Aspect | microgpt (Educational) | Production System |
|--------|----------------------|-------------------|
| **Speed** | Pure Python, scalar ops | GPU, batched, SIMD |
| **Memory** | No optimization | Mixed precision, checkpointing |
| **Scale** | ~76K params, 1 layer | Millions to billions of params |
| **Parallelism** | None | Data parallel, pipeline parallel |
| **Precision** | FP64 | FP16/BF16/FP8 |
| **IO** | None | Async, streaming |
| **Error handling** | None | Comprehensive |
| **Testing** | None | Unit, integration, load tests |

### 1.2 Performance Gap Analysis

**microgpt performance (estimated):**
```
Forward pass (1 token): ~100ms (pure Python scalar ops)
Training (1000 steps): ~30 minutes
Inference (20 tokens): ~2 seconds
```

**Optimized Rust implementation:**
```
Forward pass (1 token): ~0.1ms (SIMD, cache-friendly)
Training (1000 steps): ~30 seconds (30-60x speedup)
Inference (20 tokens): ~2ms (500-1000x speedup)
```

---

## 2. Production Architecture Overview

### 2.1 System Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Production GPT System                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   Training   │    │  Inference   │    │    Serving   │  │
│  │   Pipeline   │    │   Engine     │    │    Layer     │  │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘  │
│         │                   │                   │          │
│         └───────────────────┼───────────────────┘          │
│                             │                               │
│                    ┌────────▼────────┐                      │
│                    │  Model Store    │                      │
│                    │  (Checkpoints)  │                      │
│                    └────────┬────────┘                      │
│                             │                               │
│         ┌───────────────────┼───────────────────┐          │
│         │                   │                   │          │
│  ┌──────▼───────┐   ┌──────▼───────┐   ┌──────▼───────┐  │
│  │   Metrics    │   │    Logging   │   │   Profiling  │  │
│  │   (Prometheus)│  │   (Tracing)  │   │   (Perf)     │  │
│  └──────────────┘   └──────────────┘   └──────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Rust Crate Structure

```
production-gpt/
├── Cargo.toml
├── gpt-core/           # Core model implementation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── model.rs    # GPT architecture
│       ├── attention.rs # Multi-head attention
│       ├── mlp.rs      # Feed-forward layers
│       ├── norm.rs     # LayerNorm/RMSNorm
│       └── activations.rs
├── gpt-train/          # Training binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── trainer.rs
│       ├── optimizer.rs # Adam, AdamW implementations
│       └── data_loader.rs
├── gpt-infer/          # Inference binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── generator.rs
│       ├── sampler.rs  # Top-k, top-p, temperature
│       └── kv_cache.rs
├── gpt-serve/          # HTTP/gRPC serving
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── api.rs
│       └── batcher.rs
└── gpt-utils/
    ├── Cargo.toml
    └── src/
        ├── serialization.rs
        ├── metrics.rs
        └── config.rs
```

---

## 3. Performance Optimizations

### 3.1 Matrix Operations with BLAS

**microgpt (scalar):**
```python
def linear(x, w):
    return [sum(wi * xi for wi, xi in zip(wo, x)) for wo in w]
# O(n²) scalar operations
```

**Production Rust (BLAS):**
```rust
use ndarray::{Array1, Array2};
use ndarray_linalg::matmul;

fn linear(x: &Array1<f32>, w: &Array2<f32>) -> Array1<f32> {
    w.dot(x)  // Calls optimized BLAS routine
}
// Uses SIMD, cache-optimized, multi-threaded
```

**Speedup:** 10-100x depending on matrix size.

### 3.2 SIMD Vectorization

**Manual SIMD with `std::simd` (nightly):**
```rust
use std::simd::{f32x8, SimdFloat, Simd};

fn dot_product_simd(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = f32x8::splat(0.0);

    for i in (0..a.len()).step_by(8) {
        let va = f32x8::from_slice(&a[i..]);
        let vb = f32x8::from_slice(&b[i..]);
        sum += va * vb;
    }

    sum.reduce_sum()
}
// Processes 8 floats per instruction
```

### 3.3 Memory Layout Optimization

**Structure of Arrays (SoA) vs Array of Structures (AoS):**

```rust
// AoS (cache-unfriendly for SIMD)
struct Neuron {
    weight: f32,
    bias: f32,
    output: f32,
}
let neurons: Vec<Neuron> = ...;

// SoA (cache-friendly for SIMD)
struct Layer {
    weights: Vec<f32>,
    biases: Vec<f32>,
    outputs: Vec<f32>,
}
// Access contiguous memory for each operation
```

### 3.4 Parallel Processing with Rayon

```rust
use rayon::prelude::*;

fn forward_batch(model: &Model, batch: &[Input]) -> Vec<Output> {
    batch.par_iter()  // Parallel iterator
        .map(|input| model.forward(input))
        .collect()
}
// Automatically uses all CPU cores
```

---

## 4. Memory Management

### 4.1 Mixed Precision Training

```rust
/// Model weights stored in FP16 for memory efficiency
struct MixedPrecisionModel {
    weights_fp16: Vec<f16>,      // Main storage
    weights_fp32: Vec<f32>,      // Master copy for optimizer
    gradients_fp32: Vec<f32>,    // Gradients in FP32
}

impl MixedPrecisionModel {
    fn forward(&self, input: &Input) -> Output {
        // Convert to FP32 for computation
        let weights_fp32 = self.weights_fp16.iter()
            .map(|w| f32::from(*w))
            .collect();

        // Forward pass in FP32
        compute(&weights_fp32, input)
    }

    fn backward(&mut self, loss: f32) {
        // Compute gradients in FP32
        self.gradients_fp32 = compute_gradients(loss);

        // Apply gradient clipping (prevent overflow)
        let grad_norm = self.gradients_fp32.iter()
            .map(|g| g * g)
            .sum::<f32>().sqrt();

        if grad_norm > self.max_grad_norm {
            let scale = self.max_grad_norm / grad_norm;
            for g in &mut self.gradients_fp32 {
                *g *= scale;
            }
        }
    }
}
```

### 4.2 Gradient Checkpointing

**Problem:** Storing all intermediate activations uses too much memory.

**Solution:** Recompute activations during backward pass.

```rust
struct CheckpointedLayer {
    weights: Vec<f32>,
    save_activations: bool,  // Toggle per layer
}

impl CheckpointedLayer {
    fn forward(&self, x: &Tensor, save: bool) -> (Tensor, Option<Tensor>) {
        let activation = compute_activation(x, &self.weights);

        if save {
            (activation.clone(), Some(activation))
        } else {
            (activation, None)  // Will recompute during backward
        }
    }

    fn backward(&self, x: &Tensor, saved: Option<&Tensor>, grad: &Tensor) -> Tensor {
        let activation = saved.unwrap_or_else(|| {
            // Recompute if not saved
            compute_activation(x, &self.weights)
        });

        compute_gradient(&activation, grad)
    }
}
```

**Trade-off:** 50% less memory, 33% more computation.

### 4.3 Arena Allocation

```rust
use typed_arena::Arena;

fn training_step(arena: &Arena<Tensor>) {
    // All intermediate tensors allocated in arena
    let x = arena.alloc(Tensor::zeros([768]));
    let q = arena.alloc(Tensor::zeros([768]));
    let k = arena.alloc(Tensor::zeros([768]));
    let v = arena.alloc(Tensor::zeros([768]));
    let attn = arena.alloc(Tensor::zeros([768]));
    // ... many more allocations

    // Single free when arena drops
}
// All tensors freed at once, no individual drop overhead
```

---

## 5. Batching and Throughput

### 5.1 Batched Forward Pass

```rust
struct BatchedModel {
    weights: ModelWeights,
    max_batch_size: usize,
}

impl BatchedModel {
    fn forward_batch(&self, inputs: &[Tensor]) -> Vec<Tensor> {
        // Stack inputs into batch tensor [batch, seq, embd]
        let batch_tensor = stack_tensors(inputs);

        // Single matrix multiply for entire batch
        let output_batch = self.weights.apply(&batch_tensor);

        // Split back into individual outputs
        unstack_tensors(output_batch)
    }
}

// Throughput comparison:
// Sequential: 1000 tokens × 1ms = 1000ms
// Batched (batch=32): 32 batches × 2ms = 64ms (15x speedup)
```

### 5.2 Dynamic Batching for Serving

```rust
struct DynamicBatcher {
    pending_requests: Vec<Request>,
    max_batch_size: usize,
    max_wait_time: Duration,
}

impl DynamicBatcher {
    async fn process_requests(&mut self) -> Vec<Response> {
        // Wait for requests to accumulate
        while self.pending_requests.len() < self.max_batch_size
              && !self.max_wait_time.elapsed() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // Batch process
        let batch = self.pending_requests.drain(..).collect::<Vec<_>>();
        let responses = self.model.process_batch(&batch).await;

        responses
    }
}
```

---

## 6. Model Serialization

### 6.1 Checkpoint Format

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct ModelCheckpoint {
    version: u32,
    config: ModelConfig,
    step: u64,
    optimizer_state: OptimizerState,
    parameters: HashMap<String, Vec<f32>>,
}

#[derive(Serialize, Deserialize)]
struct ModelConfig {
    vocab_size: usize,
    hidden_size: usize,
    num_layers: usize,
    num_heads: usize,
    // ... other hyperparameters
}
```

### 6.2 Save/Load Implementation

```rust
use std::fs::File;
use std::io::{BufReader, BufWriter};
use bincode;

impl Model {
    fn save(&self, path: &str) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let checkpoint = ModelCheckpoint {
            version: 1,
            config: self.config.clone(),
            step: self.step,
            optimizer_state: self.optimizer.state(),
            parameters: self.state_dict(),
        };

        bincode::serialize_into(writer, &checkpoint)?;
        Ok(())
    }

    fn load(path: &str) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let checkpoint: ModelCheckpoint = bincode::deserialize_from(reader)?;

        Ok(Model {
            config: checkpoint.config,
            step: checkpoint.step,
            parameters: checkpoint.parameters,
            // ... restore optimizer state
        })
    }
}
```

---

## 7. Serving Infrastructure

### 7.1 HTTP API Design

```rust
use axum::{Router, routing::post, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct GenerateRequest {
    prompt: String,
    max_tokens: usize,
    temperature: Option<f32>,
    top_p: Option<f32>,
}

#[derive(Serialize)]
struct GenerateResponse {
    text: String,
    tokens: Vec<String>,
    logprobs: Option<Vec<f32>>,
}

async fn generate(Json(req): Json<GenerateRequest>) -> Json<GenerateResponse> {
    let tokens = tokenizer.encode(&req.prompt);
    let generated = model.generate(
        &tokens,
        req.max_tokens,
        req.temperature.unwrap_or(0.7),
        req.top_p,
    ).await;

    Json(GenerateResponse {
        text: tokenizer.decode(&generated),
        tokens: tokenizer.batch_decode(&generated),
        logprobs: None,
    })
}

fn create_router(model: Model) -> Router {
    Router::new()
        .route("/generate", post(generate))
        .route("/health", get(health_check))
        .with_state(model)
}
```

### 7.2 Streaming Responses

```rust
use axum::body::StreamBody;
use futures_util::stream::Stream;

async fn generate_stream(
    Json(req): Json<GenerateRequest>
) -> StreamBody<impl Stream<Item = Result<Bytes, Infallible>>> {
    let stream = model.generate_stream(
        &req.prompt,
        req.max_tokens,
        req.temperature.unwrap_or(0.7),
    );

    StreamBody::new(stream.map(|token| {
        Ok(Bytes::from(format!("data: {}\n\n", token)))
    }))
}
```

---

## 8. Monitoring and Observability

### 8.1 Metrics Collection

```rust
use prometheus::{Registry, Counter, Histogram, Gauge};

struct Metrics {
    requests_total: Counter,
    request_duration: Histogram,
    tokens_generated: Counter,
    gpu_memory: Gauge,
    batch_size: Histogram,
}

impl Metrics {
    fn new(registry: &Registry) -> Self {
        let requests_total = Counter::new(
            "gpt_requests_total",
            "Total number of requests"
        ).unwrap();
        registry.register(Box::new(requests_total.clone())).unwrap();

        let request_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "gpt_request_duration_seconds",
                "Request duration in seconds"
            ).buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0])
        ).unwrap();
        registry.register(Box::new(request_duration.clone())).unwrap();

        // ... more metrics

        Metrics {
            requests_total,
            request_duration,
            tokens_generated: /* ... */,
            gpu_memory: /* ... */,
            batch_size: /* ... */,
        }
    }

    fn record_request(&self, duration: f64, tokens: usize) {
        self.requests_total.inc();
        self.request_duration.observe(duration);
        self.tokens_generated.inc_by(tokens as u64);
    }
}
```

### 8.2 Distributed Tracing

```rust
use tracing::{info, instrument, Span};
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[instrument(skip(model, request), fields(prompt_length = request.prompt.len()))]
async fn generate_with_tracing(
    model: &Model,
    request: GenerateRequest
) -> GenerateResponse {
    info!("Starting generation");
    Span::current().record("max_tokens", request.max_tokens);

    let start = std::time::Instant::now();
    let result = model.generate(&request).await;
    let duration = start.elapsed();

    info!(duration_ms = duration.as_millis(), "Generation complete");

    result
}

// Initialize tracing
fn setup_tracing() {
    let subscriber = Registry::default()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer());

    tracing::subscriber::set_global_default(subscriber).unwrap();
}
```

### 8.3 Health Checks

```rust
async fn health_check(
    State(model): State<Model>
) -> Result<Json<HealthStatus>, StatusCode> {
    let gpu_available = model.gpu_memory_available() > 1000;
    let model_loaded = model.is_loaded();
    let last_error = model.last_error();

    let status = if gpu_available && model_loaded && last_error.is_none() {
        HealthStatus::Healthy
    } else if model_loaded {
        HealthStatus::Degraded
    } else {
        HealthStatus::Unhealthy
    };

    Ok(Json(HealthStatus {
        status,
        gpu_memory_mb: model.gpu_memory_available(),
        last_error: last_error.map(|e| e.to_string()),
        uptime_seconds: model.uptime().as_secs(),
    }))
}
```

---

## Summary

Building a production-grade GPT implementation requires:

1. **Performance optimizations:** BLAS, SIMD, parallel processing
2. **Memory management:** Mixed precision, gradient checkpointing
3. **Batching:** Static and dynamic batching for throughput
4. **Serialization:** Efficient checkpoint save/load
5. **Serving:** HTTP APIs, streaming responses
6. **Observability:** Metrics, tracing, health checks

The jump from microgpt to production is significant but follows well-established patterns.

---

*Next: Read the rust-revision.md for the complete Rust translation of microgpt.*
