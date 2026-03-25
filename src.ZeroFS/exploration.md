# ZeroFS Exploration: A Comprehensive Guide to Distributed File Systems

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/`

**Date:** 2026-03-26

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [What is ZeroFS?](#what-is-zerofs)
3. [Distributed File System Fundamentals](#distributed-file-system-fundamentals)
4. [Project Structure Overview](#project-structure-overview)
5. [Deep Dive: Component Analysis](#deep-dive-component-analysis)
6. [Architecture Diagram](#architecture-diagram)
7. [Key Design Decisions](#key-design-decisions)
8. [References](#references)

---

## Executive Summary

ZeroFS is a **high-performance distributed file system** that makes object storage (S3, Azure Blob, GCS) feel like a local filesystem. It provides:

- **File-level access** via NFS and 9P protocols
- **Block-level access** via NBD (Network Block Device)
- **LSM-tree storage** via SlateDB for efficient cloud storage operations
- **Always-on encryption** using XChaCha20-Poly1305
- **Multi-layered caching** with microsecond latencies
- **Production-grade resilience** with ZFS on top for geo-distributed storage

The project is a **monorepo** containing not just ZeroFS itself, but also related projects that form a comprehensive storage ecosystem:

| Component | Purpose | Language |
|-----------|---------|----------|
| **ZeroFS** | Main distributed filesystem | Rust |
| **raptorq** | RaptorQ fountain codes (RFC 6330) | Rust |
| **fuser** | FUSE userspace library | Rust |
| **age** | Modern file encryption | Go |
| **merkle** | Merkle tree implementations | Go |
| **compact_log** | Certificate Transparency log | Rust |
| **quickwit** | Distributed search engine | Rust |
| **bitpacking** | SIMD compression algorithms | Rust |
| **bytesize** | Byte size semantic wrapper | Rust |
| **intrusive-rs** | Intrusive collections | Rust |

---

## What is ZeroFS?

### The Problem

Traditional filesystems mounted on S3 (like s3fs) have fundamental performance problems:

1. **High latency**: S3 operations take 50-300ms
2. **Poor small I/O**: Each small read/write is an S3 API call
3. **No atomic operations**: Can't do atomic multi-file updates
4. **Expensive**: S3 API costs scale with operation count

### The ZeroFS Solution

ZeroFS uses a **Log-Structured Merge (LSM) tree** storage engine (SlateDB) that:

```
┌─────────────────────────────────────────────────────────────┐
│                    Client Layer                              │
│  NFS Client │ 9P Client │ NBD Client │ ZFS │ PostgreSQL     │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  ZeroFS Core                                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐ │
│  │ NFS      │ │ 9P       │ │ NBD      │ │ Checkpoint     │ │
│  │ Server   │ │ Server   │ │ Server   │ │ Manager        │ │
│  └──────────┘ └──────────┘ └──────────┘ └────────────────┘ │
│                            │                                 │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Virtual Filesystem (VFS)                    ││
│  │  - Inode management  - Directory operations             ││
│  │  - Permission checks - Lock coordination                ││
│  └─────────────────────────────────────────────────────────┘│
│                            │                                 │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────────┐│
│  │           Encryption Manager (XChaCha20)                ││
│  │  - Argon2id key derivation  - DEK/KEK hierarchy         ││
│  └─────────────────────────────────────────────────────────┘│
│                            │                                 │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────────┐│
│  │           Cache Manager (Foyer + foyer-memory)          ││
│  │  - Memory block cache  - Disk cache  - Metadata cache   ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  SlateDB (LSM Tree)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Memtable    │  │ WAL         │  │ Compactor           │  │
│  │ (in-memory) │  │ (append)    │  │ (background merge)  │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                            │                                 │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              SSTables (Sorted String Tables)            ││
│  │  - Immutable  - Compression (LZ4/Zstd)  - Bloom filters ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Object Storage Backend                      │
│     AWS S3 │ Azure Blob │ GCS │ MinIO │ Local Filesystem    │
└─────────────────────────────────────────────────────────────┘
```

### Key Features

| Feature | Description |
|---------|-------------|
| **NFS Server** | Mount as network filesystem on any OS |
| **9P Server** | High-performance alternative with better POSIX semantics |
| **NBD Server** | Raw block devices for ZFS, databases, any filesystem |
| **Always Encrypted** | XChaCha20-Poly1305 with LZ4/Zstd compression |
| **Multi-layered Cache** | Memory, metadata, and configurable disk cache |
| **S3 Compatible** | Works with any S3-compatible storage |
| **Checkpoints** | Named point-in-time snapshots |
| **Standalone Compactor** | Offload compaction to separate instances |

---

## Distributed File System Fundamentals

### What Makes a Distributed File System?

A distributed file system (DFS) provides file access across a network while maintaining:

1. **Transparency**: Users see a single filesystem namespace
2. **Consistency**: All clients see the same data
3. **Concurrency**: Multiple clients can access files simultaneously
4. **Fault Tolerance**: System continues operating despite failures

### Consistency Models

| Model | Description | Use Case |
|-------|-------------|----------|
| **Strict Consistency** | All reads see the latest write | Databases, financial systems |
| **Sequential Consistency** | Operations appear in some sequential order | Collaborative editing |
| **Causal Consistency** | Causally related operations are ordered | Social media feeds |
| **Eventual Consistency** | All replicas converge eventually | DNS, CDNs |

**ZeroFS uses strict consistency** through its single-writer LSM-tree design.

### Replication Strategies

```
┌─────────────────────────────────────────────────────────────┐
│                    Replication Patterns                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Primary-Backup (ZeroFS default)                         │
│     Writer ──► Reader1, Reader2, Reader3                    │
│                                                              │
│  2. Multi-Primary (ZFS mirror on top)                       │
│     Writer1 ◄──► ZFS Mirror ◄──► Writer2                   │
│                                                              │
│  3. Erasure Coding (RaptorQ)                                │
│     Data ──► Encoded Symbols ──► Recovery from subset      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Sharding/Partitioning

ZeroFS partitions data at multiple levels:

1. **Block-level**: Files split into 32KB chunks
2. **Inode-level**: Each file/directory has unique 64-bit inode ID
3. **Key-level**: SlateDB partitions keys across SSTables

---

## Project Structure Overview

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/
├── ZeroFS/                          # Main ZeroFS implementation
│   ├── zerofs/
│   │   ├── src/
│   │   │   ├── cli/                 # CLI commands
│   │   │   ├── fs/                  # Filesystem implementation
│   │   │   │   ├── store/           # Chunk store, inode store
│   │   │   │   ├── inode.rs         # Inode management
│   │   │   │   ├── types.rs         # File types
│   │   │   │   └── permissions.rs   # POSIX permissions
│   │   │   ├── nbd/                 # NBD server
│   │   │   ├── ninep/               # 9P protocol
│   │   │   ├── nfs.rs               # NFS server
│   │   │   ├── db.rs                # SlateDB wrapper
│   │   │   ├── cache.rs             # Foyer cache integration
│   │   │   └── key_management.rs    # Encryption key management
│   │   └── documentation/           # Full documentation site
│   └── assets/
│
├── raptorq/                         # RaptorQ (RFC 6330) erasure coding
│   ├── src/
│   │   ├── encoder.rs               # Fountain code encoder
│   │   ├── decoder.rs               # Fountain code decoder
│   │   ├── matrix.rs                # Binary matrix operations
│   │   ├── octet.rs                 # Galois field arithmetic
│   │   └── pi_solver.rs             # Intermediate symbol decoder
│   └── benches/
│
├── fuser/                           # FUSE userspace library
│   ├── src/
│   │   ├── ll/fuse_abi.rs           # FUSE ABI definitions
│   │   ├── session.rs               # Kernel session management
│   │   ├── mnt/                     # Mounting (fuse2, fuse3)
│   │   └── reply.rs                 # Kernel reply types
│   └── examples/
│
├── age/                             # Modern file encryption
│   ├── age.go                       # Core encryption
│   ├── primitives.go                # X25519, scrypt
│   └── cmd/
│
├── merkle/                          # Merkle tree implementations
│   ├── rfc6962/                     # RFC 6962 (CT) Merkle trees
│   ├── proof/                       # Proof generation/verification
│   └── compact/                     # Compact proof format
│
├── compact_log/                     # Certificate Transparency log
│   ├── src/
│   │   ├── log.rs                   # CT log implementation
│   │   └── s.th/                    # Signed Tree Head
│   └── grafana/                     # Monitoring dashboards
│
├── quickwit/                        # Distributed search engine
│   ├── quickwit/
│   │   ├── quickwit-actors/         # Actor framework
│   │   ├── quickwit-storage/        # Object storage abstraction
│   │   └── quickwit-search/         # Distributed search
│
├── bitpacking/                      # SIMD compression
│   ├── src/
│   │   ├── BitPacker1x.rs           # Scalar implementation
│   │   ├── BitPacker4x.rs           # SSE3 implementation
│   │   └── BitPacker8x.rs           # AVX2 implementation
│
├── bytesize/                        # Byte size semantic wrapper
├── intrusive-rs/                    # Intrusive collections
├── mixtrics/                        # Metrics collection
├── mkcert/                          # Development SSL certificates
├── mrecordlog/                      # Record logging
├── ctyd/                            # Container daemon
└── privaxy/                         # Privacy proxy
```

---

## Deep Dive: Component Analysis

### 1. SlateDB Integration (LSM Tree Storage)

ZeroFS uses SlateDB as its storage engine. LSM trees are optimized for:

- **High write throughput**: Sequential appends to memtable
- **Efficient range scans**: Sorted string tables
- **Cloud storage**: Large immutable objects

```rust
// From db.rs - Database wrapper
pub struct Db {
    inner: SlateDbHandle,  // ReadWrite or ReadOnly
}

impl Db {
    pub async fn get_bytes(&self, key: &Bytes) -> Result<Option<Bytes>> {
        let read_options = ReadOptions {
            durability_filter: DurabilityLevel::Memory,
            cache_blocks: true,
            ..Default::default()
        };
        // ... read from SlateDB
    }

    pub async fn scan<R: RangeBounds<Bytes>>(
        &self,
        range: R,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<(Bytes, Bytes)>>>> {
        // Range scans for directory listings, chunk reads
    }
}
```

**SSTable Format:**
```
┌─────────────────────────────────────────┐
│           SSTable Structure              │
├─────────────────────────────────────────┤
│  ┌─────────────────────────────────┐    │
│  │ Data Blocks                     │    │
│  │ - Compressed (LZ4/Zstd)         │    │
│  │ - Key-value pairs sorted        │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │ Index Block                     │    │
│  │ - Block offsets                 │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │ Bloom Filter                    │    │
│  │ - Key existence check           │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │ Footer (metadata)               │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

### 2. Chunk Store (32KB Block Storage)

Files are split into 32KB chunks for efficient storage:

```rust
// From chunk.rs
pub struct ChunkStore {
    db: Arc<Db>,
}

impl ChunkStore {
    pub async fn read(&self, id: InodeId, offset: u64, length: u64) -> Result<Bytes> {
        let start_chunk = offset / CHUNK_SIZE as u64;
        let end_chunk = (offset + length - 1) / CHUNK_SIZE as u64;

        // Parallel chunk fetch
        let mut chunk_map: HashMap<u64, Bytes> = HashMap::new();
        let mut stream = self.db.scan(start_key..end_key).await?;

        while let Some((key, value)) = stream.next().await {
            if let Some(chunk_idx) = KeyCodec::parse_chunk_key(&key) {
                chunk_map.insert(chunk_idx, value);
            }
        }

        // Assemble result from chunks
        // ...
    }
}
```

### 3. Encryption (KeyManager)

ZeroFS uses a **key hierarchy**:

```
┌─────────────────────────────────────────┐
│          Encryption Hierarchy            │
├─────────────────────────────────────────┤
│                                          │
│  User Password                           │
│       │                                  │
│       ▼ (Argon2id)                       │
│  ┌─────────────────┐                     │
│  │ KEK (Key        │                     │
│  │ Encryption Key) │                     │
│  └────────┬────────┘                     │
│           │ (XChaCha20-Poly1305)         │
│           ▼                              │
│  ┌─────────────────┐                     │
│  │ DEK (Data       │                     │
│  │ Encryption Key) │                     │
│  └────────┬────────┘                     │
│           │ (XChaCha20-Poly1305)         │
│           ▼                              │
│  ┌─────────────────┐                     │
│  │ File Chunks     │                     │
│  │ (32KB each)     │                     │
│  └─────────────────┘                     │
│                                          │
└─────────────────────────────────────────┘
```

```rust
// From key_management.rs
pub struct KeyManager {
    argon2: Argon2<'static>,  // Argon2id for KDF
}

pub fn generate_and_wrap_key(&self, password: &str) -> Result<(WrappedDataKey, [u8; 32])> {
    // Generate random DEK
    let mut dek = [0u8; 32];
    thread_rng().fill_bytes(&mut dek);

    // Derive KEK from password using Argon2id
    let salt = SaltString::generate(&mut thread_rng());
    let kek = self.derive_kek(password, &salt)?;

    // Encrypt DEK with KEK using XChaCha20-Poly1305
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&kek));
    let wrapped_dek = cipher.encrypt(nonce, dek.as_ref())?;

    Ok((wrapped_key, dek))
}
```

### 4. RaptorQ (Erasure Coding)

RaptorQ provides **fountain codes** for data durability:

```rust
// From raptorq/src/encoder.rs
pub struct Encoder {
    config: ObjectTransmissionInformation,
    blocks: Vec<SourceBlockEncoder>,
}

impl Encoder {
    pub fn with_defaults(data: &[u8], mtu: u16) -> Encoder {
        // Partition data into blocks
        let blocks = calculate_block_offsets(data, &config);

        // Create source block encoders
        for block in blocks {
            let encoder = SourceBlockEncoder::with_encoding_plan(
                block_id,
                &config,
                block_data,
                &plan,
            );
        }
    }
}
```

**Recovery Properties:**
- Reconstruction probability after receiving K + h packets: `1 - 1/256^(h+1)`
- Where K = original packets, h = additional packets received

**Use Case:** Store data across multiple S3 buckets/regions. Lose entire buckets, still recover data.

### 5. FUSE Integration (fuser)

FUSE allows userspace filesystem implementations:

```rust
// From fuser/src/lib.rs
pub trait Filesystem {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        reply.error(EPERM);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        reply.error(EPERM);
    }

    fn read(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, size: u32, reply: ReplyData) {
        reply.error(EPERM);
    }

    fn write(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, data: &[u8], flags: u32, reply: ReplyWrite) {
        reply.error(EPERM);
    }
}
```

**Kernel Interface:**
```
┌─────────────────────────────────────────┐
│           FUSE Architecture              │
├─────────────────────────────────────────┤
│                                          │
│  Userspace                               │
│  ┌─────────────────────────────────┐    │
│  │  fuser Library                  │    │
│  │  - Session management           │    │
│  │  - Request/Reply protocol       │    │
│  │  - Mount/Unmount                │    │
│  └─────────────┬───────────────────┘    │
│                │ /dev/fuse              │
│  Kernel        │                        │
│  ┌─────────────▼───────────────────┐    │
│  │  FUSE Kernel Module             │    │
│  │  - VFS integration              │    │
│  │  - Page cache                   │    │
│  │  - Request queuing              │    │
│  └─────────────────────────────────┘    │
│                                          │
└─────────────────────────────────────────┘
```

### 6. Quickwit (Search Engine)

Quickwit is a **distributed search engine** that shares architectural similarities with ZeroFS:

- **Object storage native**: Stores indexes on S3
- **LSM-like structure**: Indexes are immutable segments
- **Distributed actors**: Actor-based concurrency model

```
┌─────────────────────────────────────────┐
│       Quickwit Actor Architecture        │
├─────────────────────────────────────────┤
│                                          │
│  ┌─────────────┐    ┌─────────────┐     │
│  │ Indexer     │    │ Searcher    │     │
│  │ Actor       │    │ Actor       │     │
│  └──────┬──────┘    └──────┬──────┘     │
│         │                  │             │
│         ▼                  ▼             │
│  ┌─────────────────────────────────┐    │
│  │      Universe (Actor Runtime)   │    │
│  └─────────────────────────────────┘    │
│         │                  │             │
│         ▼                  ▼             │
│  ┌─────────────┐    ┌─────────────┐     │
│  │ Object      │    │ Metastore   │     │
│  │ Storage     │    │ (SQLite/PG) │     │
│  └─────────────┘    └─────────────┘     │
│                                          │
└─────────────────────────────────────────┘
```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           ZeroFS Complete Architecture                   │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   NFS Client    │     │   9P Client     │     │   NBD Client    │
│   (Linux/macOS) │     │   (Linux/9k)    │     │   (ZFS/DB)      │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         │ NFS over TCP          │ 9P over TCP           │ NBD over TCP
         │ port 2049             │ port 5564             │ port 10809
         ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            ZeroFS Server                                 │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                      Protocol Servers                              │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │  │
│  │  │   NFS    │  │    9P    │  │   NBD    │  │    Checkpoint    │  │  │
│  │  │  Server  │  │  Server  │  │  Server  │  │      Manager     │  │  │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬─────────┘  │  │
│  └───────┼─────────────┼─────────────┼──────────────────┼────────────┘  │
│          │             │             │                  │                │
│          └─────────────┴──────┬──────┴──────────────────┘                │
│                               │                                          │
│                               ▼                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   Virtual Filesystem (VFS)                         │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │   Inode      │  │  Directory   │  │  Permission  │             │  │
│  │  │   Manager    │  │   Index      │  │   Checker    │             │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘             │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                               │                                          │
│                               ▼                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   Encryption Layer                                 │  │
│  │  - XChaCha20-Poly1305 for data                                   │  │
│  │  - Argon2id for key derivation                                   │  │
│  │  - DEK/KEK key hierarchy                                         │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                               │                                          │
│                               ▼                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   Cache Manager                                    │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │   Memory     │  │   Metadata   │  │    Disk      │             │  │
│  │  │   Block      │  │   Cache      │  │   Cache      │             │  │
│  │  │   Cache      │  │              │  │  (Foyer)     │             │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘             │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                               │                                          │
│                               ▼                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   SlateDB (LSM Tree)                               │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │   Memtable   │  │     WAL      │  │  Compactor   │             │  │
│  │  │  (in-memory) │  │  (append)    │  │  (background)│             │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘             │  │
│  │                                                                    │  │
│  │  ┌──────────────────────────────────────────────────────────────┐ │  │
│  │  │                    SSTables                                   │ │  │
│  │  │  - Immutable  - Sorted  - Compressed  - Bloom Filters        │ │  │
│  │  └──────────────────────────────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      Object Storage Backend                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ AWS S3   │  │  Azure   │  │  Google  │  │  MinIO   │  │  Local   │  │
│  │          │  │  Blob    │  │   GCS    │  │          │  │   FS     │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

### 1. Why LSM Trees Over Traditional Filesystems?

| Aspect | Traditional FS (ext4, XFS) | LSM Tree (ZeroFS) |
|--------|---------------------------|-------------------|
| Write Pattern | Random I/O | Sequential appends |
| Cloud Storage | Poor (many small objects) | Excellent (large immutable objects) |
| Compaction | Not applicable | Background merge of SSTables |
| Read Amplification | Low | Higher (multiple SSTables) |
| Write Amplification | Low | Higher (compaction rewrites) |

**Decision**: LSM trees are ideal for cloud storage because:
- Large, immutable objects = fewer S3 API calls
- Sequential writes = better throughput
- Compaction = efficient space reclamation

### 2. Why 32KB Chunks?

- **S3 pricing**: Optimized for objects > 1KB
- **Compression efficiency**: Good balance for LZ4/Zstd
- **Memory overhead**: Manageable at scale
- **Random I/O**: Fine-grained enough for most workloads

### 3. Why XChaCha20-Poly1305?

- **256-bit security**: Same as AES-256
- **Nonce misuse resistance**: XChaCha uses 192-bit nonces
- **No hardware dependencies**: Works everywhere (unlike AES-NI)
- **Authenticated encryption**: Detects tampering

### 4. Why NFS and 9P?

| Protocol | Advantages | Best For |
|----------|------------|----------|
| **NFS** | Universal support, kernel clients | General purpose, macOS/Linux |
| **9P** | Better POSIX semantics, simpler | High performance, Plan 9/inferno |
| **FUSE** | Full control | Custom protocols (not used) |

**ZeroFS chose NFS/9P** because:
- No custom kernel modules needed
- Battle-tested client implementations
- Network-first design (handles disconnections)

### 5. Why SlateDB?

- **Cloud-native**: Built for object storage
- **Rust**: Memory safe, high performance
- **Simple**: Single binary, no dependencies
- **WAL + Memtable**: Durable writes with buffering

---

## Related Projects

### Merkle Trees (merkle, ct-merkle)

Merkle trees provide **cryptographic verification**:

```
                    Root Hash
                   /         \
              Hash(A,B)     Hash(C,D)
              /     \       /     \
           Hash(A) Hash(B) Hash(C) Hash(D)
```

**Use cases:**
- Certificate Transparency (RFC 6962)
- Content-addressed storage
- Data integrity verification

### Bitpacking (bitpacking)

SIMD compression for sorted integers:

```rust
// Compress 128 integers using SSE3
let bitpacker = BitPacker4x::new();
let num_bits = bitpacker.num_bits(&data);  // e.g., 4 bits
let compressed = bitpacker.compress(&data, num_bits);
// Throughput: 5+ billion integers/second
```

**Use cases:**
- Inverted indexes (search engines)
- Columnar storage (databases)
- Checkpoint compression

### Intrusive Collections (intrusive-rs)

Memory-efficient collections without allocations:

```rust
// Intrusive linked list - nodes contain links
struct Node {
    links: ListLinks,
    data: u32,
}
```

**Advantages:**
- Zero allocations for collection structure
- Object can be in multiple collections
- Safe mutation while iterating

---

## Performance Benchmarks

### ZeroFS Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| Sequential Read (warm) | < 1ms | From cache |
| Sequential Read (cold) | 10-50ms | From S3 |
| Random Read (warm) | < 1ms | From cache |
| Random Read (cold) | 50-100ms | Multiple S3 requests |
| Write (buffered) | < 1ms | To memtable |
| Write (flushed) | 100-500ms | To S3 |

### RaptorQ Performance (Ryzen 9 5900X)

| Operation | Throughput |
|-----------|------------|
| Encoding (10 symbols) | 4.7 Gbit/s |
| Encoding (1000 symbols) | 4.7 Gbit/s |
| Decoding (10 symbols) | 3.2 Gbit/s |
| Decoding (1000 symbols) | 3.3 Gbit/s |

### Bitpacking Performance (i5-6600K)

| Operation | Throughput |
|-----------|------------|
| Compress (BitPacker4x) | 5.3 billion int/s |
| Decompress (BitPacker4x) | 5.5 billion int/s |

---

## Production Deployments

### Geo-Distributed ZFS

```bash
# Machine 1 - US East
zerofs run -c zerofs-us-east.toml  # s3://bucket/us-east-db

# Machine 2 - EU West
zerofs run -c zerofs-eu-west.toml  # s3://bucket/eu-west-db

# Machine 3 - Asia Pacific
zerofs run -c zerofs-asia.toml  # s3://bucket/asia-db

# Client: Create mirrored ZFS pool
zpool create global-pool mirror /dev/nbd0 /dev/nbd1 /dev/nbd2
```

**Result:** ZFS pool spanning three continents with automatic disaster recovery.

### PostgreSQL on ZeroFS

```
pgbench results (50 concurrent clients):
- Read/Write: 53,041 tps (latency: 0.943ms)
- Read-Only: 413,436 tps (latency: 0.121ms)
```

### Linux Kernel Compilation

ZeroFS can compile the Linux kernel in **16 seconds** using:
- NBD block device + ZFS
- Parallel compilation (`make -j$(nproc)`)

---

## References

### Documentation

- [ZeroFS Documentation](https://www.zerofs.net)
- [QuickStart Guide](https://www.zerofs.net/quickstart)
- [Architecture Overview](https://www.zerofs.net/architecture)

### RFCs and Specifications

- **RFC 6330**: RaptorQ Forward Error Correction
- **RFC 6962**: Certificate Transparency
- **C2SP Static CT**: Static Certificate Transparency API

### Related Crates

- **slatedb**: LSM-tree storage engine
- **foyer**: Cache library
- **object_store**: Object storage abstraction
- **chacha20poly1305**: AEAD encryption
- **argon2**: Password hashing

---

*This exploration covers the ZeroFS project and related components as of 2026-03-26. For the most current information, refer to the source repositories.*
