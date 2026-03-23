---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.matrix-web
repository: https://github.com/element-hq (multiple repositories)
revised_at: 2026-03-23
workspace: matrix-web-rs
---

# Rust Revision: Matrix Web Ecosystem

## Overview

This revision covers the Rust components within the Matrix Web ecosystem: the Matrix Authentication Service (MAS), the Matrix Rich Text Editor (wysiwyg), the JOSE library (josekit-rs), and Synapse's Rust extensions. Together, these projects demonstrate several key Rust patterns: large workspace architecture, cross-platform FFI via Uniffi and wasm-bindgen, Python interop via PyO3, and production-grade async web services with Axum.

The revision focuses on extractable patterns, crate design decisions, and architectural choices that would apply when building similar systems in Rust.

## Workspace Structures

### MAS: Large Service Workspace (27 Crates)

```
matrix-authentication-service/
├── Cargo.toml                  # Workspace root, edition 2024
├── crates/
│   ├── cli/                    # Binary: entry point (clap CLI)
│   ├── config/                 # Config loading (figment)
│   ├── data-model/             # Domain types (zero dependencies on frameworks)
│   ├── storage/                # Trait definitions (async repository interfaces)
│   ├── storage-pg/             # PostgreSQL impl (sqlx + sea-query)
│   ├── handlers/               # HTTP handlers (axum extractors)
│   ├── router/                 # URL routing
│   ├── listener/               # HTTP binding + TLS
│   ├── graphql/                # Admin API (async-graphql)
│   ├── jose/                   # JOSE crypto (JWT signing)
│   ├── keystore/               # Key management
│   ├── email/                  # Email sending (lettre)
│   ├── templates/              # HTML rendering (minijinja)
│   ├── policy/                 # Authorization (OPA/Rego)
│   ├── tasks/                  # Background jobs
│   ├── tower/                  # Middleware layers
│   ├── http/                   # HTTP client utilities
│   ├── axum-utils/             # Shared extractors
│   ├── matrix/                 # Matrix protocol types
│   ├── matrix-synapse/         # Synapse-specific API
│   ├── oidc-client/            # Upstream IdP client
│   ├── oauth2-types/           # OAuth2 type defs
│   ├── i18n/                   # Internationalization
│   ├── i18n-scan/              # i18n tooling
│   ├── iana/                   # IANA registries as types
│   ├── iana-codegen/           # Codegen for IANA
│   ├── spa/                    # SPA file serving
│   └── syn2mas/                # Migration tooling
```

### Rich Text Editor: Cross-Platform Library Workspace

```
matrix-rich-text-editor/
├── Cargo.toml                  # Workspace root, Rust 1.83
├── crates/
│   ├── wysiwyg/                # Core library (generic over string type)
│   └── matrix_mentions/        # Domain-specific mention handling
├── bindings/
│   ├── wysiwyg-ffi/            # Uniffi bindings (Kotlin/Swift)
│   └── wysiwyg-wasm/           # wasm-bindgen bindings (JavaScript)
└── uniffi-bindgen/             # Custom binding generator
```

### Synapse Rust Extension: PyO3 Module

```
synapse/rust/
├── Cargo.toml                  # cdylib + lib crate types
└── src/
    ├── lib.rs                  # #[pymodule] definition
    ├── acl/                    # Server ACL evaluation
    ├── events/                 # Canonical JSON, event hashing
    ├── push/                   # Push rule evaluation
    ├── rendezvous/             # QR login protocol
    ├── http.rs                 # HTTP utilities
    └── identifier.rs           # Matrix ID parsing
```

## Recommended Dependencies

### Core Framework Layer

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Async runtime | tokio | 1.44+ | Industry standard, full-featured |
| HTTP framework | axum | 0.8+ | Tower-native, composable, type-safe extractors |
| Middleware | tower / tower-http | 0.5 / 0.6 | Composable service layers |
| Serialization | serde + serde_json | 1.0 | De facto standard |
| Error handling | thiserror | 2.0 | Derive macro for error types |
| Error context | anyhow | 1.0 | For application-level errors |
| CLI parsing | clap | 4.5+ | Derive-based argument parsing |
| Configuration | figment | 0.10 | Multi-source config (YAML, env, defaults) |

### Data Layer

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| PostgreSQL | sqlx | 0.8+ | Compile-time checked SQL, async |
| Query builder | sea-query | 0.32+ | Type-safe SQL construction |
| GraphQL | async-graphql | 7.0+ | Async-native, derives |
| Identifiers | ulid | 1.1 | Sortable unique IDs |
| Timestamps | chrono | 0.4 | DateTime handling |
| Short strings | compact_str | 0.8 | Memory optimization |

### Cryptography

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| EC curves | elliptic-curve, p256, p384, k256 | 0.13 | Pure Rust EC operations |
| RSA/PKCS | pkcs1, pkcs8 | 0.7/0.10 | Key encoding |
| PEM | pem-rfc7468 | 0.7 | PEM file handling |
| Hashing | sha2 | 0.10 | SHA-256 |
| Base64 | base64ct | 1.7 | Constant-time base64 |
| RNG | rand, rand_chacha | 0.8 | Cryptographic randomness |
| TLS | rustls | 0.23 | Pure Rust TLS |

### Cross-Platform FFI

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Uniffi | uniffi | git rev | Mobile FFI (Kotlin/Swift) |
| WASM | wasm-bindgen | latest | JavaScript/WASM interop |
| Python | pyo3 | 0.23 | Python extension modules |

### Observability

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Tracing | tracing + tracing-subscriber | 0.1/0.3 | Structured logging |
| OpenTelemetry | opentelemetry + opentelemetry_sdk | 0.28 | Distributed traces + metrics |
| Prometheus | opentelemetry-prometheus | 0.28 | Metrics export |
| Error tracking | sentry | 0.36 | Production error reporting |

### Testing

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Snapshots | insta | 1.42 | Snapshot testing (YAML, JSON) |
| HTTP mocks | wiremock | 0.6 | HTTP mock server |

## Type System Design

### Core Domain Types (MAS Pattern)

```rust
use chrono::{DateTime, Utc};
use ulid::Ulid;
use url::Url;

/// User identity in the authentication system
pub struct User {
    pub id: Ulid,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
}

/// OAuth2 client application
pub struct OAuthClient {
    pub id: Ulid,
    pub client_id: String,
    pub redirect_uris: Vec<Url>,
    pub grant_types: Vec<GrantType>,
    pub token_endpoint_auth_method: TokenAuthMethod,
}

/// Browser session (cookie-based)
pub struct BrowserSession {
    pub id: Ulid,
    pub user_id: Ulid,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub user_agent: Option<String>,
}

/// OAuth2 grant types
pub enum GrantType {
    AuthorizationCode,
    RefreshToken,
    ClientCredentials,
    DeviceCode,
}

/// Token endpoint auth methods
pub enum TokenAuthMethod {
    None,
    ClientSecretBasic,
    ClientSecretPost,
    PrivateKeyJwt,
}
```

### Storage Trait Pattern (Repository Abstraction)

```rust
use async_trait::async_trait;

/// Repository trait for user operations.
/// Concrete implementations live in separate crates (e.g., storage-pg).
#[async_trait]
pub trait UserRepository: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn find_by_id(&self, id: Ulid) -> Result<Option<User>, Self::Error>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, Self::Error>;
    async fn create(&self, username: String) -> Result<User, Self::Error>;
    async fn lock(&self, id: Ulid) -> Result<(), Self::Error>;
    async fn list(
        &self,
        pagination: Pagination,
        filter: UserFilter,
    ) -> Result<Page<User>, Self::Error>;
}

/// Pagination parameters for list operations
pub struct Pagination {
    pub first: Option<usize>,
    pub after: Option<Ulid>,
    pub last: Option<usize>,
    pub before: Option<Ulid>,
}

pub struct Page<T> {
    pub edges: Vec<T>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}
```

### Error Types

```rust
use thiserror::Error;

/// Application-level errors following MAS pattern
#[derive(Debug, Error)]
pub enum AppError {
    #[error("user not found: {0}")]
    UserNotFound(Ulid),

    #[error("invalid OAuth2 grant: {0}")]
    InvalidGrant(String),

    #[error("token expired")]
    TokenExpired,

    #[error("insufficient scope: required {required}, got {actual}")]
    InsufficientScope {
        required: String,
        actual: String,
    },

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid key: {0}")]
    InvalidKey(String),

    #[error("signature verification failed")]
    VerificationFailed,

    #[error("unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
}
```

### Cross-Platform Editor Types (Rich Text Editor Pattern)

```rust
/// Generic over string type for platform flexibility.
/// Web uses String, mobile platforms may use different representations.
pub struct ComposerModel<S: UnicodeString> {
    state: ComposerState<S>,
    action_history: Vec<ComposerAction>,
}

pub trait UnicodeString: Clone + Default + AsRef<str> {
    fn from_str(s: &str) -> Self;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

/// DOM tree for rich text representation
pub enum DomNode<S: UnicodeString> {
    Text(TextNode<S>),
    Container(ContainerNode<S>),
    Formatting(FormattingNode<S>),
    Link(LinkNode<S>),
    Mention(MentionNode<S>),
    LineBreak,
}

pub struct TextNode<S: UnicodeString> {
    pub data: S,
}

pub struct FormattingNode<S: UnicodeString> {
    pub format: FormatType,
    pub children: Vec<DomNode<S>>,
}

pub enum FormatType {
    Bold,
    Italic,
    Underline,
    StrikeThrough,
    InlineCode,
}

/// Result of an editor operation
pub struct ComposerUpdate<S: UnicodeString> {
    pub text_update: TextUpdate<S>,
    pub menu_state: MenuState,
    pub menu_action: MenuAction,
}
```

## Key Rust-Specific Changes

### 1. Trait-Based Storage Abstraction

**Source Pattern:** Direct database calls scattered through handlers.

**Rust Translation:** `mas-storage` defines async trait interfaces; `mas-storage-pg` provides the PostgreSQL implementation. Handlers depend only on traits.

**Rationale:** Enables testing with mock implementations, future storage backend swaps, and clean dependency inversion. The trait crate has zero framework dependencies.

### 2. Generic String Type for Cross-Platform

**Source Pattern:** Platform-specific string types (NSString, java.lang.String, JS String).

**Rust Translation:** `ComposerModel<S: UnicodeString>` is generic over the string type. WASM binding uses `String`, FFI binding uses a type compatible with Uniffi.

**Rationale:** Single source of truth for editor logic while accommodating platform-specific string representations at the boundary.

### 3. Tower Middleware Composition

**Source Pattern:** Express/Koa-style middleware chains.

**Rust Translation:** Tower `Service` trait with `Layer` composition. Each middleware (tracing, error handling, rate limiting, CORS) is a separate Tower layer.

**Rationale:** Type-safe middleware composition with compile-time guarantees. Tower's `Service` trait enables zero-cost abstraction for request/response processing.

### 4. PyO3 for Python Hot Path Optimization

**Source Pattern:** Pure Python implementations with performance bottlenecks.

**Rust Translation:** Synapse's `synapse_rust` crate uses PyO3 with `abi3-py39` for stable ABI across Python versions. Hot paths (canonical JSON, push rules) are Rust functions callable from Python.

**Rationale:** Surgical performance optimization without rewriting the entire application. `abi3` ensures binary compatibility across Python versions.

## Ownership & Borrowing Strategy

### MAS Service Layer

```rust
// Handlers take shared references to services via Arc
pub struct AuthorizationHandler {
    config: Arc<Config>,
    keystore: Arc<Keystore>,
    // Storage is passed per-request as a transaction
}

impl AuthorizationHandler {
    pub async fn handle(
        &self,
        // Axum extracts the DB pool; handler gets a transaction
        mut txn: BoxRepository,
        request: AuthorizationRequest,
    ) -> Result<AuthorizationResponse, AppError> {
        let user = txn.user().find_by_id(request.user_id).await?;
        let client = txn.oauth_client().find_by_id(request.client_id).await?;
        // ... process authorization
        txn.commit().await?;
        Ok(response)
    }
}
```

### Editor State (Rich Text Editor)

```rust
// ComposerModel owns its state; operations take &mut self
impl<S: UnicodeString> ComposerModel<S> {
    pub fn replace_text(&mut self, text: S) -> ComposerUpdate<S> {
        // Mutates internal DOM tree, returns update description
        let update = self.state.dom.replace_text_in_selection(text);
        self.compute_update(update)
    }

    // Read-only operations take &self
    pub fn get_content_as_html(&self) -> S {
        self.state.dom.to_html()
    }
}

// FFI layer wraps in Arc<Mutex<>> for thread safety across FFI boundary
pub struct FfiComposerModel {
    inner: Arc<Mutex<ComposerModel<String>>>,
}
```

## Concurrency Model

**Approach:** Tokio async for services, single-threaded ownership for editor, PyO3 GIL-aware for Synapse.

### MAS (Async Service)

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let pool = PgPool::connect(&config.database.uri).await?;

    // Background tasks run as tokio::spawn tasks
    let task_handle = tokio::spawn(async move {
        run_background_tasks(pool.clone()).await
    });

    // HTTP server runs on tokio
    let listener = TcpListener::bind(&config.http.bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### Rich Text Editor (Single-Threaded Core)

The core `ComposerModel` is not `Send` or `Sync` by default. The FFI bindings wrap it appropriately:

```rust
// WASM: single-threaded, no sync needed
#[wasm_bindgen]
pub struct WysiwygComposer {
    model: ComposerModel<String>, // No Arc/Mutex needed
}

// FFI (mobile): wrapped for thread safety
pub struct FfiComposerModel {
    inner: Arc<Mutex<ComposerModel<String>>>,
}
```

### Synapse Rust Extension (GIL-Aware)

```rust
#[pyfunction]
fn canonical_json(py: Python, value: &Bound<'_, PyAny>) -> PyResult<String> {
    // Release GIL for CPU-bound work
    let rust_value: serde_json::Value = pythonize::depythonize(value)?;
    py.allow_threads(|| {
        Ok(canonical_json_serialize(&rust_value)?)
    })
}
```

## Memory Considerations

- **MAS:** Uses `compact_str` for short string optimization (usernames, client IDs). `Arc<Config>` shared across handlers to avoid cloning large configs. `arc-swap` for atomic config updates without locks.
- **Rich Text Editor:** `opt-level = 'z'` in release profile to minimize WASM binary size. DOM tree uses owned nodes (no lifetimes across FFI boundary). `panic = 'unwind'` to catch panics at FFI boundary.
- **Synapse Extension:** `cdylib` output for Python loading. `pythonize` handles Python-Rust value conversion without manual memory management.

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Token expiry during request | Result type propagates; handlers check expiry before use |
| Database connection loss | sqlx returns error; anyhow propagates with context |
| Malformed JWT | josekit returns typed error; handler returns 401 |
| FFI panic (editor) | `panic = 'unwind'` + catch_unwind at FFI boundary |
| WASM OOM | Browser handles; Rust allocator returns error |
| Concurrent config update | arc-swap provides lock-free atomic pointer swap |
| Unicode edge cases (editor) | UnicodeString trait ensures valid UTF-8 at all times |
| SQL injection | sea-query parameterized queries + sqlx compile-time checks |

## Testing Strategy

### MAS Testing
- **Unit tests:** Per-crate with mock storage implementations
- **Integration tests:** sqlx test fixtures with PostgreSQL
- **Snapshot tests:** insta for API response snapshots (YAML, JSON)
- **HTTP mocks:** wiremock for external service testing
- **Dev profile optimization:** Crypto crates (argon2, bcrypt, sha2) built with opt-level 3 even in debug to keep tests fast

### Rich Text Editor Testing
- **Core tests:** Extensive unit tests in `crates/wysiwyg/src/tests/`
- **Cross-platform:** Same Rust tests validate logic used by all platforms
- **Property-based:** DOM tree operations tested for invariant preservation

### Synapse Extension Testing
- **Dual crate type:** `lib` for Rust tests + `cdylib` for Python loading
- **Python integration:** Maturin-based test workflow

## Performance Considerations

- MAS uses **LTO + single codegen unit** in release for maximum optimization
- Rich text editor uses **opt-level 'z'** (size optimization) since WASM bundle size directly impacts web load time
- Synapse extension uses **`allow_threads`** to release the Python GIL during CPU-bound Rust operations
- MAS selectively optimizes crypto crates in dev profile to avoid slow test suites
- `compact_str` avoids heap allocation for strings under 24 bytes (common for Matrix identifiers)

## Migration Path

For building a new Matrix-compatible service in Rust:

1. **Start with domain types** - Define core types in a `data-model` crate with zero framework dependencies
2. **Define storage traits** - Create async repository interfaces in a `storage` crate
3. **Implement storage** - Build PostgreSQL implementation in `storage-pg` using sqlx
4. **Build handlers** - Implement HTTP handlers with axum, depending only on storage traits
5. **Add crypto** - Use established crates (elliptic-curve ecosystem, not raw OpenSSL)
6. **Wire it up** - CLI crate with clap, configuration with figment, Tower middleware stack
7. **Add observability** - tracing + OpenTelemetry from day one
8. **FFI last** - Add Uniffi/wasm-bindgen bindings only after core API is stable

## Open Considerations

- **Edition 2024 adoption:** MAS uses edition 2024; other projects are on 2021. Edition 2024 changes `unsafe_op_in_unsafe_fn` to deny by default and other ergonomic improvements.
- **Uniffi stability:** The rich text editor pins Uniffi to a specific git rev rather than a released version, suggesting the API surface is still evolving.
- **OpenSSL vs pure Rust crypto:** josekit-rs depends on OpenSSL; MAS uses the pure-Rust elliptic-curve ecosystem. For new projects, prefer pure-Rust crypto to avoid linking complexity.
- **sqlx compile-time checking:** Requires a running database during compilation. Consider `query_as_unchecked!` for CI environments without database access, or use sqlx's offline mode.
- **WASM binary size:** The rich text editor optimizes for size with `opt-level = 'z'`. For production WASM, consider `wasm-opt` post-processing and tree shaking.
