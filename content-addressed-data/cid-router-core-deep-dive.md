---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/core/
repository: N/A - exploration based on cid-router project
explored_at: 2026-03-19
language: Rust
parent: exploration.md
---

# CID Router Core Library Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/core/`

---

## Overview

The `cid-router-core` crate is the heart of the CID Router system, providing:
- CID types and utilities
- Content filtering system
- CRP (CID Route Provider) trait
- Route data structures
- Database abstraction
- Repository management

---

## Module Organization

```
core/src/
├── lib.rs          # Library exports
├── cid.rs          # CID, Codec, hash functions
├── cid_filter.rs   # Content filtering
├── context.rs      # Application context
├── crp.rs          # CRP trait
├── db.rs           # SQLite database
├── indexer.rs      # Background indexing
├── repo.rs         # Repository management
└── routes.rs       # Route data structures
```

---

## cid.rs - Content Identifiers and Codecs

### Purpose

Defines types for working with content-addressed data, including CID creation, codec handling, and hash function utilities.

### Key Types

#### Codec Enum

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Codec {
    Raw,             // 0x55 - Raw binary data
    DagCbor,         // 0x71 - CBOR-encoded DAG nodes
    GitRaw,          // 0x78 - Git object format
    Blake3HashSeq,   // 0x80 - BLAKE3 hash sequence
}
```

#### Multihash Codes

```rust
pub mod mh_codes {
    pub const SHA1: u64 = 0x11;      // Legacy (Git)
    pub const SHA256: u64 = 0x12;    // IPFS default
    pub const BLAKE3: u64 = 0x1e;    // Modern, fast
}

pub mod mc_codes {
    pub const RAW: u64 = 0x55;
    pub const DAG_CBOR: u64 = 0x71;
    pub const GIT_RAW: u64 = 0x78;
    pub const BLAKE3_HASHSEQ: u64 = 0x80;
}
```

### Key Functions

#### blake3_hash_to_cid

```rust
pub fn blake3_hash_to_cid(hash: Hash, codec: Codec) -> Cid {
    let mh = Multihash::wrap(crate::cid::mh_codes::BLAKE3, hash.as_bytes()).unwrap();
    Cid::new_v1(codec.code(), mh)
}
```

**Usage in Iroh CRP:**
```rust
let hash = blake3::hash(&data);
let cid = blake3_hash_to_cid(hash.into(), Codec::Raw);
```

---

## cid_filter.rs - Content Filtering System

### Purpose

Allows CRPs to declare which CIDs they can handle, enabling:
- Provider specialization (some only serve certain content types)
- Efficiency (skip providers that can't serve a CID)
- Security (reject unexpected content types)

### Filter Hierarchy

```rust
pub enum CidFilter {
    None,                              // Accept everything
    MultihashCodeFilter(CodeFilter),   // Filter by hash algorithm
    CodecFilter(CodeFilter),           // Filter by content codec
    And(Vec<Self>),                    // All conditions must match
    Or(Vec<Self>),                     // Any condition must match
    Not(Box<Self>),                    // Negation
}

pub enum CodeFilter<T> {
    Eq(T),                             // Equal to value
    Gt(T),                             // Greater than
    Lt(T),                             // Less than
    And(Vec<Self>),
    Or(Vec<Self>),
    Not(Box<Self>),
}
```

### Filter Evaluation

```rust
impl CidFilter {
    pub fn is_match(&self, cid: &Cid) -> bool {
        match self {
            Self::None => true,
            Self::MultihashCodeFilter(f) => f.is_match(cid.hash().code()),
            Self::CodecFilter(f) => f.is_match(cid.codec()),
            Self::And(fs) => fs.iter().all(|f| f.is_match(cid)),
            Self::Or(fs) => fs.iter().any(|f| f.is_match(cid)),
            Self::Not(f) => !f.is_match(cid),
        }
    }
}
```

### Example Filters

```rust
// Iroh only accepts BLAKE3 (0x1e)
let filter = CidFilter::MultihashCodeFilter(CodeFilter::Eq(0x1e));

// Accept BLAKE3 or SHA-256
let filter = CidFilter::MultihashCodeFilter(
    CodeFilter::Eq(0x1e) | CodeFilter::Eq(0x12)
);

// Accept raw or DAG-CBOR, but only with BLAKE3
let filter = CidFilter::CodecFilter(CodeFilter::Eq(0x55) | CodeFilter::Eq(0x71))
    & CidFilter::MultihashCodeFilter(CodeFilter::Eq(0x1e));
```

---

## context.rs - Application Context

### Purpose

Bundles shared state and identity for use across the system:
- Database access
- Cryptographic identity
- Signing capability

### Core Context

```rust
#[derive(Debug, Clone)]
pub struct Context {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    key: SecretKey,    // Ed25519 identity
    db: Db,            // SQLite database
}
```

### Signer Trait

```rust
pub trait Signer {
    fn public_key(&self) -> PublicKey;
    fn sign(&self, data: &[u8]) -> Signature;
}

impl Signer for Context {
    fn public_key(&self) -> PublicKey {
        self.inner.key.public()
    }

    fn sign(&self, data: &[u8]) -> Signature {
        self.inner.key.sign(data)
    }
}
```

---

## crp.rs - CID Route Provider Trait

### The CRP Trait

```rust
#[async_trait]
pub trait Crp: Send + Sync + Debug {
    /// Unique identifier for this provider instance
    fn provider_id(&self) -> String;

    /// Type of provider (for matching routes to providers)
    fn provider_type(&self) -> ProviderType;

    /// Reindex all content and update database
    async fn reindex(&self, cx: &Context) -> Result<()>;

    /// Get available capabilities
    fn capabilities<'a>(&'a self) -> CrpCapabilities<'a>;

    /// Filter for which CIDs this provider can handle
    fn cid_filter(&self) -> CidFilter;
}
```

### ProviderType Enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum ProviderType {
    Iroh,
    Azure,
}
```

### CrpCapabilities

```rust
pub struct CrpCapabilities<'a> {
    pub route_resolver: Option<&'a dyn RouteResolver>,
    pub blob_writer: Option<&'a dyn BlobWriter>,
}
```

### RouteResolver Trait

```rust
#[async_trait]
pub trait RouteResolver {
    async fn get_bytes(
        &self,
        route: &Route,
        auth: Option<bytes::Bytes>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<bytes::Bytes>> + Send>>>;
}
```

### BlobWriter Trait

```rust
#[async_trait]
pub trait BlobWriter {
    async fn put_blob(
        &self,
        auth: Option<bytes::Bytes>,
        cid: &Cid,
        data: &[u8],
    ) -> Result<()>;
}
```

---

## routes.rs - Route Data Structures

### Route Structure

```rust
pub struct Route {
    pub id: Uuid,
    pub created_at: DateTime,
    pub verified_at: DateTime,
    pub provider_id: String,
    pub provider_type: ProviderType,
    pub url: String,
    pub cid: CidGeneric<64>,
    pub size: u64,
    pub multicodec: Codec,
    pub creator: PublicKey,
    pub signature: Vec<u8>,
}
```

### Route Builder Pattern

```rust
pub struct RouteBuilder {
    id: Uuid,
    provider_id: String,
    provider_type: ProviderType,
    cid: Option<Cid>,
    size: Option<u64>,
    url: Option<String>,
    multicodec: Option<Codec>,
}

// Usage:
let route = Route::builder(&provider)
    .cid(cid)
    .size(size)
    .url(url)
    .multicodec(Codec::Raw)
    .build(&ctx)?;
```

### RouteStub - Two-Phase Indexing

```rust
pub struct RouteStub {
    pub id: Uuid,
    pub provider_id: String,
    pub provider_type: ProviderType,
    pub created_at: DateTime,
    pub verified_at: DateTime,
    pub multicodec: Option<Codec>,
    pub size: Option<u64>,
    pub url: String,
    // cid is null - will be computed later
}
```

**Why Stubs?** Some storage systems (like Azure Blob Storage) don't store content-addressed data natively:
1. Discover blobs in storage
2. Create stubs with URLs and sizes
3. Stream each blob and compute its CID
4. Complete the stub with the CID and signature

---

## db.rs - SQLite Database Layer

### Connection Management

```rust
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}
```

### Schema

```sql
CREATE TABLE routes (
    id TEXT PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL,
    verified_at TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    url TEXT NOT NULL,
    cid BLOB,
    size INTEGER,
    creator BLOB,
    signature BLOB,
    multicodec TEXT,
    UNIQUE(provider_id, provider_type, cid),
    UNIQUE(provider_id, provider_type, url)
);
```

### Key Operations

```rust
// Insert route
db.insert_route(&route).await?;

// List routes with pagination
db.list_routes(OrderBy::CreatedAt(Direction::Desc), 0, 100).await?;

// Get routes for CID
db.routes_for_cid(cid).await?;

// Two-phase operations
db.insert_stub(&stub).await?;
db.complete_stub(&route).await?;
```

---

## indexer.rs - Background Indexing

```rust
pub struct Indexer {
    _task: tokio::task::JoinHandle<()>,
}

impl Indexer {
    pub async fn spawn(
        interval_seconds: u64,
        cx: Context,
        providers: Vec<Arc<dyn Crp>>,
    ) -> Self {
        let task = tokio::spawn(async move {
            loop {
                for provider in &providers {
                    let _ = provider.reindex(&cx).await;
                }
                tokio::time::sleep(Duration::from_secs(interval_seconds)).await;
            }
        });
        Self { _task: task }
    }
}
```

---

## repo.rs - Repository Management

### Structure

```rust
pub struct Repo(PathBuf);

impl Repo {
    const DB_FILE: &str = "db.sqlite";
    const KEY_FILE: &str = "key";
    const CONFIG_FILE: &str = "config.toml";

    pub fn default_location() -> PathBuf {
        dirs_next::data_local_dir().unwrap().join("cid-router")
    }

    pub async fn open_or_create(base_dir: impl Into<PathBuf>) -> Result<Self> {
        // Creates secret key if missing
    }

    pub async fn db(&self) -> Result<Db> {
        Db::open_or_create(self.0.join(Self::DB_FILE)).await
    }

    pub async fn secret_key(&self) -> Result<SecretKey> {
        // Load from file
    }
}
```

### Platform-Specific Locations

```
Linux:   ~/.local/share/cid-router/
macOS:   ~/Library/Application Support/cid-router/
Windows: ~\AppData\Local\cid-router\
```

---

## Testing

### CID Filter Tests

```rust
#[test]
fn multihash_eq() {
    let filter = CidFilter::MultihashCodeFilter(CodeFilter::Eq(BLAKE3));
    assert!(filter.is_match(&blake3_raw()));
    assert!(!filter.is_match(&sha256_raw()));
}
```

### Database Tests

```rust
#[tokio::test]
async fn test_route_persistence() {
    let ctx = Context::mem().await.unwrap();
    let db = Db::new_in_memory().await.unwrap();

    let route = Route::builder(&provider)
        .cid(cid)
        .size(1024)
        .url("/test/route")
        .build(&ctx)
        .unwrap();

    db.insert_route(&route).await.unwrap();
    let routes = db.list_routes(...).await.unwrap();
    assert_eq!(routes.len(), 1);
}
```

---

## See Also

- [Architecture Overview](./cid-router-architecture-deep-dive.md)
- [Iroh CRP Deep Dive](./cid-router-iroh-crp-deep-dive.md)
- [Azure CRP Deep Dive](./cid-router-azure-crp-deep-dive.md)
- [Server API Deep Dive](./cid-router-server-deep-dive.md)
