---
title: "Production-Grade LLM Inference: Deployment and Operations Guide"
subtitle: "From llama.cpp to production LLM serving systems"
based_on: "llama.cpp server + production deployment patterns"
level: "Advanced - Production deployment guide"
---

# Production-Grade LLM Inference

## Table of Contents

1. [Production Architecture Overview](#1-production-architecture-overview)
2. [Performance Optimization](#2-performance-optimization)
3. [Memory Management](#3-memory-management)
4. [Multi-GPU Inference](#4-multi-gpu-inference)
5. [Serving Infrastructure](#5-serving-infrastructure)
6. [Monitoring and Observability](#6-monitoring-and-observability)
7. [Scaling Strategies](#7-scaling-strategies)
8. [Security Considerations](#8-security-considerations)

---

## 1. Production Architecture Overview

### 1.1 Single-Node Deployment

```
┌─────────────────────────────────────────────────────────┐
│                    Single Node Server                    │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │                   llama-server                      ││
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐       ││
│  │  │  HTTP     │  │  Batch    │  │   Slot    │       ││
│  │  │  Handler  │  │  Manager  │  │  Manager  │       ││
│  │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘       ││
│  │        │              │              │               ││
│  │        └──────────────┼──────────────┘               ││
│  │                       │                              ││
│  │              ┌────────▼────────┐                     ││
│  │              │  Inference      │                     ││
│  │              │  Engine         │                     ││
│  │              └────────┬────────┘                     ││
│  │                       │                              ││
│  │              ┌────────▼────────┐                     ││
│  │              │    KV Cache     │                     ││
│  │              │  (Quantized)    │                     ││
│  │              └─────────────────┘                     ││
│  └─────────────────────────────────────────────────────┘│
│                       │                                   │
│         ┌─────────────┼─────────────┐                    │
│         ▼             ▼             ▼                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │   CPU    │  │   GPU    │  │   GPU    │              │
│  │  (Some)  │  │   (Most) │  │   (Most) │              │
│  └──────────┘  └──────────┘  └──────────┘              │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Multi-Node Cluster

```
┌─────────────────────────────────────────────────────────────────┐
│                         Load Balancer                            │
│                    (nginx / HAProxy / ALB)                      │
└────────────────────────────┬────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  Inference    │   │  Inference    │   │  Inference    │
│  Node 1       │   │  Node 2       │   │  Node N       │
│               │   │               │   │               │
│  ┌─────────┐  │   │  ┌─────────┐  │   │  ┌─────────┐  │
│  │ llama-  │  │   │  │ llama-  │  │   │  │ llama-  │  │
│  │ server  │  │   │  │ server  │  │   │  │ server  │  │
│  └─────────┘  │   │  └─────────┘  │   │  └─────────┘  │
│  4x A100      │   │  4x A100      │   │  4x A100      │
└───────────────┘   └───────────────┘   └───────────────┘
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │
                    ┌────────▼────────┐
                    │   Shared KV     │
                    │   Cache (Redis) │
                    └─────────────────┘
```

---

## 2. Performance Optimization

### 2.1 Benchmarking Metrics

```
Key metrics for production LLM inference:

1. Throughput
   - Tokens/second (generation)
   - Requests/second (overall)
   - Batch throughput

2. Latency
   - Time to first token (TTFT)
   - Time per output token (TPOT)
   - End-to-end latency

3. Resource Utilization
   - GPU memory usage
   - GPU compute utilization
   - CPU utilization
   - Memory bandwidth

4. Quality
   - Perplexity
   - User satisfaction
   - Error rates
```

### 2.2 Optimization Techniques

```bash
# llama-server optimization flags

# 1. Use all GPU layers
./llama-server -m model.gguf -ngl 999

# 2. Set optimal batch size
./llama-server -m model.gguf -b 512

# 3. Enable flash attention
./llama-server -m model.gguf --flash-attn

# 4. Use quantized KV cache
./llama-server -m model.gguf --cache-type-k q8_0 --cache-type-v q8_0

# 5. Enable continuous batching
./llama-server -m model.gguf --cont-batching

# 6. Set thread count
./llama-server -m model.gguf -t $(nproc)

# 7. Enable memory mapping
./llama-server -m model.gguf --mlock
```

### 2.3 Memory Bandwidth Optimization

```
LLM inference is memory-bandwidth bound!

Model: LLaMA 3 8B (FP16)
- Size: 16 GB
- Target: 50 tokens/second
- Required bandwidth: 16 GB × 50 = 800 GB/s

Hardware comparison:
- DDR5-6000: ~90 GB/s (too slow!)
- RTX 4090: 1008 GB/s ✓
- A100: 1555 GB/s ✓
- H100: 3350 GB/s ✓

Solutions:
1. Quantization (Q4_K_M): 16 GB → 5 GB
   Required: 5 GB × 50 = 250 GB/s ✓

2. Weight streaming:
   - Load weights layer-by-layer
   - Keep only current layer in VRAM

3. Tensor parallelism:
   - Split model across multiple GPUs
   - Each GPU processes part of each layer
```

### 2.4 Profiling

```cpp
// llama.cpp built-in profiling
struct llama_timings {
    double t_start_ms;
    double t_end_ms;
    double t_load_ms;      // Model loading
    double t_prompt_ms;    // Prompt processing
    double t_sample_ms;    // Sampling
    double t_predict_ms;   // Token generation

    int32_t n_sample;
    int32_t n_eval;
    int32_t n_decode;
};

// Usage in production
void print_timings(const llama_timings & timings) {
    fprintf(stderr, "\n");
    fprintf(stderr, "load time     = %10.2f ms\n", timings.t_load_ms);
    fprintf(stderr, "prompt eval = %10.2f ms / %5d tokens (%8.2f ms/token)\n",
            timings.t_prompt_ms, timings.n_eval,
            timings.t_prompt_ms / timings.n_eval);
    fprintf(stderr, "eval time   = %10.2f ms / %5d tokens (%8.2f ms/token, %8.2f tokens/s)\n",
            timings.t_predict_ms, timings.n_decode,
            timings.t_predict_ms / timings.n_decode,
            1000.0 * timings.n_decode / timings.t_predict_ms);
    fprintf(stderr, "sample time = %10.2f ms / %5d samples\n",
            timings.t_sample_ms, timings.n_sample);
}
```

---

## 3. Memory Management

### 3.1 Memory Budget Planning

```
Memory requirements for LLaMA 3 8B:

Component              FP16      Q4_K_M
────────────────────────────────────────
Weights               16 GB      5 GB
KV Cache (4K ctx)     4 GB      1 GB
Compute Buffer        2 GB      2 GB
Overhead              1 GB      1 GB
────────────────────────────────────────
Total                 23 GB      9 GB

For production (concurrent requests):

Requests   KV Cache   Total (Q4_K_M)
─────────────────────────────────────
1          1 GB       9 GB
4          4 GB       12 GB
8          8 GB       16 GB
16         16 GB      24 GB
```

### 3.2 KV Cache Eviction

```cpp
// Sliding window eviction
void llama_kv_cache_evict(
    struct llama_kv_cache * cache,
    uint32_t n_keep,
    uint32_t n_local
) {
    // Keep first n_keep tokens (prompt)
    // Keep last n_local tokens (sliding window)
    // Evict everything in between

    for (auto & cell : cache->cells) {
        if (cell.pos >= n_keep && cell.pos < cell.pos_max - n_local) {
            cell.seq_id = -1;  // Mark as free
        }
    }
}

// Production configuration:
// n_keep = prompt tokens (always keep)
// n_local = 512 or 1024 (recent context)
```

### 3.3 Memory Pools

```cpp
// Pre-allocate memory pools for inference
struct llama_memory_pool {
    // Weight buffer (GPU)
    void * weights_gpu;
    size_t weights_size;

    // KV cache buffer
    void * kv_cache;
    size_t kv_size;

    // Compute buffer (temporary)
    void * compute_buffer;
    size_t compute_size;

    // Host buffer (staging)
    void * host_buffer;
    size_t host_size;
};

void llama_memory_pool_init(
    struct llama_memory_pool * pool,
    const struct llama_model * model,
    uint32_t n_ctx
) {
    // Allocate GPU memory for weights
    pool->weights_size = llama_model_size(model);
    pool->weights_gpu = cuda_malloc(pool->weights_size);

    // Allocate KV cache
    pool->kv_size = llama_kv_cache_size(model, n_ctx);
    pool->kv_cache = cuda_malloc(pool->kv_size);

    // Allocate compute buffer (largest layer activation)
    pool->compute_size = llama_compute_buffer_size(model);
    pool->compute_buffer = cuda_malloc(pool->compute_size);

    // Allocate host buffer
    pool->host_size = pool->weights_size;
    pool->host_buffer = malloc(pool->host_size);
}
```

---

## 4. Multi-GPU Inference

### 4.1 Tensor Parallelism

```
Tensor Parallelism splits individual layers across GPUs:

Layer Normal x W → Split across GPUs:
GPU 0: Normal x W[0:hidden/2]
GPU 1: Normal x W[hidden/2:hidden]

Attention Heads:
GPU 0: Q[0:heads/2], K[0:heads/2], V[0:heads/2]
GPU 1: Q[heads/2:], K[heads/2:], V[heads/2:]

FFN:
GPU 0: FFN[0:intermediate/2]
GPU 1: FFN[intermediate/2:]

Communication: All-reduce after each layer
```

### 4.2 Pipeline Parallelism

```
Pipeline Parallelism splits layers across GPUs:

GPU 0: Layers 0-15 (Embedding + Early layers)
GPU 1: Layers 16-31 (Middle layers)
GPU 2: Layers 32-47 (Late layers)
GPU 3: Layers 48-63 (Output)

Flow:
Token → GPU0 → GPU1 → GPU2 → GPU3 → Logits

Higher latency but lower memory per GPU
```

### 4.3 llama.cpp Multi-GPU

```bash
# Split model across GPUs
./llama-server -m model.gguf \
    -mg 0 1 \
    -sm row

# Main GPU (for KV cache and sampling)
./llama-server -m model.gguf \
    -mg 0 1 \
    -mgg 0

# Split mode: row vs layer
# row: Split tensors row-wise (tensor parallelism)
# layer: Split by layers (pipeline parallelism)
```

---

## 5. Serving Infrastructure

### 5.1 llama-server Configuration

```bash
# Production server configuration
./llama-server \
    -m models/llama-3-70b-instruct-q4_k_m.gguf \
    -ngl 999 \
    -c 8192 \
    -b 512 \
    --flash-attn \
    --cache-type-k q8_0 \
    --cache-type-v q8_0 \
    --cont-batching \
    -t $(nproc) \
    -fa \
    \
    # HTTP configuration
    --host 0.0.0.0 \
    --port 8080 \
    --api-keys file:api_keys.txt \
    \
    # Rate limiting
    --slots \
    --max-users 100 \
    \
    # Logging
    --log-file /var/log/llama-server.log \
    --log-prefix \
    \
    # Metrics
    --metrics \
    --metrics-port 9090 \
    \
    # OpenAI compatibility
    --alias llama-3-70b \
    --chat-template llama-3
```

### 5.2 OpenAI-Compatible API

```python
# Using llama-server as OpenAI drop-in replacement
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="not-needed"  # Or your API key
)

response = client.chat.completions.create(
    model="llama-3-70b",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello!"}
    ],
    temperature=0.7,
    max_tokens=100,
    stream=True
)

for chunk in response:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="")
```

### 5.3 Load Balancing

```nginx
# nginx configuration for llama-server cluster
upstream llama_cluster {
    least_conn;
    server llama1:8080 weight=1 max_fails=3 fail_timeout=30s;
    server llama2:8080 weight=1 max_fails=3 fail_timeout=30s;
    server llama3:8080 weight=1 max_fails=3 fail_timeout=30s;
}

server {
    listen 80;
    server_name api.example.com;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;

    location /v1/ {
        limit_req zone=api_limit burst=20 nodelay;

        proxy_pass http://llama_cluster;
        proxy_http_version 1.1;
        proxy_set_header Connection "";

        # Streaming support
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 300s;

        # Health checks
        proxy_next_upstream error timeout http_502 http_503;
    }

    # Health endpoint
    location /health {
        return 200 "healthy";
    }
}
```

---

## 6. Monitoring and Observability

### 6.1 Prometheus Metrics

```python
# llama-server exposes Prometheus metrics at /metrics

# Example metrics:
# HELP llama_tokens_generated_total Total tokens generated
# TYPE llama_tokens_generated_total counter
llama_tokens_generated_total{model="llama-3-70b"} 1234567

# HELP llama_request_duration_seconds Request duration
# TYPE llama_request_duration_seconds histogram
llama_request_duration_seconds_bucket{le="0.1"} 100
llama_request_duration_seconds_bucket{le="0.5"} 500
llama_request_duration_seconds_bucket{le="1.0"} 800
llama_request_duration_seconds_bucket{le="+Inf"} 1000

# HELP llama_kv_cache_usage_bytes KV cache memory usage
# TYPE llama_kv_cache_usage_bytes gauge
llama_kv_cache_usage_bytes 4294967296

# HELP llama_gpu_memory_used_bytes GPU memory used
# TYPE llama_gpu_memory_used_bytes gauge
llama_gpu_memory_used_bytes{gpu="0"} 8589934592
```

### 6.2 Grafana Dashboard

```json
{
  "dashboard": {
    "title": "LLM Inference Dashboard",
    "panels": [
      {
        "title": "Tokens/Second",
        "targets": [{
          "expr": "rate(llama_tokens_generated_total[1m])"
        }]
      },
      {
        "title": "Request Latency (p99)",
        "targets": [{
          "expr": "histogram_quantile(0.99, rate(llama_request_duration_seconds_bucket[5m]))"
        }]
      },
      {
        "title": "KV Cache Usage",
        "targets": [{
          "expr": "llama_kv_cache_usage_bytes / llama_kv_cache_total_bytes * 100"
        }]
      },
      {
        "title": "GPU Memory",
        "targets": [{
          "expr": "llama_gpu_memory_used_bytes / llama_gpu_memory_total_bytes * 100"
        }]
      }
    ]
  }
}
```

### 6.3 Distributed Tracing

```python
# OpenTelemetry integration
from opentelemetry import trace
from opentelemetry.exporter.jaeger.thrift import JaegerExporter

tracer = trace.get_tracer("llama-inference")

@tracer.start_as_current_span("generate")
def generate(prompt: str) -> str:
    with tracer.start_as_current_span("tokenize"):
        tokens = tokenizer.encode(prompt)

    with tracer.start_as_current_span("inference"):
        output_tokens = model.generate(tokens)

    with tracer.start_as_current_span("detokenize"):
        output = tokenizer.decode(output_tokens)

    return output
```

---

## 7. Scaling Strategies

### 7.1 Horizontal Scaling

```
Add more inference nodes behind load balancer:

                    Load Balancer
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
    ┌───────┐       ┌───────┐       ┌───────┐
    │ Node 1│       │ Node 2│       │ Node N│
    │ 8x A10│       │ 8x A10│       │ 8x A10│
    └───────┘       └───────┘       └───────┘

Pros:
- Linear throughput scaling
- Simple to implement
- No code changes needed

Cons:
- Higher latency for model loading
- More expensive (replicated memory)
```

### 7.2 Model Parallelism

```
Split large models across multiple GPUs:

Model: 70B (140 GB Q4_K_M)
GPU 0-1: Layers 0-19 (35 GB each)
GPU 2-3: Layers 20-39 (35 GB each)
GPU 4-5: Layers 40-59 (35 GB each)
GPU 6-7: Layers 60-79 (35 GB each)

Enables running models larger than single GPU memory
```

### 7.3 Request Batching

```
Continuous batching (in-flight batching):

Time →
R1: [Prompt][T1][T2][T3][EOS]──────────
R2: [Prompt][T1][T2][T3][T4][T5]──────
R3: ──────[Prompt][T1][T2][EOS]───────
R4: ──────────────[Prompt][T1][T2]

Instead of waiting for all requests to complete,
immediately start new requests when others finish!

Throughput improvement: 2-5x for variable-length requests
```

---

## 8. Security Considerations

### 8.1 API Security

```bash
# Enable API key authentication
./llama-server \
    --api-keys file:api_keys.txt \
    --api-keys-prefix-match

# api_keys.txt format:
# sk-key1,user1,100  # 100 requests/minute
# sk-key2,user2,1000
```

### 8.2 Rate Limiting

```python
# Rate limiting configuration
RATE_LIMITS = {
    "free": {
        "requests_per_minute": 10,
        "tokens_per_minute": 1000,
        "max_context": 4096,
    },
    "pro": {
        "requests_per_minute": 100,
        "tokens_per_minute": 10000,
        "max_context": 32768,
    },
    "enterprise": {
        "requests_per_minute": 1000,
        "tokens_per_minute": 100000,
        "max_context": 131072,
    }
}
```

### 8.3 Input Validation

```python
# Validate inputs to prevent abuse
def validate_request(request: ChatRequest) -> ValidationResult:
    # Check prompt length
    if len(request.messages) > MAX_MESSAGES:
        return ValidationResult.error("Too many messages")

    # Check total tokens
    total_tokens = count_tokens(request.messages)
    if total_tokens > MAX_PROMPT_TOKENS:
        return ValidationResult.error("Prompt too long")

    # Check for injection attempts
    for msg in request.messages:
        if detect_injection(msg.content):
            return ValidationResult.error("Invalid content")

    return ValidationResult.ok()
```

### 8.4 Output Filtering

```python
# Filter model outputs
def filter_output(output: str) -> str:
    # Remove PII
    output = redact_pii(output)

    # Check for harmful content
    if is_harmful(output):
        return "I cannot provide that information."

    # Truncate if too long
    if len(output) > MAX_OUTPUT_LENGTH:
        output = output[:MAX_OUTPUT_LENGTH] + "..."

    return output
```

---

## Summary

### Production Checklist

- [ ] Quantized models (Q4_K_M or Q5_K_M)
- [ ] GPU offloading enabled
- [ ] KV cache quantization
- [ ] Flash attention enabled
- [ ] Continuous batching
- [ ] Rate limiting configured
- [ ] API authentication enabled
- [ ] Monitoring/metrics set up
- [ ] Logging configured
- [ ] Health checks implemented
- [ ] Load balancing configured
- [ ] Error handling robust
- [ ] Input validation in place
- [ ] Output filtering configured

### Key Takeaways

1. **Quantization is essential** for production (4-8x memory reduction)
2. **GPU offloading** provides best performance
3. **Continuous batching** improves throughput 2-5x
4. **Monitoring** is critical for production reliability
5. **Rate limiting** protects against abuse
6. **Input/output filtering** ensures safety

---

*This guide complements the llama.cpp documentation. For production deployments, always test thoroughly with your specific workload.*
