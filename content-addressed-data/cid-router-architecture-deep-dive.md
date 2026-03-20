---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/
repository: N/A - exploration based on cid-router project
explored_at: 2026-03-19
language: Rust
parent: exploration.md
---

# CID Router Architecture Deep Dive

## Overview

`cid-router` is a content-addressed data routing system that resolves requests for CIDs (Content Identifiers) into routes for retrieval. It provides a flexible, extensible architecture for managing content-addressed data across multiple storage backends.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressing/cid-router/`

```
┌─────────────────────────────────────────────────────────────────┐
│                      CID Router Architecture                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │   Client    │────▶│   HTTP API  │────▶│   Core DB   │       │
│  │  (Request)  │     │   Layer     │     │  (SQLite)   │       │
│  └─────────────┘     └─────────────┘     └─────────────┘       │
│                            │                   │                │
│                            ▼                   ▼                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              CID Route Providers (CRPs)                  │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐              │   │
│  │  │  Iroh    │  │  Azure   │  │  Local   │              │   │
│  │  │  (P2P)   │  │ (Cloud)  │  │  (FS)    │              │   │
│  │  └──────────┘  └──────────┘  └──────────┘              │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Indexer (Background)                  │   │
│  │         Periodically reindexes all providers             │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Project Structure

```
cid-router/
├── core/                    # Core library with shared types and traits
│   ├── src/
│   │   ├── cid.rs          # CID types, codecs, hash functions
│   │   ├── cid_filter.rs   # Content filtering system
│   │   ├── context.rs      # Application context and signing
│   │   ├── crp.rs          # CRP trait definition
│   │   ├── db.rs           # SQLite database layer
│   │   ├── indexer.rs      # Background indexing
│   │   ├── repo.rs         # Repository management
│   │   ├── routes.rs       # Route data structures
│   │   └── lib.rs          # Library exports
│   └── Cargo.toml
│
├── crates/                  # Shared utility crates
│   └── api-utils/          # API error handling utilities
│
├── crps/                    # CID Route Provider implementations
│   ├── iroh/               # Iroh P2P blob store provider
│   └── azure/              # Azure Blob Storage provider
│
└── server/                  # HTTP API server
    ├── src/
    │   ├── api/v1/         # API v1 endpoints
    │   ├── auth.rs         # JWT authentication
    │   ├── config.rs       # Server configuration
    │   ├── context.rs      # Server context
    │   └── main.rs         # Entry point
    └── Cargo.toml
```

---

## Key Design Principles

1. **Provider Agnostic**: The CRP (CID Route Provider) trait allows any storage backend to be plugged in
2. **Content Integrity**: All content is verified against its CID before being served
3. **Multiple Routes**: The same CID can have multiple routes from different providers
4. **Lazy Indexing**: Routes are indexed on-demand or via periodic background reindexing
5. **Extensible Filtering**: CID filters allow providers to declare which content they can handle

---

## Data Flow

### Reading Content (GET /v1/data/{cid})

```
1. Client Request → GET /v1/data/{cid}
2. API Layer → Parse CID, authenticate, query database for routes
3. Database → SELECT * FROM routes WHERE cid = ?
4. Route Resolution → For each route: find provider, call get_bytes()
5. Response → Stream bytes back with Content-Type: application/octet-stream
```

### Writing Content (POST /v1/data)

```
1. Client Request → POST /v1/data [binary data]
2. API Layer → Read body, compute BLAKE3 hash, create CID
3. Provider Selection → Filter by cid_filter, get BlobWriter capability
4. Write → Call put_blob() on eligible providers
5. Response → { cid, size, location }
```

### Background Indexing

```
1. Indexer spawns background task (every 3600 seconds)
2. For each provider: call provider.reindex(context)
3. Provider reindex:
   a. Discover new content (list blobs/files)
   b. Create stubs for missing content
   c. Compute CIDs by streaming content
   d. Complete stubs with CID and signature
4. Loop until shutdown
```

---

## Core Concepts

### CID (Content Identifier)

A self-describing content address:

```rust
// Binary format (CIDv1)
// [version: 1 byte][codec: varint][multihash: varint + digest]

// Example: BLAKE3 raw content
// 0x01 (version 1)
// 0x55 (raw codec)
// 0x1e (BLAKE3 multihash code)
// 0x20 (32 bytes digest)
// [32 bytes of BLAKE3 hash]
```

### CRP (CID Route Provider)

The core abstraction for storage backends:

```rust
#[async_trait]
pub trait Crp: Send + Sync + Debug {
    fn provider_id(&self) -> String;
    fn provider_type(&self) -> ProviderType;
    async fn reindex(&self, cx: &Context) -> Result<()>;
    fn capabilities(&self) -> CrpCapabilities;
    fn cid_filter(&self) -> CidFilter;
}
```

### Route

Connects a CID to a retrieval location:

```rust
pub struct Route {
    pub id: Uuid,
    pub created_at: DateTime,
    pub verified_at: DateTime,
    pub provider_id: String,
    pub provider_type: ProviderType,
    pub url: String,
    pub cid: Cid,
    pub size: u64,
    pub multicodec: Codec,
    pub creator: PublicKey,
    pub signature: Vec<u8>,
}
```

### Route Stub

Partially-completed route for two-phase indexing:

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
    // cid is null until content is hashed
}
```

### CID Filter

Declarative filtering for content eligibility:

```rust
pub enum CidFilter {
    None,                              // Accept all CIDs
    MultihashCodeFilter(CodeFilter),   // Filter by hash algorithm
    CodecFilter(CodeFilter),           // Filter by codec
    And(Vec<Self>),                    // Combined conditions
    Or(Vec<Self>),
    Not(Box<Self>),
}

// Example: Only BLAKE3 hashes (Iroh)
let filter = CidFilter::MultihashCodeFilter(CodeFilter::Eq(0x1e));
```

---

## API Reference

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/routes` | GET | List all routes with pagination |
| `/v1/routes/{cid}` | GET | Get all routes for a specific CID |
| `/v1/data/{cid}` | GET | Stream raw content for a CID |
| `/v1/data` | POST | Upload new content |
| `/v1/status` | GET | Get server status |
| `/swagger` | GET | OpenAPI/Swagger UI |

---

## Authentication

### None (Default)
```toml
auth = "none"
```

### EQTY JWT
```toml
[auth]
type = "eqty_jwt"
jwks_url = "https://auth.example.com/.well-known/jwks.json"
```

Requires valid JWT with RS256 signature:
```bash
GET /v1/routes
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

---

## Configuration

### Example server.toml

```toml
port = 8080
auth = "none"

[[providers]]
type = "iroh"
path = "/var/lib/cid-router/blobs"
writeable = true

[[providers]]
type = "azure"
account = "mystorageaccount"
container = "mycontainer"
filter = "all"
```

### Azure Blob Filters

```toml
# Filter by directory prefix
filter = { directory = "data/raw/" }

# Filter by file extension
filter = { file_ext = "car" }

# Filter by size range
filter = { size = { min = 1024, max = 1048576 } }

# Combined filters
filter = { and = [
  { directory = "data/" },
  { file_ext = "car" },
  { size = { min = 1024 } }
]}
```

---

## Repository Structure

Local state stored in:
```
~/.local/share/cid-router/
├── db.sqlite        # SQLite database with routes
├── key              # Ed25519 secret key for signing
└── server.toml      # Server configuration
```

### Database Schema

```sql
CREATE TABLE routes (
    id TEXT PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL,
    verified_at TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    url TEXT NOT NULL,
    cid BLOB,                    -- NULL for stubs
    size INTEGER,
    creator BLOB,
    signature BLOB,
    multicodec TEXT,
    UNIQUE(provider_id, provider_type, cid),
    UNIQUE(provider_id, provider_type, url)
);
```

---

## Provider Comparison

| Feature | Iroh | Azure | Local |
|---------|------|-------|-------|
| Storage | Local FS + P2P | Cloud | Local FS |
| Hash | BLAKE3 only | Any | Any |
| Write | Yes | No | Yes |
| P2P Sync | Yes | No | No |
| Durability | Local | High | Local |

---

## Related Documentation

- [Core Library Deep Dive](./cid-router-core-deep-dive.md)
- [Iroh CRP Deep Dive](./cid-router-iroh-crp-deep-dive.md)
- [Azure CRP Deep Dive](./cid-router-azure-crp-deep-dive.md)
- [Server API Deep Dive](./cid-router-server-deep-dive.md)

---

## Related Resources

- [Iroh Documentation](https://iroh.computer/)
- [IPFS](https://ipfs.tech/)
- [IPLD](https://ipld.io/)
- [Libp2p](https://libp2p.io/)
- [Multiformats](https://multiformats.io/)
