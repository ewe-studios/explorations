# Rust Revision - Production-Grade Rust Implementation Roadmap

## Executive Summary

This document outlines the considerations for implementing production-grade Rust versions of Moltbook ecosystem components. It leverages the existing Zeroclaw implementation as a reference and provides migration paths from TypeScript (Moltbot) and Python (Nanobot) to Rust.

---

## 1. Current Rust Implementation: Zeroclaw

### 1.1 Architecture Overview

Zeroclaw demonstrates a production-ready Rust agent with:

- **Binary Size:** 3.4-8.8MB (release build)
- **Memory Footprint:** <5MB RAM for common operations
- **Startup Time:** <10ms cold start
- **Trait-Driven Design:** Swappable providers, channels, tools, memory backends

### 1.2 Key Dependencies (Cargo.toml)

```toml
[dependencies]
# Core runtime
tokio = { version = "1.42", features = ["rt-multi-thread", "macros", "time", "net", "io-util", "sync", "process", "io-std", "fs", "signal"] }
tokio-util = { version = "0.7" }

# HTTP client
reqwest = { version = "0.12", features = ["json", "rustls-tls", "blocking", "multipart", "stream"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = { version = "1.0" }

# CLI
clap = { version = "4.5", features = ["derive"] }

# Logging
tracing = { version = "0.1" }
tracing-subscriber = { version = "0.3", features = ["fmt", "ansi", "env-filter"] }

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Memory/Database
rusqlite = { version = "0.38", features = ["bundled"] }
postgres = { version = "0.19" }

# HTTP server (Gateway)
axum = { version = "0.8", features = ["http1", "json", "tokio", "query", "ws"] }
tower = { version = "0.5" }
tower-http = { version = "0.6", features = ["limit", "timeout"] }

# Observability
opentelemetry = { version = "0.31", features = ["trace", "metrics"] }
opentelemetry_sdk = { version = "0.31", features = ["trace", "metrics"] }
opentelemetry-otlp = { version = "0.31", features = ["trace", "metrics", "http-proto", "reqwest-client"] }
prometheus = { version = "0.14" }

# Security
chacha20poly1305 = "0.10"  # AEAD for secret store
hmac = "0.12"
sha2 = "0.10"
rand = "0.9"  # CSPRNG
ring = "0.17"  # HMAC-SHA256

# Channels
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }  # Discord WebSocket
lettre = { version = "0.11.19", features = ["builder", "smtp-transport", "rustls-tls"] }  # Email
async-imap = { version = "0.11" }  # IMAP

# Hardware (optional)
nusb = { version = "0.2", optional = true }  # USB enumeration
tokio-serial = { version = "5", optional = true }  # Serial port
rppal = { version = "0.22", optional = true, target = "cfg(target_os = \"linux\")" }  # Raspberry Pi GPIO

# Build optimization
[profile.release]
opt-level = "z"
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"
```

### 1.3 Lessons from Zeroclaw

**What Works Well:**

1. **Trait-based architecture** - Clean separation of concerns, easy to extend
2. **Single binary deployment** - No runtime dependencies, easy distribution
3. **Memory safety** - No GC pauses, predictable performance
4. **SQLite integration** - rusqlite with bundled SQLite works flawlessly
5. **Tokio runtime** - Mature async ecosystem

**Challenges Encountered:**

1. **Build complexity** - OpenSSL linking on Linux, native dependencies
2. **Binary size** - Still larger than Go counterparts (but shrinking)
3. **Compile times** - 2-5 minutes for release builds
4. **WebSocket fragmentation** - Multiple implementations (tungstenite, tokio-tungstenite, axum ws)

---

## 2. Moltbot Rust Revision

### 2.1 Component Migration Priority

| Priority | Component | Complexity | Rust Benefit |
|----------|-----------|------------|--------------|
| 1 | Gateway (WebSocket server) | Medium | High (performance, memory) |
| 2 | Memory System (SQLite) | Low | High (type safety, performance) |
| 3 | Channel Adapters | High | Medium (concurrency safety) |
| 4 | Provider Clients | Medium | Medium (HTTP efficiency) |
| 5 | CLI | Low | Medium (binary distribution) |
| 6 | Plugin System | High | High (sandboxing, security) |

### 2.2 Gateway Rewrite

**Current (TypeScript):**

```typescript
// src/gateway/index.ts
import { WebSocketServer } from 'ws';

const wss = new WebSocketServer({
  port: 18789,
  host: '127.0.0.1',
});

wss.on('connection', (ws, req) => {
  ws.on('message', async (data) => {
    const frame = JSON.parse(data.toString());
    // Handle frame...
  });
});
```

**Rust Revision:**

```rust
// src/gateway/server.rs
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    response::IntoResponse,
    routing::get,
    Router,
};
use tokio::sync::broadcast;

pub struct Gateway {
    bind_host: String,
    bind_port: u16,
    tx: broadcast::Sender<GatewayEvent>,
}

impl Gateway {
    pub fn new(bind_host: String, bind_port: u16) -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { bind_host, bind_port, tx }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/ws", get(Self::ws_handler))
            .with_state(self)
    }

    async fn ws_handler(
        ws: WebSocketUpgrade,
        State(gateway): State<Self>,
    ) -> impl IntoResponse {
        ws.on_upgrade(Self::handle_socket)
    }

    async fn handle_socket(mut socket: WebSocket) {
        // Handle WebSocket frames
        // Implement pairing protocol
        // Broadcast events to subscribers
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", self.bind_host, self.bind_port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, self.into_router()).await?;
        Ok(())
    }
}
```

**Benefits:**
- Type-safe WebSocket protocol
- Compile-time validation of frame schemas
- Zero-cost abstraction for event broadcasting
- Native TLS support via rustls

### 2.3 Memory System

**Current (TypeScript with sqlite-vec):**

```typescript
// src/memory/sqlite.ts
import Database from 'better-sqlite3';
import * as sqliteVec from 'sqlite-vec';

const db = new Database(path);
sqliteVec.load(db);

// Hybrid search
const results = db.prepare(`
  SELECT chunk_id, text,
         vec_distance_cosine(embedding, ?) as vec_score,
         rank as bm25_score
  FROM memories
  JOIN memories_fts ON memories.id = memories_fts.rowid
  WHERE memories_fts MATCH ?
  ORDER BY vec_score * ? + bm25_score * ?
  LIMIT ?
`).all(embedding, query, vectorWeight, textWeight, limit);
```

**Rust Revision:**

```rust
// src/memory/sqlite.rs
use rusqlite::{Connection, params};
use rusqlite_vec::load as load_vec;

pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn open(path: &str) -> Result<Self, rusqlite::Error> {
        let mut conn = Connection::open(path)?;
        load_vec(&conn).unwrap();  // Load sqlite-vec extension

        // Create tables
        conn.execute_batch(include_str!("schema.sql"))?;
        Ok(Self { conn })
    }

    pub fn hybrid_search(
        &self,
        query_embedding: &[f32],
        query_text: &str,
        vector_weight: f32,
        text_weight: f32,
        limit: i64,
    ) -> Result<Vec<MemoryChunk>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT chunk_id, text, embedding,
                    vec_distance_cosine(embedding, ?1) as vec_score,
                    rank as bm25_score
             FROM memories
             JOIN memories_fts ON memories.id = memories_fts.rowid
             WHERE memories_fts MATCH ?2
             ORDER BY vec_score * ?3 + bm25_score * ?4
             LIMIT ?5"
        )?;

        let rows = stmt.query_map(
            params![query_embedding, query_text, vector_weight, text_weight, limit],
            |row| {
                Ok(MemoryChunk {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    embedding: row.get(2)?,
                    vec_score: row.get(3)?,
                    bm25_score: row.get(4)?,
                })
            },
        )?;

        rows.collect()
    }
}
```

**Benefits:**
- Type-safe SQL queries with compile-time validation (via sqlx optional)
- Zero-cost deserialization
- No runtime binding overhead
- Memory-safe FFI for sqlite-vec

### 2.4 Channel Adapters

**Telegram (grammY → Rust):**

```rust
// src/channels/telegram.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TelegramBot {
    client: Client,
    token: String,
    api_url: String,
}

impl TelegramBot {
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
            api_url: "https://api.telegram.org/bot".to_string(),
        }
    }

    pub async fn get_updates(
        &self,
        offset: Option<i64>,
        limit: Option<u8>,
        timeout: Option<u32>,
    ) -> Result<Vec<Update>, reqwest::Error> {
        let url = format!("{}/{}/getUpdates", self.api_url, self.token);
        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "offset": offset,
                "limit": limit,
                "timeout": timeout,
            }))
            .send()
            .await?
            .json()
            .await?;
        Ok(response.result)
    }

    pub async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<Message, reqwest::Error> {
        let url = format!("{}/{}/sendMessage", self.api_url, self.token);
        let response = self.client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            }))
            .send()
            .await?
            .json()
            .await?;
        Ok(response.result)
    }
}
```

**Benefits:**
- Type-safe API structures
- Compile-time validation of required fields
- Automatic retry with exponential backoff
- Connection pooling built into reqwest

### 2.5 Plugin System (Sandboxing)

**Security Model:**

```rust
// src/plugins/sandbox.rs
use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub workspace_only: bool,
    pub allowed_commands: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub memory_limit_mb: Option<u64>,
    pub cpu_limit: Option<f32>,
}

pub struct PluginSandbox {
    config: SandboxConfig,
    workspace: String,
}

impl PluginSandbox {
    pub fn new(config: SandboxConfig, workspace: String) -> Self {
        Self { config, workspace }
    }

    #[cfg(target_os = "linux")]
    pub async fn execute(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<SandboxResult, SandboxError> {
        use landlock::{Ruleset, RulesetAttr, Access, PathBeneath};

        // Check allowed commands
        if !self.config.allowed_commands.iter().any(|c| c == command) {
            return Err(SandboxError::CommandNotAllowed(command.to_string()));
        }

        // Create Landlock rules (Linux sandboxing)
        let mut ruleset = Ruleset::new()
            .handle_access(Access::FsRead)?
            .handle_access(Access::FsExecute)?
            .create()?
            .add_rules(vec![
                PathBeneath::new(&self.workspace, Access::FsRead | Access::FsExecute),
                PathBeneath::new("/usr/bin", Access::FsExecute),
                PathBeneath::new("/bin", Access::FsExecute),
            ])?
            .restrict()?;

        // Execute command
        let output = TokioCommand::new(command)
            .args(args)
            .current_dir(&self.workspace)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        Ok(SandboxResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }

    #[cfg(target_os = "macos")]
    pub async fn execute(
        &self,
        command: &str,
        args: &[&str],
    ) -> Result<SandboxResult, SandboxError> {
        // macOS uses SandboxExec profiles
        // Similar implementation with platform-specific sandboxing
        todo!()
    }
}
```

**Docker Sandbox Alternative:**

```rust
// src/plugins/docker_sandbox.rs
use bollard::{Docker, container::{Config, CreateContainerOptions, StartContainerOptions}};

pub struct DockerSandbox {
    docker: Docker,
    image: String,
    workspace_mount: String,
}

impl DockerSandbox {
    pub async fn new(image: String, workspace: String) -> Result<Self, bollard::errors::Error> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self {
            docker,
            image,
            workspace_mount: workspace,
        })
    }

    pub async fn execute(
        &self,
        command: &[&str],
        env: &[(&str, &str)],
    ) -> Result<SandboxResult, SandboxError> {
        // Create container
        let config = Config {
            image: Some(&self.image),
            cmd: Some(command.to_vec()),
            env: Some(env.iter().map(|(k, v)| format!("{}={}", k, v)).collect()),
            host_config: Some(bollard::models::HostConfig {
                binds: Some(vec![format!("{}:/workspace:ro", self.workspace_mount)]),
                memory: Some(512 * 1024 * 1024),  // 512MB limit
                cpu_quota: Some(50000),  // 50% CPU
                read_only_rootfs: Some(true),
                ..Default::default()
            }),
            working_dir: Some("/workspace"),
            ..Default::default()
        };

        let container = self.docker.create_container(
            Some(CreateContainerOptions { name: "plugin-sandbox", platform: None }),
            config,
        ).await?;

        // Start container
        self.docker.start_container(&container.id, None).await?;

        // Wait for completion
        let result = self.docker.wait_container(&container.id, None).await?;

        // Get logs
        let logs = self.docker.logs::<String>(
            &container.id,
            Some(bollard::models::LogsOptions {
                stdout: true,
                stderr: true,
                ..Default::default()
            }),
        ).await?;

        Ok(SandboxResult {
            stdout: logs.stdout.unwrap_or_default(),
            stderr: logs.stderr.unwrap_or_default(),
            exit_code: result.status_code,
        })
    }
}
```

---

## 3. Nanobot Rust Revision

### 3.1 Current Python Nanobot

```python
# nanobot/src/agent.py
from openai import OpenAI
import sqlite3

class NanoBot:
    def __init__(self, api_key: str):
        self.client = OpenAI(api_key=api_key)
        self.memory = sqlite3.connect('memory.sqlite')

    def chat(self, message: str) -> str:
        # Load memory
        memories = self.search_memory(message)

        # Build prompt
        prompt = f"Memories: {memories}\nUser: {message}"

        # Get response
        response = self.client.chat.completions.create(
            model="gpt-4o",
            messages=[{"role": "user", "content": prompt}]
        )

        # Save to memory
        self.save_memory(message, response.choices[0].message.content)

        return response.choices[0].message.content
```

### 3.2 Rust Equivalent

```rust
// nanobot-rust/src/main.rs
use openai_api::{Client, ChatCompletionRequest};
use rusqlite::Connection;

struct NanoBot {
    client: Client,
    memory: Connection,
}

impl NanoBot {
    fn new(api_key: String) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::new(&api_key);
        let memory = Connection::open("memory.sqlite")?;

        Ok(Self { client, memory })
    }

    fn search_memory(&self, query: &str) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = self.memory.prepare(
            "SELECT content FROM memories
             WHERE content MATCH ?1
             ORDER BY rank
             LIMIT 5"
        )?;

        let memories = stmt
            .query_map([query], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(memories)
    }

    fn save_memory(&self, role: &str, content: &str) -> Result<(), rusqlite::Error> {
        self.memory.execute(
            "INSERT INTO memories (role, content, created_at)
             VALUES (?1, ?2, datetime('now'))",
            [role, content],
        )?;
        Ok(())
    }

    async fn chat(&self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Load memory
        let memories = self.search_memory(message)?;

        // Build prompt
        let prompt = format!("Memories: {:?}\nUser: {}", memories, message);

        // Get response
        let response = self.client
            .chat_completion(ChatCompletionRequest {
                model: "gpt-4o".to_string(),
                messages: vec![("user".to_string(), prompt)],
                ..Default::default()
            })
            .await?;

        let reply = response.choices[0].message.content.clone();

        // Save to memory
        self.save_memory("assistant", &reply)?;

        Ok(reply)
    }
}
```

**Benefits:**
- 10x smaller binary (~3MB vs ~50MB Python + dependencies)
- 100x faster startup (no Python interpreter initialization)
- Type-safe API contracts
- No runtime dependency management (pip, virtualenv)

---

## 4. Production Considerations

### 4.1 Build Optimization

**Release Profile:**

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = "thin"         # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
strip = true         # Remove debug symbols
panic = "abort"      # Smaller binaries, no unwind

# Alternative: Fast release builds (for development)
[profile.release-fast]
inherits = "release"
codegen-units = 8    # Parallel codegen
lto = "off"          # Disable LTO for faster builds
```

**Cargo Configuration:**

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "target-feature=+crt-static"]

[target.aarch64-unknown-linux-gnu]
rustflags = ["-C", "target-feature=+neon"]

[build]
# Default to release profile for production
default-target = "x86_64-unknown-linux-gnu"
```

### 4.2 Cross-Compilation

**Docker Build (Linux):**

```dockerfile
FROM rust:1.85-slim as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime image
FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/moltbot-rs /moltbot-rs
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

ENTRYPOINT ["/moltbot-rs"]
```

**GitHub Actions (Multi-arch):**

```yaml
name: Build

on: [push, pull_request]

jobs:
  build:
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - x86_64-unknown-linux-musl
          - aarch64-unknown-linux-gnu
          - x86_64-apple-darwin
          - aarch64-apple-darwin
          - x86_64-pc-windows-msvc

    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          target: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: moltbot-rs-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/moltbot-rs*
```

### 4.3 Testing Strategy

**Unit Tests:**

```rust
// src/memory/tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_memory_store_open() {
        let temp = NamedTempFile::new().unwrap();
        let store = MemoryStore::open(temp.path().to_str().unwrap());
        assert!(store.is_ok());
    }

    #[test]
    fn test_hybrid_search_empty() {
        let temp = NamedTempFile::new().unwrap();
        let store = MemoryStore::open(temp.path().to_str().unwrap()).unwrap();

        let results = store.hybrid_search(
            &[0.0; 1536],  // Zero vector
            "test query",
            0.7,
            0.3,
            10,
        ).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_insert_and_search() {
        let temp = NamedTempFile::new().unwrap();
        let mut store = MemoryStore::open(temp.path().to_str().unwrap()).unwrap();

        store.insert("test chunk", &[1.0; 1536]).unwrap();

        let results = store.hybrid_search(
            &[1.0; 1536],
            "test",
            0.7,
            0.3,
            10,
        ).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "test chunk");
    }
}
```

**Integration Tests:**

```rust
// tests/gateway_integration.rs
use moltbot_rs::{Gateway, GatewayConfig};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn test_gateway_pairing() {
    // Start gateway
    let config = GatewayConfig {
        bind_host: "127.0.0.1".to_string(),
        bind_port: 18790,  // Use test port
        require_pairing: true,
        ..Default::default()
    };

    let gateway = Gateway::new(config);
    tokio::spawn(async move {
        gateway.run().await.unwrap();
    });

    // Give gateway time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect and test pairing
    let url = "ws://127.0.0.1:18790/ws";
    let (ws_stream, _) = connect_async(url).await.unwrap();

    // Send pairing request
    // ... (full implementation)
}
```

**Benchmark Tests:**

```rust
// benches/memory_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use moltbot_rs::memory::MemoryStore;

fn bench_hybrid_search(c: &mut Criterion) {
    let store = MemoryStore::open("test.sqlite").unwrap();

    // Insert test data
    for i in 0..10000 {
        store.insert(&format!("test chunk {}", i), &[i as f32; 1536]).unwrap();
    }

    c.bench_function("hybrid_search_10k", |b| {
        b.iter(|| {
            store.hybrid_search(
                black_box(&[0.5; 1536]),
                black_box("test query"),
                0.7,
                0.3,
                10,
            ).unwrap()
        })
    });
}

criterion_group!(benches, bench_hybrid_search);
criterion_main!(benches);
```

### 4.4 Continuous Integration

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

env:
  RUSTFLAGS: "-D warnings"
  CARGO_INCREMENTAL: 0

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features -- -D warnings

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable
      - uses: taiki-e/install-action@cargo-nextest
      - run: cargo nextest run

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v4
        with:
          files: lcov.info

  build:
    needs: [lint, test]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable
      - run: cargo build --release
      - run: cargo install --path .
      - run: moltbot-rs --version
```

---

## 5. Migration Roadmap

### Phase 1: Foundation (Month 1-2)

- [ ] Set up Rust workspace structure
- [ ] Implement core types and traits
- [ ] Port memory system (SQLite + sqlite-vec)
- [ ] Basic CLI with clap
- [ ] CI/CD pipeline

### Phase 2: Gateway (Month 2-3)

- [ ] WebSocket server with axum
- [ ] Pairing protocol implementation
- [ ] Event broadcasting system
- [ ] Health endpoints
- [ ] Integration tests

### Phase 3: Channels (Month 3-4)

- [ ] Telegram bot (grammY equivalent)
- [ ] Discord bot
- [ ] WhatsApp (Baileys equivalent)
- [ ] Slack integration
- [ ] Channel routing logic

### Phase 4: Providers (Month 4-5)

- [ ] OpenAI client
- [ ] Anthropic client
- [ ] OpenRouter multi-provider
- [ ] Provider fallback logic
- [ ] Token counting and rate limiting

### Phase 5: Plugin System (Month 5-6)

- [ ] Plugin manifest parsing
- [ ] Sandbox implementation (Landlock/Docker)
- [ ] Plugin SDK for Rust plugins
- [ ] NPM plugin bridge (for existing TypeScript plugins)
- [ ] Security hardening

### Phase 6: Production Hardening (Month 6-7)

- [ ] Observability (OpenTelemetry, Prometheus)
- [ ] Alerting integration
- [ ] Performance optimization
- [ ] Documentation
- [ ] Beta testing

### Phase 7: Migration (Month 7-8)

- [ ] TypeScript → Rust migration guide
- [ ] Data migration tools
- [ ] Backward compatibility layer
- [ ] Deprecation timeline for TypeScript version

---

## 6. Performance Comparison

### Expected Benchmarks

| Metric | TypeScript (Moltbot) | Rust (Moltbot-rs) | Improvement |
|--------|---------------------|-------------------|-------------|
| Binary Size | ~28MB (dist) | ~5MB | 5.6x smaller |
| Cold Start | ~500ms | ~10ms | 50x faster |
| Memory (idle) | ~200MB | ~15MB | 13x less |
| Memory (loaded) | ~1GB+ | ~50MB | 20x less |
| WebSocket throughput | ~10K msg/s | ~100K msg/s | 10x higher |
| Memory search (10K chunks) | ~50ms | ~5ms | 10x faster |

### Zeroclaw Actual Measurements

```bash
# Release binary
$ ls -lh target/release/zeroclaw
-rwxr-xr-x 1 user user 8.8M Feb 18 10:00 target/release/zeroclaw

# Cold start
$ /usr/bin/time -l target/release/zeroclaw --help
        0.02 real         0.01 user         0.00 sys
    3932160  maximum resident set size

# Status command
$ /usr/bin/time -l target/release/zeroclaw status
        0.01 real         0.00 user         0.00 sys
    4194304  maximum resident set size
```

---

## 7. Risk Mitigation

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| OpenSSL linking issues | Medium | Low | Use rustls (default in reqwest, tokio-tungstenite) |
| sqlite-vec compatibility | Low | Medium | Bundle SQLite extension, test on all platforms |
| WebSocket protocol divergence | Low | High | Maintain protocol spec, integration tests |
| Plugin ecosystem fragmentation | Medium | Medium | Provide TypeScript → Rust bridge, migration tools |

### Organizational Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Team Rust learning curve | High | Medium | Pair programming, Rust training, incremental migration |
| Slower feature development (initial) | High | Low | Accept slower initial pace, focus on foundation |
| Community resistance | Medium | Low | Clear migration path, maintain TypeScript version in parallel |

---

## 8. Recommended Team Structure

### Core Team (3-5 engineers)

- **Tech Lead** - Architecture, trait design, code review
- **Gateway Engineer** - WebSocket server, protocol implementation
- **Channel Engineer** - Channel adapters (Telegram, Discord, WhatsApp)
- **Memory Engineer** - SQLite, vector search, hybrid queries
- **DevOps Engineer** - CI/CD, cross-compilation, release automation

### Extended Team (as needed)

- **Security Engineer** - Sandboxing, security audits
- **Plugin Ecosystem** - Developer relations, plugin SDK
- **Documentation** - Migration guides, API docs

---

## 9. Conclusion

The Rust revision of Moltbook ecosystem components is not only feasible but already partially realized through Zeroclaw. The key insights:

1. **Start with Zeroclaw** - Use it as the reference implementation
2. **Incremental migration** - Port component by component, not all at once
3. **Maintain compatibility** - Keep TypeScript version running in parallel
4. **Focus on foundations** - Memory system, gateway, and types first
5. **Embrace the ecosystem** - Leverage Tokio, Axum, rusqlite, reqwest

The payoff is substantial: 10-50x performance improvements, 5-20x memory reduction, and the type safety and reliability that only Rust can provide.

---

*Rust revision roadmap - Part of Moltbook ecosystem exploration*
*Last updated: 2026-03-22*
