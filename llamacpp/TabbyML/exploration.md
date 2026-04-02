---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.TabbyML
repository: https://github.com/TabbyML/tabby
explored_at: 2026-04-02
language: Rust, TypeScript, Python, C++
---

# Project Exploration: TabbyML

## Overview

TabbyML is a **self-hosted AI coding assistant** that provides an open-source, on-premises alternative to GitHub Copilot. It combines large language models with intelligent code indexing to deliver context-aware code completions and chat-based assistance directly in your IDE.

### Key Value Proposition

- **Self-contained deployment** - No external DBMS or cloud services required
- **Consumer GPU support** - Runs on NVIDIA CUDA, AMD ROCm, Apple Metal, or CPU
- **OpenAPI interface** - Easy integration with existing infrastructure (Cloud IDEs, custom editors)
- **Enterprise features** - Team management, access controls, usage analytics
- **Multi-IDE support** - VSCode, JetBrains, Vim, Eclipse extensions available

### Architecture Summary

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         IDE Extensions                                   │
│  VSCode │ JetBrains │ Vim │ Eclipse │ Custom (LSP)                      │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTP/gRPC
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Tabby Server (Rust)                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐  │
│  │  Completion API │  │   Chat API      │  │  Answer Engine          │  │
│  │  - FIM prompts  │  │  - OpenAI compat│  │  - RAG over codebase    │  │
│  │  - Context fill │  │  - Thread state │  │  - Semantic search      │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    Core Services                                  │   │
│  │  ┌─────────────┐ ┌──────────────┐ ┌─────────────┐ ┌───────────┐  │   │
│  │  │   Scheduler │ │   Index     │ │  Inference  │ │  Registry │  │   │
│  │  │  (cron jobs)│ │  (Tantivy)  │ │  (llama.cpp)│ │  (models) │  │   │
│  │  └─────────────┘ └──────────────┘ └─────────────┘ └───────────┘  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                 Enterprise (ee/)                                   │   │
│  │  ┌─────────────┐ ┌──────────────┐ ┌─────────────┐                │   │
│  │  │  Webserver  │ │   Database   │ │    UI       │                │   │
│  │  │  (Axum+JWT) │ │   (SQLx)     │ │  (Next.js)  │                │   │
│  │  └─────────────┘ └──────────────┘ └─────────────┘                │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ File System / Git
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Data & Indexes                                        │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐  │
│  │  Code Index     │  │  Structured     │  │   Model Cache           │  │
│  │  (Tantivy)      │  │  Docs (Git/Jira)│  │   (GGUF files)          │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

## Repository Structure

The TabbyML repository is organized as a Rust workspace with multiple crates:

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.TabbyML/
├── tabby/                              # Main Tabby repository
│   ├── Cargo.toml                      # Workspace definition
│   ├── crates/                         # Open source crates
│   │   ├── tabby/                      # Main binary crate (CLI + API)
│   │   │   └── src/
│   │   │       ├── main.rs             # Entry point
│   │   │       ├── serve.rs            # HTTP server setup
│   │   │       ├── scheduler.rs        # Background job scheduler
│   │   │       └── api/                # API route handlers
│   │   │
│   │   ├── tabby-common/               # Shared types and utilities
│   │   │   └── src/
│   │   │       ├── config.rs           # Configuration structures
│   │   │       ├── registry.rs         # Model registry
│   │   │       ├── languages.rs        # Language definitions
│   │   │       ├── path.rs             # Path utilities
│   │   │       ├── index/              # Index schema
│   │   │       └── axum.rs             # Axum integration
│   │   │
│   │   ├── tabby-inference/            # Inference interfaces
│   │   │   └── src/
│   │   │       ├── lib.rs              # Core traits
│   │   │       ├── completion.rs       # CompletionStream trait
│   │   │       ├── chat.rs             # ChatCompletionStream trait
│   │   │       ├── code.rs             # CodeGeneration wrapper
│   │   │       ├── decoding.rs         # Stop condition factory
│   │   │       └── embedding.rs        # Embedding trait
│   │   │
│   │   ├── llama-cpp-server/           # llama.cpp supervision
│   │   │   └── src/
│   │   │       ├── lib.rs              # Server orchestration
│   │   │       └── supervisor.rs       # Process management
│   │   │
│   │   ├── tabby-download/             # Model downloading
│   │   │   └── src/
│   │   │       └── lib.rs              # HuggingFace downloads
│   │   │
│   │   ├── tabby-index/                # Code indexing (Tantivy)
│   │   │   └── src/
│   │   │       ├── lib.rs              # Indexer orchestration
│   │   │       ├── indexer.rs          # Document indexing
│   │   │       ├── code/               # Code-specific indexing
│   │   │       │   ├── mod.rs
│   │   │       │   ├── index.rs        # Main indexing logic
│   │   │       │   ├── intelligence.rs # Tree-sitter analysis
│   │   │       │   ├── languages.rs    # Language configurations
│   │   │       │   ├── repository.rs   # Git repository handling
│   │   │       │   └── types.rs        # Code document types
│   │   │       ├── structured_doc/     # Non-code documents
│   │   │       │   ├── mod.rs
│   │   │       │   ├── public.rs       # Public API
│   │   │       │   └── types/          # Doc type definitions
│   │   │       └── tantivy_utils.rs    # Index utilities
│   │   │
│   │   ├── tabby-git/                  # Git operations
│   │   │   └── src/
│   │   │       └── lib.rs              # Git2 bindings
│   │   │
│   │   ├── tabby-crawler/              # Web crawling
│   │   │   └── src/
│   │   │       └── lib.rs              # Content fetching
│   │   │
│   │   ├── http-api-bindings/          # HTTP model bindings
│   │   │   └── src/
│   │   │       ├── completion.rs       # OpenAI completion API
│   │   │       ├── chat.rs             # OpenAI chat API
│   │   │       └── embedding.rs        # Embedding API
│   │   │
│   │   ├── ollama-api-bindings/        # Ollama compatibility
│   │   │   └── src/
│   │   │       └── lib.rs
│   │   │
│   │   ├── aim-downloader/             # Async file downloading
│   │   │   └── src/
│   │   │       └── lib.rs
│   │   │
│   │   ├── hash-ids/                   # ID obfuscation
│   │   │   └── src/
│   │   │       └── lib.rs
│   │   │
│   │   └── sqlx-migrate-validate/      # Migration validation
│   │       └── src/
│   │           └── lib.rs
│   │
│   ├── ee/                             # Enterprise features
│   │   ├── tabby-webserver/            # Web server with auth
│   │   │   └── src/
│   │   │       ├── webserver.rs        # Main server
│   │   │       ├── jwt.rs              # JWT authentication
│   │   │       ├── ldap.rs             # LDAP integration
│   │   │       ├── oauth/              # OAuth providers
│   │   │       ├── routes/             # HTTP routes
│   │   │       ├── service/            # Business logic
│   │   │       ├── hub.rs              # Model hub
│   │   │       └── axum/               # Axum middleware
│   │   │
│   │   ├── tabby-db/                   # Database layer
│   │   │   └── src/
│   │   │       ├── cache.rs            # Caching layer
│   │   │       ├── sessions.rs         # Session management
│   │   │       ├── users.rs            # User management
│   │   │       ├── teams.rs            # Team management
│   │   │       ├── tokens.rs           # API tokens
│   │   │       ├── invitations.rs      # Team invitations
│   │   │       ├── repositories.rs     # Repository links
│   │   │       ├── job_runs.rs         # Background jobs
│   │   │       ├── email_setting.rs    # Email configuration
│   │   │       ├── oauth_credential.rs # OAuth credentials
│   │   │       ├── user_event.rs       # User activity tracking
│   │   │       ├── web_crawler.rs      # Web crawl history
│   │   │       └── docs/               # Documentation
│   │   │
│   │   ├── tabby-schema/               # GraphQL schema
│   │   │   └── src/
│   │   │       ├── schema.rs           # Root schema
│   │   │       ├── juniper/            # Juniper integration
│   │   │       └── graphql/            # GraphQL types
│   │   │
│   │   └── tabby-ui/                   # Next.js frontend
│   │       └── app/
│   │           ├── (dashboard)/        # Admin dashboard
│   │           ├── (home)/             # Main chat interface
│   │           ├── search/             # Search UI
│   │           ├── files/              # File browser
│   │           └── pages/              # Shared pages
│   │
│   ├── clients/                        # IDE extensions
│   │   ├── vscode/                     # VSCode extension
│   │   │   └── src/
│   │   │       ├── CompletionProvider.ts
│   │   │       ├── ChatViewProvider.ts
│   │   │       └── ...
│   │   │
│   │   ├── tabby-agent/                # Language server agent
│   │   │   └── src/
│   │   │       ├── codeCompletion/     # Completion logic
│   │   │       │   ├── index.ts        # Main provider
│   │   │       │   ├── contexts.ts     # Context building
│   │   │       │   ├── buildRequest.ts # Request construction
│   │   │       │   ├── cache.ts        # Completion caching
│   │   │       │   ├── debouncer.ts    # Request debouncing
│   │   │       │   ├── solution.ts     # Solution handling
│   │   │       │   ├── statistics.ts   # Usage statistics
│   │   │       │   ├── latencyTracker.ts # Latency monitoring
│   │   │       │   └── postprocess/    # Post-processing filters
│   │   │       │       ├── index.ts
│   │   │       │       ├── limitScope.ts
│   │   │       │       ├── trimSpace.ts
│   │   │       │       ├── dropDuplicated.ts
│   │   │       │       └── ...
│   │   │       ├── chat/               # Chat functionality
│   │   │       ├── config/             # Configuration
│   │   │       ├── http/               # HTTP client
│   │   │       └── protocol.ts         # LSP protocol
│   │   │
│   │   ├── intellij/                   # IntelliJ plugin
│   │   ├── vim/                        # Vim plugin
│   │   ├── eclipse/                    # Eclipse plugin
│   │   ├── tabby-chat-panel/           # Chat panel interface
│   │   └── tabby-threads/              # Threading utilities
│   │
│   ├── docker/                         # Docker configurations
│   ├── ci/                             # CI/CD scripts
│   ├── rules/                          # Model rules
│   └── experimental/                   # Experimental features
│
└── pochi/                              # Pochi AI agent (separate project)
    ├── packages/
    │   ├── vscode/                     # Pochi VSCode extension
    │   ├── vscode-webui/               # Web UI components
    │   ├── cli/                        # CLI tool
    │   ├── livekit/                    # LiveKit integration
    │   └── docs/                       # Documentation
    └── .pochi/                         # Pochi configuration
        ├── agents/                     # AI agents
        └── skills/                     # Agent skills
```

## Core Concepts

### 1. Code Completion with Fill-In-the-Middle (FIM)

Tabby uses the FIM paradigm for code completion, where the model receives both prefix and suffix context:

```
<PRE>{prefix}<SUF>{suffix}<MID>{completion}
```

**Example prompt template:**
```
<PRE>def fibonacci(n):
    if n <= 1:
        return n
    # CURSOR
    print(fibonacci(10))
<SUF>

<MID>
```

### 2. Repository Context

Tabby indexes your codebase to provide repository-aware completions:

- **Declaration snippets** - Type definitions, function signatures from LSP
- **Recently modified code** - Your recent edits are prioritized
- **Git context** - Repository structure and history

### 3. Stop Conditions

The `StopConditionFactory` uses language-specific stop words to terminate generation:

```rust
// Common stop words by language
pub fn get_stop_words(language: &Language) -> Vec<String> {
    match language {
        Language::Python => vec!["\n\nclass ", "\n\ndef ", "\n\nif ", "\n\nprint"],
        Language::Rust => vec!["\n\nfn ", "\n\nimpl ", "\n\nstruct ", "\n}"],
        Language::TypeScript => vec!["\n\nfunction ", "\n\nconst ", "\n\nexport", "\n}"],
        _ => vec![],
    }
}
```

### 4. Model Registry

Models are organized in a registry structure:

```
~/.tabby/models/
├── TabbyML/
│   └── StarCoder-1B/
│       ├── tabby.json          # Model metadata
│       └── ggml/
│           └── model-00001-of-00001.gguf
└── Qwen/
    └── Qwen2-1.5B-Instruct/
        ├── tabby.json
        └── ggml/
            └── model-00001-of-00001.gguf
```

### 5. Index Schema (Tantivy)

The search index uses a structured schema:

```rust
pub struct IndexSchema {
    pub schema: Schema,
    pub field_id: Field,              // Document ID
    pub field_source_id: Field,       // Source (Git repo, etc.)
    pub field_corpus: Field,          // Document type (code, doc)
    pub field_chunk_id: Field,        // Chunk identifier
    pub field_attributes: Field,      // JSON attributes
    pub field_chunk_tokens: Field,    // Tokenized content
    pub field_updated_at: Field,      // Last update timestamp
}
```

## Key Components

### 1. Completion Pipeline

```typescript
// From tabby-agent/src/codeCompletion/index.ts

async function provideCompletions(params: CompletionParams): Promise<CompletionList> {
    // 1. Build completion context
    const context = await buildCompletionContext({
        document,
        position,
        recentlyChangedCode,
        declarations,
        visibleRanges,
    });

    // 2. Check cache
    const cached = await this.cache.get(context);
    if (cached) return cached;

    // 3. Build API request
    const request = buildRequest(context, this.config);

    // 4. Fetch from server
    const response = await this.tabbyApiClient.completion(request);

    // 5. Post-process results
    const processed = await postCacheProcess(
        response.choices,
        context,
        this.config.postprocess
    );

    // 6. Cache and return
    await this.cache.set(context, processed);
    return processed;
}
```

### 2. Post-processing Filters

The completion pipeline applies multiple filters:

```typescript
// From tabby-agent/src/codeCompletion/postprocess/index.ts

export async function postCacheProcess(items, context, config) {
    return Promise.resolve({ items, context })
        .then(applyFilter(removeRepetitiveBlocks))    // Remove repeated patterns
        .then(applyFilter(removeRepetitiveLines))     // Remove repeated lines
        .then(applyFilter(limitScope))                // Limit to current scope
        .then(applyFilter(removeDuplicatedBlockClosingLine))
        .then(applyFilter(formatIndentation))         // Fix indentation
        .then(applyFilter(normalizeIndentation))      // Normalize whitespace
        .then(applyFilter(dropDuplicated))            // Remove duplicates
        .then(applyFilter(trimSpace))                 // Trim whitespace
        .then(applyFilter(removeDuplicateSuffixLines))
        .then(applyFilter(dropMinimum));              // Drop too short
}
```

### 3. Indexing Pipeline

```rust
// From tabby-index/src/indexer.rs

pub async fn index_repository(&self, repo: &GitRepository) {
    // 1. Iterate over files
    for file in repo.files() {
        // 2. Parse with tree-sitter
        let ast = tree_sitter::parse(&file.content, &file.language);

        // 3. Extract symbols
        let symbols = extract_symbols(&ast, &file.language);

        // 4. Build chunks
        let chunks = chunk_document(&file.content, &symbols);

        // 5. Index with Tantivy
        for chunk in chunks {
            let doc = TantivyDocument::build(chunk);
            self.writer.add_document(doc).await;
        }
    }

    // 6. Commit changes
    self.writer.commit();
}
```

### 4. llama.cpp Supervision

Tabby runs llama.cpp as a subprocess with automatic restart:

```rust
// From llama-cpp-server/src/supervisor.rs

pub struct LlamaCppSupervisor {
    name: &'static str,
    port: u16,
    handle: JoinHandle<()>,
}

impl LlamaCppSupervisor {
    pub fn new(/* params */) -> Self {
        let handle = tokio::spawn(async move {
            loop {
                // Start llama-server process
                let mut command = tokio::process::Command::new(server_binary);
                command
                    .arg("-m").arg(&model_path)
                    .arg("--cont-batching")
                    .arg("--port").arg(port.to_string())
                    .arg("-ngl").arg(num_gpu_layers.to_string());

                let mut process = command.spawn().unwrap();

                // Monitor health endpoint
                wait_for_health(port).await;

                // Wait for process exit
                let status = process.wait().await;

                // Restart on failure
                if status.code() != 0 {
                    retry_count += 1;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });

        Self { handle, port }
    }
}
```

## Architecture Deep Dives

### Completion Context Building

The completion context gathers relevant information:

```typescript
interface CompletionContext {
    // Current file info
    filepath: string;
    language: string;
    content: string;

    // Cursor position
    position: number;
    prefix: string;    // Content before cursor
    suffix: string;    // Content after cursor

    // Repository context
    gitRemote?: string;
    declarations?: DeclarationSnippet[];
    recentlyChanged?: CodeSnippet[];
    visibleRanges?: VisibleRange[];

    // User settings
    clipboard?: string;
}
```

### Chat Protocol

Tabby implements OpenAI-compatible chat API:

```typescript
interface ChatMessage {
    role: 'system' | 'user' | 'assistant';
    content: string;
}

interface CreateChatCompletionRequest {
    model: string;
    messages: ChatMessage[];
    temperature?: number;
    max_tokens?: number;
    stream?: boolean;
}

interface CreateChatCompletionResponse {
    id: string;
    choices: ChatCompletionChoice[];
    usage: CompletionUsage;
}
```

### Answer Engine (RAG)

The Answer Engine provides RAG over codebases:

1. **Query Analysis** - Parse user query for intent
2. **Retrieval** - Search code index with BM25 + semantic
3. **Reranking** - Rank results by relevance
4. **Generation** - Generate answer with context
5. **Citation** - Link back to source files

## Configuration

### Server Configuration (config.toml)

```toml
[model.completion]
model = "TabbyML/StarCoder-1B"

[model.chat]
model = "Qwen/Qwen2-1.5B-Instruct"

[server]
host = "0.0.0.0"
port = 8080

[repository.git]
dir = "/path/to/repos"
```

### Client Configuration

```json
{
    "server": {
        "endpoint": "http://localhost:8080"
    },
    "completion": {
        "timeout": 5000,
        "maximumIncompletion": 3
    },
    "postprocess": {
        "limitScope": true,
        "dropDuplicated": true
    }
}
```

## Performance Considerations

### 1. Model Loading

- Models are loaded into VRAM/RAM on startup
- First request incurs cold start latency (~5-30s)
- Subsequent requests benefit from cached weights

### 2. Batching

llama.cpp supports continuous batching:
- Multiple requests processed together
- Improves throughput on GPU

### 3. Index Performance

- Tantivy index is memory-mapped
- Searches are typically <10ms
- Index rebuilds on repository changes

### 4. Caching

Multiple caching layers:
- Completion cache (in-memory, LRU)
- HTTP response cache
- Browser cache (for web UI)

## Related Projects

### Pochi

Pochi is an AI agent built on top of Tabby:
- Autonomous task completion
- Git worktree isolation
- GitHub integration
- Custom model support

## Documentation References

- Main docs: https://tabby.tabbyml.com/docs/
- Model directory: https://tabby.tabbyml.com/docs/models/
- API reference: Available at `/swagger` endpoint
