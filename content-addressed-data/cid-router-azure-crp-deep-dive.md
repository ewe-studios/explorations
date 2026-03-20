---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/crps/azure/
repository: N/A - exploration based on cid-router project
explored_at: 2026-03-19
language: Rust
parent: exploration.md
---

# Azure Blob Storage CRP Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/crps/azure/`

---

## Overview

The Azure CRP (CID Route Provider) integrates Azure Blob Storage with the CID Router system, allowing you to:
- Index existing Azure blobs as content-addressed routes
- Serve blob content through the CID Router API
- Compute CIDs for blobs that weren't originally content-addressed

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Azure CRP Architecture                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Azure Blob Storage                          │   │
│  │  Storage Account → Container → Blobs                    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Azure Container CRP                         │   │
│  │  - Blob Lister (List Blobs)                             │   │
│  │  - Blob Filter (Path/Ext/Size)                          │   │
│  │  - CID Computer (BLAKE3 Hash)                           │   │
│  │  - Stream Reader (Chunked)                              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
crps/azure/
├── src/
│   ├── lib.rs          # Module exports
│   ├── config.rs       # Configuration types
│   └── container.rs    # Main CRP implementation
└── Cargo.toml
```

---

## Configuration

### ContainerConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub account: String,
    pub container: String,
    pub credentials: Option<Credentials>,
    pub filter: ContainerBlobFilter,
}
```

### Credentials

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
}
```

**Authentication Methods:**
| Method | When to Use |
|--------|-------------|
| Anonymous | Public containers |
| Service Principal | Production workloads |

---

## Blob Filtering

### ContainerBlobFilter

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerBlobFilter {
    All,
    Directory(String),
    FileExt(String),
    NameContains(String),
    Size { min: Option<u64>, max: Option<u64> },
    And(Vec<Self>),
    Or(Vec<Self>),
    Not(Box<Self>),
}
```

### Filter Examples

```toml
# All blobs
filter = "all"

# Only blobs in a directory
filter = { directory = "data/raw/" }

# Only CAR files
filter = { file_ext = "car" }

# Size between 1KB and 1MB
filter = { size = { min = 1024, max = 1048576 } }

# Complex: CAR files in data/ directory, larger than 1KB
filter = { and = [
  { directory = "data/" },
  { file_ext = "car" },
  { size = { min = 1024 } }
]}
```

---

## Container CRP Implementation

### Structure

```rust
#[derive(Debug, Clone)]
pub struct Container {
    cfg: ContainerConfig,
    client: ContainerClient,  // Azure SDK client
}
```

### CRP Trait Implementation

```rust
impl Crp for Container {
    fn provider_id(&self) -> String {
        self.cfg.container.clone()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Azure
    }

    fn cid_filter(&self) -> CidFilter {
        CidFilter::None  // Accepts all CIDs
    }

    fn capabilities(&self) -> CrpCapabilities {
        CrpCapabilities {
            route_resolver: Some(self),
            blob_writer: None,  // Read-only
        }
    }
}
```

---

## Reindex Process (Two-Phase)

### Phase 1: Add Stubs for Missing Blobs

```rust
async fn add_stubs_for_missing_blobs(&self, cx: &Context) -> Result<()> {
    // List all blobs in container
    let response = self.client
        .list_blobs()
        .max_results(10000)
        .into_stream()
        .next()
        .await?;

    for blob in response.blobs.blobs() {
        // Apply filter
        if !self.cfg.filter.blob_is_match(&blob.name, blob.properties.content_length) {
            continue;
        }

        // Build URL for this blob
        let url = self.blob_to_route_url(blob);

        // Check if route already exists
        if cx.db().routes_for_url(&url).await?.is_empty() {
            // Create stub (without CID)
            let stub = Route::builder(self)
                .size(blob.properties.content_length)
                .url(url)
                .multicodec(Codec::Raw)
                .build_stub()?;

            cx.db().insert_stub(&stub).await?;
        }
    }
    Ok(())
}
```

### Phase 2: Update CID Hashes

```rust
async fn update_blob_index_hashes(&self, cx: &Context) -> Result<()> {
    // Get all stubs for this provider
    let stubs = cx.db()
        .list_provider_stubs(&self.provider_id(), ...)
        .await?;

    for stub in stubs {
        // Stream blob and compute BLAKE3 hash
        let cid = self.calculate_blob_cid(&stub).await?;

        // Complete the stub with CID
        let route = stub.builder().cid(cid).build(cx)?;
        cx.db().complete_stub(&route).await?;
    }
    Ok(())
}
```

---

## CID Calculation

```rust
async fn calculate_blob_cid(&self, stub: &RouteStub) -> Result<Cid> {
    let name = Self::route_url_to_name(&stub.url)?;

    let hash = {
        let mut hasher = blake3::Hasher::new();

        // Stream blob from Azure (chunked)
        let blob_client = self.client.blob_client(&name);
        let mut blob_stream = blob_client.get().into_stream();

        while let Some(chunk_response) = blob_stream.next().await {
            let chunk = chunk_response?.data.collect().await?;
            hasher.update(&chunk);
        }

        hasher.finalize()
    };

    let cid = blake3_hash_to_cid(hash.into(), Codec::Raw);
    Ok(cid)
}
```

**Memory Efficiency:** Only one chunk in memory at a time.

---

## RouteResolver Implementation

```rust
#[async_trait]
impl RouteResolver for Container {
    async fn get_bytes(
        &self,
        route: &Route,
        _auth: Option<Bytes>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<bytes::Bytes>> + Send>>> {
        let name = Self::route_url_to_name(&route.url)?;
        let client = self.client.blob_client(&name);
        let stream = client.get().into_stream();

        // Map chunks to Bytes result
        let mapped_stream = stream.then(|chunk_response| async move {
            chunk_response?
                .data
                .collect()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        });

        Ok(Box::pin(mapped_stream))
    }
}
```

---

## URL Parsing

```rust
fn route_url_to_name(url: &str) -> Result<String> {
    // URL format: https://{account}.blob.core.windows.net/{container}/{name}
    let parts: Vec<&str> = url.split('/').collect();

    if parts.len() >= 5 && parts[2].ends_with(".blob.core.windows.net") {
        Ok(parts[4..].join("/"))
    } else {
        Err(anyhow!("Invalid blob route URL"))
    }
}
```

---

## Configuration Examples

### Public Container (Anonymous)

```toml
[[providers]]
type = "azure"
account = "publicdata"
container = "datasets"
filter = "all"
```

### Private Container (Service Principal)

```toml
[[providers]]
type = "azure"
account = "mycompany"
container = "production-data"
filter = { directory = "trusted/" }

[providers.credentials]
tenant_id = "00000000-0000-0000-0000-000000000000"
client_id = "00000000-0000-0000-0000-000000000000"
client_secret = "secret"
```

---

## Performance Considerations

### Listing Blobs
- Currently fetches only first page (10,000 blobs)
- For large containers, implement pagination

### Streaming
- Blob is streamed in 4MB chunks by default
- Hash computed incrementally

### Concurrent Hash Computation (Future)

```rust
// Current: Sequential
for stub in stubs {
    let cid = self.calculate_blob_cid(&stub).await?;
}

// Improved: Concurrent (with rate limiting)
let stream = futures::stream::iter(stubs)
    .map(|stub| self.calculate_blob_cid(&stub))
    .buffered(10);  // 10 concurrent streams
```

---

## Security Considerations

### Credential Management

```toml
# BAD: Hardcoded credentials
client_secret = "actual-secret-value"

# GOOD: Environment variable reference
client_secret = "${AZURE_CLIENT_SECRET}"
```

### Network Security
- Use private endpoints for production
- Enable HTTPS-only
- Consider VNet integration

---

## Comparison with Iroh CRP

| Feature | Azure | Iroh |
|---------|-------|------|
| Storage | Cloud (Azure) | Local + P2P |
| Hash Computation | On-read (streaming) | On-write |
| Write Support | No | Yes |
| CID Filter | None (all) | BLAKE3 only |
| Authentication | Azure AD/SAS | None |
| Durability | High (Azure) | Local |
| Cost | Paid | Free |

---

## Future Enhancements

1. **Write Support**: Implement `BlobWriter` for uploading new blobs
2. **SAS Token Generation**: Create time-limited access tokens
3. **Pagination**: Handle containers with >10,000 blobs
4. **Incremental Indexing**: Track which blobs have changed
5. **CDN Integration**: Serve through Azure CDN

---

## Related Resources

- [Azure Blob Storage Docs](https://docs.microsoft.com/en-us/azure/storage/blobs/)
- [azure-storage-blobs crate](https://crates.io/crates/azure-storage-blobs)
- [azure-identity crate](https://crates.io/crates/azure-identity)

---

## See Also

- [Architecture Overview](./cid-router-architecture-deep-dive.md)
- [Core Library Deep Dive](./cid-router-core-deep-dive.md)
- [Iroh CRP Deep Dive](./cid-router-iroh-crp-deep-dive.md)
- [Server API Deep Dive](./cid-router-server-deep-dive.md)
