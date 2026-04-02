---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/llamacpp/TabbyML
repository: https://github.com/TabbyML/tabby
explored_at: 2026-04-02
---

# Zero to TabbyML: Complete Implementation Guide

## Introduction

This guide takes you from zero knowledge to building and deploying a production-ready TabbyML instance. We cover everything from basic concepts to advanced customization.

## Table of Contents

1. [Understanding What TabbyML Is](#1-understanding-what-tabbymL-is)
2. [Core Architecture Components](#2-core-architecture-components)
3. [Building from Source](#3-building-from-source)
4. [Model Selection and Deployment](#4-model-selection-and-deployment)
5. [Configuring Code Completion](#5-configuring-code-completion)
6. [Setting Up Chat and Answer Engine](#6-setting-up-chat-and-answer-engine)
7. [Indexing Your Codebase](#7-indexing-your-codebase)
8. [IDE Integration](#8-ide-integration)
9. [Enterprise Features](#9-enterprise-features)
10. [Performance Optimization](#10-performance-optimization)
11. [Custom Extensions](#11-custom-extensions)
12. [Troubleshooting](#12-troubleshooting)

---

## 1. Understanding What TabbyML Is

### The Problem

GitHub Copilot is great, but:
- Requires cloud connectivity
- Code is sent to external servers
- No control over the model
- Expensive at scale
- Cannot be self-hosted

### The TabbyML Solution

TabbyML provides:
- **Self-hosted** - Runs on your infrastructure
- **Privacy-first** - Code never leaves your network
- **Customizable** - Fine-tune on your codebase
- **Cost-effective** - No per-user licensing
- **Open source** - Full transparency and control

### Core Capabilities

1. **Code Completion** - FIM (Fill-In-the-Middle) completions
2. **Chat Interface** - Natural language code assistance
3. **Answer Engine** - RAG over your codebase
4. **Repository Context** - Intelligent code suggestions

---

## 2. Core Architecture Components

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Client Layer                              │
│  VSCode Extension │ JetBrains Plugin │ CLI │ REST API       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    API Layer                                 │
│  /v1/completions  │ /v1/chat/completions │ /v1/search       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Application Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Completion  │  │     Chat     │  │   Answer Engine  │   │
│  │   Service    │  │   Service    │  │     (RAG)        │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Inference Layer                            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  llama.cpp Server (llama-server subprocess)          │   │
│  │  - GGUF model loading                                │   │
│  │  - GPU acceleration (CUDA/ROCm/Metal)               │   │
│  │  - Continuous batching                               │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Storage Layer                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Tantivy     │  │   SQLite     │  │   File System    │   │
│  │  (Search)    │  │   (Auth)     │  │   (Models)       │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### 1. Inference Engine (llama.cpp)

The core inference is handled by llama.cpp:
- Loads GGUF format models
- Supports CPU, CUDA, ROCm, Metal
- Provides HTTP API for completions

**Key files:**
- `crates/llama-cpp-server/src/supervisor.rs` - Process management
- `crates/llama-cpp-server/src/lib.rs` - API bindings

#### 2. Search Index (Tantivy)

Tantivy provides full-text search:
- Code indexing with tree-sitter
- BM25 scoring for relevance
- Chunked document storage

**Key files:**
- `crates/tabby-index/src/indexer.rs` - Document indexing
- `crates/tabby-index/src/code/` - Code-specific logic

#### 3. HTTP Server (Axum)

Axum handles REST API:
- OpenAI-compatible endpoints
- JWT authentication
- Rate limiting

**Key files:**
- `crates/tabby/src/serve.rs` - Server setup
- `ee/tabby-webserver/src/webserver.rs` - Enterprise server

#### 4. Scheduler

Background job scheduler:
- Repository syncing
- Index rebuilding
- Model downloading

**Key files:**
- `crates/tabby/src/scheduler.rs` - Cron jobs

---

## 3. Building from Source

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install dependencies (Ubuntu/Debian)
sudo apt-get install protobuf-compiler libopenblas-dev

# Install dependencies (macOS)
brew install protobuf

# Install optional tools
sudo apt-get install make sqlite3 graphviz
```

### Clone and Build

```bash
# Clone with submodules
git clone --recurse-submodules https://github.com/TabbyML/tabby
cd tabby

# Build release version
cargo build --release

# Run tests (skip golden tests)
cargo test -- --skip golden
```

### Run Locally

```bash
# CPU inference
cargo run -- serve --model TabbyML/StarCoder-1B

# CUDA (NVIDIA)
cargo run --features cuda -- serve --model TabbyML/StarCoder-1B --device cuda

# ROCm (AMD)
cargo run --features rocm -- serve --model TabbyML/StarCoder-1B --device rocm

# Metal (Apple Silicon)
cargo run -- serve --model TabbyML/StarCoder-1B --device metal
```

### Docker Deployment

```bash
# Basic Docker run
docker run -it \
  --gpus all \
  -p 8080:8080 \
  -v $HOME/.tabby:/data \
  tabbyml/tabby \
  serve --model StarCoder-1B --device cuda

# With chat model
docker run -it \
  --gpus all \
  -p 8080:8080 \
  -v $HOME/.tabby:/data \
  tabbyml/tabby \
  serve \
    --model StarCoder-1B \
    --chat-model Qwen2-1.5B-Instruct \
    --device cuda
```

---

## 4. Model Selection and Deployment

### Available Models

Tabby supports GGUF format models from HuggingFace:

| Model | Size | Speed | Quality | Use Case |
|-------|------|-------|---------|----------|
| StarCoder-1B | 1B | Fast | Good | Basic completion |
| StarCoder-3B | 3B | Medium | Better | Better quality |
| CodeLlama-7B | 7B | Slow | Best | High quality |
| Qwen2-1.5B-Instruct | 1.5B | Fast | Good | Chat |

### Model Registry Structure

Models are stored in `~/.tabby/models/`:

```
~/.tabby/models/
├── TabbyML/
│   └── StarCoder-1B/
│       ├── tabby.json          # Model metadata
│       └── ggml/
│           └── model-00001-of-00001.gguf
```

### tabby.json Format

```json
{
    "prompt_template": "<PRE>{prefix}<SUF>{suffix}<MID>",
    "chat_template": "<s>{% for message in messages %}{% if message['role'] == 'user' %}{{ '[INST] ' + message['content'] + ' [/INST]' }}{% elif message['role'] == 'assistant' %}{{ message['content'] + '</s> ' }}{% endif %}{% endfor %}"
}
```

### Downloading Models

```bash
# Models are auto-downloaded on first use

# Manual download using tabby download
tabby download --model TabbyML/StarCoder-1B

# Or use huggingface-cli
huggingface-cli download TabbyML/StarCoder-1B \
  --local-dir ~/.tabby/models/TabbyML/StarCoder-1B
```

### Custom Models

To use a custom model:

1. Convert to GGUF format:
```bash
python convert-hf-to-gguf.py path/to/model --outfile model.gguf
```

2. Create model directory:
```bash
mkdir -p ~/.tabby/models/MyModel
cp model.gguf ~/.tabby/models/MyModel/ggml/model-00001-of-00001.gguf
```

3. Create tabby.json:
```json
{
    "prompt_template": "<PRE>{prefix}<SUF>{suffix}<MID>"
}
```

4. Configure server:
```toml
[model.completion]
model = "~/.tabby/models/MyModel"
```

---

## 5. Configuring Code Completion

### Server Configuration

Create `~/.tabby/config.toml`:

```toml
# Model configuration
[model.completion]
model = "TabbyML/StarCoder-1B"
num_gpu_layers = 35  # Layers to offload to GPU
parallelism = 4      # Parallel requests

[model.chat]
model = "Qwen/Qwen2-1.5B-Instruct"
num_gpu_layers = 25

# Server configuration
[server]
host = "0.0.0.0"
port = 8080

# Optional: Additional stop words
[completion.additional_stop_words]
python = ["\n\nclass ", "\n\nif __name__"]
rust = ["\n\nfn ", "\n}"]
```

### Completion Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `max_decoding_tokens` | Max tokens to generate | 256 |
| `sampling_temperature` | Sampling temperature | 0.1 |
| `seed` | Random seed | Auto |
| `presence_penalty` | Penalty for repetition | 0.0 |

### API Usage

```bash
# Completion API
curl -X POST http://localhost:8080/v1/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "TabbyML/StarCoder-1B",
    "prompt": "def fibonacci(n):\n    ",
    "max_tokens": 50,
    "temperature": 0.1
  }'
```

### Response Format

```json
{
    "id": "cmpl-xxx",
    "choices": [{
        "index": 0,
        "text": "if n <= 1:\n        return n\n    return fibonacci(n-1) + fibonacci(n-2)",
        "finish_reason": "stop"
    }],
    "usage": {
        "prompt_tokens": 10,
        "completion_tokens": 25,
        "total_tokens": 35
    }
}
```

---

## 6. Setting Up Chat and Answer Engine

### Chat Configuration

Enable chat in config.toml:

```toml
[model.chat]
model = "Qwen/Qwen2-1.5B-Instruct"
num_gpu_layers = 25
```

### Chat API

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "Qwen/Qwen2-1.5B-Instruct",
    "messages": [
        {"role": "system", "content": "You are a coding assistant."},
        {"role": "user", "content": "How do I write a binary search in Rust?"}
    ]
  }'
```

### Answer Engine (RAG)

The Answer Engine provides RAG over your codebase:

1. **Index your codebase:**
```bash
tabby scheduler --now
```

2. **Query via API:**
```bash
curl -X POST http://localhost:8080/v1/answer \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How is authentication implemented?",
    "source_id": "git://github.com/myorg/myrepo"
  }'
```

### RAG Pipeline

1. **Query Analysis** - Parse intent
2. **Retrieval** - BM25 + semantic search
3. **Reranking** - Score by relevance
4. **Generation** - Generate with context
5. **Citation** - Link sources

---

## 7. Indexing Your Codebase

### Repository Configuration

Create `~/.tabby/repositories.toml`:

```toml
[[repositories]]
name = "my-project"
git_url = "https://github.com/myorg/myproject.git"

[[repositories]]
name = "internal-repo"
git_url = "ssh://git@github.com/myorg/internal.git"
```

### Manual Indexing

```bash
# Trigger immediate indexing
tabby scheduler --now

# Check index status
tabby scheduler --status
```

### Index Structure

```
~/.tabby/index/
├── meta.json           # Index metadata
├── segments/           # Tantivy segments
└── commitlog/          # Write-ahead log
```

### Supported Languages

Tabby uses tree-sitter for parsing:

- Rust
- Python
- TypeScript/JavaScript
- Go
- Java
- C/C++
- Ruby
- PHP
- Swift
- Kotlin

### Custom Language Support

Add language to `~/.tabby/languages.toml`:

```toml
[[languages]]
name = "MyLang"
extensions = ["myl", "mylang"]
top_level_keywords = ["func", "struct", "impl"]
comment_chars = ["//", "/*"]
```

---

## 8. IDE Integration

### VSCode Extension

1. Install from marketplace: `TabbyML.vscode-tabby`

2. Configure settings.json:
```json
{
    "tabby.server.endpoint": "http://localhost:8080",
    "tabby.completion.triggerMode": "auto",
    "tabby.chat.enabled": true
}
```

3. Keybindings:
```json
{
    "key": "ctrl+space",
    "command": "tabby.acceptCompletion"
},
{
    "key": "ctrl+\\",
    "command": "tabby.triggerCompletion"
}
```

### JetBrains Plugin

1. Install from marketplace

2. Configure:
   - Settings → Tools → Tabby
   - Set endpoint URL

### Vim Plugin

```vim
" In .vimrc
 Plug 'TabbyML/vim-tabby'

" In .config/nvim/init.lua
require('tabby').setup({
  server = 'http://localhost:8080',
})
```

### Custom Integration (LSP)

```typescript
// LSP client example
const client = new LanguageClient(
  'tabby',
  'Tabby Language Server',
  {
    command: 'tabby-agent',
    args: ['--stdio']
  },
  {
    documentSelector: [{ scheme: 'file', language: '*' }]
  }
);

client.start();
```

---

## 9. Enterprise Features

### Authentication

Enable JWT authentication:

```toml
[server.auth]
jwt_secret = "your-secret-key"
token_expiry = 86400  # 24 hours
```

### Team Management

```bash
# Create team
tabby team create my-team

# Add user
tabby team add-member my-team user@example.com

# List members
tabby team list-members my-team
```

### Usage Analytics

Access analytics at `/dashboard/reports`:
- Completion counts
- Active users
- Language breakdown
- Acceptance rates

### SSO Integration

Configure SSO in `config.toml`:

```toml
[server.oauth]
providers = ["github", "google"]

[server.ldap]
enabled = true
host = "ldap.example.com"
base_dn = "dc=example,dc=com"
```

---

## 10. Performance Optimization

### GPU Optimization

```toml
[model.completion]
num_gpu_layers = 35  # Offload more layers to GPU
enable_fast_attention = true  # Use flash attention
context_size = 4096  # Larger context
```

### Memory Management

```bash
# Limit RAM usage
export LLAMA_CPP_N_THREADS=4

# Set embedding batch size
export LLAMA_CPP_EMBEDDING_N_UBATCH_SIZE=2048
```

### Caching Strategy

```toml
[server.cache]
completion_cache_size = 10000  # Max cached completions
cache_ttl = 3600  # Cache TTL in seconds
```

### Scaling

For high-traffic deployments:

1. **Horizontal scaling** - Multiple instances behind load balancer
2. **Model sharding** - Different models for different languages
3. **Caching layer** - Redis for completion caching

```bash
# Load balancer configuration (nginx example)
upstream tabby_backend {
    server tabby1:8080;
    server tabby2:8080;
    server tabby3:8080;
}
```

---

## 11. Custom Extensions

### Building Custom Post-processors

```typescript
// Custom post-processor
import { PostprocessFilter } from './base';

export const customFilter: PostprocessFilter = async (item, context) => {
    // Your custom logic here
    if (item.text.includes('TODO')) {
        item.text = item.text.replace('TODO', 'FIXME');
    }
    return item;
};
```

### Custom Chat Prompts

Create prompt templates in `~/.tabby/prompts/`:

```markdown
<!-- custom-system-prompt.md -->
You are an expert {{language}} developer.
Focus on:
1. Performance
2. Readability
3. Best practices

Current file: {{filepath}}
```

### Custom Index Attributes

```rust
// Custom attribute builder
pub struct MyAttributeBuilder;

#[async_trait]
impl IndexAttributeBuilder<CodeDocument> for MyAttributeBuilder {
    async fn build_attributes(&self, doc: &CodeDocument) -> serde_json::Value {
        json!({
            "author": doc.author,
            "complexity": calculate_complexity(&doc.content)
        })
    }
}
```

---

## 12. Troubleshooting

### Common Issues

#### 1. Model Not Loading

**Symptoms:** Server starts but no completions

**Solution:**
```bash
# Check model exists
ls ~/.tabby/models/TabbyML/StarCoder-1B/ggml/

# Re-download model
tabby download --model TabbyML/StarCoder-1B --force
```

#### 2. High Latency

**Symptoms:** Completions take >5 seconds

**Solution:**
```toml
# Reduce context size
[model.completion]
context_size = 2048

# Use smaller model
model = "TabbyML/StarCoder-1B"

# Enable GPU
num_gpu_layers = 35
```

#### 3. Index Not Updating

**Symptoms:** Old code in suggestions

**Solution:**
```bash
# Force reindex
rm -rf ~/.tabby/index/*
tabby scheduler --now
```

#### 4. CUDA Out of Memory

**Symptoms:** `cudaMalloc` error

**Solution:**
```toml
# Reduce GPU layers
[model.completion]
num_gpu_layers = 20

# Use smaller model
model = "TabbyML/StarCoder-1B"
```

### Debug Mode

Enable verbose logging:

```bash
RUST_LOG=debug tabby serve --model TabbyML/StarCoder-1B
```

### Health Check

```bash
curl http://localhost:8080/v1/health

# Expected response
{
    "model": "TabbyML/StarCoder-1B",
    "device": "cuda",
    "arch": "x86_64"
}
```

### Performance Metrics

Access metrics at `/metrics` (Prometheus format):

```
# HELP tabby_completion_requests_total Total completion requests
# TYPE tabby_completion_requests_total counter
tabby_completion_requests_total{status="success"} 1234
tabby_completion_requests_total{status="error"} 56
```

---

## Conclusion

You now have a complete understanding of TabbyML from zero to production. The key takeaways:

1. **Architecture** - Understand the inference, search, and API layers
2. **Configuration** - Tune models, caching, and GPU settings
3. **Integration** - Connect your IDEs and workflows
4. **Optimization** - Balance speed, quality, and resources
5. **Extensibility** - Customize for your needs

Next steps:
- Explore deep-dive documents for specific components
- Read the rust-revision for implementation details
- Check production-grade for deployment patterns
