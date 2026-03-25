# Rust Replication Plan: Building a Distributed File System in Rust

**Source:** Analysis of ZeroFS and related Rust projects

---

## Table of Contents

1. [Why Rust for Distributed Systems](#why-rust-for-distributed-systems)
2. [Core Crate Dependencies](#core-crate-dependencies)
3. [Architecture Overview](#architecture-overview)
4. [Storage Layer Implementation](#storage-layer-implementation)
5. [Network Layer Implementation](#network-layer-implementation)
6. [Concurrency Patterns](#concurrency-patterns)
7. [Error Handling](#error-handling)
8. [Testing Strategy](#testing-strategy)
9. [Performance Optimization](#performance-optimization)

---

## Why Rust for Distributed Systems

### Advantages

| Advantage | Description | Impact |
|-----------|-------------|--------|
| **Memory Safety** | No segfaults, buffer overflows | Reliability |
| **No GC** | Predictable latency | Real-time performance |
| **Zero-cost abstractions** | High-level code, low-level performance | Efficiency |
| **Type system** | Encode invariants at compile time | Correctness |
| **Async/await** | Efficient I/O multiplexing | Scalability |
| **Ecosystem** | Rich crate ecosystem | Productivity |

### Comparison with Other Languages

```
┌─────────────────────────────────────────────────────────────┐
│  Language Comparison for Distributed FS                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  C/C++:                                                     │
│  ✓ Maximum performance                                      │
│  ✗ Memory safety issues                                     │
│  ✗ Manual resource management                               │
│                                                             │
│  Go:                                                        │
│  ✓ Simple concurrency                                       │
│  ✓ Fast compilation                                         │
│  ✗ GC pauses                                                │
│  ✗ Limited generics (improving)                             │
│                                                             │
│  Java/Scala:                                                │
│  ✓ Mature ecosystem                                         │
│  ✓ Excellent concurrency                                    │
│  ✗ GC overhead                                              │
│  ✗ JVM memory footprint                                     │
│                                                             │
│  Rust:                                                      │
│  ✓ Memory safety                                            │
│  ✓ No GC                                                    │
│  ✓ Zero-cost abstractions                                   │
│  ✗ Steeper learning curve                                   │
│  ✗ Longer compile times                                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Crate Dependencies

### Essential Crates

```toml
[dependencies]
# Async runtime
tokio = { version = "1.49", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec"] }
tokio-stream = { version = "0.1", features = ["net", "sync"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"  # Or prost for protobuf
prost = "0.14"
prost-types = "0.14"

# Error handling
anyhow = "1.0"  # Application errors
thiserror = "2.0"  # Library errors

# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Networking
tonic = "0.14"  # gRPC
tower = "0.5"  # Service utilities
hyper = { version = "1.0", features = ["full"] }

# Storage
bytes = "1.10"  # Byte buffers
object_store = { version = "0.12", features = ["aws", "azure", "gcp"] }

# Concurrency
dashmap = "6.1"  # Concurrent hash map
arc-swap = "1.7"  # Atomic Arc swaps
parking_lot = "0.12"  # Fast locks

# Cryptography
chacha20poly1305 = "0.10"  # Encryption
argon2 = "0.5"  # Password hashing
sha2 = "0.10"  # Hashing
hkdf = "0.12"  # Key derivation
rand = "0.8"  # Random numbers

# Compression
lz4_flex = "0.12"  # Fast compression
zstd = "0.13"  # High-ratio compression

# Database (LSM-tree)
slatedb = { git = "https://github.com/slatedb/slatedb.git" }

# Caching
foyer-memory = "0.21"  # Memory cache
foyer-storage = "0.21"  # Disk cache

# CLI
clap = { version = "4.5", features = ["derive"] }

# Configuration
toml = "0.9"
shellexpand = "3.1"  # Environment variables in paths

# Time
chrono = "0.4"

# Filesystem
fuser = "0.15"  # FUSE userspace
```

### Development Dependencies

```toml
[dev-dependencies]
# Testing
tokio-test = "0.4"
tempfile = "3.21"
mockall = "0.13"  # Mocking

# Benchmarking
criterion = "0.5"

# Fuzzing
proptest = "1.4"
```

---

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ZeroFS Rust Architecture                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                  CLI Layer                           │    │
│  │  main.rs, cli/                                      │    │
│  └──────────────────┬──────────────────────────────────┘    │
│                     │                                        │
│  ┌──────────────────▼──────────────────────────────────┐    │
│  │               Protocol Servers                      │    │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐             │    │
│  │  │   NFS   │  │   9P    │  │   NBD   │             │    │
│  │  │  nfs.rs │  │ ninep/  │  │  nbd/   │             │    │
│  │  └─────────┘  └─────────┘  └─────────┘             │    │
│  └──────────────────┬──────────────────────────────────┘    │
│                     │                                        │
│  ┌──────────────────▼──────────────────────────────────┐    │
│  │              Filesystem Core (fs/)                   │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
│  │  │   Inode     │  │   Chunk     │  │   Directory │  │    │
│  │  │   Manager   │  │   Store     │  │   Index     │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │    │
│  │  │ Write       │  │   Flush     │  │   Lock      │  │    │
│  │  │ Coordinator │  │ Coordinator │  │  Manager    │  │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │    │
│  └──────────────────┬──────────────────────────────────┘    │
│                     │                                        │
│  ┌──────────────────▼──────────────────────────────────┐    │
│  │              Database Layer (db.rs)                  │    │
│  │  - SlateDB wrapper                                   │    │
│  │  - Transaction support                               │    │
│  │  - Scan operations                                   │    │
│  └──────────────────┬──────────────────────────────────┘    │
│                     │                                        │
│  ┌──────────────────▼──────────────────────────────────┐    │
│  │              Cache Layer (cache.rs)                  │    │
│  │  - Foyer memory cache                                │    │
│  │  - Disk cache                                        │    │
│  └──────────────────┬──────────────────────────────────┘    │
│                     │                                        │
│  ┌──────────────────▼──────────────────────────────────┐    │
│  │           Encryption (key_management.rs)             │    │
│  │  - Argon2id KDF                                      │    │
│  │  - XChaCha20-Poly1305                                │    │
│  │  - DEK/KEK hierarchy                                 │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Directory Structure

```
zerofs/
├── Cargo.toml
├── build.rs              # Protocol buffer compilation
├── src/
│   ├── lib.rs            # Library entry point
│   ├── main.rs           # Binary entry point
│   │
│   ├── cli/              # CLI commands
│   │   ├── mod.rs
│   │   ├── server.rs     # Run server
│   │   ├── compactor.rs  # Standalone compactor
│   │   ├── checkpoint.rs # Checkpoint management
│   │   └── password.rs   # Password change
│   │
│   ├── config.rs         # Configuration
│   ├── db.rs             # Database wrapper
│   ├── cache.rs          # Cache integration
│   ├── key_management.rs # Encryption keys
│   │
│   ├── fs/               # Filesystem core
│   │   ├── mod.rs
│   │   ├── inode.rs      # Inode management
│   │   ├── types.rs      # File types
│   │   ├── permissions.rs# POSIX permissions
│   │   ├── store/
│   │   │   ├── mod.rs
│   │   │   ├── chunk.rs  # Chunk storage
│   │   │   ├── inode.rs  # Inode storage
│   │   │   └── directory.rs
│   │   ├── write_coordinator.rs
│   │   ├── flush_coordinator.rs
│   │   ├── lock_manager.rs
│   │   ├── gc.rs         # Garbage collection
│   │   └── metrics.rs
│   │
│   ├── nfs.rs            # NFS server
│   ├── ninep/            # 9P server
│   │   ├── mod.rs
│   │   ├── protocol.rs
│   │   ├── handler.rs
│   │   └── server.rs
│   ├── nbd/              # NBD server
│   │   ├── mod.rs
│   │   ├── protocol.rs
│   │   ├── handler.rs
│   │   └── server.rs
│   │
│   ├── block_transformer.rs  # Encryption transformer
│   ├── checkpoint_manager.rs
│   ├── failpoints.rs     # Testing failpoints
│   └── parse_object_store.rs
│
├── proto/                # Protocol buffers
│   └── rpc.proto
│
└── tests/
    └── failpoints/
```

---

## Storage Layer Implementation

### SlateDB Integration

```rust
// src/db.rs
use slatedb::{Db, DbBuilder, WriteBatch};
use slatedb::config::{PutOptions, ReadOptions, WriteOptions};
use bytes::Bytes;

pub struct Database {
    db: Arc<Db>,
}

impl Database {
    pub async fn open(path: &str, object_store: Arc<dyn ObjectStore>) -> Result<Self> {
        let db = DbBuilder::new()
            .with_object_store(object_store)
            .with_path(path)
            .build()
            .await?;

        Ok(Self { db: Arc::new(db) })
    }

    pub async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        Ok(self.db.get(key).await?)
    }

    pub async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.put(key, value).await?;
        Ok(())
    }

    pub async fn scan<R>(&self, range: R) -> Result<impl Stream<Item = Result<(Bytes, Bytes)>>>
    where
        R: RangeBounds<Bytes> + Send + 'static,
    {
        let mut iter = self.db.scan(range).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        tokio::spawn(async move {
            while let Some(kv) = iter.next().await {
                if tx.send(kv).await.is_err() {
                    break;
                }
            }
        });

        Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    pub async fn write_batch(&self, batch: WriteBatch) -> Result<()> {
        self.db.write(batch).await?;
        Ok(())
    }
}
```

### Chunk Store

```rust
// src/fs/store/chunk.rs
const CHUNK_SIZE: usize = 32 * 1024;  // 32KB

pub struct ChunkStore {
    db: Arc<Database>,
}

impl ChunkStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn read(&self, inode: u64, offset: u64, len: u64) -> Result<Bytes> {
        let start_chunk = offset / CHUNK_SIZE as u64;
        let end_chunk = (offset + len - 1) / CHUNK_SIZE as u64;

        let mut result = BytesMut::with_capacity(len as usize);
        let mut stream = self.db.scan(self.chunk_range(inode, start_chunk, end_chunk)).await?;

        let mut chunk_map: HashMap<u64, Bytes> = HashMap::new();
        while let Some(Ok((key, value))) = stream.next().await {
            if let Some(chunk_idx) = self.parse_chunk_key(&key) {
                chunk_map.insert(chunk_idx, value);
            }
        }

        for chunk_idx in start_chunk..=end_chunk {
            let chunk = chunk_map.get(&chunk_idx)
                .map(|b| b.as_ref())
                .unwrap_or(&ZERO_CHUNK);
            result.extend_from_slice(chunk);
        }

        Ok(result.freeze())
    }

    pub async fn write(&self, inode: u64, offset: u64, data: &[u8]) -> Result<()> {
        let mut txn = self.db.new_transaction()?;

        // Read-modify-write for partial chunk writes
        let start_chunk = offset / CHUNK_SIZE as u64;
        let end_chunk = (offset + data.len() as u64 - 1) / CHUNK_SIZE as u64;

        for chunk_idx in start_chunk..=end_chunk {
            let chunk_data = self.compute_chunk_data(inode, chunk_idx, offset, data).await?;
            if chunk_data.as_ref() == ZERO_CHUNK {
                txn.delete(self.chunk_key(inode, chunk_idx));
            } else {
                txn.put(self.chunk_key(inode, chunk_idx), &chunk_data);
            }
        }

        txn.commit().await?;
        Ok(())
    }
}
```

---

## Network Layer Implementation

### NFS Server

```rust
// src/nfs.rs
use zerofs_nfsserve::{nfs, filesystem::FileSystem};

pub struct NfsServer {
    fs: Arc<FilesystemCore>,
}

#[nfs]
impl FileSystem for NfsServer {
    async fn lookup(&self, dir: u64, name: &OsStr) -> Result<Entry> {
        self.fs.lookup(dir, name).await
    }

    async fn getattr(&self, inode: u64) -> Result<Attr> {
        self.fs.getattr(inode).await
    }

    async fn read(&self, inode: u64, offset: u64, count: u32) -> Result<Vec<u8>> {
        self.fs.read(inode, offset, count).await
    }

    async fn write(&self, inode: u64, offset: u64, data: &[u8]) -> Result<u32> {
        self.fs.write(inode, offset, data).await
    }

    // ... other NFS operations
}

impl NfsServer {
    pub async fn run(self, addr: SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;

        loop {
            let (stream, peer) = listener.accept().await?;
            let server = self.clone();

            tokio::spawn(async move {
                if let Err(e) = server.serve(stream, peer).await {
                    error!("NFS connection error: {}", e);
                }
            });
        }
    }
}
```

### 9P Server

```rust
// src/ninep/handler.rs
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

pub struct NinePHandler {
    fs: Arc<FilesystemCore>,
}

impl NinePHandler {
    pub async fn handle<S>(&self, mut stream: S) -> Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            // Read 9P message header
            let mut size_buf = [0u8; 4];
            stream.read_exact(&mut size_buf).await?;
            let size = u32::from_le_bytes(size_buf) as usize;

            // Read message body
            let mut tag_buf = vec![0u8; size - 2];
            stream.read_exact(&mut tag_buf).await?;

            // Parse and handle message
            let msg = NinePMessage::decode(&tag_buf)?;
            let response = self.process_message(msg).await?;

            // Write response
            let encoded = response.encode();
            let mut size_buf = (encoded.len() + 1) as u32;
            stream.write_all(&size_buf.to_le_bytes()).await?;
            stream.write_all(&encoded).await?;
        }
    }

    async fn process_message(&self, msg: NinePMessage) -> Result<NinePMessage> {
        match msg {
            NinePMessage::Tversion { msize, version } => {
                Ok(NinePMessage::Rversion { msize, version: "9p2000.L".into() })
            }
            NinePMessage::Tattach { fid, authentication } => {
                Ok(NinePMessage::Rattach { fid })
            }
            NinePMessage::Topen { fid, mode } => {
                self.handle_open(fid, mode).await?;
                Ok(NinePMessage::Ropen { qid, io_unit })
            }
            NinePMessage::Tread { fid, offset, count } => {
                let data = self.fs.read(fid, offset, count).await?;
                Ok(NinePMessage::Rread { data })
            }
            NinePMessage::Twrite { fid, offset, data } => {
                let written = self.fs.write(fid, offset, &data).await?;
                Ok(NinePMessage::Rwrite { count: written })
            }
            // ... other messages
        }
    }
}
```

---

## Concurrency Patterns

### Async/Await Pattern

```rust
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct FilesystemCore {
    inodes: RwLock<HashMap<u64, Inode>>,
    open_files: RwLock<HashMap<u64, OpenFile>>,
}

impl FilesystemCore {
    // Read operations use read lock (concurrent)
    pub async fn getattr(&self, inode: u64) -> Result<FileAttr> {
        let inodes = self.inodes.read().await;
        inodes.get(&inode)
            .map(|i| i.attr)
            .ok_or(Error::NotFound)
    }

    // Write operations use write lock (exclusive)
    pub async fn create(&self, parent: u64, name: &str) -> Result<u64> {
        let mut inodes = self.inodes.write().await;

        // Check for existing file
        if self.exists(&inodes, parent, name) {
            return Err(Error::AlreadyExists);
        }

        // Allocate new inode
        let new_inode = self.allocate_inode(&inodes);
        inodes.insert(new_inode, Inode::new(parent, name));

        Ok(new_inode)
    }
}
```

### Actor Pattern (Optional)

```rust
use tokio::sync::mpsc;

pub struct ActorContext<M> {
    mailbox: mpsc::Sender<M>,
}

pub trait Actor: Sized {
    type Message: Send + 'static;

    async fn handle(&mut self, msg: Self::Message) -> Result<()>;

    fn spawn(self) -> ActorContext<Self::Message> {
        let (tx, mut rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let mut actor = self;
            while let Some(msg) = rx.recv().await {
                if let Err(e) = actor.handle(msg).await {
                    error!("Actor error: {}", e);
                }
            }
        });

        ActorContext { mailbox: tx }
    }
}

// Usage
struct CompactorActor {
    db: Arc<Database>,
}

impl Actor for CompactorActor {
    type Message = CompactionRequest;

    async fn handle(&mut self, msg: CompactionRequest) -> Result<()> {
        self.compact(msg.level).await
    }
}
```

### Lock-Free Patterns

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

pub struct ReadOnlyFilesystem {
    // Atomic Arc swap for lock-free reads
    state: ArcSwap<FilesystemState>,
}

impl ReadOnlyFilesystem {
    pub fn get(&self, key: &str) -> Option<&Value> {
        // Lock-free read
        let state = self.state.load();
        state.get(key)
    }

    pub fn swap_state(&self, new_state: Arc<FilesystemState>) {
        // Atomic swap
        self.state.store(new_state);
    }
}
```

---

## Error Handling

### ThisError for Libraries

```rust
// src/fs/errors.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FsError {
    #[error("File not found")]
    NotFound,

    #[error("Permission denied")]
    PermissionDenied,

    #[error("File already exists")]
    AlreadyExists,

    #[error("Not a directory")]
    NotADirectory,

    #[error("Is a directory")]
    IsADirectory,

    #[error("No space left on device")]
    NoSpace,

    #[error("Read-only filesystem")]
    ReadOnly,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DbError(#[from] slatedb::Error),

    #[error("Encryption error: {0}")]
    EncryptionError(String),
}

impl From<FsError> for libc::c_int {
    fn from(err: FsError) -> Self {
        match err {
            FsError::NotFound => libc::ENOENT,
            FsError::PermissionDenied => libc::EACCES,
            FsError::AlreadyExists => libc::EEXIST,
            FsError::NotADirectory => libc::ENOTDIR,
            FsError::IsADirectory => libc::EISDIR,
            FsError::NoSpace => libc::ENOSPC,
            FsError::ReadOnly => libc::EROFS,
            _ => libc::EIO,
        }
    }
}
```

### Anyhow for Applications

```rust
// src/main.rs
use anyhow::{Context, Result};

async fn run() -> Result<()> {
    let config = load_config()
        .context("Failed to load configuration")?;

    let object_store = create_object_store(&config.storage)
        .context("Failed to create object store")?;

    let db = Database::open(&config.storage.path, object_store)
        .await
        .context("Failed to open database")?;

    // ... rest of initialization

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chunk_read_write() -> Result<()> {
        let db = create_test_db().await?;
        let chunk_store = ChunkStore::new(Arc::new(db));

        let inode = 1;
        let data = b"Hello, World!";

        // Write
        chunk_store.write(inode, 0, data).await?;

        // Read
        let result = chunk_store.read(inode, 0, data.len() as u64).await?;
        assert_eq!(&result[..], data);

        Ok(())
    }

    #[tokio::test]
    async fn test_partial_chunk_write() -> Result<()> {
        let db = create_test_db().await?;
        let chunk_store = ChunkStore::new(Arc::new(db));

        // Initial write
        chunk_store.write(1, 0, b"aaaaaaaaaa").await?;

        // Partial overwrite
        chunk_store.write(1, 3, b"BBB").await?;

        // Read all
        let result = chunk_store.read(1, 0, 10).await?;
        assert_eq!(&result[..], b"aaaBBBaaaa");

        Ok(())
    }
}
```

### Integration Tests

```rust
// tests/failpoints/mod.rs
use fail::fail_point;

#[tokio::test]
async fn test_write_with_failpoint() -> Result<()> {
    // Enable failpoint
    fail::cfg("db_write_fail", "return(error)").unwrap();

    let fs = create_test_filesystem().await?;

    // This should fail due to failpoint
    let result = fs.write(1, 0, b"test").await;
    assert!(result.is_err());

    // Disable failpoint
    fail::remove("db_write_fail");

    // This should succeed
    let result = fs.write(1, 0, b"test").await;
    assert!(result.is_ok());

    Ok(())
}
```

### Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_chunk_round_trip(data in any::<Vec<u8>>()) {
        let encoded = encode_chunk(&data);
        let decoded = decode_chunk(&encoded);
        prop_assert_eq!(data, decoded);
    }

    #[test]
    fn test_merkle_proof_verification(
        leaves in prop::collection::vec(any::<Vec<u8>>(), 1..100)
    ) {
        let tree = MerkleTree::new(&leaves);
        for i in 0..leaves.len() {
            let proof = tree.get_proof(i);
            prop_assert!(proof.verify(tree.root()));
        }
    }
}
```

---

## Performance Optimization

### Benchmarking

```rust
// benches/chunk_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_chunk_write(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let chunk_store = runtime.block_on(create_chunk_store());
    let data = vec![0u8; 32 * 1024];  // 32KB

    c.bench_function("chunk_write_32kb", |b| {
        b.to_async(&runtime).iter(|| async {
            chunk_store.write(black_box(1), black_box(0), black_box(&data)).await.unwrap()
        })
    });
}

criterion_group!(benches, bench_chunk_write);
criterion_main!(benches);
```

### Profiling

```bash
# CPU profiling with perf
cargo build --release
perf record -F 99 -g target/release/zerofs run -c config.toml
perf report

# Memory profiling with heaptrack
heaptrack target/release/zerofs

# Flamegraph
cargo flamegraph --bin zerofs -- run -c config.toml
```

### Optimization Techniques

```rust
// 1. Use Bytes for zero-copy
use bytes::Bytes;

fn process_data(data: Bytes) -> Result<Bytes> {
    // No copying, just references
    Ok(data.slice(0..1024))
}

// 2. Parallel chunk operations
use futures::stream::{self, StreamExt};

async fn read_parallel(chunk_ids: Vec<u64>) -> Result<Vec<Bytes>> {
    stream::iter(chunk_ids)
        .map(|id| read_chunk(id))
        .buffer_unordered(20)  // 20 concurrent reads
        .try_collect()
        .await
}

// 3. Preallocate vectors
let mut result = Vec::with_capacity(expected_size);

// 4. Use parking_lot for faster locks
use parking_lot::RwLock;
let lock = RwLock::new(data);  // Faster than std::sync::RwLock
```

---

## Summary

### Key Takeaways

1. **Rust is ideal** for distributed systems: memory safety + performance
2. **Essential crates**: tokio, serde, tonic, object_store, slatedb
3. **Architecture layers**: CLI → Protocol → FS Core → DB → Cache
4. **Concurrency**: async/await, actors, lock-free patterns
5. **Error handling**: thiserror for libraries, anyhow for applications
6. **Testing**: unit tests, integration tests, failpoints, property-based
7. **Performance**: benchmarking, profiling, zero-copy, parallelism

### Further Reading

- [Tokio Documentation](https://tokio.rs/)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Designing Data-Intensive Applications](https://dataintensive.net/)
- [ZeroFS Source Code](/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/ZeroFS/)
