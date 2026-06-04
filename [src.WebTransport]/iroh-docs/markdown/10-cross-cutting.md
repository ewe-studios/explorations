---
title: Cross-Cutting — RPC, CLI, Tickets, Metrics, Download Policies
---

# Cross-Cutting Concerns — RPC, CLI, Tickets, Metrics, Download Policies

## RPC Interface

The RPC layer uses `quic-rpc` for in-memory or network RPC:

### 28+ RPC Methods

| Category | Methods |
|----------|---------|
| Documents | open, close, create, drop, import, list, status, share, subscribe |
| Entries | set, set_hash, get_many, get_exact, del, import_file, export_file |
| Authors | create, default, set_default, list, export, import, delete |
| Sync | start_sync, leave, sync_peers |
| Policies | download_policy (get/set) |

Source: `iroh-docs/src/rpc/proto.rs:1` — `Request` enum with 28+ variants.

### RPC Client

```rust
// iroh-docs/src/rpc/client/docs.rs
pub struct Client {
    rpc: RpcClient,
}

impl Client {
    pub async fn create(&self) -> Result<Doc> { ... }
    pub async fn import(&self, capability: Capability) -> Result<Doc> { ... }
    pub async fn list(&self) -> Result<impl Stream<Item = NamespaceInfo>> { ... }
    pub async fn open(&self, namespace: NamespaceId) -> Result<Doc> { ... }
}

pub struct Doc {
    namespace: NamespaceId,
    client: Client,
    // Auto-closes on drop
}
```

Source: `iroh-docs/src/rpc/client/docs.rs:1`.

### Progress Types

```rust
// iroh-docs/src/rpc/client/docs.rs
pub enum ImportProgress {
    Found,
    Size { size: u64 },
    Ingest { current: u64 },
    Done { hash: Hash },
}
```

Source: `iroh-docs/src/rpc/client/docs.rs:1`.

## CLI Commands

```rust
// iroh-docs/src/cli.rs
pub enum DocCommands {
    /// Switch active document.
    Switch { id: NamespaceId },
    /// Create a new document.
    Create,
    /// Join a document via ticket.
    Join { ticket: String },
    /// List all documents.
    List,
    /// Share a document (print ticket).
    Share { mode: ShareMode },
    /// Set an entry in the document.
    Set { key: String, value: String },
    /// Get entries from the document.
    Get { filter: String },
    /// Delete entries.
    Del { prefix: String },
    /// Watch for changes.
    Watch,
    /// Leave the document sync.
    Leave,
    /// Drop the document.
    Drop,
    /// Import/export files.
    Import { path: PathBuf },
    Export { hash: String, path: PathBuf },
}
```

Source: `iroh-docs/src/cli.rs:1`.

## DocTicket

```rust
// iroh-docs/src/ticket.rs
pub struct DocTicket {
    capability: Capability,
    peers: Vec<NodeAddr>,
}
```

Tickets are base32-encoded with a "doc" prefix for easy sharing:

```
doc<base32-encoded-postcard-data>
```

Source: `iroh-docs/src/ticket.rs:1`.

## Metrics

```rust
// iroh-docs/src/metrics.rs
pub struct Metrics {
    /// Local entries inserted.
    pub local_entries: Counter,
    /// Remote entries inserted.
    pub remote_entries: Counter,
    /// Sync success (connect).
    pub sync_connect_success: Counter,
    /// Sync success (accept).
    pub sync_accept_success: Counter,
    /// Sync failures.
    pub sync_failure: Counter,
    /// Actor tick events.
    pub tick_actor: Counter,
    /// Gossip tick events.
    pub tick_gossip: Counter,
    /// Live actor tick events.
    pub tick_live: Counter,
}
```

Source: `iroh-docs/src/metrics.rs:1`.

## Download Policies

```rust
// iroh-docs/src/store.rs
pub enum DownloadPolicy {
    Nothing,
    Everything,
    EverythingExcept(Vec<FilterKind>),
    NothingExcept(Vec<FilterKind>),
}

pub enum FilterKind {
    Exact(Bytes),
    Prefix(Bytes),
}
```

Policies control which content blobs are automatically downloaded during sync.

Source: `iroh-docs/src/store.rs:1`.

## Related Documents

- [Engine](../markdown/07-engine.md) — Live sync coordination
- [Storage](../markdown/06-storage.md) — Download policy persistence
