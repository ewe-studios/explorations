# Security Model: Auth, Sandboxing, and Unsafe Code Isolation

Moltis is secure by design with multiple layers of authentication, sandboxed tool execution, and isolated unsafe code boundaries.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/`

## Authentication

**Source:** `crates/auth/src/`

### Module Structure

```
crates/auth/src/
├── lib.rs              ← Re-exports all public API
├── credential_store.rs ← SQLite-backed credential storage
├── webauthn.rs         ← WebAuthn (passkey) challenge management
├── locality.rs         ← Connection locality detection
└── error.rs            ← Error types
```

### CredentialStore

**Source:** `credential_store.rs`

SQLite-backed storage for:
- **Passwords** — User password hashes
- **Passkeys** — WebAuthn/FIDO2 credentials
- **API keys** — API key storage with scopes
- **Sessions** — Active session management

```rust
pub struct CredentialStore {
    pool: SqlitePool,
}
```

### WebAuthn (Passkey) Authentication

**Source:** `webauthn.rs`

```rust
pub struct WebAuthnRegistry { ... }
pub struct WebAuthnState { ... }
pub fn load_passkeys() -> Vec<Passkey>;
```

Handles WebAuthn challenge-response for passwordless authentication.

### Connection Locality Detection

**Source:** `locality.rs`

```rust
pub fn has_proxy_headers(request: &Request) -> bool;
pub fn is_local_connection(request: &Request) -> bool;
```

Detects whether connections are local or proxied for auth decisions.

## Vault

**Source:** `crates/vault/src/`

Secret vault for storing API keys and sensitive credentials. Feature-gated with `#[cfg(feature = "vault")]`.

## Sandboxing

**Source:** `crates/tools/src/exec.rs`

The exec tool implements sandboxing through:

```rust
// exec.rs:206-207
sandbox: Arc<dyn Sandbox>,
sandbox_router: Option<Arc<SandboxRouter>>,
```

### Sandbox Trait

```rust
pub trait Sandbox: Send + Sync {
    async fn execute(&self, command: &str, opts: ExecOpts) -> Result<ExecResult>;
    fn provides_fs_isolation(&self) -> bool;
    async fn cleanup(&self, id: &SandboxId) -> Result<()>;
}
```

### SandboxRouter

**Source:** `sandbox.rs`

Routes exec requests to the appropriate sandbox backend based on session configuration:

```rust
pub struct SandboxRouter { ... }

impl SandboxRouter {
    pub async fn is_sandboxed(&self, session_key: &str) -> bool;
    pub async fn resolve_backend(&self, session_key: &str) -> SandboxBackend;
}
```

### SandboxBackend

```rust
pub enum SandboxBackend {
    Docker,
    Podman,
    AppleContainer,
    RestrictedHost,  // No filesystem isolation
    Cgroup,          // No filesystem isolation
    None,            // No sandboxing
}
```

### Filesystem Isolation Check

```rust
// exec.rs:459-464
let has_container_backend = if let Some(ref router) = self.sandbox_router {
    let sk = session_key.unwrap_or("main");
    router.resolve_backend(sk).await.provides_fs_isolation()
} else {
    self.sandbox.provides_fs_isolation()
};
```

When the backend doesn't provide filesystem isolation (`RestrictedHost`, `Cgroup`, `None`), commands run directly on the host even when the session mode says "sandboxed."

## TLS

**Source:** `crates/tls/src/`

TLS configuration for secure connections. Feature-gated with `#[cfg(feature = "tls")]`.

## Network Filter

**Source:** `crates/network-filter/src/`

Controls outbound network access from tools and plugins.

## Plugin System

**Source:** `crates/plugins/src/`

Plugins run in isolated QuickJS-WASM sandboxes with:
- Capability-gated SDK access
- Network access controlled via `network.outbound` permission
- Crash isolation via separate Bun.Worker per plugin
- Crash budget (limited restarts before disabling)

## Unsafe Code Isolation

Unsafe Rust code in Moltis is isolated to FFI boundaries:

| Crate | Location | Purpose |
|-------|----------|---------|
| `memory/embeddings_local.rs` | `#[allow(unsafe_code)]` | llama-cpp-2 FFI wrappers |
| `swift-bridge/` | FFI boundary | Rust-Swift interop |
| `wasm-precompile/` | WASM boundaries | WASM precompilation |

The core agent loop, tool execution, and business logic contain **no unsafe code**.

## Next

Return to the [README](README.md) for the full index.
