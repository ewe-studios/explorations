---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/crps/iroh/
repository: N/A - exploration based on cid-router project
explored_at: 2026-03-19
language: Rust
parent: exploration.md
---

# Iroh CRP Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/crps/iroh/`

---

## Overview

The Iroh CRP (CID Route Provider) integrates with the Iroh P2P blob sync protocol to provide decentralized content-addressed storage.

**What is Iroh?**
- Content-addressed blobs using BLAKE3 hashes
- P2P sync using a BitTorrent-like protocol
- Hole-punching for direct connections between peers
- Encryption using Noise protocol

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Iroh CRP                                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              FsStore (Blob Storage)                   │   │
│  │  Content-addressed: filename = BLAKE3 hash           │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  CRP Interface:                                              │
│  - provider_id(): "iroh"                                    │
│  - cid_filter(): BLAKE3 only (0x1e)                         │
│  - capabilities: RouteResolver + BlobWriter                 │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation

### IrohCrp Structure

```rust
#[derive(Debug, Clone)]
pub struct IrohCrp {
    store: iroh_blobs::store::fs::FsStore,
    writeable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohCrpConfig {
    /// Path to the directory where blobs are stored
    pub path: PathBuf,

    /// Whether the CRP should be writeable
    #[serde(default)]
    pub writeable: bool,
}
```

### Constructor

```rust
impl IrohCrp {
    pub async fn new_from_config(config: IrohCrpConfig) -> io::Result<Self> {
        let path = config.path;
        let store = iroh_blobs::store::fs::FsStore::load(path).await?;
        Ok(Self { store, writeable: config.writeable })
    }
}
```

---

## CRP Trait Implementation

### Provider Identity

```rust
impl Crp for IrohCrp {
    fn provider_id(&self) -> String {
        "iroh".to_string()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Iroh
    }
```

### Reindex Logic

```rust
async fn reindex(&self, _cx: &Context) -> anyhow::Result<()> {
    // TODO: Implement reindexing logic
    Ok(())
}
```

### Capabilities

```rust
fn capabilities<'a>(&'a self) -> CrpCapabilities<'a> {
    CrpCapabilities {
        route_resolver: Some(self),
        blob_writer: if self.writeable { Some(self) } else { None },
    }
}
```

### CID Filter

```rust
fn cid_filter(&self) -> CidFilter {
    CidFilter::MultihashCodeFilter(CodeFilter::Eq(0x1e)) // BLAKE3 only
}
```

**Why BLAKE3 Only?** Iroh natively uses BLAKE3 for content addressing.

---

## BlobWriter Implementation

```rust
#[async_trait]
impl BlobWriter for IrohCrp {
    async fn put_blob(
        &self,
        _auth: Option<Bytes>,
        cid: &Cid,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.writeable {
            return Err("CRP is not writeable".into());
        }

        let blobs = self.store.blobs().clone();
        let data = Bytes::copy_from_slice(data);

        // Verify CID uses BLAKE3
        if cid.hash().code() != 0x1e {
            return Err("Unsupported CID hash code; only blake3 is supported".into());
        }

        // Add to blob store - Iroh computes hash internally
        blobs.add_bytes(data).with_tag().await.map_err(Box::new)?;

        Ok(())
    }
}
```

---

## RouteResolver Implementation

```rust
#[async_trait]
impl RouteResolver for IrohCrp {
    async fn get_bytes(
        &self,
        route: &Route,
        _auth: Option<Bytes>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<bytes::Bytes>> + Send>>> {
        let cid = route.cid;
        let hash = cid.hash().digest();

        // Convert to Iroh's Hash type
        let hash: [u8; 32] = hash.try_into()?;
        let hash = Hash::from_bytes(hash);

        // Fetch from blob store
        let data = self.store.blobs().get_bytes(hash).await?;

        // Wrap in stream
        let stream = futures::stream::once(async move { Ok(data) });

        Ok(Box::pin(stream))
    }
}
```

---

## FsStore Internals

### Directory Structure

```
/var/lib/cid-router/blobs/
├── meta.db              # SQLite metadata database
└── blobs/
    ├── 00/
    │   └── 00abc123...  # Blob file (hash as name)
    ├── 01/
    │   └── 01def456...
    └── ...
```

**Blob Naming:** First 2 chars of hash = directory, prevents too many files in one directory.

### Operations

```rust
// Adding a blob
let outcome = store.blobs().add_bytes(data).await?;

// Getting a blob
let data = store.blobs().get_bytes(hash).await?;

// Checking existence
let exists = store.blobs().contains(hash).await?;
```

---

## Configuration Examples

### Basic (Read-Only Mirror)

```toml
[[providers]]
type = "iroh"
path = "/var/lib/cid-router/blobs"
writeable = false
```

### Full Node (Read/Write)

```toml
[[providers]]
type = "iroh"
path = "/var/lib/cid-router/blobs"
writeable = true
```

---

## Comparison with Other CRPs

| Feature | Iroh | Azure | Local |
|---------|------|-------|-------|
| Storage | Local FS + P2P | Cloud | Local FS |
| Hash | BLAKE3 only | Any | Any |
| Write | Yes | No | Yes |
| P2P Sync | Yes | No | No |
| Durability | Local | High | Local |
| Cost | Free | Paid | Free |

---

## Future Enhancements

1. **Full Iroh Integration**: Add networking and P2P sync
2. **Streaming Put**: Support for large file uploads
3. **Blob GC**: Remove unreferenced blobs
4. **Replication**: Sync with other Iroh nodes automatically

---

## Related Resources

- [Iroh Documentation](https://iroh.computer/)
- [Iroh Rust Crate](https://crates.io/crates/iroh)
- [Iroh Blobs](https://crates.io/crates/iroh-blobs)
- [BLAKE3 Paper](https://github.com/BLAKE3-team/BLAKE3-specs)

---

## See Also

- [Architecture Overview](./cid-router-architecture-deep-dive.md)
- [Core Library Deep Dive](./cid-router-core-deep-dive.md)
- [Azure CRP Deep Dive](./cid-router-azure-crp-deep-dive.md)
- [Server API Deep Dive](./cid-router-server-deep-dive.md)
