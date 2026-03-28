---
title: "Valtron Integration: Lambda Deployment for LLM Inference"
subtitle: "Serverless LLM inference without async/await or tokio"
based_on: "valtron executor + llama.cpp inference patterns"
level: "Advanced - Serverless deployment guide"
---

# Valtron Integration: Lambda Deployment for LLM Inference

## Table of Contents

1. [Why Valtron for Lambda?](#1-why-valtron-for-lambda)
2. [Lambda Architecture Overview](#2-lambda-architecture-overview)
3. [Valtron Executor Basics](#3-valtron-executor-basics)
4. [LLM Inference Task Design](#4-llm-inference-task-design)
5. [HTTP API Compatibility](#5-http-api-compatibility)
6. [Lambda Deployment Guide](#6-lambda-deployment-guide)
7. [Performance Optimization](#7-performance-optimization)
8. [Cost Optimization](#8-cost-optimization)

---

## 1. Why Valtron for Lambda?

### 1.1 Problems with Async Runtimes in Lambda

```
Traditional async/await in Lambda:

┌─────────────────────────────────────────────────────────┐
│                   Lambda Invocation                      │
│                                                          │
│  1. Runtime initialization (~100-500ms)                 │
│     - tokio runtime startup                              │
│     - async task scheduler setup                         │
│     - Thread pool initialization                         │
│                                                          │
│  2. Handler execution                                    │
│     - async fn handler() -> Result<Response>             │
│     - tokio::block_on() to completion                    │
│     - Wasted: async machinery for single request         │
│                                                          │
│  3. Lambda freeze                                        │
│     - Pending async tasks: undefined behavior            │
│     - Runtime cleanup overhead                           │
│                                                          │
└─────────────────────────────────────────────────────────┘

Problems:
- Cold start latency from runtime initialization
- Async machinery waste (single request per invocation)
- Lambda freeze with pending async tasks
- Larger deployment package (tokio + dependencies)
```

### 1.2 Valtron Advantages

```
Valtron executor in Lambda:

┌─────────────────────────────────────────────────────────┐
│                   Lambda Invocation                      │
│                                                          │
│  1. Minimal initialization (~10-50ms)                   │
│     - No async runtime                                   │
│     - No thread pool (single-threaded)                   │
│     - Just seed the random generator                     │
│                                                          │
│  2. Handler execution                                    │
│     - Direct TaskIterator execution                      │
│     - Deterministic, step-by-step                        │
│     - No async overhead                                  │
│                                                          │
│  3. Clean exit                                           │
│     - No pending tasks                                   │
│     - Immediate return                                   │
│     - Predictable freeze behavior                        │
│                                                          │
└─────────────────────────────────────────────────────────┘

Benefits:
- 50-200ms faster cold starts
- Smaller deployment package
- Deterministic execution
- Clean Lambda lifecycle
- WASM-compatible (for Lambda WebAssembly)
```

### 1.3 Comparison Table

| Aspect | Tokio-based | Valtron |
|--------|-------------|---------|
| Cold start | 100-500ms | 10-50ms |
| Package size | +2-5 MB | +100 KB |
| Dependencies | tokio, hyper, etc. | valtron only |
| Thread overhead | Yes (thread pool) | No (single-threaded) |
| WASM compatible | No | Yes |
| Lambda freeze handling | Complex | Simple |

---

## 2. Lambda Architecture Overview

### 2.1 Serverless LLM Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    API Gateway                           │
│              (REST or HTTP API)                          │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                   Lambda Function                        │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │              Valtron Executor                        ││
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐       ││
│  │  │  Tokenize │  │  Forward  │  │  Sample   │       ││
│  │  │   Task    │  │   Task    │  │   Task    │       ││
│  │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘       ││
│  │        │              │              │               ││
│  │        └──────────────┼──────────────┘               ││
│  │                       │                              ││
│  │              execute_iter()                          ││
│  └─────────────────────────────────────────────────────┘│
│                       │                                   │
│                       ▼                                   │
│              ┌─────────────────┐                         │
│              │  Model Weights  │  (from /tmp or S3)      │
│              │  (mmapped)      │                          │
│              └─────────────────┘                         │
└─────────────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                   Response                               │
│              (JSON with tokens)                          │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Lambda Layers for Model Weights

```
Model weights storage options:

1. Lambda Layer (up to 250 MB unzipped)
   - Small models only (1-3B Q4_K_M)
   - Fast access (mounted at /opt)
   - No download needed

2. EFS (Elastic File System)
   - Up to hundreds of GB
   - Mount at /mnt/efs
   - Cold start latency for mounting

3. S3 Download to /tmp
   - /tmp has up to 10 GB (Lambda with ephemeral storage)
   - Download on cold start
   - Cache in memory after load

4. Lambda SnapStart (Java only)
   - Not available for Rust yet
```

---

## 3. Valtron Executor Basics

### 3.1 TaskIterator Trait

```rust
use valtron::{TaskIterator, TaskStatus, NoSpawner};

/// Core trait for Valtron tasks
pub trait TaskIterator {
    /// Value type when task is Ready
    type Ready;

    /// Value type when task is Pending
    type Pending;

    /// Spawner type (NoSpawner for single-threaded)
    type Spawner: ExecutionAction;

    /// Get next status of the task
    fn next_status(&mut self) -> Option<TaskStatus<
        Self::Ready,
        Self::Pending,
        Self::Spawner,
    >>;
}
```

### 3.2 TaskStatus Enum

```rust
pub enum TaskStatus<D, P, S: ExecutionAction> {
    /// Operation is still processing
    Pending(P),

    /// Initializing state
    Init,

    /// Delayed by a specific duration
    Delayed(Duration),

    /// Result is ready
    Ready(D),

    /// Request to spawn a sub-task
    Spawn(S),

    /// Skip this item
    Ignore,
}
```

### 3.3 Single-Threaded Executor

```rust
// Initialize valtron (Lambda handler)
pub fn initialize_executor() {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    valtron::single::initialize_pool(seed);
}

// Execute task to completion
pub fn execute_task<T>(task: T) -> Result<T::Ready, valtron::Error>
where
    T: TaskIterator<Spawner = NoSpawner>,
{
    valtron::single::spawn()
        .with_task(task)
        .schedule_iter(std::time::Duration::from_millis(0))
        .and_then(|_| valtron::single::run_until_complete())
}
```

---

## 4. LLM Inference Task Design

### 4.1 Tokenization Task

```rust
use valtron::{TaskIterator, TaskStatus, NoSpawner};

pub struct TokenizeTask {
    tokenizer: Arc<Tokenizer>,
    text: String,
    state: TokenizeState,
    result: Option<Vec<u32>>,
}

enum TokenizeState {
    Init,
    Encoding,
    Done,
}

impl TaskIterator for TokenizeTask {
    type Ready = Vec<u32>;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            TokenizeState::Init => {
                self.state = TokenizeState::Encoding;
                Some(TaskStatus::Pending(()))
            }
            TokenizeState::Encoding => {
                let tokens = self.tokenizer.encode(&self.text);
                self.result = Some(tokens.clone());
                self.state = TokenizeState::Done;
                Some(TaskStatus::Ready(tokens))
            }
            TokenizeState::Done => None,
        }
    }
}
```

### 4.2 Forward Pass Task

```rust
pub struct ForwardTask {
    model: Arc<LlamaModel>,
    tokens: Vec<u32>,
    positions: Vec<i32>,
    kv_cache: Arc<Mutex<KvCache>>,
    layer: usize,
    hidden: Option<Vec<f32>>,
    state: ForwardState,
}

enum ForwardState {
    Embedding,
    LayerProcessing(usize),
    OutputNorm,
    Logits,
    Done,
}

impl TaskIterator for ForwardTask {
    type Ready = Vec<f32>;  // Logits
    type Pending = ForwardProgress;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            ForwardState::Embedding => {
                // Look up token embeddings
                let embeddings = self.model.get_embeddings(&self.tokens);
                self.hidden = Some(embeddings);
                self.state = ForwardState::LayerProcessing(0);
                Some(TaskStatus::Pending(ForwardProgress::Embedding))
            }

            ForwardState::LayerProcessing(layer) => {
                if layer >= self.model.n_layers() {
                    self.state = ForwardState::OutputNorm;
                    return Some(TaskStatus::Pending(ForwardProgress::Layer(layer as u32)));
                }

                // Process one layer (simplified)
                let hidden = self.hidden.as_ref().unwrap();
                let output = self.model.forward_layer(layer, hidden, &self.positions, &self.kv_cache);
                self.hidden = Some(output);

                self.state = ForwardState::LayerProcessing(layer + 1);
                Some(TaskStatus::Pending(ForwardProgress::Layer(layer as u32)))
            }

            ForwardState::OutputNorm => {
                // Apply final RMSNorm
                let hidden = self.model.apply_norm(self.hidden.as_ref().unwrap());
                self.hidden = Some(hidden);
                self.state = ForwardState::Logits;
                Some(TaskStatus::Pending(ForwardProgress::OutputNorm))
            }

            ForwardState::Logits => {
                // Compute logits for last token
                let logits = self.model.compute_logits(self.hidden.as_ref().unwrap());
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
    OutputNorm,
}
```

### 4.3 Sampling Task

```rust
pub struct SampleTask {
    logits: Vec<f32>,
    temperature: f32,
    top_k: i32,
    top_p: f32,
    state: SampleState,
}

enum SampleState {
    Init,
    ApplyTemperature,
    ApplyTopK,
    ApplyTopP,
    Sample,
    Done,
}

impl TaskIterator for SampleTask {
    type Ready = u32;  // Token ID
    type Pending = ();
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            SampleState::Init => {
                apply_temperature(&mut self.logits, self.temperature);
                self.state = SampleState::ApplyTemperature;
                Some(TaskStatus::Pending(()))
            }
            SampleState::ApplyTemperature => {
                if self.top_k > 0 {
                    apply_top_k(&mut self.logits, self.top_k);
                }
                self.state = SampleState::ApplyTopK;
                Some(TaskStatus::Pending(()))
            }
            SampleState::ApplyTopK => {
                if self.top_p > 0.0 && self.top_p < 1.0 {
                    apply_top_p(&mut self.logits, self.top_p);
                }
                self.state = SampleState::Sample;
                Some(TaskStatus::Pending(()))
            }
            SampleState::Sample => {
                let token = sample_from_logits(&self.logits);
                self.state = SampleState::Done;
                Some(TaskStatus::Ready(token))
            }
            SampleState::Done => None,
        }
    }
}
```

### 4.4 Complete Generation Task

```rust
pub struct GenerateTask {
    model: Arc<LlamaModel>,
    tokenizer: Arc<Tokenizer>,
    prompt: String,
    max_tokens: usize,
    temperature: f32,
    top_k: i32,
    top_p: f32,

    // State
    tokens: Vec<u32>,
    generated: Vec<u32>,
    kv_cache: Arc<Mutex<KvCache>>,
    state: GenerateState,
    logits: Option<Vec<f32>>,
}

enum GenerateState {
    Tokenize,
    InitialForward,
    Sampling(usize),
    TokenForward(usize),
    Done,
}

impl TaskIterator for GenerateTask {
    type Ready = GenerationOutput;
    type Pending = GenerateProgress;
    type Spawner = NoSpawner;

    fn next_status(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            GenerateState::Tokenize => {
                // Tokenize prompt
                let tokens = self.tokenizer.encode(&self.prompt);
                self.tokens = tokens;
                self.state = GenerateState::InitialForward;
                Some(TaskStatus::Pending(GenerateProgress::Tokenize))
            }

            GenerateState::InitialForward => {
                // Initial forward pass through prompt
                let positions: Vec<i32> = (0..self.tokens.len() as i32).collect();
                let forward = ForwardTask {
                    model: self.model.clone(),
                    tokens: self.tokens.clone(),
                    positions,
                    kv_cache: self.kv_cache.clone(),
                    layer: 0,
                    hidden: None,
                    state: ForwardState::Embedding,
                };

                // Execute forward (in real implementation, this would be iterative)
                let logits = execute_forward(forward);
                self.logits = Some(logits);
                self.state = GenerateState::Sampling(0);
                Some(TaskStatus::Pending(GenerateProgress::PromptProcessing))
            }

            GenerateState::Sampling(count) => {
                if count >= self.max_tokens {
                    self.state = GenerateState::Done;
                    return Some(TaskStatus::Ready(self.build_output()));
                }

                // Sample next token
                let sample = SampleTask {
                    logits: self.logits.clone().unwrap(),
                    temperature: self.temperature,
                    top_k: self.top_k,
                    top_p: self.top_p,
                    state: SampleState::Init,
                };

                let next_token = execute_sample(sample);

                // Check for EOS
                if self.tokenizer.is_eos(next_token) {
                    self.state = GenerateState::Done;
                    return Some(TaskStatus::Ready(self.build_output()));
                }

                self.generated.push(next_token);
                self.tokens.push(next_token);
                self.state = GenerateState::TokenForward(count + 1);
                Some(TaskStatus::Pending(GenerateProgress::TokenSampled(count)))
            }

            GenerateState::TokenForward(count) => {
                // Forward pass for single token
                let positions = vec![self.tokens.len() as i32 - 1];
                let forward = ForwardTask {
                    model: self.model.clone(),
                    tokens: vec![*self.tokens.last().unwrap()],
                    positions,
                    kv_cache: self.kv_cache.clone(),
                    layer: 0,
                    hidden: None,
                    state: ForwardState::Embedding,
                };

                let logits = execute_forward(forward);
                self.logits = Some(logits);
                self.state = GenerateState::Sampling(count);
                Some(TaskStatus::Pending(GenerateProgress::TokenProcessing(count)))
            }

            GenerateState::Done => {
                Some(TaskStatus::Ready(self.build_output()))
            }
        }
    }
}

impl GenerateTask {
    fn build_output(&self) -> GenerationOutput {
        let text = self.tokenizer.decode(&self.generated).unwrap_or_default();
        GenerationOutput {
            text,
            tokens: self.generated.clone(),
            usage: UsageInfo {
                prompt_tokens: self.tokens.len() - self.generated.len(),
                completion_tokens: self.generated.len(),
                total_tokens: self.tokens.len(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum GenerateProgress {
    Tokenize,
    PromptProcessing,
    TokenSampled(usize),
    TokenProcessing(usize),
}

#[derive(Debug, Clone)]
pub struct GenerationOutput {
    pub text: String,
    pub tokens: Vec<u32>,
    pub usage: UsageInfo,
}

#[derive(Debug, Clone)]
pub struct UsageInfo {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}
```

---

## 5. HTTP API Compatibility

### 5.1 Lambda Function Handler

```rust
use aws_lambda_events::{
    apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse},
    encodings::Body,
};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::json;

/// Lambda handler for OpenAI-compatible API
async fn function_handler(event: LambdaEvent<ApiGatewayProxyRequest>) -> Result<ApiGatewayProxyResponse, Error> {
    // Initialize valtron executor (once per cold start)
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        valtron::single::initialize_pool(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        );
    });

    // Parse request
    let request: ChatCompletionRequest = match serde_json::from_str(&event.payload.body) {
        Ok(req) => req,
        Err(e) => {
            return Ok(ApiGatewayProxyResponse {
                status_code: 400,
                body: Body::Text(format!("Invalid request: {}", e)),
                ..Default::default()
            });
        }
    };

    // Get model (from global state)
    let model = get_model(&request.model)?;

    // Extract prompt from messages
    let prompt = build_prompt(&request.messages)?;

    // Create generation task
    let task = GenerateTask {
        model: model.clone(),
        tokenizer: model.tokenizer.clone(),
        prompt,
        max_tokens: request.max_tokens.unwrap_or(100),
        temperature: request.temperature.unwrap_or(0.7),
        top_k: 40,
        top_p: request.top_p.unwrap_or(0.9),
        tokens: Vec::new(),
        generated: Vec::new(),
        kv_cache: model.kv_cache.clone(),
        state: GenerateState::Tokenize,
        logits: None,
    };

    // Execute generation
    let output = valtron::single::spawn()
        .with_task(task)
        .schedule_iter(std::time::Duration::from_millis(0))?;

    valtron::single::run_until_complete();

    let output = output.collect().next().unwrap();

    // Build response
    let response = ChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        object: "chat.completion",
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        model: request.model,
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: "assistant",
                content: output.text,
            },
            finish_reason: "stop",
        }],
        usage: output.usage,
    };

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: {
            let mut h = HashMap::new();
            h.insert("Content-Type".to_string(), "application/json".to_string());
            h.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
            h
        },
        body: Body::Text(serde_json::to_string(&response)?),
        ..Default::default()
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
```

### 5.2 Streaming Response

```rust
/// Streaming Lambda handler (SSE)
async fn streaming_handler(event: LambdaEvent<ApiGatewayProxyRequest>) -> Result<ApiGatewayProxyResponse, Error> {
    // ... setup same as above ...

    // Create channel for streaming
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    // Spawn generation in background
    let task = GenerateTask { /* ... */ };

    // Execute with streaming callback
    valtron::single::spawn()
        .with_task(task)
        .with_callback(move |progress: GenerateProgress| {
            if let GenerateProgress::TokenSampled(_) = progress {
                let _ = tx.send(progress);
            }
        })
        .schedule_iter(std::time::Duration::from_millis(0))?;

    // Build SSE response
    let mut body = String::new();
    while let Some(progress) = rx.recv().await {
        if let GenerateProgress::TokenSampled(idx) = progress {
            body.push_str(&format!("data: {}\n\n", json!({
                "id": "chatcmpl-xxx",
                "object": "chat.completion.chunk",
                "created": timestamp,
                "model": model_name,
                "choices": [{
                    "index": 0,
                    "delta": {"content": token},
                    "finish_reason": null,
                }]
            })));
        }
    }

    Ok(ApiGatewayProxyResponse {
        status_code: 200,
        headers: {
            let mut h = HashMap::new();
            h.insert("Content-Type".to_string(), "text/event-stream".to_string());
            h.insert("Cache-Control".to_string(), "no-cache".to_string());
            h.insert("Connection".to_string(), "keep-alive".to_string());
            h
        },
        body: Body::Text(body),
        ..Default::default()
    })
}
```

---

## 6. Lambda Deployment Guide

### 6.1 Project Setup

```toml
# Cargo.toml
[package]
name = "llama-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
# Valtron executor
valtron = { path = "/path/to/valtron" }

# Lambda runtime (minimal, no tokio)
lambda_runtime = "0.11"
aws_lambda_events = "0.15"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Model loading
memmap2 = "0.9"  # Memory-mapped files

# Utilities
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }

[profile.release]
opt-level = 3
lto = true
strip = true
```

### 6.2 Build Script

```bash
#!/bin/bash
# build.sh

# Cross-compile for Lambda (Amazon Linux 2)
cargo build --release --target x86_64-unknown-linux-musl

# Create deployment package
mkdir -p deployment
cp target/x86_64-unknown-linux-musl/release/llama-lambda deployment/bootstrap

# Add model weights as layer (if small enough)
# Or download from S3 on cold start

# Create zip
cd deployment
zip -r ../lambda-deployment.zip .
cd ..

# Deploy
aws lambda update-function-code \
    --function-name llama-inference \
    --zip-file fileb://lambda-deployment.zip
```

### 6.3 Terraform Configuration

```hcl
# terraform/main.tf

resource "aws_lambda_function" "llama_inference" {
    function_name = "llama-inference"
    role          = aws_iam_role.lambda_role.arn
    handler       = "bootstrap"
    runtime       = "provided.al2"
    architectures = ["x86_64"]
    timeout       = 900  # 15 minutes max
    memory_size   = 10240  # 10 GB for model weights

    filename         = "lambda-deployment.zip"
    source_code_hash = filebase64sha256("lambda-deployment.zip")

    ephemeral_storage {
        size = 10240  # 10 GB /tmp for model caching
    }

    environment {
        variables = {
            MODEL_PATH     = "/opt/models/llama-3.2-1b.gguf"
            MAX_TOKENS     = "256"
            TEMPERATURE    = "0.7"
        }
    }

    layers = [aws_lambda_layer_version.model_weights.arn]
}

resource "aws_lambda_layer_version" "model_weights" {
    layer_name = "llama-model-weights"
    content    = filebase64sha256("model-layer.zip")
    filename   = "model-layer.zip"
}

resource "aws_api_gateway_rest_api" "llama_api" {
    name = "llama-api"

    body = jsonencode({
        openapi = "3.0.1"
        info = { title = "LLM API", version = "1.0" }
        paths = {
            "/v1/chat/completions" = {
                post = {
                    "x-amazon-apigateway-integration" = {
                        type = "AWS_PROXY"
                        httpMethod = "POST"
                        uri = "arn:aws:apigateway:${var.region}:lambda:path/2015-03-31/functions/${aws_lambda_function.llama_inference.arn}/invocations"
                    }
                }
            }
        }
    })
}
```

### 6.4 SAM Template

```yaml
# template.yaml
AWSTemplateFormatVersion: '2010-09-09'
Transform: AWS::Serverless-2016-10-31

Resources:
  LlamaInference:
    Type: AWS::Serverless::Function
    Properties:
      FunctionName: llama-inference
      Runtime: provided.al2
      Architecture: x86_64
      Handler: bootstrap
      Timeout: 900
      MemorySize: 10240
      EphemeralStorage:
        Size: 10240

      Environment:
        Variables:
          MODEL_PATH: /tmp/model.gguf
          S3_MODEL_BUCKET: !Ref ModelBucket

      Layers:
        - !Ref ModelLayer

      Events:
        ApiEvent:
          Type: Api
          Properties:
            Path: /v1/chat/completions
            Method: post

  ModelLayer:
    Type: AWS::Serverless::LayerVersion
    Properties:
      LayerName: llama-model-weights
      ContentUri: s3://my-bucket/model-layer.zip
      CompatibleRuntimes:
        - provided.al2

Outputs:
  ApiEndpoint:
    Description: API Gateway endpoint URL
    Value: !Sub "https://${ServerlessRestApi}.execute-api.${AWS::Region}.amazonaws.com/Prod/v1/chat/completions"
```

---

## 7. Performance Optimization

### 7.1 Cold Start Optimization

```rust
// Model lazy loading with memory mapping
static MODEL: OnceLock<Arc<LlamaModel>> = OnceLock::new();

fn get_model() -> Arc<LlamaModel> {
    MODEL.get_or_init(|| {
        let model_path = std::env::var("MODEL_PATH")
            .unwrap_or_else(|_| "/tmp/model.gguf".to_string());

        // Memory-map the model file (zero-copy loading)
        let file = File::open(&model_path).expect("Failed to open model");
        let mmap = unsafe {
            MmapOptions::new().map(&file).expect("Failed to mmap model")
        };

        // Parse GGUF header and tensor info (fast)
        // Weights remain on disk, loaded on-demand
        load_model_from_mmap(mmap)
    }).clone()
}

// Cold start breakdown:
// - Lambda initialization: ~50ms
// - Model mmap: ~10ms
// - GGUF parsing: ~50ms
// Total cold start: ~110ms (vs 500ms+ with tokio)
```

### 7.2 Warm Start Caching

```rust
// Keep model in memory between invocations
struct ModelCache {
    model: Option<Arc<LlamaModel>>,
    last_used: Instant,
}

static CACHE: Mutex<Option<ModelCache>> = Mutex::new(None);

fn get_cached_model() -> Arc<LlamaModel> {
    let mut cache = CACHE.lock().unwrap();

    if let Some(cached) = cache.as_ref() {
        if cached.last_used.elapsed() < Duration::from_secs(300) {
            return cached.model.clone();
        }
    }

    // Load new model
    let model = load_model();
    *cache = Some(ModelCache {
        model: model.clone(),
        last_used: Instant::now(),
    });
    model
}
```

### 7.3 Batching for Throughput

```
Lambda with provisioned concurrency + batching:

Request Queue (API Gateway)
       │
       ▼
┌──────────────────┐
│  Batch Collector │  ← Collect requests for 50ms
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  Valtron Batch   │  ← Process all requests together
│  Inference Task  │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  Response Split  │  ← Split results per request
└────────┬─────────┘
         │
         ▼
    Individual Responses

Throughput improvement: 3-5x for high traffic
```

---

## 8. Cost Optimization

### 8.1 Lambda Pricing Comparison

```
Standard async Lambda (tokio):
- Cold start: 500ms = $0.0000083 per invocation
- Generation (100 tokens, 10 GB): 10s = $0.0001667
- Total per request: ~$0.000175

Valtron Lambda:
- Cold start: 100ms = $0.0000017 per invocation
- Generation (100 tokens, 10 GB): 10s = $0.0001667
- Total per request: ~$0.000168

Savings: ~4% per request (mainly from cold start)

Monthly cost (1M requests, 50% cold):
- Standard: $175
- Valtron: $168
- Savings: $7/month per function

BUT: Provisioned concurrency changes everything!
```

### 8.2 Provisioned Concurrency

```
With provisioned concurrency (no cold starts):

Provisioned: 10 instances, 24/7
- Cost: 10 × 24 × 30 × $0.00000444/GB-s × 10 GB = $319.68/month

Request cost (generation only):
- 1M requests × 10s × $0.000000001667/GB-s × 10 GB = $166.70

Total: $486.38/month for 1M requests = $0.000486 per request

vs Self-hosted (EC2):
- 4x A10 instance: ~$4/hour = $2880/month
- Can handle ~10M requests/month
- Cost per request: $0.000288

Break-even: ~5M requests/month
- Below: Lambda is cheaper
- Above: EC2 is cheaper
```

### 8.3 Optimization Strategies

```
1. Use smaller models (1-3B) for Lambda
   - Lower memory = lower cost
   - Faster inference = less duration

2. Implement request batching
   - Process multiple requests together
   - Amortize model loading cost

3. Use Spot Instances for self-hosted
   - 60-70% discount vs on-demand
   - Accept occasional interruptions

4. Hybrid approach:
   - Lambda for low traffic / cold start
   - EC2 for high traffic / warm
   - Route based on current load
```

---

## Summary

### Key Takeaways

1. **Valtron eliminates async runtime overhead** - 50-200ms faster cold starts
2. **Memory-mapped model loading** - Zero-copy, fast initialization
3. **Lambda is cost-effective for < 5M requests/month** - Above that, use EC2
4. **Provisioned concurrency removes cold starts** - Worth it for production
5. **WASM-compatible** - Can deploy to Lambda WebAssembly for even faster starts

### Deployment Checklist

- [ ] Cross-compile for x86_64-unknown-linux-musl
- [ ] Memory-map model weights
- [ ] Configure ephemeral storage (10 GB)
- [ ] Set appropriate timeout (up to 15 min)
- [ ] Enable provisioned concurrency for production
- [ ] Set up CloudWatch alarms
- [ ] Configure API Gateway with proper timeouts
- [ ] Test cold start latency
- [ ] Monitor memory usage
- [ ] Set up cost alerts

---

*This guide demonstrates serverless LLM inference without tokio. For production deployments, consider hybrid Lambda + EC2 architectures.*
