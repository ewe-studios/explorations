---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.turso/libsql
repository: git@github.com:tursodatabase/libsql.git
revised_at: 2026-03-22
workspace: libsql-rust
---

# Rust Revision: libSQL/Turso

## Overview

This Rust revision translates the libSQL codebase into idiomatic Rust patterns. The key insight is that libSQL is already substantially written in Rust, so this revision focuses on:

1. **Architectural improvements** -- Cleaner separation of concerns, better error handling
2. **Type safety enhancements** -- More compile-time guarantees
3. **Async improvements** -- Better tokio integration, connection pooling
4. **API ergonomics** -- More intuitive builder patterns, better diagnostics

## Workspace Structure

```
libsql-rust/
  Cargo.toml              # Workspace root
  crates/
    libsql-core/          # Core types and traits (no runtime deps)
    libsql-ffi/           # Raw FFI to SQLite C code
    libsql-wal/           # WAL abstraction layer
    libsql-replication/   # Replication protocol
    libsql-hrana/         # Hrana protocol types
    libsql-client/        # High-level client API
    libsql-server/        # sqld server binary
    libsql-encryption/    # Encryption layer
    libsql-vector/        # Vector search extension
    libsql-ft5/           # Full-text search extension
    bottomless/           # S3 backup layer
  examples/
    embedded-replica/     # Embedded replica example
    remote-client/        # Remote client example
    vector-search/        # Vector search example
  tests/
    integration/          # Integration tests
    cluster/              # Multi-node tests
```

## Crate Breakdown

### libsql-core

**Purpose:** Core types, traits, and error definitions used across all crates.

**Type:** Library

**Public API:**
```rust
// Core database types
pub struct DatabaseId(pub Uuid);
pub struct NamespaceId(pub u64);
pub struct FrameNo(pub u64);
pub struct Lsn(pub u64);

// Page and frame types
pub struct Page {
    pub number: PageNo,
    pub data: Box<[u8; PAGE_SIZE]>,
}

pub struct Frame {
    pub header: FrameHeader,
    pub page: Page,
}

// Core traits
pub trait Database: Send + Sync {
    type Connection: Connection;
    type Transaction: Transaction;

    async fn connect(&self) -> Result<Self::Connection>;
    fn id(&self) -> DatabaseId;
}

pub trait Connection: Send + Sync {
    type Transaction: Transaction;

    async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64>;
    async fn query(&self, sql: &str, params: &[Value]) -> Result<Rows>;
    async fn transaction(&self) -> Result<Self::Transaction>;
    fn prepare(&self, sql: &str) -> Result<Statement>;
}

pub trait Transaction: Send + Sync {
    async fn execute(&self, sql: &str, params: &[Value]) -> Result<u64>;
    async fn query(&self, sql: &str, params: &[Value]) -> Result<Rows>;
    async fn commit(self) -> Result<()>;
    async fn rollback(self) -> Result<()>;
}
```

**Dependencies:** None (pure Rust types)

### libsql-ffi

**Purpose:** Safe Rust FFI wrapper around SQLite C code.

**Type:** Library (unsafe)

**Public API:**
```rust
// Safe wrapper around sqlite3* handle
pub struct SqliteHandle {
    inner: NonNull<ffi::sqlite3>,
    _marker: PhantomData<*mut ffi::sqlite3>,
}

impl SqliteHandle {
    pub fn open(path: &CStr, flags: OpenFlags) -> Result<Self> {
        unsafe {
            let mut handle = ptr::null_mut();
            let rc = ffi::sqlite3_open_v2(path.as_ptr(), &mut handle, flags.bits(), ptr::null());
            if rc != ffi::SQLITE_OK {
                return Err(Error::from_sqlite_error(rc));
            }
            Ok(Self {
                inner: NonNull::new(handle).unwrap(),
                _marker: PhantomData,
            })
        }
    }
}

// RAII guard ensures proper cleanup
impl Drop for SqliteHandle {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_close(self.inner.as_mut());
        }
    }
}

// Statement wrapper
pub struct PreparedStmt {
    stmt: NonNull<ffi::sqlite3_stmt>,
    _handle: Arc<SqliteHandle>,
}
```

**Dependencies:** `libc`, `bitflags`

### libsql-wal

**Purpose:** Virtual WAL trait system (the key architectural innovation).

**Type:** Library

**Public API:**
```rust
/// Core WAL trait - all WAL implementations must implement this
pub trait Wal: Send + Sync + 'static {
    /// Begin a read transaction
    fn begin_read_txn(&mut self) -> Result<bool>;

    /// End a read transaction
    fn end_read_txn(&mut self);

    /// Find the frame containing a page
    fn find_frame(&mut self, page_no: NonZeroU32) -> Result<Option<NonZeroU32>>;

    /// Read a frame from the WAL
    fn read_frame(&mut self, frame_no: NonZeroU32, buffer: &mut [u8]) -> Result<()>;

    /// Begin a write transaction
    fn begin_write_txn(&mut self) -> Result<()>;

    /// Insert frames into the WAL
    fn insert_frames(&mut self, pages: &[Page], commit: bool) -> Result<usize>;

    /// Checkpoint the WAL
    fn checkpoint(&mut self, db: &mut Sqlite3Db, mode: CheckpointMode) -> Result<CheckpointStatus>;
}

/// WAL factory trait
pub trait WalManager: Send + Sync + 'static {
    type Wal: Wal;

    fn open(&self, vfs: &mut Vfs, file: &mut Sqlite3File, flags: WalFlags) -> Result<Self::Wal>;
    fn close(&self, wal: &mut Self::Wal, db: &mut Sqlite3Db) -> Result<()>;
    fn destroy_log(&self, vfs: &mut Vfs, db_path: &CStr) -> Result<()>;

    /// Wrap this WAL manager with another layer
    fn wrap<U>(self, wrapper: U) -> WalWrapper<U, Self>
    where
        U: WrapWal<Self::Wal>,
        Self: Sized,
    {
        WalWrapper::new(wrapper, self)
    }
}

/// Trait for wrapping WAL implementations (composability)
pub trait WrapWal<W: Wal>: Send + Sync + 'static {
    type WrappedWal: Wal;

    fn wrap<M: WalManager<Wal = W>>(self, inner: M) -> Self::WrappedWal;
}

// Example: Replication logger wrapper
pub struct ReplicationLoggerWrapper {
    tx: broadcast::Sender<Frame>,
}

impl<W: Wal> WrapWal<W> for ReplicationLoggerWrapper {
    type WrappedWal = ReplicationLoggerWal<W>;

    fn wrap<M: WalManager<Wal = W>>(self, inner: M) -> Self::WrappedWal {
        ReplicationLoggerWal {
            inner: inner.open(...)?,
            logger: self,
        }
    }
}
```

**Dependencies:** `libsql-ffi`, `tokio`, `broadcast`

**Key Design Decision:** The `WrapWal` trait enables composable WAL wrappers. This is how the replication logger intercepts writes without modifying core SQLite code.

### libsql-replication

**Purpose:** Frame-based replication protocol.

**Type:** Library

**Public API:**
```rust
/// Frame format for replication
#[derive(Debug, Clone)]
pub struct Frame {
    pub header: FrameHeader,
    pub page: [u8; PAGE_SIZE],
}

#[derive(Debug, Clone, Copy)]
pub struct FrameHeader {
    pub frame_no: u64,      // Monotonically increasing
    pub checksum: u64,      // CRC-64
    pub page_no: u32,       // SQLite page number
    pub size_after: u32,    // DB size after commit (0 = not commit)
}

/// gRPC client for replication
#[async_trait]
pub trait ReplicatorClient: Send + Sync {
    type FrameStream: Stream<Item = Result<Frame>> + Unpin + Send;

    async fn handshake(&mut self) -> Result<HelloResponse>;
    async fn next_frames(&mut self, from: FrameNo) -> Result<Self::FrameStream>;
    async fn snapshot(&mut self) -> Result<Self::FrameStream>;
    async fn commit_frame_no(&mut self, frame_no: FrameNo) -> Result<()>;
    fn committed_frame_no(&self) -> Option<FrameNo>;
    fn rollback(&mut self);
}

/// Replication state machine
pub struct Replicator<C: ReplicatorClient> {
    client: C,
    state: ReplicatorState,
    injector: SqliteInjector,
}

enum ReplicatorState {
    Handshake,
    CatchingUp { from: FrameNo },
    Streaming { last_frame: FrameNo },
    Error { reason: String },
}

impl<C: ReplicatorClient> Replicator<C> {
    pub async fn run(mut self) -> Result<()> {
        // 1. Handshake
        let hello = self.client.handshake().await?;

        // 2. Determine sync point
        let from = self.injector.current_frame_no().unwrap_or(0);

        // 3. Stream frames
        let mut stream = self.client.next_frames(from).await?;

        while let Some(frame) = stream.next().await {
            let frame = frame?;
            self.injector.apply(frame).await?;
        }

        Ok(())
    }
}
```

**Dependencies:** `libsql-wal`, `libsql-hrana`, `tokio`, `tonic`, `prost`, `crc64`

### libsql-hrana

**Purpose:** Hrana protocol types and serialization.

**Type:** Library

**Public API:**
```rust
/// Hrana request types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    #[serde(rename = "execute")]
    Execute(ExecuteRequest),
    #[serde(rename = "batch")]
    Batch(BatchRequest),
    #[serde(rename = "close")]
    Close,
    #[serde(rename = "open")]
    Open(OpenStreamRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub sql: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub want_rows: Option<bool>,
}

/// Hrana response types
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub type: String,
    pub response: Option<ResponseInner>,
    pub error: Option<Error>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultSet {
    pub cols: Vec<Column>,
    pub rows: Vec<Row>,
    pub affected_row_count: u64,
    pub last_insert_rowid: Option<i64>,
}

/// WebSocket client for Hrana
pub struct HranaClient {
    ws: WebSocketStream,
    streams: DashMap<StreamId, StreamState>,
}
```

**Dependencies:** `serde`, `serde_json`, `tokio-tungstenite`

### libsql-client

**Purpose:** High-level client API.

**Type:** Library

**Public API:**
```rust
/// Database builder
pub struct Builder {
    config: DatabaseConfig,
}

impl Builder {
    /// Create a new local database
    pub fn new_local(path: impl Into<PathBuf>) -> Self {
        Self {
            config: DatabaseConfig::Local { path: path.into() },
        }
    }

    /// Create an embedded replica
    pub fn new_remote_replica(
        path: impl Into<PathBuf>,
        url: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            config: DatabaseConfig::EmbeddedReplica {
                path: path.into(),
                url: url.into(),
                auth_token: auth_token.into(),
            },
        }
    }

    /// Create a remote-only client
    pub fn new_remote(url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            config: DatabaseConfig::Remote {
                url: url.into(),
                auth_token: auth_token.into(),
            },
        }
    }

    /// Enable encryption
    pub fn encryption_key(mut self, key: impl Into<String>) -> Self {
        self.config.encryption_key = Some(key.into());
        self
    }

    /// Enable read-your-writes consistency
    pub fn read_your_writes(mut self, enabled: bool) -> Self {
        self.config.read_your_writes = enabled;
        self
    }

    /// Build the database
    pub async fn build(self) -> Result<Database> {
        match self.config {
            DatabaseConfig::Local { path } => {
                let handle = SqliteHandle::open(&path)?;
                Ok(Database::local(handle))
            }
            DatabaseConfig::EmbeddedReplica { path, url, auth_token } => {
                let local = SqliteHandle::open(&path)?;
                let remote = HranaClient::connect(&url, &auth_token).await?;
                Ok(Database::embedded_replica(local, remote))
            }
            DatabaseConfig::Remote { url, auth_token } => {
                let client = HranaClient::connect(&url, &auth_token).await?;
                Ok(Database::remote(client))
            }
        }
    }
}

/// Row type with ergonomic accessors
pub struct Row {
    columns: Arc<[String]>,
    values: Box<[Value]>,
}

impl Row {
    pub fn get<T: FromValue>(&self, idx: usize) -> Result<T> {
        T::from_value(&self.values[idx])
    }

    pub fn get_named<T: FromValue>(&self, name: &str) -> Result<T> {
        let idx = self.columns.iter()
            .position(|c| c == name)
            .ok_or(Error::ColumnNotFound(name.to_string()))?;
        self.get(idx)
    }
}
```

**Dependencies:** `libsql-core`, `libsql-hrana`, `libsql-replication`, `tokio`

### libsql-server

**Purpose:** sqld server binary.

**Type:** Binary

**Key Components:**
```rust
/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub db_path: PathBuf,
    pub http_addr: SocketAddr,
    pub grpc_addr: Option<SocketAddr>,
    pub admin_addr: Option<SocketAddr>,
    pub auth: AuthConfig,
    pub namespaces: bool,
}

/// Namespace-aware connection router
pub struct NamespaceRouter {
    namespaces: DashMap<NamespaceName, Namespace>,
    metadata: MetaStore,
}

impl NamespaceRouter {
    pub async fn get_or_create(&self, name: &NamespaceName) -> Result<Namespace> {
        if let Some(ns) = self.namespaces.get(name) {
            return Ok(ns.clone());
        }

        // Create new namespace
        let ns = Namespace::create(name, &self.metadata).await?;
        self.namespaces.insert(name.clone(), ns.clone());
        Ok(ns)
    }
}

/// HTTP/Hrana handler
pub struct HranaHandler {
    router: Arc<NamespaceRouter>,
    auth: Arc<AuthManager>,
}

#[axum::debug_handler]
async fn handle_hrana(
    ws: WebSocketUpgrade,
    auth: AuthHeader,
    handler: State<Arc<HranaHandler>>,
) -> Result<Response> {
    // Validate auth
    let authenticated = handler.auth.validate(&auth).await?;

    // Upgrade to WebSocket
    Ok(ws.on_upgrade(move |socket| {
        handler.handle_socket(socket, authenticated)
    }))
}
```

**Dependencies:** `libsql-core`, `libsql-wal`, `libsql-replication`, `libsql-hrana`, `tokio`, `axum`, `tonic`, `tower`, `jsonwebtoken`

### libsql-encryption

**Purpose:** Transparent database encryption.

**Type:** Library

**Public API:**
```rust
pub struct EncryptionConfig {
    pub cipher: Cipher,
    pub key: EncryptionKey,
}

pub enum Cipher {
    Aes256Cbc,
    ChaCha20Poly1305,
    Ascon,
}

pub struct EncryptionKey {
    bytes: [u8; 32],  // 256-bit key
}

impl EncryptionKey {
    pub fn from_str(s: &str) -> Result<Self> {
        // Key derivation using PBKDF2
        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(s.as_bytes(), b"libsql", 100000, &mut key)?;
        Ok(Self { bytes: key })
    }
}

/// WAL encryption wrapper
pub struct EncryptedWal<W: Wal> {
    inner: W,
    encryptor: Aes256CbcEncryptor,
    decryptor: Aes256CbcDecryptor,
}

impl<W: Wal> Wal for EncryptedWal<W> {
    fn insert_frames(&mut self, pages: &[Page], commit: bool) -> Result<usize> {
        // Encrypt pages before writing
        let encrypted_pages: Vec<Page> = pages.iter()
            .map(|p| self.encrypt_page(p))
            .collect::<Result<_>>()?;

        self.inner.insert_frames(&encrypted_pages, commit)
    }

    fn read_frame(&mut self, frame_no: NonZeroU32, buffer: &mut [u8]) -> Result<()> {
        self.inner.read_frame(frame_no, buffer)?;

        // Decrypt after reading
        self.decrypt_page(buffer)?;

        Ok(())
    }
}
```

**Dependencies:** `libsql-wal`, `aes`, `cbc`, `chacha20poly1305`, `pbkdf2`, `hmac`, `sha2`

### libsql-vector

**Purpose:** Vector similarity search extension.

**Type:** Library

**Public API:**
```rust
/// Vector type with distance metrics
pub struct Vector {
    pub dimensions: u32,
    pub data: Vec<f32>,
}

impl Vector {
    pub fn cosine_similarity(&self, other: &Vector) -> Result<f32> {
        if self.dimensions != other.dimensions {
            return Err(Error::DimensionMismatch);
        }

        let dot = self.data.iter()
            .zip(other.data.iter())
            .map(|(a, b)| a * b)
            .sum::<f32>();

        let norm_a = self.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b = other.data.iter().map(|x| x * x).sum::<f32>().sqrt();

        Ok(dot / (norm_a * norm_b))
    }

    pub fn euclidean_distance(&self, other: &Vector) -> Result<f32> {
        if self.dimensions != other.dimensions {
            return Err(Error::DimensionMismatch);
        }

        let sum = self.data.iter()
            .zip(other.data.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>();

        Ok(sum.sqrt())
    }
}

/// Vector index for similarity search
pub trait VectorIndex: Send + Sync {
    fn insert(&mut self, id: u64, vector: &Vector) -> Result<()>;
    fn search(&self, query: &Vector, k: usize) -> Result<Vec<(u64, f32)>>;
    fn delete(&mut self, id: u64) -> Result<()>;
}

/// DiskANN-based index implementation
pub struct DiskAnnIndex {
    index: Index,
    config: DiskAnnConfig,
}

pub struct DiskAnnConfig {
    pub max_dimensions: u32,
    pub pruning_alpha: f32,
    pub insert_l: u32,
    pub search_l: u32,
}
```

**Dependencies:** `libsql-core`, `ndarray`, ` rayon`

### bottomless

**Purpose:** S3-backed WAL backup.

**Type:** Library

**Public API:**
```rust
pub struct Replicator {
    client: aws_sdk_s3::Client,
    bucket: String,
    db_path: String,
    generation: Arc<AtomicUuid>,
    next_frame: Arc<AtomicU32>,
}

impl Replicator {
    pub async fn new(config: S3Config) -> Result<Self> {
        let config_builder = aws_sdk_s3::Config::builder()
            .region(Region::new(config.region))
            .endpoint_url(config.endpoint);

        if let Some(creds) = config.credentials {
            config_builder = config_builder
                .credentials_provider(creds);
        }

        Ok(Self {
            client: aws_sdk_s3::Client::from_conf(config_builder.build()),
            bucket: config.bucket,
            db_path: config.db_path,
            generation: Arc::new(AtomicUuid::new(uuid::Uuid::new_v7())),
            next_frame: Arc::new(AtomicU32::new(0)),
        })
    }

    /// Upload WAL frames to S3
    pub async fn upload_frames(&self, frames: &[Frame]) -> Result<()> {
        let key = format!(
            "{}/{}/{:020}-{:020}.wal",
            self.db_path,
            self.generation.load(),
            frames.first().unwrap().header.frame_no,
            frames.last().unwrap().header.frame_no,
        );

        let data = self.serialize_and_compress(frames)?;

        self.client.put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .send()
            .await?;

        Ok(())
    }

    /// Restore database from S3
    pub async fn restore(&self, target: &Path) -> Result<()> {
        // List all generations
        let generations = self.list_generations().await?;

        // Find most recent with snapshot
        let target_generation = generations.iter()
            .find(|g| g.has_snapshot)
            .ok_or(Error::NoSnapshotFound)?;

        // Download snapshot
        let snapshot = self.download_snapshot(target_generation).await?;

        // Apply WAL frames forward
        for generation in generations.iter().rev() {
            let frames = self.download_frames(generation).await?;
            self.apply_frames(&snapshot, &frames).await?;
        }

        Ok(())
    }
}
```

**Dependencies:** `aws-sdk-s3`, `libsql-replication`, `tokio`, `uuid`, `zstd`

## Type System Design

### Core Types

```rust
/// Database identifier (ULID for time-sortable uniqueness)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DatabaseId(pub Ulid);

/// Namespace name (validated at construction)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamespaceName {
    name: Arc<str>,
}

impl NamespaceName {
    pub fn new(name: &str) -> Result<Self> {
        // Validate: alphanumeric + dash/underscore, max 64 chars
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(Error::InvalidNamespaceName);
        }
        if name.len() > 64 || name.is_empty() {
            return Err(Error::InvalidNamespaceName);
        }

        Ok(Self { name: name.into() })
    }
}

/// Frame number with ordering guarantees
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameNo(pub u64);

impl FrameNo {
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

/// Checksum type for integrity verification
#[derive(Debug, Clone, Copy)]
pub struct Checksum(pub u64);

impl Checksum {
    pub fn compute(data: &[u8]) -> Self {
        use crc64fast::Digest;
        let mut digest = Digest::new();
        digest.write(data);
        Self(digest.sum64())
    }

    pub fn verify(&self, data: &[u8]) -> bool {
        Self::compute(data) == *self
    }
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // SQLite errors
    #[error("SQLite error {code}: {message}")]
    Sqlite { code: c_int, message: String },

    // FFI errors
    #[error("FFI error: {0}")]
    Ffi(String),

    // Protocol errors
    #[error("Hrana protocol error: {0}")]
    Hrana(#[from] HranaError),

    #[error("Replication error: {0}")]
    Replication(#[from] ReplicationError),

    // Type errors
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Null value for non-null column")]
    NullValue,

    // Connection errors
    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Connection timeout")]
    ConnectionTimeout,

    // Authentication errors
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    // Namespace errors
    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),

    #[error("Invalid namespace name")]
    InvalidNamespaceName,

    // Replication errors
    #[error("Replication lag too high: {0} seconds")]
    ReplicationLag(u64),

    #[error("Frame checksum mismatch")]
    ChecksumMismatch,
}

pub type Result<T> = std::result::Result<T, Error>;
```

### Traits

```rust
/// Trait for converting Rust values to SQL parameters
pub trait ToValue: Send + Sync {
    fn to_value(&self) -> Result<Value>;
}

impl ToValue for i64 {
    fn to_value(&self) -> Result<Value> {
        Ok(Value::Integer(*self))
    }
}

impl ToValue for &str {
    fn to_value(&self) -> Result<Value> {
        Ok(Value::Text(self.to_string()))
    }
}

impl ToValue for Vec<u8> {
    fn to_value(&self) -> Result<Value> {
        Ok(Value::Blob(self.clone()))
    }
}

/// Trait for converting SQL values to Rust types
pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self>;
}

impl FromValue for i64 {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Integer(n) => Ok(*n),
            _ => Err(Error::TypeMismatch {
                expected: "Integer".into(),
                actual: value.type_name().into(),
            }),
        }
    }
}

impl FromValue for String {
    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Text(s) => Ok(s.clone()),
            _ => Err(Error::TypeMismatch {
                expected: "Text".into(),
                actual: value.type_name().into(),
            }),
        }
    }
}
```

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Async runtime | `tokio` | 1.38 | Full features, multi-threaded runtime |
| gRPC | `tonic` | 0.11 | Best-in-class gRPC implementation |
| HTTP framework | `axum` | 0.7 | Ergonomic, tower-based |
| Serialization | `serde` + `serde_json` | 1.0 | Standard |
| Protobuf | `prost` | 0.12 | Lightweight protobuf |
| Error handling | `thiserror` | 2.0 | Zero-cost error types |
| Tracing | `tracing` + `tracing-subscriber` | 0.1 | Observability |
| Metrics | `metrics` + `metrics-exporter-prometheus` | 0.21/0.13 | Prometheus metrics |
| Caching | `moka` | 0.12 | High-performance concurrent cache |
| S3 client | `aws-sdk-s3` | 1.0 | Official AWS SDK |
| Encryption | `aes`, `cbc`, `chacha20poly1305` | 0.8/0.1 | RustCrypto |
| UUID | `uuid` | 1.0 | ULID for sortable IDs |
| CRC | `crc64fast` | 1.0 | Fast CRC-64 |
| Compression | `zstd` | 0.13 | Fast compression |

## Key Rust-Specific Changes

### 1. Type-Safe Namespace Names

**Source Pattern:** Strings passed around freely, validated at runtime.

**Rust Translation:** Newtype wrapper with validated construction.

```rust
// Before: any string can be a namespace name
fn get_namespace(name: &str) -> Result<Namespace>;

// After: must construct valid NamespaceName first
fn get_namespace(name: &NamespaceName) -> Result<Namespace>;

// Construction enforces validation
let name = NamespaceName::new("my-namespace")?;  // OK
let bad = NamespaceName::new("invalid/name");     // Error at compile time
```

**Rationale:** Moves validation from runtime to construction time.

### 2. Result Types with Context

**Source Pattern:** `anyhow::Error` throughout.

**Rust Translation:** Specific error types with `thiserror`.

```rust
// Before: anyhow::Result<T>
fn execute_query(sql: &str) -> anyhow::Result<ResultSet>;

// After: specific error type
fn execute_query(sql: &str) -> Result<ResultSet> {
    // Error includes context
}
```

**Rationale:** Callers can pattern match on specific error variants.

### 3. RAII for Resource Management

**Source Pattern:** Manual cleanup in finally blocks.

**Rust Translation:** RAII guards with Drop.

```rust
pub struct TransactionGuard<'a> {
    tx: &'a mut Transaction,
    committed: bool,
}

impl<'a> Drop for TransactionGuard<'a> {
    fn drop(&mut self) {
        if !self.committed {
            // Auto-rollback on drop
            let _ = self.tx.rollback();
        }
    }
}

// Usage
fn do_transaction(tx: &mut Transaction) -> Result<()> {
    let _guard = TransactionGuard { tx, committed: false };
    // ... do work ...
    _guard.committed = true;
    tx.commit()?;
    Ok(())
}
```

**Rationale:** Ensures cleanup even on panic or early return.

## Ownership & Borrowing Strategy

```rust
/// Database handle uses Arc for shared ownership
pub struct Database {
    inner: Arc<DatabaseInner>,
}

#[derive(Clone)]
pub struct Database {
    inner: Arc<DatabaseInner>,
}

/// Connection borrows from Database
impl Database {
    pub fn connect(&self) -> Result<Connection> {
        // Connection gets clone of Arc, no borrowing issues
        Ok(Connection {
            inner: Arc::clone(&self.inner),
        })
    }
}

/// Statement borrows from Connection
pub struct Statement<'conn> {
    conn: &'conn Connection,
    stmt: RawStatement,
}

impl<'conn> Statement<'conn> {
    pub fn query(&mut self, params: &[Value]) -> Result<Rows<'conn>> {
        // Rows borrows from Statement, which borrows from Connection
        Ok(Rows { stmt: self })
    }
}
```

**Key Decisions:**
- `Arc` for shared ownership of Database/Connection
- Borrowing for short-lived objects (Statement, Rows)
- `PhantomData` for lifetime tracking

## Concurrency Model

**Approach:** Async with tokio runtime.

**Rationale:**
- libSQL is I/O bound (network, disk)
- Async allows high concurrency with fewer threads
- tokio has best ecosystem support

```rust
/// Tokio-based server
pub async fn run_server(config: ServerConfig) -> Result<()> {
    let server = Arc::new(Server::new(config).await?);

    // Spawn HTTP and gRPC servers concurrently
    tokio::select! {
        result = server.run_http() => result,
        result = server.run_grpc() => result,
    }
}

/// Connection pool with semaphore
pub struct ConnectionPool {
    db: Arc<Database>,
    semaphore: Arc<Semaphore>,
}

impl ConnectionPool {
    pub async fn acquire(&self) -> Result<ConnectionGuard> {
        let permit = self.semaphore.acquire().await?;
        Ok(ConnectionGuard {
            conn: self.db.connect()?,
            _permit: permit,
        })
    }
}
```

## Memory Considerations

- **Stack vs. Heap:** Pages are boxed (`Box<[u8; 4096]>`) to avoid stack overflow
- **Arc for shared state:** Database, Namespace, Connection use Arc
- **No unsafe code** except in FFI layer
- **Zero-copy where possible:** `ValueRef` borrows from row data

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Concurrent writes to same page | WAL serialization via mutex |
| Checksum mismatch | Return error, request retransmission |
| Network partition during replication | Buffer frames locally, retry on reconnect |
| S3 upload failure | Retry with exponential backoff |
| Corrupted WAL frame | Detect via checksum, trigger snapshot recovery |
| Out of disk space | Return error before SQLite operation |
| Transaction timeout | Automatic rollback via RAII guard |

## Code Examples

### Example: Embedded Replica Setup

```rust
use libsql_client::{Builder, Value};

#[tokio::main]
async fn main() -> Result<()> {
    // Create embedded replica
    let db = Builder::new_remote_replica(
        "/var/lib/myapp/db.sqlite",
        "libsql://my-app.turso.io",
        "eyJhbGc...",
    )
    .read_your_writes(true)
    .build()
    .await?;

    let conn = db.connect()?;

    // Create table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, email TEXT)",
        [],
    ).await?;

    // Insert
    conn.execute(
        "INSERT INTO users (email) VALUES (?1)",
        [Value::from("user@example.com")],
    ).await?;

    // Query
    let mut rows = conn.query("SELECT * FROM users", []).await?;

    while let Some(row) = rows.next().await? {
        let id: i64 = row.get::<i64>(0)?;
        let email: String = row.get::<String>(1)?;
        println!("User {}: {}", id, email);
    }

    // Manual sync (optional, auto-syncs in background)
    db.sync().await?;

    Ok(())
}
```

### Example: Vector Search

```rust
use libsql_vector::{Vector, VectorIndex, DiskAnnIndex};

#[tokio::main]
async fn main() -> Result<()> {
    let db = Builder::new_local("vector.db").build().await?;
    let conn = db.connect()?;

    // Create vector extension
    conn.execute("LOAD EXTENSION 'libsql_vector'", []).await?;

    // Create table with vector column
    conn.execute(
        "CREATE TABLE embeddings (
            id INTEGER PRIMARY KEY,
            text TEXT,
            vector BLOB
        )",
        [],
    ).await?;

    // Insert embeddings
    let vector = Vector::from(vec![0.1f32; 1536]);
    conn.execute(
        "INSERT INTO embeddings (text, vector) VALUES (?1, ?2)",
        [Value::from("hello"), Value::Blob(vector.to_bytes())],
    ).await?;

    // Query by similarity
    let query = Vector::from(vec![0.2f32; 1536]);
    let mut rows = conn.query(
        "SELECT text, vector_distance_cos(vector, ?1) as dist
         FROM embeddings
         ORDER BY dist ASC
         LIMIT 10",
        [Value::Blob(query.to_bytes())],
    ).await?;

    while let Some(row) = rows.next().await? {
        let text: String = row.get::<String>(0)?;
        let dist: f64 = row.get::<f64>(1)?;
        println!("{} (distance: {})", text, dist);
    }

    Ok(())
}
```

## Migration Path

1. **Phase 1: Use existing libsql crate** -- The current implementation is functional
2. **Phase 2: Incremental improvements** -- Add type-safe wrappers, better errors
3. **Phase 3: Refactor WAL trait** -- Make more composable with WrapWal
4. **Phase 4: Improve async ergonomics** -- Better connection pooling, statement caching
5. **Phase 5: Add observability** -- Tracing, metrics, structured logging

## Performance Considerations

- **Batch frame uploads** -- Group WAL frames for S3 (reduces API calls)
- **Parallel replication** -- Multiple goroutines for frame application
- **Statement cache** -- LRU cache of prepared statements
- **Connection pooling** -- Reuse connections, limit concurrent
- **Compression** -- zstd for WAL frames (trade CPU for bandwidth)

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedded_replica() {
        let db = Builder::new_remote_replica(
            ":memory:",
            "libsql://test.turso.io",
            "test-token",
        ).build().await.unwrap();

        let conn = db.connect().unwrap();

        conn.execute("CREATE TABLE test (id INTEGER)", [])
            .await.unwrap();

        let rows = conn.query("SELECT * FROM test", []).await.unwrap();
        assert_eq!(rows.num_rows(), 0);
    }

    #[test]
    fn test_frame_checksum() {
        let data = b"hello world";
        let checksum = Checksum::compute(data);
        assert!(checksum.verify(data));
        assert!(!checksum.verify(b"bad data"));
    }
}
```

## Open Considerations

1. **Multi-threaded WAL** -- Current design serializes writes
2. **Snapshot compression** -- Could be more aggressive
3. **Query planning** -- No cost-based optimizer yet
4. **Distributed transactions** -- 2PC not implemented
