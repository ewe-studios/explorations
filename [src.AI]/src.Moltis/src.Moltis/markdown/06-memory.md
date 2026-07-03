# Memory: Markdown Files → Chunks → Embeddings → SQLite Search

The memory crate implements RAG (Retrieval-Augmented Generation) for Moltis: markdown files are chunked, embedded, and stored in SQLite for hybrid search.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/memory/src/`

## Module Structure

```
crates/memory/src/
├── lib.rs              ← Re-exports all public API
├── chunker.rs          ← Text chunking
├── config.rs           ← Memory configuration
├── embeddings.rs       ← Embedding interface
├── embeddings_batch.rs ← Batch embedding
├── embeddings_fallback.rs ← Fallback embedding logic
├── embeddings_local.rs ← Local embeddings (llama-cpp-2, unsafe FFI)
├── embeddings_openai.rs ← OpenAI embeddings
├── error.rs            ← Error types
├── manager.rs          ← Memory manager
├── reranking.rs        ← Result reranking
├── runtime.rs          ← Runtime embedding management
├── schema.rs           ← Database schema, migrations
├── search.rs           ← Hybrid search (FTS + vector)
├── session_export.rs   ← Session export
├── store.rs            ← Store interface
├── store_sqlite.rs     ← SQLite store implementation
├── tools.rs            ← Memory tools for agent
├── watcher.rs          ← File watcher (feature-gated)
└── writer.rs           ← Memory writer
```

## Embedding Pipeline

### Embedding Trait

```rust
// embeddings.rs
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}
```

### Local Embeddings (llama-cpp-2)

```rust
// embeddings_local.rs
#[cfg(feature = "local-embeddings")]
#[allow(unsafe_code)]  // FFI wrappers for llama-cpp-2 require unsafe Send/Sync impls
pub struct LocalEmbeddingProvider { ... }
```

Uses `llama-cpp-2` with unsafe FFI wrappers. The `unsafe` is isolated to FFI boundaries.

### OpenAI Embeddings

```rust
// embeddings_openai.rs
pub struct OpenAIEmbeddingProvider { ... }
```

Calls OpenAI's embedding API.

### Batch Embedding

```rust
// embeddings_batch.rs
pub struct BatchEmbeddingProvider { ... }
```

Batches embedding requests for efficiency.

### Fallback Embedding

```rust
// embeddings_fallback.rs
pub struct FallbackEmbeddingProvider { ... }
```

Falls back to alternative provider if primary fails.

## Chunking

**Source:** `chunker.rs`

Splits markdown documents into chunks for embedding:

```rust
pub fn chunk_text(text: &str, max_chunk_size: usize) -> Vec<String>;
```

## Search

**Source:** `search.rs`

Hybrid search combining FTS (Full-Text Search) and vector similarity:

```rust
pub async fn search(
    query: &str,
    top_k: usize,
    filter: Option<SearchFilter>,
) -> Result<Vec<SearchResult>>;
```

## SQLite Store

**Source:** `store_sqlite.rs`

```rust
pub struct SqliteMemoryStore {
    pool: SqlitePool,
}
```

All memory data stored in SQLite. Migrations via `schema::run_migrations()`.

## Memory Manager

**Source:** `manager.rs`

```rust
pub struct MemoryManager {
    store: Box<dyn MemoryStore>,
    embeddings: Box<dyn EmbeddingProvider>,
    chunker: Chunker,
}
```

Orchestrates chunking, embedding, storage, and search.

## Memory Writer

**Source:** `writer.rs`

```rust
pub struct MemoryWriter { ... }
```

Writes new memory entries: chunks text, generates embeddings, stores in SQLite.

## Memory Tools

**Source:** `tools.rs`

Exposes memory operations as agent tools:
- Save memory
- Search memory
- List memories
- Delete memory

## Reranking

**Source:** `reranking.rs`

```rust
pub fn rerank_results(results: Vec<SearchResult>, query: &str) -> Vec<SearchResult>;
```

Reranks search results for relevance.

## File Watcher

**Source:** `watcher.rs`

```rust
#[cfg(feature = "file-watcher")]
pub struct MemoryFileWatcher { ... }
```

Watches markdown files for changes and updates memory automatically.

## Next

Continue to [07-native-apps](07-native-apps.md) for iOS and macOS apps.
